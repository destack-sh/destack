#[cfg(target_os = "macos")]
use super::appkit;
#[cfg(target_os = "linux")]
use super::wayland;
#[cfg(target_os = "linux")]
use super::x11;
use crate::diagnostic::RuntimeResult;
use crate::platform::core::{BackendSupport, aggregate_backend_support, backend_support_error};
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayBackendDescriptor,
    DisplayBackendSelectionPolicy,
};
use crate::runtime::BindingCallContext;

#[cfg(target_os = "linux")]
const UNIX_BACKEND_PRIORITY: &[DisplayBackend] = &[DisplayBackend::Wayland, DisplayBackend::X11];
#[cfg(target_os = "macos")]
const UNIX_BACKEND_PRIORITY: &[DisplayBackend] = &[DisplayBackend::AppKit];
#[cfg(target_os = "android")]
const UNIX_BACKEND_PRIORITY: &[DisplayBackend] = &[DisplayBackend::Android];
#[cfg(target_os = "ios")]
const UNIX_BACKEND_PRIORITY: &[DisplayBackend] = &[DisplayBackend::UIKit];
#[cfg(all(
    unix,
    not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "android",
        target_os = "ios"
    ))
))]
const UNIX_BACKEND_PRIORITY: &[DisplayBackend] = &[];

const DISPLAY_BACKEND_SELECTORS: &[DisplayBackend] = &[
    DisplayBackend::Auto,
    DisplayBackend::Wayland,
    DisplayBackend::X11,
    DisplayBackend::Win32,
    DisplayBackend::AppKit,
    DisplayBackend::UIKit,
    DisplayBackend::Android,
    DisplayBackend::Null,
];

/// Descriptor support and capabilities for one host backend row.
#[derive(Clone, Copy)]
struct DisplayBackendDescriptorState {
    /// The host integration support state.
    support: BackendSupport,
    /// The advertised capability mask for this backend row.
    capability_flags: DisplayBackendCapabilityFlags,
}

/// Return unix display backend priority order for auto-selection.
pub(crate) fn preferred_host_backends() -> &'static [DisplayBackend] {
    UNIX_BACKEND_PRIORITY
}

/// Return one backend name for diagnostics and descriptors.
pub(crate) fn backend_name(backend: DisplayBackend) -> &'static str {
    match backend {
        DisplayBackend::Wayland => "wayland",
        DisplayBackend::X11 => "x11",
        DisplayBackend::AppKit => "appkit",
        DisplayBackend::UIKit => "uikit",
        DisplayBackend::Android => "android",
        DisplayBackend::Win32 => "win32",
        DisplayBackend::Null => "null",
        DisplayBackend::Auto => "auto",
    }
}

/// Return combined support for the default host backend lane.
fn auto_backend_support() -> BackendSupport {
    aggregate_backend_support(
        preferred_host_backends()
            .iter()
            .copied()
            .map(backend_support),
    )
}

/// Return the current active host backend for auto selection.
fn active_host_backend() -> Option<DisplayBackend> {
    preferred_host_backends()
        .iter()
        .copied()
        .find(|backend| backend_support(*backend).is_available())
}

/// Return one auto-selection priority for one descriptor row.
fn backend_priority(backend: DisplayBackend) -> u16 {
    if backend == DisplayBackend::Auto {
        return u16::MAX;
    }

    preferred_host_backends()
        .iter()
        .position(|candidate| *candidate == backend)
        .map(|index| u16::MAX.saturating_sub(index as u16 + 1))
        .unwrap_or(0)
}

/// Return descriptor support and capability flags for one unix host backend.
fn backend_descriptor_state(
    binding: &BindingCallContext,
    backend: DisplayBackend,
) -> DisplayBackendDescriptorState {
    #[cfg(target_os = "linux")]
    if backend == DisplayBackend::X11 {
        let (support, capability_flags) = x11::backend_descriptor_state(binding);

        return DisplayBackendDescriptorState {
            support,
            capability_flags,
        };
    }

    #[cfg(target_os = "linux")]
    if backend == DisplayBackend::Wayland {
        let (support, capability_flags) = wayland::backend_descriptor_state(binding);

        return DisplayBackendDescriptorState {
            support,
            capability_flags,
        };
    }

    #[cfg(target_os = "macos")]
    if backend == DisplayBackend::AppKit {
        let (support, capability_flags) = appkit::backend_descriptor_state(binding);

        return DisplayBackendDescriptorState {
            support,
            capability_flags,
        };
    }

    DisplayBackendDescriptorState {
        support: backend_support(backend),
        capability_flags: backend_capabilities(binding, backend),
    }
}

/// Return cached descriptor state rows for host backends.
fn host_backend_descriptor_states(
    binding: &BindingCallContext,
) -> Vec<(DisplayBackend, DisplayBackendDescriptorState)> {
    preferred_host_backends()
        .iter()
        .copied()
        .map(|backend| (backend, backend_descriptor_state(binding, backend)))
        .collect()
}

