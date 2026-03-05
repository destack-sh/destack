use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::bindings::{BindingDescriptor, BindingReplayKind, BindingReplayPayload};
use crate::runtime::replay::{
    BindingCallEvent, ReplayEvent, ReplayHeader, ReplayLog, ReplayLogReader,
};
use crate::runtime::world::WorldCommand;
use destack_workspace::ExecutionMode;
use parking_lot::Mutex;
use postcard::experimental::serialized_size;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::validator::ReplayValidator;

/// Replay state for record/replay pipelines.
#[derive(Debug)]
pub struct Replay {
    /// TODO #Incomplete: validate replay sequences and enforce log compatibility
    /// Active execution mode.
    mode: ExecutionMode,
    /// Replay log backing store.
    log: ReplayLog,
    /// Replay reader for log playback.
    reader: Option<ReplayLogReader>,
    /// Replay ordering validator.
    validator: Mutex<ReplayValidator>,
    /// Scratch buffer for replay payload encoding.
    scratch: Mutex<Vec<u8>>,
}

impl Replay {
    /// Create replay state with an explicit execution mode.
    pub fn new(
        mode: ExecutionMode,
        header: ReplayHeader,
    ) -> Self {
        Self::from_log(mode, ReplayLog::new(header))
    }

    /// Create replay state with one existing log and execution mode.
    pub fn from_log(mode: ExecutionMode, log: ReplayLog) -> Self {
        let reader = match mode {
            ExecutionMode::Replay => Some(log.reader()),
            _ => None,
        };

        Self {
            mode,
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
        self.record_event(ReplayEvent::BindingCall(BindingCallEvent {
            binding_id: spec.id,
            codec: spec.codec,
            payload: payload.to_vec(),
        }))
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
        // encode one stable world command payload
        let command_bytes = serde_json::to_vec(command).map_err(|_| {
            RuntimeError::ReplayEncodeFailed {
                name: "world".to_string(),
            }
            .boxed()
        })?;

        // record one world command event
        self.record_event(ReplayEvent::WorldCommand {
            payload: command_bytes,
        })
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

impl Default for Replay {
    fn default() -> Self {
        Self::new(
            ExecutionMode::Fast,
            ReplayHeader::default(),
        )
    }
}
