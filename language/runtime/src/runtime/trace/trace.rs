use destack_core::{Capture, CaptureMode, SnapshotCodec};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::bindings::{BindingDescriptor, BindingReplayKind, BindingReplayPayload};
use crate::runtime::time::WorldInstant;
use crate::runtime::trace::{
    BindingCallEvent, EntropyEvent, Outcome, TraceCursor, TraceCursorImage, TraceHeader, TraceLog,
    TraceLogImage, TraceRecord, TraceSequence,
};
use crate::runtime::world::{BranchId, Command};
use destack_workspace::ExecutionMode;
use parking_lot::Mutex;
use postcard::experimental::serialized_size;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Validation state for trace record ordering.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct Validator {
    /// The last observed monotonic timestamp.
    last_monotonic_nanos: Option<u64>,
}

impl Validator {
    /// Validate one trace record against ordering invariants.
    fn validate(&mut self, record: &TraceRecord) -> RuntimeResult<()> {
        match record {
            TraceRecord::Outcome(Outcome::TimeAdvance(deadline)) => {
                if let Some(last) = self.last_monotonic_nanos
                    && deadline.get() < last
                {
                    return Err(RuntimeError::TraceMismatch {
                        name: "time".to_string(),
                    }
                    .boxed());
                }

                self.last_monotonic_nanos = Some(deadline.get());
            }

            TraceRecord::Outcome(Outcome::Entropy(event)) => match event {
                EntropyEvent::TimeReadMonotonic { outcome, .. } => {
                    if let Ok(time_nanos) = outcome {
                        if let Some(last) = self.last_monotonic_nanos
                            && *time_nanos < last
                        {
                            return Err(RuntimeError::TraceMismatch {
                                name: "entropy".to_string(),
                            }
                            .boxed());
                        }

                        self.last_monotonic_nanos = Some(*time_nanos);
                    }
                }

                EntropyEvent::RandomReadBytes {
                    len,
                    outcome: Ok(bytes),
                    ..
                } => {
                    if bytes.len() != *len as usize {
                        return Err(RuntimeError::TraceMismatch {
                            name: "entropy".to_string(),
                        }
                        .boxed());
                    }
                }

                _ => {}
            },

            _ => {}
        }

        Ok(())
    }
}

/// Materialized trace state captured in one world image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceImage {
    /// The active trace execution mode.
    pub(crate) mode: ExecutionMode,
    /// The captured branch identifier.
    pub(crate) branch_id: BranchId,
    /// The next sequence number after the captured image.
    pub(crate) next_sequence: TraceSequence,
    /// The captured trace-log image.
    pub(crate) log: TraceLogImage,
    /// The captured reader cursor when replay mode is active.
    pub(crate) cursor: Option<TraceCursorImage>,
    /// The captured trace validator state.
    pub(crate) validator: Validator,
}

/// Trace controller for record and replay pipelines.
#[derive(Debug)]
pub struct Trace {
    /// Active execution mode.
    mode: ExecutionMode,
    /// Trace log backing store.
    log: TraceLog,
    /// Trace cursor for log playback.
    reader: Option<TraceCursor>,
    /// Trace ordering validator.
    validator: Mutex<Validator>,
    /// Scratch buffer for trace payload encoding.
    scratch: Mutex<Vec<u8>>,
}

impl Trace {
    /// Create trace state with an explicit execution mode.
    pub fn new(mode: ExecutionMode, header: TraceHeader) -> Self {
        Self::from_log(mode, TraceLog::new(header))
    }

    /// Create trace state with one existing log and execution mode.
    pub fn from_log(mode: ExecutionMode, log: TraceLog) -> Self {
        let reader = match mode {
            ExecutionMode::Replay => Some(log.reader()),
            _ => None,
        };

        Self {
            mode,
            log,
            reader,
            validator: Mutex::new(Validator::default()),
            scratch: Mutex::new(Vec::new()),
        }
    }

    /// Create one replay trace from one captured trace image.
    pub(crate) fn replay_from_image(image: &TraceImage) -> RuntimeResult<Self> {
        let trace = Self::new(ExecutionMode::Replay, image.log.header());
        trace.restore_replay_image(image)?;

        Ok(trace)
    }

    /// Return the active execution mode.
    pub fn mode(&self) -> ExecutionMode {
        if !cfg!(feature = "replay") {
            return ExecutionMode::Fast;
        }
        self.mode
    }

