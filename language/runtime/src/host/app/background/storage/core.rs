use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::platform::PlatformError;
use crate::platform::core::not_supported;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    BackgroundTaskDescriptorValue, BackgroundTaskOptionsValue,
};

/// Prefix for persisted desktop background scheduler artifacts.
pub(in crate::host::app::background) const DESKTOP_BACKGROUND_SCHEDULER_PREFIX: &str =
    "destack-background";
/// Minimum desktop background interval accepted on Windows task scheduler.
pub(in crate::host::app::background) const DESKTOP_BACKGROUND_WINDOWS_MINIMUM_INTERVAL_NS: u64 =
    60_000_000_000;

/// Persisted desktop background task record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::host::app::background) struct DesktopBackgroundTaskRecord {
    /// Registered background task options.
    pub(in crate::host::app::background) options: BackgroundTaskOptionsValue,
}

/// Validate one desktop background task payload against the current host model.
pub(in crate::host::app::background) fn validate_desktop_background_options(
    platform: Platform,
    options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    if options.identifier.trim().is_empty() {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "identifier",
            "desktop background identifier must not be empty",
        ))
        .boxed());
    }

    // scheduler registrations are always periodic and persisted
    if options.minimum_interval_ns == 0 {
        return Err(not_supported("destack.os.background.register"));
    }

    // task scheduler only accepts minute-granularity repetition
    if matches!(platform, Platform::Windows)
        && options.minimum_interval_ns < DESKTOP_BACKGROUND_WINDOWS_MINIMUM_INTERVAL_NS
    {
        return Err(not_supported("destack.os.background.register"));
    }

    Ok(())
}

/// Return one descriptor view for one persisted task options payload.
pub(in crate::host::app::background) fn background_descriptor_from_options(
    options: BackgroundTaskOptionsValue,
) -> BackgroundTaskDescriptorValue {
    BackgroundTaskDescriptorValue {
        identifier: options.identifier,
        trigger: options.trigger,
        minimum_interval_ns: options.minimum_interval_ns,
    }
}

/// Return the effective repetition interval in whole seconds.
pub(in crate::host::app::background) fn background_interval_seconds(
    options: &BackgroundTaskOptionsValue,
) -> u64 {
    let interval_seconds = options.minimum_interval_ns.saturating_add(999_999_999) / 1_000_000_000;

    interval_seconds.max(1)
}

/// Remove one persisted background file when it exists.
pub(in crate::host::app::background) fn remove_background_file_if_exists(
    path: &Path,
    operation: &'static str,
) -> RuntimeResult<()> {
    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(path).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("{operation}: failed to remove {}: {error}", path.display()),
        ))
        .boxed()
    })?;

    Ok(())
}
