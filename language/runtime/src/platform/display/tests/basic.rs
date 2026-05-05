use super::event::drain_window_event_stream;
use super::{
    HarnessValue, decode_display_descriptor, decode_monitor_list, decode_monitor_modes,
    decode_window_descriptor, default_monitor_event_open_options, default_monitor_list_request,
    default_monitor_open_options, default_window_options, error_code, harness_string,
    is_not_supported_code, open_window_or_skip_not_supported, result_or_skip_not_supported,
    run_execution_case_or_return, wait_window_visibility, window_event_open_options_with_filter,
    with_harness_context,
};
#[cfg(windows)]
use crate::platform::core::BackendSupport;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::display as display_platform;
use crate::platform::display::WindowVisibility;
#[cfg(windows)]
use display_platform::DisplayBackend;

#[cfg(windows)]
const DISPLAY_CAP_WINDOW_ICON: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_ICON.0;
#[cfg(windows)]
const DISPLAY_CAP_WINDOW_ASPECT_RATIO: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_ASPECT_RATIO.0;
#[cfg(windows)]
const DISPLAY_CAP_MONITOR_COLOR_STATE: u64 =
    display_platform::DISPLAY_BACKEND_CAP_MONITOR_COLOR_STATE.0;
#[cfg(windows)]
const DISPLAY_CAP_MONITOR_HDR_CONTROL: u64 =
    display_platform::DISPLAY_BACKEND_CAP_MONITOR_HDR_CONTROL.0;
#[cfg(windows)]
const DISPLAY_CAP_MONITOR_GAMMA_CONTROL: u64 =
    display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0;
#[cfg(windows)]
const DISPLAY_CAP_WINDOW_DROP_EVENTS: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_DROP_EVENTS.0;

