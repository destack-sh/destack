/// Environment key used to override the cache directory.
pub const DESTACK_CACHE_DIR: &str = "DESTACK_CACHE_DIR";
/// Environment key used to override the cache mode.
pub const DESTACK_CACHE_MODE: &str = "DESTACK_CACHE_MODE";
/// Environment key used to override the cache scope.
pub const DESTACK_CACHE_SCOPE: &str = "DESTACK_CACHE_SCOPE";
/// Environment key used to override the cache policy.
pub const DESTACK_CACHE_POLICY: &str = "DESTACK_CACHE_POLICY";
/// Environment key used to override the cache size limit in megabytes.
pub const DESTACK_CACHE_MAX_MB: &str = "DESTACK_CACHE_MAX_MB";
/// Environment key used to override the cache validation mode.
pub const DESTACK_CACHE_VALIDATE: &str = "DESTACK_CACHE_VALIDATE";
/// Environment key used to override the watch mode.
pub const DESTACK_WATCH_MODE: &str = "DESTACK_WATCH_MODE";
/// Environment key used to override the watch poll interval in milliseconds.
pub const DESTACK_WATCH_POLL_MS: &str = "DESTACK_WATCH_POLL_MS";
/// Environment key used to override the watch debounce interval in milliseconds.
pub const DESTACK_WATCH_DEBOUNCE_MS: &str = "DESTACK_WATCH_DEBOUNCE_MS";
/// Environment key used to override the worker count.
pub const DESTACK_WORKERS: &str = "DESTACK_WORKERS";
/// Environment key used to override the slow task threshold in milliseconds.
pub const DESTACK_SLOW_TASK_MS: &str = "DESTACK_SLOW_TASK_MS";
/// Environment key used to override the default target selection.
pub const DESTACK_TARGET: &str = "DESTACK_TARGET";
/// Environment key used to override the default profile selection.
pub const DESTACK_PROFILE: &str = "DESTACK_PROFILE";
/// Environment key used to override the output directory for targets.
pub const DESTACK_OUT_DIR: &str = "DESTACK_OUT_DIR";
/// Environment key used to override the output file for single file targets.
pub const DESTACK_OUT_FILE: &str = "DESTACK_OUT_FILE";
/// Environment key used to override the declaration output directory.
pub const DESTACK_DECLARATION_DIR: &str = "DESTACK_DECLARATION_DIR";
/// Environment key used to override compiler logging filters.
pub const DESTACK_LOG: &str = "DESTACK_LOG";
/// Environment key for xdg cache home.
pub const XDG_CACHE_HOME: &str = "XDG_CACHE_HOME";
/// Environment key for home directory on unix.
pub const HOME: &str = "HOME";
/// Environment key for local app data on windows.
pub const LOCAL_APPDATA: &str = "LOCALAPPDATA";
/// Environment key for user profile on windows.
pub const USERPROFILE: &str = "USERPROFILE";
