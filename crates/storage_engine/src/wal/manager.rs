use std::collections::VecDeque;
use std::path::PathBuf;

use crate::wal::log::WriteAheadLog;
use crate::Result;

pub struct WalManager {
    wals: VecDeque<WriteAheadLog>,
    current_unique_id: u64,
    sequence: u64,
    dir_path: PathBuf,
}

impl WalManager {
    pub fn new(dir_path: PathBuf) -> Result<Self> {
        Self::open(dir_path, 0, 1)
    }

    pub fn open(dir_path: PathBuf, next_wal_id: u64, next_sequence_id: u64) -> Result<Self> {
        std::fs::create_dir_all(&dir_path)?;

        let mut manager = Self {
            wals: VecDeque::new(),
            current_unique_id: next_wal_id,
            sequence: next_sequence_id.saturating_sub(1),
            dir_path,
        };
        manager.rotate();
        Ok(manager)
    }

    pub fn current_mut(&mut self) -> Option<&mut WriteAheadLog> {
        self.wals.back_mut()
    }

    pub fn append(&mut self, record: crate::wal::WalRecord) -> Result<()> {
        if self.wals.is_empty() {
            self.rotate();
        }

        self.current_mut()
            .expect("WAL segment exists after rotate")
            .append(record)
    }

    pub fn next_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    pub fn next_wal_id(&self) -> u64 {
        self.current_unique_id
    }

    pub fn next_sequence_id(&self) -> u64 {
        self.sequence + 1
    }

    pub fn rotate(&mut self) {
        self.wals.push_back(WriteAheadLog::new(
            self.dir_path
                .join(format!("wal_{}.log", self.current_unique_id)),
        ));
        self.current_unique_id += 1;
    }

    pub fn pop_oldest(&mut self) -> Option<WriteAheadLog> {
        self.wals.pop_front()
    }

    pub fn len(&self) -> usize {
        self.wals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.wals.is_empty()
    }
}
