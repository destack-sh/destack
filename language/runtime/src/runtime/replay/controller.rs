use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::bindings::{BindingDescriptor, BindingReplayKind, BindingReplayPayload};
use crate::runtime::replay::{
    BindingCallEvent, RandomEventKind, ReplayEvent, ReplayHeader, ReplayLog, ReplayLogReader,
    TimeEventKind,
};
use crate::runtime::world::WorldCommand;
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

    /// Return the backing replay log.
    pub fn log(&self) -> &ReplayLog {
        &self.log
    }

    /// Return one replay mismatch error for one event channel.
    fn replay_mismatch_error(name: &str) -> Box<RuntimeError> {
        RuntimeError::ReplayMismatch {
            name: name.to_string(),
        }
        .boxed()
    }

    /// Require replay execution mode for one event channel.
    fn ensure_replay_mode(&self, name: &str) -> RuntimeResult<()> {
        if self.mode() == ExecutionMode::Replay {
            return Ok(());
        }

        Err(Self::replay_mismatch_error(name))
    }

    /// Read one required event from replay for one event channel.
    fn next_required_event(&self, name: &str) -> RuntimeResult<ReplayEvent> {
        self.ensure_replay_mode(name)?;

        let Some(event) = self.next_event()? else {
            let sequence = self.log.next_sequence().get();
            return Err(RuntimeError::ReplayLogExhausted { sequence }.boxed());
        };

        Ok(event)
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
        // skip replay when disabled
        if self.mode() != ExecutionMode::Replay {
            return Ok(None);
        }

        // fetch the next event from the reader
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| Self::replay_mismatch_error("replay"))?;
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
        // read the next event from the log
        let event = self.next_required_event(spec.name)?;

        // validate the binding event shape
        let ReplayEvent::BindingCall(call) = event else {
            return Err(Self::replay_mismatch_error(spec.name));
        };

        // validate binding id
        if call.binding_id != spec.id {
            return Err(Self::replay_mismatch_error(spec.name));
        }

        // validate codec id
        if call.codec != spec.codec {
            return Err(Self::replay_mismatch_error(spec.name));
        }

        Ok(call)
    }

    /// Record one runtime world command for replay.
    pub fn record_world_command(&self, command: &WorldCommand) -> RuntimeResult<()> {
        // skip recording when disabled
        if self.mode() != ExecutionMode::Record {
            return Ok(());
        }

        // encode one stable world command payload
        let command_bytes = serde_json::to_vec(command).map_err(|_| {
            RuntimeError::ReplayEncodeFailed {
                name: "world".to_string(),
            }
            .boxed()
        })?;

        // record one world command event
        self.log.record_event(ReplayEvent::WorldCommand {
            payload: command_bytes,
        })?;

        Ok(())
    }

    /// Read the next runtime world command from replay.
    pub fn next_world_command(&self) -> RuntimeResult<WorldCommand> {
        // read the next event from the log
        let event = self.next_required_event("world")?;

        // validate the world command event shape
        let ReplayEvent::WorldCommand {
            payload: command_bytes,
        } = event
        else {
            return Err(Self::replay_mismatch_error("world"));
        };

        serde_json::from_slice(&command_bytes).map_err(|_| {
            RuntimeError::ReplayDecodeFailed {
                name: "world".to_string(),
            }
            .boxed()
        })
    }

    /// Resolve one world command under the active replay mode.
    pub fn resolve_world_command(
        &self,
        requested_command: WorldCommand,
    ) -> RuntimeResult<WorldCommand> {
        match self.mode() {
            // fast execution applies requested command directly
            ExecutionMode::Fast => Ok(requested_command),
            // replay execution aligns requested command with the replay log
            ExecutionMode::Replay => {
                let replayed_command = self.next_world_command()?;
                if replayed_command != requested_command {
                    return Err(Self::replay_mismatch_error("world"));
                }

                Ok(replayed_command)
            }
            // deterministic and record modes keep local command behavior
            ExecutionMode::Deterministic | ExecutionMode::Record => Ok(requested_command),
        }
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
        let mut unit = ();
        self.run_binding_internal(
            spec,
            requested_payload,
            &mut unit,
            move |_| call(),
            move |_, result| encode(result),
            move |_, payload| decode(payload),
        )
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
        self.run_binding_internal(spec, requested_payload, context, call, encode, decode)
    }

    /// Run one binding with replay handling against one mutable context.
    fn run_binding_internal<Payload, Value, Context, Call, Encode, Decode>(
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
            return Err(RuntimeError::ReplayMismatch {
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
                let _ = self.payload_policy_for_requested(spec, requested_payload)?;
                call(context)
            }

            // record mode validates payload policy, executes, then stores payload
            ExecutionMode::Record => {
                let _ = self.payload_policy_for_requested(spec, requested_payload)?;
                let result = call(context);

                let payload = encode(context, &result)?;
                if let Some(payload) = payload {
                    self.record_binding_payload(spec, &payload)?;
                }

                result
            }
        }
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
