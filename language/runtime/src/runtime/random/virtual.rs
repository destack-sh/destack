use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use super::{RandomStreamId, ScopedRandomStreamKey};

/// Step size for deterministic random streams.
const STREAM_INCREMENT: u64 = 0x9e3779b97f4a7c15;
/// Stream state payload format version.
const STREAM_STATE_VERSION: u8 = 1;
/// Stream state payload length in bytes.
const STREAM_STATE_BYTES: usize = 10;

/// Decode error for serialized stream state payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StreamStateDecodeError {
    /// Stream state payload length does not match the expected format.
    InvalidLength,
    /// Stream state payload version is unsupported.
    UnsupportedVersion,
    /// Stream state payload carries an invalid initialized marker.
    InvalidInitializedFlag,
}

/// Internal deterministic random stream state.
#[derive(Debug)]
pub(crate) struct VirtualRandom {
    /// Root seed for deterministic streams.
    root_seed: u64,
    /// Internal default stream state.
    state: AtomicU64,
    /// Next user-defined stream identifier.
    next_stream_id: AtomicU64,
    /// Per-stream deterministic state.
    streams: Mutex<HashMap<RandomStreamId, u64>>,
    /// Cached scoped stream identities keyed by runtime and worker scope.
    scoped_streams: Mutex<HashMap<ScopedRandomStreamKey, RandomStreamId>>,
}

impl VirtualRandom {
    /// Create one deterministic random state from one root seed.
    pub(crate) fn new(root_seed: u64) -> Self {
        Self {
            root_seed,
            state: AtomicU64::new(root_seed),
            next_stream_id: AtomicU64::new(1),
            streams: Mutex::new(HashMap::new()),
            scoped_streams: Mutex::new(HashMap::new()),
        }
    }

    /// Return the root seed for deterministic streams.
    pub(crate) fn root_seed(&self) -> u64 {
        self.root_seed
    }

    /// Reseed deterministic stream state.
    pub(crate) fn reseed(&self, seed: u64) {
        // update the shared state
        self.state.store(seed, Ordering::Relaxed);
        self.next_stream_id.store(1, Ordering::Relaxed);

        // reset deterministic streams
        self.streams.lock().clear();
        self.scoped_streams.lock().clear();
    }

    /// Resolve one scoped implicit random stream id.
    pub(crate) fn scoped_stream_id(
        &self,
        runtime_id: u64,
        worker_id: u64,
        task_id: Option<u64>,
        microtask_id: Option<u64>,
    ) -> RandomStreamId {
        // resolve one stable key for this runtime and worker scope
        let key = ScopedRandomStreamKey {
            runtime_id,
            worker_id,
            task_id,
            microtask_id,
        };

        // reuse an existing scoped stream mapping when available
        let mut scoped_streams = self.scoped_streams.lock();
        if let Some(stream_id) = scoped_streams.get(&key) {
            return *stream_id;
        }

        // allocate and cache a stable scoped stream id
        let stream_id = self.new_stream_id();
        scoped_streams.insert(key, stream_id);

        stream_id
    }

    /// Allocate a new deterministic random stream id.
    pub(crate) fn new_stream_id(&self) -> RandomStreamId {
        // reserve the high tag bits for user-defined streams
        const STREAM_ID_MASK: u64 = (1u64 << 62) - 1;
        const USER_TAG: u64 = 3u64 << 62;

        let value = self.next_stream_id.fetch_add(1, Ordering::Relaxed) & STREAM_ID_MASK;
        RandomStreamId::new(USER_TAG | value)
    }

    /// Advance a deterministic stream by a fixed jump count.
    pub(crate) fn jump_stream(&self, stream_id: RandomStreamId, jump: u64) {
        // no-op for zero jump
        if jump == 0 {
            return;
        }

        // compute the deterministic increment once
        let increment = STREAM_INCREMENT.wrapping_mul(jump);

        // fast path the default stream without locking
        if stream_id == RandomStreamId::DEFAULT {
            self.state.fetch_add(increment, Ordering::Relaxed);
            return;
        }

        // advance the deterministic state for the requested stream
        let mut streams = self.streams.lock();
        let seed = streams
            .entry(stream_id)
            .or_insert_with(|| derive_stream_seed(self.root_seed, stream_id));
        *seed = seed.wrapping_add(increment);
    }

    /// Split a deterministic stream and return a child stream id.
    pub(crate) fn split_stream(&self, parent_stream_id: RandomStreamId) -> RandomStreamId {
        // allocate the child stream id first
        let child_stream_id = self.new_stream_id();

        // derive a deterministic child seed from parent state
        let parent_seed = if parent_stream_id == RandomStreamId::DEFAULT {
            self.state.load(Ordering::Relaxed)
        } else {
            let mut streams = self.streams.lock();
            let seed = streams
                .entry(parent_stream_id)
                .or_insert_with(|| derive_stream_seed(self.root_seed, parent_stream_id));
            *seed
        };

        // store the child stream seed
        let child_seed = mix64(parent_seed ^ child_stream_id.get().wrapping_mul(STREAM_INCREMENT));
        self.streams.lock().insert(child_stream_id, child_seed);

        child_stream_id
    }

    /// Return the next deterministic u64 value from the default stream.
    pub(crate) fn next_u64(&self) -> u64 {
        // advance the state and mix the output
        let value = self
            .state
            .fetch_add(STREAM_INCREMENT, Ordering::Relaxed)
            .wrapping_add(STREAM_INCREMENT);

        mix64(value)
    }

    /// Fill a buffer with deterministic random bytes from the default stream.
    pub(crate) fn fill_bytes(&self, buffer: &mut [u8]) {
        // fill bytes from the deterministic stream
        fill_bytes_from_seed(buffer, || self.next_u64());
    }

