use std::path::{Path, PathBuf};

use crate::index::{Key, MemTable, Value};
use crate::storage::sstable::meta::SSTableMeta;
use crate::storage::sstable::reader::SsTableReader;
use crate::storage::sstable::writer::SsTableWriter;
use crate::Result;

#[derive(Debug)]
pub struct SSTableManager {
    data_dir: PathBuf,
    next_sstable_id: u64,
}

impl SSTableManager {
    pub fn new(data_dir: impl Into<PathBuf>, next_sstable_id: u64) -> Self {
        Self {
            data_dir: data_dir.into(),
            next_sstable_id,
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn next_sstable_id(&self) -> u64 {
        self.next_sstable_id
    }

    pub fn sstable_path(&self, file_id: u64) -> PathBuf {
        self.data_dir.join(format!("{file_id:020}.sst"))
    }

    pub fn write_from<'a, I>(&mut self, entries: I) -> Result<Option<SSTableMeta>>
    where
        I: IntoIterator<Item = (&'a Key, &'a Value)>,
    {
        let file_id = self.next_sstable_id;
        let path = self.sstable_path(file_id);
        let table = SsTableWriter::create(file_id, path)?.write_from(entries)?;
        self.next_sstable_id += 1;
        Ok(table.meta().cloned())
    }

    pub fn write_memtable<T: MemTable>(&mut self, memtable: &T) -> Result<Option<SSTableMeta>> {
        self.write_from(memtable.iter())
    }

    pub fn get(&self, key: &Key, sstables: &[SSTableMeta]) -> Result<Option<Value>> {
        for meta in sstables.iter().rev() {
            let reader = SsTableReader::open(meta.file_id(), self.sstable_path(meta.file_id()))?;
            if let Some(value) = reader.get(key)? {
                return Ok(Some(value));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{nanos}"))
    }

    #[test]
    fn writes_sstable_and_advances_next_id() {
        let dir = temp_dir("sstable-manager-write");
        std::fs::create_dir_all(&dir).unwrap();
        let mut manager = SSTableManager::new(&dir, 7);
        let entries = [(Key::new("a"), Value::put("1"))];

        let meta = manager
            .write_from(entries.iter().map(|(key, value)| (key, value)))
            .unwrap()
            .unwrap();

        assert_eq!(meta.file_id(), 7);
        assert_eq!(manager.next_sstable_id(), 8);
        assert!(manager.sstable_path(7).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reads_newest_value_from_manifest_ordered_sstables() {
        let dir = temp_dir("sstable-manager-read");
        std::fs::create_dir_all(&dir).unwrap();
        let mut manager = SSTableManager::new(&dir, 0);

        let old_entries = [(Key::new("alpha"), Value::put("one"))];
        let old_meta = manager
            .write_from(old_entries.iter().map(|(key, value)| (key, value)))
            .unwrap()
            .unwrap();
        let new_entries = [(Key::new("alpha"), Value::put("two"))];
        let new_meta = manager
            .write_from(new_entries.iter().map(|(key, value)| (key, value)))
            .unwrap()
            .unwrap();

        assert_eq!(
            manager
                .get(&Key::new("alpha"), &[old_meta, new_meta])
                .unwrap(),
            Some(Value::put("two"))
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
