#[cfg(unix)]
use super::close_tty_worker_resource;
#[cfg(unix)]
use super::decode_harness_value;
#[cfg(unix)]
use super::tty_descriptor;
use super::{
    assert_ok_or_expected_error, assert_platform_error_codes, open_pty_or_skip_not_supported,
    with_harness_context,
};
#[cfg(unix)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{FileHandle, ResourceId, TtyHandle};
#[cfg(unix)]
use crate::platform::resource::{ResourceEntry, ResourceFinalizer, ResourceKind};
#[cfg(unix)]
use crate::platform::{PlatformError, resource};
#[cfg(unix)]
use crate::runtime::BindingCallContext;

/// Finalizer for unix file descriptors used by tests.
#[cfg(unix)]
#[derive(Debug)]
struct UnixFileFinalizer {
    /// Descriptor to close.
    descriptor: libc::c_int,
}

#[cfg(unix)]
impl ResourceFinalizer for UnixFileFinalizer {
    /// Close the descriptor when the resource is finalized.
    fn finalize(self: Box<Self>, _resource_id: resource::ResourceId) {
        unsafe {
            libc::close(self.descriptor);
        }
    }
}

/// Register one non-terminal file handle for tty detection tests.
#[cfg(unix)]
fn register_non_terminal_file(binding: &BindingCallContext) -> RuntimeResult<FileHandle> {
    let path = std::ffi::CString::new("/dev/null").map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument("invalid null path")).boxed()
    })?;
    let descriptor = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY) };
    if descriptor < 0 {
        return Err(RuntimeError::from(PlatformError::io("failed to open /dev/null")).boxed());
    }

    let entry = ResourceEntry::new(ResourceKind::File)
        .with_label("tty.test.non-terminal")
        .with_fd(descriptor)
        .with_finalizer(UnixFileFinalizer { descriptor });
    let resource_id =
        binding
            .worker()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    Ok(FileHandle(resource_id))
}

/// Reject unknown tty handles for close operations.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_handle_close_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = TtyHandle(ResourceId(0));
        let result = context.destack_tty_close(unknown);
        assert_platform_error_codes(result, &[PlatformErrorCode::InvalidArgumentValue])
    });
}

/// Reject closing one tty handle twice.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_handle_close_rejects_double_close() {
    with_harness_context(|mut context| {
        let pair = open_pty_or_skip_not_supported(&mut context, 24, 80, 0)?;
        let Some(pair) = pair else {
            return Ok(());
        };

        context.destack_tty_close(pair.worker)?;

        let second = context.destack_tty_close(pair.worker);
        assert_platform_error_codes(second, &[PlatformErrorCode::InvalidArgumentValue])?;

        context.destack_tty_pty_close(pair.controller)?;

        Ok(())
    });
}

/// Reject unknown file handles for tty detection.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_handle_is_terminal_file_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = FileHandle(ResourceId(0));
        let result = context.destack_tty_is_terminal_file(unknown);
        assert_platform_error_codes(result, &[PlatformErrorCode::InvalidArgumentValue])
    });
}

/// Report false for known non-terminal files.
#[cfg(unix)]
#[test]
fn test_tty_handle_is_terminal_file_reports_false_for_non_terminal_file() {
    with_harness_context(|mut context| {
        let handle = register_non_terminal_file(context.call_context)?;
        let is_terminal = context.destack_tty_is_terminal_file(handle)?;
        assert!(!is_terminal);

        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            handle.0,
            Some(context.call_context.engine()),
        );

        Ok(())
    });
}

/// Report true for file handles backed by terminal descriptors.
#[cfg(unix)]
#[test]
fn test_tty_handle_is_terminal_file_reports_true_for_terminal_file() {
    with_harness_context(|mut context| {
        let pair = context.destack_tty_pty_open(24, 80, 0)?;
        let pair = decode_harness_value(pair);

        let worker_descriptor = tty_descriptor(context.call_context, pair.worker)?;
        let file_descriptor = unsafe { libc::dup(worker_descriptor) };
        if file_descriptor < 0 {
            return Err(RuntimeError::from(PlatformError::io(
                "failed to duplicate worker descriptor".to_string(),
            ))
            .boxed());
        }

        let file_entry = ResourceEntry::new(ResourceKind::File)
            .with_label("tty.test.terminal")
            .with_fd(file_descriptor)
            .with_finalizer(UnixFileFinalizer {
                descriptor: file_descriptor,
            });
        let file_id = context.call_context.worker().resources.insert(
            context.call_context.world(),
            file_entry,
            Some(context.call_context.engine()),
        );
        let file_handle = FileHandle(file_id);

        let is_terminal = context.destack_tty_is_terminal_file(file_handle)?;
        assert!(is_terminal);

        context.call_context.worker().resources.remove_and_finalize(
            context.call_context.world(),
            file_handle.0,
            Some(context.call_context.engine()),
        );
        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)?;

        Ok(())
    });
}

/// Open and close stdio tty handles or report expected host limitations.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_handle_stdio_open_close_or_expected_errors() {
    with_harness_context(|mut context| {
        let mut opened = Vec::<TtyHandle>::new();

        let stdin_result = context.destack_tty_stdio_stdin();
        let stdin = assert_ok_or_expected_error(
            stdin_result,
            &[
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::IoPermissionDenied,
            ],
        )?;
        if let Some(stdin) = stdin {
            opened.push(stdin);
        }

        let stdout_result = context.destack_tty_stdio_stdout();
        let stdout = assert_ok_or_expected_error(
            stdout_result,
            &[
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::IoPermissionDenied,
            ],
        )?;
        if let Some(stdout) = stdout {
            opened.push(stdout);
        }

        let stderr_result = context.destack_tty_stdio_stderr();
        let stderr = assert_ok_or_expected_error(
            stderr_result,
            &[
                PlatformErrorCode::IoInvalidData,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::IoPermissionDenied,
            ],
        )?;
        if let Some(stderr) = stderr {
            opened.push(stderr);
        }

        for handle in opened {
            context.destack_tty_close(handle)?;
        }

        Ok(())
    });
}
