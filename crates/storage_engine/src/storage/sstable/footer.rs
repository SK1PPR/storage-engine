pub const SSTABLE_MAGIC: u64 = 0x5353_5441_424c_4501;
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

    pub fn encode(&self) -> [u8; FOOTER_LEN] {
        let mut bytes = [0; FOOTER_LEN];
        bytes[0..8].copy_from_slice(&self.magic.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.block_index_offset.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.block_index_len.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.bloom_offset.to_le_bytes());
        bytes[32..40].copy_from_slice(&self.bloom_len.to_le_bytes());
        bytes
    }
}
