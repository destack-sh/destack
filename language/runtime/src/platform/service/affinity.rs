/// One execution domain for one host-affine platform service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServiceAffinity {
    /// Run directly on the caller thread.
    CallerThread,

    /// Run on one existing host loop.
    HostLoop(ServiceHostLoop),

    /// Run on one dedicated service thread.
    #[cfg(windows)]
    DedicatedThread(ServiceThreadBootstrap),
}

/// One existing host loop kind.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ServiceThreadBootstrap {
    /// Run without extra thread initialization.
    None,

    /// Initialize one Windows multithreaded apartment.
    #[cfg(windows)]
    WindowsMta,
}