    /// Return the next deterministic u64 value for a stream.
    pub(crate) fn next_stream_u64(&self, stream_id: RandomStreamId) -> u64 {
        // fast path the default stream without locking
        if stream_id == RandomStreamId::DEFAULT {
            return self.next_u64();
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
    pub(crate) fn fill_stream_bytes(&self, stream_id: RandomStreamId, buffer: &mut [u8]) {
        // fast path the default stream without locking
        if stream_id == RandomStreamId::DEFAULT {
            self.fill_bytes(buffer);
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

    /// Export one stream state into one versioned byte payload.
    pub(crate) fn export_stream_state_bytes(&self, stream_id: RandomStreamId) -> Vec<u8> {
        // capture stream initialization marker and seed
        let (is_initialized, seed) = self.stream_state(stream_id);

        // encode the versioned payload
        let mut bytes = Vec::with_capacity(STREAM_STATE_BYTES);
        bytes.push(STREAM_STATE_VERSION);
        bytes.push(if is_initialized { 1 } else { 0 });
        bytes.extend_from_slice(&seed.to_le_bytes());

        bytes
    }

    /// Import one stream state from one versioned byte payload.
    pub(crate) fn import_stream_state_bytes(
        &self,
        stream_id: RandomStreamId,
        bytes: &[u8],
    ) -> Result<(), StreamStateDecodeError> {
        // validate payload length and version
        if bytes.len() != STREAM_STATE_BYTES {
            return Err(StreamStateDecodeError::InvalidLength);
        }
        if bytes[0] != STREAM_STATE_VERSION {
            return Err(StreamStateDecodeError::UnsupportedVersion);
        }

        // decode initialization marker and seed
        let is_initialized = match bytes[1] {
            0 => false,
            1 => true,
            _ => return Err(StreamStateDecodeError::InvalidInitializedFlag),
        };
        let mut seed_bytes = [0u8; 8];
        seed_bytes.copy_from_slice(&bytes[2..10]);
        let seed = u64::from_le_bytes(seed_bytes);

        // apply imported stream state
        self.set_stream_state(stream_id, is_initialized, seed);

        Ok(())
    }

    /// Return one initialization marker and seed for one stream.
    fn stream_state(&self, stream_id: RandomStreamId) -> (bool, u64) {
        // default stream always has one materialized seed
        if stream_id == RandomStreamId::DEFAULT {
            let seed = self.state.load(Ordering::Relaxed);
            return (true, seed);
        }

        // non-default streams are materialized on first use
        let streams = self.streams.lock();
        let Some(seed) = streams.get(&stream_id).copied() else {
            return (false, 0);
        };

        (true, seed)
    }

    /// Set one initialization marker and seed for one stream.
    fn set_stream_state(&self, stream_id: RandomStreamId, is_initialized: bool, seed: u64) {
        // default stream stores state in one lock-free atomic slot
        if stream_id == RandomStreamId::DEFAULT {
            let next_seed = if is_initialized { seed } else { self.root_seed };
            self.state.store(next_seed, Ordering::Relaxed);
            return;
        }

        // non-default streams live in the stream seed map
        let mut streams = self.streams.lock();
        if is_initialized {
            streams.insert(stream_id, seed);
            return;
        }

        streams.remove(&stream_id);
    }

    /// Capture one durable deterministic random snapshot.
    pub(crate) fn snapshot(
        &self,
    ) -> (
        u64,
        Vec<u8>,
        u64,
        std::collections::BTreeMap<RandomStreamId, Vec<u8>>,
        std::collections::BTreeMap<ScopedRandomStreamKey, RandomStreamId>,
    ) {
        // capture default stream state
        let default_stream = self.export_stream_state_bytes(RandomStreamId::DEFAULT);

        // capture explicit stream states in stable order
        let mut streams = std::collections::BTreeMap::new();
        let locked_streams = self.streams.lock();
        for stream_id in locked_streams.keys().copied() {
            let bytes = self.export_stream_state_bytes(stream_id);
            streams.insert(stream_id, bytes);
        }
        drop(locked_streams);

        // capture scoped stream bindings in stable order
        let scoped_streams = self
            .scoped_streams
            .lock()
            .iter()
            .map(|(key, stream_id)| (*key, *stream_id))
            .collect();

        (
            self.root_seed,
            default_stream,
            self.next_stream_id.load(Ordering::Relaxed),
            streams,
            scoped_streams,
        )
    }

    /// Restore one durable deterministic random snapshot.
    pub(crate) fn restore_snapshot(
        &self,
        root_seed: u64,
        default_stream: &[u8],
        next_stream_id: u64,
        streams: &std::collections::BTreeMap<RandomStreamId, Vec<u8>>,
        scoped_streams: &std::collections::BTreeMap<ScopedRandomStreamKey, RandomStreamId>,
    ) -> Result<(), StreamStateDecodeError> {
        // reset deterministic state
        self.state.store(root_seed, Ordering::Relaxed);
        self.next_stream_id.store(next_stream_id, Ordering::Relaxed);
        self.streams.lock().clear();
        self.scoped_streams.lock().clear();

        // restore default stream state
        self.import_stream_state_bytes(RandomStreamId::DEFAULT, default_stream)?;

        // restore materialized stream states
        for (stream_id, bytes) in streams {
            self.import_stream_state_bytes(*stream_id, bytes)?;
        }

        // restore scoped stream bindings
        let mut locked_scoped_streams = self.scoped_streams.lock();
        for (key, stream_id) in scoped_streams {
            locked_scoped_streams.insert(*key, *stream_id);
        }

        Ok(())
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
    let seed = root_seed ^ stream_id.get().wrapping_mul(STREAM_INCREMENT);

    // mix the combined seed
    mix64(seed)
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
