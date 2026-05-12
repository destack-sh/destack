#[cfg(unix)]
use std::os::fd::AsRawFd;

#[cfg(unix)]
use super::ProcessFdActionKind;
#[cfg(unix)]
use super::unique_temp_file_path;
use super::{
    ProcessFdActionSpec, ProcessSpawnOptionsSpec, ProcessStdioKind, ProcessStdioSpec,
    ProcessWaitKind, shell_exit_command, with_harness_context,
};
#[cfg(unix)]
use super::{
    assert_platform_error_code_with_privileged_policy,
    assert_platform_error_codes_with_privileged_policy,
};
#[cfg(any(target_os = "linux", target_os = "android"))]
use super::{fork_child_sleep_then_exit, is_would_block};
#[cfg(any(target_os = "linux", target_os = "android", windows))]
use super::{shell_sleep_then_exit_command, spawn_shell};
#[cfg(any(unix, windows))]
use crate::diagnostic::RuntimeError;
#[cfg(any(unix, windows))]
use crate::diagnostic::RuntimeResult;
#[cfg(any(unix, windows))]
use crate::platform::PlatformError;
#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::platform::ResourceId;
#[cfg(unix)]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(unix)]
use crate::platform::fs;
use crate::platform::process as process_platform;
#[cfg(any(unix, windows))]
use crate::platform::resource;
#[cfg(unix)]
use crate::platform::resource::ProcessFdHandle;
#[cfg(any(target_os = "linux", target_os = "android"))]
use process_platform::ProcessId;
use process_platform::{ProcessFdFlags, ProcessWaitFlags};
#[cfg(any(target_os = "linux", target_os = "android"))]
use process_platform::{ProcessFdSignalFlags, Signal};

/// Read the current process working directory as UTF-8 text.
fn current_working_directory() -> String {
    std::env::current_dir()
        .expect("cwd should succeed")
        .to_string_lossy()
        .to_string()
}

/// Build one I/O runtime error from the current OS error state.
#[cfg(windows)]
fn windows_io_error(message: &str) -> Box<RuntimeError> {
    let error = std::io::Error::last_os_error();
    RuntimeError::from(PlatformError::io(format!("{message}: {error}"))).boxed()
}

/// Own one Windows CRT pipe descriptor pair for process spawn tests.
#[cfg(windows)]
struct WindowsPipeDescriptors {
    /// Read descriptor owned by the parent.
    read_fd: i32,
    /// Write descriptor used for child stdio wiring.
    write_fd: i32,
}

#[cfg(windows)]
impl WindowsPipeDescriptors {
    /// Create one anonymous pipe descriptor pair.
    fn open() -> RuntimeResult<Self> {
        let mut descriptors = [0_i32; 2];
        let rc = unsafe { libc::pipe(descriptors.as_mut_ptr(), 4096, libc::O_BINARY) };
        if rc != 0 {
            return Err(windows_io_error("failed to open test pipe"));
        }

        Ok(Self {
            read_fd: descriptors[0],
            write_fd: descriptors[1],
        })
    }

    /// Read all bytes from the read descriptor until EOF.
    fn read_all(&self) -> RuntimeResult<Vec<u8>> {
        let mut output = Vec::<u8>::new();

        loop {
            let mut chunk = [0_u8; 256];
            let read = unsafe {
                libc::read(
                    self.read_fd,
                    chunk.as_mut_ptr() as *mut libc::c_void,
                    chunk.len() as u32,
                )
            };
            if read < 0 {
                return Err(windows_io_error("failed to read test pipe"));
            }
            if read == 0 {
                break;
            }

            output.extend_from_slice(&chunk[..read as usize]);
        }

        Ok(output)
    }

    /// Close the write descriptor before reading EOF from the read side.
    fn close_write(&mut self) {
        if self.write_fd >= 0 {
            unsafe {
                libc::close(self.write_fd);
            }
            self.write_fd = -1;
        }
    }
}

#[cfg(windows)]
impl Drop for WindowsPipeDescriptors {
    fn drop(&mut self) {
        if self.write_fd >= 0 {
            unsafe {
                libc::close(self.write_fd);
            }
        }

        if self.read_fd >= 0 {
            unsafe {
                libc::close(self.read_fd);
            }
        }
    }
}

/// Own one unix pipe descriptor pair for process spawn tests.
#[cfg(unix)]
struct UnixPipeDescriptors {
    /// Read descriptor owned by the parent.
    read_fd: i32,
    /// Write descriptor used for child stdio wiring.
    write_fd: i32,
}

