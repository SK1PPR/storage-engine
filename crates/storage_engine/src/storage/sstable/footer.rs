use crate::constants::SSTABLE_MAGIC;
use crate::format::{Decoder, Encoder};
use crate::{EngineError, Result};

pub const FOOTER_LEN: usize = std::mem::size_of::<u64>() * 5;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Footer {
    pub magic: u64,
    pub block_index_offset: u64,
    pub block_index_len: u64,
    pub bloom_offset: u64,
    pub bloom_len: u64,
}

impl Footer {
    pub fn new(
        block_index_offset: u64,
        block_index_len: u64,
        bloom_offset: u64,
        bloom_len: u64,
    ) -> Self {
        Self {
            magic: SSTABLE_MAGIC,
            block_index_offset,
            block_index_len,
            bloom_offset,
            bloom_len,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoder = Encoder::with_capacity(FOOTER_LEN);
        encoder.write_u64(self.magic);
        encoder.write_u64(self.block_index_offset);
        encoder.write_u64(self.block_index_len);
        encoder.write_u64(self.bloom_offset);
        encoder.write_u64(self.bloom_len);
        encoder.finish()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != FOOTER_LEN {
            return Err(EngineError::CorruptFormat("invalid footer length"));
        }

        let mut decoder = Decoder::new(bytes);
        let magic = decoder.read_u64()?;
        if magic != SSTABLE_MAGIC {
            return Err(EngineError::CorruptFormat("invalid footer magic"));
        }

        Ok(Self {
            magic,
            block_index_offset: decoder.read_u64()?,
            block_index_len: decoder.read_u64()?,
            bloom_offset: decoder.read_u64()?,
            bloom_len: decoder.read_u64()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_round_trip() {
        let footer = Footer::new(10, 20, 30, 40);

        assert_eq!(Footer::decode(&footer.encode()).unwrap(), footer);
    }
}
