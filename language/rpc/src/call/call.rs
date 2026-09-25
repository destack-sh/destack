use std::marker::PhantomData;
use std::sync::Arc;

use crossbeam_channel::Receiver;
use tspp_serde::Codec;

use super::{CallError, CallId, Response};
use crate::protocol::message::Completion;
use crate::protocol::payload::{Payload, PayloadError};
use crate::{Connection, ConnectionError, Status};

/// One active client-side RPC invocation.
#[derive(Debug)]
pub struct Call<Reply, Input, Output> {
    /// Owning connection.
    connection: Arc<Connection>,
    /// Caller-scoped identifier.
    id: CallId,
    /// Messages routed to this call.
    receiver: Receiver<CallEvent>,
    /// Output received before its consumer requested it.
    output: Option<Payload>,
    /// Terminal result received from the service.
    completion: Option<Result<Response<Payload>, Status>>,
    /// Whether the service sent a terminal response.
    is_complete: bool,
    /// Whether the caller-to-service stream remains open.
    is_input_open: bool,
    /// Statically typed call values.
    marker: PhantomData<fn(Input) -> (Output, Reply)>,
}

impl<Reply, Input, Output> Call<Reply, Input, Output> {
    /// Create one active call owned by a connection.
    pub(crate) fn new(
        connection: Arc<Connection>,
        id: CallId,
        receiver: Receiver<CallEvent>,
        is_input_open: bool,
    ) -> Self {
        Self {
            connection,
            id,
            receiver,
            output: None,
            completion: None,
            is_complete: false,
            is_input_open,
            marker: PhantomData,
        }
    }

    /// Return this call's caller-scoped identifier.
    pub const fn id(&self) -> CallId {
        self.id
    }

    /// Send one caller-to-service stream item.
    pub fn send(&mut self, input: &Input) -> Result<(), CallError>
    where
        Input: Codec,
    {
        if !self.is_input_open {
            return Err(CallError::Complete);
        }

        let payload = Payload::encode(input).map_err(CallError::Encode)?;
        self.connection.validate_payload(&payload)?;
        let window = self.connection.input_window(self.id)?;
        if !window.acquire() {
            return Err(CallError::Complete);
        }

        self.connection.send_input(self.id, payload)
    }

    /// Close the caller-to-service stream.
    pub fn close_input(&mut self) -> Result<(), CallError> {
        if !self.is_input_open {
            return Ok(());
        }

        self.is_input_open = false;
        self.connection.close_input(self.id, true)
    }

    /// Receive one service-to-caller stream item.
    pub fn receive(&mut self) -> Result<Option<Output>, CallError>
    where
        Output: Codec,
    {
        if let Some(payload) = self.output.take() {
            return self.decode_output(payload).map(Some);
        }
        if self.is_complete {
            return Ok(None);
        }

        // wait until the next output item or terminal response
        match self.receive_event()? {
            CallEvent::Output(payload) => self.decode_output(payload).map(Some),
            CallEvent::Complete(completion) => {
                self.completion = Some(completion.into_result());
                self.finish();

                Ok(None)
            }
            CallEvent::Canceled => {
                self.finish();

                Err(CallError::Canceled)
            }
            CallEvent::Connection(error) => {
                self.finish();

                Err(CallError::Connection(error))
            }
        }
    }

    /// Receive the terminal response after consuming every output item.
    pub fn response(&mut self) -> Result<Response<Reply>, CallError>
    where
        Reply: Codec,
    {
        if self.output.is_some() {
            return Err(CallError::OutputPending);
        }
        if self.is_complete && self.completion.is_none() {
            return Err(CallError::Complete);
        }

        // wait for completion without silently discarding streamed output
        while self.completion.is_none() {
            match self.receive_event()? {
                CallEvent::Output(payload) => {
                    self.output = Some(payload);

                    return Err(CallError::OutputPending);
                }
                CallEvent::Complete(completion) => {
                    self.completion = Some(completion.into_result());
                    self.finish();
                }
                CallEvent::Canceled => {
                    self.finish();

                    return Err(CallError::Canceled);
                }
                CallEvent::Connection(error) => {
                    self.finish();

                    return Err(CallError::Connection(error));
                }
            }
        }

        match self.completion.take() {
            Some(Ok(response)) => {
                let value = response.value.decode().map_err(|error| match error {
                    PayloadError::Decode(error) => CallError::Decode(error),
                    PayloadError::Deferred => {
                        CallError::Connection(Arc::new(ConnectionError::Protocol(
                            "terminal response contains an unresolved payload".to_string(),
                        )))
                    }
                })?;

                Ok(Response {
                    metadata: response.metadata,
                    value,
                })
            }
            Some(Err(status)) => Err(CallError::Status(status)),
            None => Err(CallError::Complete),
        }
    }

    /// Request cancellation of this call.
    pub fn cancel(&mut self) -> Result<(), CallError> {
        if self.is_complete {
            return Err(CallError::Complete);
        }

        self.connection.cancel(self.id)?;
        self.is_input_open = false;

        Ok(())
    }

    /// Receive one routed call event.
    fn receive_event(&self) -> Result<CallEvent, CallError> {
        self.receiver.recv().map_err(|_| CallError::Complete)
    }

    /// Record terminal call state.
    fn finish(&mut self) {
        self.is_complete = true;
        self.is_input_open = false;
    }

    /// Decode and release window for one output item.
    fn decode_output(&self, payload: Payload) -> Result<Output, CallError>
    where
        Output: Codec,
    {
        let output = payload.decode().map_err(|error| match error {
            PayloadError::Decode(error) => CallError::Decode(error),
            PayloadError::Deferred => CallError::Connection(Arc::new(ConnectionError::Protocol(
                "stream item contains an unresolved payload".to_string(),
            ))),
        })?;
        self.connection.release_output(self.id)?;

        Ok(output)
    }
}

impl<Reply, Input, Output> Drop for Call<Reply, Input, Output> {
    /// Abandon this call and cancel unfinished service work.
    fn drop(&mut self) {
        if !self.is_complete {
            self.connection.abandon(self.id);
        }
    }
}

/// One message routed from a connection to an active call.
#[derive(Debug)]
pub(crate) enum CallEvent {
    /// One service-to-caller stream item.
    Output(Payload),
    /// Terminal call completion.
    Complete(Completion),
    /// Peer cancellation.
    Canceled,
    /// Terminal connection failure.
    Connection(Arc<ConnectionError>),
}
