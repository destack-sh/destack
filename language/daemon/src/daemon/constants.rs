/// Directory name for endpoint state.
pub(crate) const ENDPOINT_DIRECTORY: &str = "daemon";

/// Lock file name for endpoint ownership.
pub(crate) const LOCK_FILE: &str = "daemon.lock";

/// Metadata file name for endpoint discovery.
pub(crate) const METADATA_FILE: &str = "daemon.json";

/// Temporary directory name for endpoint sockets.
pub(crate) const SOCKET_DIRECTORY: &str = "destack-daemon";

/// Default idle shutdown timeout for the daemon.
pub(crate) const DEFAULT_IDLE_SHUTDOWN_MS: u64 = 600_000;

/// Idle shutdown monitor poll interval.
pub(crate) const IDLE_SHUTDOWN_POLL_MS: u64 = 250;
