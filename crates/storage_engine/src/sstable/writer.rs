use std::path::PathBuf;

#[derive(Debug)]
pub struct SsTableWriter {
    path: PathBuf,
}

impl SsTableWriter {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}
