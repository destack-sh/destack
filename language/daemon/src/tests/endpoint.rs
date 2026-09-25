use super::harness::TestDaemon;

/// Publish exact RPC and browser discovery metadata.
#[test]
fn test_publish_endpoint_metadata() {
    let daemon = TestDaemon::start("daemon_endpoint_metadata");
    let connection = daemon.connect();
    let metadata = daemon
        .endpoint
        .read_metadata()
        .expect("metadata should read")
        .expect("metadata should exist");

    assert_eq!(metadata.home, daemon.endpoint.home);
    assert_eq!(metadata.socket_path, daemon.endpoint.socket_path);
    assert_eq!(metadata.rpc_version, tspp_rpc::ProtocolVersion::CURRENT.0);
    assert!(metadata.websocket_url.starts_with("ws://127.0.0.1:"));

    daemon.shutdown(connection);
}
