use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use crossbeam_channel::{Sender, unbounded};
use parking_lot::Mutex;
use tspp_serde::Codec;

use super::{ConnectionError, MessageReceiver, MessageSender};
use crate::call::{CallEvent, SendWindow};
use crate::protocol::message::{CallStart, Cancel, Message, StreamClose, StreamItem, WindowUpdate};
use crate::protocol::payload::Payload;
use crate::protocol::{
    Handshake, HandshakeCodec, HandshakeRequest, HandshakeResponse, MessageCodec, Peer,
    ProtocolVersion, ServiceOffer,
};
use crate::{
    Call, CallError, CallId, Code, ConnectionOptions, IntoRequest, Limits, Method, Response,
    ServiceId, ServiceSchema, Status, Transport,
};

/// One negotiated RPC connection to a service peer.
#[derive(Debug)]
pub struct Connection {
    /// Encoded message sender.
    sender: MessageSender,
    /// Active calls keyed by caller-scoped identifier.
    calls: Mutex<HashMap<CallId, Arc<CallRoute>>>,
    /// Negotiated grammar version.
    version: ProtocolVersion,
    /// Negotiated resource limits.
    limits: Limits,
    /// Remote peer description.
    peer: Peer,
    /// Services offered by the remote peer.
    services: Vec<ServiceOffer>,
    /// Next caller-scoped identifier.
    next_call: AtomicU64,
    /// Whether this connection has terminated.
    is_closed: AtomicBool,
    /// First terminal connection failure.
    failure: Mutex<Option<Arc<ConnectionError>>>,
}

impl Connection {
    /// Negotiate and start one RPC connection.
    pub fn connect(
        transport: Arc<dyn Transport>,
        options: ConnectionOptions,
        services: Vec<ServiceId>,
    ) -> Result<Arc<Self>, ConnectionError> {
        let connection = Self::establish(transport.clone(), options, services);
        match connection {
            Ok(connection) => Ok(connection),
            Err(failure) => match transport.close() {
                Ok(()) => Err(failure),
                Err(close) => Err(ConnectionError::Shutdown {
                    failure: Box::new(failure),
                    close: Box::new(close.into()),
                }),
            },
        }
    }

    /// Establish one negotiated connection over an open transport.
    fn establish(
        transport: Arc<dyn Transport>,
        options: ConnectionOptions,
        services: Vec<ServiceId>,
    ) -> Result<Arc<Self>, ConnectionError> {
        options
            .limits
            .validate()
            .map_err(|status| ConnectionError::Protocol(status.to_string()))?;
        let initial_limit = usize::try_from(options.limits.max_message_bytes).map_err(|_| {
            ConnectionError::Protocol("local message limit does not fit this platform".to_string())
        })?;
        let handshake_codec = HandshakeCodec::new(initial_limit);
        let versions = options.versions.clone();
        let requested_limits = options.limits;
        let handshake = HandshakeRequest {
            versions: options.versions,
            limits: options.limits,
            peer: options.peer,
            services: services.clone(),
        };
        handshake_codec.send(transport.as_ref(), &Handshake::Request(handshake))?;

        // require one exact handshake response before starting the receiver
        let handshake = handshake_codec.receive(transport.as_ref())?;
        let Handshake::Response(response) = handshake else {
            return Err(ConnectionError::Protocol(
                "expected handshake response".to_string(),
            ));
        };
        let HandshakeResponse::Accepted {
            version,
            limits,
            peer,
            services: offers,
        } = response
        else {
            let HandshakeResponse::Rejected { status, .. } = response else {
                return Err(ConnectionError::Protocol(
                    "invalid handshake response".to_string(),
                ));
            };

            return Err(ConnectionError::Rejected(status));
        };
        if !versions.contains(&version) {
            return Err(ConnectionError::Protocol(format!(
                "peer selected unsupported RPC protocol version {version:?}"
            )));
        }
        requested_limits
            .validate_selected(limits)
            .map_err(|status| ConnectionError::Protocol(status.to_string()))?;
        Self::validate_offers(&offers, &services)?;

        let message_limit = usize::try_from(limits.max_message_bytes).map_err(|_| {
            ConnectionError::Protocol(
                "negotiated message limit does not fit this platform".to_string(),
            )
        })?;
        let payload_limit = usize::try_from(limits.max_payload_bytes).map_err(|_| {
            ConnectionError::Protocol(
                "negotiated payload limit does not fit this platform".to_string(),
            )
        })?;
        let codec = MessageCodec::new(version, message_limit);
        let sender = MessageSender::new(codec.clone(), payload_limit, transport.clone());
        let receiver = MessageReceiver::new(codec, payload_limit, transport);
        let connection = Arc::new(Self {
            sender,
            calls: Mutex::new(HashMap::new()),
            version,
            limits,
            peer,
            services: offers,
            next_call: AtomicU64::new(1),
            is_closed: AtomicBool::new(false),
            failure: Mutex::new(None),
        });
        connection.start_receiver(receiver);

        Ok(connection)
    }

