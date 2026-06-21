use crate::constants::{MANIFEST_RECORD_DELETE, MANIFEST_RECORD_MAGIC, MANIFEST_RECORD_PUT};
use crate::format::{Decoder, Encoder, Serializable};
use crate::storage::sstable::meta::SSTableMeta;
use crate::{EngineError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestRecord {
    AddSSTable(SSTableMeta),
    RemoveSSTable { file_id: u64 },
}

impl Serializable for ManifestRecord {
    fn encoded_len(&self) -> usize {
        8 + 1
            + match self {
                Self::AddSSTable(meta) => meta.encoded_len(),
                Self::RemoveSSTable { .. } => 8,
            }
    }

    fn encode_to(&self, encoder: &mut Encoder) {
        encoder.write_u64(MANIFEST_RECORD_MAGIC);

        match self {
            Self::AddSSTable(meta) => {
                encoder.write_u8(MANIFEST_RECORD_PUT);
                encoder.write_serializable(meta);
            }
            Self::RemoveSSTable { file_id } => {
                encoder.write_u8(MANIFEST_RECORD_DELETE);
                encoder.write_u64(*file_id);
            }
        }
    }

    fn decode_from(decoder: &mut Decoder<'_>) -> Result<Self> {
        let magic = decoder.read_u64()?;
        if magic != MANIFEST_RECORD_MAGIC {
            return Err(EngineError::CorruptFormat("invalid manifest record magic"));
        }

        match decoder.read_u8()? {
            MANIFEST_RECORD_PUT => Ok(Self::AddSSTable(decoder.read_serializable()?)),
            MANIFEST_RECORD_DELETE => Ok(Self::RemoveSSTable {
                file_id: decoder.read_u64()?,
            }),
            _ => Err(EngineError::CorruptFormat("unknown manifest record type")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_sstable_round_trips() {
        let record = ManifestRecord::AddSSTable(SSTableMeta::new(
            7,
            2,
            b"alpha".to_vec(),
            b"omega".to_vec(),
            4096,
        ));

        assert_eq!(ManifestRecord::decode(&record.encode()).unwrap(), record);
    }

    #[test]
    fn remove_sstable_round_trips() {
        let record = ManifestRecord::RemoveSSTable { file_id: 42 };

        assert_eq!(ManifestRecord::decode(&record.encode()).unwrap(), record);
    }
}
