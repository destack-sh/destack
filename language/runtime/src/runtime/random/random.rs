use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use destack_core::{Capture, CaptureMode};
use destack_repository::{ExecutionMode, RandomOptions};

#[cfg(test)]
use super::r#virtual::StreamStateDecodeError;
use super::r#virtual::VirtualRandom;

/// Effective runtime randomness source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RandomSource {
    /// Use the host randomness source.
    #[default]
    Host,
    /// Use deterministic runtime-managed randomness.
    Deterministic,
}

/// Runtime randomness and entropy providers.
#[derive(Debug)]
pub struct Random {
    /// Active randomness source.
    source: RandomSource,
    /// Deterministic randomness for virtualized runtime draws.
    virtual_random: VirtualRandom,
}

/// Identifier for a deterministic random stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

/// Key for one worker scoped implicit random stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorkerRandomStreamKey {
    /// Runtime identifier component.
    pub runtime_id: u64,
    /// Worker identifier component.
    pub worker_id: u64,
}

impl Default for Random {
    fn default() -> Self {
        Self::from_options(RandomSource::Host, &RandomOptions::default())
    }
}

impl Random {
    /// Create a random source from runtime options.
    pub fn from_options(source: RandomSource, options: &RandomOptions) -> Self {
        let root_seed = options.seed.unwrap_or_default();

        Self::with_source(source, root_seed)
    }

    /// Create a random source from one source and root seed.
    pub fn with_source(source: RandomSource, root_seed: u64) -> Self {
        Self {
            source,
            virtual_random: VirtualRandom::new(root_seed),
        }
    }

    /// Return the active random source.
    pub const fn source(&self) -> RandomSource {
        self.source
    }

    /// Return the deterministic root seed.
    pub fn root_seed(&self) -> u64 {
        self.virtual_random.root_seed()
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
        self.virtual_random.next_stream_u64(stream_id)
    }

    /// Fill a buffer with random bytes from a stream.
    pub fn fill_stream_bytes(&self, stream_id: RandomStreamId, buffer: &mut [u8]) {
        self.virtual_random.fill_stream_bytes(stream_id, buffer);
    }

    /// Reseed the deterministic random stream.
    pub fn reseed(&self, seed: u64) {
        self.virtual_random.reseed(seed);
    }

    /// Resolve one worker scoped implicit random stream id.
    pub fn worker_stream_id(&self, runtime_id: u64, worker_id: u64) -> RandomStreamId {
        self.virtual_random.worker_stream_id(runtime_id, worker_id)
    }

    /// Allocate a new deterministic random stream id.
    pub fn new_stream_id(&self) -> RandomStreamId {
        self.virtual_random.new_stream_id()
    }

    /// Advance a deterministic stream by a fixed jump count.
    pub fn jump_stream(&self, stream_id: RandomStreamId, jump: u64) {
        self.virtual_random.jump_stream(stream_id, jump);
    }

    /// Split a deterministic stream and return a child stream id.
    pub fn split_stream(&self, parent_stream_id: RandomStreamId) -> RandomStreamId {
        self.virtual_random.split_stream(parent_stream_id)
    }

    /// Export one stream state into one versioned byte payload.
    #[cfg(test)]
    pub(crate) fn export_stream_state_bytes(&self, stream_id: RandomStreamId) -> Vec<u8> {
        self.virtual_random.export_stream_state_bytes(stream_id)
    }

    /// Import one stream state from one versioned byte payload.
    #[cfg(test)]
    pub(crate) fn import_stream_state_bytes(
        &self,
        stream_id: RandomStreamId,
        bytes: &[u8],
    ) -> Result<(), StreamStateDecodeError> {
        self.virtual_random
            .import_stream_state_bytes(stream_id, bytes)
    }

    /// Capture one materialized random image.
    pub(crate) fn snapshot(&self) -> RandomImage {
        let (root_seed, default_stream, next_stream_id, streams, worker_streams) =
            self.virtual_random.snapshot();

        RandomImage {
            root_seed,
            default_stream,
            next_stream_id,
            streams,
            worker_streams,
        }
    }

    /// Fork this random state for one child branch.
    pub(crate) fn fork(&self) -> RuntimeResult<Self> {
        // capture the current deterministic state first
        let image = self.snapshot();

        // rebuild one fresh random source with the same deterministic streams
        let forked = Self::with_source(self.source, image.root_seed);
        forked.restore_snapshot(&image)?;

        Ok(forked)
    }

    /// Restore one materialized random image.
    pub(crate) fn restore_snapshot(&self, snapshot: &RandomImage) -> RuntimeResult<()> {
        self.virtual_random
            .restore_snapshot(
                snapshot.root_seed,
                &snapshot.default_stream,
                snapshot.next_stream_id,
                &snapshot.streams,
                &snapshot.worker_streams,
            )
            .map_err(|error| {
                RuntimeError::Internal {
                    message: format!("random snapshot restore failed: {error:?}"),
                }
                .boxed()
            })?;

        Ok(())
    }
}

impl RandomSource {
    /// Resolve the effective random source for one execution mode.
    pub const fn from_execution_mode(mode: ExecutionMode) -> Self {
        match mode {
            ExecutionMode::Fast | ExecutionMode::Record => Self::Host,
            ExecutionMode::Strict | ExecutionMode::Replay => Self::Deterministic,
        }
    }
}

/// Materialized random state captured in one world image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomImage {
    /// Captured deterministic root seed.
    pub root_seed: u64,
    /// Captured deterministic default stream state.
    pub default_stream: Vec<u8>,
    /// Captured deterministic next stream identifier.
    pub next_stream_id: u64,
    /// Captured deterministic user stream states.
    pub streams: std::collections::BTreeMap<RandomStreamId, Vec<u8>>,
    /// Captured worker stream bindings.
    pub worker_streams: std::collections::BTreeMap<WorkerRandomStreamKey, RandomStreamId>,
}

