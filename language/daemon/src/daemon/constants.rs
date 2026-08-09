use std::time::Duration;

/// Default time before an idle daemon shuts down.
pub(crate) const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(600);