    /// Return the backing trace log.
    pub fn log(&self) -> &TraceLog {
        &self.log
    }

    /// Set the current trace branch identifier.
    pub(crate) fn set_branch_id(&self, branch_id: BranchId) {
        self.log.set_branch_id(branch_id);
    }

    /// Capture one materialized trace image.
    pub(crate) fn capture_image(&self) -> TraceImage {
        let cursor = self.reader.as_ref().map(TraceCursor::capture_image);
        let validator = self.validator.lock().clone();

        TraceImage {
            mode: self.mode,
            branch_id: self.log.branch_id(),
            next_sequence: self.log.next_sequence(),
            log: self.log.image(),
            cursor,
            validator,
        }
    }

    /// Capture one trace image truncated to one exact sequence boundary.
    pub(crate) fn capture_image_through(
        &self,
        sequence: TraceSequence,
    ) -> RuntimeResult<TraceImage> {
        if sequence == self.log.next_sequence() {
            return Ok(self.capture_image());
        }

        if sequence.get() > self.log.next_sequence().get() {
            return Err(RuntimeError::TraceMismatch {
                name: "sequence".to_string(),
            }
            .boxed());
        }

        let header = self.log.header();
        let replay_trace = Trace::from_log(ExecutionMode::Replay, self.log.clone());
        let captured_trace = Trace::new(self.mode, header);
        replay_trace.seek_sequence(TraceSequence::new(0))?;

        while replay_trace.sequence()? != sequence {
            let event = replay_trace.next_event()?.ok_or_else(|| {
                RuntimeError::TraceExhausted {
                    sequence: sequence.get(),
                }
                .boxed()
            })?;
            captured_trace.record_event(event)?;
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

        if image.log.branch_id() != image.branch_id {
            return Err(RuntimeError::Internal {
                message: "trace image branch metadata does not match trace log image".to_string(),
            }
            .boxed());
        }

        if image.log.next_sequence() != image.next_sequence {
            return Err(RuntimeError::Internal {
                message: "trace image sequence metadata does not match trace log image".to_string(),
            }
            .boxed());
        }

        // restore the shared log image first
        self.log.restore_image(image.log.clone());

        // restore the reader cursor when replay is active
        match (&self.reader, image.cursor) {
            (Some(reader), Some(cursor_image)) => {
                reader.restore_image(cursor_image)?;
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

    /// Return one trace mismatch error for one event channel.
    fn trace_mismatch_error(name: &str) -> Box<RuntimeError> {
        RuntimeError::TraceMismatch {
            name: name.to_string(),
        }
        .boxed()
    }

    /// Require replay execution mode for one event channel.
    fn ensure_replay_mode(&self, name: &str) -> RuntimeResult<()> {
        if self.mode() == ExecutionMode::Replay {
            return Ok(());
        }

        Err(Self::trace_mismatch_error(name))
    }

    /// Read one required record from replay for one event channel.
    fn next_required_record(&self, name: &str) -> RuntimeResult<TraceRecord> {
        self.ensure_replay_mode(name)?;

        let Some(record) = self.next_event()? else {
            let sequence = self.log.next_sequence().get();
            return Err(RuntimeError::TraceExhausted { sequence }.boxed());
        };

        Ok(record)
    }

    /// Resolve one requested payload policy for a binding descriptor.
    pub fn payload_policy_for_requested(
        &self,
        spec: BindingDescriptor,
        requested: BindingReplayPayload,
    ) -> RuntimeResult<BindingReplayPayload> {
        let supported = spec.replay_payload();

        if requested == BindingReplayPayload::ArgumentsAndResults
            && supported == BindingReplayPayload::Results
        {
            return Err(RuntimeError::TracePayloadUnsupported {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        match requested {
            BindingReplayPayload::Results => Ok(BindingReplayPayload::Results),
            BindingReplayPayload::ArgumentsAndResults => Ok(supported),
        }
    }

    /// Record one trace record when recording is enabled.
    pub(crate) fn record_event(&self, event: TraceRecord) -> RuntimeResult<()> {
        // skip recording when disabled
        if self.mode() != ExecutionMode::Record {
            return Ok(());
        }

        // record the event in the log
        self.log.record_event(event)?;
        Ok(())
    }

    /// Record one authoritative trace command.
    pub(crate) fn record_command(&self, command: Command) -> RuntimeResult<()> {
        self.record_event(TraceRecord::Command(command))
    }

    /// Record one authoritative trace outcome.
    pub(crate) fn record_outcome(&self, outcome: Outcome) -> RuntimeResult<()> {
        self.record_event(TraceRecord::Outcome(outcome))
    }

    /// Record one authoritative trace anchor and return its assigned sequence.
    pub(crate) fn record_anchor(&self, anchor: String) -> RuntimeResult<TraceSequence> {
        if self.mode() != ExecutionMode::Record {
            return Ok(self.log.next_sequence());
        }

        self.log.record_event(TraceRecord::Anchor(anchor))
    }

    /// Read the next event when replay is enabled.
    pub(crate) fn next_event(&self) -> RuntimeResult<Option<TraceRecord>> {
        // skip replay when disabled
        if self.mode() != ExecutionMode::Replay {
            return Ok(None);
        }

        // fetch the next event from the reader
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| Self::trace_mismatch_error("replay"))?;
        let Some(event) = reader.next_event()? else {
            return Ok(None);
        };

        let mut validator = self.validator.lock();
        validator.validate(&event)?;

        Ok(Some(event))
    }

    /// Return the next trace sequence for the active reader cursor.
    pub fn sequence(&self) -> RuntimeResult<TraceSequence> {
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| Self::trace_mismatch_error("replay"))?;

        Ok(reader.sequence())
    }

    /// Seek the active reader cursor to one sequence boundary.
    pub fn seek_sequence(&self, sequence: TraceSequence) -> RuntimeResult<()> {
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| Self::trace_mismatch_error("replay"))?;

        reader.seek_sequence(sequence)
    }

    /// Record a binding call payload for replay.
    pub fn record_binding_call(
        &self,
        spec: BindingDescriptor,
        payload: &[u8],
    ) -> RuntimeResult<()> {
        self.record_outcome(Outcome::BindingCall(BindingCallEvent {
            binding_id: spec.id,
            codec: spec.codec,
            payload: payload.to_vec(),
        }))
    }

    /// Read the next binding call payload for replay.
    pub fn next_binding_call(&self, spec: BindingDescriptor) -> RuntimeResult<BindingCallEvent> {
        // read the next event from the log
        let event = self.next_required_record(spec.name)?;

        // validate the binding event shape
        let TraceRecord::Outcome(Outcome::BindingCall(call)) = event else {
            return Err(Self::trace_mismatch_error(spec.name));
        };

        // validate binding id
        if call.binding_id != spec.id {
            return Err(Self::trace_mismatch_error(spec.name));
        }

        // validate codec id
        if call.codec != spec.codec {
            return Err(Self::trace_mismatch_error(spec.name));
        }

        Ok(call)
    }

    /// Record one virtual-time advance outcome.
    pub fn record_time_advance(&self, deadline: WorldInstant) -> RuntimeResult<()> {
        self.record_outcome(Outcome::TimeAdvance(deadline))
    }

    /// Record one explicit trace anchor label and return its assigned sequence number.
    pub fn label(&self, label: impl Into<String>) -> RuntimeResult<TraceSequence> {
        self.record_anchor(label.into())
    }

    /// Read the next virtual-time advance outcome from replay.
    pub fn next_time_advance(&self) -> RuntimeResult<WorldInstant> {
        let event = self.next_required_record("time")?;
        let TraceRecord::Outcome(Outcome::TimeAdvance(deadline)) = event else {
            return Err(Self::trace_mismatch_error("time"));
        };
        Ok(deadline)
    }

    /// Resolve one requested virtual-time advance under the active replay mode.
    pub fn resolve_time_advance(
        &self,
        requested_deadline: WorldInstant,
    ) -> RuntimeResult<WorldInstant> {
        match self.mode() {
            // fast, deterministic, and record use the local scheduler decision
            ExecutionMode::Fast | ExecutionMode::Deterministic | ExecutionMode::Record => {
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

    /// Read the next command from replay.
    pub(crate) fn next_command(&self) -> RuntimeResult<Command> {
        // read the next event from the log
        let event = self.next_required_record("world")?;

        // validate the world input event shape
        let TraceRecord::Command(command) = event else {
            return Err(Self::trace_mismatch_error("world"));
        };

        Ok(command)
    }

    /// Resolve one command under the active replay mode.
    pub(crate) fn resolve_command(&self, requested_command: Command) -> RuntimeResult<Command> {
        match self.mode() {
            // fast execution applies the requested command directly
            ExecutionMode::Fast => Ok(requested_command),
            // replay execution aligns the requested command with the replay log
            ExecutionMode::Replay => {
                let replayed_command = self.next_command()?;
                if replayed_command != requested_command {
                    return Err(Self::trace_mismatch_error("world"));
                }

                Ok(replayed_command)
            }
            // deterministic and record modes keep local command behavior
            ExecutionMode::Deterministic | ExecutionMode::Record => Ok(requested_command),
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

        self.log.restore_image(image.log.clone());
        self.seek_sequence(TraceSequence::new(0))?;

        let mut validator = self.validator.lock();
        *validator = image.validator.clone();

        Ok(())
    }

    /// Record a typed trace payload for a binding.
    pub fn record_binding_payload<T: Serialize>(
        &self,
        spec: BindingDescriptor,
        payload: &T,
    ) -> RuntimeResult<()> {
        // skip recording when disabled
        if self.mode() != ExecutionMode::Record {
            return Ok(());
        }

        // encode the payload with the configured codec
        let payload_size = serialized_size(payload).map_err(|_| {
            RuntimeError::TraceEncodeFailed {
                name: spec.name.to_string(),
            }
            .boxed()
        })?;
        let mut scratch = self.scratch.lock();
        scratch.resize(payload_size, 0);
        let payload_bytes = postcard::to_slice(payload, &mut scratch).map_err(|_| {
            RuntimeError::TraceEncodeFailed {
                name: spec.name.to_string(),
            }
            .boxed()
        })?;

        // record the encoded payload
        self.record_binding_call(spec, payload_bytes)
    }

    /// Decode the next typed binding payload for replay.
    pub fn read_binding_payload<T: DeserializeOwned>(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<T> {
        // read the next binding call payload
        let call = self.next_binding_call(spec)?;

        // decode the payload bytes
        let payload = postcard::from_bytes(&call.payload).map_err(|_| {
            RuntimeError::TraceDecodeFailed {
                name: spec.name.to_string(),
            }
            .boxed()
        })?;
        Ok(payload)
    }

    /// Run a binding with replay handling against one mutable context.
    #[inline]
    pub fn run_binding<Payload, Value, Context, Call, Encode, Decode>(
        &self,
        spec: BindingDescriptor,
        requested_payload: BindingReplayPayload,
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
        if spec.replay_kind != BindingReplayKind::BindingCall {
            return Err(RuntimeError::TraceMismatch {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        let mode = self.mode();

        match mode {
            // fast execution bypasses replay state entirely
            ExecutionMode::Fast => call(context),
            // replay execution decodes the next recorded payload
            ExecutionMode::Replay => {
                let payload = self.read_binding_payload(spec)?;
                decode(context, payload)
            }
            // deterministic mode validates payload policy, then runs without recording
            ExecutionMode::Deterministic => {
                self.payload_policy_for_requested(spec, requested_payload)?;
                call(context)
            }

            // record mode validates payload policy, executes, then stores payload
            ExecutionMode::Record => {
                self.payload_policy_for_requested(spec, requested_payload)?;
                let result = call(context);

                let payload = encode(context, &result)?;
                if let Some(payload) = payload {
                    self.record_binding_payload(spec, &payload)?;
                }

                result
            }
        }
    }

    /// Run a binding with replay handling and no explicit mutable context.
    #[inline]
    pub fn run_binding_without_context<Payload, Value, Call, Encode, Decode>(
        &self,
        spec: BindingDescriptor,
        requested_payload: BindingReplayPayload,
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
            spec,
            requested_payload,
            &mut context,
            move |_| call(),
            move |_, result| encode(result),
            move |_, payload| decode(payload),
        )
    }
}

impl TraceImage {
    /// Report whether this image shares the same immutable trace head.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn shares_log_head_with(&self, other: &Self) -> bool {
        self.log.shares_head_with(&other.log)
    }

    /// Report whether this image extends the other image's immutable trace head.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn extends_log_head_of(&self, other: &Self) -> bool {
        self.log.extends_head_of(&other.log)
    }
}

impl Capture for Trace {
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
        Ok(Trace::capture_image(self))
    }

    /// Restore one trace image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        _context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        Trace::restore_image(self, image)
    }
}

impl SnapshotCodec for Trace {
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
