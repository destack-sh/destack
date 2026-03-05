use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::random::RandomStreamId;
use crate::runtime::replay::{RandomEvent, RandomEventKind, ReplayController, ReplayEvent};
use destack_workspace::ExecutionMode;

/// Replay channel name for random events.
const RANDOM_CHANNEL: &str = "random";

/// Return one replay mismatch error for the random channel.
fn random_mismatch_error() -> Box<RuntimeError> {
    RuntimeError::ReplayMismatch {
        name: RANDOM_CHANNEL.to_string(),
    }
    .boxed()
}

impl ReplayController {
    /// Read the next random event for replay.
    pub fn next_random_event(&self, expected: RandomEventKind) -> RuntimeResult<RandomEvent> {
        // reject reads outside replay execution
        if self.mode() != ExecutionMode::Replay {
            return Err(random_mismatch_error());
        }

        // read the next event from the log
        let Some(event) = self.next_event()? else {
            let sequence = self.log().next_sequence().get();
            return Err(RuntimeError::ReplayLogExhausted { sequence }.boxed());
        };

        // validate the random event
        let ReplayEvent::RandomEvent(random_event) = event else {
            return Err(random_mismatch_error());
        };

        // validate the random event kind
        if random_event.kind != expected {
            return Err(random_mismatch_error());
        }

        Ok(random_event)
    }

    /// Run a random binding that yields a u64.
    pub fn run_random_u64<Call>(&self, stream_id: RandomStreamId, call: Call) -> RuntimeResult<u64>
    where
        Call: FnOnce() -> RuntimeResult<u64>,
    {
        let mode = self.mode();

        match mode {
            // fast and deterministic modes execute directly
            ExecutionMode::Fast | ExecutionMode::Deterministic => call(),
            // replay mode decodes one recorded u64 sample
            ExecutionMode::Replay => {
                let event = self.next_random_event(RandomEventKind::NextU64)?;
                if event.stream_id != stream_id {
                    return Err(random_mismatch_error());
                }

                let bytes: [u8; 8] = event
                    .bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| random_mismatch_error())?;

                Ok(u64::from_le_bytes(bytes))
            }
            // record mode executes and records one u64 sample
            ExecutionMode::Record => {
                let value = call()?;
                self.record_event(ReplayEvent::RandomEvent(RandomEvent {
                    stream_id,
                    kind: RandomEventKind::NextU64,
                    bytes: value.to_le_bytes().to_vec(),
                }))?;

                Ok(value)
            }
        }
    }

    /// Run a random binding that allocates a stream.
    pub fn run_random_stream<Call>(&self, call: Call) -> RuntimeResult<u64>
    where
        Call: FnOnce() -> RuntimeResult<u64>,
    {
        let mode = self.mode();

        match mode {
            // fast and deterministic modes execute directly
            ExecutionMode::Fast | ExecutionMode::Deterministic => call(),
            // replay mode decodes one recorded stream allocation
            ExecutionMode::Replay => {
                let event = self.next_random_event(RandomEventKind::Stream)?;
                if !event.bytes.is_empty() {
                    return Err(random_mismatch_error());
                }

                Ok(event.stream_id.get())
            }
            // record mode executes and records one stream allocation
            ExecutionMode::Record => {
                let value = call()?;
                self.record_event(ReplayEvent::RandomEvent(RandomEvent {
                    stream_id: RandomStreamId::new(value),
                    kind: RandomEventKind::Stream,
                    bytes: Vec::new(),
                }))?;

                Ok(value)
            }
        }
    }

    /// Run a random binding that yields or fills bytes.
    pub fn run_random_bytes<Call, Encode, Decode>(
        &self,
        stream_id: RandomStreamId,
        call: Call,
        encode: Encode,
        decode: Decode,
    ) -> RuntimeResult<()>
    where
        Call: FnOnce() -> RuntimeResult<()>,
        Encode: FnOnce() -> RuntimeResult<Vec<u8>>,
        Decode: FnOnce(Vec<u8>) -> RuntimeResult<()>,
    {
        let mode = self.mode();

        match mode {
            // fast and deterministic modes execute directly
            ExecutionMode::Fast | ExecutionMode::Deterministic => call(),
            // replay mode decodes one recorded byte payload
            ExecutionMode::Replay => {
                let event = self.next_random_event(RandomEventKind::Bytes)?;
                if event.stream_id != stream_id {
                    return Err(random_mismatch_error());
                }

                decode(event.bytes)
            }
            // record mode executes and records one byte payload
            ExecutionMode::Record => {
                call()?;

                let bytes = encode()?;
                self.record_event(ReplayEvent::RandomEvent(RandomEvent {
                    stream_id,
                    kind: RandomEventKind::Bytes,
                    bytes,
                }))?;

                Ok(())
            }
        }
    }
}
