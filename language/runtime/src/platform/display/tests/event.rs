#[cfg(any(unix, windows))]
use super::HarnessWindowMode;
#[cfg(any(unix, windows))]
use super::harness_window_mode_options;
use super::{
    DisplayHarnessContext, HarnessValue, decode_monitor_list, default_monitor_event_open_options,
    default_monitor_list_request, default_window_event_open_options, default_window_options,
    error_code, is_not_supported_code, monitor_event_open_options,
    monitor_event_open_options_with_kind_mask, open_window_or_skip_not_supported,
    result_or_skip_not_supported, run_execution_case_or_return, window_event_open_options,
    window_event_open_options_with_filter, with_harness_context,
};
#[cfg(any(unix, windows))]
use crate::diagnostic::RuntimeError;
#[cfg(any(unix, windows))]
use crate::diagnostic::RuntimeResult;
#[cfg(any(unix, windows))]
use crate::platform::PlatformError;
use crate::platform::core::BackendSupport;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{display as display_platform, resource};
use display_platform::DisplayEventOverflowPolicy;
#[cfg(any(unix, windows))]
use display_platform::{
    DisplayBackend, DisplayBackendSelectionPolicy, WindowOcclusionState, WindowVisibility,
};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_CLOSE, WM_DISPLAYCHANGE};

#[cfg(any(unix, windows))]
const DISPLAY_CAP_WINDOW_PARENTING: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_PARENTING.0;
#[cfg(any(unix, windows))]
const DISPLAY_CAP_WINDOW_MODAL: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0;

#[cfg(any(unix, windows))]
fn support_allows_host_execution(support: BackendSupport) -> bool {
    matches!(support, BackendSupport::Available)
}

#[cfg(any(unix, windows))]
fn is_concrete_host_backend(backend: DisplayBackend) -> bool {
    !matches!(backend, DisplayBackend::Auto | DisplayBackend::Null)
}

#[cfg(any(unix, windows))]
/// One available backend summary used by behavior tests.
#[derive(Clone, Copy)]
struct BackendDescriptorSummary {
    /// Backend identifier.
    backend: DisplayBackend,
    /// Backend capability bitset.
    capability_flags: u64,
}

#[cfg(any(unix, windows))]
/// One window-event marker used for close-path ordering assertions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowEventMarker {
    /// Destroyed event for one target window.
    Destroyed,
}

#[cfg(any(unix, windows))]
/// Return backend descriptors that can run live host specimen cases here.
fn backend_descriptors_for_host_execution(
    context: &mut DisplayHarnessContext<'_>,
) -> RuntimeResult<Vec<BackendDescriptorSummary>> {
    let descriptors = context.destack_display_backend_list()?;
    let descriptors = match descriptors {
        HarnessValue::Native(values) => unsafe { values.as_slice()? }
            .iter()
            .filter(|descriptor| {
                support_allows_host_execution(descriptor.support)
                    && is_concrete_host_backend(descriptor.backend)
            })
            .map(|descriptor| BackendDescriptorSummary {
                backend: descriptor.backend,
                capability_flags: descriptor.capability_flags.0,
            })
            .collect(),
        HarnessValue::Vm(values) => {
            let Some(vm_context) = context.vm_context else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };
            let vm_context = unsafe { &mut *(vm_context as *mut destack_vm::BindingContext<'_>) };
            values
                .read_values(&vm_context.read())?
                .into_iter()
                .filter(|descriptor| {
                    support_allows_host_execution(descriptor.support)
                        && is_concrete_host_backend(descriptor.backend)
                })
                .map(|descriptor| BackendDescriptorSummary {
                    backend: descriptor.backend,
                    capability_flags: descriptor.capability_flags.0,
                })
                .collect()
        }
    };

    Ok(descriptors)
}

#[cfg(any(unix, windows))]
/// Return whether one backend capability flag is present.
fn has_capability(capability_flags: u64, capability: u64) -> bool {
    capability_flags & capability != 0
}

#[cfg(any(unix, windows))]
/// Force one window-options payload to use one strict backend.
fn force_window_backend(
    options: &mut HarnessValue<display_platform::WindowOptions, display_platform::WindowOptionsVm>,
    backend: DisplayBackend,
) {
    match options {
        HarnessValue::Native(options) => {
            options.backend = backend;
            options.backend_policy = DisplayBackendSelectionPolicy::Strict;
        }
        HarnessValue::Vm(options) => {
            options.backend = backend;
            options.backend_policy = DisplayBackendSelectionPolicy::Strict;
        }
    }
}

#[cfg(any(unix, windows))]
/// Force one window-event open options payload to use one strict backend.
fn force_window_event_backend(
    options: &mut HarnessValue<
        display_platform::WindowEventOpenOptions,
        display_platform::WindowEventOpenOptionsVm,
    >,
    backend: DisplayBackend,
) {
    match options {
        HarnessValue::Native(options) => {
            options.backend = backend;
            options.backend_policy = DisplayBackendSelectionPolicy::Strict;
        }
        HarnessValue::Vm(options) => {
            options.backend = backend;
            options.backend_policy = DisplayBackendSelectionPolicy::Strict;
        }
    }
}

#[cfg(any(unix, windows))]
/// Drain one window-event stream until it reports would-block.
pub(crate) fn drain_window_event_stream(
    context: &mut DisplayHarnessContext<'_>,
    stream: resource::WindowEventHandle,
) -> RuntimeResult<()> {
    loop {
        let event = context.destack_display_window_event_try_read(stream);
        match event {
            Ok(_) => {}
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::IoWouldBlock) {
                    return Ok(());
                }

                return Err(error);
            }
        }
    }
}

