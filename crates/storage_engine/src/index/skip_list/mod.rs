use crate::constants::{SKIP_LIST_DEFAULT_LEVEL, SKIP_LIST_DEFAULT_RNG_SEED};
use crate::index::{Key, MemTable, Value};

type Link = Option<usize>;

#[derive(Debug)]
pub struct SkipList {
    nodes: Vec<Node>,
    level: usize,
    max_level: usize,
    approximate_size: usize,
    rng_state: u64,
}

#[derive(Debug, Default)]
pub struct Node {
    key: Key,
    value: Value,
    forward: Vec<Link>,
}

impl Default for SkipList {
    fn default() -> Self {
        let max_level = SKIP_LIST_DEFAULT_LEVEL;

        Self {
            nodes: vec![Node {
                key: Key::default(),
                value: Value::default(),
                forward: vec![None; max_level],
            }],
            level: 1,
            max_level,
            approximate_size: 0,
            rng_state: SKIP_LIST_DEFAULT_RNG_SEED,
        }
    }
}

impl MemTable for SkipList {
    fn put(&mut self, key: Key, value: Value) {
        let mut update = self.find_update_path(&key);

        if let Some(existing) = self.next_at(update[0], 0) {
            if self.nodes[existing].key == key {
                let old_size = self.nodes[existing].entry_size();

                self.nodes[existing].value = value;
                self.approximate_size =
                    self.approximate_size - old_size + self.nodes[existing].entry_size();
                return;
            }
        }

        let node_level = self.random_level();
        if node_level > self.level {
            for slot in update.iter_mut().take(node_level).skip(self.level) {
                *slot = 0;
            }
            self.level = node_level;
        }

        let node_index = self.nodes.len();
        let mut node = Node {
            key,
            value,
            forward: vec![None; node_level],
        };

        for (level, previous) in update.iter().copied().enumerate().take(node_level) {
            node.forward[level] = self.nodes[previous].forward[level];
            self.nodes[previous].forward[level] = Some(node_index);
        }

        self.approximate_size += node.entry_size();
        self.nodes.push(node);
    }

    fn get(&self, key: &Key) -> Option<&Value> {
        let previous = self.find_previous(key);
        let candidate = self.next_at(previous, 0)?;

        if &self.nodes[candidate].key == key {
            Some(&self.nodes[candidate].value)
        } else {
            None
        }
    }

    fn delete(&mut self, key: Key) {
        self.put(key, Value::Tombstone);
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (&Key, &Value)> + '_> {
        Box::new(SkipListIter {
            list: self,
            next: self.nodes[0].forward[0],
        })
    }

    fn approximate_size(&self) -> usize {
        self.approximate_size
    }

    fn is_empty(&self) -> bool {
        self.nodes.len() == 1
    }
}

impl SkipList {
    pub fn new() -> Self {
        Self::default()
    }

    fn find_update_path(&self, key: &Key) -> Vec<usize> {
        let mut update = vec![0; self.max_level];
        let mut current = 0;

        for level in (0..self.level).rev() {
            while let Some(next) = self.next_at(current, level) {
                if self.nodes[next].key < *key {
                    current = next;
                } else {
                    break;
                }
            }

            update[level] = current;
        }

        update
    }

    fn find_previous(&self, key: &Key) -> usize {
        let mut current = 0;

        for level in (0..self.level).rev() {
            while let Some(next) = self.next_at(current, level) {
                if self.nodes[next].key < *key {
                    current = next;
                } else {
                    break;
                }
            }
        }

        current
    }

    fn next_at(&self, node_index: usize, level: usize) -> Link {
        self.nodes[node_index].forward.get(level).copied().flatten()
    }

    fn random_level(&mut self) -> usize {
        let mut level = 1;

        while level < self.max_level && self.next_random_bit() {
            level += 1;
        }

        level
    }

    fn next_random_bit(&mut self) -> bool {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;

        x & 1 == 1
    }
}

impl Node {
    fn entry_size(&self) -> usize {
        self.key.approximate_size() + self.value.approximate_size()
    }
}

struct SkipListIter<'a> {
    list: &'a SkipList,
    next: Link,
}

impl<'a> Iterator for SkipListIter<'a> {
    type Item = (&'a Key, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.next?;
        let node = &self.list.nodes[index];
        self.next = node.forward[0];

        Some((&node.key, &node.value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_reads_values_by_key() {
        let mut list = SkipList::default();
        list.put(Key::new("hello"), Value::put("world"));

        assert_eq!(
            list.get(&Key::new("hello")).and_then(Value::as_bytes),
            Some(b"world".as_slice())
        );
        assert_eq!(list.get(&Key::new("missing")), None);
    }

    #[test]
    fn iterates_in_key_order() {
        let mut list = SkipList::default();
        list.put(Key::new("b"), Value::put("2"));
        list.put(Key::new("a"), Value::put("1"));
        list.put(Key::new("c"), Value::put("3"));

        let keys: Vec<&[u8]> = list.iter().map(|(key, _)| key.as_bytes()).collect();
        assert_eq!(
            keys,
            vec![b"a".as_slice(), b"b".as_slice(), b"c".as_slice()]
        );
    }

    #[test]
    fn overwrites_existing_value_and_keeps_size_current() {
        let mut list = SkipList::default();
        list.put(Key::new("a"), Value::put("1"));
        let first_size = list.approximate_size();

        list.put(Key::new("a"), Value::put("longer"));

        assert!(list.approximate_size() > first_size);
        assert_eq!(
            list.get(&Key::new("a")).and_then(Value::as_bytes),
            Some(b"longer".as_slice())
        );
        assert_eq!(list.iter().count(), 1);
    }

    #[test]
    fn delete_writes_tombstone() {
        let mut list = SkipList::default();
        list.put(Key::new("a"), Value::put("1"));
        list.delete(Key::new("a"));

        assert_eq!(list.get(&Key::new("a")), Some(&Value::Tombstone));
    }
}
