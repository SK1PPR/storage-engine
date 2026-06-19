use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::format::Decoder;
use crate::wal::record::{checksum, WalRecord};
use crate::{EngineError, Result};

#[derive(Debug, Default)]
pub struct WriteAheadLog {
    write_path: Option<PathBuf>,
    file: Option<File>,
    sequence: u64,
    records: Vec<WalRecord>,
}

impl WriteAheadLog {
    pub fn new(write_path: impl Into<PathBuf>) -> Self {
        Self {
            write_path: Some(write_path.into()),
            file: None,
            sequence: 0,
            records: Vec::new(),
        }
    }

    pub fn append(&mut self, record: WalRecord) -> Result<()> {
        self.sequence = self.sequence.max(record.sequence_id());

        if self.write_path.is_some() {
            let file = self.file_mut()?;
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

    pub fn path(&self) -> Option<&Path> {
        self.write_path.as_deref()
    }

    pub fn truncate(&mut self) -> Result<()> {
        self.records.clear();

        if let Some(path) = &self.write_path {
            if let Some(file) = &mut self.file {
                file.set_len(0)?;
                file.sync_data()?;
            } else {
                File::create(path)?.sync_data()?;
            }
        }

        Ok(())
    }

    pub fn remove_file(mut self) -> Result<()> {
        self.file = None;

        if let Some(path) = &self.write_path {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(err.into()),
            }
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
            let expected_checksum = decoder.read_u64()?;
            let record_bytes = decoder.read_bytes(record_len)?;
            let actual_checksum = checksum(record_bytes);
            if actual_checksum != expected_checksum {
                return Err(EngineError::CorruptWal("checksum mismatch"));
            }
            records.push(WalRecord::decode_payload(record_bytes)?);
        }

        Ok(records)
    }

    fn file_mut(&mut self) -> Result<&mut File> {
        if self.file.is_none() {
            let path = self
                .write_path
                .as_ref()
                .expect("checked by caller before opening WAL file");
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            self.file = Some(OpenOptions::new().create(true).append(true).open(path)?);
        }

        Ok(self.file.as_mut().expect("WAL file was opened above"))
    }
}
