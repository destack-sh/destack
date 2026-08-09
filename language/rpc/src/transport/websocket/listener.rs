use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::WebSocketError;
use super::transport::WebSocketTransport;
use crate::{Listener, Transport};

/// Listener for authenticated WebSocket RPC connections.
#[derive(Debug)]
pub struct WebSocketListener {
    /// TCP listener.
    listener: TcpListener,
    /// Bound TCP address.
    address: SocketAddr,
    /// Largest accepted transport message.
    max_message_bytes: usize,
    /// Required HTTP request path.
    path: String,
    /// Required request bearer token.
    token: String,
    /// Whether closure was requested.
    is_closed: AtomicBool,
}

impl WebSocketListener {
    /// Bind one authenticated WebSocket listener.
    pub fn bind(
        address: SocketAddr,
        max_message_bytes: usize,
        path: String,
        token: String,
    ) -> Result<Self, WebSocketError> {
        let listener = TcpListener::bind(address)?;
        let address = listener.local_addr()?;

        Ok(Self {
            listener,
            address,
            max_message_bytes,
            path,
            token,
            is_closed: AtomicBool::new(false),
        })
    }

    /// Return the bound socket address.
    pub fn local_address(&self) -> SocketAddr {
        self.address
    }
}

impl Listener for WebSocketListener {
    type Error = WebSocketError;

    /// Accept one WebSocket connection, or return `None` after closure.
    fn accept(&self) -> Result<Option<Arc<dyn Transport>>, Self::Error> {
        if self.is_closed.load(Ordering::Acquire) {
            return Ok(None);
        }

        let (stream, _) = self.listener.accept()?;
        if self.is_closed.load(Ordering::Acquire) {
            drop(stream);

            return Ok(None);
        }
        let transport: Arc<dyn Transport> = WebSocketTransport::new(
            stream,
            self.max_message_bytes,
            self.path.clone(),
            self.token.clone(),
        )?;

        Ok(Some(transport))
    }

    /// Close this listener and interrupt a pending accept.
    fn close(&self) -> Result<(), Self::Error> {
        if self.is_closed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }

        let stream = TcpStream::connect(self.address)?;
        drop(stream);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    use super::WebSocketListener;
    use crate::Listener;

    /// Upgrade and exchange one complete binary transport message.
    #[test]
    fn test_accept_transport() {
        let listener = WebSocketListener::bind(
            "127.0.0.1:0".parse().expect("parse address"),
            1024,
            "/rpc".to_string(),
            "secret".to_string(),
        )
        .expect("bind listener");
        let mut client = TcpStream::connect(listener.local_address()).expect("connect client");
        let transport = listener
            .accept()
            .expect("accept transport")
            .expect("listener should remain open");
        let receiver = transport.clone();
        let receive = std::thread::spawn(move || receiver.receive());

        // complete the deferred HTTP upgrade in the connection receiver
        let request = "GET /rpc?token=secret HTTP/1.1\r\n\
                       Host: 127.0.0.1\r\n\
                       Upgrade: websocket\r\n\
                       Connection: Upgrade\r\n\
                       Sec-WebSocket-Version: 13\r\n\
                       Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
                       \r\n";
        client.write_all(request.as_bytes()).expect("write upgrade");
        let response = read_header(&mut client);
        let expected = "HTTP/1.1 101 Switching Protocols\r\n\
                        Upgrade: websocket\r\n\
                        Connection: Upgrade\r\n\
                        Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\
                        \r\n";
        assert_eq!(response, expected.as_bytes());

        // deliver one masked client message through the accepted transport
        write_masked(&mut client, &[1, 2, 3]);
        assert_eq!(
            receive
                .join()
                .expect("join receiver")
                .expect("receive message"),
            [1, 2, 3]
        );

        transport.close().expect("close transport");
        let mut close = [0; 2];
        client.read_exact(&mut close).expect("read close frame");
        assert_eq!(close, [0x88, 0]);
    }

    /// Read one complete HTTP response header.
    fn read_header(stream: &mut TcpStream) -> Vec<u8> {
        let mut response = Vec::new();

        while !response.ends_with(b"\r\n\r\n") {
            let mut byte = [0; 1];
            stream.read_exact(&mut byte).expect("read response byte");
            response.push(byte[0]);
        }

        response
    }

    /// Write one masked final binary client message.
    fn write_masked(stream: &mut TcpStream, payload: &[u8]) {
        let mask = [0x11, 0x22, 0x33, 0x44];
        let mut frame = vec![0x82, 0x80 | payload.len() as u8];
        frame.extend_from_slice(&mask);
        frame.extend(
            payload
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % mask.len()]),
        );

        stream.write_all(&frame).expect("write message");
    }
}
