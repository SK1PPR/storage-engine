use crate::index::Key;
use crate::storage::format::{Decoder, Encoder};
use crate::{EngineError, Result};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BlockHandle {
    pub offset: u64,
    pub len: u64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BlockIndexEntry {
    pub first_key: Key,
    pub handle: BlockHandle,
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct BlockIndex {
    entries: Vec<BlockIndexEntry>,
}

impl BlockIndex {
    pub fn push(&mut self, first_key: Key, handle: BlockHandle) {
        self.entries.push(BlockIndexEntry { first_key, handle });
    }

    pub fn entries(&self) -> &[BlockIndexEntry] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoder = Encoder::new();
        encoder.write_u32(self.entries.len() as u32);

        for entry in &self.entries {
            let key = entry.first_key.as_bytes();
            encoder.write_u32(key.len() as u32);
            encoder.write_bytes(key);
            encoder.write_u64(entry.handle.offset);
            encoder.write_u64(entry.handle.len);
        }

        encoder.finish()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut decoder = Decoder::new(bytes);
        let entry_count = decoder.read_u32()? as usize;
        let mut entries = Vec::with_capacity(entry_count);

        for _ in 0..entry_count {
            let key_len = decoder.read_u32()? as usize;
            let first_key = Key::new(decoder.read_bytes(key_len)?.to_vec());
            let offset = decoder.read_u64()?;
            let len = decoder.read_u64()?;
            entries.push(BlockIndexEntry {
                first_key,
                handle: BlockHandle { offset, len },
            });
        }

        if !decoder.is_finished() {
            return Err(EngineError::CorruptFormat("trailing block index bytes"));
        }

        Ok(Self { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_round_trip() {
        let mut index = BlockIndex::default();
        index.push(
            Key::new("a"),
            BlockHandle {
                offset: 10,
                len: 20,
            },
        );
        index.push(
            Key::new("m"),
            BlockHandle {
                offset: 30,
                len: 40,
            },
        );

        let decoded = BlockIndex::decode(&index.encode()).unwrap();

        assert_eq!(decoded, index);
    }
}
