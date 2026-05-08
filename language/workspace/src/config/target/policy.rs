use serde::Deserialize;

/// Integer overflow checking policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OverflowCheckPolicy {
    /// Always emit overflow checks.
    Always,
    /// Emit overflow checks only in debug builds.
    #[default]
    Debug,
    /// Never emit overflow checks.
    Never,
}

impl std::str::FromStr for OverflowCheckPolicy {
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

impl OverflowCheckPolicy {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Floating point math optimization policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FloatMathPolicy {
    /// Strict IEEE semantics.
    #[default]
    Strict,
    /// Allow reassociation and algebraic simplifications.
    Reassociate,
    /// Enable fast math optimizations (assume no NaN, inf, or signed zero).
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

/// Safety preset that configures runtime checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SafetyPreset {
    /// Debug safety mode with checks always enabled.
    Debug,
    /// Release mode with checks enabled.
    ReleaseSafe,
    /// Release mode with checks disabled for maximum speed.
    ReleaseFast,
    /// Release mode with checks disabled and size focused settings.
    ReleaseSmall,
}

impl std::str::FromStr for SafetyPreset {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "debug" => Ok(Self::Debug),
            "release_safe" | "releasesafe" | "safe" => Ok(Self::ReleaseSafe),
            "release_fast" | "releasefast" | "fast" => Ok(Self::ReleaseFast),
            "release_small" | "releasesmall" | "small" => Ok(Self::ReleaseSmall),
            _ => Err(()),
        }
    }
}

impl SafetyPreset {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }

    /// Return the runtime check policies for this preset.
    pub fn runtime_check_policies(self) -> RuntimeCheckPolicies {
        match self {
            SafetyPreset::Debug | SafetyPreset::ReleaseSafe => RuntimeCheckPolicies {
                overflow: OverflowCheckPolicy::Always,
                bounds: BoundsCheckPolicy::Always,
                null: NullCheckPolicy::Always,
                division: DivisionCheckPolicy::Always,
                shift: ShiftCheckPolicy::Always,
            },
            SafetyPreset::ReleaseFast | SafetyPreset::ReleaseSmall => RuntimeCheckPolicies {
                overflow: OverflowCheckPolicy::Never,
                bounds: BoundsCheckPolicy::Never,
                null: NullCheckPolicy::Never,
                division: DivisionCheckPolicy::Never,
                shift: ShiftCheckPolicy::Never,
            },
        }
    }

    /// Return the float math policy for this preset.
    pub fn float_math_policy(self) -> FloatMathPolicy {
        match self {
            SafetyPreset::Debug | SafetyPreset::ReleaseSafe => FloatMathPolicy::Strict,
            SafetyPreset::ReleaseFast | SafetyPreset::ReleaseSmall => FloatMathPolicy::Fast,
        }
    }
}

/// Runtime check policy bundle for safety presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RuntimeCheckPolicies {
    /// Overflow check policy.
    pub overflow: OverflowCheckPolicy,
    /// Bounds check policy.
    pub bounds: BoundsCheckPolicy,
    /// Null check policy.
    pub null: NullCheckPolicy,
    /// Division check policy.
    pub division: DivisionCheckPolicy,
    /// Shift range check policy.
    pub shift: ShiftCheckPolicy,
}

/// Bounds check policy for array and slice accesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BoundsCheckPolicy {
    /// Always emit bounds checks.
    Always,
    /// Emit bounds checks only in debug builds.
    #[default]
    Debug,
    /// Never emit bounds checks (unsafe, fastest).
    Never,
}

