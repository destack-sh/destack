use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::bindings::{
    BindingDescriptor, BindingReplayKind, ExecutionMode, ReplayPayload,
};
use crate::replay::{BindingCallEvent, ReplayEvent, ReplayHeader, ReplayLog};
use postcard::experimental::serialized_size;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Replay controller for record/replay pipelines.
#[derive(Debug, Clone)]
pub struct ReplayController {
    // NOTE #Incomplete: validate replay sequences and enforce log compatibility
    /// Active replay mode.
    mode: ExecutionMode,
    /// Replay payload policy for record mode.
    payload_policy: ReplayPayload,
    /// Replay log backing store.
    log: ReplayLog,
}

impl ReplayController {
    /// Create a replay controller with an explicit mode.
    pub fn new(mode: ExecutionMode, payload_policy: ReplayPayload, header: ReplayHeader) -> Self {
        Self {
            mode,
            payload_policy,
            log: ReplayLog::new(header),
        }
    }

    /// Create a replay controller with an existing log.
    pub fn from_log(mode: ExecutionMode, payload_policy: ReplayPayload, log: ReplayLog) -> Self {
        Self {
            mode,
            payload_policy,
            log,
        }
    }

    /// Return the active replay mode.
    pub fn mode(&self) -> ExecutionMode {
        if !cfg!(feature = "replay") {
            return ExecutionMode::Fast;
        }
        self.mode
    }

    /// Return the replay payload policy.
    pub fn payload_policy(&self) -> ReplayPayload {
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
    pub fn payload_policy_for(&self, spec: BindingDescriptor) -> RuntimeResult<ReplayPayload> {
        let requested = self.payload_policy;
        let supported = spec.replay_payload();

        if requested == ReplayPayload::ArgumentsAndResults
            && supported == ReplayPayload::Results
        {
            return Err(RuntimeError::ReplayPayloadUnsupported {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        match requested {
            ReplayPayload::Results => Ok(ReplayPayload::Results),
            ReplayPayload::ArgumentsAndResults => Ok(supported),
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

        // fetch the next event from the log
        self.log.next_event()
    }

    /// Record a binding call payload for replay.
    pub fn record_binding_call(
        &self,
        spec: BindingDescriptor,
        payload: Vec<u8>,
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
                payload,
            }))?;

        Ok(())
    }

    /// Read the next binding call payload for replay.
    pub fn next_binding_call(&self, spec: BindingDescriptor) -> RuntimeResult<BindingCallEvent> {
        // reject reads outside replay mode
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
        if self.mode != ExecutionMode::Record {
            return Ok(());
        }

        // encode the payload with the configured codec
        let payload_size = serialized_size(payload).map_err(|_| {
            RuntimeError::ReplayEncodeFailed {
                name: spec.name.to_string(),
            }
            .boxed()
        })?;
        let mut payload_bytes = vec![0u8; payload_size];
        postcard::to_slice(payload, &mut payload_bytes).map_err(|_| {
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
        if spec.replay_kind != BindingReplayKind::Regular {
            return Err(RuntimeError::ReplayMismatch {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        // fast path
        if !cfg!(feature = "replay") || self.mode() == ExecutionMode::Fast {
            return call();
        }

        // replay path
        if self.mode() == ExecutionMode::Replay {
            let payload = self.read_binding_payload(spec)?;
            return decode(payload);
        }

        // record path
        let result = call();
        if self.mode() == ExecutionMode::Record {
            let payload = encode(&result)?;
            if let Some(payload) = payload {
                self.record_binding_payload(spec, &payload)?;
            }
        }

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
        if spec.replay_kind != BindingReplayKind::Regular {
            return Err(RuntimeError::ReplayMismatch {
                name: spec.name.to_string(),
            }
            .boxed());
        }

        // fast path
        if !cfg!(feature = "replay") || self.mode() == ExecutionMode::Fast {
            return call(context);
        }

        // replay path
        if self.mode() == ExecutionMode::Replay {
            let payload = self.read_binding_payload(spec)?;
            return decode(context, payload);
        }

        // record path
        let result = call(context);
        if self.mode() == ExecutionMode::Record {
            let payload = encode(context, &result)?;
            if let Some(payload) = payload {
                self.record_binding_payload(spec, &payload)?;
            }
        }

        result
    }
}

impl Default for ReplayController {
    fn default() -> Self {
        Self::new(
            ExecutionMode::Fast,
            ReplayPayload::Results,
            ReplayHeader::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ReplayController;
    use crate::platform::bindings::{
        BindingDescriptor, BindingReplayKind, ExecutionMode, ReplayPayload, ReplayPolicy,
    };
    use crate::replay::ReplayHeader;

    #[test]
    /// Recordable binding calls replay in order.
    fn test_record_replay_binding_call() {
        // setup a recordable binding descriptor
        let descriptor = BindingDescriptor::external(
            "destack.test.call",
            "test() -> u64",
            ReplayPolicy::Recordable,
            BindingReplayKind::Regular,
        );

        // record a binding call
        let record_state = ReplayController::new(
            ExecutionMode::Record,
            ReplayPayload::Results,
            ReplayHeader::default(),
        );
        record_state
            .record_binding_call(descriptor, vec![1, 2, 3])
            .expect("record binding call");

        // replay the binding call from the same log
        let replay_state = ReplayController::from_log(
            ExecutionMode::Replay,
            ReplayPayload::Results,
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
            ReplayPayload::Results,
            ReplayHeader::default(),
        );
        let stream_id = record_state
            .run_random_stream(|| Ok(42))
            .expect("record random stream");
        assert_eq!(stream_id, 42);

        // replay the stream allocation
        let replay_state = ReplayController::from_log(
            ExecutionMode::Replay,
            ReplayPayload::Results,
            record_state.log().clone(),
        );
        let replayed = replay_state
            .run_random_stream(|| Ok(7))
            .expect("replay random stream");

        // verify the replayed value matches the recorded one
        assert_eq!(replayed, 42);
    }
}
