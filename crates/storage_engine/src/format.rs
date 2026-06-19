use crate::{EngineError, Result};

#[derive(Debug, Default)]
pub struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

#[derive(Debug)]
pub struct Decoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    pub fn is_finished(&self) -> bool {
        self.remaining() == 0
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        let value = *self
            .bytes
            .get(self.position)
            .ok_or(EngineError::CorruptFormat("truncated u8"))?;
        self.position += 1;
        Ok(value)
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read_array("truncated u32")?))
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.read_array("truncated u64")?))
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(len)
            .ok_or(EngineError::CorruptFormat("decoder cursor overflow"))?;
        let slice = self
            .bytes
            .get(self.position..end)
            .ok_or(EngineError::CorruptFormat("truncated bytes"))?;
        self.position = end;
        Ok(slice)
    }

    pub fn read_record(&mut self) -> Result<&'a [u8]> {
        let record_len = self.read_u32()? as usize;
        self.read_bytes(record_len)
    }

    fn read_array<const N: usize>(&mut self, error: &'static str) -> Result<[u8; N]> {
        let bytes = self.read_bytes(N)?;
        bytes
            .try_into()
            .map_err(|_| EngineError::CorruptFormat(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_len_wraps_payload_bytes_only() {
        let mut payload = Encoder::new();
        payload.write_u8(7);
        payload.write_u32(11);
        let payload = payload.finish();

        let mut encoder = Encoder::new();
        encoder.write_u32(payload.len() as u32);
        encoder.write_bytes(&payload);

        let bytes = encoder.finish();
        let mut decoder = Decoder::new(&bytes);
        let record = decoder.read_record().unwrap();
        let mut record_decoder = Decoder::new(record);

        assert_eq!(record.len(), 5);
        assert_eq!(record_decoder.read_u8().unwrap(), 7);
        assert_eq!(record_decoder.read_u32().unwrap(), 11);
        assert!(decoder.is_finished());
    }
}
