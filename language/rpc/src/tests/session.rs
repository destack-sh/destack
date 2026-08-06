use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::{Condvar, Mutex};

use super::arithmetic::{ArithmeticService, NumberRequest, NumberResponse};
use crate::protocol::message::{
    CallStart, Cancel, Completion, Message, StreamClose, StreamItem, WindowUpdate,
};
use crate::protocol::payload::Payload;
use crate::protocol::{
    Handshake, HandshakeCodec, HandshakeRequest, HandshakeResponse, MessageCodec, ProtocolVersion,
};
use crate::{
    CallError, CallId, Code, Connection, ConnectionOptions, Method, Registry, Request, Response,
    Service, ServiceId, Session, Status, Transport, TransportError,
};

/// One embedded session and its negotiated client connection.
struct TestSession {
    /// Client connection that owns the session transport.
    connection: Arc<Connection>,
}

impl TestSession {
    /// Start one embedded service and connect its client.
    fn new(service: impl Service) -> Self {
        let service_id = service.schema().id();
        let mut services = Registry::new();
        services.insert(service).expect("register service");
        let session = Session::new(services, ConnectionOptions::new("embedded-server"))
            .expect("create session");
        let transport = Arc::new(SessionTransport::new(session));
        let connection = Connection::connect(
            transport,
            ConnectionOptions::new("embedded-client"),
            vec![service_id],
        )
        .expect("connect session");

        Self { connection }
    }

    /// Return the negotiated client connection.
    fn connection(&self) -> &Arc<Connection> {
        &self.connection
    }

    /// Close the embedded session through its client connection.
    fn close(self) {
        self.connection.close().expect("close connection");
    }
}

/// Execute one generated contract through an explicitly driven in-process session.
#[test]
fn test_dispatch_session_call() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let method = Method::unary(
        service_id,
        service.double,
        service.fingerprint(service.double),
    );
    let session = TestSession::new(service);
    let connection = session.connection();
    let response: Response<NumberResponse> = connection
        .call(method, NumberRequest { value: 21 })
        .expect("call double");

    assert_eq!(response.value, NumberResponse { value: 42 });

    session.close();
}

/// Suspend and resume one bidirectional call across separate session dispatches.
#[test]
fn test_dispatch_session_bidirectional_call() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let method: Method<NumberRequest, NumberResponse, u32, u32> =
        Method::bidirectional_streaming(service_id, service.sum, service.fingerprint(service.sum));
    let session = TestSession::new(service);
    let connection = session.connection();

    let mut call = connection
        .start(method, NumberRequest { value: 1 })
        .expect("start sum");
    call.send(&2).expect("send first input");
    call.send(&3).expect("send second input");
    call.close_input().expect("close input");

    assert_eq!(call.receive().expect("receive first output"), Some(3));
    assert_eq!(call.receive().expect("receive second output"), Some(6));
    assert_eq!(call.receive().expect("finish output"), None);
    assert_eq!(
        call.response().expect("receive response").value,
        NumberResponse { value: 6 }
    );

    session.close();
}

/// Resume a cooperative response stream after the caller restores output window.
#[test]
fn test_dispatch_session_stream_backpressure() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let method: Method<NumberRequest, NumberResponse, u32, u32> =
        Method::bidirectional_streaming(service_id, service.sum, service.fingerprint(service.sum));
    let session = TestSession::new(service);
    let connection = session.connection();

    let mut call = connection
        .start(method, NumberRequest { value: 0 })
        .expect("start sum");
    for value in 1..=40 {
        call.send(&value).expect("send input");
    }
    call.close_input().expect("close input");

    let mut outputs = Vec::new();
    while let Some(output) = call.receive().expect("receive output") {
        outputs.push(output);
    }
    let expected = (1..=40)
        .scan(0, |total, value| {
            *total += value;

            Some(*total)
        })
        .collect::<Vec<_>>();

    assert_eq!(outputs, expected);
    assert_eq!(
        call.response().expect("receive response").value,
        NumberResponse { value: 820 }
    );

    session.close();
}

