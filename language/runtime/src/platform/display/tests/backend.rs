use super::event::drain_window_event_stream;
use super::{
    DisplayHarnessContext, HarnessValue, decode_display_mode, decode_harness_value,
    decode_monitor_list, decode_monitor_modes, decode_window_descriptor,
    default_monitor_list_request, default_monitor_open_options, default_window_event_open_options,
    default_window_options, error_code, harness_string, harness_window_position,
    harness_window_size_constraints, harness_window_size_constraints_none, is_not_supported_code,
    result_or_skip_not_supported, run_execution_case_or_return, wait_window_visibility,
    window_event_open_options_with_filter, with_harness_context,
};
#[cfg(target_os = "linux")]
use super::{
    HarnessWindowMode, harness_window_icon_set, harness_window_icon_set_none,
    harness_window_mode_options,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::core::BackendSupport;
#[cfg(any(unix, windows))]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(any(unix, windows))]
use crate::platform::display as display_platform;
#[cfg(target_os = "linux")]
use crate::platform::resource;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use display_platform::WindowOcclusionState;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use display_platform::WindowRole;
use display_platform::{DisplayBackend, DisplayBackendSelectionPolicy, WindowVisibility};
#[cfg(target_os = "linux")]
use display_platform::{
    WindowAspectRatio, WindowAspectRatioVm, WindowAttentionLevel, WindowChromeKind,
    WindowCursorMode,
};
#[cfg(any(unix, windows))]
use display_platform::{WindowCursorIcon, WindowPosition, WindowResizeEdge};

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
const DISPLAY_CAP_MONITOR_COLOR_STATE: u64 =
    display_platform::DISPLAY_BACKEND_CAP_MONITOR_COLOR_STATE.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_MONITOR_GAMMA_CONTROL: u64 =
    display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_MONITOR_MODE_SET: u64 = display_platform::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_EXCLUSIVE_FULLSCREEN: u64 =
    display_platform::DISPLAY_BACKEND_CAP_EXCLUSIVE_FULLSCREEN.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_BORDERLESS_FULLSCREEN: u64 =
    display_platform::DISPLAY_BACKEND_CAP_BORDERLESS_FULLSCREEN.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_OCCLUSION: u64 = display_platform::DISPLAY_BACKEND_CAP_OCCLUSION.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_SAFE_AREA: u64 = display_platform::DISPLAY_BACKEND_CAP_SAFE_AREA.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_THEME: u64 = display_platform::DISPLAY_BACKEND_CAP_THEME.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_WINDOW_HIT_TEST: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_HIT_TEST.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_ALWAYS_ON_TOP: u64 = display_platform::DISPLAY_BACKEND_CAP_ALWAYS_ON_TOP.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_WINDOW_TASKBAR_VISIBILITY: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_TASKBAR_VISIBILITY.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_WINDOW_ROLE_POPUP: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_POPUP.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_WINDOW_ROLE_OVERLAY: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_OVERLAY.0;
#[cfg(any(unix, windows))]
const DISPLAY_CAP_WINDOW_DRAG_INTERACTION: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_DRAG_INTERACTION.0;
#[cfg(any(unix, windows))]
const DISPLAY_CAP_WINDOW_FOCUS: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_FOCUS.0;
#[cfg(any(unix, windows))]
const DISPLAY_CAP_WINDOW_RAISE: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_RAISE.0;
#[cfg(any(unix, windows))]
const DISPLAY_CAP_WINDOW_PARENTING: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_PARENTING.0;
#[cfg(any(unix, windows))]
const DISPLAY_CAP_CURSOR_ICON_GENERAL: u64 = display_platform::DISPLAY_BACKEND_CAP_CURSOR_ICON.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_CURSOR_VISIBILITY: u64 =
    display_platform::DISPLAY_BACKEND_CAP_CURSOR_VISIBILITY.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_CURSOR_ICON: u64 = display_platform::DISPLAY_BACKEND_CAP_CURSOR_ICON.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_CURSOR_LOCK: u64 = display_platform::DISPLAY_BACKEND_CAP_CURSOR_LOCK.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_CURSOR_CONFINE: u64 = display_platform::DISPLAY_BACKEND_CAP_CURSOR_CONFINE.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_CURSOR_WARP: u64 = display_platform::DISPLAY_BACKEND_CAP_CURSOR_WARP.0;
#[cfg(any(unix, windows))]
const DISPLAY_CAP_WINDOW_OPACITY: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_OPACITY.0;
#[cfg(target_os = "linux")]
const DISPLAY_CAP_WINDOW_ICON: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_ICON.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_MONITOR_MODE_SET: u64 = display_platform::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_MONITOR_COLOR_STATE: u64 =
    display_platform::DISPLAY_BACKEND_CAP_MONITOR_COLOR_STATE.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_MONITOR_HDR_CONTROL: u64 =
    display_platform::DISPLAY_BACKEND_CAP_MONITOR_HDR_CONTROL.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_MONITOR_GAMMA_CONTROL: u64 =
    display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_EXCLUSIVE_FULLSCREEN: u64 =
    display_platform::DISPLAY_BACKEND_CAP_EXCLUSIVE_FULLSCREEN.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_BORDERLESS_FULLSCREEN: u64 =
    display_platform::DISPLAY_BACKEND_CAP_BORDERLESS_FULLSCREEN.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_OCCLUSION: u64 = display_platform::DISPLAY_BACKEND_CAP_OCCLUSION.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_SAFE_AREA: u64 = display_platform::DISPLAY_BACKEND_CAP_SAFE_AREA.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_THEME: u64 = display_platform::DISPLAY_BACKEND_CAP_THEME.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_WINDOW_HIT_TEST: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_HIT_TEST.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_ALWAYS_ON_TOP: u64 = display_platform::DISPLAY_BACKEND_CAP_ALWAYS_ON_TOP.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_WINDOW_TASKBAR_VISIBILITY: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_TASKBAR_VISIBILITY.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_WINDOW_ROLE_POPUP: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_POPUP.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_WINDOW_ROLE_OVERLAY: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_OVERLAY.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_WINDOW_MODAL: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_WINDOW_ASPECT_RATIO: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_ASPECT_RATIO.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_WINDOW_CHROME: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_CHROME.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_ATTENTION_REQUEST: u64 =
    display_platform::DISPLAY_BACKEND_CAP_ATTENTION_REQUEST.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_WINDOW_DROP_EVENTS: u64 =
    display_platform::DISPLAY_BACKEND_CAP_WINDOW_DROP_EVENTS.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_CURSOR_VISIBILITY: u64 =
    display_platform::DISPLAY_BACKEND_CAP_CURSOR_VISIBILITY.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_CURSOR_ICON: u64 = display_platform::DISPLAY_BACKEND_CAP_CURSOR_ICON.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_CURSOR_WARP: u64 = display_platform::DISPLAY_BACKEND_CAP_CURSOR_WARP.0;
#[cfg(target_os = "macos")]
const DISPLAY_CAP_WINDOW_ICON: u64 = display_platform::DISPLAY_BACKEND_CAP_WINDOW_ICON.0;

#[cfg(target_os = "macos")]
/// Return one capability-ceiling mask for the appkit backend.
fn appkit_capability_ceiling_mask() -> u64 {
    display_platform::DISPLAY_BACKEND_CAP_WINDOW.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_STATE.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0
        | display_platform::DISPLAY_BACKEND_CAP_BORDERLESS_FULLSCREEN.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_ICON.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_VISIBILITY.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_WARP.0
        | display_platform::DISPLAY_BACKEND_CAP_TRANSPARENCY.0
        | display_platform::DISPLAY_BACKEND_CAP_ALWAYS_ON_TOP.0
        | display_platform::DISPLAY_BACKEND_CAP_ATTENTION_REQUEST.0
        | display_platform::DISPLAY_BACKEND_CAP_BEGIN_FRAME_STREAM.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_INVALIDATE.0
        | display_platform::DISPLAY_BACKEND_CAP_SAFE_AREA.0
        | display_platform::DISPLAY_BACKEND_CAP_THEME.0
        | display_platform::DISPLAY_BACKEND_CAP_OCCLUSION.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_OPACITY.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ICON.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_FOCUS.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_RAISE.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_HIT_TEST.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_PARENTING.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ASPECT_RATIO.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_DROP_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_CHROME.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_POPUP.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_OVERLAY.0
}

#[cfg(target_os = "linux")]
/// Return one capability-ceiling mask for the x11 backend.
fn x11_capability_ceiling_mask() -> u64 {
    display_platform::DISPLAY_BACKEND_CAP_WINDOW.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_STATE.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_COLOR_STATE.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0
        | display_platform::DISPLAY_BACKEND_CAP_EXCLUSIVE_FULLSCREEN.0
        | display_platform::DISPLAY_BACKEND_CAP_BORDERLESS_FULLSCREEN.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_LOCK.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_CONFINE.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_WARP.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_ICON.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_VISIBILITY.0
        | display_platform::DISPLAY_BACKEND_CAP_TRANSPARENCY.0
        | display_platform::DISPLAY_BACKEND_CAP_ALWAYS_ON_TOP.0
        | display_platform::DISPLAY_BACKEND_CAP_ATTENTION_REQUEST.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_INVALIDATE.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ICON.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_OPACITY.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_FOCUS.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_RAISE.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_HIT_TEST.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_DRAG_INTERACTION.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_PARENTING.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ASPECT_RATIO.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_DROP_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_CHROME.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_TASKBAR_VISIBILITY.0
        | display_platform::DISPLAY_BACKEND_CAP_OCCLUSION.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_POPUP.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_OVERLAY.0
}

#[cfg(target_os = "linux")]
/// Return one capability-ceiling mask for the wayland backend.
fn wayland_capability_ceiling_mask() -> u64 {
    display_platform::DISPLAY_BACKEND_CAP_WINDOW.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_STATE.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_COLOR_STATE.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0
        | display_platform::DISPLAY_BACKEND_CAP_BORDERLESS_FULLSCREEN.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_LOCK.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_CONFINE.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_WARP.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_ICON.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_VISIBILITY.0
        | display_platform::DISPLAY_BACKEND_CAP_TRANSPARENCY.0
        | display_platform::DISPLAY_BACKEND_CAP_ATTENTION_REQUEST.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_INVALIDATE.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ICON.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_OPACITY.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_FOCUS.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_RAISE.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_HIT_TEST.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_DRAG_INTERACTION.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_PARENTING.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_DROP_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_CHROME.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_POPUP.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_OVERLAY.0
}

/// Force one monitor-list request to use one strict backend.
fn force_monitor_list_backend(
    request: &mut HarnessValue<
        display_platform::DisplayMonitorListRequest,
        display_platform::DisplayMonitorListRequestVm,
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
        display_platform::DisplayMonitorOpenOptions,
        display_platform::DisplayMonitorOpenOptionsVm,
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

#[cfg(target_os = "linux")]
/// Build one harness aspect-ratio payload.
fn harness_aspect_ratio(
    context: &DisplayHarnessContext<'_>,
    numerator: u32,
    denominator: u32,
) -> HarnessValue<Option<WindowAspectRatio>, Option<WindowAspectRatioVm>> {
    if context.vm_context.is_some() {
        return HarnessValue::Vm(Some(WindowAspectRatioVm {
            numerator,
            denominator,
        }));
    }

    HarnessValue::Native(Some(WindowAspectRatio {
        numerator,
        denominator,
    }))
}

/// Return available host display backends for the active harness context.
fn available_backends(
    context: &mut DisplayHarnessContext<'_>,
) -> RuntimeResult<Vec<DisplayBackend>> {
    let descriptors = backend_descriptors_for_host_execution(context)?;
    let backends = descriptors
        .into_iter()
        .filter(|descriptor| is_concrete_host_backend(descriptor.backend))
        .map(|descriptor| descriptor.backend)
        .collect();

    Ok(backends)
}

#[cfg(any(unix, windows))]
/// One available backend summary used by cross-backend tests.
#[derive(Clone, Copy)]
struct BackendDescriptorSummary {
    /// Backend identifier.
    backend: DisplayBackend,
    /// Host support state for the selector.
    support: BackendSupport,
    /// Auto-selection priority for the selector.
    priority: u16,
    /// Backend capability bitset.
    capability_flags: u64,
}

#[cfg(any(unix, windows))]
fn support_allows_host_execution(support: BackendSupport) -> bool {
    matches!(support, BackendSupport::Available)
}

#[cfg(any(unix, windows))]
fn is_concrete_host_backend(backend: DisplayBackend) -> bool {
    !matches!(backend, DisplayBackend::Auto | DisplayBackend::Null)
}

#[cfg(any(unix, windows))]
/// Return available backend descriptors for the active harness context.
fn backend_descriptors(
    context: &mut DisplayHarnessContext<'_>,
) -> RuntimeResult<Vec<BackendDescriptorSummary>> {
    let descriptors = context.destack_display_backend_list()?;
    let descriptors = match descriptors {
        HarnessValue::Native(values) => unsafe { values.as_slice()? }
            .iter()
            .map(|descriptor| BackendDescriptorSummary {
                backend: descriptor.backend,
                support: descriptor.support,
                priority: descriptor.priority,
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
                .map(|descriptor| BackendDescriptorSummary {
                    backend: descriptor.backend,
                    support: descriptor.support,
                    priority: descriptor.priority,
                    capability_flags: descriptor.capability_flags.0,
                })
                .collect()
        }
    };

    Ok(descriptors)
}

#[cfg(any(unix, windows))]
/// Return backend descriptors that can run live host specimen cases here.
fn backend_descriptors_for_host_execution(
    context: &mut DisplayHarnessContext<'_>,
) -> RuntimeResult<Vec<BackendDescriptorSummary>> {
    Ok(backend_descriptors(context)?
        .into_iter()
        .filter(|descriptor| support_allows_host_execution(descriptor.support))
        .collect())
}

#[cfg(any(unix, windows))]
/// Return whether one backend capability flag is present.
fn has_capability(capability_flags: u64, capability: u64) -> bool {
    capability_flags & capability != 0
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_display_backend_list_support_contract_matches_advertised_capabilities() {
    if run_execution_case_or_return(display_case_name!(
        test_display_backend_list_support_contract_matches_advertised_capabilities
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let descriptors = backend_descriptors(&mut context)?;
        let auto = descriptors
            .iter()
            .find(|descriptor| descriptor.backend == DisplayBackend::Auto)
            .expect("backend list should contain auto selector");
        let null = descriptors
            .iter()
            .find(|descriptor| descriptor.backend == DisplayBackend::Null)
            .expect("backend list should contain null selector");
        let has_available_host_backend = descriptors.iter().any(|descriptor| {
            is_concrete_host_backend(descriptor.backend)
                && descriptor.support == BackendSupport::Available
        });

        assert_eq!(auto.priority, u16::MAX);
        assert_eq!(null.priority, 0);
        assert_ne!(null.support, BackendSupport::Available);
        assert_eq!(null.capability_flags, 0);

        if has_available_host_backend {
            assert_eq!(auto.support, BackendSupport::Available);
            assert_ne!(auto.capability_flags, 0);
        } else {
            assert_ne!(auto.support, BackendSupport::Available);
            assert_eq!(auto.capability_flags, 0);
        }

        for descriptor in descriptors {
            if descriptor.support == BackendSupport::Available {
                continue;
            }

            assert_eq!(
                descriptor.capability_flags, 0,
                "unavailable backend {:?} should not advertise capability lanes",
                descriptor.backend
            );
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
/// Validate one lane result against one advertised capability flag.
fn expect_capability_lane(
    result: RuntimeResult<()>,
    capability_flags: u64,
    capability: u64,
    allow_would_block: bool,
) -> RuntimeResult<()> {
    match result {
        Ok(()) => {
            assert!(has_capability(capability_flags, capability));
            Ok(())
        }
        Err(error) => {
            let code = error_code(&error);

            // unsupported lanes must report missing backend capability
            if is_not_supported_code(code) {
                assert!(!has_capability(capability_flags, capability));
                return Ok(());
            }

            // interactive lanes can legitimately require host interaction state
            if allow_would_block && code == Some(PlatformErrorCode::IoWouldBlock) {
                assert!(has_capability(capability_flags, capability));
                return Ok(());
            }

            Err(error)
        }
    }
}

#[cfg(any(unix, windows))]
/// Accept one optional lane call where support is backend-specific without a dedicated capability bit.
fn expect_optional_lane(result: RuntimeResult<()>, allow_would_block: bool) -> RuntimeResult<()> {
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            let code = error_code(&error);
            if is_not_supported_code(code) {
                return Ok(());
            }
            if allow_would_block && code == Some(PlatformErrorCode::IoWouldBlock) {
                return Ok(());
            }
            Err(error)
        }
    }
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_display_monitor_surface_supports_strict_backend_selection() {
    if run_execution_case_or_return(display_case_name!(
        test_display_monitor_surface_supports_strict_backend_selection
    )) {
        return;
    }

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
            if monitor_list.is_empty() {
                continue;
            }

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
#[cfg_attr(test, test)]
pub(crate) fn test_display_window_surface_supports_strict_backend_selection() {
    if run_execution_case_or_return(display_case_name!(
        test_display_window_surface_supports_strict_backend_selection
    )) {
        return;
    }

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

            let mut event_options = window_event_open_options_with_filter(
                &context,
                256,
                display_platform::DisplayEventOverflowPolicy::DropOldest,
                Some(window),
                Some(display_platform::WINDOW_EVENT_KIND_VISIBILITY_CHANGED.0),
            );
            force_window_event_backend(&mut event_options, backend);
            let Some(stream) = result_or_skip_not_supported(
                context.destack_display_window_event_open(event_options),
            )?
            else {
                context.destack_display_window_close(window)?;
                continue;
            };

            drain_window_event_stream(&mut context, stream)?;
            context.destack_display_window_set_visibility(window, WindowVisibility::Minimized)?;

            let event = context.destack_display_window_event_read(stream, 100_000_000)?;
            let saw_visibility_changed = matches!(
                event,
                HarnessValue::Native(display_platform::WindowEvent::WindowVisibilityChangedEvent(
                    _
                )) | HarnessValue::Vm(
                    display_platform::WindowEventVm::WindowVisibilityChangedEvent(_)
                )
            );
            assert!(saw_visibility_changed);

            assert!(wait_window_visibility(
                &mut context,
                window,
                WindowVisibility::Minimized,
            )?);

            context.destack_display_window_event_close(stream)?;
            context.destack_display_window_close(window)?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_display_monitor_desktop_mode_is_consistent_with_modes() {
    if run_execution_case_or_return(display_case_name!(
        test_display_monitor_desktop_mode_is_consistent_with_modes
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let backend_descriptors = backend_descriptors_for_host_execution(&mut context)?;
        if backend_descriptors.is_empty() {
            return Ok(());
        }

        for descriptor in backend_descriptors {
            if !has_capability(
                descriptor.capability_flags,
                display_platform::DISPLAY_BACKEND_CAP_MONITOR.0,
            ) {
                continue;
            }

            let mut list_request = default_monitor_list_request(&context);
            force_monitor_list_backend(&mut list_request, descriptor.backend);
            let Some(monitor_list) =
                result_or_skip_not_supported(context.destack_display_monitor_list(list_request))?
            else {
                continue;
            };
            let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
            if monitor_list.is_empty() {
                continue;
            }

            let display_id = harness_string(&mut context, &monitor_list[0].0)?;
            let mut open_options = default_monitor_open_options(&context);
            force_monitor_open_backend(&mut open_options, descriptor.backend);
            let Some(display) = result_or_skip_not_supported(
                context.destack_display_monitor_open(display_id, open_options),
            )?
            else {
                continue;
            };

            let desktop_mode_result = context.destack_display_monitor_desktop_mode(display);
            let desktop_mode = match desktop_mode_result {
                Ok(mode) => decode_display_mode(mode),
                Err(error) => {
                    context.destack_display_monitor_close(display)?;

                    if is_not_supported_code(error_code(&error)) {
                        continue;
                    }

                    return Err(error);
                }
            };

            let modes = context.destack_display_monitor_modes(display)?;
            let modes = decode_monitor_modes(&mut context, modes)?;
            assert!(modes.contains(&desktop_mode));

            context.destack_display_monitor_close(display)?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_display_window_remaining_surface_calls_follow_backend_contract() {
    if run_execution_case_or_return(display_case_name!(
        test_display_window_remaining_surface_calls_follow_backend_contract
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let backend_descriptors = backend_descriptors_for_host_execution(&mut context)?;
        if backend_descriptors.is_empty() {
            return Ok(());
        }

        for descriptor in backend_descriptors {
            if !has_capability(
                descriptor.capability_flags,
                display_platform::DISPLAY_BACKEND_CAP_WINDOW.0,
            ) {
                continue;
            }

            let title = format!(
                "remaining-surface-{backend:?}",
                backend = descriptor.backend
            );
            let mut options = default_window_options(&mut context, &title)?;
            force_window_backend(&mut options, descriptor.backend);
            let Some(window) =
                result_or_skip_not_supported(context.destack_display_window_open(options))?
            else {
                continue;
            };

            let mut owner_options = default_window_options(&mut context, "remaining-owner")?;
            force_window_backend(&mut owner_options, descriptor.backend);
            let owner = match result_or_skip_not_supported(
                context.destack_display_window_open(owner_options),
            )? {
                Some(owner) => owner,
                None => {
                    context.destack_display_window_close(window)?;
                    continue;
                }
            };

            let mut event_options = default_window_event_open_options(&context);
            force_window_event_backend(&mut event_options, descriptor.backend);
            let stream = match result_or_skip_not_supported(
                context.destack_display_window_event_open(event_options),
            )? {
                Some(stream) => stream,
                None => {
                    context.destack_display_window_close(owner)?;
                    context.destack_display_window_close(window)?;
                    continue;
                }
            };

            let batch = context.destack_display_window_event_read_batch(stream, 8, 100_000_000)?;
            let batch_len = match batch {
                HarnessValue::Native(values) => {
                    let values = unsafe { values.as_slice()? };
                    values.len()
                }
                HarnessValue::Vm(values) => {
                    let Some(vm_context) = context.vm_context else {
                        context.destack_display_window_event_close(stream)?;
                        context.destack_display_window_close(owner)?;
                        context.destack_display_window_close(window)?;
                        return Err(RuntimeError::from(PlatformError::invalid_argument(
                            "missing vm context",
                        ))
                        .boxed());
                    };
                    let vm_context =
                        unsafe { &mut *(vm_context as *mut destack_vm::BindingContext<'_>) };
                    values.read_values(&vm_context.read())?.len()
                }
            };
            assert!(batch_len > 0);

            let try_batch = context.destack_display_window_event_try_read_batch(stream, 8);
            match try_batch {
                Ok(batch) => {
                    let batch_len = match batch {
                        HarnessValue::Native(values) => {
                            let values = unsafe { values.as_slice()? };
                            values.len()
                        }
                        HarnessValue::Vm(values) => {
                            let Some(vm_context) = context.vm_context else {
                                context.destack_display_window_event_close(stream)?;
                                context.destack_display_window_close(owner)?;
                                context.destack_display_window_close(window)?;
                                return Err(RuntimeError::from(PlatformError::invalid_argument(
                                    "missing vm context",
                                ))
                                .boxed());
                            };
                            let vm_context = unsafe {
                                &mut *(vm_context as *mut destack_vm::BindingContext<'_>)
                            };
                            values.read_values(&vm_context.read())?.len()
                        }
                    };
                    assert!(batch_len <= 8);
                }
                Err(error) => {
                    if error_code(&error) != Some(PlatformErrorCode::IoWouldBlock) {
                        context.destack_display_window_event_close(stream)?;
                        context.destack_display_window_close(owner)?;
                        context.destack_display_window_close(window)?;
                        return Err(error);
                    }
                }
            }

            expect_capability_lane(
                context.destack_display_window_focus(window),
                descriptor.capability_flags,
                DISPLAY_CAP_WINDOW_FOCUS,
                true,
            )?;
            expect_capability_lane(
                context.destack_display_window_raise(window),
                descriptor.capability_flags,
                DISPLAY_CAP_WINDOW_RAISE,
                true,
            )?;

            let maximize_result = context.destack_display_window_maximize(window);
            match maximize_result {
                Ok(()) => {
                    assert!(wait_window_visibility(
                        &mut context,
                        window,
                        WindowVisibility::Maximized,
                    )?);
                }
                Err(error) => {
                    if !is_not_supported_code(error_code(&error)) {
                        context.destack_display_window_event_close(stream)?;
                        context.destack_display_window_close(owner)?;
                        context.destack_display_window_close(window)?;
                        return Err(error);
                    }
                }
            }

            let minimize_result = context.destack_display_window_minimize(window);
            match minimize_result {
                Ok(()) => {
                    assert!(wait_window_visibility(
                        &mut context,
                        window,
                        WindowVisibility::Minimized,
                    )?);
                }
                Err(error) => {
                    if !is_not_supported_code(error_code(&error)) {
                        context.destack_display_window_event_close(stream)?;
                        context.destack_display_window_close(owner)?;
                        context.destack_display_window_close(window)?;
                        return Err(error);
                    }
                }
            }

            let restore_result = context.destack_display_window_restore(window);
            match restore_result {
                Ok(()) => {
                    let restored_visible =
                        wait_window_visibility(&mut context, window, WindowVisibility::Visible)?;
                    let restored_maximized =
                        wait_window_visibility(&mut context, window, WindowVisibility::Maximized)?;
                    assert!(restored_visible || restored_maximized);
                }
                Err(error) => {
                    if !is_not_supported_code(error_code(&error)) {
                        context.destack_display_window_event_close(stream)?;
                        context.destack_display_window_close(owner)?;
                        context.destack_display_window_close(window)?;
                        return Err(error);
                    }
                }
            }
            expect_optional_lane(
                context.destack_display_window_set_resizable(window, false),
                false,
            )?;
            expect_optional_lane(
                context.destack_display_window_set_resizable(window, true),
                false,
            )?;
            expect_optional_lane(
                context.destack_display_window_set_decorated(window, false),
                false,
            )?;
            expect_optional_lane(
                context.destack_display_window_set_decorated(window, true),
                false,
            )?;
            let requested_position = WindowPosition { x: 120, y: 130 };
            let set_position_result = context.destack_display_window_set_position(
                window,
                harness_window_position(&context, requested_position.x, requested_position.y),
            );
            match set_position_result {
                Ok(()) => {
                    let state = decode_harness_value(context.destack_display_window_state(window)?);
                    assert_eq!(state.position, requested_position);
                }
                Err(error) => {
                    if !is_not_supported_code(error_code(&error)) {
                        context.destack_display_window_event_close(stream)?;
                        context.destack_display_window_close(owner)?;
                        context.destack_display_window_close(window)?;
                        return Err(error);
                    }
                }
            }
            expect_optional_lane(
                context.destack_display_window_set_size_constraints(
                    window,
                    harness_window_size_constraints(&context, 320.0, 240.0, 1920.0, 1080.0),
                ),
                false,
            )?;
            expect_optional_lane(
                context.destack_display_window_set_size_constraints(
                    window,
                    harness_window_size_constraints_none(&context),
                ),
                false,
            )?;

            let set_transient_result =
                context.destack_display_window_set_transient_for(window, Some(owner));
            match set_transient_result {
                Ok(()) => {
                    assert!(has_capability(
                        descriptor.capability_flags,
                        DISPLAY_CAP_WINDOW_PARENTING
                    ));
                    let state = decode_harness_value(context.destack_display_window_state(window)?);
                    assert_eq!(state.transient_for, Some(owner));
                }
                Err(error) => {
                    let code = error_code(&error);
                    if is_not_supported_code(code) {
                        assert!(!has_capability(
                            descriptor.capability_flags,
                            DISPLAY_CAP_WINDOW_PARENTING
                        ));
                    } else if code != Some(PlatformErrorCode::IoWouldBlock) {
                        context.destack_display_window_event_close(stream)?;
                        context.destack_display_window_close(owner)?;
                        context.destack_display_window_close(window)?;
                        return Err(error);
                    }
                }
            }

            let clear_transient_result =
                context.destack_display_window_set_transient_for(window, None);
            match clear_transient_result {
                Ok(()) => {
                    assert!(has_capability(
                        descriptor.capability_flags,
                        DISPLAY_CAP_WINDOW_PARENTING
                    ));
                    let state = decode_harness_value(context.destack_display_window_state(window)?);
                    assert_eq!(state.transient_for, None);
                }
                Err(error) => {
                    let code = error_code(&error);
                    if is_not_supported_code(code) {
                        assert!(!has_capability(
                            descriptor.capability_flags,
                            DISPLAY_CAP_WINDOW_PARENTING
                        ));
                    } else if code != Some(PlatformErrorCode::IoWouldBlock) {
                        context.destack_display_window_event_close(stream)?;
                        context.destack_display_window_close(owner)?;
                        context.destack_display_window_close(window)?;
                        return Err(error);
                    }
                }
            }
            expect_capability_lane(
                context.destack_display_window_set_cursor_icon(window, WindowCursorIcon::Pointer),
                descriptor.capability_flags,
                DISPLAY_CAP_CURSOR_ICON_GENERAL,
                true,
            )?;

            let set_opacity_result = context.destack_display_window_set_opacity(window, 0.75);
            match set_opacity_result {
                Ok(()) => {
                    assert!(has_capability(
                        descriptor.capability_flags,
                        DISPLAY_CAP_WINDOW_OPACITY
                    ));
                    let state = decode_harness_value(context.destack_display_window_state(window)?);
                    assert!((state.opacity - 0.75).abs() < 0.02);
                }
                Err(error) => {
                    let code = error_code(&error);
                    if is_not_supported_code(code) {
                        assert!(!has_capability(
                            descriptor.capability_flags,
                            DISPLAY_CAP_WINDOW_OPACITY
                        ));
                    } else if code != Some(PlatformErrorCode::IoWouldBlock) {
                        context.destack_display_window_event_close(stream)?;
                        context.destack_display_window_close(owner)?;
                        context.destack_display_window_close(window)?;
                        return Err(error);
                    }
                }
            }

            let opacity_result = context.destack_display_window_opacity(window);
            match opacity_result {
                Ok(opacity) => {
                    assert!(has_capability(
                        descriptor.capability_flags,
                        DISPLAY_CAP_WINDOW_OPACITY
                    ));
                    assert!((0.0..=1.0).contains(&opacity));
                }
                Err(error) => {
                    let code = error_code(&error);
                    if is_not_supported_code(code) {
                        assert!(!has_capability(
                            descriptor.capability_flags,
                            DISPLAY_CAP_WINDOW_OPACITY
                        ));
                    } else {
                        context.destack_display_window_event_close(stream)?;
                        context.destack_display_window_close(owner)?;
                        context.destack_display_window_close(window)?;
                        return Err(error);
                    }
                }
            }

            expect_capability_lane(
                context
                    .destack_display_window_begin_resize_drag(window, WindowResizeEdge::NorthWest),
                descriptor.capability_flags,
                DISPLAY_CAP_WINDOW_DRAG_INTERACTION,
                true,
            )?;

            context.destack_display_window_event_close(stream)?;
            context.destack_display_window_close(owner)?;
            context.destack_display_window_close(window)?;
        }

        Ok(())
    });
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[cfg_attr(test, test)]
pub(crate) fn test_display_backend_identity_tracks_strict_backend_selection() {
    if run_execution_case_or_return(display_case_name!(
        test_display_backend_identity_tracks_strict_backend_selection
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let backends = available_backends(&mut context)?;
        if backends.is_empty() {
            return Ok(());
        }

        for backend in backends {
            // verify monitor descriptor backend identity for strict backend selection
            let mut list_request = default_monitor_list_request(&context);
            force_monitor_list_backend(&mut list_request, backend);
            let Some(monitor_list) =
                result_or_skip_not_supported(context.destack_display_monitor_list(list_request))?
            else {
                continue;
            };
            match monitor_list {
                HarnessValue::Native(values) => {
                    let values = unsafe { values.as_slice()? };
                    if let Some(value) = values.first() {
                        assert_eq!(value.backend, backend);
                    }
                }
                HarnessValue::Vm(values) => {
                    let Some(vm_context) = context.vm_context else {
                        return Err(RuntimeError::from(PlatformError::invalid_argument(
                            "missing vm context",
                        ))
                        .boxed());
                    };
                    let vm_context =
                        unsafe { &mut *(vm_context as *mut destack_vm::BindingContext<'_>) };
                    let values = values.read_values(&vm_context.read())?;
                    if let Some(value) = values.first() {
                        assert_eq!(value.backend, backend);
                    }
                }
            }

            // verify window descriptor and state backend identity
            let title = format!("identity-{backend:?}");
            let mut options = default_window_options(&mut context, &title)?;
            force_window_backend(&mut options, backend);
            let Some(window) =
                result_or_skip_not_supported(context.destack_display_window_open(options))?
            else {
                continue;
            };

            let descriptor = context.destack_display_window_descriptor(window)?;
            match descriptor {
                HarnessValue::Native(value) => assert_eq!(value.backend, backend),
                HarnessValue::Vm(value) => assert_eq!(value.backend, backend),
            }

            let state = context.destack_display_window_state(window)?;
            let state = decode_harness_value(state);
            assert_eq!(state.backend, backend);

            // verify event metadata backend identity
            let mut event_options = window_event_open_options_with_filter(
                &context,
                256,
                display_platform::DisplayEventOverflowPolicy::DropOldest,
                Some(window),
                Some(display_platform::WINDOW_EVENT_KIND_VISIBILITY_CHANGED.0),
            );
            force_window_event_backend(&mut event_options, backend);
            let Some(stream) = result_or_skip_not_supported(
                context.destack_display_window_event_open(event_options),
            )?
            else {
                context.destack_display_window_close(window)?;
                continue;
            };

            drain_window_event_stream(&mut context, stream)?;
            context.destack_display_window_set_visibility(window, WindowVisibility::Minimized)?;

            let event = context.destack_display_window_event_read(stream, 100_000_000)?;
            let event_backend = match event {
                HarnessValue::Native(
                    display_platform::WindowEvent::WindowVisibilityChangedEvent(value),
                ) => value.metadata.backend,
                HarnessValue::Vm(
                    display_platform::WindowEventVm::WindowVisibilityChangedEvent(value),
                ) => value.metadata.backend,
                _ => {
                    context.destack_display_window_event_close(stream)?;
                    context.destack_display_window_close(window)?;
                    return Err(RuntimeError::from(PlatformError::invalid_argument(
                        "unexpected window event kind for backend identity test",
                    ))
                    .boxed());
                }
            };
            assert_eq!(event_backend, backend);

            context.destack_display_window_event_close(stream)?;
            context.destack_display_window_close(window)?;
        }

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_display_window_capabilities_match_opened_window_backend() {
    if run_execution_case_or_return(display_case_name!(
        test_display_window_capabilities_match_opened_window_backend
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let descriptors = backend_descriptors_for_host_execution(&mut context)?;
        if descriptors.is_empty() {
            return Ok(());
        }

        for descriptor in descriptors {
            let backend = descriptor.backend;

            // open one window for this explicit backend
            let title = format!("window-capabilities-{backend:?}");
            let mut options = default_window_options(&mut context, &title)?;
            force_window_backend(&mut options, backend);
            let Some(window) =
                result_or_skip_not_supported(context.destack_display_window_open(options))?
            else {
                continue;
            };

            // assert that per-window capabilities are one truthful subset of backend descriptor lanes
            let capability_flags = context.destack_display_window_capabilities(window)?;
            assert_eq!(capability_flags.0 & !descriptor.capability_flags, 0);

            // non-wayland backends currently expose backend-equal per-window capability masks
            if backend != DisplayBackend::Wayland {
                assert_eq!(capability_flags.0, descriptor.capability_flags);
            }

            context.destack_display_window_close(window)?;
        }

        Ok(())
    });
}

#[cfg(target_os = "linux")]
#[cfg_attr(test, test)]
pub(crate) fn test_display_x11_capabilities_match_implemented_contract() {
    if run_execution_case_or_return(display_case_name!(
        test_display_x11_capabilities_match_implemented_contract
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let descriptors = context.destack_display_backend_list()?;
        let (available, capability_flags) = match descriptors {
            HarnessValue::Native(values) => unsafe { values.as_slice()? }
                .iter()
                .find(|descriptor| descriptor.backend == DisplayBackend::X11)
                .map(|descriptor| {
                    (
                        support_allows_host_execution(descriptor.support),
                        descriptor.capability_flags.0,
                    )
                })
                .expect("backend list should contain x11 descriptor"),
            HarnessValue::Vm(values) => {
                let Some(vm_context) = context.vm_context else {
                    return Err(RuntimeError::from(PlatformError::invalid_argument(
                        "missing vm context",
                    ))
                    .boxed());
                };
                let vm_context =
                    unsafe { &mut *(vm_context as *mut destack_vm::BindingContext<'_>) };
                values
                    .read_values(&vm_context.read())?
                    .into_iter()
                    .find(|descriptor| descriptor.backend == DisplayBackend::X11)
                    .map(|descriptor| {
                        (
                            support_allows_host_execution(descriptor.support),
                            descriptor.capability_flags.0,
                        )
                    })
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

        let cursor_icon_result =
            context.destack_display_window_set_cursor_icon(window, WindowCursorIcon::Pointer);
        match cursor_icon_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_ICON, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_CURSOR_ICON, 0);
            }
        }

        let cursor_lock_result =
            context.destack_display_window_set_cursor_mode(window, WindowCursorMode::Locked);
        match cursor_lock_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_LOCK, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_CURSOR_LOCK, 0);
            }
        }

        let cursor_confine_result =
            context.destack_display_window_set_cursor_mode(window, WindowCursorMode::Confined);
        match cursor_confine_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_CONFINE, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_CURSOR_CONFINE, 0);
            }
        }

        context.destack_display_window_set_cursor_mode(window, WindowCursorMode::Normal)?;

        let chrome_result =
            context.destack_display_window_set_chrome(window, WindowChromeKind::Popup);
        match chrome_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_CHROME, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_CHROME, 0);
            }
        }

        let state = decode_harness_value(context.destack_display_window_state(window)?);
        assert_eq!(state.occlusion, WindowOcclusionState::Unknown);

        let mut owner_options = default_window_options(&mut context, "x11-capability-owner")?;
        force_window_backend(&mut owner_options, DisplayBackend::X11);
        let owner = context.destack_display_window_open(owner_options)?;
        context.destack_display_window_set_parent(window, Some(owner))?;

        let mut popup_options = default_window_options(&mut context, "x11-capability-popup")?;
        force_window_backend(&mut popup_options, DisplayBackend::X11);
        match &mut popup_options {
            HarnessValue::Native(options) => {
                options.role = WindowRole::Popup;
                options.parent = Some(owner);
            }
            HarnessValue::Vm(options) => {
                options.role = WindowRole::Popup;
                options.parent = Some(owner);
            }
        }

        let popup_result = context.destack_display_window_open(popup_options);
        match popup_result {
            Ok(popup) => {
                assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_ROLE_POPUP, 0);

                let popup_state =
                    decode_harness_value(context.destack_display_window_state(popup)?);
                assert_eq!(popup_state.role, WindowRole::Popup);
                assert!(!popup_state.taskbar_visible);
                assert!(!popup_state.always_on_top);

                context.destack_display_window_close(popup)?;
            }
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    context.destack_display_window_set_parent(window, None)?;
                    context.destack_display_window_close(owner)?;
                    context.destack_display_window_close(window)?;
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_ROLE_POPUP, 0);
            }
        }

        let mut overlay_options = default_window_options(&mut context, "x11-capability-overlay")?;
        force_window_backend(&mut overlay_options, DisplayBackend::X11);
        match &mut overlay_options {
            HarnessValue::Native(options) => {
                options.role = WindowRole::Overlay;
            }
            HarnessValue::Vm(options) => {
                options.role = WindowRole::Overlay;
            }
        }

        let overlay_result = context.destack_display_window_open(overlay_options);
        match overlay_result {
            Ok(overlay) => {
                assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_ROLE_OVERLAY, 0);

                let overlay_state =
                    decode_harness_value(context.destack_display_window_state(overlay)?);
                assert_eq!(overlay_state.role, WindowRole::Overlay);
                assert!(!overlay_state.taskbar_visible);
                assert!(overlay_state.always_on_top);

                context.destack_display_window_close(overlay)?;
            }
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    context.destack_display_window_set_parent(window, None)?;
                    context.destack_display_window_close(owner)?;
                    context.destack_display_window_close(window)?;
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_ROLE_OVERLAY, 0);
            }
        }

        context.destack_display_window_set_parent(window, None)?;
        context.destack_display_window_close(owner)?;
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

#[cfg(target_os = "linux")]
#[cfg_attr(test, test)]
pub(crate) fn test_display_wayland_capabilities_match_implemented_contract() {
    if run_execution_case_or_return(display_case_name!(
        test_display_wayland_capabilities_match_implemented_contract
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let descriptors = context.destack_display_backend_list()?;
        let (available, capability_flags) = match descriptors {
            HarnessValue::Native(values) => unsafe { values.as_slice()? }
                .iter()
                .find(|descriptor| descriptor.backend == DisplayBackend::Wayland)
                .map(|descriptor| {
                    (
                        support_allows_host_execution(descriptor.support),
                        descriptor.capability_flags.0,
                    )
                })
                .expect("backend list should contain wayland descriptor"),
            HarnessValue::Vm(values) => {
                let Some(vm_context) = context.vm_context else {
                    return Err(RuntimeError::from(PlatformError::invalid_argument(
                        "missing vm context",
                    ))
                    .boxed());
                };
                let vm_context =
                    unsafe { &mut *(vm_context as *mut destack_vm::BindingContext<'_>) };
                values
                    .read_values(&vm_context.read())?
                    .into_iter()
                    .find(|descriptor| descriptor.backend == DisplayBackend::Wayland)
                    .map(|descriptor| {
                        (
                            support_allows_host_execution(descriptor.support),
                            descriptor.capability_flags.0,
                        )
                    })
                    .expect("backend list should contain wayland descriptor")
            }
        };

        if !available {
            return Ok(());
        }

        assert_eq!(capability_flags & DISPLAY_CAP_EXCLUSIVE_FULLSCREEN, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_ASPECT_RATIO, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_ALWAYS_ON_TOP, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_TASKBAR_VISIBILITY, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_THEME, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_SAFE_AREA, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_OCCLUSION, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_MONITOR_HDR_CONTROL, 0);

        let mut window_options = default_window_options(&mut context, "wayland-capability-check")?;
        force_window_backend(&mut window_options, DisplayBackend::Wayland);
        let Some(window) =
            result_or_skip_not_supported(context.destack_display_window_open(window_options))?
        else {
            return Ok(());
        };

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

        let attention_result = context
            .destack_display_window_request_attention(window, WindowAttentionLevel::Informational);
        match attention_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_ATTENTION_REQUEST, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_ATTENTION_REQUEST, 0);
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

        let cursor_icon_result =
            context.destack_display_window_set_cursor_icon(window, WindowCursorIcon::Pointer);
        match cursor_icon_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_ICON, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_CURSOR_ICON, 0);
            }
        }

        let cursor_lock_result =
            context.destack_display_window_set_cursor_mode(window, WindowCursorMode::Locked);
        match cursor_lock_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_LOCK, 0),
            Err(error) => {
                let code = error_code(&error);
                if is_not_supported_code(code) {
                    assert_eq!(capability_flags & DISPLAY_CAP_CURSOR_LOCK, 0);
                } else if code == Some(PlatformErrorCode::IoWouldBlock) {
                    assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_LOCK, 0);
                } else {
                    return Err(error);
                }
            }
        }

        let cursor_confine_result =
            context.destack_display_window_set_cursor_mode(window, WindowCursorMode::Confined);
        match cursor_confine_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_CONFINE, 0),
            Err(error) => {
                let code = error_code(&error);
                if is_not_supported_code(code) {
                    assert_eq!(capability_flags & DISPLAY_CAP_CURSOR_CONFINE, 0);
                } else if code == Some(PlatformErrorCode::IoWouldBlock) {
                    assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_CONFINE, 0);
                } else {
                    return Err(error);
                }
            }
        }

        let _ = context.destack_display_window_set_cursor_mode(window, WindowCursorMode::Normal);

        let cursor_warp_result = context.destack_display_window_set_cursor_position(
            window,
            harness_window_position(&context, 10, 10),
        );
        match cursor_warp_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_WARP, 0),
            Err(error) => {
                let code = error_code(&error);
                if is_not_supported_code(code) {
                    assert_eq!(capability_flags & DISPLAY_CAP_CURSOR_WARP, 0);
                } else if code == Some(PlatformErrorCode::IoWouldBlock) {
                    assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_WARP, 0);
                } else {
                    return Err(error);
                }
            }
        }

        let begin_move_drag_result = context.destack_display_window_begin_move_drag(window);
        match begin_move_drag_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_DRAG_INTERACTION, 0),
            Err(error) => {
                let code = error_code(&error);
                if is_not_supported_code(code) {
                    assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_DRAG_INTERACTION, 0);
                } else if code == Some(PlatformErrorCode::IoWouldBlock) {
                    assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_DRAG_INTERACTION, 0);
                } else {
                    return Err(error);
                }
            }
        }

        let chrome_result =
            context.destack_display_window_set_chrome(window, WindowChromeKind::Popup);
        match chrome_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_CHROME, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_CHROME, 0);
            }
        }

        let opacity_result = context.destack_display_window_set_opacity(window, 0.75);
        match opacity_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_OPACITY, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_OPACITY, 0);
            }
        }

        let icon_set = harness_window_icon_set(&mut context)?;
        let icon_set_result = context.destack_display_window_set_icons(window, icon_set);
        match icon_set_result {
            Ok(()) => {
                assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_ICON, 0);
                context.destack_display_window_set_icons(
                    window,
                    harness_window_icon_set_none(&context),
                )?;
            }
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_ICON, 0);
            }
        }

        let aspect_ratio = harness_aspect_ratio(&context, 16, 9);
        let aspect_ratio_result =
            context.destack_display_window_set_aspect_ratio(window, aspect_ratio);
        match aspect_ratio_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_ASPECT_RATIO, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_ASPECT_RATIO, 0);
            }
        }

        let always_on_top_result = context.destack_display_window_set_always_on_top(window, true);
        match always_on_top_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_ALWAYS_ON_TOP, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_ALWAYS_ON_TOP, 0);
            }
        }

        let taskbar_result = context.destack_display_window_set_taskbar_visible(window, false);
        match taskbar_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_TASKBAR_VISIBILITY, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_TASKBAR_VISIBILITY, 0);
            }
        }

        let mut owner_options = default_window_options(&mut context, "wayland-capability-owner")?;
        force_window_backend(&mut owner_options, DisplayBackend::Wayland);
        let owner = context.destack_display_window_open(owner_options)?;
        context.destack_display_window_set_parent(window, Some(owner))?;

        let mut popup_options = default_window_options(&mut context, "wayland-capability-popup")?;
        force_window_backend(&mut popup_options, DisplayBackend::Wayland);
        match &mut popup_options {
            HarnessValue::Native(options) => {
                options.role = WindowRole::Popup;
                options.parent = Some(owner);
            }
            HarnessValue::Vm(options) => {
                options.role = WindowRole::Popup;
                options.parent = Some(owner);
            }
        }

        let popup_result = context.destack_display_window_open(popup_options);
        match popup_result {
            Ok(popup) => {
                assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_ROLE_POPUP, 0);

                // popup effective capabilities should drop toplevel-only lanes
                let popup_capability_flags = context.destack_display_window_capabilities(popup)?;
                assert_eq!(
                    popup_capability_flags.0 & DISPLAY_CAP_BORDERLESS_FULLSCREEN,
                    0
                );
                assert_eq!(
                    popup_capability_flags.0 & DISPLAY_CAP_WINDOW_DRAG_INTERACTION,
                    0
                );
                assert_eq!(popup_capability_flags.0 & DISPLAY_CAP_WINDOW_PARENTING, 0);
                assert_eq!(popup_capability_flags.0 & DISPLAY_CAP_WINDOW_MODAL, 0);
                assert_eq!(popup_capability_flags.0 & DISPLAY_CAP_WINDOW_CHROME, 0);
                assert_eq!(popup_capability_flags.0 & DISPLAY_CAP_WINDOW_ICON, 0);
                assert_eq!(popup_capability_flags.0 & DISPLAY_CAP_ALWAYS_ON_TOP, 0);
                assert_eq!(
                    popup_capability_flags.0 & DISPLAY_CAP_WINDOW_TASKBAR_VISIBILITY,
                    0
                );
                assert_eq!(popup_capability_flags.0 & DISPLAY_CAP_WINDOW_ROLE_POPUP, 0);
                assert_eq!(
                    popup_capability_flags.0 & DISPLAY_CAP_WINDOW_ROLE_OVERLAY,
                    0
                );

                context.destack_display_window_close(popup)?;
            }
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    context.destack_display_window_close(owner)?;
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_ROLE_POPUP, 0);
            }
        }

        let mut overlay_options =
            default_window_options(&mut context, "wayland-capability-overlay")?;
        force_window_backend(&mut overlay_options, DisplayBackend::Wayland);
        match &mut overlay_options {
            HarnessValue::Native(options) => {
                options.role = WindowRole::Overlay;
            }
            HarnessValue::Vm(options) => {
                options.role = WindowRole::Overlay;
            }
        }

        let overlay_result = context.destack_display_window_open(overlay_options);
        match overlay_result {
            Ok(overlay) => {
                assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_ROLE_OVERLAY, 0);

                // overlay effective capabilities should drop toplevel-only lanes
                let overlay_capability_flags =
                    context.destack_display_window_capabilities(overlay)?;
                assert_eq!(
                    overlay_capability_flags.0 & DISPLAY_CAP_BORDERLESS_FULLSCREEN,
                    0
                );
                assert_eq!(
                    overlay_capability_flags.0 & DISPLAY_CAP_WINDOW_DRAG_INTERACTION,
                    0
                );
                assert_eq!(overlay_capability_flags.0 & DISPLAY_CAP_WINDOW_PARENTING, 0);
                assert_eq!(overlay_capability_flags.0 & DISPLAY_CAP_WINDOW_MODAL, 0);
                assert_eq!(overlay_capability_flags.0 & DISPLAY_CAP_WINDOW_CHROME, 0);
                assert_eq!(overlay_capability_flags.0 & DISPLAY_CAP_WINDOW_ICON, 0);
                assert_eq!(overlay_capability_flags.0 & DISPLAY_CAP_ALWAYS_ON_TOP, 0);
                assert_eq!(
                    overlay_capability_flags.0 & DISPLAY_CAP_WINDOW_TASKBAR_VISIBILITY,
                    0
                );
                assert_eq!(
                    overlay_capability_flags.0 & DISPLAY_CAP_WINDOW_ROLE_POPUP,
                    0
                );
                assert_eq!(
                    overlay_capability_flags.0 & DISPLAY_CAP_WINDOW_ROLE_OVERLAY,
                    0
                );

                context.destack_display_window_close(overlay)?;
            }
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    context.destack_display_window_close(owner)?;
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_ROLE_OVERLAY, 0);
            }
        }

        let modal_result = context.destack_display_window_set_modal(window, true);
        match modal_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_MODAL, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    context.destack_display_window_close(owner)?;
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_MODAL, 0);
            }
        }
        let _ = context.destack_display_window_set_modal(window, false);
        context.destack_display_window_set_parent(window, None)?;
        context.destack_display_window_close(owner)?;

        let exclusive_mode = harness_window_mode_options(
            &context,
            HarnessWindowMode::ExclusiveFullscreen {
                display: resource::DisplayHandle(resource::ResourceId::local(0)),
                display_mode: None,
            },
        );
        let exclusive_result = context.destack_display_window_set_mode(window, exclusive_mode);
        match exclusive_result {
            Ok(()) => assert_ne!(capability_flags & DISPLAY_CAP_EXCLUSIVE_FULLSCREEN, 0),
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_EXCLUSIVE_FULLSCREEN, 0);
            }
        }

        let state = decode_harness_value(context.destack_display_window_state(window)?);
        assert_eq!(state.occlusion, WindowOcclusionState::Unknown);

        context.destack_display_window_close(window)?;

        let mut monitor_request = default_monitor_list_request(&context);
        force_monitor_list_backend(&mut monitor_request, DisplayBackend::Wayland);
        let monitor_list = context.destack_display_monitor_list(monitor_request)?;
        let monitor_list = decode_monitor_list(&mut context, monitor_list)?;
        assert!(!monitor_list.is_empty());
        let display_id = harness_string(&mut context, &monitor_list[0].0)?;

        let mut monitor_options = default_monitor_open_options(&context);
        force_monitor_open_backend(&mut monitor_options, DisplayBackend::Wayland);
        let display = context.destack_display_monitor_open(display_id, monitor_options)?;

        let color_state_result = context.destack_display_monitor_color_state(display);
        let mut color_state_hdr_mode = None;
        match color_state_result {
            Ok(color_state) => {
                assert_ne!(capability_flags & DISPLAY_CAP_MONITOR_COLOR_STATE, 0);
                color_state_hdr_mode = Some(match color_state {
                    HarnessValue::Native(value) => value.hdr_mode,
                    HarnessValue::Vm(value) => value.hdr_mode,
                });
            }
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    context.destack_display_monitor_close(display)?;
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_MONITOR_COLOR_STATE, 0);
            }
        }

        let hdr_mode_result = context.destack_display_monitor_hdr_mode(display);
        match hdr_mode_result {
            Ok(hdr_mode) => {
                let has_hdr_lane = capability_flags & DISPLAY_CAP_MONITOR_HDR_CONTROL != 0;
                let has_color_lane = capability_flags & DISPLAY_CAP_MONITOR_COLOR_STATE != 0;
                assert!(has_hdr_lane || has_color_lane);

                if let Some(color_state_hdr_mode) = color_state_hdr_mode {
                    assert_eq!(hdr_mode, color_state_hdr_mode);
                }
            }
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    context.destack_display_monitor_close(display)?;
                    return Err(error);
                }
                assert_eq!(capability_flags & DISPLAY_CAP_MONITOR_HDR_CONTROL, 0);
                assert_eq!(capability_flags & DISPLAY_CAP_MONITOR_COLOR_STATE, 0);
            }
        }

        let gamma_result = context.destack_display_monitor_gamma_ramp(display);
        match gamma_result {
            Ok(ramp) => {
                assert_ne!(capability_flags & DISPLAY_CAP_MONITOR_GAMMA_CONTROL, 0);
                context.destack_display_monitor_set_gamma_ramp(display, ramp)?;
            }
            Err(error) => {
                if !is_not_supported_code(error_code(&error)) {
                    context.destack_display_monitor_close(display)?;
                    return Err(error);
                }

                // wayland gamma-control protocol does not provide generic readback on all compositors
                // so gamma-ramp reads may be notSupported even when write control exists
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

#[cfg(target_os = "linux")]
#[cfg_attr(test, test)]
pub(crate) fn test_display_linux_backend_capabilities_respect_ceiling_inventory() {
    if run_execution_case_or_return(display_case_name!(
        test_display_linux_backend_capabilities_respect_ceiling_inventory
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let descriptors = backend_descriptors_for_host_execution(&mut context)?;

        // verify every available unix backend reports one subset of its capability ceiling
        for descriptor in descriptors {
            let ceiling_mask = match descriptor.backend {
                DisplayBackend::X11 => x11_capability_ceiling_mask(),
                DisplayBackend::Wayland => wayland_capability_ceiling_mask(),
                _ => {
                    continue;
                }
            };

            let unsupported_bits = descriptor.capability_flags & !ceiling_mask;
            assert_eq!(unsupported_bits, 0);
        }

        Ok(())
    });
}

#[cfg(target_os = "linux")]
#[cfg_attr(test, test)]
pub(crate) fn test_display_wayland_window_event_filter_accepts_scale_factor_kind() {
    if run_execution_case_or_return(display_case_name!(
        test_display_wayland_window_event_filter_accepts_scale_factor_kind
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let descriptors = backend_descriptors_for_host_execution(&mut context)?;
        let is_wayland_available = descriptors
            .iter()
            .any(|descriptor| descriptor.backend == DisplayBackend::Wayland);
        if !is_wayland_available {
            return Ok(());
        }

        let mut options = window_event_open_options_with_filter(
            &context,
            8,
            display_platform::DisplayEventOverflowPolicy::DropOldest,
            None,
            Some(display_platform::WINDOW_EVENT_KIND_SCALE_FACTOR_CHANGED.0),
        );
        force_window_event_backend(&mut options, DisplayBackend::Wayland);
        let stream = context.destack_display_window_event_open(options)?;
        context.destack_display_window_event_close(stream)?;

        Ok(())
    });
}

#[cfg(target_os = "windows")]
#[cfg_attr(test, test)]
pub(crate) fn test_display_win32_visible_state_reports_unknown_occlusion() {
    if run_execution_case_or_return(display_case_name!(
        test_display_win32_visible_state_reports_unknown_occlusion
    )) {
        return;
    }

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

#[cfg(target_os = "windows")]
#[cfg_attr(test, test)]
pub(crate) fn test_display_win32_capabilities_match_implemented_contract() {
    if run_execution_case_or_return(display_case_name!(
        test_display_win32_capabilities_match_implemented_contract
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let descriptors = context.destack_display_backend_list()?;
        let (available, capability_flags) = match descriptors {
            HarnessValue::Native(values) => unsafe { values.as_slice()? }
                .iter()
                .find(|descriptor| descriptor.backend == DisplayBackend::Win32)
                .map(|descriptor| {
                    (
                        support_allows_host_execution(descriptor.support),
                        descriptor.capability_flags.0,
                    )
                })
                .expect("backend list should contain win32 descriptor"),
            HarnessValue::Vm(values) => {
                let Some(vm_context) = context.vm_context else {
                    return Err(RuntimeError::from(PlatformError::invalid_argument(
                        "missing vm context",
                    ))
                    .boxed());
                };
                let vm_context =
                    unsafe { &mut *(vm_context as *mut destack_vm::BindingContext<'_>) };
                values
                    .read_values(&vm_context.read())?
                    .into_iter()
                    .find(|descriptor| descriptor.backend == DisplayBackend::Win32)
                    .map(|descriptor| {
                        (
                            support_allows_host_execution(descriptor.support),
                            descriptor.capability_flags.0,
                        )
                    })
                    .expect("backend list should contain win32 descriptor")
            }
        };

        if !available {
            return Ok(());
        }

        assert_ne!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0,
            0
        );
        assert_ne!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_WINDOW_ASPECT_RATIO.0,
            0
        );
        assert_ne!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_WINDOW_CHROME.0,
            0
        );
        assert_ne!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_ATTENTION_REQUEST.0,
            0
        );
        assert_ne!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_WINDOW_DROP_EVENTS.0,
            0
        );
        assert_ne!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_POPUP.0,
            0
        );
        assert_ne!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_OVERLAY.0,
            0
        );
        assert_ne!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0,
            0
        );
        assert_ne!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_MONITOR_COLOR_STATE.0,
            0
        );
        assert_ne!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_MONITOR_HDR_CONTROL.0,
            0
        );
        assert_ne!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0,
            0
        );
        assert_eq!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_OCCLUSION.0,
            0
        );
        assert_eq!(
            capability_flags & display_platform::DISPLAY_BACKEND_CAP_SAFE_AREA.0,
            0
        );

        let mut window_options = default_window_options(&mut context, "win32-capability-check")?;
        force_window_backend(&mut window_options, DisplayBackend::Win32);
        let Some(window) =
            result_or_skip_not_supported(context.destack_display_window_open(window_options))?
        else {
            return Ok(());
        };

        let state = decode_harness_value(context.destack_display_window_state(window)?);
        assert_eq!(state.occlusion, WindowOcclusionState::Unknown);

        let mut owner_options = default_window_options(&mut context, "win32-capability-owner")?;
        force_window_backend(&mut owner_options, DisplayBackend::Win32);
        let owner = context.destack_display_window_open(owner_options)?;
        context.destack_display_window_set_parent(window, Some(owner))?;

        let mut popup_options = default_window_options(&mut context, "win32-capability-popup")?;
        force_window_backend(&mut popup_options, DisplayBackend::Win32);
        match &mut popup_options {
            HarnessValue::Native(options) => {
                options.role = WindowRole::Popup;
                options.parent = Some(owner);
            }
            HarnessValue::Vm(options) => {
                options.role = WindowRole::Popup;
                options.parent = Some(owner);
            }
        }

        let popup = context.destack_display_window_open(popup_options)?;
        context.destack_display_window_close(popup)?;

        let mut overlay_options = default_window_options(&mut context, "win32-capability-overlay")?;
        force_window_backend(&mut overlay_options, DisplayBackend::Win32);
        match &mut overlay_options {
            HarnessValue::Native(options) => {
                options.role = WindowRole::Overlay;
            }
            HarnessValue::Vm(options) => {
                options.role = WindowRole::Overlay;
            }
        }

        let overlay = context.destack_display_window_open(overlay_options)?;
        context.destack_display_window_close(overlay)?;

        context.destack_display_window_set_parent(window, None)?;
        context.destack_display_window_close(owner)?;
        context.destack_display_window_close(window)?;

        Ok(())
    });
}

