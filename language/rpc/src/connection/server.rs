use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Wake, Waker};

use crate::protocol::message::{CallStart, Completion, Message};
use crate::protocol::{
    Code, Handshake, HandshakeCodec, HandshakeRequest, HandshakeResponse, MessageCodec, Status,
};
use crate::service::{CallState, Registry, ServerCall, ServiceError};
use crate::{
    CallId, ConnectionOptions, Limits, MethodId, Peer, ProtocolVersion, Service, ServiceId,
    ServiceOffer, Transport, TransportError,
};
use crossbeam_channel::{Receiver, bounded, select};
use parking_lot::Mutex;

use super::{ConnectionError, HostWaker, MessageReceiver, MessageSender, ServerError};

/// RPC server accepting connections for one service registry.
#[derive(Debug, Clone)]
pub struct Server {
    /// Registered service implementations.
    services: Registry,
    /// Connection negotiation options.
    options: ConnectionOptions,
}

impl Server {
    /// Create one RPC server.
    pub fn new(services: Registry, options: ConnectionOptions) -> Self {
        Self { services, options }
    }

    /// Return this server's peer description.
    pub(super) const fn peer(&self) -> &Peer {
        &self.options.peer
    }

    /// Return this server's advertised limits.
    pub(super) const fn limits(&self) -> Limits {
        self.options.limits
    }

    /// Negotiate one connection request.
    pub(super) fn negotiate(
        &self,
        request: &HandshakeRequest,
    ) -> Result<(ProtocolVersion, Limits, Vec<ServiceOffer>), Status> {
        let (version, limits) = self.options.negotiate(request)?;
        let services = self.services.offer(&request.services)?;

        Ok((version, limits, services))
    }

    /// Open one negotiated server connection.
    pub(super) fn open(
        &self,
        sender: MessageSender,
        services: Vec<ServiceOffer>,
        limits: Limits,
        host_waker: HostWaker,
    ) -> ServerConnection {
        let methods = services
            .into_iter()
            .flat_map(|service| {
                service
                    .methods
                    .into_iter()
                    .map(move |method| (service.service, method.method))
            })
            .collect();

        ServerConnection::new(sender, self.services.clone(), methods, limits, host_waker)
    }

    /// Negotiate and serve one transport until it closes.
    pub fn serve(&self, transport: Arc<dyn Transport>) -> Result<(), ServerError> {
        self.options
            .limits
            .validate()
            .map_err(ConnectionError::Rejected)?;
        let initial_limit =
            usize::try_from(self.options.limits.max_message_bytes).map_err(|_| {
                ConnectionError::Protocol(
                    "local message limit does not fit this platform".to_string(),
                )
            })?;
        let handshake_codec = HandshakeCodec::new(initial_limit);
        let handshake = handshake_codec.receive(transport.as_ref())?;
        let Handshake::Request(request) = handshake else {
            return Err(ConnectionError::Protocol("expected handshake request".to_string()).into());
        };

        // negotiate the grammar, limits, and requested services together
        let accepted = self.negotiate(&request);
        let (version, limits, offers) = match accepted {
            Ok(accepted) => accepted,
            Err(status) => {
                let response = HandshakeResponse::Rejected {
                    peer: self.options.peer.clone(),
                    status: status.clone(),
                };
                handshake_codec.send(transport.as_ref(), &Handshake::Response(response))?;

                return Err(ConnectionError::Rejected(status).into());
            }
        };

        // build the exact negotiated message codecs
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
        let response = HandshakeResponse::Accepted {
            version,
            limits,
            peer: self.options.peer.clone(),
            services: offers.clone(),
        };
        let handshake_codec = HandshakeCodec::new(message_limit);
        handshake_codec.send(transport.as_ref(), &Handshake::Response(response))?;

        // serve messages through the negotiated service methods
        let receiver = MessageReceiver::new(codec.clone(), payload_limit, transport.clone());
        let sender = MessageSender::new(codec, payload_limit, transport);
        let host_waker = HostWaker::default();
        let (wake_sender, wake_receiver) = bounded(1);
        host_waker.set(Arc::new(move || {
            let _notified = wake_sender.try_send(());
        }));
        let connection = self.open(sender, offers, limits, host_waker);

        connection.serve(receiver, wake_receiver)
    }
}

