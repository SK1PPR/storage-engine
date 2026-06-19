use crate::constants::{WAL_CHECKSUM_SEED, WAL_RECORD_DELETE, WAL_RECORD_PUT};
use crate::format::{Decoder, Encoder};
use crate::{EngineError, Result};
use twox_hash::XxHash3_64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalRecord {
    Put {
        sequence_id: u64,
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Delete {
        sequence_id: u64,
        key: Vec<u8>,
    },
}

impl WalRecord {
    pub fn put(sequence_id: u64, key: Vec<u8>, value: Vec<u8>) -> Self {
        Self::Put {
            sequence_id,
            key,
            value,
        }
    }

    pub fn delete(sequence_id: u64, key: Vec<u8>) -> Self {
        Self::Delete { sequence_id, key }
    }

    pub fn sequence_id(&self) -> u64 {
        match self {
            Self::Put { sequence_id, .. } | Self::Delete { sequence_id, .. } => *sequence_id,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut payload = Encoder::new();

        match self {
            Self::Put {
                sequence_id,
                key,
                value,
            } => {
                payload.write_u8(WAL_RECORD_PUT);
                payload.write_u64(*sequence_id);
                payload.write_u32(key.len() as u32);
                payload.write_u32(value.len() as u32);
                payload.write_bytes(key);
                payload.write_bytes(value);
            }
            Self::Delete { sequence_id, key } => {
                payload.write_u8(WAL_RECORD_DELETE);
                payload.write_u64(*sequence_id);
                payload.write_u32(key.len() as u32);
                payload.write_u32(0);
                payload.write_bytes(key);
            }
        }

        let payload = payload.finish();
        let mut record = Encoder::new();
        record.write_u32(payload.len() as u32);
        record.write_u64(checksum(&payload));
        record.write_bytes(&payload);
        record.finish()
    }

    pub(crate) fn decode_payload(bytes: &[u8]) -> Result<Self> {
        let mut decoder = Decoder::new(bytes);
        let record_type = decoder.read_u8()?;
        let sequence_id = decoder.read_u64()?;
        let key_len = decoder.read_u32()? as usize;
        let value_len = decoder.read_u32()? as usize;
        let key = decoder.read_bytes(key_len)?.to_vec();

        let record = match record_type {
            WAL_RECORD_PUT => {
                let value = decoder.read_bytes(value_len)?.to_vec();
                Self::Put {
                    sequence_id,
                    key,
                    value,
                }
            }
            WAL_RECORD_DELETE => {
                if value_len != 0 {
                    return Err(EngineError::CorruptWal("delete record has value bytes"));
                }
                Self::Delete { sequence_id, key }
            }
            _ => return Err(EngineError::CorruptWal("unknown record type")),
        };

        if !decoder.is_finished() {
            return Err(EngineError::CorruptWal("trailing record bytes"));
        }

        Ok(record)
    }
}

pub(crate) fn checksum(bytes: &[u8]) -> u64 {
    XxHash3_64::oneshot_with_seed(WAL_CHECKSUM_SEED, bytes)
}
