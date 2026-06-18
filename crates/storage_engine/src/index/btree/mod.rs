use std::collections::BTreeMap;

use crate::index::{Key, MemTable, Value};

#[derive(Debug, Default)]
pub struct BTree {
    keys: BTreeMap<Key, Value>,
    approximate_size: usize,
}

impl MemTable for BTree {
    fn put(&mut self, key: Key, value: Value) {
        match self.keys.insert(key.clone(), value.clone()) {
            Some(old_value) => {
                self.approximate_size -= old_value.approximate_size();
                self.approximate_size += value.approximate_size();
            }
            None => {
                self.approximate_size += key.approximate_size() + value.approximate_size();
            }
        }
    }

    fn get(&self, key: &Key) -> Option<&Value> {
        self.keys.get(key)
    }

    fn delete(&mut self, key: Key) {
        let tombstone = Value::Tombstone;

        match self.keys.insert(key.clone(), tombstone.clone()) {
            Some(old_value) => {
                self.approximate_size -= old_value.approximate_size();
                self.approximate_size += tombstone.approximate_size();
            }
            None => {
                self.approximate_size += key.approximate_size() + tombstone.approximate_size();
            }
        }
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (&Key, &Value)> + '_> {
        Box::new(self.keys.iter())
    }

    fn approximate_size(&self) -> usize {
        self.approximate_size
    }

    fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}
