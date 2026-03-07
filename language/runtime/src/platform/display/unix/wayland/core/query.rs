use std::sync::Mutex;

/// Runtime adaptive-sync support state from wlr output management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaylandWlrAdaptiveSyncState {
    /// Adaptive sync is currently disabled for this output head.
    Disabled,
    /// Adaptive sync is currently enabled for this output head.
    Enabled,
}

/// Result state for one pending wlr output configuration request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaylandOutputConfigurationOutcome {
    /// Compositor accepted and applied this configuration.
    Succeeded,
    /// Compositor rejected this configuration.
    Failed,
    /// Compositor cancelled this configuration because state changed.
    Cancelled,
}

/// Shared completion payload for one pending output configuration request.
#[derive(Debug, Default)]
pub(crate) struct WaylandOutputConfigurationState {
    /// Final result state observed from configuration events.
    pub(crate) outcome: Mutex<Option<WaylandOutputConfigurationOutcome>>,
}

/// Mutable query payload for one output color-description request.
#[derive(Debug, Clone, Default)]
pub(crate) struct WaylandColorDescriptionQueryState {
    /// Whether the image-description object reached ready state.
    pub(crate) ready: bool,
    /// Optional failure details from image-description creation.
    pub(crate) failed_message: Option<String>,
    /// Optional image-description failure cause value.
    pub(crate) failed_cause: Option<u32>,
    /// Whether image-description information delivery completed.
    pub(crate) info_done: bool,
    /// Optional named primaries enum value.
    pub(crate) primaries_named: Option<u32>,
    /// Optional named transfer-function enum value.
    pub(crate) transfer_function_named: Option<u32>,
    /// Optional minimum luminance value multiplied by 10000.
    pub(crate) minimum_luminance: Option<u32>,
    /// Optional maximum luminance value.
    pub(crate) maximum_luminance: Option<u32>,
    /// Optional reference white luminance value.
    pub(crate) reference_luminance: Option<u32>,
    /// Optional target maximum content light-level value.
    pub(crate) target_max_cll: Option<u32>,
    /// Optional target maximum frame-average light-level value.
    pub(crate) target_max_fall: Option<u32>,
}

/// Mutable query payload for one gamma-control request.
#[derive(Debug, Clone, Default)]
pub(crate) struct WaylandGammaControlQueryState {
    /// Optional reported gamma table size.
    pub(crate) gamma_size: Option<u32>,
    /// Whether the gamma-control object reported failure.
    pub(crate) failed: bool,
}

/// Shared state for one pending xdg activation token request.
#[derive(Debug)]
pub(crate) struct WaylandActivationTokenState {
    /// Resolved token string when compositor responds.
    pub(crate) token: Mutex<Option<String>>,
}
