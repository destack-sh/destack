#[cfg(windows)]
use super::HarnessWindowMode;
#[cfg(windows)]
use super::harness_window_mode_options;
use super::{
    default_monitor_event_open_options, default_window_options, error_code,
    monitor_event_open_options, window_event_open_options, with_harness_context,
};
#[cfg(windows)]
use crate::diagnostic::RuntimeError;
#[cfg(windows)]
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::display::DisplayEventOverflowPolicy;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_CLOSE, WM_DISPLAYCHANGE};

#[cfg(any(unix, windows))]
#[test]
fn test_monitor_event_stream_is_seeded() {
    with_harness_context(|mut context| {
        let stream = match context
            .destack_display_monitor_event_open(default_monitor_event_open_options(&context))
        {
            Ok(stream) => stream,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let event = context.destack_display_monitor_event_read(stream, 100_000_000)?;
        assert!(matches!(
            event,
            super::HarnessValue::Native(
                crate::platform::display::DisplayMonitorEvent::DisplayAddedEvent(_)
            ) | super::HarnessValue::Native(
                crate::platform::display::DisplayMonitorEvent::DisplayModeChangedEvent(_)
            ) | super::HarnessValue::Native(
                crate::platform::display::DisplayMonitorEvent::DisplayPrimaryChangedEvent(_)
            ) | super::HarnessValue::Vm(
                crate::platform::display::DisplayMonitorEventVm::DisplayAddedEvent(_)
            ) | super::HarnessValue::Vm(
                crate::platform::display::DisplayMonitorEventVm::DisplayModeChangedEvent(_)
            ) | super::HarnessValue::Vm(
                crate::platform::display::DisplayMonitorEventVm::DisplayPrimaryChangedEvent(_)
            )
        ));

        context.destack_display_monitor_event_close(stream)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_monitor_event_batch_rejects_zero_maxevents() {
    with_harness_context(|mut context| {
        let stream = match context
            .destack_display_monitor_event_open(default_monitor_event_open_options(&context))
        {
            Ok(stream) => stream,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let result = context.destack_display_monitor_event_read_batch(stream, 0, 100_000_000);
        let error = match result {
            Ok(_) => panic!("zero maxevents should fail"),
            Err(error) => error,
        };
        assert!(matches!(
            error_code(&error),
            Some(PlatformErrorCode::InvalidArgument)
                | Some(PlatformErrorCode::InvalidArgumentValue)
        ));

        context.destack_display_monitor_event_close(stream)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_window_event_stream_reports_would_block_after_drain() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-event-drain")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let stream = context.destack_display_window_event_open(window_event_open_options(
            &context,
            64,
            DisplayEventOverflowPolicy::DropOldest,
        ))?;

        for _ in 0..64 {
            let result = context.destack_display_window_event_try_read(stream);
            match result {
                Ok(_) => {}
                Err(error) => {
                    if error_code(&error) == Some(PlatformErrorCode::IoWouldBlock) {
                        break;
                    }

                    context.destack_display_window_event_close(stream)?;
                    context.destack_display_window_close(window)?;
                    return Err(error);
                }
            }
        }

        let empty = context.destack_display_window_event_try_read(stream);
        let empty_error = match empty {
            Ok(_) => panic!("drained stream should report would-block"),
            Err(error) => error,
        };
        assert_eq!(
            error_code(&empty_error),
            Some(PlatformErrorCode::IoWouldBlock)
        );

        context.destack_display_window_event_close(stream)?;
        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_window_event_overflow_error_policy_reports_busy() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-overflow")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let stream = context.destack_display_window_event_open(window_event_open_options(
            &context,
            1,
            DisplayEventOverflowPolicy::Error,
        ))?;

        for _ in 0..8 {
            let result = context.destack_display_window_event_try_read(stream);
            if let Err(error) = result {
                if error_code(&error) == Some(PlatformErrorCode::IoWouldBlock) {
                    break;
                }

                context.destack_display_window_event_close(stream)?;
                context.destack_display_window_close(window)?;
                return Err(error);
            }
        }

        context.destack_display_window_request_refresh(window)?;
        context.destack_display_window_request_refresh(window)?;
        context.destack_display_window_request_refresh(window)?;

        let overflow = context.destack_display_window_event_try_read(stream);
        let overflow_error = match overflow {
            Ok(_) => panic!("overflow policy error should report busy"),
            Err(error) => error,
        };
        assert_eq!(error_code(&overflow_error), Some(PlatformErrorCode::IoBusy));

        context.destack_display_window_event_close(stream)?;
        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_monitor_event_overflow_error_policy_reports_busy() {
    with_harness_context(|mut context| {
        let stream = match context.destack_display_monitor_event_open(monitor_event_open_options(
            &context,
            1,
            DisplayEventOverflowPolicy::Error,
        )) {
            Ok(stream) => stream,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let overflow = context.destack_display_monitor_event_try_read(stream);
        let overflow_error = match overflow {
            Ok(_) => panic!("overflow policy error should report busy"),
            Err(error) => error,
        };
        assert_eq!(error_code(&overflow_error), Some(PlatformErrorCode::IoBusy));

        context.destack_display_monitor_event_close(stream)?;
        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_window_event_stream_receives_host_close_message() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-host-close")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let stream = context.destack_display_window_event_open(window_event_open_options(
            &context,
            64,
            DisplayEventOverflowPolicy::DropOldest,
        ))?;

        let hwnd = context
            .call_context
            .runtime()
            .resources
            .with_entry(window.0, |entry| entry.raw_handle)
            .flatten()
            .expect("window resource should expose raw hwnd");
        unsafe {
            let _ = SendMessageW(hwnd as isize, WM_CLOSE, 0, 0);
        }

        let mut saw_close_requested = false;
        for _ in 0..16 {
            let event = context.destack_display_window_event_read(stream, 100_000_000)?;
            if matches!(
                event,
                super::HarnessValue::Native(
                    crate::platform::display::WindowEvent::WindowCloseRequestedEvent(_)
                ) | super::HarnessValue::Vm(
                    crate::platform::display::WindowEventVm::WindowCloseRequestedEvent(_)
                )
            ) {
                saw_close_requested = true;
                break;
            }
        }

        assert!(saw_close_requested);
        context.destack_display_window_event_close(stream)?;
        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_window_close_emits_single_lifecycle_events_after_host_close_request() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-close-lifecycle")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let stream = context.destack_display_window_event_open(window_event_open_options(
            &context,
            64,
            DisplayEventOverflowPolicy::DropOldest,
        ))?;

        let hwnd = context
            .call_context
            .runtime()
            .resources
            .with_entry(window.0, |entry| entry.raw_handle)
            .flatten()
            .expect("window resource should expose raw hwnd");
        unsafe {
            let _ = SendMessageW(hwnd as isize, WM_CLOSE, 0, 0);
        }

        context.destack_display_window_close(window)?;

        let mut close_requested_count = 0usize;
        let mut destroyed_count = 0usize;
        loop {
            let event = context.destack_display_window_event_try_read(stream);
            let event = match event {
                Ok(event) => event,
                Err(error) => {
                    if error_code(&error) == Some(PlatformErrorCode::IoWouldBlock) {
                        break;
                    }

                    context.destack_display_window_event_close(stream)?;
                    return Err(error);
                }
            };

            if matches!(
                event,
                super::HarnessValue::Native(
                    crate::platform::display::WindowEvent::WindowCloseRequestedEvent(_)
                ) | super::HarnessValue::Vm(
                    crate::platform::display::WindowEventVm::WindowCloseRequestedEvent(_)
                )
            ) {
                close_requested_count = close_requested_count.saturating_add(1);
            }

            if matches!(
                event,
                super::HarnessValue::Native(
                    crate::platform::display::WindowEvent::WindowDestroyedEvent(_)
                ) | super::HarnessValue::Vm(
                    crate::platform::display::WindowEventVm::WindowDestroyedEvent(_)
                )
            ) {
                destroyed_count = destroyed_count.saturating_add(1);
            }
        }

        assert_eq!(close_requested_count, 1);
        assert_eq!(destroyed_count, 1);
        context.destack_display_window_event_close(stream)?;
        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_window_state_read_does_not_synthesize_window_events() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-state-no-events")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let stream = context.destack_display_window_event_open(window_event_open_options(
            &context,
            64,
            DisplayEventOverflowPolicy::DropOldest,
        ))?;

        // drain any host-originated events that were already queued
        loop {
            let event = context.destack_display_window_event_try_read(stream);
            let Err(error) = event else {
                continue;
            };
            if error_code(&error) == Some(PlatformErrorCode::IoWouldBlock) {
                break;
            }

            context.destack_display_window_event_close(stream)?;
            context.destack_display_window_close(window)?;
            return Err(error);
        }

        let _ = context.destack_display_window_state(window)?;
        let read_result = context.destack_display_window_event_try_read(stream);
        let read_error = match read_result {
            Ok(_) => {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "window_state",
                    "window state read should not emit events",
                ))
                .boxed());
            }
            Err(error) => error,
        };
        assert_eq!(
            error_code(&read_error),
            Some(PlatformErrorCode::IoWouldBlock)
        );

        context.destack_display_window_event_close(stream)?;
        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_window_set_mode_noop_does_not_emit_mode_event() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-mode-noop")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let stream = context.destack_display_window_event_open(window_event_open_options(
            &context,
            64,
            DisplayEventOverflowPolicy::DropOldest,
        ))?;

        loop {
            let event = context.destack_display_window_event_try_read(stream);
            let Err(error) = event else {
                continue;
            };
            if error_code(&error) == Some(PlatformErrorCode::IoWouldBlock) {
                break;
            }

            context.destack_display_window_event_close(stream)?;
            context.destack_display_window_close(window)?;
            return Err(error);
        }

        let mode = harness_window_mode_options(&context, HarnessWindowMode::Windowed);
        context.destack_display_window_set_mode(window, mode)?;

        let event = context.destack_display_window_event_try_read(stream);
        let error = match event {
            Ok(_) => {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "window_mode",
                    "no-op mode change should not emit window mode event",
                ))
                .boxed());
            }
            Err(error) => error,
        };
        assert_eq!(error_code(&error), Some(PlatformErrorCode::IoWouldBlock));

        context.destack_display_window_event_close(stream)?;
        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_monitor_event_stream_ignores_noop_displaychange_message() {
    with_harness_context(|mut context| {
        let stream = match context
            .destack_display_monitor_event_open(default_monitor_event_open_options(&context))
        {
            Ok(stream) => stream,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let options = default_window_options(&mut context, "monitor-event-noop-displaychange")?;
        let window = context.destack_display_window_open(options)?;

        loop {
            let event = context.destack_display_monitor_event_try_read(stream);
            let Err(error) = event else {
                continue;
            };
            if error_code(&error) == Some(PlatformErrorCode::IoWouldBlock) {
                break;
            }

            context.destack_display_window_close(window)?;
            context.destack_display_monitor_event_close(stream)?;
            return Err(error);
        }

        let hwnd = context
            .call_context
            .runtime()
            .resources
            .with_entry(window.0, |entry| entry.raw_handle)
            .flatten()
            .expect("window resource should expose raw hwnd");
        unsafe {
            let _ = SendMessageW(hwnd as isize, WM_DISPLAYCHANGE, 0, 0);
        }

        let event = context.destack_display_monitor_event_try_read(stream);
        let error = match event {
            Ok(_) => {
                return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                    "monitor_event",
                    "noop displaychange should not emit monitor topology events",
                ))
                .boxed());
            }
            Err(error) => error,
        };
        assert_eq!(error_code(&error), Some(PlatformErrorCode::IoWouldBlock));

        context.destack_display_window_close(window)?;
        context.destack_display_monitor_event_close(stream)?;
        Ok(())
    });
}
