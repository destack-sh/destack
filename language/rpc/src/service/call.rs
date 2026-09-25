use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::task::{Context, Poll};

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures::task::AtomicWaker;
use parking_lot::Mutex;
use tspp_serde::Codec;

use super::{MethodId, MethodKind, RequestStream, ResponseSender};
use crate::call::{CallId, SendWindow};
use crate::connection::MessageSender;
use crate::protocol::message::{Completion, Message, StreamItem, WindowUpdate};
use crate::protocol::payload::{Payload, PayloadError};
use crate::protocol::{Code, Status};
use crate::{ConnectionError, Request, Response};

/// One active call being handled by an RPC service.
#[derive(Debug)]
pub struct ServerCall {
    /// Shared call state routed by the server.
    state: Arc<CallState>,
    /// Encoded initial request.
    request: Request<Payload>,
    /// Caller-to-service stream events.
    receiver: Option<UnboundedReceiver<ServerEvent>>,
    /// Whether the service-to-caller stream sender was taken.
    has_response_sender: bool,
}

impl ServerCall {
    /// Create one server call from its wire request.
    pub(crate) fn new(
        id: CallId,
        method: MethodId,
        request: Request<Payload>,
        sender: MessageSender,
        kind: MethodKind,
        stream_window: u32,
    ) -> (Self, Arc<CallState>) {
        let (event_sender, receiver) = unbounded();
        let state = Arc::new(CallState {
            id,
            method,
            sender,
            events: event_sender,
            output_window: SendWindow::new(if kind.has_output() { stream_window } else { 0 }),
            input_window: AtomicU32::new(if kind.has_input() { stream_window } else { 0 }),
            kind,
            is_input_closed: AtomicBool::new(!kind.has_input()),
            is_canceled: AtomicBool::new(false),
            is_complete: AtomicBool::new(false),
            cancellation_waker: AtomicWaker::new(),
            output_sending: Mutex::new(()),
        });
        let call = Self {
            state: state.clone(),
            request,
            receiver: Some(receiver),
            has_response_sender: false,
        };

        (call, state)
    }

    /// Return this call's caller-scoped identifier.
    pub fn id(&self) -> CallId {
        self.state.id
    }

    /// Return the called method identifier.
    pub fn method(&self) -> MethodId {
        self.state.method
    }

    /// Decode the initial request value.
    pub fn decode<T: Codec>(&self) -> Result<T, ServiceError> {
        self.decode_request().map(Request::into_inner)
    }

    /// Decode the initial request and preserve its call options.
    pub fn decode_request<T: Codec>(&self) -> Result<Request<T>, ServiceError> {
        let value = self
            .request
            .value
            .decode()
            .map_err(ServiceError::from_payload)?;

        Ok(Request::from_parts(self.request.metadata.clone(), value))
    }

    /// Take this call's typed caller-to-service stream.
    pub fn request_stream<T>(&mut self) -> Result<RequestStream<T>, ServiceError> {
        if !self.state.kind.has_input() {
            return Err(ServiceError::Status(Status::new(
                Code::FailedPrecondition,
                "method has no input stream",
            )));
        }
        let receiver = self.receiver.take().ok_or_else(|| {
            ServiceError::Status(Status::new(
                Code::FailedPrecondition,
                "request stream was already taken",
            ))
        })?;

        Ok(RequestStream::new(self.state.clone(), receiver))
    }

    /// Take this call's typed service-to-caller sender.
    pub fn response_sender<T>(&mut self) -> Result<ResponseSender<T>, ServiceError> {
        if !self.state.kind.has_output() {
            return Err(ServiceError::Status(Status::new(
                Code::FailedPrecondition,
                "method has no output stream",
            )));
        }
        if self.has_response_sender {
            return Err(ServiceError::Status(Status::new(
                Code::FailedPrecondition,
                "response sender was already taken",
            )));
        }

        self.has_response_sender = true;

        Ok(ResponseSender::new(self.state.clone()))
    }

