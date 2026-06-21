use std::collections::VecDeque;

use crate::index::{skip_list::SkipList, Key, MemTable, Value};
use crate::wal::WalRecord;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MemTableWriteOutcome {
    pub rotated: bool,
    pub should_flush: bool,
}

#[derive(Debug)]
pub struct MemTableManager {
    active: SkipList,
    immutable: VecDeque<SkipList>,
    flush_threshold: usize,
    maximum_immutable: usize,
}

impl MemTableManager {
    pub fn new(flush_threshold: usize, maximum_immutable: usize) -> Self {
        Self {
            active: SkipList::default(),
            immutable: VecDeque::new(),
            flush_threshold,
            maximum_immutable,
        }
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> MemTableWriteOutcome {
        self.active.put(Key::new(key), Value::Put(value));
        self.rotate_if_needed()
    }

    pub fn delete(&mut self, key: Vec<u8>) -> MemTableWriteOutcome {
        self.active.delete(Key::new(key));
        self.rotate_if_needed()
    }

    pub fn recover(&mut self, record: WalRecord) -> MemTableWriteOutcome {
        match record {
            WalRecord::Put { key, value, .. } => self.put(key, value),
            WalRecord::Delete { key, .. } => self.delete(key),
        }
    }

    pub fn recover_active_segment(&mut self, records: impl IntoIterator<Item = WalRecord>) {
        self.active = recover_segment(records);
    }

    pub fn recover_immutable_segment(&mut self, records: impl IntoIterator<Item = WalRecord>) {
        self.immutable.push_back(recover_segment(records));
    }

    pub fn get(&self, key: &Key) -> Option<&Value> {
        if let Some(value) = self.active.get(key) {
            return Some(value);
        }

        for memtable in self.immutable.iter().rev() {
            if let Some(value) = memtable.get(key) {
                return Some(value);
            }
        }

        None
    }

    pub fn pop_flushable(&mut self) -> Option<SkipList> {
        self.immutable.pop_front()
    }

    pub fn active_size(&self) -> usize {
        self.active.approximate_size()
    }

    pub fn immutable_count(&self) -> usize {
        self.immutable.len()
    }

    fn rotate_if_needed(&mut self) -> MemTableWriteOutcome {
        if self.active.approximate_size() < self.flush_threshold {
            return MemTableWriteOutcome {
                rotated: false,
                should_flush: false,
            };
        }

        let full_memtable = std::mem::take(&mut self.active);
        self.immutable.push_back(full_memtable);

        MemTableWriteOutcome {
            rotated: true,
            should_flush: self.immutable.len() > self.maximum_immutable,
        }
    }
}

fn recover_segment(records: impl IntoIterator<Item = WalRecord>) -> SkipList {
    let mut memtable = SkipList::default();

    for record in records {
        match record {
            WalRecord::Put { key, value, .. } => memtable.put(Key::new(key), Value::Put(value)),
            WalRecord::Delete { key, .. } => memtable.delete(Key::new(key)),
        }
    }

    memtable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotates_when_active_memtable_crosses_threshold() {
        let mut manager = MemTableManager::new(2, 4);

        let outcome = manager.put(b"a".to_vec(), b"1".to_vec());

        assert!(outcome.rotated);
        assert!(!outcome.should_flush);
        assert_eq!(manager.active_size(), 0);
        assert_eq!(manager.immutable_count(), 1);
    }

    #[test]
    fn reads_newest_value_across_active_and_immutable_memtables() {
        let mut manager = MemTableManager::new(8, 4);

        manager.put(b"alpha".to_vec(), b"one".to_vec());
        manager.put(b"alpha".to_vec(), b"two".to_vec());

        assert_eq!(
            manager.get(&Key::new("alpha")).and_then(Value::as_bytes),
            Some(b"two".as_slice())
        );
    }
}
