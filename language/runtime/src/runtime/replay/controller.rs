use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::bindings::{BindingDescriptor, BindingReplayKind, BindingReplayPayload};
use crate::runtime::replay::{
    BindingCallEvent, RandomEventKind, ReplayEvent, ReplayHeader, ReplayLog, ReplayLogReader,
    TimeEventKind,
};
use crate::runtime::{RuntimeHookState, with_current_binding_call_context};
use destack_workspace::ExecutionMode;
use parking_lot::Mutex;
use postcard::experimental::serialized_size;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Validation state for replay event ordering.
#[derive(Debug, Default)]
struct ReplayValidator {
    /// Last observed task queue sequence.
    last_task_queue_sequence: Option<u64>,
    /// Last observed monotonic time sample.
    last_monotonic_nanos: Option<u64>,
}

impl ReplayValidator {
    /// Validate a replay event against ordering invariants.
    fn validate(&mut self, event: &ReplayEvent) -> RuntimeResult<()> {
        match event {
            ReplayEvent::TaskQueueEvent(event) => {
                if let Some(last) = self.last_task_queue_sequence
                    && event.sequence < last
                {
                    return Err(RuntimeError::ReplayMismatch {
                        name: "event_loop".to_string(),
                    }
                    .boxed());
                }
                self.last_task_queue_sequence = Some(event.sequence);
            }
            ReplayEvent::TimeEvent(event) => {
                if event.kind == TimeEventKind::MonotonicSample {
                    if let Some(last) = self.last_monotonic_nanos
                        && event.time_nanos < last
                    {
                        return Err(RuntimeError::ReplayMismatch {
                            name: "time".to_string(),
                        }
                        .boxed());
                    }
                    self.last_monotonic_nanos = Some(event.time_nanos);
                }
            }
            ReplayEvent::RandomEvent(event) => match event.kind {
                RandomEventKind::Stream => {
                    if !event.bytes.is_empty() {
                        return Err(RuntimeError::ReplayMismatch {
                            name: "random".to_string(),
                        }
                        .boxed());
                    }
                }
                RandomEventKind::NextU64 => {
                    if event.bytes.len() != 8 {
                        return Err(RuntimeError::ReplayMismatch {
                            name: "random".to_string(),
                        }
                        .boxed());
                    }
                }
                RandomEventKind::Bytes | RandomEventKind::Seed => {}
            },
            _ => {}
        }

        Ok(())
    }
}

/// Replay controller for record/replay pipelines.
#[derive(Debug)]
pub struct ReplayController {
    // NOTE #Incomplete: validate replay sequences and enforce log compatibility
    /// Active execution mode.
    mode: ExecutionMode,
    /// Replay payload policy for record mode.
    payload_policy: BindingReplayPayload,
    /// Replay log backing store.
    log: ReplayLog,
    /// Replay reader for log playback.
    reader: Option<ReplayLogReader>,
    /// Replay ordering validator.
    validator: Mutex<ReplayValidator>,
    /// Scratch buffer for replay payload encoding.
    scratch: Mutex<Vec<u8>>,
}

impl ReplayController {
    /// Run post-call binding hooks for the current TLS call context.
    fn run_after_binding_hook(spec: BindingDescriptor) {
        let _ = with_current_binding_call_context(|context| {
            context
                .hooks()
                .on_after_binding(spec, RuntimeHookState::from_engine(Some(context.engine())))
        });
    }

    /// Create a replay controller with an explicit execution mode.
    pub fn new(
        mode: ExecutionMode,
        payload_policy: BindingReplayPayload,
        header: ReplayHeader,
    ) -> Self {
        let log = ReplayLog::new(header);
        let reader = match mode {
            ExecutionMode::Replay => Some(log.reader()),
            _ => None,
        };

        Self {
            mode,
            payload_policy,
            log,
            reader,
            validator: Mutex::new(ReplayValidator::default()),
            scratch: Mutex::new(Vec::new()),
        }
    }

    /// Create a replay controller with an existing log and execution mode.
    pub fn from_log(
        mode: ExecutionMode,
        payload_policy: BindingReplayPayload,
        log: ReplayLog,
    ) -> Self {
        let reader = match mode {
            ExecutionMode::Replay => Some(log.reader()),
            _ => None,
        };

        Self {
            mode,
            payload_policy,
            log,
            reader,
            validator: Mutex::new(ReplayValidator::default()),
            scratch: Mutex::new(Vec::new()),
        }
    }

    /// Return the active execution mode.
    pub fn mode(&self) -> ExecutionMode {
        if !cfg!(feature = "replay") {
            return ExecutionMode::Fast;
        }
        self.mode
    }

