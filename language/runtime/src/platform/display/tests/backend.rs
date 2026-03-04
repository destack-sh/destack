use super::{
    HarnessValue, decode_harness_value, decode_monitor_list, decode_window_descriptor,
    default_monitor_list_request, default_monitor_open_options, default_window_event_open_options,
    default_window_options, harness_string, result_or_skip_not_supported, with_harness_context,
};
#[cfg(target_os = "linux")]
use super::{error_code, is_not_supported_code};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
#[cfg(target_os = "linux")]
use crate::platform::display as display_platform;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use crate::platform::display::WindowOcclusionState;
use crate::platform::display::{DisplayBackend, DisplayBackendSelectionPolicy, WindowVisibility};

#[cfg(target_os = "linux")]
const DISPLAY_CAP_WINDOW_MODAL: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_WINDOW_ASPECT_RATIO: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_ASPECT_RATIO.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_WINDOW_CHROME: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_CHROME.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_ATTENTION_REQUEST: u64 =
    display_platform::DISPLAY_BACKEND_CAP_ATTENTION_REQUEST.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_WINDOW_DROP_EVENTS: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_DROP_EVENTS.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_MONITOR_HDR_CONTROL: u64 =
    display_platform::DISPLAY_BACKEND_CAP_MONITOR_HDR_CONTROL.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_MONITOR_GAMMA_CONTROL: u64 =
    display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_MONITOR_MODE_SET: u64 = display_platform::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_OCCLUSION: u64 = display_platform::DISPLAY_BACKEND_CAP_OCCLUSION.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_SAFE_AREA: u64 = display_platform::DISPLAY_BACKEND_CAP_SAFE_AREA.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_THEME: u64 = display_platform::DISPLAY_BACKEND_CAP_THEME.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_WINDOW_HIT_TEST: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_HIT_TEST.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_CURSOR_VISIBILITY: u64 =
    display_platform::DISPLAY_BACKEND_CAP_CURSOR_VISIBILITY.0;

/// Force one monitor-list request to use one strict backend.
fn force_monitor_list_backend(
    request: &mut HarnessValue<
        crate::platform::display::DisplayMonitorListRequest,
        crate::platform::display::DisplayMonitorListRequestVm,
    >,
    backend: DisplayBackend,
) {
    match request {
        HarnessValue::Native(request) => {
            request.backend = backend;
            request.backend_policy = DisplayBackendSelectionPolicy::Strict;
        }
        HarnessValue::Vm(request) => {
            request.backend = backend;
            request.backend_policy = DisplayBackendSelectionPolicy::Strict;
        }
    }
}