#[cfg(unix)]
impl UnixPipeDescriptors {
    /// Create one anonymous pipe descriptor pair.
    fn open() -> RuntimeResult<Self> {
        let mut descriptors = [0_i32; 2];
        let rc = unsafe { libc::pipe(descriptors.as_mut_ptr()) };
        if rc != 0 {
            return Err(RuntimeError::from(PlatformError::io("failed to open test pipe")).boxed());
        }

        Ok(Self {
            read_fd: descriptors[0],
            write_fd: descriptors[1],
        })
    }

    /// Read all bytes from the read descriptor until EOF.
    fn read_all(&self) -> RuntimeResult<Vec<u8>> {
        let mut output = Vec::<u8>::new();

        loop {
            let mut chunk = [0_u8; 256];
            let read = unsafe {
                libc::read(
                    self.read_fd,
                    chunk.as_mut_ptr() as *mut libc::c_void,
                    chunk.len(),
                )
            };
            if read < 0 {
                let error = std::io::Error::last_os_error();
                return Err(RuntimeError::from(PlatformError::io(format!(
                    "failed to read test pipe: {error}",
                )))
                .boxed());
            }
            if read == 0 {
                break;
            }

            output.extend_from_slice(&chunk[..read as usize]);
        }

        Ok(output)
    }

    /// Close the write descriptor before reading EOF from the read side.
    fn close_write(&mut self) {
        if self.write_fd >= 0 {
            unsafe {
                libc::close(self.write_fd);
            }
            self.write_fd = -1;
        }
    }
}

#[cfg(unix)]
impl Drop for UnixPipeDescriptors {
    fn drop(&mut self) {
        if self.write_fd >= 0 {
            unsafe {
                libc::close(self.write_fd);
            }
        }

        if self.read_fd >= 0 {
            unsafe {
                libc::close(self.read_fd);
            }
        }
    }
}

/// Spawn a child process, wait for exit, and reject stale handle reuse.
#[cfg(unix)]
#[test]
fn test_process_spawn_wait_handle_roundtrip() {
    let cwd = current_working_directory();

    with_harness_context(|mut context| {
        let options = ProcessSpawnOptionsSpec::inherit(cwd.clone());
        let command = "/bin/sh";
        let arguments = vec!["-c".to_string(), "exit 17".to_string()];
        let environment = Vec::new();

        // spawn should exit with the requested status
        let handle = context.destack_process_spawn(
            context.path_value(command)?,
            context.string_slice_value(&arguments)?,
            context.string_slice_value(&environment)?,
            context.spawn_options_value(&options)?,
        )?;
        let status = context.destack_process_wait(handle, ProcessWaitFlags(0))?;
        let status = context.wait_status_from_value(status);
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, Some(17));

        // consumed handles should not remain waitable
        assert_platform_error_codes_with_privileged_policy(
            context.destack_process_try_wait(handle),
            &[
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::InvalidArgumentValue,
            ],
        )?;

        Ok(())
    });
}

/// Wait on a process-fd handle for a forked child exit.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_process_fd_wait_roundtrip() {
    with_harness_context(|mut context| {
        let child_pid = fork_child_sleep_then_exit(1, 31)?;
        let handle = context.destack_process_process_fd_open(child_pid, ProcessFdFlags(0))?;
        let status = context.destack_process_process_fd_wait(handle, 2_000_000_000)?;
        let status = context.wait_status_from_value(status);

        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, Some(31));

        context.destack_process_process_fd_close(handle)?;
        Ok(())
    });
}

/// Reject forged process-fd handles created from process handles.
#[cfg(unix)]
#[test]
fn test_process_fd_close_rejects_forged_process_handle() {
    let cwd = current_working_directory();

    with_harness_context(|mut context| {
        let options = ProcessSpawnOptionsSpec::inherit(cwd.clone());
        let command = "/bin/sh";
        let arguments = vec!["-c".to_string(), "sleep 1; exit 0".to_string()];
        let environment = Vec::new();

        // forged process-fd handles should be rejected
        let process_handle = context.destack_process_spawn(
            context.path_value(command)?,
            context.string_slice_value(&arguments)?,
            context.string_slice_value(&environment)?,
            context.spawn_options_value(&options)?,
        )?;
        let forged_handle = ProcessFdHandle(process_handle.0);
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_process_fd_close(forged_handle),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        let status = context.destack_process_wait(process_handle, ProcessWaitFlags(0))?;
        let status = context.wait_status_from_value(status);
        assert_eq!(status.kind, ProcessWaitKind::Exited);

        Ok(())
    });
}

