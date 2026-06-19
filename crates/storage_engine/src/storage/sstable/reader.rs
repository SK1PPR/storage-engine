use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use crate::index::{Key, Value};
use crate::storage::bloom::BloomFilter;
use crate::storage::sstable::block::Block;
use crate::storage::sstable::block_index::BlockIndex;
use crate::storage::sstable::footer::{Footer, FOOTER_LEN};
use crate::storage::sstable::SsTable;
use crate::{EngineError, Result};

#[derive(Debug)]
pub struct SsTableReader {
    table: SsTable,
}

impl SsTableReader {
    pub fn open(id: u64, path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let mut file = File::open(&path)?;
        let file_len = file.metadata()?.len();
        if file_len < FOOTER_LEN as u64 {
            return Err(EngineError::CorruptFormat("sstable file is too small"));
        }

        file.seek(SeekFrom::End(-(FOOTER_LEN as i64)))?;
        let mut footer_bytes = vec![0; FOOTER_LEN];
        file.read_exact(&mut footer_bytes)?;
        let footer = Footer::decode(&footer_bytes)?;

        let block_index_bytes =
            read_section(&mut file, footer.block_index_offset, footer.block_index_len)?;
        let block_index = BlockIndex::decode(&block_index_bytes)?;

        let bloom_bytes = read_section(&mut file, footer.bloom_offset, footer.bloom_len)?;
        let bloom_filter = BloomFilter::decode(&bloom_bytes)?;

        Ok(Self {
            table: SsTable::from_parts(id, path, footer, block_index, bloom_filter),
        })
    }

    pub fn path(&self) -> &PathBuf {
        self.table.path()
    }

    pub fn into_table(self) -> SsTable {
        self.table
    }

    pub fn get(&self, key: &Key) -> Result<Option<Value>> {
        read_from_table(&self.table, key)
    }

    pub fn iter(&self) -> Result<Vec<(Key, Value)>> {
        iter_table(&self.table)
    }
}

pub fn read_from_table(table: &SsTable, key: &Key) -> Result<Option<Value>> {
    if let Some(bloom_filter) = table.bloom_filter() {
        if !bloom_filter.contains(key.as_bytes()) {
            return Ok(None);
        }
    }

    let Some(block_index) = table.block_index() else {
        return Ok(table.get(key).cloned());
    };
    let Some(entry) = block_index.find_block(key) else {
        return Ok(None);
    };

    let mut file = File::open(table.path())?;
    let block_bytes = read_section(&mut file, entry.handle.offset, entry.handle.len)?;
    Block::from_bytes(block_bytes).get(key)
}

pub fn iter_table(table: &SsTable) -> Result<Vec<(Key, Value)>> {
    if table.block_index().is_none() {
        return Ok(table
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect());
    }

    let mut file = File::open(table.path())?;
    let mut entries = Vec::new();

    for index_entry in table.block_index().expect("checked above").entries() {
        let block_bytes =
            read_section(&mut file, index_entry.handle.offset, index_entry.handle.len)?;
        entries.extend(Block::from_bytes(block_bytes).entries()?);
    }

    Ok(entries)
}

fn read_section(file: &mut File, offset: u64, len: u64) -> Result<Vec<u8>> {
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = vec![0; len as usize];
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::sstable::writer::SsTableWriter;

    fn temp_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{nanos}.sst"))
    }

    #[test]
    fn write_creates_sstable_file() {
        let path = temp_path("sstable-reader");
        let entries = [
            (Key::new("a"), Value::put("1")),
            (Key::new("b"), Value::put("2")),
            (Key::new("c"), Value::Tombstone),
        ];
        SsTableWriter::create(1, &path)
            .unwrap()
            .write_from(entries.iter().map(|(key, value)| (key, value)))
            .unwrap();

        assert!(std::fs::metadata(&path).unwrap().len() > 0);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn reopen_and_get_returns_existing_value() {
        let path = temp_path("sstable-reader-get");
        let entries = [
            (Key::new("a"), Value::put("1")),
            (Key::new("b"), Value::put("2")),
        ];
        SsTableWriter::create(1, &path)
            .unwrap()
            .write_from(entries.iter().map(|(key, value)| (key, value)))
            .unwrap();

        let reader = SsTableReader::open(1, &path).unwrap();

        assert_eq!(reader.get(&Key::new("a")).unwrap(), Some(Value::put("1")));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn miss_returns_none() {
        let path = temp_path("sstable-reader-miss");
        let entries = [(Key::new("a"), Value::put("1"))];
        SsTableWriter::create(1, &path)
            .unwrap()
            .write_from(entries.iter().map(|(key, value)| (key, value)))
            .unwrap();

        let reader = SsTableReader::open(1, &path).unwrap();

        assert_eq!(reader.get(&Key::new("missing")).unwrap(), None);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn tombstone_is_preserved() {
        let path = temp_path("sstable-reader-tombstone");
        let entries = [
            (Key::new("a"), Value::put("1")),
            (Key::new("c"), Value::Tombstone),
        ];
        SsTableWriter::create(1, &path)
            .unwrap()
            .write_from(entries.iter().map(|(key, value)| (key, value)))
            .unwrap();

        let reader = SsTableReader::open(1, &path).unwrap();

        assert_eq!(reader.get(&Key::new("c")).unwrap(), Some(Value::Tombstone));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn sorted_iteration_returns_entries_in_order() {
        let path = temp_path("sstable-reader-iter");
        let entries = [
            (Key::new("a"), Value::put("1")),
            (Key::new("b"), Value::put("2")),
            (Key::new("c"), Value::Tombstone),
        ];
        SsTableWriter::create(1, &path)
            .unwrap()
            .write_from(entries.iter().map(|(key, value)| (key, value)))
            .unwrap();

        let reader = SsTableReader::open(1, &path).unwrap();
        let keys: Vec<Vec<u8>> = reader
            .iter()
            .unwrap()
            .into_iter()
            .map(|(key, _)| key.as_bytes().to_vec())
            .collect();

        assert_eq!(keys, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
        std::fs::remove_file(path).unwrap();
    }
}