/// Force one monitor-open options payload to use one strict backend.
fn force_monitor_open_backend(
    options: &mut HarnessValue<
        crate::platform::display::DisplayMonitorOpenOptions,
        crate::platform::display::DisplayMonitorOpenOptionsVm,
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

/// Force one window-options payload to use one strict backend.
fn force_window_backend(
    options: &mut HarnessValue<
        crate::platform::display::WindowOptions,
        crate::platform::display::WindowOptionsVm,
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

/// Force one window-event open options payload to use one strict backend.
fn force_window_event_backend(
    options: &mut HarnessValue<
        crate::platform::display::WindowEventOpenOptions,
        crate::platform::display::WindowEventOpenOptionsVm,
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

/// Return available host display backends for the active harness context.
fn available_backends(
    context: &mut super::DisplayHarnessContext<'_>,
) -> RuntimeResult<Vec<DisplayBackend>> {
    let descriptors = context.destack_display_backend_list()?;
    let backends = match descriptors {
        HarnessValue::Native(values) => unsafe { values.as_slice()? }
            .iter()
            .filter(|descriptor| descriptor.available)
            .map(|descriptor| descriptor.backend)
            .collect(),
        HarnessValue::Vm(values) => {
            let Some(vm_context) = context.vm_context else {
                return Err(RuntimeError::from(PlatformError::invalid_argument(
                    "missing vm context",
                ))
                .boxed());
            };
            let vm_context =
                unsafe { &mut *(vm_context as *mut destack_vm::ExternalCallContext<'_>) };
            values
                .read_values(vm_context)?
                .into_iter()
                .filter(|descriptor| descriptor.available)
                .map(|descriptor| descriptor.backend)
                .collect()
        }
    };

    Ok(backends)
}

#[cfg(any(unix, windows))]
#[test]
fn test_display_monitor_surface_supports_strict_backend_selection() {
    with_harness_context(|mut context| {
        let backends = available_backends(&mut context)?;
        if backends.is_empty() {
            return Ok(());
        }

        for backend in backends {
            let mut list_request = default_monitor_list_request(&context);
            force_monitor_list_backend(&mut list_request, backend);
            let Some(monitor_list) =
                result_or_skip_not_supported(context.destack_display_monitor_list(list_request))?
            else {
                continue;
            };
            let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
            assert!(!monitor_list.is_empty());

            let display_id = harness_string(&mut context, &monitor_list[0].0)?;
            let mut open_options = default_monitor_open_options(&context);
            force_monitor_open_backend(&mut open_options, backend);
            let Some(display) = result_or_skip_not_supported(
                context.destack_display_monitor_open(display_id, open_options),
            )?
            else {
                continue;
            };
            context.destack_display_monitor_close(display)?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_display_window_surface_supports_strict_backend_selection() {
    with_harness_context(|mut context| {
        let backends = available_backends(&mut context)?;
        if backends.is_empty() {
            return Ok(());
        }

        for backend in backends {
            let title = format!("backend-{backend:?}");
            let mut options = default_window_options(&mut context, &title)?;
            force_window_backend(&mut options, backend);
            let Some(window) =
                result_or_skip_not_supported(context.destack_display_window_open(options))?
            else {
                continue;
            };

            let descriptor = context.destack_display_window_descriptor(window)?;
            let (_, current_title) = decode_window_descriptor(&mut context, descriptor)?;
            assert_eq!(current_title, title);

            let mut event_options = default_window_event_open_options(&context);
            force_window_event_backend(&mut event_options, backend);
            let Some(stream) = result_or_skip_not_supported(
                context.destack_display_window_event_open(event_options),
            )?
            else {
                context.destack_display_window_close(window)?;
                continue;
            };

            context.destack_display_window_request_refresh(window)?;
            let event = context.destack_display_window_event_read(stream, 100_000_000)?;
            let saw_refresh_or_created = matches!(
                event,
                HarnessValue::Native(
                    crate::platform::display::WindowEvent::WindowRefreshRequestedEvent(_)
                ) | HarnessValue::Native(
                    crate::platform::display::WindowEvent::WindowCreatedEvent(_)
                ) | HarnessValue::Vm(
                    crate::platform::display::WindowEventVm::WindowRefreshRequestedEvent(_)
                ) | HarnessValue::Vm(crate::platform::display::WindowEventVm::WindowCreatedEvent(
                    _
                ))
            );
            assert!(saw_refresh_or_created);

            context.destack_display_window_set_visibility(window, WindowVisibility::Minimized)?;
            let state = decode_harness_value(context.destack_display_window_state(window)?);
            assert_eq!(state.visibility, WindowVisibility::Minimized);

            context.destack_display_window_event_close(stream)?;
            context.destack_display_window_close(window)?;
        }

        Ok(())
    });
}

#[cfg(target_os = "linux")]
#[test]
fn test_display_x11_capabilities_match_implemented_contract() {
    with_harness_context(|mut context| {
        let descriptors = context.destack_display_backend_list()?;
        let (available, capability_flags) = match descriptors {
            HarnessValue::Native(values) => unsafe { values.as_slice()? }
                .iter()
                .find(|descriptor| descriptor.backend == DisplayBackend::X11)
                .map(|descriptor| (descriptor.available, descriptor.capability_flags.0))
                .expect("backend list should contain x11 descriptor"),
            HarnessValue::Vm(values) => {
                let Some(vm_context) = context.vm_context else {
                    return Err(RuntimeError::from(PlatformError::invalid_argument(
                        "missing vm context",
                    ))
                    .boxed());
                };
                let vm_context =
                    unsafe { &mut *(vm_context as *mut destack_vm::ExternalCallContext<'_>) };
                values
                    .read_values(vm_context)?
                    .into_iter()
                    .find(|descriptor| descriptor.backend == DisplayBackend::X11)
                    .map(|descriptor| (descriptor.available, descriptor.capability_flags.0))
                    .expect("backend list should contain x11 descriptor")
            }
        };

        if !available {
            return Ok(());
        }

        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_MODAL, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_ASPECT_RATIO, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_CHROME, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_ATTENTION_REQUEST, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_DROP_EVENTS, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_MONITOR_HDR_CONTROL, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_OCCLUSION, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_SAFE_AREA, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_THEME, 0);

        let mut window_options = default_window_options(&mut context, "x11-capability-check")?;
        force_window_backend(&mut window_options, DisplayBackend::X11);
        let window = context.destack_display_window_open(window_options)?;

        let mouse_passthrough_result =
            context.destack_display_window_set_mouse_passthrough(window, true);
        match mouse_passthrough_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_HIT_TEST, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_HIT_TEST, 0);
            }
        }

        let cursor_visibility_result =
            context.destack_display_window_set_cursor_visible(window, false);
        match cursor_visibility_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_VISIBILITY, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_CURSOR_VISIBILITY, 0);
            }
        }

        let state = decode_harness_value(context.destack_display_window_state(window)?);
        assert_eq!(state.occlusion, WindowOcclusionState::Unknown);

        context.destack_display_window_close(window)?;

        let mut monitor_request = default_monitor_list_request(&context);
        force_monitor_list_backend(&mut monitor_request, DisplayBackend::X11);
        let monitor_list = context.destack_display_monitor_list(monitor_request)?;
        let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
        assert!(!monitor_list.is_empty());
        let display_id = harness_string(&mut context, &monitor_list[0].0)?;

        let mut monitor_options = default_monitor_open_options(&context);
        force_monitor_open_backend(&mut monitor_options, DisplayBackend::X11);
        let display = context.destack_display_monitor_open(display_id, monitor_options)?;

        let gamma_result = context.destack_display_monitor_gamma_ramp(display);
        match gamma_result {
            Ok(_) => assert_ne!(capability_flags & DISPLAY_CAP_MONITOR_GAMMA_CONTROL, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_MONITOR_GAMMA_CONTROL, 0);
            }
        }

        let current_mode = context.destack_display_monitor_current_mode(display)?;
        let mode_set_result = context.destack_display_monitor_set_mode(display, current_mode);
        match mode_set_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_MONITOR_MODE_SET, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_MONITOR_MODE_SET, 0);
            }
        }

        context.destack_display_monitor_close(display)?;

        Ok(())
    });
}

#[cfg(target_os = "windows")]
#[test]
fn test_display_win32_without_occlusion_capability_reports_unknown_occlusion_state() {
    with_harness_context(|mut context| {
        let mut options = default_window_options(&mut context, "win32-occlusion-contract")?;
        force_window_backend(&mut options, DisplayBackend::Win32);
        let Some(window) =
            result_or_skip_not_supported(context.destack_display_window_open(options))?
        else {
            return Ok(());
        };

        let state = decode_harness_value(context.destack_display_window_state(window)?);
        assert_eq!(state.occlusion, WindowOcclusionState::Unknown);

        context.destack_display_window_close(window)?;
        Ok(())
    });
}