#[cfg(target_os = "macos")]
#[cfg_attr(test, test)]
pub(crate) fn test_display_appkit_capabilities_match_implemented_contract() {
    if run_execution_case_or_return(display_case_name!(
        test_display_appkit_capabilities_match_implemented_contract
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let descriptors = context.destack_display_backend_list()?;
        let (available, capability_flags) = match descriptors {
            HarnessValue::Native(values) => unsafe { values.as_slice()? }
                .iter()
                .find(|descriptor| descriptor.backend == DisplayBackend::AppKit)
                .map(|descriptor| {
                    (
                        support_allows_host_execution(descriptor.support),
                        descriptor.capability_flags.0,
                    )
                })
                .expect("backend list should contain appkit descriptor"),
            HarnessValue::Vm(values) => {
                let Some(vm_context) = context.vm_context else {
                    return Err(RuntimeError::from(PlatformError::invalid_argument(
                        "missing vm context",
                    ))
                    .boxed());
                };
                let vm_context =
                    unsafe { &mut *(vm_context as *mut destack_vm::BindingContext<'_>) };
                values
                    .read_values(&vm_context.read())?
                    .into_iter()
                    .find(|descriptor| descriptor.backend == DisplayBackend::AppKit)
                    .map(|descriptor| {
                        (
                            support_allows_host_execution(descriptor.support),
                            descriptor.capability_flags.0,
                        )
                    })
                    .expect("backend list should contain appkit descriptor")
            }
        };

        assert!(available);
        assert_eq!(capability_flags & !appkit_capability_ceiling_mask(), 0);
        assert_ne!(capability_flags & DISPLAY_CAP_MONITOR_MODE_SET, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_MONITOR_COLOR_STATE, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_MONITOR_HDR_CONTROL, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_MONITOR_GAMMA_CONTROL, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_EXCLUSIVE_FULLSCREEN, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_BORDERLESS_FULLSCREEN, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_OCCLUSION, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_SAFE_AREA, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_THEME, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_HIT_TEST, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_ALWAYS_ON_TOP, 0);
        assert_eq!(capability_flags & DISPLAY_CAP_WINDOW_TASKBAR_VISIBILITY, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_ROLE_POPUP, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_ROLE_OVERLAY, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_MODAL, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_ASPECT_RATIO, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_CHROME, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_ATTENTION_REQUEST, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_DROP_EVENTS, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_VISIBILITY, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_ICON, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_CURSOR_WARP, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_OPACITY, 0);
        assert_ne!(capability_flags & DISPLAY_CAP_WINDOW_ICON, 0);

        Ok(())
    });
}
