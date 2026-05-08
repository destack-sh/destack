use std::sync::{Mutex, OnceLock};

use destack_workspace::{AppPermission, RuntimeOptions};

use crate::diagnostic::RuntimeResult;
use crate::host::HostSessionId;
use crate::host::os::macos::request::location::{
    MacosLocationHooks, set_macos_location_test_hooks,
};
use crate::platform::os::tests::{
    decode_permission_entries_value, with_configured_harness_context,
};
use crate::platform::os::{Permission, PermissionState};

/// Shared macOS location-permission-test mutex that serializes the process-global hook slot.
static TEST_LOCATION_PERMISSION_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Reset the installed macOS location hooks after one permission test.
struct LocationPermissionHookGuard;

impl Drop for LocationPermissionHookGuard {
    /// Clear the active macOS location hook set.
    fn drop(&mut self) {
        set_macos_location_test_hooks(MacosLocationHooks::default());
    }
}

/// Verify macOS location permission requests route through the shared host permission lane.
#[test]
fn test_permission_request_routes_macos_location_permission_hook() {
    let _guard = TEST_LOCATION_PERMISSION_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    set_macos_location_test_hooks(MacosLocationHooks {
        request_permission: Some(test_location_permission_request_hook),
        ..MacosLocationHooks::default()
    });
    let _guard = LocationPermissionHookGuard;

    with_configured_harness_context(enable_location_declaration, |mut context| {
        let handle = context.destack_os_permission_request_open(Permission::Location)?;
        let entries = context.destack_os_permission_request_read(handle, 0)?;
        context.destack_os_permission_request_close(handle)?;
        let entries = decode_permission_entries_value(&mut context, entries)?;
        let state = entries
            .first()
            .expect("permission request should return one result entry")
            .state;

        assert_eq!(state, PermissionState::Granted);

        Ok(())
    });
}

/// Enable one location declaration so permission tests reach the backend boundary.
fn enable_location_declaration(options: &mut RuntimeOptions) {
    options
        .app
        .permissions
        .insert(AppPermission::Location, Default::default());
}

/// Report one granted location permission result from the macOS host hook.
fn test_location_permission_request_hook(
    _host_runtime_id: HostSessionId,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    assert_eq!(permission, Permission::Location);

    Ok(PermissionState::Granted)
}
