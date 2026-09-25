use tspp_core::{Capture, CaptureMode, SnapshotCodec};

use crate::binding::{Binding, ReplayPayload};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::time::Instant;
use crate::world::trace::{
    BindingTrace, ClockTrace, EntropySubject, EntrypointCall, RandomTrace, Trace, TraceCursor,
    TraceCursorImage, TraceEntry, TraceHeader, TraceSequence, TraceStore, TraceTag,
};
use crate::world::{BranchId, Mutation};
use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tspp_repository::ExecutionMode;
use tspp_serde::append_to_vec;

use super::file::TraceFile;

/// Trace channel name for entropy records.
pub(super) const ENTROPY_CHANNEL: &str = "runtime.random.entropy";

/// Validation state for trace entry ordering.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct Validator {
    /// The last observed monotonic timestamp.
    last_monotonic_nanos: Option<u64>,
}

impl Validator {
    /// Validate one trace entry against ordering invariants.
    fn validate(&mut self, entry: &TraceEntry) -> RuntimeResult<()> {
        match &entry.trace {
            Trace::Clock(trace) => self.validate_clock(trace)?,
            Trace::Random(trace) => self.validate_random(trace)?,
            _ => {}
        }

        Ok(())
    }

    /// Validate one clock trace against ordering invariants.
    fn validate_clock(&mut self, trace: &ClockTrace) -> RuntimeResult<()> {
        match trace {
            // advance virtual time monotonically
            ClockTrace::Advance(deadline) => {
                if let Some(last) = self.last_monotonic_nanos
                    && deadline.get() < last
                {
                    return Err(RuntimeError::trace_mismatch("time".to_string()).boxed());
                }

                self.last_monotonic_nanos = Some(deadline.get());
            }

            // read monotonic time monotonically
            ClockTrace::ReadMonotonic { outcome, .. } => {
                if let Ok(time_nanos) = outcome {
                    if let Some(last) = self.last_monotonic_nanos
                        && *time_nanos < last
                    {
                        return Err(
                            RuntimeError::trace_mismatch(ENTROPY_CHANNEL.to_string()).boxed()
                        );
                    }

                    self.last_monotonic_nanos = Some(*time_nanos);
                }
            }

            // wall time has no monotonic invariant
            ClockTrace::ReadWall { .. } => {}
        }

        Ok(())
    }

    /// Validate one random trace against payload invariants.
    fn validate_random(&mut self, trace: &RandomTrace) -> RuntimeResult<()> {
        if let RandomTrace::ReadBytes {
            len,
            outcome: Ok(bytes),
            ..
        } = trace
            && bytes.len() != *len as usize
        {
            return Err(RuntimeError::trace_mismatch(ENTROPY_CHANNEL.to_string()).boxed());
        }

        Ok(())
    }
}

/// Materialized trace state captured in one world image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceImage {
    /// The active trace execution mode.
    mode: ExecutionMode,
    /// The captured flat trace file.
    file: TraceFile,
    /// The captured reader cursor when replay mode is active.
    cursor: Option<TraceCursorImage>,
    /// The captured trace validator state.
    validator: Validator,
}

impl TraceImage {
    /// Return the captured trace header.
    pub(crate) fn header(&self) -> TraceHeader {
        self.file.header()
    }

    /// Return the next sequence number after this trace image.
    pub(crate) fn next_sequence(&self) -> RuntimeResult<TraceSequence> {
        self.file.next_sequence()
    }
}

/// Trace log for record and replay execution.
#[derive(Debug)]
pub struct TraceLog {
    /// Active execution mode.
    mode: ExecutionMode,
    /// Trace backing store.
    store: TraceStore,
    /// Trace cursor for log playback.
    reader: Option<Mutex<TraceCursor>>,
    /// Trace ordering validator.
    validator: Mutex<Validator>,
    /// Scratch buffer for trace payload encoding.
    scratch: Mutex<Vec<u8>>,
}

impl TraceLog {
    /// Create trace state with an explicit execution mode.
    pub fn new(mode: ExecutionMode, header: TraceHeader) -> Self {
        Self::from_store(mode, TraceStore::new(header))
    }

    /// Create trace state with one existing store and execution mode.
    pub(crate) fn from_store(mode: ExecutionMode, store: TraceStore) -> Self {
        let reader = match mode {
            ExecutionMode::Replay => Some(Mutex::new(store.reader())),
            _ => None,
        };

        Self {
            mode,
            store,
            reader,
            validator: Mutex::new(Validator::default()),
            scratch: Mutex::new(Vec::new()),
        }
    }

