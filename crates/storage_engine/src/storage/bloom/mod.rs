pub mod hash;

use hash::XxHash3Impl;

use crate::format::{Decoder, Encoder};
use crate::storage::bloom::hash::HashFunc;
use crate::{EngineError, Result};

const BLOOM_MAGIC: u64 = 13794972908406357291;

#[derive(Debug)]
pub struct BloomFilter {
    bitset: Vec<u64>,
    size: u64,
    hashes: Vec<XxHash3Impl>,
}

impl Default for BloomFilter {
    fn default() -> Self {
        Self {
            bitset: vec![0; 4],
            size: 256,
            hashes: vec![XxHash3Impl::default(); 4],
        }
    }
}

impl BloomFilter {
    pub fn new(size: u64, hashes: Vec<XxHash3Impl>) -> Self {
        Self {
            bitset: vec![0; word_count(size)],
            size,
            hashes,
        }
    }

    fn set_bit(&mut self, pos: usize) {
        let word = pos / 64;
        let bit = pos % 64;
        self.bitset[word] |= 1 << bit;
    }

    fn has_bit(&self, pos: usize) -> bool {
        let word = pos / 64;
        let bit = pos % 64;
        (self.bitset[word] & (1 << bit)) != 0
    }

    pub fn add(&mut self, member: impl AsRef<[u8]>) {
        let member = member.as_ref();
        let size = self.size;
        let positions = self
            .hashes
            .iter()
            .map(|hash| (hash.encode(member) % size) as usize);

        for pos in positions.collect::<Vec<_>>() {
            self.set_bit(pos);
        }
    }

    pub fn contains(&self, member: &[u8]) -> bool {
        self.hashes
            .iter()
            .all(|hf| self.has_bit((hf.encode(member) % self.size) as usize))
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut encoder = Encoder::new();
        encoder.write_u64(BLOOM_MAGIC);
        encoder.write_u64(self.size);
        encoder.write_u32(self.hashes.len() as u32);

        for hash in &self.hashes {
            encoder.write_u64(hash.seed());
        }

        let byte_count = self.size.div_ceil(8) as usize;
        for i in 0..byte_count {
            let word = i / 8;
            let byte = i % 8;
            encoder.write_u8((self.bitset[word] >> (byte * 8)) as u8);
        }

        encoder.finish()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut decoder = Decoder::new(bytes);
        let magic = decoder.read_u64()?;
        if magic != BLOOM_MAGIC {
            return Err(EngineError::CorruptBloomFilter("invalid magic"));
        }

        let size = decoder.read_u64()?;
        if size == 0 {
            return Err(EngineError::CorruptBloomFilter("zero-sized filter"));
        }

        let hash_count = decoder.read_u32()? as usize;
        if hash_count == 0 {
            return Err(EngineError::CorruptBloomFilter("no hash functions"));
        }

        let mut hashes = Vec::with_capacity(hash_count);
        for _ in 0..hash_count {
            hashes.push(XxHash3Impl::from_seed(decoder.read_u64()?));
        }

        let byte_count = size.div_ceil(8) as usize;
        if decoder.remaining() != byte_count {
            return Err(EngineError::CorruptBloomFilter("unexpected bitset length"));
        }

        let mut bitset = vec![0; word_count(size)];
        for i in 0..byte_count {
            let word = i / 8;
            let byte = i % 8;
            bitset[word] |= (decoder.read_u8()? as u64) << (byte * 8);
        }

        Ok(Self {
            bitset,
            size,
            hashes,
        })
    }
}

fn word_count(size: u64) -> usize {
    size.div_ceil(64) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_member_is_probably_contained() {
        let mut filter = BloomFilter::default();

        filter.add(b"alpha");

        assert!(filter.contains(b"alpha"));
    }

    #[test]
    fn add_accepts_owned_and_borrowed_bytes() {
        let mut filter = BloomFilter::default();
        let owned = b"beta".to_vec();

        filter.add(owned.clone());
        filter.add(&owned);
        filter.add(owned.as_slice());

        assert!(filter.contains(b"beta"));
    }

    #[test]
    fn decode_round_trips_filter() {
        let mut filter = BloomFilter::default();
        filter.add(b"alpha");
        filter.add(b"beta");

        let decoded = BloomFilter::decode(&filter.encode()).unwrap();

        assert!(decoded.contains(b"alpha"));
        assert!(decoded.contains(b"beta"));
    }

    #[test]
    fn encode_stores_only_required_bitset_bytes() {
        let filter = BloomFilter::new(120, vec![XxHash3Impl::from_seed(1)]);
        let bytes = filter.encode();

        assert_eq!(bytes.len(), 8 + 8 + 4 + 8 + 15);
    }
}
