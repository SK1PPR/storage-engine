pub mod block;
pub mod block_index;
pub mod footer;
pub mod reader;
pub mod writer;

use crate::index::{Key, Value};

use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SsTable {
    id: u64,
    path: PathBuf,
    entries: Vec<(Key, Value)>,
}

impl SsTable {
    pub fn new(id: u64, path: impl Into<PathBuf>) -> Self {
        Self {
            id,
            path: path.into(),
            entries: Vec::new(),
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
