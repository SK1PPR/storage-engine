use std::path::{Path, PathBuf};

use crate::{EngineError, Result};

const CURRENT_FILE: &str = "CURRENT";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CurrentState {
    pub next_sstable_id: u64,
    pub next_wal_id: u64,
    pub next_manifest_id: u64,
    pub next_sequence_id: u64,
}

#[derive(Debug)]
pub struct CurrentFile {
    path: PathBuf,
    state: CurrentState,
}

impl Default for CurrentState {
    fn default() -> Self {
        Self {
            next_sstable_id: 0,
            next_wal_id: 0,
            next_manifest_id: 0,
            next_sequence_id: 1,
        }
    }
}

impl CurrentFile {
    pub fn open(data_dir: impl Into<PathBuf>) -> Result<Self> {
        let data_dir = data_dir.into();
        std::fs::create_dir_all(&data_dir)?;
        let path = data_dir.join(CURRENT_FILE);
        let state = match std::fs::read_to_string(&path) {
            Ok(contents) => CurrentState::decode(&contents)?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => CurrentState::default(),
            Err(err) => return Err(err.into()),
        };

        Ok(Self { path, state })
    }

    pub fn state(&self) -> &CurrentState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut CurrentState {
        &mut self.state
    }

    pub fn persist(&self) -> Result<()> {
        let tmp_path = self.path.with_extension("tmp");
        std::fs::write(&tmp_path, self.state.encode())?;
        std::fs::rename(tmp_path, &self.path)?;
        Ok(())
    }
}

impl CurrentState {
    fn encode(&self) -> String {
        format!(
            "next_sstable_id={}\nnext_wal_id={}\nnext_manifest_id={}\nnext_sequence_id={}\n",
            self.next_sstable_id, self.next_wal_id, self.next_manifest_id, self.next_sequence_id
        )
    }

    fn decode(contents: &str) -> Result<Self> {
        let mut state = Self::default();

        for line in contents.lines() {
            let Some((key, value)) = line.split_once('=') else {
                return Err(EngineError::CorruptFormat("invalid CURRENT line"));
            };
            let value = value
                .parse::<u64>()
                .map_err(|_| EngineError::CorruptFormat("invalid CURRENT value"))?;

            match key {
                "next_sstable_id" => state.next_sstable_id = value,
                "next_wal_id" => state.next_wal_id = value,
                "next_manifest_id" => state.next_manifest_id = value,
                "next_sequence_id" => state.next_sequence_id = value,
                _ => return Err(EngineError::CorruptFormat("unknown CURRENT key")),
            }
        }

        Ok(state)
    }
}

pub fn current_path(data_dir: &Path) -> PathBuf {
    data_dir.join(CURRENT_FILE)
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
    fn persists_and_loads_current_state() {
        let dir = temp_dir("current-file");
        let mut current = CurrentFile::open(&dir).unwrap();
        current.state_mut().next_sstable_id = 7;
        current.state_mut().next_wal_id = 3;
        current.state_mut().next_manifest_id = 2;
        current.state_mut().next_sequence_id = 11;
        current.persist().unwrap();

        let loaded = CurrentFile::open(&dir).unwrap();

        assert_eq!(
            loaded.state(),
            &CurrentState {
                next_sstable_id: 7,
                next_wal_id: 3,
                next_manifest_id: 2,
                next_sequence_id: 11,
            }
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
