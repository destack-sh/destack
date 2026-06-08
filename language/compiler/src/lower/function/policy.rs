use destack_repository::{CheckFailurePolicy, CheckPolicy, Target};

/// Runtime check configuration for lowering.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RuntimeCheckConfig {
    /// Whether to emit overflow checks.
    pub(crate) overflow: bool,
    /// Whether to emit bounds checks.
    pub(crate) bounds: bool,
    /// Whether to emit null checks.
    pub(crate) null: bool,
    /// Whether to emit division checks.
    pub(crate) division: bool,
    /// Whether to emit shift checks.
    pub(crate) shift: bool,
    /// Check failure behavior.
    pub(crate) failure: CheckFailurePolicy,
}

impl RuntimeCheckConfig {
    /// Resolve runtime check policies for a target.
    pub(crate) fn from_target(target: &Target, debug: bool) -> Self {
        let checks = target.checks;

        Self {
            overflow: Self::policy_enabled(checks.overflow, debug),
            bounds: Self::policy_enabled(checks.bounds, debug),
            null: Self::policy_enabled(checks.null, debug),
            division: Self::policy_enabled(checks.division, debug),
            shift: Self::policy_enabled(checks.shift, debug),
            failure: checks.failure,
        }
    }

    /// Return true when a policy is enabled for the current debug mode.
    fn policy_enabled(policy: CheckPolicy, debug: bool) -> bool {
        match policy {
            CheckPolicy::Always => true,
            CheckPolicy::Debug => debug,
            CheckPolicy::Never => false,
        }
    }
}
