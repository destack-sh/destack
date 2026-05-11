use super::super::core::overflow_event;
use super::config::*;
use super::core::*;
use super::state::*;
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::Arc;
use std::time::Duration;

use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};

/// Convert one optional deadline into one poll timeout in milliseconds.
pub(super) fn poll_timeout_millis(deadline: Option<Instant>) -> i32 {
    let Some(deadline) = deadline else {
        return MAX_POLL_TIMEOUT_MILLIS;
    };

    let now = Instant::now();
    if now >= deadline {
        return 0;
    }

    let remaining = deadline.duration_since(now);
    let remaining_millis = remaining.as_millis();
    if remaining_millis >= MAX_POLL_TIMEOUT_MILLIS as u128 {
        return MAX_POLL_TIMEOUT_MILLIS;
    }
    if remaining.subsec_nanos() == 0 {
        return remaining_millis as i32;
    }
    if remaining_millis == 0 {
        return 1;
    }

    (remaining_millis as i32).saturating_add(1)
}

/// Wait for one serial descriptor poll result.
pub(super) fn wait_for_poll(
    descriptor: RawFd,
    events: libc::c_short,
    deadline: Option<Instant>,
    operation: &'static str,
    timeout_message: &'static str,
) -> RuntimeResult<libc::c_short> {
    loop {
        let timeout_millis = poll_timeout_millis(deadline);
        let mut poll_descriptor = libc::pollfd {
            fd: descriptor,
            events,
            revents: 0,
        };

        let status = unsafe { libc::poll(&mut poll_descriptor, 1, timeout_millis) };
        if status > 0 {
            return Ok(poll_descriptor.revents);
        }
        if status == 0 {
            return Err(core_platform::io_would_block(operation, timeout_message));
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            if let Some(deadline) = deadline
                && Instant::now() >= deadline
            {
                return Err(core_platform::io_would_block(operation, timeout_message));
            }

            continue;
        }

        return Err(serial_io_error_with_errno(
            operation,
            "poll",
            errno,
            "failed to wait for serial readiness",
        ));
    }
}

/// Queue one unix serial event record.
fn queue_serial_event(resource: &UnixSerialPortResource, record: UnixSerialEventRecord) {
    // gate read-ready records so level-triggered poll does not flood the queue
    if let UnixSerialEventRecord::ReadReady { .. } = record
        && !should_queue_read_ready(resource)
    {
        return;
    }

    resource.event_queue.push_drop_oldest(record);
}

/// Process one unix serial poll mask into queued session events.
fn process_serial_revents(resource: &UnixSerialPortResource, revents: libc::c_short) -> bool {
    // invalid descriptor
    if revents & libc::POLLNVAL != 0 {
        queue_serial_event(resource, UnixSerialEventRecord::Disconnected);

        return false;
    }

    // disconnected
    if revents & libc::POLLHUP != 0 {
        queue_serial_event(resource, UnixSerialEventRecord::Disconnected);

        return false;
    }

    // generic poll error
    if revents & libc::POLLERR != 0 {
        queue_serial_event(
            resource,
            UnixSerialEventRecord::Error {
                kind: SerialErrorKind::Unknown,
                backend_code: None,
                backend_detail: None,
            },
        );
    }

    // modem lines
    if revents & libc::POLLPRI != 0 {
        match read_signals_optional(resource.descriptor, "destack.device.serial.readEvent") {
            Ok(Some(signals)) => {
                if let Some(signals) = update_modem_status(resource, signals) {
                    queue_serial_event(
                        resource,
                        UnixSerialEventRecord::ModemStatusChanged { signals },
                    );
                }
            }
            Ok(None) => {}
            Err(_) => {
                queue_serial_event(
                    resource,
                    UnixSerialEventRecord::Error {
                        kind: SerialErrorKind::Unknown,
                        backend_code: None,
                        backend_detail: None,
                    },
                );

                return false;
            }
        }
    }

    // read readiness
    if revents & libc::POLLIN != 0 {
        queue_serial_event(
            resource,
            UnixSerialEventRecord::ReadReady {
                available_bytes: available_bytes(resource.descriptor),
            },
        );
    }

    true
}

