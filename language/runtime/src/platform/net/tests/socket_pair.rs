use super::{assert_platform_error_codes, with_harness_context};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{SocketFamily, SocketProtocol, SocketType};

/// Exchange payload bytes over a stream socket pair.
#[cfg(windows)]
#[test]
fn test_net_socket_pair_stream_roundtrip() {
    with_harness_context(|mut context| {
        // create a stream socket pair
        let socket_type = SocketType(windows_sys::Win32::Networking::WinSock::SOCK_STREAM as u32);
        let protocol = SocketProtocol(windows_sys::Win32::Networking::WinSock::IPPROTO_TCP);
        let (first, second) = context.socket_pair(SocketFamily::IPv4, socket_type, protocol)?;

        // write from first and read on second
        let sent = context.write(first, b"pair-stream-first")?;
        assert_eq!(sent, 17);
        let mut read_buffer = vec![0u8; 32];
        let read = context.read(second, &mut read_buffer)?;
        read_buffer.truncate(read as usize);
        assert_eq!(read_buffer, b"pair-stream-first");

        // write from second and read on first
        let sent = context.write(second, b"pair-stream-second")?;
        assert_eq!(sent, 18);
        let mut read_buffer = vec![0u8; 32];
        let read = context.read(first, &mut read_buffer)?;
        read_buffer.truncate(read as usize);
        assert_eq!(read_buffer, b"pair-stream-second");

        // close both handles
        context.close(first)?;
        context.close(second)?;

        Ok(())
    });
}

/// Exchange payload bytes over a datagram socket pair.
#[cfg(windows)]
#[test]
fn test_net_socket_pair_dgram_roundtrip() {
    with_harness_context(|mut context| {
        // create a datagram socket pair
        let socket_type = SocketType(windows_sys::Win32::Networking::WinSock::SOCK_DGRAM as u32);
        let protocol = SocketProtocol(windows_sys::Win32::Networking::WinSock::IPPROTO_UDP);
        let (first, second) = context.socket_pair(SocketFamily::IPv4, socket_type, protocol)?;

        // write from first and read one datagram on second
        let sent = context.write(first, b"pair-dgram-first")?;
        assert_eq!(sent, 16);
        let mut read_buffer = vec![0u8; 32];
        let read = context.read(second, &mut read_buffer)?;
        read_buffer.truncate(read as usize);
        assert_eq!(read_buffer, b"pair-dgram-first");

        // write from second and read one datagram on first
        let sent = context.write(second, b"pair-dgram-second")?;
        assert_eq!(sent, 17);
        let mut read_buffer = vec![0u8; 32];
        let read = context.read(first, &mut read_buffer)?;
        read_buffer.truncate(read as usize);
        assert_eq!(read_buffer, b"pair-dgram-second");

        // close both handles
        context.close(first)?;
        context.close(second)?;

        Ok(())
    });
}

/// Exchange payload bytes over a unix-domain socket pair.
#[cfg(windows)]
#[test]
fn test_net_uds_socket_pair_roundtrip() {
    with_harness_context(|mut context| {
        // create a stream unix-domain socket pair
        let socket_type = SocketType(windows_sys::Win32::Networking::WinSock::SOCK_STREAM as u32);
        let (first, second) = match context.uds_socket_pair(socket_type) {
            Ok(pair) => pair,
            Err(error) => {
                // uds may be unavailable on some windows targets
                assert_platform_error_codes::<()>(
                    Err(error),
                    &[
                        PlatformErrorCode::NotSupported,
                        PlatformErrorCode::Io,
                        PlatformErrorCode::Net,
                    ],
                )?;
                return Ok(());
            }
        };

        // write from first and read on second
        let sent = context.write(first, b"pair-uds-first")?;
        assert_eq!(sent, 14);
        let mut read_buffer = vec![0u8; 32];
        let read = context.read(second, &mut read_buffer)?;
        read_buffer.truncate(read as usize);
        assert_eq!(read_buffer, b"pair-uds-first");

        // write from second and read on first
        let sent = context.write(second, b"pair-uds-second")?;
        assert_eq!(sent, 15);
        let mut read_buffer = vec![0u8; 32];
        let read = context.read(first, &mut read_buffer)?;
        read_buffer.truncate(read as usize);
        assert_eq!(read_buffer, b"pair-uds-second");

        // close both handles
        context.close(first)?;
        context.close(second)?;

        Ok(())
    });
}
