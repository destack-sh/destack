use core_foundation_sys::base::{CFIndex, CFRelease, CFTypeRef};
use core_foundation_sys::string::{
    CFStringGetCString, CFStringGetLength, CFStringGetMaximumSizeForEncoding, CFStringRef,
    kCFStringEncodingUTF8,
};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
use crate::diagnostic::RuntimeError;
use crate::diagnostic::RuntimeResult;
#[cfg(test)]
use crate::platform::PlatformError;
use crate::platform::core::{invalid_argument, io_operation_error, io_would_block, not_supported};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::PowerState;
use crate::platform::os::power::core::{OS_POWER_STATE_OPERATION, OS_POWER_SUSPEND_OPERATION};
use crate::runtime::BindingCallContext;

/// Null mach port value used by IOPMFindPowerManagement.
#[cfg(not(test))]
const MACH_PORT_NULL: u32 = 0;
/// Successful IOKit return code.
const KIO_RETURN_SUCCESS: i32 = 0;
/// IOKit privilege-denied return code.
const KIO_RETURN_NOT_PRIVILEGED: i32 = 0xe000_02c1u32 as i32;
/// IOKit unsupported-operation return code.
const KIO_RETURN_UNSUPPORTED: i32 = 0xe000_02c7u32 as i32;
/// IOKit busy return code.
const KIO_RETURN_BUSY: i32 = 0xe000_02d5u32 as i32;
/// IOKit timeout return code.
const KIO_RETURN_TIMEOUT: i32 = 0xe000_02d6u32 as i32;
/// IOKit not-permitted return code.
const KIO_RETURN_NOT_PERMITTED: i32 = 0xe000_02e2u32 as i32;
/// IOKit no-power return code.
const KIO_RETURN_NO_POWER: i32 = 0xe000_02e3u32 as i32;
/// Runtime name returned by IOKit for AC power.
const MACOS_POWER_SOURCE_AC: &str = "AC Power";
/// Runtime name returned by IOKit for battery power.
const MACOS_POWER_SOURCE_BATTERY: &str = "Battery Power";
/// Runtime name returned by IOKit for UPS power.
const MACOS_POWER_SOURCE_UPS: &str = "UPS Power";

/// Native mach port type used by IOKit power-management entry points.
#[cfg(not(test))]
type MachPort = u32;
/// Native IOKit connection handle used by IOPM power-management calls.
#[cfg(not(test))]
type IoConnect = u32;
/// Native IOKit error code type.
type IoReturn = i32;
/// Native boolean type used by IOPMSleepEnabled.
#[cfg(not(test))]
type Boolean = u32;

// link IOKit power-state entry points
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    /// Copy one retained power-source snapshot payload.
    fn IOPSCopyPowerSourcesInfo() -> CFTypeRef;
    /// Return the active power source name for one power-source snapshot.
    fn IOPSGetProvidingPowerSourceType(blob: CFTypeRef) -> CFStringRef;
}

// link IOKit power-management entry points used for host suspend requests
#[cfg(not(test))]
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    /// Resolve one root power-domain connection handle.
    fn IOPMFindPowerManagement(master_device_port: MachPort) -> IoConnect;
    /// Return whether the host supports full suspend.
    fn IOPMSleepEnabled() -> Boolean;
    /// Request one host suspend transition.
    fn IOPMSleepSystem(connection: IoConnect) -> IoReturn;
    /// Close one root power-domain connection handle.
    fn IOServiceClose(connection: IoConnect) -> IoReturn;
}

/// Owned root-domain power-management connection on macOS.
#[cfg(not(test))]
struct MacosPowerConnection {
    /// Root power-domain connection handle.
    connection: IoConnect,
}

/// Retained CoreFoundation power-snapshot payload.
struct MacosPowerSnapshot {
    /// Retained CoreFoundation payload.
    value: CFTypeRef,
}

impl Drop for MacosPowerSnapshot {
    /// Release the retained CoreFoundation payload.
    fn drop(&mut self) {
        if !self.value.is_null() {
            unsafe {
                CFRelease(self.value);
            }
        }
    }
}

#[cfg(not(test))]
impl Drop for MacosPowerConnection {
    /// Close the root power-domain connection.
    fn drop(&mut self) {
        if self.connection != 0 {
            unsafe {
                IOServiceClose(self.connection);
            }
        }
    }
}

