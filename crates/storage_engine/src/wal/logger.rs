use std::collections::VecDeque;
use std::path::PathBuf;

use crate::wal::log::WriteAheadLog;
use crate::Result;

pub struct WriteAheadLogger {
    wals: VecDeque<WriteAheadLog>,
    current_unique_id: u64,
    sequence: u64,
    dir_path: PathBuf,
}

impl WriteAheadLogger {
    pub fn new(dir_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&dir_path)?;

        let mut logger = Self {
            wals: VecDeque::new(),
            current_unique_id: 0,
            sequence: 0,
            dir_path,
        };
        logger.new_wal();
        Ok(logger)
    }

    pub fn current_mut(&mut self) -> Option<&mut WriteAheadLog> {
        self.wals.back_mut()
    }

    pub fn append(&mut self, record: crate::wal::WalRecord) -> Result<()> {
        if self.wals.is_empty() {
            self.new_wal();
        }

        self.current_mut()
            .expect("WAL segment exists after new_wal")
            .append(record)
    }

    pub fn next_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    pub fn current_records(&self) -> &[crate::wal::WalRecord] {
        self.wals.back().map(WriteAheadLog::records).unwrap_or(&[])
    }

    pub fn record_count(&self) -> usize {
        self.wals.iter().map(|wal| wal.records().len()).sum()
    }

    pub fn new_wal(&mut self) {
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
