pub const SSTABLE_MAGIC: u64 = 0x5353_5441_424c_4501;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Footer {
    pub index_offset: u64,
    pub index_len: u64,
    pub magic: u64,
}

impl Footer {
    pub fn new(index_offset: u64, index_len: u64) -> Self {
        Self {
            index_offset,
            index_len,
            magic: SSTABLE_MAGIC,
        }
    }
}