/// Shared suspend hook used by macOS tests.
#[cfg(test)]
type MacosSuspendHook = fn() -> IoReturn;

/// Return the shared macOS suspend hook slot for tests.
#[cfg(test)]
fn macos_suspend_hook_slot() -> &'static Mutex<Option<MacosSuspendHook>> {
    static HOOK: OnceLock<Mutex<Option<MacosSuspendHook>>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(None))
}

/// Install one macOS suspend hook for tests.
#[cfg(test)]
pub(crate) fn set_test_suspend_hook(hook: Option<MacosSuspendHook>) {
    let mut slot = macos_suspend_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hook;
}

/// Resolve the active macOS suspend hook for tests.
#[cfg(test)]
fn require_test_suspend_hook() -> RuntimeResult<MacosSuspendHook> {
    let hook = macos_suspend_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .to_owned();

    let Some(hook) = hook else {
        return Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            "power suspend tests must install a suspend hook before calling suspend",
        ))
        .boxed());
    };

    Ok(hook)
}

/// Copy one CoreFoundation string into UTF-8.
fn copy_cf_string(value: CFStringRef, operation: &'static str) -> RuntimeResult<String> {
    if value.is_null() {
        return Err(invalid_argument(
            "powerSource",
            format!("{operation} returned one null power-source string"),
        ));
    }

    let length = unsafe { CFStringGetLength(value) };
    if length < 0 {
        return Err(io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "power-source string length was negative",
        ));
    }

    let max_utf8 = unsafe { CFStringGetMaximumSizeForEncoding(length, kCFStringEncodingUTF8) };
    if max_utf8 < 0 {
        return Err(io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "power-source string length could not be encoded as utf8",
        ));
    }

    let mut bytes = vec![0u8; max_utf8 as usize + 1];
    let converted = unsafe {
        CFStringGetCString(
            value,
            bytes.as_mut_ptr().cast(),
            bytes.len() as CFIndex,
            kCFStringEncodingUTF8,
        )
    };
    if converted == 0 {
        return Err(io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "power-source string was not valid utf8",
        ));
    }

    let terminator = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let value = String::from_utf8(bytes[..terminator].to_vec()).map_err(|_| {
        io_operation_error(
            operation,
            Some(PlatformErrorCode::IoInvalidData),
            "power-source string was not valid utf8",
        )
    })?;

    Ok(value)
}

/// Copy one retained power-source snapshot.
fn copy_power_snapshot() -> RuntimeResult<MacosPowerSnapshot> {
    let snapshot = unsafe { IOPSCopyPowerSourcesInfo() };
    if snapshot.is_null() {
        return Err(io_operation_error(
            OS_POWER_STATE_OPERATION,
            None,
            "IOPSCopyPowerSourcesInfo returned one null snapshot",
        ));
    }

    Ok(MacosPowerSnapshot { value: snapshot })
}

/// Open one root power-domain connection on macOS.
#[cfg(not(test))]
fn open_macos_power_connection() -> RuntimeResult<MacosPowerConnection> {
    let is_sleep_enabled = unsafe { IOPMSleepEnabled() };
    if is_sleep_enabled == 0 {
        return Err(not_supported(OS_POWER_SUSPEND_OPERATION));
    }

    let connection = unsafe { IOPMFindPowerManagement(MACH_PORT_NULL) };
    if connection == 0 {
        return Err(not_supported(OS_POWER_SUSPEND_OPERATION));
    }

    Ok(MacosPowerConnection { connection })
}

/// Request one macOS suspend transition.
#[cfg(not(test))]
fn request_macos_suspend(connection: IoConnect) -> IoReturn {
    unsafe { IOPMSleepSystem(connection) }
}

