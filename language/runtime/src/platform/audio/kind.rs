/// Internal event kind selector for audio monitor records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AudioEventKind {
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
