use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use destack_workspace::{RandomMode, RandomOptions};

/// Runtime randomness and entropy providers.
#[derive(Debug)]
pub struct Random {
    /// Root seed for deterministic streams.
    pub root_seed: u64,
    /// Internal stream state.
    state: AtomicU64,
    /// Randomness mode selection.
    mode: RandomMode,
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
        Self::from_options(&RandomOptions::default())
    }
}

impl Random {
    /// Create a random source from runtime options.
    pub fn from_options(options: &RandomOptions) -> Self {
        // resolve seed and mode
        let root_seed = options.seed.unwrap_or(0);

        // construct the random source
        Self::new(root_seed, options.mode)
    }

    /// Create a random source from a root seed and mode.
    pub fn new(root_seed: u64, mode: RandomMode) -> Self {
        Self {
            root_seed,
            state: AtomicU64::new(root_seed),
            mode,
        }
    }

    /// Return the next random u64 value.
    pub fn next_u64(&self) -> u64 {
        // select the source based on mode
        match self.mode {
            RandomMode::Host => host_u64().unwrap_or_else(|_| self.next_deterministic_u64()),
            RandomMode::Deterministic => self.next_deterministic_u64(),
        }
    }

    /// Fill a buffer with random bytes.
    pub fn fill_bytes(&self, buffer: &mut [u8]) {
        // select the source based on mode
        match self.mode {
            RandomMode::Host => {
                // attempt host entropy and fall back on failure
                if host_fill_bytes(buffer).is_ok() {
                    return;
                }

                self.fill_deterministic_bytes(buffer);
            }
            RandomMode::Deterministic => self.fill_deterministic_bytes(buffer),
        }
    }

    /// Return a deterministic stream for a given id.
    pub fn stream(&self, stream_id: RandomStreamId) -> RandomStream {
        // derive a stable stream seed
        let seed = derive_stream_seed(self.root_seed, stream_id);

        // construct the stream
        RandomStream { seed, stream_id }
    }

    /// Reseed the random stream.
    pub fn reseed(&self, seed: u64) {
        // update the shared state
        self.state.store(seed, Ordering::Relaxed);
    }

    /// Return the next deterministic u64 value.
    pub fn next_deterministic_u64(&self) -> u64 {
        // advance the state and mix the output
        let value = self
            .state
            .fetch_add(0x9e3779b97f4a7c15, Ordering::Relaxed)
            .wrapping_add(0x9e3779b97f4a7c15);

        mix64(value)
    }

    /// Fill a buffer with deterministic random bytes.
    pub fn fill_deterministic_bytes(&self, buffer: &mut [u8]) {
        // fill bytes from the deterministic stream
        fill_bytes_from_seed(buffer, || self.next_deterministic_u64());
    }
}

impl RandomStream {
    /// Return the next deterministic u64 value from the stream.
    pub fn next_u64(&mut self) -> u64 {
        // advance the stream seed and mix the output
        self.seed = self.seed.wrapping_add(0x9e3779b97f4a7c15);

        mix64(self.seed)
    }

    /// Fill a buffer with deterministic random bytes from the stream.
    pub fn fill_bytes(&mut self, buffer: &mut [u8]) {
        // fill bytes from the stream generator
        fill_bytes_from_seed(buffer, || self.next_u64());
    }
}

fn mix64(mut value: u64) -> u64 {
    // splitmix64 mixing pipeline
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58476d1ce4e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d049bb133111eb);
    value ^= value >> 31;
    value
}

fn derive_stream_seed(root_seed: u64, stream_id: RandomStreamId) -> u64 {
    // combine root seed and stream id
    let mut seed = root_seed ^ stream_id.get().wrapping_mul(0x9e3779b97f4a7c15);

    // mix the combined seed
    seed = mix64(seed);
    seed
}

fn fill_bytes_from_seed(buffer: &mut [u8], mut next: impl FnMut() -> u64) {
    // fill the buffer with repeated 64-bit draws
    let mut offset = 0;
    while offset < buffer.len() {
        let value = next().to_le_bytes();
        let remaining = buffer.len() - offset;
        let copy_len = remaining.min(value.len());
        buffer[offset..offset + copy_len].copy_from_slice(&value[..copy_len]);
        offset += copy_len;
    }
}

fn host_u64() -> Result<u64, getrandom::Error> {
    // read host entropy for a single u64
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes)?;

    Ok(u64::from_le_bytes(bytes))
}

fn host_fill_bytes(buffer: &mut [u8]) -> Result<(), getrandom::Error> {
    // fill bytes from host entropy
    getrandom::fill(buffer)
}