    /// Return the negotiated grammar version.
    pub const fn version(&self) -> ProtocolVersion {
        self.version
    }

    /// Return the negotiated resource limits.
    pub const fn limits(&self) -> Limits {
        self.limits
    }

    /// Return the remote peer description.
    pub const fn peer(&self) -> &Peer {
        &self.peer
    }

    /// Return the services offered by the remote peer.
    pub fn services(&self) -> &[ServiceOffer] {
        &self.services
    }

    /// Bind one generated client schema to this connection's offered contracts.
    pub fn bind(&self, schema: &ServiceSchema) -> Result<(), ConnectionError> {
        let Some(service) = self
            .services
            .iter()
            .find(|offer| offer.service == schema.id())
        else {
            return Err(ConnectionError::ServiceNotOffered(schema.id()));
        };

        for method in schema.methods() {
            let fingerprint = method.fingerprint();
            let is_offered = service
                .methods
                .iter()
                .any(|offer| offer.method == method.id() && offer.fingerprint == fingerprint);
            if !is_offered {
                return Err(ConnectionError::MethodNotOffered {
                    service: schema.id(),
                    method: method.id(),
                });
            }
        }

        Ok(())
    }

    /// Start one statically typed RPC call.
    pub fn start<RequestValue, Reply, Input, Output>(
        self: &Arc<Self>,
        method: Method<RequestValue, Reply, Input, Output>,
        request: impl IntoRequest<RequestValue>,
    ) -> Result<Call<Reply, Input, Output>, CallError>
    where
        RequestValue: Codec,
    {
        let service = self
            .services
            .iter()
            .find(|offer| offer.service == method.service())
            .ok_or_else(|| {
                CallError::Connection(Arc::new(ConnectionError::ServiceNotOffered(
                    method.service(),
                )))
            })?;
        let is_offered = service
            .methods
            .iter()
            .any(|offer| offer.method == method.id() && offer.fingerprint == method.fingerprint());
        if !is_offered {
            return Err(CallError::Connection(Arc::new(
                ConnectionError::MethodNotOffered {
                    service: method.service(),
                    method: method.id(),
                },
            )));
        }

        if self.is_closed.load(Ordering::Acquire) {
            return Err(CallError::Connection(Arc::new(ConnectionError::Closed)));
        }

        let id = CallId::new(self.next_call.fetch_add(1, Ordering::Relaxed));
        let request = request.into_request();
        let payload = Payload::encode(&request.value).map_err(CallError::Encode)?;
        self.validate_payload(&payload)?;
        let (event_sender, receiver) = unbounded();
        let route = Arc::new(CallRoute {
            events: event_sender,
            input_window: Arc::new(SendWindow::new(self.limits.stream_window)),
            output_window: AtomicU32::new(self.limits.stream_window),
            input_sending: Mutex::new(()),
            is_input_open: AtomicBool::new(method.kind().has_input()),
            is_abandoned: AtomicBool::new(false),
            is_cancel_sent: AtomicBool::new(false),
        });

        // publish the route before the peer can answer the request
        {
            let mut calls = self.calls.lock();
            let limit = self.limits.max_concurrent_calls as usize;
            if self.is_closed.load(Ordering::Acquire) {
                return Err(CallError::Connection(Arc::new(ConnectionError::Closed)));
            }
            if calls.len() >= limit {
                return Err(CallError::Status(Status::new(
                    Code::ResourceExhausted,
                    "connection reached its concurrent call limit",
                )));
            }
            calls.insert(id, route);
        }

        let request = request.map(|_| payload);
        let start = CallStart {
            call: id,
            service: method.service(),
            method: method.id(),
            request,
        };
        self.send(Message::Start(start))?;

        Ok(Call::new(
            self.clone(),
            id,
            receiver,
            method.kind().has_input(),
        ))
    }