    /// Return the active execution mode.
    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }

    /// Return the backing trace store.
    pub(crate) fn store(&self) -> &TraceStore {
        &self.store
    }

    /// Set the current trace branch identifier.
    pub(crate) fn set_branch_id(&self, branch_id: BranchId) {
        self.store.set_branch_id(branch_id);
    }

    /// Capture one materialized trace image.
    pub(crate) fn capture_image(&self) -> TraceImage {
        let cursor = self
            .reader
            .as_ref()
            .map(|reader| reader.lock().capture_image());
        let validator = self.validator.lock().clone();

        TraceImage {
            mode: self.mode,
            file: self.store.file(),
            cursor,
            validator,
        }
    }

    /// Capture one trace image truncated to one exact sequence boundary.
    pub(crate) fn capture_image_through(
        &self,
        sequence: TraceSequence,
    ) -> RuntimeResult<TraceImage> {
        if sequence == self.store.next_sequence() {
            return Ok(self.capture_image());
        }

        if sequence.get() > self.store.next_sequence().get() {
            return Err(RuntimeError::trace_mismatch("sequence".to_string()).boxed());
        }

        let header = self.store.header();
        let replay_trace = TraceLog::from_store(ExecutionMode::Replay, self.store.clone());
        let captured_trace = TraceLog::new(self.mode, header);
        replay_trace.seek_sequence(TraceSequence::new(0))?;

        while replay_trace.sequence()? != sequence {
            let entry = replay_trace
                .next_entry()?
                .ok_or_else(|| RuntimeError::trace_exhausted(sequence.get()).boxed())?;
            captured_trace.record_trace(entry.trace)?;
        }

        Ok(captured_trace.capture_image())
    }

    /// Restore one materialized trace image.
    pub(crate) fn restore_image(&self, image: &TraceImage) -> RuntimeResult<()> {
        // require matching replay mode
        if self.mode != image.mode {
            return Err(RuntimeError::Internal {
                message: "trace image mode does not match world execution mode".to_string(),
            }
            .boxed());
        }

        // restore the shared log image first
        self.store.restore_file(image.file.clone())?;

        // restore the reader cursor when replay is active
        match (&self.reader, image.cursor) {
            (Some(reader), Some(cursor_image)) => {
                reader.lock().restore_image(cursor_image)?;
            }
            (None, None) => {}
            _ => {
                return Err(RuntimeError::Internal {
                    message: "replay reader state does not match world replay mode".to_string(),
                }
                .boxed());
            }
        }

        // restore validator state
        let mut validator = self.validator.lock();
        *validator = image.validator.clone();

        Ok(())
    }

    /// Return one trace mismatch error for one entry name.
    fn trace_mismatch_error(name: &str) -> Box<RuntimeError> {
        RuntimeError::trace_mismatch(name.to_string()).boxed()
    }

    /// Resolve one requested payload policy for a binding.
    pub fn payload_policy_for_requested(
        &self,
        binding: Binding,
        name: &str,
        requested: ReplayPayload,
    ) -> RuntimeResult<ReplayPayload> {
        let supported = binding.replay_payload();

        if requested == ReplayPayload::ArgumentsAndResults && supported == ReplayPayload::Results {
            return Err(RuntimeError::trace_payload_unsupported(name.to_string()).boxed());
        }

        match requested {
            ReplayPayload::Results => Ok(ReplayPayload::Results),
            ReplayPayload::ArgumentsAndResults => Ok(supported),
        }
    }

    /// Record one decoded trace payload when recording is enabled.
    pub(crate) fn record_trace(&self, trace: Trace) -> RuntimeResult<()> {
        match trace {
            Trace::Mutation(mutation) => {
                self.record_payload(TraceTag::Mutation, "world", &mutation)
            }
            Trace::Entrypoint(invocation) => {
                self.record_payload(TraceTag::Entrypoint, "entrypoint", &invocation)
            }
            Trace::Binding(trace) => self.record_payload(TraceTag::Binding, "binding", &trace),
            Trace::Clock(trace) => self.record_payload(TraceTag::Clock, "time", &trace),
            Trace::Random(trace) => self.record_payload(TraceTag::Random, ENTROPY_CHANNEL, &trace),
        }
    }

    /// Record one typed trace payload when recording is enabled.
    pub(crate) fn record_payload<T>(
        &self,
        tag: TraceTag,
        name: &str,
        payload: &T,
    ) -> RuntimeResult<()>
    where
        T: Serialize,
    {
        // skip recording when disabled
        if self.mode() != ExecutionMode::Record {
            return Ok(());
        }

        // encode once into reusable scratch before appending
        let mut scratch = self.scratch.lock();
        scratch.clear();
        append_to_vec(payload, &mut scratch)
            .map_err(|_| RuntimeError::trace_encode_failed(name.to_string()).boxed())?;

        // append the encoded payload
        self.store.record_encoded(tag, &scratch)?;
        Ok(())
    }

    /// Record one authoritative trace mutation.
    pub(crate) fn record_mutation(&self, mutation: Mutation) -> RuntimeResult<()> {
        self.record_payload(TraceTag::Mutation, "world", &mutation)
    }

    /// Record one authoritative entrypoint call.
    pub(crate) fn record_entrypoint(&self, invocation: EntrypointCall) -> RuntimeResult<()> {
        self.record_payload(TraceTag::Entrypoint, "entrypoint", &invocation)
    }

    /// Return one trace mismatch error for the entropy channel.
    pub(crate) fn entropy_mismatch_error(&self) -> Box<RuntimeError> {
        RuntimeError::trace_mismatch(ENTROPY_CHANNEL.to_string()).boxed()
    }

    /// Read and validate one clock trace from replay.
    pub(crate) fn next_clock_trace(&self) -> RuntimeResult<ClockTrace> {
        if self.mode() != ExecutionMode::Replay {
            return Err(self.entropy_mismatch_error());
        }

        let Some((_sequence, trace)) = self
            .reader
            .as_ref()
            .ok_or_else(|| Self::trace_mismatch_error("replay"))?
            .lock()
            .next_payload(TraceTag::Clock)?
        else {
            let sequence = self.store.next_sequence().get();
            return Err(RuntimeError::trace_exhausted(sequence).boxed());
        };
        self.validator.lock().validate_clock(&trace)?;

        Ok(trace)
    }

    /// Read and validate one random trace from replay.
    pub(crate) fn next_random_trace(
        &self,
        expected_subject: EntropySubject,
    ) -> RuntimeResult<RandomTrace> {
        if self.mode() != ExecutionMode::Replay {
            return Err(self.entropy_mismatch_error());
        }

        let Some((_sequence, trace)) = self
            .reader
            .as_ref()
            .ok_or_else(|| Self::trace_mismatch_error("replay"))?
            .lock()
            .next_payload(TraceTag::Random)?
        else {
            let sequence = self.store.next_sequence().get();
            return Err(RuntimeError::trace_exhausted(sequence).boxed());
        };
        self.validator.lock().validate_random(&trace)?;

        if trace.subject() != expected_subject {
            return Err(self.entropy_mismatch_error());
        }

        Ok(trace)
    }

    /// Read the next entry when replay is enabled.
    pub(crate) fn next_entry(&self) -> RuntimeResult<Option<TraceEntry>> {
        // skip replay when disabled
        if self.mode() != ExecutionMode::Replay {
            return Ok(None);
        }

        // fetch the next entry from the reader
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| Self::trace_mismatch_error("replay"))?;
        let Some(entry) = reader.lock().next_entry()? else {
            return Ok(None);
        };

        let mut validator = self.validator.lock();
        validator.validate(&entry)?;

        Ok(Some(entry))
    }

    /// Return the next trace sequence for the active reader cursor.
    pub fn sequence(&self) -> RuntimeResult<TraceSequence> {
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| Self::trace_mismatch_error("replay"))?;

        Ok(reader.lock().sequence())
    }

    /// Seek the active reader cursor to one sequence boundary.
    pub fn seek_sequence(&self, sequence: TraceSequence) -> RuntimeResult<()> {
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| Self::trace_mismatch_error("replay"))?;

        reader.lock().seek_sequence(sequence)
    }

    /// Record a binding call payload for replay.
    pub fn record_binding_call(
        &self,
        binding: Binding,
        name: &str,
        bytes: &[u8],
    ) -> RuntimeResult<()> {
        self.record_binding_call_bytes(binding, name, bytes.to_vec())
    }

    /// Record an owned binding call payload for replay.
    fn record_binding_call_bytes(
        &self,
        binding: Binding,
        name: &str,
        bytes: Vec<u8>,
    ) -> RuntimeResult<()> {
        let trace = BindingTrace {
            binding_id: binding.id(),
            codec: binding.codec(),
            bytes,
        };

        self.record_payload(TraceTag::Binding, name, &trace)
    }

    /// Read the next binding call payload for replay.
    pub fn next_binding_call(&self, binding: Binding, name: &str) -> RuntimeResult<BindingTrace> {
        // read the next binding payload from the log
        let Some((_sequence, call)) = self
            .reader
            .as_ref()
            .ok_or_else(|| Self::trace_mismatch_error("replay"))?
            .lock()
            .next_payload::<BindingTrace>(TraceTag::Binding)?
        else {
            let sequence = self.store.next_sequence().get();
            return Err(RuntimeError::trace_exhausted(sequence).boxed());
        };

        // validate binding id
        if call.binding_id != binding.id() {
            return Err(Self::trace_mismatch_error(name));
        }

        // validate codec id
        if call.codec != binding.codec() {
            return Err(Self::trace_mismatch_error(name));
        }

        Ok(call)
    }

    /// Record one virtual-time advance outcome.
    pub fn record_time_advance(&self, deadline: Instant) -> RuntimeResult<()> {
        self.record_payload(TraceTag::Clock, "time", &ClockTrace::Advance(deadline))
    }

    /// Read the next virtual-time advance outcome from replay.
    pub fn next_time_advance(&self) -> RuntimeResult<Instant> {
        let trace = self.next_clock_trace()?;
        let ClockTrace::Advance(deadline) = trace else {
            return Err(Self::trace_mismatch_error("time"));
        };
        Ok(deadline)
    }

    /// Resolve one requested virtual-time advance under the active replay mode.
    pub fn resolve_time_advance(&self, requested_deadline: Instant) -> RuntimeResult<Instant> {
        match self.mode() {
            // fast, strict, and record use the requested deadline
            ExecutionMode::Fast | ExecutionMode::Strict | ExecutionMode::Record => {
                Ok(requested_deadline)
            }

            // replay requires the next recorded outcome to match
            ExecutionMode::Replay => {
                let deadline = self.next_time_advance()?;
                if deadline != requested_deadline {
                    return Err(Self::trace_mismatch_error("time"));
                }

                Ok(deadline)
            }
        }
    }

    /// Read the next mutation from replay.
    pub(crate) fn next_mutation(&self) -> RuntimeResult<Mutation> {
        // read the next mutation payload from the log
        let Some((_sequence, mutation)) = self
            .reader
            .as_ref()
            .ok_or_else(|| Self::trace_mismatch_error("replay"))?
            .lock()
            .next_payload(TraceTag::Mutation)?
        else {
            let sequence = self.store.next_sequence().get();
            return Err(RuntimeError::trace_exhausted(sequence).boxed());
        };

        Ok(mutation)
    }

    /// Resolve one mutation under the active replay mode.
    pub(crate) fn resolve_mutation(&self, requested_mutation: Mutation) -> RuntimeResult<Mutation> {
        match self.mode() {
            // fast execution applies the requested mutation directly
            ExecutionMode::Fast => Ok(requested_mutation),
            // replay execution aligns the requested mutation with the replay log
            ExecutionMode::Replay => {
                let replayed_mutation = self.next_mutation()?;
                if replayed_mutation != requested_mutation {
                    return Err(Self::trace_mismatch_error("world"));
                }

                Ok(replayed_mutation)
            }
            // strict and record modes keep local mutation behavior
            ExecutionMode::Strict | ExecutionMode::Record => Ok(requested_mutation),
        }
    }

    /// Read the next entrypoint call from replay.
    pub(crate) fn next_entrypoint(&self) -> RuntimeResult<EntrypointCall> {
        let Some((_sequence, invocation)) = self
            .reader
            .as_ref()
            .ok_or_else(|| Self::trace_mismatch_error("replay"))?
            .lock()
            .next_payload(TraceTag::Entrypoint)?
        else {
            let sequence = self.store.next_sequence().get();
            return Err(RuntimeError::trace_exhausted(sequence).boxed());
        };

        Ok(invocation)
    }

    /// Resolve one entrypoint call under the active replay mode.
    pub(crate) fn resolve_entrypoint(
        &self,
        requested_invocation: EntrypointCall,
    ) -> RuntimeResult<EntrypointCall> {
        match self.mode() {
            ExecutionMode::Fast => Ok(requested_invocation),
            ExecutionMode::Replay => {
                let replayed_invocation = self.next_entrypoint()?;
                if replayed_invocation != requested_invocation {
                    return Err(Self::trace_mismatch_error("entrypoint"));
                }

                Ok(replayed_invocation)
            }
            ExecutionMode::Strict | ExecutionMode::Record => Ok(requested_invocation),
        }
    }

    /// Restore one captured trace log into one replay trace and reset the reader.
    pub(crate) fn restore_replay_image(&self, image: &TraceImage) -> RuntimeResult<()> {
        if self.mode() != ExecutionMode::Replay {
            return Err(RuntimeError::Internal {
                message: "trace replay restore requires replay execution mode".to_string(),
            }
            .boxed());
        }

        self.store.restore_file(image.file.clone())?;
        self.seek_sequence(TraceSequence::new(0))?;

        let mut validator = self.validator.lock();
        *validator = image.validator.clone();

        Ok(())
    }

    /// Record a typed trace payload for a binding.
    pub fn record_binding_payload<T: Serialize>(
        &self,
        binding: Binding,
        name: &str,
        payload: &T,
    ) -> RuntimeResult<()> {
        // skip recording when disabled
        if self.mode() != ExecutionMode::Record {
            return Ok(());
        }

        // encode the binding payload into reusable scratch
        let payload_bytes = {
            let mut scratch = self.scratch.lock();
            scratch.clear();
            append_to_vec(payload, &mut scratch)
                .map_err(|_| RuntimeError::trace_encode_failed(name.to_string()).boxed())?;

            scratch.clone()
        };

        // record the encoded payload
        self.record_binding_call_bytes(binding, name, payload_bytes)
    }

    /// Decode the next typed binding payload for replay.
    pub fn read_binding_payload<T: DeserializeOwned>(
        &self,
        binding: Binding,
        name: &str,
    ) -> RuntimeResult<T> {
        // read the next binding call payload
        let call = self.next_binding_call(binding, name)?;

        // decode the payload bytes
        let payload = tspp_serde::from_slice(&call.bytes)
            .map_err(|_| RuntimeError::trace_decode_failed(name.to_string()).boxed())?;
        Ok(payload)
    }

    /// Run a binding with replay handling against one mutable context.
    #[inline]
    pub fn run_binding<Payload, Value, Context, Call, Encode, Decode>(
        &self,
        binding: Binding,
        name: &str,
        requested_payload: ReplayPayload,
        context: &mut Context,
        call: Call,
        encode: Encode,
        decode: Decode,
    ) -> RuntimeResult<Value>
    where
        Payload: Serialize + DeserializeOwned,
        Call: FnOnce(&mut Context) -> RuntimeResult<Value>,
        Encode: FnOnce(&mut Context, &RuntimeResult<Value>) -> RuntimeResult<Option<Payload>>,
        Decode: FnOnce(&mut Context, Payload) -> RuntimeResult<Value>,
    {
        let mode = self.mode();

        match mode {
            // fast execution bypasses replay state entirely
            ExecutionMode::Fast => call(context),
            // replay execution decodes the next recorded payload
            ExecutionMode::Replay => {
                let payload = self.read_binding_payload(binding, name)?;
                decode(context, payload)
            }
            // strict mode validates payload policy, then runs without recording
            ExecutionMode::Strict => {
                self.payload_policy_for_requested(binding, name, requested_payload)?;
                call(context)
            }

            // record mode validates payload policy, executes, then stores payload
            ExecutionMode::Record => {
                self.payload_policy_for_requested(binding, name, requested_payload)?;
                let result = call(context);

                let payload = encode(context, &result)?;
                if let Some(payload) = payload {
                    self.record_binding_payload(binding, name, &payload)?;
                }

                result
            }
        }
    }

    /// Run a binding with replay handling and no explicit mutable context.
    #[inline]
    pub fn run_binding_without_context<Payload, Value, Call, Encode, Decode>(
        &self,
        binding: Binding,
        name: &str,
        requested_payload: ReplayPayload,
        call: Call,
        encode: Encode,
        decode: Decode,
    ) -> RuntimeResult<Value>
    where
        Payload: Serialize + DeserializeOwned,
        Call: FnOnce() -> RuntimeResult<Value>,
        Encode: FnOnce(&RuntimeResult<Value>) -> RuntimeResult<Option<Payload>>,
        Decode: FnOnce(Payload) -> RuntimeResult<Value>,
    {
        let mut context = ();
        self.run_binding(
            binding,
            name,
            requested_payload,
            &mut context,
            move |_| call(),
            move |_, result| encode(result),
            move |_, payload| decode(payload),
        )
    }
}

impl Capture for TraceLog {
    type Image = TraceImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = ();

    /// Capture one trace image.
    fn capture_image(
        &mut self,
        _mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        Ok(TraceLog::capture_image(self))
    }

    /// Restore one trace image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        TraceLog::restore_image(self, image)
    }
}

impl SnapshotCodec for TraceLog {
    type Snapshot = TraceImage;

    /// Encode one trace image as one trace snapshot.
    fn encode_snapshot(image: &Self::Image) -> Result<Self::Snapshot, Self::Error> {
        Ok(image.clone())
    }

    /// Decode one trace snapshot back into one trace image.
    fn decode_snapshot(snapshot: &Self::Snapshot) -> Result<Self::Image, Self::Error> {
        Ok(snapshot.clone())
    }
}
