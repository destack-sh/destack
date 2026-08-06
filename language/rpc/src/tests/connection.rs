use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::arithmetic::{ArithmeticService, BlobRequest, NumberRequest, NumberResponse};
use super::server::TestServer;
use crate::{
    CallError, Code, Connection, ConnectionOptions, LoopbackTransport, Method, Registry, Response,
    Server, Transport, TransportError,
};

/// Execute a unary request through a negotiated loopback connection.
#[test]
fn test_call_unary_method() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let method = Method::unary(
        service_id,
        service.double,
        service.fingerprint(service.double),
    );
    let server = TestServer::new(service);
    let connection = server.connection();
    let response: Response<NumberResponse> = connection
        .call(method, NumberRequest { value: 21 })
        .expect("call double");

    assert_eq!(response.value, NumberResponse { value: 42 });

    server.close();
}

/// Close the transport when the final connection owner is dropped.
#[test]
fn test_drop_connection() {
    let service = ArithmeticService::new();
    let server = TestServer::new(service);

    server.disconnect();
}

/// Exchange input and output streams before receiving the terminal response.
#[test]
fn test_call_bidirectional_streaming_method() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let method: Method<NumberRequest, NumberResponse, u32, u32> =
        Method::bidirectional_streaming(service_id, service.sum, service.fingerprint(service.sum));
    let server = TestServer::new(service);
    let connection = server.connection();
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

    server.close();
}

/// Split and reassemble values larger than the negotiated message limit.
#[test]
fn test_call_deferred_payload() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let method: Method<BlobRequest, u32> =
        Method::unary(service_id, service.blob, service.fingerprint(service.blob));
    let mut server_options = ConnectionOptions::new("test-server");
    server_options.limits.max_message_bytes = 256;
    server_options.limits.max_payload_bytes = 16 * 1024;
    let mut client_options = ConnectionOptions::new("test-client");
    client_options.limits.max_message_bytes = 256;
    client_options.limits.max_payload_bytes = 16 * 1024;
    let server = TestServer::with_options(service, server_options, client_options);
    let connection = server.connection();
    let request = BlobRequest {
        bytes: vec![0x5a; 8 * 1024],
    };
    let request_bytes = request.bytes.len() as u32;
    let response = connection.call(method, request).expect("call blob");

    assert_eq!(response.value, request_bytes);

    server.close();
}

/// Cancel one blocked streaming request and receive its terminal status.
#[test]
fn test_cancel_call() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let method: Method<NumberRequest, NumberResponse, u32, u32> =
        Method::bidirectional_streaming(service_id, service.sum, service.fingerprint(service.sum));
    let server = TestServer::new(service);
    let connection = server.connection();
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

    server.close();
}

/// Fail every active call with the same terminal outbound transport failure.
#[test]
fn test_propagate_send_failure() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let method: Method<NumberRequest, NumberResponse, u32, u32> =
        Method::bidirectional_streaming(service_id, service.sum, service.fingerprint(service.sum));
    let (client_transport, server_transport) = LoopbackTransport::pair(64);
    let client_transport = Arc::new(FailingTransport {
        transport: client_transport,
        is_failed: AtomicBool::new(false),
    });
    let mut registry = Registry::new();
    registry.insert(service).expect("register service");
    let server = Server::new(registry, ConnectionOptions::new("test-server"));
    let server = std::thread::spawn(move || server.serve(Arc::new(server_transport)));
    let connection = Connection::connect(
        client_transport.clone(),
        ConnectionOptions::new("test-client"),
        vec![service_id],
    )
    .expect("connect client");
    let mut first = connection
        .start(method, NumberRequest { value: 1 })
        .expect("start first call");
    let mut second = connection
        .start(method, NumberRequest { value: 2 })
        .expect("start second call");

    // fail one outbound item after both calls are routed
    client_transport.is_failed.store(true, Ordering::Release);
    let first_error = first.send(&3).expect_err("fail first send");
    let second_error = second.response().expect_err("fail second call");
    let CallError::Connection(first_error) = first_error else {
        panic!("expected connection failure, got {first_error}");
    };
    let CallError::Connection(second_error) = second_error else {
        panic!("expected connection failure, got {second_error}");
    };

    assert!(Arc::ptr_eq(&first_error, &second_error));
    assert_eq!(
        first_error.to_string(),
        "transport I/O failed: injected send failure"
    );

    drop(first);
    drop(second);
    drop(connection);
    server
        .join()
        .expect("join server")
        .expect("observe connection closure");
}

/// Transport that can reject every subsequent outbound message.
#[derive(Debug)]
struct FailingTransport {
    /// Working loopback transport.
    transport: LoopbackTransport,
    /// Whether sends should fail.
    is_failed: AtomicBool,
}

impl Transport for FailingTransport {
    /// Send one message unless failure was requested.
    fn send(&self, message: &[u8]) -> Result<(), TransportError> {
        if self.is_failed.load(Ordering::Acquire) {
            Err(std::io::Error::other("injected send failure").into())
        } else {
            self.transport.send(message)
        }
    }

    /// Receive one loopback message.
    fn receive(&self) -> Result<Vec<u8>, TransportError> {
        self.transport.receive()
    }

    /// Close the loopback transport.
    fn close(&self) -> Result<(), TransportError> {
        self.transport.close()
    }
}
