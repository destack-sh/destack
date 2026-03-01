use super::{
    default_monitor_event_open_options, default_window_options, error_code,
    monitor_event_open_options, window_event_open_options, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::display::DisplayEventOverflowPolicy;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_CLOSE};

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
            super::HarnessValue::Native(crate::platform::display::DisplayEvent::DisplayAddedEvent(
                _
            )) | super::HarnessValue::Native(
                crate::platform::display::DisplayEvent::DisplayModeChangedEvent(_)
            ) | super::HarnessValue::Native(
                crate::platform::display::DisplayEvent::DisplayPrimaryChangedEvent(_)
            ) | super::HarnessValue::Vm(
                crate::platform::display::DisplayEventVm::DisplayAddedEvent(_)
            ) | super::HarnessValue::Vm(
                crate::platform::display::DisplayEventVm::DisplayModeChangedEvent(_)
            ) | super::HarnessValue::Vm(
                crate::platform::display::DisplayEventVm::DisplayPrimaryChangedEvent(_)
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
