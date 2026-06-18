//! eBPF integration boundary.
//!
//! Keep this module behind a narrow API. The core engine should work without
//! eBPF first; later this layer can mirror hot index metadata into pinned maps.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotIndexEntry {
    pub key_hash: u64,
    pub page_id: u64,
}

#[derive(Debug, Default)]
pub struct HotIndexMap {
    entries: Vec<HotIndexEntry>,
}

impl HotIndexMap {
    pub fn publish(&mut self, entry: HotIndexEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[HotIndexEntry] {
        &self.entries
    }
}
