pub mod block;
pub mod footer;
pub mod reader;
pub mod writer;

use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SsTable {
    id: u64,
    path: PathBuf,
}

impl SsTable {
    pub fn new(id: u64, path: impl Into<PathBuf>) -> Self {
        Self {
            id,
            path: path.into(),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