/// Spawn with explicit stdio/actions and observe exit status.
#[cfg(any(unix, windows))]
#[test]
fn test_process_spawn_with_actions_wait_roundtrip() {
    let cwd = current_working_directory();

    with_harness_context(|mut context| {
        let (command, arguments) = shell_exit_command(23);
        let options = ProcessSpawnOptionsSpec {
            cwd: cwd.clone(),
            detached: false,
            reset_signals: true,
            new_process_group: false,
        };
        let environment = Vec::new();
        let stdio = vec![
            ProcessStdioSpec {
                kind: ProcessStdioKind::Null,
                descriptor: 0,
                resource_id: None,
            },
            ProcessStdioSpec {
                kind: ProcessStdioKind::Null,
                descriptor: 0,
                resource_id: None,
            },
            ProcessStdioSpec {
                kind: ProcessStdioKind::Null,
                descriptor: 0,
                resource_id: None,
            },
        ];
        let actions = Vec::<ProcessFdActionSpec>::new();

        let handle = context.destack_process_spawn_with_actions(
            context.path_value(&command)?,
            context.string_slice_value(&arguments)?,
            context.string_slice_value(&environment)?,
            context.spawn_options_value(&options)?,
            context.stdio_slice_value(&stdio)?,
            context.fd_action_slice_value(&actions)?,
        )?;
        let status = context.destack_process_wait(handle, ProcessWaitFlags(0))?;
        let status = context.wait_status_from_value(status);

        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, Some(23));

        Ok(())
    });
}

/// Route spawned child stdout through a provided pipe handle.
#[cfg(any(unix, windows))]
#[test]
fn test_process_spawn_with_actions_pipe_stdout_roundtrip() {
    let cwd = current_working_directory();

    #[cfg(unix)]
    with_harness_context(|mut context| {
        let mut pipe = UnixPipeDescriptors::open()?;

        let call_context = context.call_context;
        let pipe_entry = resource::ResourceEntry::new(resource::ResourceKind::Pipe)
            .with_label("process.test.pipe.stdout")
            .with_fd(pipe.write_fd);
        let pipe_resource_id = call_context.worker().resources.insert(
            call_context.world(),
            pipe_entry,
            Some(call_context.engine()),
        );
        let pipe_handle = resource::PipeHandle(pipe_resource_id);

        let command = "/bin/sh".to_string();
        let arguments = vec!["-c".to_string(), "printf PIPE_STDIO_OK".to_string()];
        let environment = Vec::new();
        let options = ProcessSpawnOptionsSpec::inherit(cwd.clone());

        let stdio = vec![
            ProcessStdioSpec {
                kind: ProcessStdioKind::Inherit,
                descriptor: 0,
                resource_id: None,
            },
            ProcessStdioSpec {
                kind: ProcessStdioKind::Pipe,
                descriptor: 0,
                resource_id: Some(pipe_handle.0),
            },
            ProcessStdioSpec {
                kind: ProcessStdioKind::Inherit,
                descriptor: 0,
                resource_id: None,
            },
        ];
        let actions = Vec::<ProcessFdActionSpec>::new();

        let child = context.destack_process_spawn_with_actions(
            context.path_value(&command)?,
            context.string_slice_value(&arguments)?,
            context.string_slice_value(&environment)?,
            context.spawn_options_value(&options)?,
            context.stdio_slice_value(&stdio)?,
            context.fd_action_slice_value(&actions)?,
        )?;

        // close the parent write descriptor to allow EOF on the read side
        let _ = call_context.worker().resources.remove(
            call_context.world(),
            pipe_resource_id,
            Some(call_context.engine()),
        );
        pipe.close_write();

        // wait for the child process to complete successfully
        let status = context.destack_process_wait(child, ProcessWaitFlags(0))?;
        let status = context.wait_status_from_value(status);
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, Some(0));

        // pipe output should contain the spawned command payload
        let output = pipe.read_all()?;
        let output = String::from_utf8_lossy(&output);
        assert_eq!(output, "PIPE_STDIO_OK");

        Ok(())
    });

    #[cfg(windows)]
    with_harness_context(|mut context| {
        let mut pipe = WindowsPipeDescriptors::open()?;

        let call_context = context.call_context;
        let write_handle_raw = unsafe { libc::get_osfhandle(pipe.write_fd) };
        if write_handle_raw == -1 {
            return Err(windows_io_error(
                "failed to resolve write descriptor handle",
            ));
        }

        let write_handle = write_handle_raw as *mut libc::c_void;
        let pipe_entry = resource::ResourceEntry::new(resource::ResourceKind::Pipe)
            .with_label("process.test.pipe.stdout")
            .with_handle(write_handle);
        let pipe_resource_id = call_context.worker().resources.insert(
            call_context.world(),
            pipe_entry,
            Some(call_context.engine()),
        );
        let pipe_handle = resource::PipeHandle(pipe_resource_id);

        let command = "cmd".to_string();
        let arguments = vec!["/C".to_string(), "echo PIPE_STDIO_OK".to_string()];
        let environment = Vec::new();
        let options = ProcessSpawnOptionsSpec::inherit(cwd.clone());

        let stdio = vec![
            ProcessStdioSpec {
                kind: ProcessStdioKind::Inherit,
                descriptor: 0,
                resource_id: None,
            },
            ProcessStdioSpec {
                kind: ProcessStdioKind::Pipe,
                descriptor: 0,
                resource_id: Some(pipe_handle.0),
            },
            ProcessStdioSpec {
                kind: ProcessStdioKind::Inherit,
                descriptor: 0,
                resource_id: None,
            },
        ];
        let actions = Vec::<ProcessFdActionSpec>::new();

        let child = context.destack_process_spawn_with_actions(
            context.path_value(&command)?,
            context.string_slice_value(&arguments)?,
            context.string_slice_value(&environment)?,
            context.spawn_options_value(&options)?,
            context.stdio_slice_value(&stdio)?,
            context.fd_action_slice_value(&actions)?,
        )?;

        // close the parent write descriptor to allow EOF on the read side
        let _ = call_context.worker().resources.remove(
            call_context.world(),
            pipe_resource_id,
            Some(call_context.engine()),
        );
        pipe.close_write();

        // wait for the child process to complete successfully
        let status = context.destack_process_wait(child, ProcessWaitFlags(0))?;
        let status = context.wait_status_from_value(status);
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, Some(0));

        // pipe output should contain the spawned command payload
        let output = pipe.read_all()?;
        let output = String::from_utf8_lossy(&output);
        assert!(output.contains("PIPE_STDIO_OK"));

        Ok(())
    });
}

