use risingwave_common::error::Result;
use std::fmt;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Epoch keeping for some specific key types.
pub(crate) const SINGLE_VERSION_EPOCH: Epoch = Epoch(0);

pub(crate) const MAX_NAMING_EPOCH: Epoch = Epoch(50);

pub const MIN_EPOCH: Epoch = Epoch(0);
pub const MAX_EPOCH: Epoch = Epoch(u64::MAX);
pub const EPOCH_PHYSICAL_SHIFT_BITS: u8 = 16;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub struct Epoch(u64);

impl Epoch {
    pub fn init() -> Self {
        Epoch(Epoch::physical_now() << EPOCH_PHYSICAL_SHIFT_BITS)
    }

    pub fn physical_time(&self) -> u64 {
        self.0 >> EPOCH_PHYSICAL_SHIFT_BITS
    }

    pub fn into_inner(self) -> u64 {
        self.0
    }

    pub fn next(&self) -> Epoch {
        let physical_now = Epoch::physical_now();
        if physical_now == self.physical_time() {
            Epoch(self.0 + 1)
        } else {
            Epoch(physical_now << EPOCH_PHYSICAL_SHIFT_BITS)
        }
    }

    // TODO: use a monotonic library to replace SystemTime.
    pub fn physical_now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

impl From<u64> for Epoch {
    fn from(e: u64) -> Self {
        Epoch(e)
    }
}

impl fmt::Display for Epoch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

pub trait EpochGenerator: Sync + Send {
    fn generate(&self) -> Result<Epoch>;
}

pub type EpochGeneratorRef = Box<dyn EpochGenerator>;

pub struct MemEpochGenerator {
    current_epoch: Mutex<Epoch>,
}

impl MemEpochGenerator {
    pub fn new() -> Self {
        MemEpochGenerator {
            current_epoch: Mutex::new(Epoch::init()),
        }
    }
}

impl EpochGenerator for MemEpochGenerator {
    fn generate(&self) -> Result<Epoch> {
        let mut ce = self.current_epoch.lock().unwrap();
        *ce = ce.next();
        Ok(*ce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoch_generator() -> Result<()> {
        let generator = MemEpochGenerator::new();
        let mut pre = generator.generate().unwrap();
        loop {
            let epoch = generator.generate().unwrap();
            assert!(epoch > pre);
            if epoch.physical_time() > pre.physical_time() {
                break;
            }
            pre = epoch;
        }
        Ok(())
    }
}