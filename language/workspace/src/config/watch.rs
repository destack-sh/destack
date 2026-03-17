use serde::Deserialize;

/// Watch configuration options.
#[derive(Debug, Clone)]
pub struct WatchOptions {
    /// Debounce interval in milliseconds.
    pub debounce_ms: u64,
    /// Poll interval in milliseconds for polling watchers.
    pub poll_interval_ms: Option<u64>,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            debounce_ms: 30,
            poll_interval_ms: None,
        }
    }
}

/// Watch options (top-level).
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct WatchJson {
    /// Debounce interval in milliseconds.
    pub debounce_ms: Option<u64>,
    /// Poll interval in milliseconds for polling watchers.
    pub poll_interval_ms: Option<u64>,
}

impl From<&WatchJson> for WatchOptions {
    fn from(json: &WatchJson) -> Self {
        Self {
            debounce_ms: json.debounce_ms.unwrap_or(30),
            poll_interval_ms: json.poll_interval_ms,
        }
    }
}
