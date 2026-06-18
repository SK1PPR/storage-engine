#[derive(Debug, Default)]
pub struct BlockBuilder {
    bytes: Vec<u8>,
}

impl BlockBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}
