use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use destack_workspace::{RandomMode, RandomOptions};

/// Runtime randomness and entropy providers.
#[derive(Debug)]
pub struct Random {
    /// Root seed for deterministic streams.
    pub root_seed: u64,
    /// Internal stream state.
    state: AtomicU64,
    /// Next user-defined stream identifier.
    next_stream_id: AtomicU64,
    /// Per-stream deterministic state.
    streams: Mutex<HashMap<RandomStreamId, u64>>,
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

    /// Default runtime stream identifier.
    pub const DEFAULT: Self = Self(0);

    /// Return the raw stream identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Step size for deterministic random streams.
const STREAM_INCREMENT: u64 = 0x9e3779b97f4a7c15;

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
            next_stream_id: AtomicU64::new(1),
            streams: Mutex::new(HashMap::new()),
            mode,
        }
    }

    /// Return the next random u64 value.
    pub fn next_u64(&self) -> u64 {
        self.next_stream_u64(RandomStreamId::DEFAULT)
    }

    /// Fill a buffer with random bytes.
    pub fn fill_bytes(&self, buffer: &mut [u8]) {
        self.fill_stream_bytes(RandomStreamId::DEFAULT, buffer);
    }

    /// Return the next random u64 value for a stream.
    pub fn next_stream_u64(&self, stream_id: RandomStreamId) -> u64 {
        // select the source based on mode
        match self.mode {
            RandomMode::Host => {
                host_u64().unwrap_or_else(|_| self.next_stream_deterministic_u64(stream_id))
            }
            RandomMode::Deterministic => self.next_stream_deterministic_u64(stream_id),
        }
    }

    /// Fill a buffer with random bytes from a stream.
    pub fn fill_stream_bytes(&self, stream_id: RandomStreamId, buffer: &mut [u8]) {
        // select the source based on mode
        match self.mode {
            RandomMode::Host => {
                // attempt host entropy and fall back on failure
                if host_fill_bytes(buffer).is_ok() {
                    return;
                }

                self.fill_stream_deterministic_bytes(stream_id, buffer);
            }
            RandomMode::Deterministic => self.fill_stream_deterministic_bytes(stream_id, buffer),
        }
    }

    /// Reseed the random stream.
    pub fn reseed(&self, seed: u64) {
        // update the shared state
        self.state.store(seed, Ordering::Relaxed);
        self.next_stream_id.store(1, Ordering::Relaxed);
        // reset deterministic streams
        self.streams.lock().clear();
    }

    /// Allocate a new deterministic random stream id.
    pub fn new_stream_id(&self) -> RandomStreamId {
        // reserve the high tag bits for user-defined streams
        const STREAM_ID_MASK: u64 = (1u64 << 62) - 1;
        const USER_TAG: u64 = 3u64 << 62;

        let value = self.next_stream_id.fetch_add(1, Ordering::Relaxed) & STREAM_ID_MASK;
        RandomStreamId::new(USER_TAG | value)
    }

    /// Return the next deterministic u64 value.
    pub fn next_deterministic_u64(&self) -> u64 {
        // advance the state and mix the output
        let value = self
            .state
            .fetch_add(STREAM_INCREMENT, Ordering::Relaxed)
            .wrapping_add(STREAM_INCREMENT);

        mix64(value)
    }

    /// Fill a buffer with deterministic random bytes.
    pub fn fill_deterministic_bytes(&self, buffer: &mut [u8]) {
        // fill bytes from the deterministic stream
        fill_bytes_from_seed(buffer, || self.next_deterministic_u64());
    }

    /// Return the next deterministic u64 value for a stream.
    pub fn next_stream_deterministic_u64(&self, stream_id: RandomStreamId) -> u64 {
        // fast path the default stream without locking
        if stream_id == RandomStreamId::DEFAULT {
            return self.next_deterministic_u64();
        }

        // update the stream seed
        let mut streams = self.streams.lock();
        let seed = streams
            .entry(stream_id)
            .or_insert_with(|| derive_stream_seed(self.root_seed, stream_id));

        *seed = seed.wrapping_add(STREAM_INCREMENT);
        mix64(*seed)
    }

    /// Fill a buffer with deterministic random bytes from a stream.
    pub fn fill_stream_deterministic_bytes(&self, stream_id: RandomStreamId, buffer: &mut [u8]) {
        // fast path the default stream without locking
        if stream_id == RandomStreamId::DEFAULT {
            self.fill_deterministic_bytes(buffer);
            return;
        }

        // update the stream seed and fill the buffer
        let mut streams = self.streams.lock();
        let seed = streams
            .entry(stream_id)
            .or_insert_with(|| derive_stream_seed(self.root_seed, stream_id));

        fill_bytes_from_seed(buffer, || {
            *seed = seed.wrapping_add(STREAM_INCREMENT);
            mix64(*seed)
        });
    }
}

/// Mix a 64-bit value using splitmix64.
fn mix64(mut value: u64) -> u64 {
    // splitmix64 mixing pipeline
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58476d1ce4e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d049bb133111eb);
    value ^= value >> 31;
    value
}

/// Derive a deterministic stream seed from the root seed and stream id.
fn derive_stream_seed(root_seed: u64, stream_id: RandomStreamId) -> u64 {
    // combine root seed and stream id
    let mut seed = root_seed ^ stream_id.get().wrapping_mul(0x9e3779b97f4a7c15);

    // mix the combined seed
    seed = mix64(seed);
    seed
}

/// Fill a buffer from a deterministic 64-bit generator.
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

/// Read a u64 from host entropy.
fn host_u64() -> Result<u64, getrandom::Error> {
    // read host entropy for a single u64
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes)?;

    Ok(u64::from_le_bytes(bytes))
}

/// Fill a buffer with host entropy.
fn host_fill_bytes(buffer: &mut [u8]) -> Result<(), getrandom::Error> {
    // fill bytes from host entropy
    getrandom::fill(buffer)
}

#[cfg(test)]
mod tests {
    use super::Random;
    use destack_workspace::RandomMode;

    #[test]
    fn test_streams_are_isolated() {
        // allocate two streams and interleave draws
        let random = Random::new(0xdead_beef, RandomMode::Deterministic);
        let stream_a = random.new_stream_id();
        let stream_b = random.new_stream_id();

        let a1 = random.next_stream_deterministic_u64(stream_a);
        let b1 = random.next_stream_deterministic_u64(stream_b);
        let a2 = random.next_stream_deterministic_u64(stream_a);
        let b2 = random.next_stream_deterministic_u64(stream_b);

        // draw each stream without interleaving
        let random = Random::new(0xdead_beef, RandomMode::Deterministic);
        let stream_a = random.new_stream_id();
        let stream_b = random.new_stream_id();

        let a1_isolated = random.next_stream_deterministic_u64(stream_a);
        let a2_isolated = random.next_stream_deterministic_u64(stream_a);
        let b1_isolated = random.next_stream_deterministic_u64(stream_b);
        let b2_isolated = random.next_stream_deterministic_u64(stream_b);

        // verify stream sequences are unaffected by interleaving
        assert_eq!((a1, a2), (a1_isolated, a2_isolated));
        assert_eq!((b1, b2), (b1_isolated, b2_isolated));
    }
}
