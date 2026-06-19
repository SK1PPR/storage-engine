use std::path::PathBuf;

#[derive(Debug)]
pub struct SsTableReader {
    path: PathBuf,
}

impl SsTableReader {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