    /// Complete this call successfully.
    pub fn respond<T: Codec>(&self, response: Response<T>) -> Result<(), ServiceError> {
        if self.state.is_canceled.load(Ordering::Acquire) {
            return Err(ServiceError::Canceled);
        }

        let payload = Payload::encode(&response.value).map_err(ServiceError::Encode)?;
        self.state.validate_payload(&payload)?;
        let response = Response {
            metadata: response.metadata,
            value: payload,
        };
        let completion = Completion::Response {
            call: self.state.id,
            response,
        };
        self.state.complete(completion)
    }

    /// Complete this call unsuccessfully.
    pub fn fail(&self, status: Status) -> Result<(), ServiceError> {
        let completion = Completion::Status {
            call: self.state.id,
            status,
        };
        self.state.complete(completion)
    }

    /// Return whether the caller requested cancellation.
    pub fn is_canceled(&self) -> bool {
        self.state.is_canceled.load(Ordering::Acquire)
    }
}

/// Failure while handling one service call.
#[derive(Debug)]
pub enum ServiceError {
    /// The service rejected the call.
    Status(Status),
    /// A response or stream value could not be encoded.
    Encode(tspp_serde::Error),
    /// A request or stream value could not be decoded.
    Decode(tspp_serde::Error),
    /// The RPC connection failed.
    Connection(Arc<ConnectionError>),
    /// The call is already complete.
    Complete,
    /// The caller canceled the request.
    Canceled,
}

impl ServiceError {
    /// Convert this handler failure into one terminal status.
    pub fn into_status(self) -> Status {
        match self {
            Self::Status(status) => status,
            Self::Encode(error) => Status::new(Code::Internal, error.to_string()),
            Self::Decode(error) => Status::new(Code::InvalidArgument, error.to_string()),
            Self::Connection(error) => Status::new(Code::Unavailable, error.to_string()),
            Self::Complete => Status::new(Code::FailedPrecondition, "RPC call is complete"),
            Self::Canceled => Status::new(Code::Canceled, "RPC call was canceled"),
        }
    }

    /// Convert one encoded payload failure.
    pub(super) fn from_payload(error: PayloadError) -> Self {
        match error {
            PayloadError::Decode(error) => Self::Decode(error),
            PayloadError::Deferred => Self::Status(Status::new(
                Code::DataLoss,
                "request contains an unresolved payload",
            )),
        }
    }
}

impl std::fmt::Display for ServiceError {
    /// Format this service failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Status(status) => write!(formatter, "{status}"),
            Self::Encode(error) => write!(formatter, "RPC value encode failed: {error}"),
            Self::Decode(error) => write!(formatter, "RPC value decode failed: {error}"),
            Self::Connection(error) => write!(formatter, "{error}"),
            Self::Complete => write!(formatter, "RPC call is complete"),
            Self::Canceled => write!(formatter, "RPC call was canceled"),
        }
    }
}

impl std::error::Error for ServiceError {
    /// Return the underlying service failure when one exists.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Status(status) => Some(status),
            Self::Encode(error) | Self::Decode(error) => Some(error),
            Self::Connection(error) => Some(error.as_ref()),
            Self::Complete | Self::Canceled => None,
        }
    }
}

impl From<Status> for ServiceError {
    /// Convert one terminal service status.
    fn from(status: Status) -> Self {
        Self::Status(status)
    }
}

impl From<Arc<ConnectionError>> for ServiceError {
    /// Convert one connection failure.
    fn from(error: Arc<ConnectionError>) -> Self {
        Self::Connection(error)
    }
}

impl From<ConnectionError> for ServiceError {
    /// Convert one owned connection failure.
    fn from(error: ConnectionError) -> Self {
        Self::Connection(Arc::new(error))
    }
}

