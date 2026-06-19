use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::format::Decoder;
use crate::wal::record::WalRecord;
use crate::Result;

#[derive(Debug, Default)]
pub struct WriteAheadLog {
    write_path: Option<PathBuf>,
    sequence: u64,
    records: Vec<WalRecord>,
}

impl WriteAheadLog {
    pub fn new(write_path: impl Into<PathBuf>) -> Self {
        Self {
            write_path: Some(write_path.into()),
            sequence: 0,
            records: Vec::new(),
        }
    }

    pub fn append(&mut self, record: WalRecord) -> Result<()> {
        self.sequence = self.sequence.max(record.sequence_id());

        if let Some(path) = &self.write_path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut file = OpenOptions::new().create(true).append(true).open(path)?;
            file.write_all(&record.encode())?;
            file.sync_data()?;
        }

        self.records.push(record);
        Ok(())
    }

    pub fn next_sequence(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }

    pub fn records(&self) -> &[WalRecord] {
        &self.records
    }

    pub fn truncate(&mut self) -> Result<()> {
        self.records.clear();

        if let Some(path) = &self.write_path {
            File::create(path)?.sync_data()?;
        }

        Ok(())
    }

    pub fn replay(path: impl AsRef<Path>) -> Result<Vec<WalRecord>> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err.into()),
        };

        let mut decoder = Decoder::new(&bytes);
        let mut records = Vec::new();

        while !decoder.is_finished() {
            let record_len = decoder.read_u32()? as usize;
            let record_bytes = decoder.read_bytes(record_len)?;
            records.push(WalRecord::decode_payload(record_bytes)?);
        }

        Ok(records)
    }
}