/// State for one accepted server connection.
#[derive(Debug)]
pub(super) struct ServerConnection {
    /// Encoded message sender.
    sender: MessageSender,
    /// Registered service implementations.
    services: Registry,
    /// Active service calls keyed by caller-scoped identifier.
    calls: Arc<Mutex<HashMap<CallId, Arc<CallState>>>>,
    /// Calls retained for cooperative execution.
    tasks: Mutex<HashMap<CallId, CallTask>>,
    /// Methods selected during connection negotiation.
    methods: HashSet<(ServiceId, MethodId)>,
    /// First asynchronous service failure.
    failure: Arc<Mutex<Option<ServerError>>>,
    /// Negotiated resource limits.
    limits: Limits,
    /// Embedded host wake notification.
    host_waker: HostWaker,
}

impl ServerConnection {
    /// Create one negotiated server connection.
    pub(super) fn new(
        sender: MessageSender,
        services: Registry,
        methods: HashSet<(ServiceId, MethodId)>,
        limits: Limits,
        host_waker: HostWaker,
    ) -> Self {
        Self {
            sender,
            services,
            calls: Arc::new(Mutex::new(HashMap::new())),
            tasks: Mutex::new(HashMap::new()),
            methods,
            failure: Arc::new(Mutex::new(None)),
            limits,
            host_waker,
        }
    }

    /// Serve messages and cooperatively ready calls until the transport closes.
    fn serve(
        &self,
        mut receiver: MessageReceiver,
        wake_receiver: Receiver<()>,
    ) -> Result<(), ServerError> {
        let (message_sender, message_receiver) = bounded(1);
        let receiver_thread = std::thread::spawn(move || {
            loop {
                let message = receiver.receive();
                let is_terminal = message.is_err();
                if message_sender.send(message).is_err() || is_terminal {
                    return;
                }
            }
        });

        // drive the same cooperative call state used by embedded sessions
        let result = self.drive(message_receiver, wake_receiver);
        self.host_waker.clear();
        let result = match result {
            Err(failure) => match self.sender.close() {
                Ok(()) => Err(failure),
                Err(close) => Err(ServerError::Shutdown {
                    failure: Box::new(failure),
                    close,
                }),
            },
            Ok(()) => Ok(()),
        };
        self.disconnect();

        receiver_thread.join().map_err(|_| ServerError::Thread)?;

        result
    }

    /// Route inbound messages and poll calls selected by their wakers.
    fn drive(
        &self,
        message_receiver: Receiver<Result<Message, ConnectionError>>,
        wake_receiver: Receiver<()>,
    ) -> Result<(), ServerError> {
        loop {
            select! {
                recv(message_receiver) -> message => {
                    let message = message.map_err(|_| {
                        ConnectionError::Protocol("RPC receiver stopped without a result".to_string())
                    })?;
                    match message {
                        Ok(message) => self.route(message)?,
                        Err(ConnectionError::Transport(TransportError::Closed)) => {
                            return match self.failure.lock().take() {
                                Some(error) => Err(error),
                                None => Ok(()),
                            };
                        }
                        Err(error) => return Err(error.into()),
                    }
                    self.poll()?;
                }
                recv(wake_receiver) -> wake => {
                    wake.map_err(|_| {
                        ConnectionError::Protocol("RPC call waker stopped unexpectedly".to_string())
                    })?;
                    self.host_waker.acknowledge();
                    self.poll()?;
                }
            }
        }
    }

    /// Route one caller message.
    pub(super) fn route(&self, message: Message) -> Result<(), ServerError> {
        match message {
            Message::Start(start) => self.start(start),
            Message::Item(item) => {
                let state = self.call(item.call)?;
                state.push(item.payload).map_err(ServiceError::from)?;

                Ok(())
            }
            Message::Close(close) => {
                let state = self.call(close.call)?;
                state.close_input().map_err(ServiceError::from)?;
                if state.is_complete() {
                    self.calls.lock().remove(&close.call);
                }

                Ok(())
            }
            Message::Window(update) => {
                let Some(state) = self.active(update.call) else {
                    // terminal responses may cross their final window in flight
                    return Ok(());
                };
                state
                    .update_output_window(update.items)
                    .map_err(ServiceError::from)?;

                Ok(())
            }
            Message::Cancel(cancel) => {
                let Some(state) = self.active(cancel.call) else {
                    // cancellation is idempotent after terminal completion
                    return Ok(());
                };
                state.cancel().map_err(ServiceError::from)?;
                if state.is_complete() {
                    self.calls.lock().remove(&cancel.call);
                }

                Ok(())
            }
            Message::Complete(_) | Message::Chunk(_) => Err(ConnectionError::Protocol(
                "caller sent an unexpected message".to_string(),
            )
            .into()),
        }
    }

