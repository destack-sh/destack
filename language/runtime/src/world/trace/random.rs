use crate::diagnostic::RuntimeResult;
use crate::world::random::RandomStreamId;
use crate::world::trace::{EntropySubject, RandomTrace, TraceLog, TraceTag};
use tspp_repository::ExecutionMode;

impl TraceLog {
    /// Run one random u64 binding through the entropy replay channel.
    pub fn run_random_u64<Hook, Call>(
        &self,
        subject: EntropySubject,
        stream_id: RandomStreamId,
        on_replay_read: Hook,
        call: Call,
    ) -> RuntimeResult<u64>
    where
        Hook: FnOnce(),
        Call: FnOnce() -> RuntimeResult<u64>,
    {
        let mode = self.mode();
        match mode {
            // fast and strict modes execute directly
            ExecutionMode::Fast | ExecutionMode::Strict => call(),
            // replay mode decodes one recorded random fact
            ExecutionMode::Replay => {
                on_replay_read();
                let trace = self.next_random_trace(subject)?;

                match trace {
                    RandomTrace::ReadU64 {
                        stream_id: replay_stream_id,
                        outcome,
                        ..
                    } => {
                        if replay_stream_id != stream_id {
                            return Err(self.entropy_mismatch_error());
                        }

                        outcome
                    }
                    _ => Err(self.entropy_mismatch_error()),
                }
            }
            // record mode executes and records one random fact
            ExecutionMode::Record => {
                let result = call();
                let outcome = result.as_ref().map(|value| *value).map_err(Clone::clone);
                let trace = RandomTrace::ReadU64 {
                    subject,
                    stream_id,
                    outcome,
                };
                self.record_payload(TraceTag::Random, "runtime.random.read", &trace)?;

                result
            }
        }
    }

    /// Run one stream allocation binding through the entropy replay channel.
    pub fn run_random_stream<Hook, Call>(
        &self,
        subject: EntropySubject,
        on_replay_read: Hook,
        call: Call,
    ) -> RuntimeResult<u64>
    where
        Hook: FnOnce(),
        Call: FnOnce() -> RuntimeResult<u64>,
    {
        let mode = self.mode();

        match mode {
            // fast and strict modes execute directly
            ExecutionMode::Fast | ExecutionMode::Strict => call(),
            // replay mode decodes one recorded stream allocation
            ExecutionMode::Replay => {
                on_replay_read();
                let trace = self.next_random_trace(subject)?;

                match trace {
                    RandomTrace::StreamCreate { outcome, .. } => {
                        outcome.map(|stream_id| stream_id.get())
                    }
                    _ => Err(self.entropy_mismatch_error()),
                }
            }
            // record mode executes and records one stream allocation
            ExecutionMode::Record => {
                let result = call();
                let outcome = result
                    .as_ref()
                    .map(|value| RandomStreamId::new(*value))
                    .map_err(Clone::clone);
                let trace = RandomTrace::StreamCreate { subject, outcome };
                self.record_payload(TraceTag::Random, "runtime.random.read", &trace)?;

                result
            }
        }
    }

    /// Run one random bytes binding through the entropy replay channel.
    pub fn run_random_bytes<Hook, Call, Encode, Decode>(
        &self,
        subject: EntropySubject,
        stream_id: RandomStreamId,
        requested_len: u32,
        on_replay_read: Hook,
        call: Call,
        encode: Encode,
        decode: Decode,
    ) -> RuntimeResult<()>
    where
        Hook: FnOnce(),
        Call: FnOnce() -> RuntimeResult<()>,
        Encode: FnOnce() -> RuntimeResult<Vec<u8>>,
        Decode: FnOnce(Vec<u8>) -> RuntimeResult<()>,
    {
        let mode = self.mode();
        match mode {
            // fast and strict modes execute directly
            ExecutionMode::Fast | ExecutionMode::Strict => call(),
            // replay mode decodes one recorded byte payload
            ExecutionMode::Replay => {
                on_replay_read();
                let trace = self.next_random_trace(subject)?;

                match trace {
                    RandomTrace::ReadBytes {
                        stream_id: replay_stream_id,
                        len,
                        outcome,
                        ..
                    } => {
                        if replay_stream_id != stream_id || len != requested_len {
                            return Err(self.entropy_mismatch_error());
                        }

                        let bytes = outcome?;
                        decode(bytes)
                    }
                    _ => Err(self.entropy_mismatch_error()),
                }
            }
            // record mode executes and records one byte payload
            ExecutionMode::Record => {
                let result = call();
                let outcome = match result.as_ref() {
                    Ok(()) => encode(),
                    Err(error) => Err(error.clone()),
                };
                let trace = RandomTrace::ReadBytes {
                    subject,
                    stream_id,
                    len: requested_len,
                    outcome,
                };
                self.record_payload(TraceTag::Random, "runtime.random.read", &trace)?;

                result
            }
        }
    }
}