/// Wait a short-lived spawned process after a delay and still observe exit status.
#[cfg(any(unix, windows))]
#[test]
fn test_process_spawn_wait_after_exit_delay_roundtrip() {
    let cwd = current_working_directory();

    with_harness_context(|mut context| {
        let (command, arguments) = shell_exit_command(29);
        let options = ProcessSpawnOptionsSpec::inherit(cwd.clone());
        let environment = Vec::new();
        let stdio = Vec::<ProcessStdioSpec>::new();
        let actions = Vec::<ProcessFdActionSpec>::new();

        let handle = context.destack_process_spawn_with_actions(
            context.path_value(&command)?,
            context.string_slice_value(&arguments)?,
            context.string_slice_value(&environment)?,
            context.spawn_options_value(&options)?,
            context.stdio_slice_value(&stdio)?,
            context.fd_action_slice_value(&actions)?,
        )?;

        // allow the child to exit before the wait call
        std::thread::sleep(std::time::Duration::from_millis(100));

        // waits should still observe the terminal status
        let status = context.destack_process_wait(handle, ProcessWaitFlags(0))?;
        let status = context.wait_status_from_value(status);
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, Some(29));

        Ok(())
    });
}

/// Redirect stdout through an open fd action and verify captured output.
#[cfg(unix)]
#[test]
fn test_process_spawn_with_actions_open_stdout_roundtrip() {
    let cwd = current_working_directory();

    with_harness_context(|mut context| {
        let output_path = unique_temp_file_path("SPAWN_OPEN_STDOUT");
        let output_path_text = output_path.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&output_path);

        let options = ProcessSpawnOptionsSpec {
            cwd: cwd.clone(),
            detached: false,
            reset_signals: true,
            new_process_group: false,
        };
        let command = "/bin/sh".to_string();
        let arguments = vec!["-c".to_string(), "printf open-action".to_string()];
        let environment = Vec::new();
        let stdio = Vec::<ProcessStdioSpec>::new();
        let actions = vec![ProcessFdActionSpec {
            op: ProcessFdActionKind::Open,
            source: 0,
            target: 1,
            path: output_path_text.clone(),
            flags: fs::OpenFlags((libc::O_CREAT | libc::O_TRUNC | libc::O_WRONLY) as u32),
            mode: fs::FileMode(0o644),
        }];

        let handle = context.destack_process_spawn_with_actions(
            context.path_value(&command)?,
            context.string_slice_value(&arguments)?,
            context.string_slice_value(&environment)?,
            context.spawn_options_value(&options)?,
            context.stdio_slice_value(&stdio)?,
            context.fd_action_slice_value(&actions)?,
        )?;
        let status = context.destack_process_wait(handle, ProcessWaitFlags(0))?;
        let status = context.wait_status_from_value(status);
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, Some(0));

        // output file should contain child stdout payload
        let output = std::fs::read_to_string(&output_path).expect("output file should be readable");
        assert_eq!(output, "open-action");
        let _ = std::fs::remove_file(&output_path);

        Ok(())
    });
}