    /// Start one service call.
    fn start(&self, start: CallStart) -> Result<(), ServerError> {
        if !self.methods.contains(&(start.service, start.method)) {
            return self.reject(
                start.call,
                Status::new(
                    Code::PermissionDenied,
                    "requested method was not negotiated",
                ),
            );
        }
        let Some(service) = self.services.get(start.service) else {
            return self.reject(
                start.call,
                Status::new(Code::Unimplemented, "requested service is not registered"),
            );
        };
        let Some(method) = service.schema().method(start.method) else {
            return self.reject(
                start.call,
                Status::new(Code::Unimplemented, "requested method is not registered"),
            );
        };
        let kind = method.kind();

        let (call, state) = ServerCall::new(
            start.call,
            start.method,
            start.request,
            self.sender.clone(),
            kind,
            self.limits.stream_window,
        );

        // register the call before its handler can receive stream messages
        {
            let mut calls = self.calls.lock();
            if calls.len() >= self.limits.max_concurrent_calls as usize {
                drop(calls);

                return self.reject(
                    start.call,
                    Status::new(
                        Code::ResourceExhausted,
                        "connection reached its concurrent call limit",
                    ),
                );
            }
            if calls.insert(start.call, state.clone()).is_some() {
                return Err(
                    ConnectionError::Protocol("duplicate active call id".to_string()).into(),
                );
            }
        }

        let handler = CallHandler {
            service,
            call,
            state,
            id: start.call,
            calls: self.calls.clone(),
            failure: self.failure.clone(),
            sender: self.sender.clone(),
        };
        let task = CallTask::new(handler.run(), self.host_waker.clone());
        self.tasks.lock().insert(start.call, task);

        Ok(())
    }

    /// Return one required active service call.
    fn call(&self, id: CallId) -> Result<Arc<CallState>, ServerError> {
        self.active(id).ok_or_else(|| {
            ConnectionError::Protocol(format!("message references inactive call {id:?}")).into()
        })
    }

    /// Return one active service call when present.
    fn active(&self, id: CallId) -> Option<Arc<CallState>> {
        self.calls.lock().get(&id).cloned()
    }

    /// Wake and remove every call after connection termination.
    pub(super) fn disconnect(&self) {
        let states = self
            .calls
            .lock()
            .drain()
            .map(|(_, state)| state)
            .collect::<Vec<_>>();
        for state in states {
            state.disconnect();
        }
        self.tasks.lock().clear();
    }

    /// Poll every cooperatively executed call until each stalls or completes.
    pub(super) fn poll(&self) -> Result<(), ServerError> {
        let calls = self.tasks.lock().keys().copied().collect::<Vec<_>>();

        for call in calls {
            let Some(mut task) = self.tasks.lock().remove(&call) else {
                continue;
            };

            let is_pending = !task.is_ready() || task.poll();
            if is_pending {
                let is_ready = task.is_ready();
                self.tasks.lock().insert(call, task);

                // repeat a wake acknowledged by a re-entrant host before reinsertion
                if is_ready {
                    self.host_waker.wake();
                }
            }
        }

        match self.failure.lock().take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Return whether any cooperative call requested another poll.
    pub(super) fn is_ready(&self) -> bool {
        self.tasks.lock().values().any(CallTask::is_ready)
    }

    /// Reject one call before starting a service handler.
    fn reject(&self, call: CallId, status: Status) -> Result<(), ServerError> {
        let completion = Completion::Status { call, status };
        self.sender
            .send(Message::Complete(completion))
            .map_err(ServerError::Connection)
    }
}

/// One cooperatively executed service call.
struct CallTask {
    /// Resumable call operation.
    future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
    /// Whether the call requested another immediate poll.
    is_ready: Arc<AtomicBool>,
    /// Embedded host woken when this call becomes ready.
    host_waker: HostWaker,
}

impl std::fmt::Debug for CallTask {
    /// Format the visible cooperative task state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CallTask")
            .field("is_ready", &self.is_ready)
            .finish_non_exhaustive()
    }
}

impl CallTask {
    /// Create one ready cooperative call.
    fn new(future: impl Future<Output = ()> + Send + 'static, host_waker: HostWaker) -> Self {
        Self {
            future: Box::pin(future),
            is_ready: Arc::new(AtomicBool::new(true)),
            host_waker,
        }
    }

