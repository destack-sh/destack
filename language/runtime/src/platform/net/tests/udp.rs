#![cfg_attr(windows, allow(dead_code, unused_imports))]
use super::with_harness_context;
use crate::platform::net::SocketFamily;

#[cfg(unix)]
#[test]
fn test_net_udp_roundtrip() {
    with_harness_context(|mut context| {
        // setup server socket
        let server = context.udp_socket(SocketFamily::IPv4)?;
        context.udp_bind(server, "127.0.0.1", 0)?;

        // resolve server port
        let (_host, port, _family) = context.local_address(server)?;

        // setup client socket
        let client = context.udp_socket(SocketFamily::IPv4)?;

        // send a datagram
        let sent = context.udp_send_to(client, "127.0.0.1", port, b"ping")?;
        assert_eq!(sent, 4);

        // receive the datagram
        let mut buffer = vec![0u8; 32];
        let (host, recv_port, family, bytes) = context.udp_recv_from(server, &mut buffer)?;
        buffer.truncate(bytes as usize);

        assert_eq!(family, SocketFamily::IPv4);
        assert_eq!(host, "127.0.0.1");
        assert!(recv_port > 0);
        assert_eq!(buffer, b"ping");

        // close sockets
        context.close(client)?;
        context.close(server)?;

        Ok(())
    });
}

#[cfg(unix)]
#[test]
fn test_net_udp_connect_roundtrip() {
    with_harness_context(|mut context| {
        // set up server socket
        let server = context.udp_socket(SocketFamily::IPv4)?;
        context.udp_bind(server, "127.0.0.1", 0)?;
        let (_host, port, _family) = context.local_address(server)?;

        // set up client socket and connect it
        let client = context.udp_socket(SocketFamily::IPv4)?;
        context.udp_connect(client, "127.0.0.1", port)?;

        // write through connected udp socket
        let sent = context.write(client, b"ping")?;
        assert_eq!(sent, 4);

        // read datagram on server
        let mut buffer = vec![0u8; 32];
        let (_host, _port, family, bytes) = context.udp_recv_from(server, &mut buffer)?;
        buffer.truncate(bytes as usize);
        assert_eq!(family, SocketFamily::IPv4);
        assert_eq!(buffer, b"ping");

        // close resources
        context.close(client)?;
        context.close(server)?;

        Ok(())
    });
}