/// Shared state for one call routed by the server.
#[derive(Debug)]
pub(crate) struct CallState {
    /// Caller-scoped call identifier.
    id: CallId,
    /// Called method identifier.
    method: MethodId,
    /// Called method streaming kind.
    kind: MethodKind,
    /// Connection message sender.
    sender: MessageSender,
    /// Service request event sender.
    events: UnboundedSender<ServerEvent>,
    /// Remaining service-to-caller send window.
    output_window: SendWindow,
    /// Remaining caller-to-service receive window.
    input_window: AtomicU32,
    /// Whether the caller closed its input stream.
    is_input_closed: AtomicBool,
    /// Whether the caller canceled the request.
    is_canceled: AtomicBool,
    /// Whether a terminal response was sent.
    is_complete: AtomicBool,
    /// Service task waiting for caller cancellation.
    cancellation_waker: AtomicWaker,
    /// Serializes output items with terminal completion.
    output_sending: Mutex<()>,
}

impl CallState {
    /// Route one caller-to-service stream item.
    pub(crate) fn push(&self, payload: Payload) -> Result<(), Status> {
        if !self.kind.has_input() {
            return Err(Status::new(
                Code::FailedPrecondition,
                "method has no input stream",
            ));
        }
        if self.is_input_closed.load(Ordering::Acquire) {
            return Err(Status::new(
                Code::FailedPrecondition,
                "input stream is closed",
            ));
        }
        self.input_window
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |window| {
                window.checked_sub(1)
            })
            .map_err(|_| {
                Status::new(Code::ResourceExhausted, "input stream exceeded its window")
            })?;

