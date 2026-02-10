use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::bindings::ExecutionMode;
use crate::random::RandomStreamId;
use crate::replay::{RandomEvent, RandomEventKind, ReplayController, ReplayEvent};

impl ReplayController {
    /// Read the next random event for replay.
    pub fn next_random_event(&self, expected: RandomEventKind) -> RuntimeResult<RandomEvent> {
        // reject reads outside replay execution
        if self.mode() != ExecutionMode::Replay {
            return Err(RuntimeError::ReplayMismatch {
                name: "random".to_string(),
            }
            .boxed());
        }

        // read the next event from the log
        let Some(event) = self.next_event()? else {
            let sequence = self.log().next_sequence().get();
            return Err(RuntimeError::ReplayLogExhausted { sequence }.boxed());
        };

        // validate the random event
        let ReplayEvent::RandomEvent(random_event) = event else {
            return Err(RuntimeError::ReplayMismatch {
                name: "random".to_string(),
            }
            .boxed());
        };

        // validate the random event kind
        if random_event.kind != expected {
            return Err(RuntimeError::ReplayMismatch {
                name: "random".to_string(),
            }
            .boxed());
        }

        Ok(random_event)
    }

    /// Run a random binding that yields a u64.
    pub fn run_random_u64<Call>(&self, stream_id: RandomStreamId, call: Call) -> RuntimeResult<u64>
    where
        Call: FnOnce() -> RuntimeResult<u64>,
    {
        // fast path
        if !cfg!(feature = "replay") || self.mode() == ExecutionMode::Fast {
            return call();
        }

        // replay path
        if self.mode() == ExecutionMode::Replay {
            let event = self.next_random_event(RandomEventKind::NextU64)?;
            if event.stream_id != stream_id {
                return Err(RuntimeError::ReplayMismatch {
                    name: "random".to_string(),
                }
                .boxed());
            }
            let bytes: [u8; 8] = event.bytes.as_slice().try_into().map_err(|_| {
                RuntimeError::ReplayMismatch {
                    name: "random".to_string(),
                }
                .boxed()
            })?;
            return Ok(u64::from_le_bytes(bytes));
        }

        // record path
        let value = call()?;
        if self.mode() == ExecutionMode::Record {
            self.record_event(ReplayEvent::RandomEvent(RandomEvent {
                stream_id,
                kind: RandomEventKind::NextU64,
                bytes: value.to_le_bytes().to_vec(),
            }))?;
        }

        Ok(value)
    }

    /// Run a random binding that allocates a stream.
    pub fn run_random_stream<Call>(&self, call: Call) -> RuntimeResult<u64>
    where
        Call: FnOnce() -> RuntimeResult<u64>,
    {
        // fast path
        if !cfg!(feature = "replay") || self.mode() == ExecutionMode::Fast {
            return call();
        }

        // replay path
        if self.mode() == ExecutionMode::Replay {
            let event = self.next_random_event(RandomEventKind::Stream)?;
            if !event.bytes.is_empty() {
                return Err(RuntimeError::ReplayMismatch {
                    name: "random".to_string(),
                }
                .boxed());
            }
            return Ok(event.stream_id.get());
        }

        // record path
        let value = call()?;
        if self.mode() == ExecutionMode::Record {
            self.record_event(ReplayEvent::RandomEvent(RandomEvent {
                stream_id: RandomStreamId::new(value),
                kind: RandomEventKind::Stream,
                bytes: Vec::new(),
            }))?;
        }

        Ok(value)
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
        // fast path
        if !cfg!(feature = "replay") || self.mode() == ExecutionMode::Fast {
            return call();
        }

        // replay path
        if self.mode() == ExecutionMode::Replay {
            let event = self.next_random_event(RandomEventKind::Bytes)?;
            if event.stream_id != stream_id {
                return Err(RuntimeError::ReplayMismatch {
                    name: "random".to_string(),
                }
                .boxed());
            }
            return decode(event.bytes);
        }

        // record path
        call()?;
        if self.mode() == ExecutionMode::Record {
            let bytes = encode()?;
            self.record_event(ReplayEvent::RandomEvent(RandomEvent {
                stream_id,
                kind: RandomEventKind::Bytes,
                bytes,
            }))?;
        }

        Ok(())
    }
}
