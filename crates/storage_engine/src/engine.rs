use crate::{
    index::{skip_list::SkipList, Key, MemTable, Value},
    storage::sstable::{reader::read_from_table, writer::SsTableWriter, SsTable},
    wal::{WalRecord, WriteAheadLog},
    EngineConfig, Result,
};
use std::path::Path;

pub struct Engine {
    wal: WriteAheadLog,
    memtable: SkipList,
    sstables: Vec<SsTable>,
    next_sstable_id: u64,
    config: EngineConfig,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        let wal_path = config.data_dir.join("wal").join("active.log");

        Self {
            wal: WriteAheadLog::new(wal_path),
            memtable: SkipList::default(),
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

        if self.memtable.approximate_size() >= self.config.memtable_threshold {
            self.flush_memtable()?;
        }

        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let key = Key::new(key);

        if let Some(value) = self.memtable.get(&key) {
            return Ok(value.as_bytes().map(ToOwned::to_owned));
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

        if self.memtable.approximate_size() >= self.config.memtable_threshold {
            self.flush_memtable()?;
        }

        Ok(())
    }

    pub fn flush_memtable(&mut self) -> Result<()> {
        if self.memtable.is_empty() {
            return Ok(());
        }

        let path = self
            .config
            .data_dir
            .join(format!("{:020}.sst", self.next_sstable_id));
        let sstable =
            SsTableWriter::create(self.next_sstable_id, path)?.write_from(self.memtable.iter())?;

        self.next_sstable_id += 1;
        self.sstables.push(sstable);
        self.memtable = SkipList::default();
        self.wal.truncate()?;

        Ok(())
    }

    pub fn sstable_count(&self) -> usize {
        self.sstables.len()
    }

    pub fn memtable_size(&self) -> usize {
        self.memtable.approximate_size()
    }

    pub fn wal_records(&self) -> &[WalRecord] {
        self.wal.records()
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
}
