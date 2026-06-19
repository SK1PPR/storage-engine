use std::fs::{File, OpenOptions};
use std::io::{Seek, Write};
use std::path::PathBuf;

use crate::index::{Key, Value};
use crate::storage::bloom::BloomFilter;
use crate::storage::sstable::block::BlockBuilder;
use crate::storage::sstable::block_index::{BlockHandle, BlockIndex};
use crate::storage::sstable::footer::Footer;
use crate::storage::sstable::SsTable;
use crate::Result;

#[derive(Debug)]
pub struct SsTableWriter {
    id: u64,
    path: PathBuf,
    file: File,
    current_block: BlockBuilder,
    block_index: BlockIndex,
    bloom_filter: BloomFilter,
}

impl SsTableWriter {
    pub fn create(id: u64, path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
            .open(&path)?;

        Ok(Self {
            id,
            path,
            file,
            current_block: BlockBuilder::new(),
            block_index: BlockIndex::default(),
            bloom_filter: BloomFilter::default(),
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn write_from<'a, I>(mut self, entries: I) -> Result<SsTable>
    where
        I: IntoIterator<Item = (&'a Key, &'a Value)>,
    {
        for (key, value) in entries {
            self.add_entry(key, value)?;
        }

        self.finish()
    }

    fn add_entry(&mut self, key: &Key, value: &Value) -> Result<()> {
        if !self.current_block.can_add_value(key.as_bytes(), value)
            && !self.current_block.is_empty()
        {
            self.flush_current_block()?;
        }

        self.bloom_filter.add(key.as_bytes());
        self.current_block.add_value(key.as_bytes(), value);
        Ok(())
    }

    fn flush_current_block(&mut self) -> Result<()> {
        let block_builder = std::mem::take(&mut self.current_block);
        let Some(built_block) = block_builder.build() else {
            return Ok(());
        };

        let offset = self.file.stream_position()?;
        self.file.write_all(built_block.block.as_bytes())?;
        let len = built_block.block.as_bytes().len() as u64;

        self.block_index
            .push(Key::new(built_block.first_key), BlockHandle { offset, len });

        Ok(())
    }

    fn finish(mut self) -> Result<SsTable> {
        self.flush_current_block()?;

        let block_index_offset = self.file.stream_position()?;
        let block_index = self.block_index.encode();
        self.file.write_all(&block_index)?;

        let bloom_filter_offset = self.file.stream_position()?;
        let bloom_filter = self.bloom_filter.encode();
        self.file.write_all(&bloom_filter)?;

        let footer = Footer::new(
            block_index_offset,
            block_index.len() as u64,
            bloom_filter_offset,
            bloom_filter.len() as u64,
        );
        self.file.write_all(&footer.encode())?;
        self.file.sync_data()?;

        Ok(SsTable::new(self.id, self.path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{nanos}.sst"))
    }

    #[test]
    fn writes_sorted_entries_to_file() {
        let path = temp_path("storage-engine-writer");
        let entries = [
            (Key::new("a"), Value::put("1")),
            (Key::new("b"), Value::put("2")),
            (Key::new("c"), Value::Tombstone),
        ];

        let table = SsTableWriter::create(7, &path)
            .unwrap()
            .write_from(entries.iter().map(|(key, value)| (key, value)))
            .unwrap();

        assert_eq!(table.id(), 7);
        assert_eq!(table.path(), &path);
        assert!(std::fs::metadata(&path).unwrap().len() > 0);

        std::fs::remove_file(path).unwrap();
    }
}
