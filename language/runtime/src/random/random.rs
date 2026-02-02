use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// Runtime randomness and entropy providers.
#[derive(Debug)]
pub struct Random {
    /// Root seed for deterministic streams.
    pub root_seed: u64,
    /// Internal stream state.
    state: AtomicU64,
}

/// Identifier for a deterministic random stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RandomStreamId(u64);

impl RandomStreamId {
    /// Create a new random stream identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw stream identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Deterministic random stream derived from the root seed.
#[derive(Debug, Clone, Copy)]
pub struct RandomStream {
    /// Seed for this stream.
    pub seed: u64,
    /// Stream identifier for reproducibility.
    pub stream_id: RandomStreamId,
}

/// Policy for entropy access in deterministic modes.
#[derive(Debug, Clone, Copy, Default)]
pub enum EntropyPolicy {
    /// Entropy reads are forbidden.
    #[default]
    Disabled,
    /// Entropy reads are allowed but logged.
    Logged,
    /// Entropy reads pass through without logging.
    Passthrough,
}

impl Default for Random {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Random {
    /// Create a deterministic random source from a root seed.
    pub fn new(root_seed: u64) -> Self {
        Self {
            root_seed,
            state: AtomicU64::new(root_seed),
        }
    }

    /// Return the next random u64 value.
    pub fn next_u64(&self) -> u64 {
        let value = self
            .state
            .fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed)
            .wrapping_add(0x9e3779b97f4a7c15);
        mix64(value)
    }

    /// Reseed the random stream.
    pub fn reseed(&self, seed: u64) {
        self.state.store(seed, Ordering::Relaxed);
    }
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58476d1ce4e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d049bb133111eb);
    value ^= value >> 31;
    value
}
