#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::with_harness_context;
use crate::platform::net::SocketFamily;

/// Exchange bytes over a tcp connection and verify address metadata.
#[cfg(unix)]
#[test]
fn test_net_roundtrip() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.listen("127.0.0.1", 0, 128)?;
        let port = context.listener_port(listener);

        // connect a client socket
        let client = context.connect("127.0.0.1", port)?;

        // accept on the server side
        let server = context.accept(listener)?;

        // client writes to server
        let sent = context.write(client, b"ping")?;
        assert_eq!(sent, 4);

        // server reads from client
        let mut buffer = vec![0u8; 8];
        let received = context.read(server, &mut buffer)?;
        buffer.truncate(received as usize);
        assert_eq!(buffer, b"ping");

        // server writes to client
        let sent_back = context.write(server, b"pong")?;
        assert_eq!(sent_back, 4);

        // client reads from server
        let mut buffer = vec![0u8; 8];
        let received_back = context.read(client, &mut buffer)?;
        buffer.truncate(received_back as usize);
        assert_eq!(buffer, b"pong");

        // local and peer addresses should match the active connection endpoints
        let (local_host, local_port, local_family) = context.local_address(client)?;
        let (peer_host, peer_port, peer_family) = context.peer_address(client)?;

        assert_eq!(local_family, SocketFamily::IPv4);
        assert_eq!(peer_family, SocketFamily::IPv4);
        assert_eq!(peer_port, port);
        assert_eq!(peer_host, "127.0.0.1");
        assert_eq!(local_host, "127.0.0.1");
        assert!(local_port > 0);

        // close sockets and listener
        context.close(server)?;
        context.close(client)?;
        context.close_listener(listener)?;

        Ok(())
    });
}

/// Exchange bytes over a localhost tcp connection.
#[cfg(unix)]
#[test]
fn test_net_roundtrip_localhost() {
    with_harness_context(|mut context| {
        // start listening on localhost and an ephemeral port
        let listener = context.listen("localhost", 0, 128)?;
        let port = context.listener_port(listener);

        // connect a client socket through hostname resolution
        let client = context.connect("localhost", port)?;

        // accept on the server side
        let server = context.accept(listener)?;

        // client writes to server
        let sent = context.write(client, b"ping")?;
        assert_eq!(sent, 4);

        // server reads from client
        let mut buffer = vec![0u8; 8];
        let received = context.read(server, &mut buffer)?;
        buffer.truncate(received as usize);
        assert_eq!(buffer, b"ping");

        // local and peer addresses should resolve through localhost
        let (local_host, local_port, _) = context.local_address(client)?;
        let (peer_host, peer_port, _) = context.peer_address(client)?;
        assert!(!local_host.is_empty());
        assert!(!peer_host.is_empty());
        assert!(local_port > 0);
        assert_eq!(peer_port, port);

        // close sockets and listener
        context.close(server)?;
        context.close(client)?;
        context.close_listener(listener)?;

        Ok(())
    });
}

/// Exchange bytes over tcp using vectored read and write operations.
#[cfg(unix)]
#[test]
fn test_net_readv_writev_roundtrip() {
    with_harness_context(|mut context| {
        // start listening on an ephemeral port
        let listener = context.listen("127.0.0.1", 0, 128)?;
        let port = context.listener_port(listener);

        // connect and accept
        let client = context.connect("127.0.0.1", port)?;
        let server = context.accept(listener)?;

        // write two iovecs from the client
        let first = b"ping";
        let second = b"pong";
        let sent = context.writev(client, &[first.as_slice(), second.as_slice()])?;
        assert_eq!(sent, 8);

        // read into two iovecs on the server
        let mut read_buffers = vec![vec![0u8; 4], vec![0u8; 4]];
        let received = context.readv(server, &mut read_buffers)?;
        assert_eq!(received, 8);
        assert_eq!(read_buffers[0], b"ping");
        assert_eq!(read_buffers[1], b"pong");

        // close resources
        context.close(server)?;
        context.close(client)?;
        context.close_listener(listener)?;

        Ok(())
    });
}
