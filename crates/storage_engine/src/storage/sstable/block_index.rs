use crate::index::Key;

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
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());

        for entry in &self.entries {
            let key = entry.first_key.as_bytes();
            bytes.extend_from_slice(&(key.len() as u32).to_le_bytes());
            bytes.extend_from_slice(key);
            bytes.extend_from_slice(&entry.handle.offset.to_le_bytes());
            bytes.extend_from_slice(&entry.handle.len.to_le_bytes());
        }

        bytes
    }
}