    /// Return the replay payload policy.
    pub fn payload_policy(&self) -> BindingReplayPayload {
        self.payload_policy
    }

    /// Return whether replay tracking is enabled.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        if !cfg!(feature = "replay") {
            return false;
        }
        self.mode != ExecutionMode::Fast
    }

    /// Return the backing replay log.
    pub fn log(&self) -> &ReplayLog {
        &self.log
    }

    /// Resolve the payload policy for a binding descriptor.
    pub fn payload_policy_for(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<BindingReplayPayload> {
        self.payload_policy_for_requested(spec, self.payload_policy)
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
            return Err(RuntimeError::ReplayPayloadUnsupported {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        match requested {
            BindingReplayPayload::Results => Ok(BindingReplayPayload::Results),
            BindingReplayPayload::ArgumentsAndResults => Ok(supported),
        }
    }

    /// Record an event when replay recording is enabled.
    pub fn record_event(&self, event: ReplayEvent) -> RuntimeResult<()> {
        if !cfg!(feature = "replay") {
            return Ok(());
        }
        // skip recording when disabled
        if self.mode() != ExecutionMode::Record {
            return Ok(());
        }

        // record the event in the log
        self.log.record_event(event)?;
        Ok(())
    }

    /// Read the next event when replay is enabled.
    pub fn next_event(&self) -> RuntimeResult<Option<ReplayEvent>> {
        if !cfg!(feature = "replay") {
            return Ok(None);
        }
        // skip replay when disabled
        if self.mode() != ExecutionMode::Replay {
            return Ok(None);
        }

        // fetch the next event from the reader
        let reader = self.reader.as_ref().ok_or_else(|| {
            RuntimeError::ReplayMismatch {
                name: "replay".to_string(),
            }
            .boxed()
        })?;
        let Some(event) = reader.next_event()? else {
            return Ok(None);
        };

        let mut validator = self.validator.lock();
        validator.validate(&event)?;

        Ok(Some(event))
    }

    /// Record a binding call payload for replay.
    pub fn record_binding_call(
        &self,
        spec: BindingDescriptor,
        payload: &[u8],
    ) -> RuntimeResult<()> {
        if !cfg!(feature = "replay") {
            return Ok(());
        }
        // skip recording when disabled
        if self.mode() != ExecutionMode::Record {
            return Ok(());
        }

        // record the binding call event
        self.log
            .record_event(ReplayEvent::BindingCall(BindingCallEvent {
                binding_id: spec.id,
                codec: spec.codec,
                payload: payload.to_vec(),
            }))?;

        Ok(())
    }

    /// Read the next binding call payload for replay.
    pub fn next_binding_call(&self, spec: BindingDescriptor) -> RuntimeResult<BindingCallEvent> {
        // reject reads outside replay execution
        if self.mode() != ExecutionMode::Replay {
            return Err(RuntimeError::ReplayMismatch {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        // read the next event from the log
        let Some(event) = self.next_event()? else {
            let sequence = self.log.next_sequence().get();
            return Err(RuntimeError::ReplayLogExhausted { sequence }.boxed());
        };

        // validate the binding event shape
        let ReplayEvent::BindingCall(call) = event else {
            return Err(RuntimeError::ReplayMismatch {
                name: spec.name.to_string(),
            }
            .boxed());
        };

        // validate binding id
        if call.binding_id != spec.id {
            return Err(RuntimeError::ReplayMismatch {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        // validate codec id
        if call.codec != spec.codec {
            return Err(RuntimeError::ReplayMismatch {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        Ok(call)
    }

    /// Record a typed replay payload for a binding.
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
            RuntimeError::ReplayEncodeFailed {
                name: spec.name.to_string(),
            }
            .boxed()
        })?;
        let mut scratch = self.scratch.lock();
        scratch.resize(payload_size, 0);
        let payload_bytes = postcard::to_slice(payload, &mut scratch).map_err(|_| {
            RuntimeError::ReplayEncodeFailed {
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
            RuntimeError::ReplayDecodeFailed {
                name: spec.name.to_string(),
            }
            .boxed()
        })?;
        Ok(payload)
    }

    /// Run a binding with replay handling.
    #[inline]
    pub fn run_binding<Payload, Value, Call, Encode, Decode>(
        &self,
        spec: BindingDescriptor,
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
        self.run_binding_with_policy(spec, self.payload_policy, call, encode, decode)
    }

    /// Run a binding with replay handling and one requested payload policy.
    #[inline]
    pub fn run_binding_with_policy<Payload, Value, Call, Encode, Decode>(
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
        if spec.replay_kind != BindingReplayKind::Regular {
            return Err(RuntimeError::ReplayMismatch {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        let mode = self.mode();

        // fast path
        if !cfg!(feature = "replay") || mode == ExecutionMode::Fast {
            let result = call();
            Self::run_after_binding_hook(spec);
            return result;
        }

        // replay path
        if mode == ExecutionMode::Replay {
            let payload = self.read_binding_payload(spec)?;
            let result = decode(payload);
            Self::run_after_binding_hook(spec);
            return result;
        }

        // record path
        let _ = self.payload_policy_for_requested(spec, requested_payload)?;
        let result = call();
        if mode == ExecutionMode::Record {
            let payload = encode(&result)?;
            if let Some(payload) = payload {
                self.record_binding_payload(spec, &payload)?;
            }
        }

        Self::run_after_binding_hook(spec);
        result
    }

    /// Run a binding with replay handling and an explicit mutable context.
    #[inline]
    pub fn run_binding_with_context<Payload, Value, Context, Call, Encode, Decode>(
        &self,
        spec: BindingDescriptor,
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
        self.run_binding_with_context_policy(
            spec,
            self.payload_policy,
            context,
            call,
            encode,
            decode,
        )
    }

    /// Run a binding with replay handling, context, and one requested payload policy.
    #[inline]
    pub fn run_binding_with_context_policy<Payload, Value, Context, Call, Encode, Decode>(
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
        if spec.replay_kind != BindingReplayKind::Regular {
            return Err(RuntimeError::ReplayMismatch {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        let mode = self.mode();

        // fast path
        if !cfg!(feature = "replay") || mode == ExecutionMode::Fast {
            let result = call(context);
            Self::run_after_binding_hook(spec);
            return result;
        }

        // replay path
        if mode == ExecutionMode::Replay {
            let payload = self.read_binding_payload(spec)?;
            let result = decode(context, payload);
            Self::run_after_binding_hook(spec);
            return result;
        }

        // record path
        let _ = self.payload_policy_for_requested(spec, requested_payload)?;
        let result = call(context);
        if mode == ExecutionMode::Record {
            let payload = encode(context, &result)?;
            if let Some(payload) = payload {
                self.record_binding_payload(spec, &payload)?;
            }
        }

        Self::run_after_binding_hook(spec);
        result
    }
}

impl Default for ReplayController {
    fn default() -> Self {
        Self::new(
            ExecutionMode::Fast,
            BindingReplayPayload::Results,
            ReplayHeader::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ReplayController;
    use crate::runtime::bindings::{
        BindingDescriptor, BindingReplayKind, BindingReplayPayload, BindingReplayPolicy,
    };
    use crate::runtime::replay::ReplayHeader;
    use destack_workspace::ExecutionMode;

    #[test]
    /// Recordable binding calls replay in order.
    fn test_record_replay_binding_call() {
        // setup a recordable binding descriptor
        let descriptor = BindingDescriptor::external(
            "destack.test.call",
            "test() -> u64",
            BindingReplayPolicy::Recordable,
            BindingReplayKind::Regular,
        );

        // record a binding call
        let record_state = ReplayController::new(
            ExecutionMode::Record,
            BindingReplayPayload::Results,
            ReplayHeader::default(),
        );
        record_state
            .record_binding_call(descriptor, &[1, 2, 3])
            .expect("record binding call");

        // replay the binding call from the same log
        let replay_state = ReplayController::from_log(
            ExecutionMode::Replay,
            BindingReplayPayload::Results,
            record_state.log().clone(),
        );
        let call = replay_state
            .next_binding_call(descriptor)
            .expect("read binding call");

        // verify the payload matches
        assert_eq!(call.payload, vec![1, 2, 3]);
    }

    #[test]
    /// Stream allocation replays deterministically.
    fn test_record_replay_random_stream() {
        // record a stream allocation
        let record_state = ReplayController::new(
            ExecutionMode::Record,
            BindingReplayPayload::Results,
            ReplayHeader::default(),
        );
        let stream_id = record_state
            .run_random_stream(|| Ok(42))
            .expect("record random stream");
        assert_eq!(stream_id, 42);

        // replay the stream allocation
        let replay_state = ReplayController::from_log(
            ExecutionMode::Replay,
            BindingReplayPayload::Results,
            record_state.log().clone(),
        );
        let replayed = replay_state
            .run_random_stream(|| Ok(7))
            .expect("replay random stream");

        // verify the replayed value matches the recorded one
        assert_eq!(replayed, 42);
    }
}
