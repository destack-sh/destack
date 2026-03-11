/// One execution domain for one host-affine platform service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServiceAffinity {
    /// Run directly on the caller thread.
    CallerThread,

    /// Run on one existing host loop.
    #[cfg(any(target_os = "macos", windows))]
    HostLoop(ServiceHostLoop),

    /// Run on one dedicated service thread.
    #[cfg(windows)]
    DedicatedThread(ServiceThreadBootstrap),
}

/// One existing host loop kind.
#[cfg(any(target_os = "macos", windows))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServiceHostLoop {
    /// Use the process main thread.
    #[cfg(target_os = "macos")]
    MainThread,

    /// Use the Windows message loop thread.
    #[cfg(windows)]
    WindowsMessageLoop,
}

/// One dedicated service-thread bootstrap mode.
#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServiceThreadBootstrap {
    /// Initialize one Windows multithreaded apartment.
    WindowsMta,
}
