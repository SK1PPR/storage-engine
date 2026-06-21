use std::hash::Hasher;

use crate::constants::{WAL_CHECKSUM_SEED, WAL_RECORD_DELETE, WAL_RECORD_PUT};
use crate::format::{Decoder, Encoder, Serializable};
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

    fn payload_len(&self) -> usize {
        match self {
            Self::Put { key, value, .. } => 1 + 8 + 4 + 4 + key.len() + value.len(),
            Self::Delete { key, .. } => 1 + 8 + 4 + 4 + key.len(),
        }
    }

    fn encode_payload_to(&self, payload: &mut Encoder) {
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
    }

    pub(crate) fn decode_payload(bytes: &[u8]) -> Result<Self> {
        let mut decoder = Decoder::new(bytes);
        let record = Self::decode_payload_from(&mut decoder)?;

        if !decoder.is_finished() {
            return Err(EngineError::CorruptWal("trailing record bytes"));
        }

        Ok(record)
    }

    fn decode_payload_from(decoder: &mut Decoder<'_>) -> Result<Self> {
        let record_type = decoder.read_u8()?;
        let sequence_id = decoder.read_u64()?;
        let key_len = decoder.read_u32()? as usize;
        let value_len = decoder.read_u32()? as usize;
        let key = decoder.read_bytes(key_len)?.to_vec();

        match record_type {
            WAL_RECORD_PUT => {
                let value = decoder.read_bytes(value_len)?.to_vec();
                Ok(Self::Put {
                    sequence_id,
                    key,
                    value,
                })
            }
            WAL_RECORD_DELETE => {
                if value_len != 0 {
                    return Err(EngineError::CorruptWal("delete record has value bytes"));
                }
                Ok(Self::Delete { sequence_id, key })
            }
            _ => Err(EngineError::CorruptWal("unknown record type")),
        }
    }

    fn checksum_payload(&self) -> u64 {
        let mut hasher = XxHash3_64::with_seed(WAL_CHECKSUM_SEED);
        match self {
            Self::Put {
                sequence_id,
                key,
                value,
            } => {
                hasher.write(&[WAL_RECORD_PUT]);
                hasher.write(&sequence_id.to_le_bytes());
                hasher.write(&(key.len() as u32).to_le_bytes());
                hasher.write(&(value.len() as u32).to_le_bytes());
                hasher.write(key);
                hasher.write(value);
            }
            Self::Delete { sequence_id, key } => {
                hasher.write(&[WAL_RECORD_DELETE]);
                hasher.write(&sequence_id.to_le_bytes());
                hasher.write(&(key.len() as u32).to_le_bytes());
                hasher.write(&0u32.to_le_bytes());
                hasher.write(key);
            }
        }
        hasher.finish()
    }
}

impl Serializable for WalRecord {
    fn encoded_len(&self) -> usize {
        4 + 8 + self.payload_len()
    }

    fn encode_to(&self, encoder: &mut Encoder) {
        encoder.write_u32(self.payload_len() as u32);
        encoder.write_u64(self.checksum_payload());
        self.encode_payload_to(encoder);
    }

    fn decode_from(decoder: &mut Decoder<'_>) -> Result<Self> {
        let record_len = decoder.read_u32()? as usize;
        let expected_checksum = decoder.read_u64()?;
        let record_bytes = decoder.read_bytes(record_len)?;
        let actual_checksum = checksum(record_bytes);
        if actual_checksum != expected_checksum {
            return Err(EngineError::CorruptWal("checksum mismatch"));
        }

        Self::decode_payload(record_bytes)
    }
}

pub(crate) fn checksum(bytes: &[u8]) -> u64 {
    XxHash3_64::oneshot_with_seed(WAL_CHECKSUM_SEED, bytes)
}