/// Run one unix serial session event loop.
fn unix_serial_event_loop(resource: Arc<UnixSerialPortResource>, shutdown_read: OwnedFd) {
    let descriptor = resource.descriptor;
    let shutdown_descriptor = shutdown_read.as_raw_fd();

    loop {
        let mut poll_descriptors = [
            libc::pollfd {
                fd: descriptor,
                events: libc::POLLIN | libc::POLLPRI | libc::POLLERR | libc::POLLHUP,
                revents: 0,
            },
            libc::pollfd {
                fd: shutdown_descriptor,
                events: libc::POLLIN,
                revents: 0,
            },
        ];

        // wait for either one serial event or one shutdown request
        let status = unsafe {
            libc::poll(
                poll_descriptors.as_mut_ptr(),
                poll_descriptors.len() as _,
                -1,
            )
        };
        if status < 0 {
            let errno = core_platform::get_errno();
            if errno == libc::EINTR {
                continue;
            }

            // surface one terminal backend failure before retiring the event loop
            if matches!(errno, libc::ENODEV | libc::EBADF | libc::EIO) {
                queue_serial_event(&resource, UnixSerialEventRecord::Disconnected);
            } else {
                queue_serial_event(
                    &resource,
                    UnixSerialEventRecord::Error {
                        kind: SerialErrorKind::Unknown,
                        backend_code: Some(errno),
                        backend_detail: None,
                    },
                );
            }

            tracing::warn!("unix serial event poll failed: errno={errno}");

            return;
        }

        // stop immediately when teardown requests shutdown
        if poll_descriptors[1].revents & libc::POLLIN != 0 {
            return;
        }

        // publish the next session-event batch in deterministic priority order
        if poll_descriptors[0].revents != 0
            && !process_serial_revents(&resource, poll_descriptors[0].revents)
        {
            return;
        }
    }
}

/// Start one unix serial session event runtime.
pub(super) fn start_event_runtime(
    resource: Arc<UnixSerialPortResource>,
) -> RuntimeResult<UnixSerialEventRuntime> {
    let (shutdown_read, shutdown_write) = unix_event_shutdown_pipe("destack.device.serial.open")?;

    // run one dedicated session event thread for this descriptor
    let join_handle = start_with_policy(
        "destack-serial-event",
        "destack.device.serial.open",
        ExecutionPolicy::resource(ExecutionMode::Loop),
        move || unix_serial_event_loop(resource, shutdown_read),
    )?;

    Ok(UnixSerialEventRuntime {
        shutdown_write,
        join_handle: Some(join_handle),
    })
}

/// Return whether one descriptor currently reports one hangup condition.
pub(super) fn serial_descriptor_is_hung_up(
    descriptor: RawFd,
    operation: &'static str,
) -> RuntimeResult<bool> {
    loop {
        let mut pollfd = libc::pollfd {
            fd: descriptor,
            events: libc::POLLIN | libc::POLLERR | libc::POLLHUP,
            revents: 0,
        };

        let status = unsafe { libc::poll(&mut pollfd, 1, 0) };
        if status > 0 {
            return Ok(pollfd.revents & libc::POLLHUP != 0);
        }
        if status == 0 {
            return Ok(false);
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }

        return Err(serial_io_error_with_errno(
            operation,
            "poll",
            errno,
            "failed to classify one zero-byte serial read",
        ));
    }
}

/// Read one serial buffer using one timeout-aware nonblocking descriptor.
pub(super) fn read_serial_bytes(
    resource: &UnixSerialPortResource,
    buffer: NativeSlice<u8>,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let descriptor = resource.descriptor;
    let buffer = unsafe { buffer.as_mut_slice()? };
    if buffer.is_empty() {
        return Ok(0);
    }

    // rearm read-ready publication before attempting a new read
    mark_read_ready_queued(resource, false);
    let deadline = core_platform::timeout_deadline(timeout_ns);

    loop {
        let status = unsafe {
            libc::read(
                descriptor,
                buffer.as_mut_ptr().cast::<libc::c_void>(),
                buffer.len(),
            )
        };
        if status > 0 {
            return Ok(status as u64);
        }
        if status == 0 {
            if serial_descriptor_is_hung_up(descriptor, operation)? {
                return Err(RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoNotFound),
                    None,
                    None,
                    Some(operation.to_string()),
                    None,
                    "serial endpoint disconnected during read",
                ))
                .boxed());
            }

            wait_for_poll(
                descriptor,
                libc::POLLIN | libc::POLLERR | libc::POLLHUP,
                deadline,
                operation,
                "serial read would block",
            )?;
            continue;
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }
        if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
            wait_for_poll(
                descriptor,
                libc::POLLIN | libc::POLLERR | libc::POLLHUP,
                deadline,
                operation,
                "serial read would block",
            )?;
            continue;
        }

        return Err(serial_io_error_with_errno(
            operation,
            "read",
            errno,
            "failed to read serial bytes",
        ));
    }
}

