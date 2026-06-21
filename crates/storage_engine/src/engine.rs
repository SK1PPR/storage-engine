use std::collections::VecDeque;

use crate::{
    index::{skip_list::SkipList, Key, MemTable, Value},
    storage::sstable::{reader::read_from_table, writer::SsTableWriter, SsTable},
    wal::{WriteAheadLogger,WalRecord},
    EngineConfig, Result,
};
use std::path::Path;

pub struct Engine {
    wal: WriteAheadLogger,
    memtable: SkipList,
    memtable_list: VecDeque<SkipList>,
    sstables: Vec<SsTable>,
    next_sstable_id: u64,
    config: EngineConfig,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        let wal_dir = config.data_dir.join("wal");

        Self {
            wal: WriteAheadLogger::new(wal_dir).expect("failed to create WAL directory"),
            memtable: SkipList::default(),
            memtable_list: VecDeque::new(),
            sstables: Vec::new(),
            next_sstable_id: 0,
            config,
        }
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let sequence_id = self.wal.next_sequence();
        self.wal
            .append(WalRecord::put(sequence_id, key.clone(), value.clone()))?;
        self.memtable.put(Key::new(key), Value::Put(value));

        self.rotate_memtable_if_needed()?;

        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let key = Key::new(key);

        if let Some(value) = self.memtable.get(&key) {
            return Ok(value.as_bytes().map(ToOwned::to_owned));
        }

        for memtable in self.memtable_list.iter().rev() {
            if let Some(value) = memtable.get(&key) {
                return Ok(value.as_bytes().map(ToOwned::to_owned));
            }
        }

        for sstable in self.sstables.iter().rev() {
            if let Some(value) = read_from_table(sstable, &key)? {
                return Ok(value.as_bytes().map(ToOwned::to_owned));
            }
        }

        Ok(None)
    }

    pub fn delete(&mut self, key: Vec<u8>) -> Result<()> {
        let sequence_id = self.wal.next_sequence();
        self.wal
            .append(WalRecord::delete(sequence_id, key.clone()))?;
        self.memtable.delete(Key::new(key));

        self.rotate_memtable_if_needed()?;

        Ok(())
    }

    fn rotate_memtable_if_needed(&mut self) -> Result<()> {
        if self.memtable.approximate_size() >= self.config.memtable_threshold {
            let full_memtable = std::mem::take(&mut self.memtable);
            self.memtable_list.push_back(full_memtable);
            self.wal.new_wal();

            if self.memtable_list.len() > self.config.maximum_memtables {
                self.flush_memtable()?;
            }
        }

        Ok(())
    }

    pub fn flush_memtable(&mut self) -> Result<()> {
        let Some(memtable) = self.memtable_list.pop_front() else {
            return Ok(());
        };

        let path = self
            .config
            .data_dir
            .join(format!("{:020}.sst", self.next_sstable_id));
        let sstable =
            SsTableWriter::create(self.next_sstable_id, path)?.write_from(memtable.iter())?;

        self.next_sstable_id += 1;
        self.sstables.push(sstable);

        if let Some(wal) = self.wal.pop_oldest() {
            wal.remove_file()?;
        }

        Ok(())
    }

    pub fn sstable_count(&self) -> usize {
        self.sstables.len()
    }

    pub fn memtable_size(&self) -> usize {
        self.memtable.approximate_size()
    }

    pub fn immutable_memtable_count(&self) -> usize {
        self.memtable_list.len()
    }

    pub fn data_dir(&self) -> &Path {
        &self.config.data_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> EngineConfig {
        let mut config = EngineConfig::new(std::env::temp_dir().join("storage-engine-tests"));
        config.memtable_threshold = 8;
        config.maximum_memtables = 0;
        config
    }

    fn config_with_immutable_queue() -> EngineConfig {
        let mut config = EngineConfig::new(std::env::temp_dir().join("storage-engine-tests"));
        config.memtable_threshold = 8;
        config.maximum_memtables = 4;
        config
    }

    #[test]
    fn put_and_get_from_active_memtable() {
        let mut engine = Engine::new(config());
        engine.put(b"a".to_vec(), b"1".to_vec()).unwrap();

        assert_eq!(engine.get(b"a").unwrap(), Some(b"1".to_vec()));
    }

    #[test]
    fn flushes_filled_memtable_to_sstable_and_reads_from_it() {
        let mut engine = Engine::new(config());
        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();

        assert_eq!(engine.sstable_count(), 1);
        assert_eq!(engine.memtable_size(), 0);
        assert_eq!(engine.get(b"alpha").unwrap(), Some(b"one".to_vec()));
    }

    #[test]
    fn newest_value_wins_across_sstables() {
        let mut engine = Engine::new(config());
        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();
        engine.put(b"alpha".to_vec(), b"two".to_vec()).unwrap();

        assert_eq!(engine.sstable_count(), 2);
        assert_eq!(engine.get(b"alpha").unwrap(), Some(b"two".to_vec()));
    }

    #[test]
    fn delete_hides_older_values() {
        let mut engine = Engine::new(config());
        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();
        engine.delete(b"alpha".to_vec()).unwrap();

        assert_eq!(engine.get(b"alpha").unwrap(), None);
    }

    #[test]
    fn reads_from_immutable_memtable_before_flush() {
        let mut engine = Engine::new(config_with_immutable_queue());
        engine.put(b"alpha".to_vec(), b"one".to_vec()).unwrap();

        assert_eq!(engine.sstable_count(), 0);
        assert_eq!(engine.immutable_memtable_count(), 1);
        assert_eq!(engine.get(b"alpha").unwrap(), Some(b"one".to_vec()));
    }
}
