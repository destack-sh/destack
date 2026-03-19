use crate::diagnostic::RuntimeResult;
use crate::host::Platform;
use crate::host::core::HostRequestContext;
use crate::platform::core::not_supported;
use crate::platform::os::abi_generated::{BackgroundStatusValue, BackgroundTaskOptionsValue};

use super::macos;

/// Return the current desktop background scheduler status.
pub(super) fn background_status(
    context: &HostRequestContext,
) -> RuntimeResult<BackgroundStatusValue> {
    match context.platform {
        Platform::MacOS => macos::background_status(),
        Platform::Windows => windows_background_status(),
        Platform::Linux => linux_background_status(),
        _ => Ok(BackgroundStatusValue::Unavailable),
    }
}

/// Register one desktop background task through the host scheduler.
pub(super) fn background_register(
    context: &HostRequestContext,
    options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    match context.platform {
        Platform::MacOS => macos::register_background_task(context, options),
        Platform::Windows => windows_register_background_task(context, options),
        Platform::Linux => linux_register_background_task(context, options),
        _ => Err(not_supported("destack.os.background.register")),
    }
}

/// Unregister one desktop background task from the host scheduler.
pub(super) fn background_unregister(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<()> {
    match context.platform {
        Platform::MacOS => macos::unregister_background_task(context, identifier),
        Platform::Windows => windows_unregister_background_task(context, identifier),
        Platform::Linux => linux_unregister_background_task(context, identifier),
        _ => Err(not_supported("destack.os.background.unregister")),
    }
}

/// Trigger one persisted desktop background task immediately through the host scheduler.
pub(super) fn trigger_background_task(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<bool> {
    match context.platform {
        Platform::MacOS => macos::trigger_background_task(context, identifier),
        Platform::Windows => windows_trigger_background_task(context, identifier),
        Platform::Linux => linux_trigger_background_task(identifier),
        _ => Err(not_supported("destack.os.background.triggerTest")),
    }
}

#[cfg(target_os = "linux")]
use super::linux as linux_scheduler_backend;
#[cfg(windows)]
use super::windows as windows_scheduler_backend;

#[cfg(target_os = "linux")]
/// Return the current Linux background scheduler status.
fn linux_background_status() -> RuntimeResult<BackgroundStatusValue> {
    linux_scheduler_backend::background_status()
}

#[cfg(not(target_os = "linux"))]
/// Return the current Linux background scheduler status.
fn linux_background_status() -> RuntimeResult<BackgroundStatusValue> {
    Ok(BackgroundStatusValue::Unavailable)
}

#[cfg(target_os = "linux")]
/// Register one Linux background task through the native scheduler.
fn linux_register_background_task(
    context: &HostRequestContext,
    options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    linux_scheduler_backend::register_background_task(context, options)
}

#[cfg(not(target_os = "linux"))]
/// Register one Linux background task through the native scheduler.
fn linux_register_background_task(
    _context: &HostRequestContext,
    _options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.background.register"))
}

#[cfg(target_os = "linux")]
/// Remove one Linux background task from the native scheduler.
fn linux_unregister_background_task(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<()> {
    linux_scheduler_backend::unregister_background_task(context, identifier)
}

#[cfg(not(target_os = "linux"))]
/// Remove one Linux background task from the native scheduler.
fn linux_unregister_background_task(
    _context: &HostRequestContext,
    _identifier: &str,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.background.unregister"))
}

#[cfg(target_os = "linux")]
/// Trigger one Linux background task immediately.
fn linux_trigger_background_task(identifier: &str) -> RuntimeResult<bool> {
    linux_scheduler_backend::trigger_background_task(identifier)
}

#[cfg(not(target_os = "linux"))]
/// Trigger one Linux background task immediately.
fn linux_trigger_background_task(_identifier: &str) -> RuntimeResult<bool> {
    Err(not_supported("destack.os.background.triggerTest"))
}

#[cfg(windows)]
/// Return the current Windows background scheduler status.
fn windows_background_status() -> RuntimeResult<BackgroundStatusValue> {
    windows_scheduler_backend::background_status()
}

#[cfg(not(windows))]
/// Return the current Windows background scheduler status.
fn windows_background_status() -> RuntimeResult<BackgroundStatusValue> {
    Ok(BackgroundStatusValue::Unavailable)
}

#[cfg(windows)]
/// Register one Windows background task through the native scheduler.
fn windows_register_background_task(
    context: &HostRequestContext,
    options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    windows_scheduler_backend::register_background_task(context, options)
}

#[cfg(not(windows))]
/// Register one Windows background task through the native scheduler.
fn windows_register_background_task(
    _context: &HostRequestContext,
    _options: &BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.background.register"))
}

#[cfg(windows)]
/// Remove one Windows background task from the native scheduler.
fn windows_unregister_background_task(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<()> {
    windows_scheduler_backend::unregister_background_task(context, identifier)
}

#[cfg(not(windows))]
/// Remove one Windows background task from the native scheduler.
fn windows_unregister_background_task(
    _context: &HostRequestContext,
    _identifier: &str,
) -> RuntimeResult<()> {
    Err(not_supported("destack.os.background.unregister"))
}

#[cfg(windows)]
/// Trigger one Windows background task immediately.
fn windows_trigger_background_task(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<bool> {
    windows_scheduler_backend::trigger_background_task(context, identifier)
}

#[cfg(not(windows))]
/// Trigger one Windows background task immediately.
fn windows_trigger_background_task(
    _context: &HostRequestContext,
    _identifier: &str,
) -> RuntimeResult<bool> {
    Err(not_supported("destack.os.background.triggerTest"))
}
