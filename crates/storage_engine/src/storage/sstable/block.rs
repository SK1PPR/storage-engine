use crate::constants::{
    SSTABLE_DEFAULT_BLOCK_LEN, SSTABLE_VALUE_KIND_PUT, SSTABLE_VALUE_KIND_TOMBSTONE,
};
use crate::format::Decoder;
use crate::index::{Key, Value};
use crate::Result;

const ENTRY_HEADER_LEN: usize = std::mem::size_of::<u32>() + 1 + std::mem::size_of::<u32>();

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Block {
    bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct BlockBuilder {
    first_key: Option<Vec<u8>>,
    bytes: Vec<u8>,
    entry_count: u32,
    max_len: usize,
}

impl Default for BlockBuilder {
    fn default() -> Self {
        Self {
            first_key: None,
            bytes: 0_u32.to_le_bytes().to_vec(),
            entry_count: 0,
            max_len: SSTABLE_DEFAULT_BLOCK_LEN,
        }
    }
}

impl BlockBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn can_add(&self, key: &[u8], value: &[u8]) -> bool {
        self.bytes.len() + entry_len(key, value) <= self.max_len
    }

    pub fn add(&mut self, key: &[u8], value: &[u8]) {
        self.add_encoded(key, SSTABLE_VALUE_KIND_PUT, value);
    }

    pub fn can_add_value(&self, key: &[u8], value: &Value) -> bool {
        self.bytes.len() + value_entry_len(key, value) <= self.max_len
    }

    pub fn add_value(&mut self, key: &[u8], value: &Value) {
        match value {
            Value::Put(bytes) => self.add_encoded(key, SSTABLE_VALUE_KIND_PUT, bytes),
            Value::Tombstone => self.add_encoded(key, SSTABLE_VALUE_KIND_TOMBSTONE, &[]),
        }
    }

    fn add_encoded(&mut self, key: &[u8], value_kind: u8, value: &[u8]) {
        if self.first_key.is_none() {
            self.first_key = Some(key.to_vec());
        }

        self.bytes
            .extend_from_slice(&(key.len() as u32).to_le_bytes());
        self.bytes.push(value_kind);
        self.bytes
            .extend_from_slice(&(value.len() as u32).to_le_bytes());
        self.bytes.extend_from_slice(key);
        self.bytes.extend_from_slice(value);
        self.entry_count += 1;
    }

    pub fn build(mut self) -> Option<BuiltBlock> {
        let first_key = self.first_key?;
        self.bytes[0..4].copy_from_slice(&self.entry_count.to_le_bytes());

        Some(BuiltBlock {
            first_key,
            block: Block { bytes: self.bytes },
        })
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entry_count == 0
    }
}

impl Block {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn get(&self, key: &Key) -> Result<Option<Value>> {
        let mut decoder = Decoder::new(&self.bytes);
        let entry_count = decoder.read_u32()?;

        for _ in 0..entry_count {
            let key_len = decoder.read_u32()? as usize;
            let value_kind = decoder.read_u8()?;
            let value_len = decoder.read_u32()? as usize;
            let entry_key = decoder.read_bytes(key_len)?;
            let value_bytes = decoder.read_bytes(value_len)?;

            if entry_key == key.as_bytes() {
                return Ok(Some(match value_kind {
                    SSTABLE_VALUE_KIND_PUT => Value::Put(value_bytes.to_vec()),
                    SSTABLE_VALUE_KIND_TOMBSTONE => Value::Tombstone,
                    _ => return Err(crate::EngineError::CorruptFormat("unknown value kind")),
                }));
            }

            if entry_key > key.as_bytes() {
                return Ok(None);
            }
        }

        Ok(None)
    }

    pub fn entries(&self) -> Result<Vec<(Key, Value)>> {
        let mut decoder = Decoder::new(&self.bytes);
        let entry_count = decoder.read_u32()?;
        let mut entries = Vec::with_capacity(entry_count as usize);

        for _ in 0..entry_count {
            let key_len = decoder.read_u32()? as usize;
            let value_kind = decoder.read_u8()?;
            let value_len = decoder.read_u32()? as usize;
            let key = Key::new(decoder.read_bytes(key_len)?.to_vec());
            let value_bytes = decoder.read_bytes(value_len)?;

            let value = match value_kind {
                SSTABLE_VALUE_KIND_PUT => Value::Put(value_bytes.to_vec()),
                SSTABLE_VALUE_KIND_TOMBSTONE => Value::Tombstone,
                _ => return Err(crate::EngineError::CorruptFormat("unknown value kind")),
            };

            entries.push((key, value));
        }

        Ok(entries)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BuiltBlock {
    pub first_key: Vec<u8>,
    pub block: Block,
}

fn entry_len(key: &[u8], value: &[u8]) -> usize {
    ENTRY_HEADER_LEN + key.len() + value.len()
}

fn value_entry_len(key: &[u8], value: &Value) -> usize {
    match value {
        Value::Put(bytes) => entry_len(key, bytes),
        Value::Tombstone => entry_len(key, &[]),
    }
}
