use serde::Deserialize;

/// Floating point math optimization policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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

/// Floating point math policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum FloatMathPolicyJson {
    /// Strict IEEE semantics.
    Strict,
    /// Permit reassociation but preserve NaNs and infinities.
    #[serde(alias = "reassoc", alias = "reassociate")]
    Reassociate,
    /// Enable fast math optimizations.
    Fast,
}

impl From<FloatMathPolicyJson> for FloatMathPolicy {
    fn from(value: FloatMathPolicyJson) -> Self {
        match value {
            FloatMathPolicyJson::Strict => FloatMathPolicy::Strict,
            FloatMathPolicyJson::Reassociate => FloatMathPolicy::Reassociate,
            FloatMathPolicyJson::Fast => FloatMathPolicy::Fast,
        }
    }
}

/// Runtime check policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CheckPolicyJson {
    /// Always emit the check.
    Always,
    /// Emit the check for debug profiles.
    Debug,
    /// Never emit the check.
    Never,
}

impl From<CheckPolicyJson> for CheckPolicy {
    fn from(value: CheckPolicyJson) -> Self {
        match value {
            CheckPolicyJson::Always => CheckPolicy::Always,
            CheckPolicyJson::Debug => CheckPolicy::Debug,
            CheckPolicyJson::Never => CheckPolicy::Never,
        }
    }
}

/// Runtime checks for JSON deserialization.
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum RuntimeChecksJson {
    /// Shorthand used for every runtime check.
    Policy(CheckPolicyJson),
    /// Structured runtime checks.
    Checks(RuntimeChecksObjectJson),
}

impl From<RuntimeChecksJson> for RuntimeChecks {
    fn from(value: RuntimeChecksJson) -> Self {
        match value {
            RuntimeChecksJson::Policy(policy) => Self::all(CheckPolicy::from(policy)),
            RuntimeChecksJson::Checks(checks) => RuntimeChecks::from(checks),
        }
    }
}

/// Structured runtime checks for JSON deserialization.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeChecksObjectJson {
    /// Default policy for checks without an explicit override.
    pub default: Option<CheckPolicyJson>,
    /// Integer overflow check policy.
    pub overflow: Option<CheckPolicyJson>,
    /// Bounds check policy for array and slice accesses.
    pub bounds: Option<CheckPolicyJson>,
    /// Null check policy for reference operations.
    pub null: Option<CheckPolicyJson>,
    /// Division check policy for divide and remainder operations.
    pub division: Option<CheckPolicyJson>,
    /// Shift range check policy.
    pub shift: Option<CheckPolicyJson>,
    /// Check failure behavior.
    pub failure: Option<CheckFailurePolicyJson>,
}

impl From<RuntimeChecksObjectJson> for RuntimeChecks {
    fn from(value: RuntimeChecksObjectJson) -> Self {
        let default = value.default.map(CheckPolicy::from).unwrap_or_default();

        Self {
            overflow: value.overflow.map(CheckPolicy::from).unwrap_or(default),
            bounds: value.bounds.map(CheckPolicy::from).unwrap_or(default),
            null: value.null.map(CheckPolicy::from).unwrap_or(default),
            division: value.division.map(CheckPolicy::from).unwrap_or(default),
            shift: value.shift.map(CheckPolicy::from).unwrap_or(default),
            failure: value
                .failure
                .map(CheckFailurePolicy::from)
                .unwrap_or_default(),
        }
    }
}

/// Check failure behavior for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CheckFailurePolicyJson {
    /// Trap immediately on a failed check.
    Trap,
    /// Trigger a panic on a failed check.
    Panic,
    /// Abort execution on a failed check.
    Abort,
}

impl From<CheckFailurePolicyJson> for CheckFailurePolicy {
    fn from(value: CheckFailurePolicyJson) -> Self {
        match value {
            CheckFailurePolicyJson::Trap => CheckFailurePolicy::Trap,
            CheckFailurePolicyJson::Panic => CheckFailurePolicy::Panic,
            CheckFailurePolicyJson::Abort => CheckFailurePolicy::Abort,
        }
    }
}
