use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::random::RandomStreamId;
use crate::world::trace::{EntropySample, EntropySubject, Outcome, Trace, TraceError, TraceRecord};
use destack_workspace::ExecutionMode;

impl Trace {
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
            // replay mode decodes one recorded entropy sample
            ExecutionMode::Replay => {
                on_replay_read();
                let event = self.next_entropy_sample(subject)?;

                match event {
                    EntropySample::RandomReadU64 {
                        stream_id: replay_stream_id,
                        outcome,
                        ..
                    } => {
                        if replay_stream_id != stream_id {
                            return Err(self.entropy_mismatch_error());
                        }

                        outcome.map_err(Box::<RuntimeError>::from)
                    }
                    _ => Err(self.entropy_mismatch_error()),
                }
            }
            // record mode executes and records one entropy sample
            ExecutionMode::Record => {
                let result = call();
                let outcome = result
                    .as_ref()
                    .map(|value| *value)
                    .map_err(|error| TraceError::from(error.as_ref()));
                self.record_event(TraceRecord::Outcome(Outcome::Entropy(
                    EntropySample::RandomReadU64 {
                        subject,
                        stream_id,
                        outcome,
                    },
                )))?;

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
                let event = self.next_entropy_sample(subject)?;

                match event {
                    EntropySample::RandomStreamCreate { outcome, .. } => outcome
                        .map(|stream_id| stream_id.get())
                        .map_err(Box::<RuntimeError>::from),
                    _ => Err(self.entropy_mismatch_error()),
                }
            }
            // record mode executes and records one stream allocation
            ExecutionMode::Record => {
                let result = call();
                let outcome = result
                    .as_ref()
                    .map(|value| RandomStreamId::new(*value))
                    .map_err(|error| TraceError::from(error.as_ref()));
                self.record_event(TraceRecord::Outcome(Outcome::Entropy(
                    EntropySample::RandomStreamCreate { subject, outcome },
                )))?;

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
                let event = self.next_entropy_sample(subject)?;

                match event {
                    EntropySample::RandomReadBytes {
                        stream_id: replay_stream_id,
                        len,
                        outcome,
                        ..
                    } => {
                        if replay_stream_id != stream_id || len != requested_len {
                            return Err(self.entropy_mismatch_error());
                        }

                        let bytes = outcome.map_err(Box::<RuntimeError>::from)?;
                        decode(bytes)
                    }
                    _ => Err(self.entropy_mismatch_error()),
                }
            }
            // record mode executes and records one byte payload
            ExecutionMode::Record => {
                let result = call();
                let outcome = match result.as_ref() {
                    Ok(()) => encode().map_err(|error| TraceError::from(error.as_ref())),
                    Err(error) => Err(TraceError::from(error.as_ref())),
                };
                self.record_event(TraceRecord::Outcome(Outcome::Entropy(
                    EntropySample::RandomReadBytes {
                        subject,
                        stream_id,
                        len: requested_len,
                        outcome,
                    },
                )))?;

                result
            }
        }
    }
}