/// Retain and independently resume concurrent cooperative calls.
#[test]
fn test_dispatch_session_concurrent_calls() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let method: Method<NumberRequest, NumberResponse, u32, u32> =
        Method::bidirectional_streaming(service_id, service.sum, service.fingerprint(service.sum));
    let session = TestSession::new(service);
    let connection = session.connection();

    let mut first = connection
        .start(method, NumberRequest { value: 1 })
        .expect("start first sum");
    let mut second = connection
        .start(method, NumberRequest { value: 10 })
        .expect("start second sum");
    first.send(&2).expect("send first input");
    second.send(&3).expect("send second input");
    first.send(&4).expect("send first input");
    second.send(&5).expect("send second input");
    first.close_input().expect("close first input");
    second.close_input().expect("close second input");

    assert_eq!(first.receive().expect("receive first output"), Some(3));
    assert_eq!(first.receive().expect("receive first output"), Some(7));
    assert_eq!(first.receive().expect("finish first output"), None);
    assert_eq!(
        first.response().expect("receive first response").value,
        NumberResponse { value: 7 }
    );
    assert_eq!(second.receive().expect("receive second output"), Some(13));
    assert_eq!(second.receive().expect("receive second output"), Some(18));
    assert_eq!(second.receive().expect("finish second output"), None);
    assert_eq!(
        second.response().expect("receive second response").value,
        NumberResponse { value: 18 }
    );

    session.close();
}

/// Resume one suspended cooperative call with caller cancellation.
#[test]
fn test_cancel_session_call() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let method: Method<NumberRequest, NumberResponse, u32, u32> =
        Method::bidirectional_streaming(service_id, service.sum, service.fingerprint(service.sum));
    let session = TestSession::new(service);
    let connection = session.connection();

    let mut call = connection
        .start(method, NumberRequest { value: 1 })
        .expect("start sum");
    call.cancel().expect("cancel sum");

    let error = call.response().expect_err("receive cancellation");
    let CallError::Status(status) = error else {
        panic!("expected cancellation status, got {error}");
    };
    assert_eq!(status.code, Code::Canceled);
    assert_eq!(status.message, "call did not complete");

    session.close();
}

/// Retain a completed client stream until every earlier caller message arrives.
#[test]
fn test_complete_session_before_input_closes() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let method_id = service.early;
    let mut services = Registry::new();
    services.insert(service).expect("register service");
    let session =
        Session::new(services, ConnectionOptions::new("embedded-server")).expect("create session");
    let codec = connect_session(&session, service_id);
    let call = CallId::new(1);
    let start = Message::Start(CallStart {
        call,
        service: service_id,
        method: method_id,
        request: Request::new(Payload::encode(&()).expect("encode request")),
    });

    // complete while the caller stream remains open
    let messages = session
        .dispatch(&codec.encode(&start).expect("encode start"))
        .expect("dispatch start");
    assert_eq!(messages.len(), 1);
    let message = codec.decode(&messages[0]).expect("decode completion");
    let Message::Complete(completion) = message else {
        panic!("expected call completion, got {message:?}");
    };
    let response = completion.into_result().expect("successful completion");
    let value: NumberResponse = response.value.decode().expect("decode response");
    assert_eq!(value, NumberResponse { value: 42 });

    // discard an already windowed item that crossed the response in flight
    let item = Message::Item(StreamItem {
        call,
        payload: Payload::encode(&7_u32).expect("encode item"),
    });
    let messages = session
        .dispatch(&codec.encode(&item).expect("encode item"))
        .expect("dispatch in-flight item");
    assert!(messages.is_empty());

    // retire the completed request at the ordered input closure
    let close = Message::Close(StreamClose { call });
    let messages = session
        .dispatch(&codec.encode(&close).expect("encode close"))
        .expect("dispatch input closure");
    assert!(messages.is_empty());

    // late idempotent control messages cannot recreate retired call state
    let window = Message::Window(WindowUpdate { call, items: 1 });
    let messages = session
        .dispatch(&codec.encode(&window).expect("encode window"))
        .expect("dispatch late window");
    assert!(messages.is_empty());

    // accept the matching late cancellation with the same idempotent behavior
    let cancel = Message::Cancel(Cancel { call });
    let messages = session
        .dispatch(&codec.encode(&cancel).expect("encode cancellation"))
        .expect("dispatch late cancellation");
    assert!(messages.is_empty());

    session.close().expect("close session");
}

/// Make embedded protocol failures permanently terminal.
#[test]
fn test_close_failed_session() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let mut services = Registry::new();
    services.insert(service).expect("register service");
    let session =
        Session::new(services, ConnectionOptions::new("embedded-server")).expect("create session");
    let codec = connect_session(&session, service_id);
    let invalid = Message::Complete(Completion::Status {
        call: CallId::new(1),
        status: Status::new(Code::Internal, "invalid direction"),
    });
    let bytes = codec.encode(&invalid).expect("encode invalid message");

    let error = session
        .dispatch(&bytes)
        .expect_err("reject invalid message");
    assert_eq!(
        error.to_string(),
        "RPC protocol failed: caller sent an unexpected message"
    );

    let error = session.dispatch(&bytes).expect_err("keep session closed");
    assert_eq!(error.to_string(), "RPC connection is closed");
}