    /// Return whether this call requested a cooperative poll.
    fn is_ready(&self) -> bool {
        self.is_ready.load(Ordering::Acquire)
    }

    /// Poll this call once and return whether it remains pending.
    fn poll(&mut self) -> bool {
        let waker = Waker::from(Arc::new(CallWaker {
            is_ready: self.is_ready.clone(),
            host_waker: self.host_waker.clone(),
        }));
        let mut context = Context::from_waker(&waker);

        self.is_ready.store(false, Ordering::Release);

        self.future.as_mut().poll(&mut context).is_pending()
    }
}

/// Wake state for one cooperative call.
#[derive(Debug)]
struct CallWaker {
    /// Whether the call should be polled again.
    is_ready: Arc<AtomicBool>,
    /// Embedded host woken when this call becomes ready.
    host_waker: HostWaker,
}

impl Wake for CallWaker {
    /// Mark this call ready for another poll.
    fn wake(self: Arc<Self>) {
        self.is_ready.store(true, Ordering::Release);
        self.host_waker.wake();
    }

    /// Mark this call ready without consuming its waker.
    fn wake_by_ref(self: &Arc<Self>) {
        self.is_ready.store(true, Ordering::Release);
        self.host_waker.wake();
    }
}

/// One registered service call ready for execution.
struct CallHandler {
    /// Selected service implementation.
    service: Arc<dyn Service>,
    /// Routed service call.
    call: ServerCall,
    /// Shared call state.
    state: Arc<CallState>,
    /// Caller-scoped call identifier.
    id: CallId,
    /// Active calls for this connection.
    calls: Arc<Mutex<HashMap<CallId, Arc<CallState>>>>,
    /// First asynchronous service failure.
    failure: Arc<Mutex<Option<ServerError>>>,
    /// Connection message sender.
    sender: MessageSender,
}

impl CallHandler {
    /// Run this service call and enforce exactly one terminal completion.
    async fn run(self) {
        let Self {
            service,
            call,
            state,
            id,
            calls,
            failure,
            sender,
        } = self;
        let result = service.call(call).await;

        // terminate the connection when its message path failed
        let result = match result {
            Err(ServiceError::Connection(error)) => {
                Self::fail(
                    &failure,
                    &sender,
                    ServerError::Service(ServiceError::Connection(error)),
                );

                return;
            }
            result => result,
        };

        // require a completed handler to return successfully
        if state.is_complete() {
            if let Err(error) = result {
                Self::fail(&failure, &sender, ServerError::Service(error));

                return;
            }
            Self::retire(&calls, id, &state);

            return;
        }

        // translate a handler return into exactly one terminal response
        let status = Self::status(result);
        let completion = state.complete(Completion::Status { call: id, status });
        if let Err(error) = completion {
            Self::fail(&failure, &sender, ServerError::Service(error));

            return;
        }

        Self::retire(&calls, id, &state);
    }

    /// Retain the first service failure and terminate its connection.
    fn fail(failure: &Mutex<Option<ServerError>>, sender: &MessageSender, error: ServerError) {
        let mut failure = failure.lock();
        if failure.is_none() {
            let error = match sender.close() {
                Ok(()) => error,
                Err(close) => ServerError::Shutdown {
                    failure: Box::new(error),
                    close,
                },
            };
            *failure = Some(error);
        }
    }

    /// Remove one completed call after caller input can no longer be in flight.
    fn retire(calls: &Mutex<HashMap<CallId, Arc<CallState>>>, id: CallId, state: &CallState) {
        if !state.is_input_open() {
            calls.lock().remove(&id);
        }
    }

    /// Translate one incomplete handler result into a terminal status.
    fn status(result: Result<(), ServiceError>) -> Status {
        match result {
            Ok(()) => Status::new(
                Code::Internal,
                "service returned without completing its call",
            ),
            Err(ServiceError::Status(status)) => status,
            Err(ServiceError::Decode(error)) => {
                Status::new(Code::InvalidArgument, error.to_string())
            }
            Err(ServiceError::Encode(error)) => Status::new(Code::Internal, error.to_string()),
            Err(ServiceError::Canceled | ServiceError::Complete) => {
                Status::new(Code::Canceled, "call did not complete")
            }
            Err(ServiceError::Connection(_)) => {
                Status::new(Code::Unavailable, "RPC connection failed")
            }
        }
    }
}
