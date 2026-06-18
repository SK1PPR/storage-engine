use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    index::{skip_list::SkipList, Key, MemTable, Value},
    wal::{WalRecord, WriteAheadLog},
    EngineConfig,
};

pub struct Engine {
    wal: WriteAheadLog,
    memtable: SkipList,
    config: EngineConfig,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            wal: WriteAheadLog::default(),
            memtable: SkipList::default(),
            config,
        }
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let time = SystemTime::now();
        let timestamp = time
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_micros();

        self.wal.append(WalRecord::Put {
            key: key.clone(),
            value: value.clone(),
            timestamp,
        });

        self.memtable.put(Key::new(key), Value::Put(value));

        if self.memtable.approximate_size() >= self.config.memtable_threshold {
            // Next step: rotate this single memtable into an SSTable.
        }
    }
}