/// Permit a synchronous host wake callback to poll the same session again.
#[test]
fn test_reenter_session_from_wake_handler() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let method_id = service.double;
    let mut services = Registry::new();
    services.insert(service).expect("register service");
    let session = Arc::new(
        Session::new(services, ConnectionOptions::new("embedded-server")).expect("create session"),
    );
    let codec = connect_session(&session, service_id);
    let messages = Arc::new(Mutex::new(Vec::new()));
    let weak_session = Arc::downgrade(&session);
    let woken_messages = messages.clone();
    session.set_wake_handler(Arc::new(move || {
        let Some(session) = weak_session.upgrade() else {
            return;
        };
        let messages = session.poll().expect("poll woken session");
        woken_messages.lock().extend(messages);
    }));
    let start = Message::Start(CallStart {
        call: CallId::new(1),
        service: service_id,
        method: method_id,
        request: Request::new(
            Payload::encode(&NumberRequest { value: 21 }).expect("encode request"),
        ),
    });

    let immediate = session
        .dispatch(&codec.encode(&start).expect("encode start"))
        .expect("dispatch start");
    assert!(immediate.is_empty());

    let messages = messages.lock();
    assert_eq!(messages.len(), 1);
    let message = codec.decode(&messages[0]).expect("decode completion");
    let Message::Complete(completion) = message else {
        panic!("expected call completion, got {message:?}");
    };
    let response = completion.into_result().expect("successful completion");
    let value: NumberResponse = response.value.decode().expect("decode response");
    assert_eq!(value, NumberResponse { value: 42 });
    drop(messages);

    session.close().expect("close session");
}

/// Negotiate one embedded session and return its selected message codec.
fn connect_session(session: &Session, service: ServiceId) -> MessageCodec {
    let options = ConnectionOptions::new("embedded-client");
    let max_message_bytes =
        usize::try_from(options.limits.max_message_bytes).expect("message limit fits platform");
    let handshake_codec = HandshakeCodec::new(max_message_bytes);
    let request = Handshake::Request(HandshakeRequest {
        versions: options.versions,
        limits: options.limits,
        peer: options.peer,
        services: vec![service],
    });
    let bytes = handshake_codec.encode(&request).expect("encode handshake");
    let messages = session.dispatch(&bytes).expect("dispatch handshake");
    assert_eq!(messages.len(), 1);
    let response = handshake_codec
        .decode(&messages[0])
        .expect("decode handshake");
    let Handshake::Response(HandshakeResponse::Accepted {
        version, limits, ..
    }) = response
    else {
        panic!("expected accepted handshake, got {response:?}");
    };
    assert_eq!(version, ProtocolVersion::CURRENT);
    let max_message_bytes =
        usize::try_from(limits.max_message_bytes).expect("message limit fits platform");

    MessageCodec::new(version, max_message_bytes)
}

/// Test transport that drives one session on every outbound message.
#[derive(Debug)]
struct SessionTransport {
    /// Embedded RPC session.
    session: Session,
    /// Messages returned by the session.
    messages: Mutex<VecDeque<Vec<u8>>>,
    /// Message arrival or transport closure.
    changed: Condvar,
    /// Whether this transport has closed.
    is_closed: AtomicBool,
}

impl SessionTransport {
    /// Create one empty transport over a session.
    fn new(session: Session) -> Self {
        Self {
            session,
            messages: Mutex::new(VecDeque::new()),
            changed: Condvar::new(),
            is_closed: AtomicBool::new(false),
        }
    }
}

impl Transport for SessionTransport {
    /// Dispatch one message and queue every immediate response.
    fn send(&self, message: &[u8]) -> Result<(), TransportError> {
        let messages = self.session.dispatch(message).expect("dispatch session");
        self.messages.lock().extend(messages);
        self.changed.notify_all();

        Ok(())
    }

    /// Receive one queued session message.
    fn receive(&self) -> Result<Vec<u8>, TransportError> {
        loop {
            // return the next queued message
            let mut messages = self.messages.lock();
            if let Some(message) = messages.pop_front() {
                return Ok(message);
            }
            if self.is_closed.load(Ordering::Acquire) {
                return Err(TransportError::Closed);
            }
            drop(messages);

            // advance an immediately ready cooperative call
            if self.session.is_ready().expect("inspect session readiness") {
                let messages = self.session.poll().expect("poll session");
                self.messages.lock().extend(messages);

                continue;
            }

            // wait for a caller message or transport closure
            let mut messages = self.messages.lock();
            if messages.is_empty() && !self.is_closed.load(Ordering::Acquire) {
                self.changed.wait(&mut messages);
            }
        }
    }

    /// Close the embedded session.
    fn close(&self) -> Result<(), TransportError> {
        self.session.close().expect("close session");
        self.is_closed.store(true, Ordering::Release);
        self.changed.notify_all();

        Ok(())
    }
}