        // discard windowed input that crossed the terminal response in flight
        if self.is_complete.load(Ordering::Acquire) {
            Ok(())
        } else {
            self.events
                .unbounded_send(ServerEvent::Input(payload))
                .map_err(|_| Status::new(Code::Canceled, "request is no longer active"))
        }
    }

    /// Route caller input closure.
    pub(crate) fn close_input(&self) -> Result<(), Status> {
        let was_closed = self.is_input_closed.swap(true, Ordering::AcqRel);
        if was_closed {
            return Err(Status::new(
                Code::FailedPrecondition,
                "input stream is already closed",
            ));
        }

        // input closure after completion only retires the routed request
        if self.is_complete.load(Ordering::Acquire) {
            return Ok(());
        }

        self.events
            .unbounded_send(ServerEvent::Close)
            .map_err(|_| Status::new(Code::Canceled, "request is no longer active"))
    }

    /// Extend the service-to-caller stream window.
    pub(crate) fn update_output_window(&self, items: u32) -> Result<(), Status> {
        if items == 0 {
            return Err(Status::new(
                Code::InvalidArgument,
                "stream window must be positive",
            ));
        }

        if self.output_window.try_update(items) {
            Ok(())
        } else {
            Err(Status::new(
                Code::OutOfRange,
                "output stream window overflowed",
            ))
        }
    }

    /// Route caller cancellation.
    pub(crate) fn cancel(&self) -> Result<(), Status> {
        if self.is_complete.load(Ordering::Acquire) {
            return Ok(());
        }
        let was_canceled = self.is_canceled.swap(true, Ordering::AcqRel);
        if was_canceled {
            return Ok(());
        }

        self.is_input_closed.store(true, Ordering::Release);
        self.output_window.close();
        self.cancellation_waker.wake();

        if self.kind.has_input() {
            self.events
                .unbounded_send(ServerEvent::Cancel)
                .map_err(|_| Status::new(Code::Canceled, "request is no longer active"))
        } else {
            Ok(())
        }
    }

    /// Wake this request after its connection has closed.
    pub(crate) fn disconnect(&self) {
        self.is_canceled.store(true, Ordering::Release);
        self.output_window.close();
        self.events.close_channel();
        self.cancellation_waker.wake();
    }

    /// Return whether the request sent a terminal response.
    pub(crate) fn is_complete(&self) -> bool {
        self.is_complete.load(Ordering::Acquire)
    }

    /// Return whether caller input may still arrive.
    pub(crate) fn is_input_open(&self) -> bool {
        !self.is_input_closed.load(Ordering::Acquire)
    }

    /// Return whether the caller requested cancellation.
    pub(super) fn is_canceled(&self) -> bool {
        self.is_canceled.load(Ordering::Acquire)
    }

    /// Poll until the caller cancels this request.
    pub(super) fn poll_canceled(&self, context: &mut Context<'_>) -> Poll<()> {
        if self.is_canceled() {
            return Poll::Ready(());
        }

        self.cancellation_waker.register(context.waker());

        if self.is_canceled() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    /// Complete the request with one terminal response.
    pub(crate) fn complete(&self, completion: Completion) -> Result<(), ServiceError> {
        let _sending = self.output_sending.lock();
        let completed = self
            .is_complete
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();
        if !completed {
            return Err(ServiceError::Complete);
        }

        self.output_window.close();
        self.sender.send(Message::Complete(completion))?;

        Ok(())
    }

    /// Release one consumed caller-to-service stream item.
    pub(super) fn release_input(&self) -> Result<(), ServiceError> {
        self.input_window
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |window| {
                window.checked_add(1)
            })
            .map_err(|_| {
                ServiceError::Status(Status::new(
                    Code::OutOfRange,
                    "input stream window overflowed",
                ))
            })?;
        let update = WindowUpdate {
            call: self.id,
            items: 1,
        };
        self.sender.send(Message::Window(update))?;

        Ok(())
    }

    /// Send one output after asynchronously acquiring stream window.
    pub(super) async fn send<T: Codec>(&self, output: &T) -> Result<(), ServiceError> {
        self.validate_output()?;
        let payload = Payload::encode(output).map_err(ServiceError::Encode)?;
        self.validate_payload(&payload)?;
        if !self.output_window.acquire_async().await {
            return Err(ServiceError::Complete);
        }

        self.send_output(payload)
    }

    /// Send one output only when stream window is immediately available.
    pub(super) fn try_send<T: Codec>(&self, output: &T) -> Result<(), ServiceError> {
        self.validate_output()?;
        let payload = Payload::encode(output).map_err(ServiceError::Encode)?;
        self.validate_payload(&payload)?;
        if !self.output_window.try_acquire() {
            return Err(ServiceError::Status(Status::new(
                Code::ResourceExhausted,
                "output stream has no available window",
            )));
        }

        self.send_output(payload)
    }

    /// Validate one service-to-caller stream send.
    fn validate_output(&self) -> Result<(), ServiceError> {
        if !self.kind.has_output() {
            return Err(ServiceError::Status(Status::new(
                Code::FailedPrecondition,
                "method has no output stream",
            )));
        }
        if self.is_canceled.load(Ordering::Acquire) {
            return Err(ServiceError::Canceled);
        }
        if self.is_complete.load(Ordering::Acquire) {
            return Err(ServiceError::Complete);
        }

        Ok(())
    }

    /// Enforce the negotiated assembled payload limit before consuming stream window.
    fn validate_payload(&self, payload: &Payload) -> Result<(), ServiceError> {
        let bytes = payload.inline().map_err(ServiceError::from_payload)?;
        let limit = self.sender.max_payload_bytes();
        if bytes.len() > limit {
            Err(ServiceError::Status(Status::new(
                Code::ResourceExhausted,
                format!("RPC payload exceeds {limit} bytes: {}", bytes.len()),
            )))
        } else {
            Ok(())
        }
    }

    /// Send one encoded service-to-caller stream item within the peer window.
    fn send_output(&self, payload: Payload) -> Result<(), ServiceError> {
        let _sending = self.output_sending.lock();
        if self.is_canceled.load(Ordering::Acquire) {
            return Err(ServiceError::Canceled);
        }
        if self.is_complete.load(Ordering::Acquire) {
            return Err(ServiceError::Complete);
        }

        let item = StreamItem {
            call: self.id,
            payload,
        };
        self.sender.send(Message::Item(item))?;

        Ok(())
    }
}

/// One caller-to-service stream event.
#[derive(Debug)]
pub(super) enum ServerEvent {
    /// One input stream item.
    Input(Payload),
    /// Caller input closure.
    Close,
    /// Caller cancellation.
    Cancel,
}
