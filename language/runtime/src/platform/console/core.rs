#[cfg(not(windows))]
use crate::diagnostic::RuntimeError;
use crate::diagnostic::RuntimeResult;
#[cfg(not(windows))]
use crate::platform::PlatformError;
#[cfg(any(unix, windows))]
use crate::platform::core as core_platform;

/// Output selector for console writes.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ConsoleStream {
    /// Write to stdout.
    Stdout,
    /// Write to stderr.
    Stderr,
}

/// Write a console line to the selected stream.
pub(crate) fn write_console_line(line: &str, stream: ConsoleStream) -> RuntimeResult<()> {
    write_console_line_platform(line, stream)
}

// =============================================================================
// Platform-specific console writes
// =============================================================================

#[cfg(unix)]
fn write_console_line_platform(line: &str, stream: ConsoleStream) -> RuntimeResult<()> {
    // select the output file descriptor
    let fd = match stream {
        ConsoleStream::Stdout => libc::STDOUT_FILENO,
        ConsoleStream::Stderr => libc::STDERR_FILENO,
    };

    // write the line and newline using a single writev
    let newline = b"\n";
    let mut iovecs = [
        libc::iovec {
            iov_base: line.as_ptr() as *mut libc::c_void,
            iov_len: line.len(),
        },
        libc::iovec {
            iov_base: newline.as_ptr() as *mut libc::c_void,
            iov_len: newline.len(),
        },
    ];
    core_platform::writev_all_fd(fd, &mut iovecs).map_err(|error| {
        let message = match stream {
            ConsoleStream::Stdout => format!("stdout write failed: {}", error.message()),
            ConsoleStream::Stderr => format!("stderr write failed: {}", error.message()),
        };
        RuntimeError::from(PlatformError::io(message)).boxed()
    })
}

#[cfg(windows)]
fn write_console_line_platform(line: &str, stream: ConsoleStream) -> RuntimeResult<()> {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};

    // select the output handle
    let handle = unsafe {
        match stream {
            ConsoleStream::Stdout => GetStdHandle(STD_OUTPUT_HANDLE),
            ConsoleStream::Stderr => GetStdHandle(STD_ERROR_HANDLE),
        }
    };
    if handle == 0 || handle == INVALID_HANDLE_VALUE {
        return Err(core_platform::io_error("GetStdHandle"));
    }

    // attempt a wide console write first
    let mut wide: Vec<u16> = line.encode_utf16().collect();
    wide.push('\n' as u16);
    if core_platform::write_console_wide(handle, &wide)? {
        return Ok(());
    }

    // fall back to raw bytes for redirected handles
    let mut bytes = line.as_bytes().to_vec();
    bytes.push(b'\n');
    core_platform::write_file_bytes(handle, &bytes)
}

#[cfg(not(any(unix, windows)))]
fn write_console_line_platform(_line: &str, _stream: ConsoleStream) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported("destack.console.write")).boxed())
}
