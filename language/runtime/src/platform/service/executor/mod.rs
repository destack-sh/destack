mod caller;
#[cfg(windows)]
mod dedicated;
#[cfg(any(target_os = "macos", windows))]
mod host;

pub(crate) use caller::CallerThreadExecutor;
#[cfg(windows)]
pub(crate) use dedicated::DedicatedThreadExecutor;
#[cfg(windows)]
pub(crate) use dedicated::ServiceThreadGuard;
#[cfg(any(target_os = "macos", windows))]
pub(crate) use host::HostLoopExecutor;
