#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

#[allow(unused_imports, unreachable_pub)]
pub use abi_generated::*;
#[allow(unused_imports, unreachable_pub)]
pub use bindings_generated::*;

/// Internal event kind selector for audio monitor records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioEventKind {
    /// Backend disconnected.
    BackendDisconnected,
    /// Backend reset.
    BackendReset,
    /// Default capture device changed.
    DefaultCaptureChanged,
    /// Default loopback device changed.
    DefaultLoopbackChanged,
    /// Default playback device changed.
    DefaultPlaybackChanged,
    /// Device added.
    DeviceAdded,
    /// Device format changed.
    DeviceFormatChanged,
    /// Device removed.
    DeviceRemoved,
    /// Device rerouted.
    DeviceRerouted,
    /// Interruption began.
    InterruptionBegan,
    /// Interruption ended.
    InterruptionEnded,
    /// Stream device changed.
    StreamDeviceChanged,
    /// Stream state changed.
    StreamStateChanged,
    /// Stream xrun reported.
    StreamXRun,
}

mod core;
mod host;
pub mod native;
pub(crate) mod simulation;
#[cfg(test)]
mod tests;
pub mod vm;
