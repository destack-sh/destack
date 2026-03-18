use std::sync::{Mutex, OnceLock};

use crate::diagnostic::RuntimeResult;
use crate::host::core::HostRuntimeId;
use crate::host::windows::{
    WindowsLocationHooks, set_windows_location_test_hooks, windows_notify_location_sample,
};
use crate::platform::os::abi_generated::{LocationSampleValue, LocationWatchOptionsValue};
use crate::platform::os::tests::with_configured_harness_context;

use super::common::{
    decode_location_sample, enable_location_declaration, location_watch_options,
    test_location_sample,
};

/// Shared Windows location-test mutex that serializes the process-global hook slot.
static TEST_LOCATION_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Shared captured Windows location watch id from the active watch-open hook.
static TEST_LOCATION_WATCH_ID: OnceLock<Mutex<Option<String>>> = OnceLock::new();

/// Reset the installed location hooks after one Windows test case.
struct LocationHookGuard;

impl Drop for LocationHookGuard {
    /// Clear the active location hooks after one test case.
    fn drop(&mut self) {
        set_windows_location_test_hooks(WindowsLocationHooks::default());

        let mut slot = TEST_LOCATION_WATCH_ID
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *slot = None;
    }
}

/// Verify Windows location routes last-known reads and watch events through runtime state.
#[test]
fn test_location_windows_routes_last_known_and_watch_streams() {
    let _guard = TEST_LOCATION_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    set_windows_location_test_hooks(WindowsLocationHooks {
        services_enabled: Some(test_location_services_enabled_hook),
        last_known: Some(test_location_last_known_hook),
        watch_open: Some(test_location_watch_open_hook),
        watch_close: Some(test_location_watch_close_hook),
        request_permission: None,
    });
    let _guard = LocationHookGuard;

    with_configured_harness_context(enable_location_declaration, |mut context| {
        let services_enabled = context.destack_os_location_services_enabled()?;
        assert!(services_enabled);

        let last_known = context.destack_os_location_last_known()?;
        let last_known = decode_location_sample(&mut context, last_known)?;
        assert_eq!(last_known, test_location_sample());

        let options = location_watch_options(&context);
        let handle = context.destack_os_location_watch_open(options)?;
        let watched = context.destack_os_location_watch_read(handle, 50_000_000)?;
        let watched = decode_location_sample(&mut context, watched)?;
        assert_eq!(watched, test_location_sample());

        context.destack_os_location_watch_close(handle)?;

        Ok(())
    });

    let watch_id = TEST_LOCATION_WATCH_ID
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
        .expect("Windows location watch hook should capture one watch id");
    assert!(watch_id.starts_with("location-watch-"));
}

/// Report that Windows location services are available in tests.
fn test_location_services_enabled_hook() -> RuntimeResult<bool> {
    Ok(true)
}

/// Report one cached Windows location sample in tests.
fn test_location_last_known_hook() -> RuntimeResult<LocationSampleValue> {
    Ok(test_location_sample())
}

/// Publish one location event for the opened watch in tests.
fn test_location_watch_open_hook(
    host_runtime_id: HostRuntimeId,
    watch_id: String,
    _options: LocationWatchOptionsValue,
) -> RuntimeResult<()> {
    let mut slot = TEST_LOCATION_WATCH_ID
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    *slot = Some(watch_id.clone());

    windows_notify_location_sample(host_runtime_id.0, &watch_id, test_location_sample())?;

    Ok(())
}

/// Accept one watch-close request in Windows tests.
fn test_location_watch_close_hook(
    _host_runtime_id: HostRuntimeId,
    _watch_id: String,
) -> RuntimeResult<()> {
    Ok(())
}
