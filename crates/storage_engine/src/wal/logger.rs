use std::collections::VecDeque;
use std::path::PathBuf;

use crate::wal::log::WriteAheadLog;
use crate::Result;

pub struct WriteAheadLogger {
    wals: VecDeque<WriteAheadLog>,
    current_unique_id: u64,
    dir_path: PathBuf,
}

impl WriteAheadLogger {
    pub fn new(dir_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&dir_path)?;

        let mut logger = Self {
            wals: VecDeque::new(),
            current_unique_id: 0,
            dir_path,
        };
        logger.new_wal();
        Ok(logger)
    }

    pub fn current_mut(&mut self) -> Option<&mut WriteAheadLog> {
        self.wals.back_mut()
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
