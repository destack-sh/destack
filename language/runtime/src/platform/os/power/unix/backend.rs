#[cfg(any(target_os = "linux", target_os = "android"))]
use std::io::ErrorKind;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::path::Path;

use crate::diagnostic::RuntimeResult;
#[cfg(any(target_os = "linux", target_os = "android"))]
use crate::platform::core as core_platform;
use crate::platform::os::PowerState;
use crate::runtime::BindingCallContext;

/// Power-supply root path on linux-like hosts.
#[cfg(any(target_os = "linux", target_os = "android"))]
const LINUX_POWER_SUPPLY_PATH: &str = "/sys/class/power_supply";

#[cfg(any(target_os = "linux", target_os = "android"))]
/// Read one trimmed lowercase line from one sysfs file.
fn read_trimmed(path: &Path) -> Option<String> {
    let value = std::fs::read_to_string(path).ok()?;
    Some(value.trim().to_ascii_lowercase())
}

#[cfg(any(target_os = "linux", target_os = "android"))]
/// Detect host power state from linux sysfs power-supply metadata.
fn read_linux_power_state() -> RuntimeResult<PowerState> {
    // enumerate host power-supply entries
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

    // track the strongest observed power-source evidence
    let mut mains_online = false;
    let mut has_battery = false;

    // scan each power-supply endpoint
    for entry in entries.flatten() {
        let path = entry.path();
        let kind = read_trimmed(&path.join("type"));
        let status = read_trimmed(&path.join("status"));
        let online = read_trimmed(&path.join("online"));

        // classify mains adapters and detect online status
        if matches!(kind.as_deref(), Some("mains" | "ac" | "usb")) {
            if matches!(online.as_deref(), Some("1")) {
                mains_online = true;
            }

            continue;
        }

        // classify batteries and infer active discharge state
        if matches!(kind.as_deref(), Some("battery")) {
            has_battery = true;
            if matches!(status.as_deref(), Some("discharging" | "not charging")) {
                mains_online = false;
            }
        }
    }

    // map classified supply evidence into normalized power state
    if mains_online {
        return Ok(PowerState::AC);
    }
    if has_battery {
        return Ok(PowerState::Battery);
    }

    Ok(PowerState::Unknown)
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
/// Detect host power state on non-linux unix hosts.
fn read_linux_power_state() -> RuntimeResult<PowerState> {
    Ok(PowerState::Unknown)
}

/// Read one host power-state value from unix APIs.
pub(crate) fn read_power_state(_binding: &BindingCallContext) -> RuntimeResult<PowerState> {
    read_linux_power_state()
}