impl std::str::FromStr for BoundsCheckPolicy {
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

impl BoundsCheckPolicy {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Null check policy for reference operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NullCheckPolicy {
    /// Always emit null checks.
    Always,
    /// Emit null checks only in debug builds.
    #[default]
    Debug,
    /// Never emit null checks (unsafe, fastest).
    Never,
}

impl std::str::FromStr for NullCheckPolicy {
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

impl NullCheckPolicy {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Division check policy for divide and remainder operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DivisionCheckPolicy {
    /// Always emit division checks.
    Always,
    /// Emit division checks only in debug builds.
    #[default]
    Debug,
    /// Never emit division checks (unsafe, fastest).
    Never,
}

impl std::str::FromStr for DivisionCheckPolicy {
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

impl DivisionCheckPolicy {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Shift range check policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShiftCheckPolicy {
    /// Always emit shift range checks.
    Always,
    /// Emit shift range checks only in debug builds.
    #[default]
    Debug,
    /// Never emit shift range checks (unsafe, fastest).
    Never,
}

impl std::str::FromStr for ShiftCheckPolicy {
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

impl ShiftCheckPolicy {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
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

/// Safety preset for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum SafetyPresetJson {
    /// Debug safety mode with checks enabled.
    #[serde(alias = "debug")]
    Debug,
    /// Release mode with checks enabled.
    #[serde(alias = "releaseSafe")]
    #[serde(alias = "safe")]
    ReleaseSafe,
    /// Release mode with checks disabled.
    #[serde(alias = "releaseFast")]
    #[serde(alias = "fast")]
    ReleaseFast,
    /// Release mode with checks disabled and size focused settings.
    #[serde(alias = "releaseSmall")]
    #[serde(alias = "small")]
    ReleaseSmall,
}

impl From<SafetyPresetJson> for SafetyPreset {
    fn from(value: SafetyPresetJson) -> Self {
        match value {
            SafetyPresetJson::Debug => SafetyPreset::Debug,
            SafetyPresetJson::ReleaseSafe => SafetyPreset::ReleaseSafe,
            SafetyPresetJson::ReleaseFast => SafetyPreset::ReleaseFast,
            SafetyPresetJson::ReleaseSmall => SafetyPreset::ReleaseSmall,
        }
    }
}

/// Floating point math policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum FloatMathPolicyJson {
    /// Strict IEEE semantics.
    #[serde(alias = "strict")]
    Strict,
    /// Permit reassociation but preserve NaNs and infinities.
    #[serde(alias = "reassoc", alias = "reassociate")]
    Reassociate,
    /// Enable fast math optimizations.
    #[serde(alias = "fast")]
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

/// Overflow checking policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum OverflowCheckPolicyJson {
    /// Always check for overflow.
    Always,
    /// Debug builds check; release does not.
    #[serde(alias = "debug")]
    Debug,
    /// Never check for overflow.
    Never,
}

impl From<OverflowCheckPolicyJson> for OverflowCheckPolicy {
    fn from(value: OverflowCheckPolicyJson) -> Self {
        match value {
            OverflowCheckPolicyJson::Always => OverflowCheckPolicy::Always,
            OverflowCheckPolicyJson::Debug => OverflowCheckPolicy::Debug,
            OverflowCheckPolicyJson::Never => OverflowCheckPolicy::Never,
        }
    }
}

/// Bounds checking policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BoundsCheckPolicyJson {
    /// Always check bounds.
    Always,
    /// Debug builds check; release does not.
    #[serde(alias = "debug")]
    Debug,
    /// Never check bounds.
    Never,
}

impl From<BoundsCheckPolicyJson> for BoundsCheckPolicy {
    fn from(value: BoundsCheckPolicyJson) -> Self {
        match value {
            BoundsCheckPolicyJson::Always => BoundsCheckPolicy::Always,
            BoundsCheckPolicyJson::Debug => BoundsCheckPolicy::Debug,
            BoundsCheckPolicyJson::Never => BoundsCheckPolicy::Never,
        }
    }
}

/// Null checking policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum NullCheckPolicyJson {
    /// Always check for null.
    Always,
    /// Debug builds check; release does not.
    #[serde(alias = "debug")]
    Debug,
    /// Never check for null.
    Never,
}

impl From<NullCheckPolicyJson> for NullCheckPolicy {
    fn from(value: NullCheckPolicyJson) -> Self {
        match value {
            NullCheckPolicyJson::Always => NullCheckPolicy::Always,
            NullCheckPolicyJson::Debug => NullCheckPolicy::Debug,
            NullCheckPolicyJson::Never => NullCheckPolicy::Never,
        }
    }
}

/// Division checking policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DivisionCheckPolicyJson {
    /// Always check for divide-by-zero.
    Always,
    /// Debug builds check; release does not.
    #[serde(alias = "debug")]
    Debug,
    /// Never check for divide-by-zero.
    Never,
}

impl From<DivisionCheckPolicyJson> for DivisionCheckPolicy {
    fn from(value: DivisionCheckPolicyJson) -> Self {
        match value {
            DivisionCheckPolicyJson::Always => DivisionCheckPolicy::Always,
            DivisionCheckPolicyJson::Debug => DivisionCheckPolicy::Debug,
            DivisionCheckPolicyJson::Never => DivisionCheckPolicy::Never,
        }
    }
}

/// Shift range check policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ShiftCheckPolicyJson {
    /// Always check for invalid shift ranges.
    Always,
    /// Debug builds check; release does not.
    #[serde(alias = "debug")]
    Debug,
    /// Never check for invalid shift ranges.
    Never,
}

impl From<ShiftCheckPolicyJson> for ShiftCheckPolicy {
    fn from(value: ShiftCheckPolicyJson) -> Self {
        match value {
            ShiftCheckPolicyJson::Always => ShiftCheckPolicy::Always,
            ShiftCheckPolicyJson::Debug => ShiftCheckPolicy::Debug,
            ShiftCheckPolicyJson::Never => ShiftCheckPolicy::Never,
        }
    }
}

/// Check failure behavior for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CheckFailurePolicyJson {
    /// Trap immediately on a failed check.
    #[serde(alias = "trap")]
    Trap,
    /// Trigger a panic on a failed check.
    #[serde(alias = "panic")]
    Panic,
    /// Abort execution on a failed check.
    #[serde(alias = "abort")]
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
