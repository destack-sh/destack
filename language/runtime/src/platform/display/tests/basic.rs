use super::{
    decode_display_descriptor, decode_harness_value, decode_monitor_list, decode_monitor_modes,
    decode_window_descriptor, default_monitor_event_open_options, default_monitor_list_request,
    default_monitor_open_options, default_window_event_open_options, default_window_options,
    harness_string, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(windows)]
use crate::platform::display as display_platform;
#[cfg(windows)]
use crate::platform::display::DisplayBackend;
use crate::platform::display::WindowVisibility;

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
const DISPLAY_CAP_OCCLUSION: u64 = display_platform::DISPLAY_BACKEND_CAP_OCCLUSION.0;

#[cfg(any(unix, windows))]
#[test]
fn test_display_monitor_surface_works_end_to_end() {
    with_harness_context(|mut context| {
        let monitor_list =
            match context.destack_display_monitor_list(default_monitor_list_request(&context)) {
                Ok(monitor_list) => monitor_list,
                Err(error) => {
                    let code = error.platform_error().map(|platform| platform.code);
                    if code == Some(PlatformErrorCode::NotSupported) {
                        return Ok(());
                    }

                    return Err(error);
                }
            };
        let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
        assert!(!monitor_list.is_empty());

        let (display_id, _, is_primary) = monitor_list[0].clone();
        assert!(is_primary);

        let display_id_value = harness_string(&mut context, &display_id)?;
        let display = context.destack_display_monitor_open(
            display_id_value,
            default_monitor_open_options(&context),
        )?;
        let descriptor = context.destack_display_monitor_descriptor(display)?;
        let (descriptor_id, _, descriptor_primary) =
            decode_display_descriptor(&mut context, descriptor)?;
        assert_eq!(descriptor_id, display_id);
        assert!(descriptor_primary);

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
            let code = error.platform_error().map(|platform| platform.code);
            if code == Some(PlatformErrorCode::IoWouldBlock) {
                break;
            }
            return Err(error);
        }

        let requested_mode = context.destack_display_monitor_current_mode(display)?;
        context.destack_display_monitor_set_mode(display, requested_mode)?;

        let mut saw_mode_changed = false;
        for _ in 0..16 {
            let event = context.destack_display_monitor_event_read(event_stream, 100_000_000)?;
            if matches!(
                event,
                super::HarnessValue::Native(
                    crate::platform::display::DisplayMonitorEvent::DisplayModeChangedEvent(_)
                ) | super::HarnessValue::Vm(
                    crate::platform::display::DisplayMonitorEventVm::DisplayModeChangedEvent(_)
                )
            ) {
                saw_mode_changed = true;
                break;
            }
        }
        assert!(saw_mode_changed);

        let primary =
            context.destack_display_monitor_primary(default_monitor_list_request(&context))?;
        assert!(primary.is_some());

        context.destack_display_monitor_event_close(event_stream)?;
        context.destack_display_monitor_close(display)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_display_window_surface_works_end_to_end() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "alpha")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                let code = error.platform_error().map(|platform| platform.code);
                if code == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let descriptor = context.destack_display_window_descriptor(window)?;
        let (_, title) = decode_window_descriptor(&mut context, descriptor)?;
        assert_eq!(title, "alpha");

        let title = harness_string(&mut context, "beta")?;
        context.destack_display_window_set_title(window, title)?;
        let descriptor = context.destack_display_window_descriptor(window)?;
        let (_, title) = decode_window_descriptor(&mut context, descriptor)?;
        assert_eq!(title, "beta");

        let event_stream = context
            .destack_display_window_event_open(default_window_event_open_options(&context))?;
        context.destack_display_window_request_refresh(window)?;

        let event = context.destack_display_window_event_read(event_stream, 100_000_000)?;
        assert!(matches!(
            event,
            super::HarnessValue::Native(
                crate::platform::display::WindowEvent::WindowRefreshRequestedEvent(_)
            ) | super::HarnessValue::Native(
                crate::platform::display::WindowEvent::WindowCreatedEvent(_)
            ) | super::HarnessValue::Vm(
                crate::platform::display::WindowEventVm::WindowRefreshRequestedEvent(_)
            ) | super::HarnessValue::Vm(
                crate::platform::display::WindowEventVm::WindowCreatedEvent(_)
            )
        ));

        context.destack_display_window_set_visibility(window, WindowVisibility::Minimized)?;
        let state = context.destack_display_window_state(window)?;
        let state = decode_harness_value(state);
        assert_eq!(state.visibility, WindowVisibility::Minimized);

        context.destack_display_window_event_close(event_stream)?;
        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_display_backend_capabilities_match_win32_implementation() {
    with_harness_context(|mut context| {
        let backends = context.destack_display_backend_list()?;
        let (win32_available, win32_capability_flags) = match backends {
            super::HarnessValue::Native(values) => {
                let backends = unsafe { values.as_slice()? };
                let backend = backends
                    .iter()
                    .find(|backend| backend.backend == DisplayBackend::Win32)
                    .expect("backend list should contain win32 descriptor");
                (backend.available, backend.capability_flags.0)
            }
            super::HarnessValue::Vm(values) => {
                let vm_context = context
                    .vm_context
                    .map(|vm_context| unsafe {
                        &mut *(vm_context as *mut destack_vm::ExternalCallContext<'_>)
                    })
                    .expect("vm context should exist for vm harness");
                let backends = values.read_values(vm_context)?;
                let backend = backends
                    .iter()
                    .find(|backend| backend.backend == DisplayBackend::Win32)
                    .expect("backend list should contain win32 descriptor");
                (backend.available, backend.capability_flags.0)
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
        assert_eq!(win32_capability_flags & DISPLAY_CAP_OCCLUSION, 0);

        Ok(())
    });
}
