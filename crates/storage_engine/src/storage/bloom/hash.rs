use std::hash::Hasher;

use fastrand;
use twox_hash::XxHash3_64;

pub trait HashFunc {
    fn encode(&self, bytes: &[u8]) -> u64;
    fn from_seed(seed: u64) -> Self;
    fn seed(&self) -> u64;
}

#[derive(Clone, Debug)]
pub struct XxHash3Impl {
    seed: u64,
}

impl Default for XxHash3Impl {
    fn default() -> Self {
        Self {
            seed: fastrand::u64(..),
        }
    }
}

impl HashFunc for XxHash3Impl {
    fn encode(&self, bytes: &[u8]) -> u64 {
        let mut hasher = XxHash3_64::with_seed(self.seed);
        hasher.write(bytes);
        hasher.finish()
    }

    fn from_seed(seed: u64) -> Self {
        Self { seed }
    }

    fn seed(&self) -> u64 {
        self.seed
    }
}