/// Return one selector descriptor state from cached host backend rows.
fn selector_descriptor_state(
    backend: DisplayBackend,
    active_backend: Option<DisplayBackend>,
    auto_support: BackendSupport,
    host_descriptor_states: &[(DisplayBackend, DisplayBackendDescriptorState)],
) -> DisplayBackendDescriptorState {
    // auto mirrors the active host backend capability row
    if backend == DisplayBackend::Auto {
        let capability_flags = active_backend
            .and_then(|active_backend| {
                host_descriptor_states
                    .iter()
                    .find(|(backend, _)| *backend == active_backend)
                    .map(|(_, state)| state.capability_flags)
            })
            .unwrap_or(DisplayBackendCapabilityFlags(0));

        return DisplayBackendDescriptorState {
            support: auto_support,
            capability_flags,
        };
    }

    // host rows reuse the cached backend descriptor state
    if let Some((_, state)) = host_descriptor_states
        .iter()
        .find(|(candidate, _)| *candidate == backend)
    {
        return *state;
    }

    // non-host selectors expose support only and no host capabilities
    DisplayBackendDescriptorState {
        support: backend_support(backend),
        capability_flags: DisplayBackendCapabilityFlags(0),
    }
}

/// Return host backend support for one unix display backend.
pub(crate) fn backend_support(backend: DisplayBackend) -> BackendSupport {
    // auto reflects the combined preferred host lane
    if backend == DisplayBackend::Auto {
        return auto_backend_support();
    }

    // null is not a unix display host backend
    if backend == DisplayBackend::Null {
        return BackendSupport::UnsupportedTarget;
    }

    #[cfg(target_os = "linux")]
    if backend == DisplayBackend::X11 {
        // x11 depends on one configured display endpoint
        if std::env::var_os("DISPLAY").is_some() {
            return BackendSupport::Available;
        }

        return BackendSupport::HostUnavailable;
    }

    #[cfg(target_os = "linux")]
    if backend == DisplayBackend::Wayland {
        // wayland depends on one configured compositor endpoint
        if std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var_os("WAYLAND_SOCKET").is_some()
        {
            return BackendSupport::Available;
        }

        return BackendSupport::HostUnavailable;
    }

    #[cfg(target_os = "macos")]
    if backend == DisplayBackend::AppKit {
        // appkit availability depends on the native application host
        if appkit::backend_available() {
            return BackendSupport::Available;
        }

        return BackendSupport::HostUnavailable;
    }

    #[cfg(target_os = "android")]
    if backend == DisplayBackend::Android {
        return BackendSupport::Available;
    }

    #[cfg(target_os = "ios")]
    if backend == DisplayBackend::UIKit {
        return BackendSupport::Available;
    }

    let _ = backend;
    BackendSupport::UnsupportedTarget
}

/// Return one backend capability mask for one unix backend.
pub(crate) fn backend_capabilities(
    binding: &BindingCallContext,
    backend: DisplayBackend,
) -> DisplayBackendCapabilityFlags {
    // zero capability flags for unavailable backend rows
    if !backend_support(backend).is_available() {
        return DisplayBackendCapabilityFlags(0);
    }

    #[cfg(target_os = "linux")]
    if backend == DisplayBackend::X11 {
        return x11::backend_descriptor_state(binding).1;
    }

    #[cfg(target_os = "linux")]
    if backend == DisplayBackend::Wayland {
        return wayland::backend_descriptor_state(binding).1;
    }

    #[cfg(target_os = "macos")]
    if backend == DisplayBackend::AppKit {
        return appkit::backend_descriptor_state(binding).1;
    }

    let _ = binding;
    let _ = backend;
    DisplayBackendCapabilityFlags(0)
}

/// List unix display backend descriptors.
pub(crate) fn backend_descriptors(binding: &BindingCallContext) -> Vec<DisplayBackendDescriptor> {
    // cache host descriptor rows so auto can mirror the active backend state
    let host_descriptor_states = host_backend_descriptor_states(binding);
    let active_backend = host_descriptor_states
        .iter()
        .find_map(|(backend, state)| state.support.is_available().then_some(*backend));
    let auto_support = aggregate_backend_support(
        host_descriptor_states
            .iter()
            .map(|(_, state)| state.support),
    );
    let mut descriptors = Vec::with_capacity(DISPLAY_BACKEND_SELECTORS.len());

    // emit one stable descriptor row per selector
    for backend in DISPLAY_BACKEND_SELECTORS.iter().copied() {
        let state = selector_descriptor_state(
            backend,
            active_backend,
            auto_support,
            &host_descriptor_states,
        );

        descriptors.push(DisplayBackendDescriptor {
            backend,
            name: binding.store_string(backend_name(backend)),
            support: state.support,
            priority: backend_priority(backend),
            capability_flags: state.capability_flags,
        });
    }

    descriptors
}

/// Resolve one backend selection from options for one operation.
pub(crate) fn resolve_backend(
    requested: DisplayBackend,
    selection_policy: DisplayBackendSelectionPolicy,
    operation: &'static str,
) -> RuntimeResult<DisplayBackend> {
    if requested == DisplayBackend::Auto {
        return resolve_default_backend(operation);
    }

    let support = backend_support(requested);
    if support.is_available() {
        return Ok(requested);
    }

    if selection_policy == DisplayBackendSelectionPolicy::AllowFallback {
        return resolve_auto_backend(operation);
    }

    Err(backend_support_error(
        operation,
        backend_name(requested),
        support,
    ))
}

