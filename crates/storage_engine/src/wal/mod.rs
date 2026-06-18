#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalRecord {
    Put {
        key: Vec<u8>,
        value: Vec<u8>,
        timestamp: u128,
    },
    Delete {
        key: Vec<u8>,
        timestamp: u128,
    },
}

#[derive(Debug, Default)]
pub struct WriteAheadLog {
    records: Vec<WalRecord>,
}

impl WriteAheadLog {
    pub fn append(&mut self, record: WalRecord) {
        self.records.push(record);
    }

    pub fn records(&self) -> &[WalRecord] {
        &self.records
    }
}
