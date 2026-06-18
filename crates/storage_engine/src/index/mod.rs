pub mod btree;
pub mod skip_list;

const VALUE_KIND_SIZE: usize = 1;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Default)]
pub struct Key(Vec<u8>);

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Value {
    Put(Vec<u8>),
    Tombstone,
}

pub trait MemTable {
    fn put(&mut self, key: Key, value: Value);
    fn get(&self, key: &Key) -> Option<&Value>;
    fn delete(&mut self, key: Key);
    fn iter(&self) -> Box<dyn Iterator<Item = (&Key, &Value)> + '_>;
    fn approximate_size(&self) -> usize;
    fn is_empty(&self) -> bool;
}

impl Key {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn approximate_size(&self) -> usize {
        self.0.len()
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Tombstone
    }
}

impl Value {
    pub fn put(bytes: impl Into<Vec<u8>>) -> Self {
        Self::Put(bytes.into())
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Put(bytes) => Some(bytes),
            Self::Tombstone => None,
        }
    }

    pub fn approximate_size(&self) -> usize {
        match self {
            Self::Put(val) => VALUE_KIND_SIZE + val.len(),
            Self::Tombstone => VALUE_KIND_SIZE,
        }
    }
}