/// Resolve one default backend for operations without an explicit selector.
pub(crate) fn resolve_default_backend(operation: &'static str) -> RuntimeResult<DisplayBackend> {
    resolve_auto_backend(operation)
}

/// Resolve one auto-selected backend for one operation.
fn resolve_auto_backend(operation: &'static str) -> RuntimeResult<DisplayBackend> {
    // return the first reachable host backend in priority order
    if let Some(backend) = active_host_backend() {
        return Ok(backend);
    }

    Err(backend_support_error(
        operation,
        backend_name(DisplayBackend::Auto),
        auto_backend_support(),
    ))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::ffi::OsString;
    use std::sync::Mutex;

    use super::{DisplayBackend, backend_support, resolve_default_backend};

    /// Serialize environment-variable mutation across tests.
    static ENVIRONMENT_LOCK: Mutex<()> = Mutex::new(());

    /// Environment snapshot for one test mutation lane.
    struct EnvironmentRestore {
        /// Previous DISPLAY value.
        display: Option<OsString>,
        /// Previous WAYLAND_DISPLAY value.
        wayland_display: Option<OsString>,
        /// Previous WAYLAND_SOCKET value.
        wayland_socket: Option<OsString>,
    }

    impl EnvironmentRestore {
        /// Capture current environment variables for later restoration.
        fn capture() -> Self {
            Self {
                display: std::env::var_os("DISPLAY"),
                wayland_display: std::env::var_os("WAYLAND_DISPLAY"),
                wayland_socket: std::env::var_os("WAYLAND_SOCKET"),
            }
        }

        /// Restore captured environment variables.
        fn restore(mut self) {
            let display = self.display.take();
            let wayland_display = self.wayland_display.take();
            let wayland_socket = self.wayland_socket.take();

            unsafe {
                match display {
                    Some(value) => std::env::set_var("DISPLAY", value),
                    None => std::env::remove_var("DISPLAY"),
                }

                match wayland_display {
                    Some(value) => std::env::set_var("WAYLAND_DISPLAY", value),
                    None => std::env::remove_var("WAYLAND_DISPLAY"),
                }

                match wayland_socket {
                    Some(value) => std::env::set_var("WAYLAND_SOCKET", value),
                    None => std::env::remove_var("WAYLAND_SOCKET"),
                }
            }
        }
    }

    impl Drop for EnvironmentRestore {
        fn drop(&mut self) {
            let display = self.display.take();
            let wayland_display = self.wayland_display.take();
            let wayland_socket = self.wayland_socket.take();
            unsafe {
                match display {
                    Some(value) => std::env::set_var("DISPLAY", value),
                    None => std::env::remove_var("DISPLAY"),
                }

                match wayland_display {
                    Some(value) => std::env::set_var("WAYLAND_DISPLAY", value),
                    None => std::env::remove_var("WAYLAND_DISPLAY"),
                }

                match wayland_socket {
                    Some(value) => std::env::set_var("WAYLAND_SOCKET", value),
                    None => std::env::remove_var("WAYLAND_SOCKET"),
                }
            }
        }
    }

    /// Resolve wayland when only wayland environment is configured.
    #[test]
    fn test_resolve_default_backend_prefers_wayland_when_display_is_missing() {
        let _lock = ENVIRONMENT_LOCK.lock().unwrap();
        let restore = EnvironmentRestore::capture();
        unsafe {
            std::env::remove_var("DISPLAY");
            std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
            std::env::remove_var("WAYLAND_SOCKET");
        }

        let backend = resolve_default_backend("destack.display.monitor.list")
            .expect("wayland environment should resolve a default backend");
        assert_eq!(backend, DisplayBackend::Wayland);

        restore.restore();
    }

    /// Prefer wayland when both wayland and x11 endpoints are configured.
    #[test]
    fn test_resolve_default_backend_prefers_wayland_when_both_endpoints_exist() {
        let _lock = ENVIRONMENT_LOCK.lock().unwrap();
        let restore = EnvironmentRestore::capture();
        unsafe {
            std::env::set_var("DISPLAY", ":0");
            std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
            std::env::remove_var("WAYLAND_SOCKET");
        }

        let backend = resolve_default_backend("destack.display.monitor.list")
            .expect("configured wayland and x11 endpoints should resolve a default backend");
        assert_eq!(backend, DisplayBackend::Wayland);

        restore.restore();
    }

    /// Report wayland support when one wayland endpoint is configured.
    #[test]
    fn test_backend_support_marks_wayland_available_when_environment_exists() {
        let _lock = ENVIRONMENT_LOCK.lock().unwrap();
        let restore = EnvironmentRestore::capture();
        unsafe {
            std::env::remove_var("DISPLAY");
            std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
            std::env::remove_var("WAYLAND_SOCKET");
        }

        assert!(backend_support(DisplayBackend::Wayland).is_available());

        restore.restore();
    }
}
