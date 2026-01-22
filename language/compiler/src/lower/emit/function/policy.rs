use destack_workspace::{
    BoundsCheckPolicy, CheckFailurePolicy, DivisionCheckPolicy, NullCheckPolicy,
    OverflowCheckPolicy, ShiftCheckPolicy, Target,
};

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
        // resolve policies with debug awareness
        let overflow = Self::policy_enabled(target.overflow_checks, debug);
        let bounds = Self::policy_enabled(target.bounds_checks, debug);
        let null = Self::policy_enabled(target.null_checks, debug);
        let division = Self::policy_enabled(target.division_checks, debug);
        let shift = Self::policy_enabled(target.shift_checks, debug);

        Self {
            overflow,
            bounds,
            null,
            division,
            shift,
            failure: target.check_failure,
        }
    }


    /// Return true when a policy is enabled for the current debug mode.
    fn policy_enabled<T>(policy: T, debug: bool) -> bool
    where
        T: Into<RuntimePolicy>,
    {
        match policy.into() {
            RuntimePolicy::Always => true,
            RuntimePolicy::Debug => debug,
            RuntimePolicy::Never => false,
        }
    }
}

/// Normalized runtime check policy variants.
#[derive(Debug, Clone, Copy)]
enum RuntimePolicy {
    /// Always enable this check.
    Always,
    /// Enable only in debug mode.
    Debug,
    /// Never enable this check.
    Never,
}

impl From<OverflowCheckPolicy> for RuntimePolicy {
    fn from(policy: OverflowCheckPolicy) -> Self {
        match policy {
            OverflowCheckPolicy::Always => Self::Always,
            OverflowCheckPolicy::Debug => Self::Debug,
            OverflowCheckPolicy::Never => Self::Never,
        }
    }
}

impl From<BoundsCheckPolicy> for RuntimePolicy {
    fn from(policy: BoundsCheckPolicy) -> Self {
        match policy {
            BoundsCheckPolicy::Always => Self::Always,
            BoundsCheckPolicy::Debug => Self::Debug,
            BoundsCheckPolicy::Never => Self::Never,
        }
    }
}

impl From<DivisionCheckPolicy> for RuntimePolicy {
    fn from(policy: DivisionCheckPolicy) -> Self {
        match policy {
            DivisionCheckPolicy::Always => Self::Always,
            DivisionCheckPolicy::Debug => Self::Debug,
            DivisionCheckPolicy::Never => Self::Never,
        }
    }
}

impl From<ShiftCheckPolicy> for RuntimePolicy {
    fn from(policy: ShiftCheckPolicy) -> Self {
        match policy {
            ShiftCheckPolicy::Always => Self::Always,
            ShiftCheckPolicy::Debug => Self::Debug,
            ShiftCheckPolicy::Never => Self::Never,
        }
    }
}

impl From<NullCheckPolicy> for RuntimePolicy {
    fn from(policy: NullCheckPolicy) -> Self {
        match policy {
            NullCheckPolicy::Always => Self::Always,
            NullCheckPolicy::Debug => Self::Debug,
            NullCheckPolicy::Never => Self::Never,
        }
    }
}
