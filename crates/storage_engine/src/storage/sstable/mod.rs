pub mod block;
pub mod block_index;
pub mod footer;
pub mod reader;
pub mod writer;

use crate::index::{Key, Value};
use crate::storage::bloom::BloomFilter;
use crate::storage::sstable::block_index::BlockIndex;
use crate::storage::sstable::footer::Footer;

use std::path::PathBuf;

#[derive(Debug)]
pub struct SsTable {
    id: u64,
    path: PathBuf,
    entries: Vec<(Key, Value)>,
    footer: Option<Footer>,
    block_index: Option<BlockIndex>,
    bloom_filter: Option<BloomFilter>,
}

impl SsTable {
    pub fn new(id: u64, path: impl Into<PathBuf>) -> Self {
        Self {
            id,
            path: path.into(),
            entries: Vec::new(),
            footer: None,
            block_index: None,
            bloom_filter: None,
        }
    }

    pub fn from_parts(
        id: u64,
        path: impl Into<PathBuf>,
        footer: Footer,
        block_index: BlockIndex,
        bloom_filter: BloomFilter,
    ) -> Self {
        Self {
            id,
            path: path.into(),
            entries: Vec::new(),
            footer: Some(footer),
            block_index: Some(block_index),
            bloom_filter: Some(bloom_filter),
        }
    }

    pub fn from_entries<I>(id: u64, path: impl Into<PathBuf>, entries: I) -> Self
    where
        I: IntoIterator<Item = (Key, Value)>,
    {
        let mut entries: Vec<(Key, Value)> = entries.into_iter().collect();
        entries.sort_by(|(left, _), (right, _)| left.cmp(right));

        Self {
            id,
            path: path.into(),
            entries,
            footer: None,
            block_index: None,
            bloom_filter: None,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn get(&self, key: &Key) -> Option<&Value> {
        self.entries
            .binary_search_by(|(candidate, _)| candidate.cmp(key))
            .ok()
            .map(|index| &self.entries[index].1)
    }

    pub fn footer(&self) -> Option<&Footer> {
        self.footer.as_ref()
    }

    pub fn block_index(&self) -> Option<&BlockIndex> {
        self.block_index.as_ref()
    }

    pub fn bloom_filter(&self) -> Option<&BloomFilter> {
        self.bloom_filter.as_ref()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Key, &Value)> {
        self.entries.iter().map(|(key, value)| (key, value))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
