use std::io::ErrorKind;
use std::path::Path;
#[cfg(all(test, not(target_os = "android")))]
use std::sync::{Mutex, OnceLock};

#[cfg(not(target_os = "android"))]
use crate::diagnostic::RuntimeError;
use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
#[cfg(not(target_os = "android"))]
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::PowerState;
use crate::platform::os::power::core::OS_POWER_SUSPEND_OPERATION;
use crate::runtime::BindingCallContext;

/// Power-supply root path on linux-like hosts.
const LINUX_POWER_SUPPLY_PATH: &str = "/sys/class/power_supply";
/// System suspend-state control path on linux-like hosts.
#[cfg(not(target_os = "android"))]
const LINUX_POWER_STATE_PATH: &str = "/sys/power/state";
/// Preferred linux suspend target ordering.
#[cfg(not(target_os = "android"))]
const LINUX_SUSPEND_STATES: [&str; 3] = ["mem", "standby", "freeze"];

/// Shared suspend hook used by linux tests.
#[cfg(all(test, not(target_os = "android")))]
type LinuxSuspendHook = fn() -> RuntimeResult<()>;

/// Return the shared linux suspend hook slot for tests.
#[cfg(all(test, not(target_os = "android")))]
fn linux_suspend_hook_slot() -> &'static Mutex<Option<LinuxSuspendHook>> {
    static HOOK: OnceLock<Mutex<Option<LinuxSuspendHook>>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(None))
}

/// Install one linux suspend hook for tests.
#[cfg(all(test, not(target_os = "android")))]
pub(crate) fn set_test_suspend_hook(hook: Option<LinuxSuspendHook>) {
    let mut slot = linux_suspend_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hook;
}

/// Resolve the active linux suspend hook for tests.
#[cfg(all(test, not(target_os = "android")))]
fn require_test_suspend_hook() -> RuntimeResult<LinuxSuspendHook> {
    let hook = linux_suspend_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .to_owned();

    let Some(hook) = hook else {
        return Err(core_platform::invalid_state(
            "power suspend tests must install a suspend hook before calling suspend",
        ));
    };

    Ok(hook)
}

/// Read one trimmed lowercase line from one sysfs file.
fn read_trimmed(path: &Path) -> Option<String> {
    let value = std::fs::read_to_string(path).ok()?;
    Some(value.trim().to_ascii_lowercase())
}

/// Read the preferred linux suspend target from the kernel power-state interface.
#[cfg(not(target_os = "android"))]
fn read_suspend_target() -> RuntimeResult<&'static str> {
    let states = std::fs::read_to_string(LINUX_POWER_STATE_PATH).map_err(|error| {
        let Some(error_code) = error.raw_os_error() else {
            return core_platform::io_operation_error(
                OS_POWER_SUSPEND_OPERATION,
                None,
                format!("read {LINUX_POWER_STATE_PATH} failed: {error}"),
            );
        };

        match error_code {
            libc::ENOENT | libc::ENODEV | libc::ENOSYS => {
                core_platform::not_supported(OS_POWER_SUSPEND_OPERATION)
            }
            libc::EACCES | libc::EPERM => core_platform::io_operation_error(
                OS_POWER_SUSPEND_OPERATION,
                Some(PlatformErrorCode::IoPermissionDenied),
                format!("read {LINUX_POWER_STATE_PATH} failed: {error}"),
            ),
            _ => core_platform::io_operation_error(
                OS_POWER_SUSPEND_OPERATION,
                None,
                format!("read {LINUX_POWER_STATE_PATH} failed: {error}"),
            ),
        }
    })?;

    for state in LINUX_SUSPEND_STATES {
        if states.split_ascii_whitespace().any(|entry| entry == state) {
            return Ok(state);
        }
    }

    Err(core_platform::not_supported(OS_POWER_SUSPEND_OPERATION))
}

/// Map one linux suspend write error into one runtime error.
#[cfg(not(target_os = "android"))]
fn linux_suspend_error(error: std::io::Error) -> Box<RuntimeError> {
    let Some(error_code) = error.raw_os_error() else {
        return core_platform::io_operation_error(
            OS_POWER_SUSPEND_OPERATION,
            None,
            format!("write {LINUX_POWER_STATE_PATH} failed: {error}"),
        );
    };

    match error_code {
        libc::EACCES | libc::EPERM => core_platform::io_operation_error(
            OS_POWER_SUSPEND_OPERATION,
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("write {LINUX_POWER_STATE_PATH} failed: {error}"),
        ),
        libc::EBUSY => core_platform::io_would_block(
            OS_POWER_SUSPEND_OPERATION,
            format!("write {LINUX_POWER_STATE_PATH} failed: {error}"),
        ),
        libc::EINVAL | libc::ENOENT | libc::ENODEV | libc::ENOSYS => {
            core_platform::not_supported(OS_POWER_SUSPEND_OPERATION)
        }
        _ => core_platform::io_operation_error(
            OS_POWER_SUSPEND_OPERATION,
            None,
            format!("write {LINUX_POWER_STATE_PATH} failed: {error}"),
        ),
    }
}

/// Detect host power state from linux sysfs power-supply metadata.
pub(crate) fn read_power_state(_binding: &BindingCallContext) -> RuntimeResult<PowerState> {
    let entries = match std::fs::read_dir(LINUX_POWER_SUPPLY_PATH) {
        Ok(entries) => entries,
        Err(error) => {
            if error.kind() == ErrorKind::NotFound {
                return Ok(PowerState::Unknown);
            }

            return Err(core_platform::io_error(
                "read_dir(/sys/class/power_supply)",
                None,
            ));
        }
    };

    let mut mains_online = false;
    let mut has_battery = false;

    for entry in entries.flatten() {
        let path = entry.path();
        let kind = read_trimmed(&path.join("type"));
        let status = read_trimmed(&path.join("status"));
        let online = read_trimmed(&path.join("online"));

        if matches!(kind.as_deref(), Some("mains" | "ac" | "usb")) {
            if matches!(online.as_deref(), Some("1")) {
                mains_online = true;
            }

            continue;
        }

        if matches!(kind.as_deref(), Some("battery")) {
            has_battery = true;
            if matches!(status.as_deref(), Some("discharging" | "not charging")) {
                mains_online = false;
            }
        }
    }

    if mains_online {
        return Ok(PowerState::AC);
    }

    if has_battery {
        return Ok(PowerState::Battery);
    }

    Ok(PowerState::Unknown)
}

/// Request one host suspend transition through the linux power-state interface.
pub(crate) fn request_suspend(_binding: &BindingCallContext) -> RuntimeResult<()> {
    #[cfg(all(test, not(target_os = "android")))]
    {
        let hook = require_test_suspend_hook()?;

        return hook();
    }

    #[cfg(target_os = "android")]
    {
        Err(core_platform::not_supported(OS_POWER_SUSPEND_OPERATION))
    }

    #[cfg(all(not(test), not(target_os = "android")))]
    let state = read_suspend_target()?;
    #[cfg(all(not(test), not(target_os = "android")))]
    let payload = format!("{state}\n");
    #[cfg(all(not(test), not(target_os = "android")))]
    std::fs::write(LINUX_POWER_STATE_PATH, payload).map_err(linux_suspend_error)?;

    #[cfg(all(not(test), not(target_os = "android")))]
    Ok(())
}