impl Capture for Random {
    type Image = RandomImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one random image.
    fn capture_image(
        &mut self,
        _mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        Ok(self.snapshot())
    }

    /// Restore one random image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        self.restore_snapshot(image)
    }
}

#[cfg(test)]
mod tests {
    use super::{Random, RandomSource, RandomStreamId, StreamStateDecodeError};

    /// Create one deterministic test random source.
    fn deterministic_random(root_seed: u64) -> Random {
        Random::with_source(RandomSource::Deterministic, root_seed)
    }

    #[test]
    fn test_streams_are_isolated() {
        // allocate two streams and interleave draws
        let random = deterministic_random(0xdead_beef);
        let stream_a = random.new_stream_id();
        let stream_b = random.new_stream_id();

        let a1 = random.next_stream_u64(stream_a);
        let b1 = random.next_stream_u64(stream_b);
        let a2 = random.next_stream_u64(stream_a);
        let b2 = random.next_stream_u64(stream_b);

        // draw each stream without interleaving
        let random = deterministic_random(0xdead_beef);
        let stream_a = random.new_stream_id();
        let stream_b = random.new_stream_id();

        let a1_isolated = random.next_stream_u64(stream_a);
        let a2_isolated = random.next_stream_u64(stream_a);
        let b1_isolated = random.next_stream_u64(stream_b);
        let b2_isolated = random.next_stream_u64(stream_b);

        // verify stream sequences are unaffected by interleaving
        assert_eq!((a1, a2), (a1_isolated, a2_isolated));
        assert_eq!((b1, b2), (b1_isolated, b2_isolated));
    }

    #[test]
    fn test_jump_stream_matches_manual_advance() {
        // advance one stream with jump
        let random = deterministic_random(0xdead_beef);
        let stream = random.new_stream_id();
        random.jump_stream(stream, 3);
        let jumped_value = random.next_stream_u64(stream);

        // advance one stream manually
        let random = deterministic_random(0xdead_beef);
        let stream = random.new_stream_id();
        let _ = random.next_stream_u64(stream);
        let _ = random.next_stream_u64(stream);
        let _ = random.next_stream_u64(stream);
        let manual_value = random.next_stream_u64(stream);

        // verify jump semantics match manual advance
        assert_eq!(jumped_value, manual_value);
    }

    #[test]
    fn test_split_stream_is_deterministic() {
        // derive parent and child streams in one runtime
        let random = deterministic_random(0xdead_beef);
        let parent = random.new_stream_id();
        let _ = random.next_stream_u64(parent);
        let child = random.split_stream(parent);
        let child_value = random.next_stream_u64(child);

        // repeat the same sequence in a fresh runtime
        let random = deterministic_random(0xdead_beef);
        let parent_repeated = random.new_stream_id();
        let _ = random.next_stream_u64(parent_repeated);
        let child_repeated = random.split_stream(parent_repeated);
        let child_value_repeated = random.next_stream_u64(child_repeated);

        // verify deterministic split ids and values
        assert_eq!(child, child_repeated);
        assert_eq!(child_value, child_value_repeated);
        assert_ne!(child, RandomStreamId::DEFAULT);
    }

    #[test]
    fn test_worker_stream_id_is_stable_for_same_worker() {
        // resolve one worker stream id twice for one runtime and worker scope
        let random = deterministic_random(0xdead_beef);
        let first = random.worker_stream_id(1, 7);
        let second = random.worker_stream_id(1, 7);

        // ensure worker stream mapping is stable
        assert_eq!(first, second);
    }

    #[test]
    fn test_worker_stream_id_differs_across_workers() {
        // resolve one worker stream id for two workers
        let random = deterministic_random(0xdead_beef);
        let first_worker_stream = random.worker_stream_id(1, 7);
        let second_worker_stream = random.worker_stream_id(1, 8);

        // ensure runtime and worker identity separates streams
        assert_ne!(first_worker_stream, second_worker_stream);
    }

    #[test]
    fn test_stream_state_export_import_roundtrip_for_default_stream() {
        // step deterministic state and export one stream snapshot
        let random = deterministic_random(0xdead_beef);
        let _ = random.next_stream_u64(RandomStreamId::DEFAULT);
        let state = random.export_stream_state_bytes(RandomStreamId::DEFAULT);

        // capture one next value and restore the exported state
        let expected = random.next_stream_u64(RandomStreamId::DEFAULT);
        random
            .import_stream_state_bytes(RandomStreamId::DEFAULT, &state)
            .expect("default stream import should succeed");

        // imported stream state should replay the same next value
        let actual = random.next_stream_u64(RandomStreamId::DEFAULT);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_stream_state_export_import_roundtrip_for_named_stream() {
        // allocate one named stream and advance deterministic state
        let random = deterministic_random(0xdead_beef);
        let stream = random.new_stream_id();
        let _ = random.next_stream_u64(stream);
        let state = random.export_stream_state_bytes(stream);

        // capture one next value and restore the exported state
        let expected = random.next_stream_u64(stream);
        random
            .import_stream_state_bytes(stream, &state)
            .expect("named stream import should succeed");

        // imported stream state should replay the same next value
        let actual = random.next_stream_u64(stream);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_stream_state_import_rejects_invalid_payload_length() {
        // construct one deterministic random runtime
        let random = deterministic_random(0xdead_beef);

        // import should reject malformed payload lengths
        let error = random
            .import_stream_state_bytes(RandomStreamId::DEFAULT, &[1u8, 1u8, 2u8])
            .expect_err("stream import should fail for malformed payload");
        assert_eq!(error, StreamStateDecodeError::InvalidLength);
    }
}
