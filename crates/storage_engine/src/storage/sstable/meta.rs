use crate::format::{Decoder, Encoder, Serializable};
use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SSTableMeta {
    file_id: u64,
    level: u32,

    smallest_key: Vec<u8>,
    largest_key: Vec<u8>,

    file_size: u64,
}

impl SSTableMeta {
    pub fn new(
        file_id: u64,
        level: u32,
        smallest_key: Vec<u8>,
        largest_key: Vec<u8>,
        file_size: u64,
    ) -> Self {
        Self {
            file_id,
            level,
            smallest_key,
            largest_key,
            file_size,
        }
    }

    pub fn file_id(&self) -> u64 {
        self.file_id
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    pub fn smallest_key(&self) -> &[u8] {
        &self.smallest_key
    }

    pub fn largest_key(&self) -> &[u8] {
        &self.largest_key
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }
}

impl Serializable for SSTableMeta {
    fn encoded_len(&self) -> usize {
        8 + 4 + 4 + self.smallest_key.len() + 4 + self.largest_key.len() + 8
    }

    fn encode_to(&self, encoder: &mut Encoder) {
        encoder.write_u64(self.file_id);
        encoder.write_u32(self.level);
        encoder.write_u32(self.smallest_key.len() as u32);
        encoder.write_bytes(&self.smallest_key);
        encoder.write_u32(self.largest_key.len() as u32);
        encoder.write_bytes(&self.largest_key);
        encoder.write_u64(self.file_size);
    }

    fn decode_from(decoder: &mut Decoder<'_>) -> Result<Self> {
        let file_id = decoder.read_u64()?;
        let level = decoder.read_u32()?;
        let smallest_key_len = decoder.read_u32()? as usize;
        let smallest_key = decoder.read_bytes(smallest_key_len)?.to_vec();
        let largest_key_len = decoder.read_u32()? as usize;
        let largest_key = decoder.read_bytes(largest_key_len)?.to_vec();
        let file_size = decoder.read_u64()?;

        Ok(Self {
            file_id,
            level,
            smallest_key,
            largest_key,
            file_size,
        })
    }
}
