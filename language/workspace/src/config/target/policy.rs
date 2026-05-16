use serde::{Deserialize, Serialize};

/// Floating point math optimization policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum FloatMathPolicy {
    /// Strict IEEE semantics.
    #[default]
    Strict,
    /// Allow reassociation and algebraic simplifications.
    Reassociate,
    /// Enable fast math optimizations.
    Fast,
}

impl std::str::FromStr for FloatMathPolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "strict" => Ok(Self::Strict),
            "reassoc" | "reassociate" | "relaxed" => Ok(Self::Reassociate),
            "fast" | "fast_math" | "fastmath" => Ok(Self::Fast),
            _ => Err(()),
        }
    }
}

impl FloatMathPolicy {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Runtime check policy for generated safety checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CheckPolicy {
    /// Always emit the check.
    Always,
    /// Emit the check for debug profiles.
    #[default]
    Debug,
    /// Never emit the check.
    Never,
}

impl std::str::FromStr for CheckPolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "always" => Ok(Self::Always),
            "debug" => Ok(Self::Debug),
            "never" | "off" => Ok(Self::Never),
            _ => Err(()),
        }
    }
}

impl CheckPolicy {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Runtime checks emitted by generated code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeChecks {
    /// Integer overflow check policy.
    pub overflow: CheckPolicy,
    /// Bounds check policy for array and slice accesses.
    pub bounds: CheckPolicy,
    /// Null check policy for reference operations.
    pub null: CheckPolicy,
    /// Division check policy for divide and remainder operations.
    pub division: CheckPolicy,
    /// Shift range check policy.
    pub shift: CheckPolicy,
    /// Check failure behavior.
    pub failure: CheckFailurePolicy,
}

impl RuntimeChecks {
    /// Return checks where every check uses the same policy.
    pub fn all(policy: CheckPolicy) -> Self {
        Self {
            overflow: policy,
            bounds: policy,
            null: policy,
            division: policy,
            shift: policy,
            failure: CheckFailurePolicy::default(),
        }
    }
}

/// Check failure behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CheckFailurePolicy {
    /// Trap immediately on a failed check.
    Trap,
    /// Trigger a panic on a failed check.
    #[default]
    Panic,
    /// Abort execution on a failed check.
    Abort,
}

impl std::str::FromStr for CheckFailurePolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "trap" => Ok(Self::Trap),
            "panic" => Ok(Self::Panic),
            "abort" => Ok(Self::Abort),
            _ => Err(()),
        }
    }
}

impl CheckFailurePolicy {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}