#[cfg(any(unix, windows))]
/// Classify one window event for one target window handle.
fn marker_for_window_event(
    event: &HarnessValue<display_platform::WindowEvent, display_platform::WindowEventVm>,
    window: resource::WindowHandle,
) -> Option<WindowEventMarker> {
    // classify destroyed lanes
    if matches!(
        event,
        HarnessValue::Native(
            display_platform::WindowEvent::WindowDestroyedEvent(
                display_platform::WindowDestroyedEvent { metadata, .. }
            )
        ) if metadata.window == window
    ) || matches!(
        event,
        HarnessValue::Vm(
            display_platform::WindowEventVm::WindowDestroyedEvent(
                display_platform::WindowDestroyedEventVm { metadata, .. }
            )
        ) if metadata.window == window
    ) {
        return Some(WindowEventMarker::Destroyed);
    }

    None
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_event_stream_is_seeded() {
    if run_execution_case_or_return(display_case_name!(test_monitor_event_stream_is_seeded)) {
        return;
    }

    with_harness_context(|mut context| {
        let Some(stream) = result_or_skip_not_supported(
            context
                .destack_display_monitor_event_open(default_monitor_event_open_options(&context)),
        )?
        else {
            return Ok(());
        };

        let event = context.destack_display_monitor_event_read(stream, 100_000_000)?;
        assert!(matches!(
            event,
            HarnessValue::Native(display_platform::DisplayMonitorEvent::DisplayAddedEvent(_))
                | HarnessValue::Native(
                    display_platform::DisplayMonitorEvent::DisplayModeChangedEvent(_)
                )
                | HarnessValue::Native(
                    display_platform::DisplayMonitorEvent::DisplayPrimaryChangedEvent(_)
                )
                | HarnessValue::Vm(display_platform::DisplayMonitorEventVm::DisplayAddedEvent(
                    _
                ))
                | HarnessValue::Vm(
                    display_platform::DisplayMonitorEventVm::DisplayModeChangedEvent(_)
                )
                | HarnessValue::Vm(
                    display_platform::DisplayMonitorEventVm::DisplayPrimaryChangedEvent(_)
                )
        ));

        context.destack_display_monitor_event_close(stream)?;
        Ok(())
    });
}

#[cfg(windows)]
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_event_kind_filter_restricts_seeded_events() {
    if run_execution_case_or_return(display_case_name!(
        test_monitor_event_kind_filter_restricts_seeded_events
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let stream = context.destack_display_monitor_event_open(
            monitor_event_open_options_with_kind_mask(
                &context,
                64,
                DisplayEventOverflowPolicy::DropOldest,
                0x10,
            ),
        )?;

        let event = context.destack_display_monitor_event_read(stream, 100_000_000)?;
        assert!(matches!(
            event,
            HarnessValue::Native(display_platform::DisplayMonitorEvent::DisplayModeChangedEvent(_))
                | HarnessValue::Vm(
                    display_platform::DisplayMonitorEventVm::DisplayModeChangedEvent(_)
                )
        ));

        context.destack_display_monitor_event_close(stream)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_event_batch_rejects_zero_maxevents() {
    if run_execution_case_or_return(display_case_name!(
        test_monitor_event_batch_rejects_zero_maxevents
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let Some(stream) = result_or_skip_not_supported(
            context
                .destack_display_monitor_event_open(default_monitor_event_open_options(&context)),
        )?
        else {
            return Ok(());
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
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_event_filter_rejects_invalid_kind_mask() {
    if run_execution_case_or_return(display_case_name!(
        test_monitor_event_filter_rejects_invalid_kind_mask
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let Some(stream) = result_or_skip_not_supported(
            context
                .destack_display_monitor_event_open(default_monitor_event_open_options(&context)),
        )?
        else {
            return Ok(());
        };
        context.destack_display_monitor_event_close(stream)?;

        let result =
            context.destack_display_monitor_event_open(monitor_event_open_options_with_kind_mask(
                &context,
                64,
                DisplayEventOverflowPolicy::DropOldest,
                0x8000_0000,
            ));
        let error = result.expect_err("unsupported monitor kind mask should fail");
        assert!(matches!(
            error_code(&error),
            Some(PlatformErrorCode::InvalidArgument)
                | Some(PlatformErrorCode::InvalidArgumentValue)
        ));

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_event_filter_rejects_invalid_kind_mask() {
    if run_execution_case_or_return(display_case_name!(
        test_window_event_filter_rejects_invalid_kind_mask
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-filter-invalid-mask")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let stream = context
            .destack_display_window_event_open(default_window_event_open_options(&context))?;
        context.destack_display_window_event_close(stream)?;

        let result =
            context.destack_display_window_event_open(window_event_open_options_with_filter(
                &context,
                64,
                DisplayEventOverflowPolicy::DropOldest,
                None,
                Some(0x8000_0000_0000_0000),
            ));
        let error = result.expect_err("unsupported window kind mask should fail");
        assert!(matches!(
            error_code(&error),
            Some(PlatformErrorCode::InvalidArgument)
                | Some(PlatformErrorCode::InvalidArgumentValue)
        ));

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_event_stream_double_close_reports_not_found() {
    if run_execution_case_or_return(display_case_name!(
        test_monitor_event_stream_double_close_reports_not_found
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let Some(stream) = result_or_skip_not_supported(
            context
                .destack_display_monitor_event_open(default_monitor_event_open_options(&context)),
        )?
        else {
            return Ok(());
        };

        context.destack_display_monitor_event_close(stream)?;

        let second_close = context.destack_display_monitor_event_close(stream);
        let error = second_close.expect_err("second monitor event close should fail");
        assert_eq!(error_code(&error), Some(PlatformErrorCode::IoNotFound));

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_event_stream_double_close_reports_not_found() {
    if run_execution_case_or_return(display_case_name!(
        test_window_event_stream_double_close_reports_not_found
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-event-double-close")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let stream = context
            .destack_display_window_event_open(default_window_event_open_options(&context))?;
        context.destack_display_window_event_close(stream)?;

        let second_close = context.destack_display_window_event_close(stream);
        let error = second_close.expect_err("second window event close should fail");
        assert_eq!(error_code(&error), Some(PlatformErrorCode::IoNotFound));

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_event_read_after_stream_close_reports_not_found() {
    if run_execution_case_or_return(display_case_name!(
        test_monitor_event_read_after_stream_close_reports_not_found
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let Some(stream) = result_or_skip_not_supported(
            context
                .destack_display_monitor_event_open(default_monitor_event_open_options(&context)),
        )?
        else {
            return Ok(());
        };
        context.destack_display_monitor_event_close(stream)?;

        let read = context.destack_display_monitor_event_try_read(stream);
        let error = match read {
            Ok(_) => panic!("closed monitor event stream should reject reads"),
            Err(error) => error,
        };
        assert_eq!(error_code(&error), Some(PlatformErrorCode::IoNotFound));

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_event_read_after_stream_close_reports_not_found() {
    if run_execution_case_or_return(display_case_name!(
        test_window_event_read_after_stream_close_reports_not_found
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-event-stale-read")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let stream = context
            .destack_display_window_event_open(default_window_event_open_options(&context))?;
        context.destack_display_window_event_close(stream)?;

        let read = context.destack_display_window_event_try_read(stream);
        let error = match read {
            Ok(_) => panic!("closed window event stream should reject reads"),
            Err(error) => error,
        };
        assert_eq!(error_code(&error), Some(PlatformErrorCode::IoNotFound));

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_event_stream_reports_would_block_after_drain() {
    if run_execution_case_or_return(display_case_name!(
        test_window_event_stream_reports_would_block_after_drain
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-event-drain")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
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
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_event_overflow_error_policy_reports_busy() {
    if run_execution_case_or_return(display_case_name!(
        test_monitor_event_overflow_error_policy_reports_busy
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let Some(monitor_list) = result_or_skip_not_supported(
            context.destack_display_monitor_list(default_monitor_list_request(&context)),
        )?
        else {
            return Ok(());
        };
        let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
        if monitor_list.is_empty() {
            return Ok(());
        }

        let Some(stream) =
            result_or_skip_not_supported(context.destack_display_monitor_event_open(
                monitor_event_open_options(&context, 1, DisplayEventOverflowPolicy::Error),
            ))?
        else {
            return Ok(());
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

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_event_visibility_changes_emit_expected_payloads() {
    if run_execution_case_or_return(display_case_name!(
        test_window_event_visibility_changes_emit_expected_payloads
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let backends = backend_descriptors_for_host_execution(&mut context)?;
        if backends.is_empty() {
            return Ok(());
        }

        for descriptor in backends {
            // open one strict-backend window and stream for behavior verification
            let mut options = default_window_options(&mut context, "window-visibility-events")?;
            force_window_backend(&mut options, descriptor.backend);
            let Some(window) =
                result_or_skip_not_supported(context.destack_display_window_open(options))?
            else {
                continue;
            };

            let mut event_options = default_window_event_open_options(&context);
            force_window_event_backend(&mut event_options, descriptor.backend);
            let Some(stream) = result_or_skip_not_supported(
                context.destack_display_window_event_open(event_options),
            )?
            else {
                context.destack_display_window_close(window)?;
                continue;
            };

            drain_window_event_stream(&mut context, stream)?;

            // request one minimized transition and verify one matching payload when supported
            let minimized_result =
                context.destack_display_window_set_visibility(window, WindowVisibility::Minimized);
            if let Err(error) = minimized_result {
                if is_not_supported_code(error_code(&error)) {
                    context.destack_display_window_event_close(stream)?;
                    context.destack_display_window_close(window)?;
                    continue;
                }

                context.destack_display_window_event_close(stream)?;
                context.destack_display_window_close(window)?;
                return Err(error);
            }

            let mut saw_minimized = false;
            for _ in 0..32 {
                let event = context.destack_display_window_event_read(stream, 100_000_000)?;
                if matches!(
                    event,
                    HarnessValue::Native(
                        display_platform::WindowEvent::WindowVisibilityChangedEvent(
                            display_platform::WindowVisibilityChangedEvent { metadata, payload, .. }
                        )
                    ) if metadata.window == window && payload.current_visibility == WindowVisibility::Minimized
                ) || matches!(
                    event,
                    HarnessValue::Vm(
                        display_platform::WindowEventVm::WindowVisibilityChangedEvent(
                            display_platform::WindowVisibilityChangedEventVm { metadata, payload, .. }
                        )
                    ) if metadata.window == window && payload.current_visibility == WindowVisibility::Minimized
                ) {
                    saw_minimized = true;
                    break;
                }
            }
            assert!(saw_minimized);

            // request one visible transition and verify one matching payload
            context.destack_display_window_set_visibility(window, WindowVisibility::Visible)?;
            let mut saw_visible = false;
            for _ in 0..32 {
                let event = context.destack_display_window_event_read(stream, 100_000_000)?;
                if matches!(
                    event,
                    HarnessValue::Native(
                        display_platform::WindowEvent::WindowVisibilityChangedEvent(
                            display_platform::WindowVisibilityChangedEvent { metadata, payload, .. }
                        )
                    ) if metadata.window == window && payload.current_visibility == WindowVisibility::Visible
                ) || matches!(
                    event,
                    HarnessValue::Vm(
                        display_platform::WindowEventVm::WindowVisibilityChangedEvent(
                            display_platform::WindowVisibilityChangedEventVm { metadata, payload, .. }
                        )
                    ) if metadata.window == window && payload.current_visibility == WindowVisibility::Visible
                ) {
                    saw_visible = true;
                    break;
                }
            }
            assert!(saw_visible);

            context.destack_display_window_event_close(stream)?;
            context.destack_display_window_close(window)?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_event_occlusion_changes_follow_visibility_transitions() {
    if run_execution_case_or_return(display_case_name!(
        test_window_event_occlusion_changes_follow_visibility_transitions
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let backends = backend_descriptors_for_host_execution(&mut context)?;
        if backends.is_empty() {
            return Ok(());
        }

        for descriptor in backends {
            // only explicit occlusion backends should expose this event lane
            if descriptor.capability_flags & display_platform::DISPLAY_BACKEND_CAP_OCCLUSION.0 == 0
            {
                continue;
            }

            // open one strict-backend window and occlusion-only stream
            let mut options = default_window_options(&mut context, "window-occlusion-events")?;
            force_window_backend(&mut options, descriptor.backend);
            let Some(window) =
                result_or_skip_not_supported(context.destack_display_window_open(options))?
            else {
                continue;
            };

            let mut event_options = window_event_open_options_with_filter(
                &context,
                64,
                DisplayEventOverflowPolicy::DropOldest,
                Some(window),
                Some(display_platform::WINDOW_EVENT_KIND_OCCLUSION_CHANGED.0),
            );
            force_window_event_backend(&mut event_options, descriptor.backend);
            let Some(stream) = result_or_skip_not_supported(
                context.destack_display_window_event_open(event_options),
            )?
            else {
                context.destack_display_window_close(window)?;
                continue;
            };

            drain_window_event_stream(&mut context, stream)?;

            // request one minimized transition and verify occluded payload
            context.destack_display_window_set_visibility(window, WindowVisibility::Minimized)?;
            let mut saw_occluded = false;
            for _ in 0..32 {
                let event = context.destack_display_window_event_read(stream, 100_000_000)?;
                if matches!(
                    event,
                    HarnessValue::Native(
                        display_platform::WindowEvent::WindowOcclusionChangedEvent(
                            display_platform::WindowOcclusionChangedEvent { metadata, payload, .. }
                        )
                    ) if metadata.window == window && payload.current_occlusion == WindowOcclusionState::Occluded
                ) || matches!(
                    event,
                    HarnessValue::Vm(
                        display_platform::WindowEventVm::WindowOcclusionChangedEvent(
                            display_platform::WindowOcclusionChangedEventVm { metadata, payload, .. }
                        )
                    ) if metadata.window == window && payload.current_occlusion == WindowOcclusionState::Occluded
                ) {
                    saw_occluded = true;
                    break;
                }
            }
            assert!(saw_occluded);

            // request one visible transition and verify unknown payload
            context.destack_display_window_set_visibility(window, WindowVisibility::Visible)?;
            let mut saw_unknown = false;
            for _ in 0..32 {
                let event = context.destack_display_window_event_read(stream, 100_000_000)?;
                if matches!(
                    event,
                    HarnessValue::Native(
                        display_platform::WindowEvent::WindowOcclusionChangedEvent(
                            display_platform::WindowOcclusionChangedEvent { metadata, payload, .. }
                        )
                    ) if metadata.window == window && payload.current_occlusion == WindowOcclusionState::Unknown
                ) || matches!(
                    event,
                    HarnessValue::Vm(
                        display_platform::WindowEventVm::WindowOcclusionChangedEvent(
                            display_platform::WindowOcclusionChangedEventVm { metadata, payload, .. }
                        )
                    ) if metadata.window == window && payload.current_occlusion == WindowOcclusionState::Unknown
                ) {
                    saw_unknown = true;
                    break;
                }
            }
            assert!(saw_unknown);

            context.destack_display_window_event_close(stream)?;
            context.destack_display_window_close(window)?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_event_relation_and_modal_payloads_match_state_transitions() {
    if run_execution_case_or_return(display_case_name!(
        test_window_event_relation_and_modal_payloads_match_state_transitions
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let backends = backend_descriptors_for_host_execution(&mut context)?;
        if backends.is_empty() {
            return Ok(());
        }

        for descriptor in backends {
            if !has_capability(descriptor.capability_flags, DISPLAY_CAP_WINDOW_PARENTING)
                && !has_capability(descriptor.capability_flags, DISPLAY_CAP_WINDOW_MODAL)
            {
                continue;
            }

            // open one strict-backend child-owner pair and one shared stream
            let mut child_options = default_window_options(&mut context, "window-relation-child")?;
            force_window_backend(&mut child_options, descriptor.backend);
            let Some(child) =
                result_or_skip_not_supported(context.destack_display_window_open(child_options))?
            else {
                continue;
            };

            let mut owner_options = default_window_options(&mut context, "window-relation-owner")?;
            force_window_backend(&mut owner_options, descriptor.backend);
            let Some(owner) =
                result_or_skip_not_supported(context.destack_display_window_open(owner_options))?
            else {
                context.destack_display_window_close(child)?;
                continue;
            };

            let mut event_options = default_window_event_open_options(&context);
            force_window_event_backend(&mut event_options, descriptor.backend);
            let Some(stream) = result_or_skip_not_supported(
                context.destack_display_window_event_open(event_options),
            )?
            else {
                context.destack_display_window_close(owner)?;
                context.destack_display_window_close(child)?;
                continue;
            };

            drain_window_event_stream(&mut context, stream)?;

            // verify transient relation payloads for backends that support parenting lanes
            if has_capability(descriptor.capability_flags, DISPLAY_CAP_WINDOW_PARENTING) {
                context.destack_display_window_set_transient_for(child, Some(owner))?;
                let mut saw_transient_set = false;
                for _ in 0..32 {
                    let event = context.destack_display_window_event_read(stream, 100_000_000)?;
                    if matches!(
                        event,
                        HarnessValue::Native(
                            display_platform::WindowEvent::WindowTransientChangedEvent(
                                display_platform::WindowTransientChangedEvent { metadata, payload, .. }
                            )
                        ) if metadata.window == child && payload.current_transient_for == Some(owner)
                    ) || matches!(
                        event,
                        HarnessValue::Vm(
                            display_platform::WindowEventVm::WindowTransientChangedEvent(
                                display_platform::WindowTransientChangedEventVm { metadata, payload, .. }
                            )
                        ) if metadata.window == child && payload.current_transient_for == Some(owner)
                    ) {
                        saw_transient_set = true;
                        break;
                    }
                }
                assert!(saw_transient_set);

                context.destack_display_window_set_transient_for(child, None)?;
                let mut saw_transient_clear = false;
                for _ in 0..32 {
                    let event = context.destack_display_window_event_read(stream, 100_000_000)?;
                    if matches!(
                        event,
                        HarnessValue::Native(
                            display_platform::WindowEvent::WindowTransientChangedEvent(
                                display_platform::WindowTransientChangedEvent { metadata, payload, .. }
                            )
                        ) if metadata.window == child && payload.current_transient_for.is_none()
                    ) || matches!(
                        event,
                        HarnessValue::Vm(
                            display_platform::WindowEventVm::WindowTransientChangedEvent(
                                display_platform::WindowTransientChangedEventVm { metadata, payload, .. }
                            )
                        ) if metadata.window == child && payload.current_transient_for.is_none()
                    ) {
                        saw_transient_clear = true;
                        break;
                    }
                }
                assert!(saw_transient_clear);
            }

            // verify modal payload transitions for backends that support modal lanes
            if has_capability(descriptor.capability_flags, DISPLAY_CAP_WINDOW_MODAL)
                && has_capability(descriptor.capability_flags, DISPLAY_CAP_WINDOW_PARENTING)
            {
                context.destack_display_window_set_parent(child, Some(owner))?;
                context.destack_display_window_set_modal(child, true)?;

                let mut saw_modal_set = false;
                for _ in 0..32 {
                    let event = context.destack_display_window_event_read(stream, 100_000_000)?;
                    if matches!(
                        event,
                        HarnessValue::Native(
                            display_platform::WindowEvent::WindowModalChangedEvent(
                                display_platform::WindowModalChangedEvent { metadata, payload, .. }
                            )
                        ) if metadata.window == child && payload.current_modal
                    ) || matches!(
                        event,
                        HarnessValue::Vm(
                            display_platform::WindowEventVm::WindowModalChangedEvent(
                                display_platform::WindowModalChangedEventVm { metadata, payload, .. }
                            )
                        ) if metadata.window == child && payload.current_modal
                    ) {
                        saw_modal_set = true;
                        break;
                    }
                }
                assert!(saw_modal_set);

                context.destack_display_window_set_modal(child, false)?;
                let mut saw_modal_clear = false;
                for _ in 0..32 {
                    let event = context.destack_display_window_event_read(stream, 100_000_000)?;
                    if matches!(
                        event,
                        HarnessValue::Native(
                            display_platform::WindowEvent::WindowModalChangedEvent(
                                display_platform::WindowModalChangedEvent { metadata, payload, .. }
                            )
                        ) if metadata.window == child && !payload.current_modal
                    ) || matches!(
                        event,
                        HarnessValue::Vm(
                            display_platform::WindowEventVm::WindowModalChangedEvent(
                                display_platform::WindowModalChangedEventVm { metadata, payload, .. }
                            )
                        ) if metadata.window == child && !payload.current_modal
                    ) {
                        saw_modal_clear = true;
                        break;
                    }
                }
                assert!(saw_modal_clear);
            }

            context.destack_display_window_event_close(stream)?;
            context.destack_display_window_close(owner)?;
            context.destack_display_window_close(child)?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_close_emits_single_destroyed_lifecycle_event() {
    if run_execution_case_or_return(display_case_name!(
        test_window_close_emits_single_destroyed_lifecycle_event
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let backends = backend_descriptors_for_host_execution(&mut context)?;
        if backends.is_empty() {
            return Ok(());
        }

        for descriptor in backends {
            // open one strict-backend window and stream for lifecycle close checks
            let mut options = default_window_options(&mut context, "window-close-lifecycle")?;
            force_window_backend(&mut options, descriptor.backend);
            let Some(window) =
                result_or_skip_not_supported(context.destack_display_window_open(options))?
            else {
                continue;
            };

            let mut event_options = default_window_event_open_options(&context);
            force_window_event_backend(&mut event_options, descriptor.backend);
            let Some(stream) = result_or_skip_not_supported(
                context.destack_display_window_event_open(event_options),
            )?
            else {
                context.destack_display_window_close(window)?;
                continue;
            };

            drain_window_event_stream(&mut context, stream)?;
            context.destack_display_window_close(window)?;

            // drain lifecycle events and enforce single destroyed delivery
            let mut destroyed_count = 0usize;
            let mut close_requested_count = 0usize;
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
                    HarnessValue::Native(
                        display_platform::WindowEvent::WindowDestroyedEvent(
                            display_platform::WindowDestroyedEvent { metadata, .. }
                        )
                    ) if metadata.window == window
                ) || matches!(
                    event,
                    HarnessValue::Vm(
                        display_platform::WindowEventVm::WindowDestroyedEvent(
                            display_platform::WindowDestroyedEventVm { metadata, .. }
                        )
                    ) if metadata.window == window
                ) {
                    destroyed_count = destroyed_count.saturating_add(1);
                }

                if matches!(
                    event,
                    HarnessValue::Native(
                        display_platform::WindowEvent::WindowCloseRequestedEvent(
                            display_platform::WindowCloseRequestedEvent { metadata, .. }
                        )
                    ) if metadata.window == window
                ) || matches!(
                    event,
                    HarnessValue::Vm(
                        display_platform::WindowEventVm::WindowCloseRequestedEvent(
                            display_platform::WindowCloseRequestedEventVm { metadata, .. }
                        )
                    ) if metadata.window == window
                ) {
                    close_requested_count = close_requested_count.saturating_add(1);
                }
            }

            assert_eq!(destroyed_count, 1);
            assert!(close_requested_count <= 1);

            context.destack_display_window_event_close(stream)?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_destroyed_is_terminal_for_window_event_stream() {
    if run_execution_case_or_return(display_case_name!(
        test_window_destroyed_is_terminal_for_window_event_stream
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let backends = backend_descriptors_for_host_execution(&mut context)?;
        if backends.is_empty() {
            return Ok(());
        }

        for descriptor in backends {
            // open one strict-backend window and one stream
            let mut options = default_window_options(&mut context, "window-destroyed-terminal")?;
            force_window_backend(&mut options, descriptor.backend);
            let Some(window) =
                result_or_skip_not_supported(context.destack_display_window_open(options))?
            else {
                continue;
            };

            let mut event_options = default_window_event_open_options(&context);
            force_window_event_backend(&mut event_options, descriptor.backend);
            let Some(stream) = result_or_skip_not_supported(
                context.destack_display_window_event_open(event_options),
            )?
            else {
                context.destack_display_window_close(window)?;
                continue;
            };

            // isolate this sequence from seeded records
            drain_window_event_stream(&mut context, stream)?;

            context.destack_display_window_close(window)?;

            // once destroyed is observed, the same window must not emit more classified events
            let mut saw_destroyed = false;
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

                let marker = marker_for_window_event(&event, window);
                if let Some(marker) = marker
                    && marker == WindowEventMarker::Destroyed
                {
                    saw_destroyed = true;
                }
            }

            assert!(saw_destroyed);
            context.destack_display_window_event_close(stream)?;
        }

        Ok(())
    });
}

#[cfg(windows)]
#[cfg_attr(test, test)]
pub(crate) fn test_window_event_stream_receives_host_close_message() {
    if run_execution_case_or_return(display_case_name!(
        test_window_event_stream_receives_host_close_message
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-host-close")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let stream = context.destack_display_window_event_open(window_event_open_options(
            &context,
            64,
            DisplayEventOverflowPolicy::DropOldest,
        ))?;

        let hwnd = context
            .call_context
            .worker()
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
                HarnessValue::Native(display_platform::WindowEvent::WindowCloseRequestedEvent(_))
                    | HarnessValue::Vm(display_platform::WindowEventVm::WindowCloseRequestedEvent(
                        _
                    ))
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
#[cfg_attr(test, test)]
pub(crate) fn test_window_close_emits_single_lifecycle_events_after_host_close_request() {
    if run_execution_case_or_return(display_case_name!(
        test_window_close_emits_single_lifecycle_events_after_host_close_request
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-close-lifecycle")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let stream = context.destack_display_window_event_open(window_event_open_options(
            &context,
            64,
            DisplayEventOverflowPolicy::DropOldest,
        ))?;

        let hwnd = context
            .call_context
            .worker()
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
                HarnessValue::Native(display_platform::WindowEvent::WindowCloseRequestedEvent(_))
                    | HarnessValue::Vm(display_platform::WindowEventVm::WindowCloseRequestedEvent(
                        _
                    ))
            ) {
                close_requested_count = close_requested_count.saturating_add(1);
            }

            if matches!(
                event,
                HarnessValue::Native(display_platform::WindowEvent::WindowDestroyedEvent(_))
                    | HarnessValue::Vm(display_platform::WindowEventVm::WindowDestroyedEvent(_))
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

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_state_read_does_not_synthesize_window_events() {
    if run_execution_case_or_return(display_case_name!(
        test_window_state_read_does_not_synthesize_window_events
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-state-no-events")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
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

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_set_mode_noop_does_not_emit_mode_event() {
    if run_execution_case_or_return(display_case_name!(
        test_window_set_mode_noop_does_not_emit_mode_event
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "window-mode-noop")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
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
#[cfg_attr(test, test)]
pub(crate) fn test_monitor_event_stream_ignores_noop_displaychange_message() {
    if run_execution_case_or_return(display_case_name!(
        test_monitor_event_stream_ignores_noop_displaychange_message
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let Some(stream) = result_or_skip_not_supported(
            context
                .destack_display_monitor_event_open(default_monitor_event_open_options(&context)),
        )?
        else {
            return Ok(());
        };

        let options = default_window_options(&mut context, "monitor-event-noop-displaychange")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            context.destack_display_monitor_event_close(stream)?;
            return Ok(());
        };

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
            .worker()
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