/// Read one serial buffer without blocking.
pub(super) fn try_read_serial_bytes(
    resource: &UnixSerialPortResource,
    buffer: NativeSlice<u8>,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let descriptor = resource.descriptor;
    let buffer = unsafe { buffer.as_mut_slice()? };
    if buffer.is_empty() {
        return Ok(0);
    }

    // rearm read-ready publication before attempting a new read
    mark_read_ready_queued(resource, false);

    let status = unsafe {
        libc::read(
            descriptor,
            buffer.as_mut_ptr().cast::<libc::c_void>(),
            buffer.len(),
        )
    };
    if status > 0 {
        return Ok(status as u64);
    }
    if status == 0 {
        if serial_descriptor_is_hung_up(descriptor, operation)? {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoNotFound),
                None,
                None,
                Some(operation.to_string()),
                None,
                "serial endpoint disconnected during read",
            ))
            .boxed());
        }

        return Err(core_platform::io_would_block(
            operation,
            "serial read would block",
        ));
    }

    let errno = core_platform::get_errno();
    if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
        return Err(core_platform::io_would_block(
            operation,
            "serial read would block",
        ));
    }

    Err(serial_io_error_with_errno(
        operation,
        "read",
        errno,
        "failed to read serial bytes",
    ))
}

/// Write one serial buffer using one timeout-aware nonblocking descriptor.
pub(super) fn write_serial_bytes(
    resource: &UnixSerialPortResource,
    data: NativeSlice<u8>,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<u64> {
    let descriptor = resource.descriptor;
    let data = unsafe { data.as_slice()? };
    if data.is_empty() {
        return Ok(0);
    }

    // serialize host write submission with other mutating operations
    let _operation_lock = resource.operation_lock.lock();
    let deadline = core_platform::timeout_deadline(timeout_ns);

    loop {
        let status =
            unsafe { libc::write(descriptor, data.as_ptr().cast::<libc::c_void>(), data.len()) };
        if status > 0 {
            return Ok(status as u64);
        }
        if status == 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::IoNotFound),
                None,
                None,
                Some(operation.to_string()),
                None,
                "serial endpoint disconnected during write",
            ))
            .boxed());
        }

        let errno = core_platform::get_errno();
        if errno == libc::EINTR {
            continue;
        }
        if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
            wait_for_poll(
                descriptor,
                libc::POLLOUT | libc::POLLERR | libc::POLLHUP,
                deadline,
                operation,
                "serial write would block",
            )?;
            continue;
        }

        return Err(serial_io_error_with_errno(
            operation,
            "write",
            errno,
            "failed to write serial bytes",
        ));
    }
}

/// Update one stored modem-status baseline and return whether the snapshot changed.
pub(super) fn update_modem_status(
    resource: &UnixSerialPortResource,
    signals: SerialInputSignals,
) -> Option<SerialInputSignals> {
    let mut state = resource.state.lock();
    let changed = match state.last_signals {
        Some(previous) => previous != signals,
        None => true,
    };
    state.last_signals = Some(signals);

    if changed {
        return Some(signals);
    }

    None
}

/// Wait for one serial event payload and project it into one binding event.
pub(super) fn wait_serial_event(
    binding: &BindingCallContext,
    resource: &UnixSerialPortResource,
    timeout_ns: u64,
    operation: &'static str,
) -> RuntimeResult<SerialEvent> {
    // surface any dropped records before the next queued event
    if let Some(dropped_count) = take_event_overflow_count(resource) {
        let sequence = next_event_sequence(resource);

        return Ok(overflow_event(binding, sequence, dropped_count));
    }

    let timeout = Duration::from_nanos(timeout_ns);
    let Some(record) = resource.event_queue.pop_with_timeout(timeout) else {
        return Err(core_platform::io_would_block(
            operation,
            "serial event would block",
        ));
    };

    Ok(event_from_record(binding, resource, record))
}
