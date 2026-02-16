use std::os::unix::io::RawFd;

use crate::diagnostic::RuntimeResult;

use super::{set_socket_bool, set_socket_u32};

/// Configure TCP keepalive delay on openbsd.
pub(crate) fn set_keepalive_delay(fd: RawFd, delay_seconds: u32) -> RuntimeResult<()> {
    set_socket_u32(fd, libc::IPPROTO_TCP, libc::TCP_KEEPIDLE, delay_seconds)
}

/// Configure SO_REUSEPORT on openbsd.
pub(crate) fn set_reuse_port(fd: RawFd, enabled: bool) -> RuntimeResult<()> {
    set_socket_bool(fd, libc::SOL_SOCKET, libc::SO_REUSEPORT, enabled)
}

/// Return flags for send on openbsd.
pub(crate) fn send_flags() -> libc::c_int {
    libc::MSG_NOSIGNAL
}