/// Map one macOS suspend result into one runtime error.
fn map_macos_suspend_error(result: IoReturn) -> RuntimeResult<()> {
    if result == KIO_RETURN_SUCCESS {
        return Ok(());
    }

    if matches!(result, KIO_RETURN_NOT_PRIVILEGED | KIO_RETURN_NOT_PERMITTED) {
        return Err(io_operation_error(
            OS_POWER_SUSPEND_OPERATION,
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("IOPMSleepSystem failed with status 0x{:08x}", result as u32),
        ));
    }

    if matches!(result, KIO_RETURN_BUSY | KIO_RETURN_TIMEOUT) {
        return Err(io_would_block(
            OS_POWER_SUSPEND_OPERATION,
            format!("IOPMSleepSystem failed with status 0x{:08x}", result as u32),
        ));
    }

    if matches!(result, KIO_RETURN_UNSUPPORTED | KIO_RETURN_NO_POWER) {
        return Err(not_supported(OS_POWER_SUSPEND_OPERATION));
    }

    Err(io_operation_error(
        OS_POWER_SUSPEND_OPERATION,
        None,
        format!("IOPMSleepSystem failed with status 0x{:08x}", result as u32),
    ))
}

/// Read one host power-state value from macOS APIs.
pub(crate) fn read_power_state(_binding: &BindingCallContext) -> RuntimeResult<PowerState> {
    let snapshot = copy_power_snapshot()?;
    let source = unsafe { IOPSGetProvidingPowerSourceType(snapshot.value) };
    let source = copy_cf_string(source, OS_POWER_STATE_OPERATION)?;

    let state = match source.as_str() {
        MACOS_POWER_SOURCE_AC | MACOS_POWER_SOURCE_UPS => PowerState::AC,
        MACOS_POWER_SOURCE_BATTERY => PowerState::Battery,
        _ => PowerState::Unknown,
    };

    Ok(state)
}

/// Request one host suspend transition through macOS power-management APIs.
pub(crate) fn request_suspend(_binding: &BindingCallContext) -> RuntimeResult<()> {
    #[cfg(test)]
    {
        let hook = require_test_suspend_hook()?;

        map_macos_suspend_error(hook())
    }

    #[cfg(not(test))]
    let connection = open_macos_power_connection()?;
    #[cfg(not(test))]
    let result = request_macos_suspend(connection.connection);

    #[cfg(not(test))]
    map_macos_suspend_error(result)
}

#[cfg(test)]
mod tests {
    use crate::platform::diagnostic::PlatformErrorCode;
    use crate::tests::platform::error_code_from_result;

    use super::{
        KIO_RETURN_BUSY, KIO_RETURN_NO_POWER, KIO_RETURN_NOT_PERMITTED, KIO_RETURN_NOT_PRIVILEGED,
        KIO_RETURN_SUCCESS, KIO_RETURN_TIMEOUT, KIO_RETURN_UNSUPPORTED, map_macos_suspend_error,
    };

    #[test]
    fn test_map_macos_suspend_success() {
        let result = map_macos_suspend_error(KIO_RETURN_SUCCESS);

        assert!(result.is_ok());
    }

    #[test]
    fn test_map_macos_suspend_permission_denied() {
        let not_privileged =
            error_code_from_result(map_macos_suspend_error(KIO_RETURN_NOT_PRIVILEGED))
                .expect("not-privileged result should decode");
        let not_permitted =
            error_code_from_result(map_macos_suspend_error(KIO_RETURN_NOT_PERMITTED))
                .expect("not-permitted result should decode");

        assert_eq!(not_privileged, PlatformErrorCode::IoPermissionDenied);
        assert_eq!(not_permitted, PlatformErrorCode::IoPermissionDenied);
    }

    #[test]
    fn test_map_macos_suspend_would_block() {
        let busy = error_code_from_result(map_macos_suspend_error(KIO_RETURN_BUSY))
            .expect("busy result should decode");
        let timeout = error_code_from_result(map_macos_suspend_error(KIO_RETURN_TIMEOUT))
            .expect("timeout result should decode");

        assert_eq!(busy, PlatformErrorCode::IoWouldBlock);
        assert_eq!(timeout, PlatformErrorCode::IoWouldBlock);
    }

    #[test]
    fn test_map_macos_suspend_not_supported() {
        let unsupported = error_code_from_result(map_macos_suspend_error(KIO_RETURN_UNSUPPORTED))
            .expect("unsupported result should decode");
        let no_power = error_code_from_result(map_macos_suspend_error(KIO_RETURN_NO_POWER))
            .expect("no-power result should decode");

        assert_eq!(unsupported, PlatformErrorCode::NotSupported);
        assert_eq!(no_power, PlatformErrorCode::NotSupported);
    }
}