    /// Execute one unary RPC call.
    pub fn call<RequestValue, Reply>(
        self: &Arc<Self>,
        method: Method<RequestValue, Reply>,
        request: impl IntoRequest<RequestValue>,
    ) -> Result<Response<Reply>, CallError>
    where
        RequestValue: Codec,
        Reply: Codec,
    {
        let mut call: Call<Reply, Infallible, Infallible> = self.start(method, request)?;

        call.response()
    }

    /// Close this connection.
    pub fn close(&self) -> Result<(), ConnectionError> {
        self.fail(Arc::new(ConnectionError::Closed));
        self.sender.close()
    }
}

impl Drop for Connection {
    /// Interrupt the receiver when the final connection owner is abandoned.
    fn drop(&mut self) {
        let _closed = self.sender.close();
    }
}

impl Connection {
    /// Send one message for an active call.
    pub(crate) fn send(&self, message: Message) -> Result<(), CallError> {
        if let Err(error) = self.sender.send(message) {
            let error = self.terminate(error);

            Err(CallError::Connection(error))
        } else {
            Ok(())
        }
    }

    /// Return caller-to-service stream window for one call.
    pub(crate) fn input_window(&self, id: CallId) -> Result<Arc<SendWindow>, CallError> {
        let calls = self.calls.lock();
        let route = calls.get(&id).ok_or(CallError::Complete)?;

        Ok(route.input_window.clone())
    }

    /// Send one caller-to-service input item while its stream remains open.
    pub(crate) fn send_input(&self, id: CallId, payload: Payload) -> Result<(), CallError> {
        let route = self.active(id)?;
        let _sending = route.input_sending.lock();
        if !route.is_input_open.load(Ordering::Acquire) {
            return Err(CallError::Complete);
        }

        let item = StreamItem { call: id, payload };
        self.send(Message::Item(item))
    }

    /// Release one consumed service-to-caller stream item.
    pub(crate) fn release_output(&self, id: CallId) -> Result<(), CallError> {
        let calls = self.calls.lock();
        let Some(route) = calls.get(&id) else {
            return Ok(());
        };
        route
            .output_window
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |window| {
                window.checked_add(1)
            })
            .map_err(|_| {
                CallError::Connection(Arc::new(ConnectionError::Protocol(
                    "output stream window overflowed".to_string(),
                )))
            })?;
        drop(calls);