/// Redirect stderr to stdout through dup2 action and verify merged output.
#[cfg(unix)]
#[test]
fn test_process_spawn_with_actions_dup2_stderr_roundtrip() {
    let cwd = current_working_directory();

    with_harness_context(|mut context| {
        let output_path = unique_temp_file_path("SPAWN_DUP2_STDERR");
        let output_path_text = output_path.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&output_path);

        let output_file = std::fs::File::create(&output_path).map_err(|error| {
            RuntimeError::from(PlatformError::io(format!(
                "failed to create output file: {error}"
            )))
            .boxed()
        })?;
        let output_descriptor = output_file.as_raw_fd();

        let options = ProcessSpawnOptionsSpec {
            cwd: cwd.clone(),
            detached: false,
            reset_signals: true,
            new_process_group: false,
        };
        let command = "/bin/sh".to_string();
        let arguments = vec!["-c".to_string(), "printf out; printf err 1>&2".to_string()];
        let environment = Vec::new();
        let stdio = vec![
            ProcessStdioSpec {
                kind: ProcessStdioKind::Inherit,
                descriptor: 0,
                resource_id: None,
            },
            ProcessStdioSpec {
                kind: ProcessStdioKind::Descriptor,
                descriptor: output_descriptor,
                resource_id: None,
            },
            ProcessStdioSpec {
                kind: ProcessStdioKind::Inherit,
                descriptor: 0,
                resource_id: None,
            },
        ];
        let actions = vec![ProcessFdActionSpec {
            op: ProcessFdActionKind::Dup2,
            source: 1,
            target: 2,
            path: output_path_text.clone(),
            flags: fs::OpenFlags(0),
            mode: fs::FileMode(0),
        }];

        let handle = context.destack_process_spawn_with_actions(
            context.path_value(&command)?,
            context.string_slice_value(&arguments)?,
            context.string_slice_value(&environment)?,
            context.spawn_options_value(&options)?,
            context.stdio_slice_value(&stdio)?,
            context.fd_action_slice_value(&actions)?,
        )?;
        let status = context.destack_process_wait(handle, ProcessWaitFlags(0))?;
        let status = context.wait_status_from_value(status);
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, Some(0));

        // output file should contain both stdout and redirected stderr payload
        drop(output_file);
        let output = std::fs::read_to_string(&output_path).expect("output file should be readable");
        assert_eq!(output, "outerr");
        let _ = std::fs::remove_file(&output_path);

        Ok(())
    });
}

/// Wait on a process-fd opened from a spawned child process id.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
#[test]
fn test_process_fd_wait_spawn_roundtrip() {
    with_harness_context(|mut context| {
        let (command, arguments) = shell_sleep_then_exit_command(1, 37);
        let child_pid = spawn_shell(command, arguments)?;

        let handle = context.destack_process_process_fd_open(child_pid, ProcessFdFlags(0))?;
        let status = context.destack_process_process_fd_wait(handle, 2_000_000_000)?;
        let status = context.wait_status_from_value(status);

        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, Some(37));
        context.destack_process_process_fd_close(handle)?;

        Ok(())
    });
}

