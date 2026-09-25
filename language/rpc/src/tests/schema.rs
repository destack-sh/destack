use std::sync::Arc;

use super::arithmetic::{ArithmeticService, NumberRequest, NumberResponse};
use super::server::TestServer;
use crate::{
    Code, Connection, ConnectionError, ConnectionOptions, Idempotency, LoopbackTransport, Method,
    ProtocolVersion, Registry, Response, Server, ServerError, ServiceSchema,
};

/// Keep service compatibility stable across declarations and documentation edits.
#[test]
fn test_canonicalize_service_fingerprint() {
    let service = ArithmeticService::new();
    let mut methods = service.schema.methods().to_vec();
    methods.reverse();
    let mut types = service.schema.types().clone();
    for item in types.items.values_mut() {
        item.docs.push("Updated documentation.".to_string());
    }

    let schema =
        ServiceSchema::new(service.schema.name(), methods, types).expect("rebuild service schema");

    assert_eq!(service.schema.id().0, 5_154_739_212_060_250_957);
    assert_eq!(service.double.0, 666_045_904_407_490_870);
    assert_eq!(
        service.fingerprint(service.double).0,
        98_151_332_897_294_745_605_305_899_252_637_444_896
    );
    assert_eq!(
        service.schema.fingerprint().0,
        63_301_317_089_302_594_779_645_085_898_397_344_182
    );
    assert_eq!(schema.fingerprint(), service.schema.fingerprint());
    assert_eq!(
        schema
            .methods()
            .iter()
            .map(|method| method.name())
            .collect::<Vec<_>>(),
        vec!["blob", "double", "early", "sum"]
    );
}

/// Recompute a method fingerprint as soon as its idempotency changes.
#[test]
fn test_update_method_fingerprint() {
    let service = ArithmeticService::new();
    let method = service
        .schema
        .method(service.double)
        .expect("registered method")
        .clone();
    let fingerprint = method.fingerprint();
    let method = method
        .with_idempotency(Idempotency::Idempotent, service.schema.types())
        .expect("update idempotency");

    assert_eq!(method.idempotency(), Idempotency::Idempotent);
    assert_ne!(method.fingerprint(), fingerprint);
}

/// Keep existing method contracts compatible when a service adds another method.
#[test]
fn test_connect_after_adding_unrelated_method() {
    let client_service = ArithmeticService::new();
    let service_id = client_service.schema.id();
    let method = Method::unary(
        service_id,
        client_service.double,
        client_service.fingerprint(client_service.double),
    );
    let server_service = ArithmeticService::with_extra_method();

    // require the selected method contract while allowing the service declaration to grow
    assert_ne!(
        client_service.schema.fingerprint(),
        server_service.schema.fingerprint()
    );
    assert_eq!(
        client_service.fingerprint(client_service.double),
        server_service.fingerprint(server_service.double)
    );

    let mut server_options = ConnectionOptions::new("test-server");
    server_options.peer.build_id = Some("server-build".to_string());
    let mut client_options = ConnectionOptions::new("test-client");
    client_options.peer.build_id = Some("client-build".to_string());
    let server = TestServer::with_options(server_service, server_options, client_options);
    let connection = server.connection();
    connection
        .bind(&client_service.schema)
        .expect("bind retained client contracts");
    let response: Response<NumberResponse> = connection
        .call(method, NumberRequest { value: 21 })
        .expect("call retained method");

    assert_eq!(response.value, NumberResponse { value: 42 });

    server.close();
}

/// Reject one generated client whose exact method contract is unsupported.
#[test]
fn test_reject_changed_method_contract() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let double = service.double;
    let client_service = ArithmeticService::with_changed_method();
    let server = TestServer::new(service);
    let connection = server.connection();
    let error = connection
        .bind(&client_service.schema)
        .expect_err("reject changed method contract");
    let ConnectionError::MethodNotOffered { service, method } = error else {
        panic!("expected method contract rejection, got {error}");
    };

    assert_eq!(service, service_id);
    assert_eq!(method, double);

    server.close();
}

/// Reject peers without one exact shared wire grammar.
#[test]
fn test_reject_unsupported_protocol_version() {
    let service = ArithmeticService::new();
    let service_id = service.schema.id();
    let (client_transport, server_transport) = LoopbackTransport::pair(8);
    let mut registry = Registry::new();
    registry.insert(service).expect("register service");
    let server = Server::new(registry, ConnectionOptions::new("test-server"));
    let server_thread = std::thread::spawn(move || server.serve(Arc::new(server_transport)));
    let mut client_options = ConnectionOptions::new("test-client");
    client_options.versions = vec![ProtocolVersion::new(2)];

    let error = Connection::connect(Arc::new(client_transport), client_options, vec![service_id])
        .expect_err("reject unsupported protocol");
    let ConnectionError::Rejected(status) = error else {
        panic!("expected handshake rejection, got {error}");
    };

    assert_eq!(status.code, Code::FailedPrecondition);

    let server_error = server_thread
        .join()
        .expect("join server")
        .expect_err("server rejects connection");
    assert!(matches!(
        server_error,
        ServerError::Connection(ConnectionError::Rejected(_))
    ));
}