        let update = WindowUpdate { call: id, items: 1 };
        self.send(Message::Window(update))
    }

    /// Abandon one active call and cancel unfinished service work.
    pub(crate) fn abandon(&self, id: CallId) {
        let route = self.calls.lock().get(&id).cloned();
        let Some(route) = route else {
            return;
        };

        let _sending = route.input_sending.lock();
        route.is_abandoned.store(true, Ordering::Release);
        route.is_input_open.store(false, Ordering::Release);
        route.input_window.close();
        if route.is_cancel_sent.swap(true, Ordering::AcqRel) {
            return;
        }

        let cancel = Cancel { call: id };
        if let Err(error) = self.sender.send(Message::Cancel(cancel)) {
            self.terminate(error);
        }
    }

    /// Close one caller-to-service input stream.
    pub(crate) fn close_input(&self, id: CallId, notify: bool) -> Result<(), CallError> {
        let route = self.active(id)?;
        self.close_route_input(id, &route, notify)
            .map_err(Arc::new)
            .map_err(CallError::Connection)
    }

    /// Close one caller input route and optionally notify its service.
    fn close_route_input(
        &self,
        id: CallId,
        route: &CallRoute,
        notify: bool,
    ) -> Result<(), ConnectionError> {
        let _sending = route.input_sending.lock();
        let was_open = route.is_input_open.swap(false, Ordering::AcqRel);
        route.input_window.close();
        if notify && was_open {
            let close = StreamClose { call: id };

            self.sender.send(Message::Close(close))?;
        }

        Ok(())
    }

    /// Cancel one active call after stopping its input stream.
    pub(crate) fn cancel(&self, id: CallId) -> Result<(), CallError> {
        let route = self.active(id)?;
        let _sending = route.input_sending.lock();
        route.is_input_open.store(false, Ordering::Release);
        route.input_window.close();
        if route.is_cancel_sent.swap(true, Ordering::AcqRel) {
            return Ok(());
        }

        let cancel = Cancel { call: id };
        self.send(Message::Cancel(cancel))
    }

    /// Terminate this connection with one shared failure.
    fn fail(&self, error: Arc<ConnectionError>) -> Arc<ConnectionError> {
        let mut failure = self.failure.lock();
        if let Some(failure) = failure.as_ref() {
            return failure.clone();
        }
        self.is_closed.store(true, Ordering::Release);
        *failure = Some(error.clone());
        drop(failure);

        self.wake_calls(error.clone());

        error
    }

    /// Close transport and terminate this connection after a message path failure.
    fn terminate(&self, error: ConnectionError) -> Arc<ConnectionError> {
        let mut failure = self.failure.lock();
        if let Some(failure) = failure.as_ref() {
            return failure.clone();
        }
        self.is_closed.store(true, Ordering::Release);
        let error = match self.sender.close() {
            Ok(()) => error,
            Err(close) => ConnectionError::Shutdown {
                failure: Box::new(error),
                close: Box::new(close),
            },
        };
        let error = Arc::new(error);
        *failure = Some(error.clone());
        drop(failure);

        self.wake_calls(error.clone());

        error
    }

    /// Wake every routed call with one terminal connection failure.
    fn wake_calls(&self, error: Arc<ConnectionError>) {
        let calls = self
            .calls
            .lock()
            .drain()
            .map(|(_, route)| route)
            .collect::<Vec<_>>();
        for route in calls {
            route.input_window.close();
            let _result = route.events.send(CallEvent::Connection(error.clone()));
        }
    }

    /// Route one decoded peer message.
    fn route(&self, message: Message) -> Result<(), ConnectionError> {
        match message {
            Message::Item(item) => {
                let route = self.require_route(item.call)?;
                route
                    .output_window
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |window| {
                        window.checked_sub(1)
                    })
                    .map_err(|_| {
                        ConnectionError::Protocol("peer exceeded output stream window".to_string())
                    })?;
                if !route.is_abandoned.load(Ordering::Acquire)
                    && route.events.send(CallEvent::Output(item.payload)).is_err()
                {
                    self.abandon(item.call);
                }

                Ok(())
            }
            Message::Window(update) => {
                let route = self.require_route(update.call)?;
                if route.is_abandoned.load(Ordering::Acquire) {
                    return Ok(());
                }
                if update.items == 0 {
                    return Err(ConnectionError::Protocol(
                        "stream window update must be positive".to_string(),
                    ));
                }
                if route.input_window.try_update(update.items) {
                    Ok(())
                } else {
                    Err(ConnectionError::Protocol(
                        "input stream window overflowed".to_string(),
                    ))
                }
            }
            Message::Complete(completion) => {
                let id = completion.call();
                let route = self.require_route(id)?;
                let is_abandoned = route.is_abandoned.load(Ordering::Acquire);
                if !is_abandoned {
                    self.close_route_input(id, &route, true)?;
                }
                self.calls.lock().remove(&id);
                if !is_abandoned {
                    let _consumer = route.events.send(CallEvent::Complete(completion));
                }

                Ok(())
            }
            Message::Cancel(cancel) => {
                let route = self.require_route(cancel.call)?;
                self.calls.lock().remove(&cancel.call);
                route.is_input_open.store(false, Ordering::Release);
                route.input_window.close();
                if !route.is_abandoned.load(Ordering::Acquire) {
                    let _consumer = route.events.send(CallEvent::Canceled);
                }

                Ok(())
            }
            Message::Close(_) => Err(ConnectionError::Protocol(
                "service peer closed a caller input stream".to_string(),
            )),
            Message::Start(_) | Message::Chunk(_) => Err(ConnectionError::Protocol(
                "service peer sent an unexpected message".to_string(),
            )),
        }
    }

    /// Return one required routed call.
    fn require_route(&self, id: CallId) -> Result<Arc<CallRoute>, ConnectionError> {
        self.calls.lock().get(&id).cloned().ok_or_else(|| {
            ConnectionError::Protocol(format!("message references inactive call {id:?}"))
        })
    }

    /// Return one active call route.
    fn active(&self, id: CallId) -> Result<Arc<CallRoute>, CallError> {
        let route = self.calls.lock().get(&id).cloned();
        match route {
            Some(route) if !route.is_abandoned.load(Ordering::Acquire) => Ok(route),
            Some(_) | None => Err(CallError::Complete),
        }
    }

    /// Enforce the negotiated assembled payload limit before opening a call.
    pub(crate) fn validate_payload(&self, payload: &Payload) -> Result<(), CallError> {
        let actual = payload.inline().map_err(|_| {
            CallError::Connection(Arc::new(ConnectionError::Protocol(
                "outbound payload is unexpectedly deferred".to_string(),
            )))
        })?;
        let limit = self.sender.max_payload_bytes();
        if actual.len() > limit {
            Err(CallError::Status(Status::new(
                Code::ResourceExhausted,
                format!("RPC payload exceeds {limit} bytes: {}", actual.len()),
            )))
        } else {
            Ok(())
        }
    }
}