#[cfg(windows)]
fn support_allows_host_execution(support: BackendSupport) -> bool {
    matches!(support, BackendSupport::Available)
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_display_monitor_surface_lists_opens_and_observes_primary_monitor() {
    if run_execution_case_or_return(display_case_name!(
        test_display_monitor_surface_lists_opens_and_observes_primary_monitor
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

        // use the reported primary when the backend can actually discover one
        let primary_descriptor = monitor_list
            .iter()
            .find(|(_, _, is_primary)| *is_primary)
            .cloned()
            .unwrap_or_else(|| monitor_list[0].clone());
        let (display_id, _, is_primary) = primary_descriptor;

        let display_id_value = harness_string(&mut context, &display_id)?;
        let display = context.destack_display_monitor_open(
            display_id_value,
            default_monitor_open_options(&context),
        )?;
        let descriptor = context.destack_display_monitor_descriptor(display)?;
        let (descriptor_id, _, descriptor_primary) =
            decode_display_descriptor(&mut context, descriptor)?;
        assert_eq!(descriptor_id, display_id);
        assert_eq!(descriptor_primary, is_primary);

        let modes = context.destack_display_monitor_modes(display)?;
        let modes = decode_monitor_modes(&mut context, modes)?;
        assert!(!modes.is_empty());

        let event_stream = context
            .destack_display_monitor_event_open(default_monitor_event_open_options(&context))?;

        for _ in 0..16 {
            let drain = context.destack_display_monitor_event_try_read_batch(event_stream, 32);
            let Err(error) = drain else {
                continue;
            };
            let code = error_code(&error);
            if code == Some(PlatformErrorCode::IoWouldBlock) {
                break;
            }
            return Err(error);
        }

        let requested_mode = context.destack_display_monitor_current_mode(display)?;
        if let Err(error) = context.destack_display_monitor_set_mode(display, requested_mode)
            && !is_not_supported_code(error_code(&error))
        {
            return Err(error);
        }

        let mut saw_mode_changed = false;
        for _ in 0..16 {
            let event = context.destack_display_monitor_event_read(event_stream, 100_000_000)?;
            if matches!(
                event,
                HarnessValue::Native(
                    display_platform::DisplayMonitorEvent::DisplayModeChangedEvent(_)
                ) | HarnessValue::Vm(
                    display_platform::DisplayMonitorEventVm::DisplayModeChangedEvent(_)
                )
            ) {
                saw_mode_changed = true;
                break;
            }
        }
        assert!(saw_mode_changed);

        let primary =
            context.destack_display_monitor_primary(default_monitor_list_request(&context))?;
        assert_eq!(primary.is_some(), is_primary);

        context.destack_display_monitor_event_close(event_stream)?;
        context.destack_display_monitor_close(display)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_display_window_surface_open_mutate_and_observe_roundtrip() {
    if run_execution_case_or_return(display_case_name!(
        test_display_window_surface_open_mutate_and_observe_roundtrip
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "alpha")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let descriptor = context.destack_display_window_descriptor(window)?;
        let (_, title) = decode_window_descriptor(&mut context, descriptor)?;
        assert_eq!(title, "alpha");

        let title = harness_string(&mut context, "beta")?;
        context.destack_display_window_set_title(window, title)?;
        let descriptor = context.destack_display_window_descriptor(window)?;
        let (_, title) = decode_window_descriptor(&mut context, descriptor)?;
        assert_eq!(title, "beta");

        let event_stream =
            context.destack_display_window_event_open(window_event_open_options_with_filter(
                &context,
                256,
                display_platform::DisplayEventOverflowPolicy::DropOldest,
                Some(window),
                Some(display_platform::WINDOW_EVENT_KIND_VISIBILITY_CHANGED.0),
            ))?;

        drain_window_event_stream(&mut context, event_stream)?;
        context.destack_display_window_set_visibility(window, WindowVisibility::Minimized)?;

        let event = context.destack_display_window_event_read(event_stream, 100_000_000)?;
        assert!(matches!(
            event,
            HarnessValue::Native(display_platform::WindowEvent::WindowVisibilityChangedEvent(
                _
            )) | HarnessValue::Vm(display_platform::WindowEventVm::WindowVisibilityChangedEvent(_))
        ));

        assert!(wait_window_visibility(
            &mut context,
            window,
            WindowVisibility::Minimized,
        )?);

        context.destack_display_window_event_close(event_stream)?;
        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[cfg_attr(test, test)]
pub(crate) fn test_display_backend_capabilities_match_win32_implementation() {
    if run_execution_case_or_return(display_case_name!(
        test_display_backend_capabilities_match_win32_implementation
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let backends = context.destack_display_backend_list()?;
        let (win32_available, win32_capability_flags) = match backends {
            HarnessValue::Native(values) => {
                let backends = unsafe { values.as_slice()? };
                let backend = backends
                    .iter()
                    .find(|backend| backend.backend == DisplayBackend::Win32)
                    .expect("backend list should contain win32 descriptor");
                (
                    support_allows_host_execution(backend.support),
                    backend.capability_flags.0,
                )
            }
            HarnessValue::Vm(values) => {
                let vm_context = context
                    .vm_context
                    .map(|vm_context| unsafe {
                        &mut *(vm_context as *mut destack_vm::BindingContext<'_>)
                    })
                    .expect("vm context should exist for vm harness");
                let backends = values.read_values(&vm_context.read())?;
                let backend = backends
                    .iter()
                    .find(|backend| backend.backend == DisplayBackend::Win32)
                    .expect("backend list should contain win32 descriptor");
                (
                    support_allows_host_execution(backend.support),
                    backend.capability_flags.0,
                )
            }
        };

        assert!(win32_available);
        assert_ne!(win32_capability_flags & DISPLAY_CAP_WINDOW_ICON, 0);
        assert_ne!(win32_capability_flags & DISPLAY_CAP_WINDOW_ASPECT_RATIO, 0);
        assert_ne!(win32_capability_flags & DISPLAY_CAP_MONITOR_COLOR_STATE, 0);
        assert_ne!(win32_capability_flags & DISPLAY_CAP_MONITOR_HDR_CONTROL, 0);
        assert_ne!(
            win32_capability_flags & DISPLAY_CAP_MONITOR_GAMMA_CONTROL,
            0
        );
        assert_ne!(win32_capability_flags & DISPLAY_CAP_WINDOW_DROP_EVENTS, 0);

        Ok(())
    });
}
