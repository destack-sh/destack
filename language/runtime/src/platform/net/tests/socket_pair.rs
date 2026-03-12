#[cfg(windows)]
use super::assert_platform_error_codes_with_privileged_policy;
use super::with_harness_context;
#[cfg(windows)]
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::net::{SocketFamily, SocketProtocol, SocketType};

#[cfg(unix)]
const STREAM_SOCKET_TYPE: u32 = libc::SOCK_STREAM as u32;
#[cfg(windows)]
const STREAM_SOCKET_TYPE: u32 = windows_sys::Win32::Networking::WinSock::SOCK_STREAM as u32;

#[cfg(unix)]
const DATAGRAM_SOCKET_TYPE: u32 = libc::SOCK_DGRAM as u32;
#[cfg(windows)]
const DATAGRAM_SOCKET_TYPE: u32 = windows_sys::Win32::Networking::WinSock::SOCK_DGRAM as u32;

#[cfg(unix)]
const TCP_PROTOCOL: i32 = libc::IPPROTO_TCP;
#[cfg(windows)]
const TCP_PROTOCOL: i32 = windows_sys::Win32::Networking::WinSock::IPPROTO_TCP;

#[cfg(unix)]
const UDP_PROTOCOL: i32 = libc::IPPROTO_UDP;
#[cfg(windows)]
const UDP_PROTOCOL: i32 = windows_sys::Win32::Networking::WinSock::IPPROTO_UDP;

/// Exchange payload bytes over a stream socket pair.
#[cfg(any(unix, windows))]
#[test]
fn test_net_socket_pair_stream_roundtrip() {
    with_harness_context(|mut context| {
        // create a stream socket pair
        let socket_type = SocketType(STREAM_SOCKET_TYPE);
        let protocol = SocketProtocol(TCP_PROTOCOL);
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
        context.destack_net_close(first)?;
        context.destack_net_close(second)?;

        Ok(())
    });
}

/// Exchange payload bytes over a datagram socket pair.
#[cfg(any(unix, windows))]
#[test]
fn test_net_socket_pair_dgram_roundtrip() {
    with_harness_context(|mut context| {
        // create a datagram socket pair
        let socket_type = SocketType(DATAGRAM_SOCKET_TYPE);
        let protocol = SocketProtocol(UDP_PROTOCOL);
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
        context.destack_net_close(first)?;
        context.destack_net_close(second)?;

        Ok(())
    });
}

/// Exchange payload bytes over a unix-domain socket pair.
#[cfg(any(unix, windows))]
#[test]
fn test_net_uds_socket_pair_roundtrip() {
    with_harness_context(|mut context| {
        // create a stream unix-domain socket pair
        let socket_type = SocketType(STREAM_SOCKET_TYPE);
        let (first, second) = match context.uds_socket_pair(socket_type) {
            Ok(pair) => pair,
            Err(error) => {
                #[cfg(windows)]
                {
                    // af_unix support may be absent at runtime on windows
                    assert_platform_error_codes_with_privileged_policy::<()>(
                        Err(error),
                        &[PlatformErrorCode::NotSupported],
                    )?;
                    return Ok(());
                }

                #[cfg(unix)]
                {
                    return Err(error);
                }
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
        context.destack_net_close(first)?;
        context.destack_net_close(second)?;

        Ok(())
    });
}

/// Exchange payload bytes over a unix-domain datagram socket pair.
#[cfg(any(unix, windows))]
#[test]
fn test_net_uds_socket_pair_dgram_roundtrip() {
    with_harness_context(|mut context| {
        // create a datagram unix-domain socket pair
        let socket_type = SocketType(DATAGRAM_SOCKET_TYPE);
        let (first, second) = match context.uds_socket_pair(socket_type) {
            Ok(pair) => pair,
            Err(error) => {
                #[cfg(windows)]
                {
                    assert_platform_error_codes_with_privileged_policy::<()>(
                        Err(error),
                        &[PlatformErrorCode::NotSupported],
                    )?;
                    return Ok(());
                }

                #[cfg(unix)]
                {
                    return Err(error);
                }
            }
        };

        // write one datagram from first to second
        let sent = context.write(first, b"pair-uds-dgram-first")?;
        assert_eq!(sent, 20);
        let mut read_buffer = vec![0u8; 32];
        let read = context.read(second, &mut read_buffer)?;
        read_buffer.truncate(read as usize);
        assert_eq!(read_buffer, b"pair-uds-dgram-first");

        // write one datagram from second to first
        let sent = context.write(second, b"pair-uds-dgram-second")?;
        assert_eq!(sent, 21);
        let mut read_buffer = vec![0u8; 32];
        let read = context.read(first, &mut read_buffer)?;
        read_buffer.truncate(read as usize);
        assert_eq!(read_buffer, b"pair-uds-dgram-second");

        // close both handles
        context.destack_net_close(first)?;
        context.destack_net_close(second)?;

        Ok(())
    });
}