/// Per-call connection routing state.
#[derive(Debug)]
struct CallRoute {
    /// Routed call event sender.
    events: Sender<CallEvent>,
    /// Available caller-to-service send window.
    input_window: Arc<SendWindow>,
    /// Remaining peer output window.
    output_window: AtomicU32,
    /// Serializes caller input items with closure and cancellation.
    input_sending: Mutex<()>,
    /// Whether caller input may still be sent.
    is_input_open: AtomicBool,
    /// Whether the local call consumer was dropped before completion.
    is_abandoned: AtomicBool,
    /// Whether cancellation was sent to the peer.
    is_cancel_sent: AtomicBool,
}

impl Connection {
    /// Start the blocking receiver owned by this connection.
    fn start_receiver(self: &Arc<Self>, mut receiver: MessageReceiver) {
        let connection = Arc::downgrade(self);
        std::thread::spawn(move || {
            loop {
                let message = receiver.receive();
                let Some(connection) = connection.upgrade() else {
                    return;
                };
                let result = message.and_then(|message| connection.route(message));
                if let Err(error) = result {
                    connection.terminate(error);

                    return;
                }
            }
        });
    }

    /// Validate the exact service offers returned by one accepting peer.
    fn validate_offers(
        offers: &[ServiceOffer],
        requested: &[ServiceId],
    ) -> Result<(), ConnectionError> {
        let requested = requested.iter().copied().collect::<HashSet<_>>();
        if requested.len() != offers.len() {
            return Err(ConnectionError::Protocol(
                "peer returned an unexpected RPC service offer count".to_string(),
            ));
        }

        let mut offered = HashSet::with_capacity(offers.len());
        for service in offers {
            if !requested.contains(&service.service) || !offered.insert(service.service) {
                return Err(ConnectionError::Protocol(
                    "peer returned an unexpected or duplicate RPC service offer".to_string(),
                ));
            }

            let mut methods = HashSet::with_capacity(service.methods.len());
            if service
                .methods
                .iter()
                .any(|method| !methods.insert(method.method))
            {
                return Err(ConnectionError::Protocol(
                    "peer returned a duplicate RPC method offer".to_string(),
                ));
            }
        }

        Ok(())
    }
}