/// Wait a process-fd after child exit delay and still observe terminal status.
#[cfg(any(target_os = "linux", target_os = "android", windows))]
#[test]
fn test_process_fd_wait_after_exit_delay_roundtrip() {
    with_harness_context(|mut context| {
        let (command, arguments) = shell_sleep_then_exit_command(1, 41);
        let child_pid = spawn_shell(command, arguments)?;
        let handle = context.destack_process_process_fd_open(child_pid, ProcessFdFlags(0))?;

        // allow the child process to exit before waiting through the process-fd
        std::thread::sleep(std::time::Duration::from_millis(1500));

        // wait should still return the terminal exit status
        let status = context.destack_process_process_fd_wait(handle, 2_000_000_000)?;
        let status = context.wait_status_from_value(status);
        assert_eq!(status.kind, ProcessWaitKind::Exited);
        assert_eq!(status.exit_code, Some(41));
        context.destack_process_process_fd_close(handle)?;

        Ok(())
    });
}

/// Poll a process-fd, send a no-op signal, and await completion.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_process_fd_try_wait_and_send_signal_roundtrip() {
    with_harness_context(|mut context| {
        let (command, arguments) = shell_sleep_then_exit_command(1, 0);
        let child_pid = spawn_shell(command, arguments)?;

        let handle = context.destack_process_process_fd_open(child_pid, ProcessFdFlags(0))?;

        // try-wait should either report running/exited or a would-block error
        let first_try_wait = context.destack_process_process_fd_try_wait(handle);
        match first_try_wait {
            Ok(status) => {
                let status = context.wait_status_from_value(status);
                assert!(matches!(
                    status.kind,
                    ProcessWaitKind::Running | ProcessWaitKind::Exited
                ));
            }
            Err(error) => {
                assert!(is_would_block(&error));
            }
        }

        context.destack_process_process_fd_send_signal(
            handle,
            Signal(0),
            ProcessFdSignalFlags(0),
        )?;

        let _ = context.destack_process_process_fd_wait(handle, 3_000_000_000)?;
        context.destack_process_process_fd_close(handle)?;

        Ok(())
    });
}

/// Report specific errors for invalid process-fd arguments and handles.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_process_fd_validation_errors_are_specific() {
    with_harness_context(|mut context| {
        let pid = context.destack_process_pid()?;

        // invalid flags and handles should be rejected with invalid-argument errors
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_process_fd_open(pid, ProcessFdFlags(1)),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        let handle = context.destack_process_process_fd_open(pid, ProcessFdFlags(0))?;
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_process_fd_send_signal(
                handle,
                Signal(0),
                ProcessFdSignalFlags(1),
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        context.destack_process_process_fd_close(handle)?;

        let invalid_handle = ProcessFdHandle(ResourceId::local(0));
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_process_fd_try_wait(invalid_handle),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_process_fd_close(invalid_handle),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code_with_privileged_policy(
            context.destack_process_process_fd_send_signal(
                invalid_handle,
                Signal(0),
                ProcessFdSignalFlags(0),
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        // opening an unreachable pid should report one of the expected process errors
        assert_platform_error_codes_with_privileged_policy(
            context.destack_process_process_fd_open(ProcessId(i32::MAX as u32), ProcessFdFlags(0)),
            &[
                PlatformErrorCode::ProcessNotFound,
                PlatformErrorCode::ProcessPermissionDenied,
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::Process,
            ],
        )?;

        let (command, arguments) = shell_sleep_then_exit_command(1, 0);
        let child_pid = spawn_shell(command, arguments)?;
        let timeout_handle =
            context.destack_process_process_fd_open(child_pid, ProcessFdFlags(0))?;
        let timeout_status = context.destack_process_process_fd_wait(timeout_handle, u64::MAX)?;
        let timeout_status = context.wait_status_from_value(timeout_status);
        assert_eq!(timeout_status.kind, ProcessWaitKind::Exited);
        context.destack_process_process_fd_close(timeout_handle)?;

        Ok(())
    });
}

/// Reject process-fd support on unix hosts without pidfd semantics.
#[cfg(all(unix, not(target_os = "linux"), not(target_os = "android")))]
#[test]
fn test_process_fd_reports_not_supported_without_pidfd_semantics() {
    with_harness_context(|mut context| {
        let pid = context.destack_process_pid()?;

        assert_platform_error_code_with_privileged_policy(
            context.destack_process_process_fd_open(pid, ProcessFdFlags(0)),
            PlatformErrorCode::NotSupported,
        )?;

        Ok(())
    });
}
