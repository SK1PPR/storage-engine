use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub data_dir: PathBuf,
    pub memtable_threshold: usize,
}

impl EngineConfig {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            memtable_threshold: 4096,
        }
    }
}
