/// One execution lifetime policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionLifetime {
    /// One process-global or subsystem-global execution object.
    Global,
    /// One execution object owned by one live instance.
    Instance,
    /// One execution object owned by one bounded task.
    Task,
}

/// One execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionMode {
    /// Run directly on the caller thread.
    Inline,
    /// Run on one existing host-owned loop or affinity thread.
    #[cfg_attr(target_os = "android", allow(dead_code))]
    Host,
    /// Run on one owned dedicated thread.
    Thread,
    /// Run on one owned long-lived loop.
    Loop,
    /// Run on one polling execution object.
    Polling,
    /// Run as one finite blocking task.
    Blocking,
}

/// One execution affinity requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionAffinity {
    /// Use the process main thread.
    #[cfg(target_os = "macos")]
    MainThread,
    /// Use the Windows message loop thread.
    #[cfg(windows)]
    WindowsMessageLoop,
    /// Initialize one Windows multithreaded apartment.
    #[cfg(windows)]
    WindowsMta,
}

/// One declared execution policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExecutionPolicy {
    /// The lifetime for this execution object.
    pub(crate) lifetime: ExecutionLifetime,
    /// The mode for this execution object.
    pub(crate) mode: ExecutionMode,
    /// The optional affinity requirement for this execution object.
    pub(crate) affinity: Option<ExecutionAffinity>,
}

#[allow(dead_code)]
impl ExecutionPolicy {
    /// Build one global execution policy.
    pub(crate) const fn global(mode: ExecutionMode) -> Self {
        Self {
            lifetime: ExecutionLifetime::Global,
            mode,
            affinity: None,
        }
    }

    /// Build one instance execution policy.
    pub(crate) const fn instance(mode: ExecutionMode) -> Self {
        Self {
            lifetime: ExecutionLifetime::Instance,
            mode,
            affinity: None,
        }
    }

    /// Build one task execution policy.
    pub(crate) const fn task(mode: ExecutionMode) -> Self {
        Self {
            lifetime: ExecutionLifetime::Task,
            mode,
            affinity: None,
        }
    }

    /// Return this policy with one explicit affinity.
    pub(crate) const fn with_affinity(self, affinity: ExecutionAffinity) -> Self {
        Self {
            lifetime: self.lifetime,
            mode: self.mode,
            affinity: Some(affinity),
        }
    }

    /// Require one specific execution mode.
    pub(crate) fn expect_mode(self, mode: ExecutionMode) {
        if self.mode != mode {
            panic!(
                "execution policy declared {} mode, expected {} mode",
                self.mode.name(),
                mode.name(),
            );
        }
    }
}

impl ExecutionLifetime {
    /// Return one stable lowercase lifetime name.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Instance => "instance",
            Self::Task => "task",
        }
    }
}

impl ExecutionMode {
    /// Return whether this mode owns one thread.
    pub(crate) const fn owns_thread(self) -> bool {
        matches!(
            self,
            Self::Thread | Self::Loop | Self::Polling | Self::Blocking
        )
    }

    /// Return one stable lowercase mode name.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Host => "host",
            Self::Thread => "thread",
            Self::Loop => "loop",
            Self::Polling => "polling",
            Self::Blocking => "blocking",
        }
    }
}
