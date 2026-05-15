use std::io;

use crate::diagnostic::{RuntimeError, RuntimeResult, io_error_code_from_errno};
use crate::host::HostError;

/// Build one secure-random runtime error from one io error.
fn secure_random_io_error(operation: &'static str, error: io::Error) -> Box<RuntimeError> {
    // map errno-derived error codes when available
    let code = error.raw_os_error().and_then(io_error_code_from_errno);

    RuntimeError::from(HostError::random(
        code,
        format!("{operation} failed: {error}"),
    ))
    .boxed()
}

/// Fill one buffer from the unix host entropy backend.
pub(crate) fn host_fill_bytes(buffer: &mut [u8]) -> RuntimeResult<()> {
    // linux: use getrandom syscall directly
    #[cfg(target_os = "linux")]
    {
        return fill_with_getrandom_flags(buffer, 0)
            .map_err(|error| secure_random_io_error("getrandom", error));
    }

    // bsd and macos families: use getentropy with chunking
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        return fill_with_getentropy(buffer)
            .map_err(|error| secure_random_io_error("getentropy", error));
    }

    // fallback: read from urandom
    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )))]
    {
        return fill_with_urandom(buffer)
            .map_err(|error| secure_random_io_error("read /dev/urandom", error));
    }

    #[allow(unreachable_code)]
    Ok(())
}

/// Try to fill one buffer from the unix host entropy backend without blocking.
pub(crate) fn host_try_fill_bytes(buffer: &mut [u8]) -> RuntimeResult<()> {
    // linux: use nonblocking getrandom syscall
    #[cfg(target_os = "linux")]
    {
        return fill_with_getrandom_flags(buffer, libc::GRND_NONBLOCK)
            .map_err(|error| secure_random_io_error("getrandom nonblock", error));
    }

    // bsd and macos families: no nonblocking api, use getentropy directly
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        return fill_with_getentropy(buffer)
            .map_err(|error| secure_random_io_error("getentropy", error));
    }

    // fallback: read from urandom
    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )))]
    {
        return fill_with_urandom(buffer)
            .map_err(|error| secure_random_io_error("read /dev/urandom", error));
    }

    #[allow(unreachable_code)]
    Ok(())
}

/// Fill one buffer with the getrandom syscall.
#[cfg(target_os = "linux")]
fn fill_with_getrandom_flags(buffer: &mut [u8], flags: u32) -> io::Result<()> {
    let mut offset = 0usize;
    while offset < buffer.len() {
        let remaining = &mut buffer[offset..];

        // use the libc symbol on linux
        let read = unsafe {
            libc::getrandom(
                remaining.as_mut_ptr().cast::<libc::c_void>(),
                remaining.len(),
                flags,
            )
        };

        if read < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }

            return Err(error);
        }
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "getrandom returned zero bytes",
            ));
        }

        offset = offset.saturating_add(read as usize);
    }

    Ok(())
}

/// Fill one buffer with the getentropy syscall.
#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn fill_with_getentropy(buffer: &mut [u8]) -> io::Result<()> {
    // getentropy supports at most 256 bytes per call
    const MAX_CHUNK_BYTES: usize = 256;

    for chunk in buffer.chunks_mut(MAX_CHUNK_BYTES) {
        loop {
            let status =
                unsafe { libc::getentropy(chunk.as_mut_ptr().cast::<libc::c_void>(), chunk.len()) };
            if status == 0 {
                break;
            }

            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }

    Ok(())
}

/// Fill one buffer from /dev/urandom.
#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
)))]
fn fill_with_urandom(buffer: &mut [u8]) -> io::Result<()> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open("/dev/urandom")?;
    file.read_exact(buffer)
}
