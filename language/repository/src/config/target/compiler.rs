use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{CompilerRestrictions, Derive};

/// Target compiler behavior options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct TargetCompilerOptions {
    /// Target tree tag builder override.
    pub tree: Option<String>,
    /// Target well-known derives.
    pub derive: Vec<Derive>,
    /// Static semantic restrictions for this target.
    pub restrictions: CompilerRestrictions,
    /// Optimization level.
    pub optimize: OptimizeLevel,
    /// Generated safety check policies.
    pub checks: CheckPolicySet,
}

/// Optimization level for builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum OptimizeLevel {
    /// Debug build level with no optimization.
    #[default]
    O0,
    /// Fast local optimization level.
    O1,
    /// Standard release optimization level.
    O2,
    /// Aggressive release optimization level.
    O3,
    /// Maximum TS++ optimization level.
    O4,
}

impl From<u8> for OptimizeLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => Self::O0,
            1 => Self::O1,
            2 => Self::O2,
            3 => Self::O3,
            _ => Self::O4,
        }
    }
}

impl From<OptimizeLevel> for u8 {
    fn from(level: OptimizeLevel) -> Self {
        match level {
            OptimizeLevel::O0 => 0,
            OptimizeLevel::O1 => 1,
            OptimizeLevel::O2 => 2,
            OptimizeLevel::O3 => 3,
            OptimizeLevel::O4 => 4,
        }
    }
}

/// Generated check policy set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct CheckPolicySet {
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

impl CheckPolicySet {
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

/// Generated safety check policy.
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

impl FromStr for CheckPolicy {
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

/// Check failure behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CheckFailurePolicy {
    /// Trigger a panic on a failed check.
    #[default]
    Panic,
    /// Abort execution on a failed check.
    Abort,
}

impl FromStr for CheckFailurePolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
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
