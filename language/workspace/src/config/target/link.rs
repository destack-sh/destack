use serde::{Deserialize, Serialize};

/// Link time optimization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum LtoMode {
    /// Use defaults based on optimization level.
    /// Auto enables Thin LTO at O4 and disables LTO at lower levels.
    #[default]
    Auto,
    /// Disable link time optimization.
    None,
    /// Enable Thin LTO at package scope.
    Thin,
    /// Enable Full LTO at program scope.
    Full,
}

impl std::str::FromStr for LtoMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "auto" => Ok(Self::Auto),
            "none" | "off" | "disabled" => Ok(Self::None),
            "thin" | "thinlto" | "thin_lto" => Ok(Self::Thin),
            "full" | "lto" => Ok(Self::Full),
            _ => Err(()),
        }
    }
}

impl LtoMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Symbol stripping policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum StripLevel {
    /// Keep all symbols.
    #[default]
    None,
    /// Strip local symbols.
    Partial,
    /// Strip all symbols.
    Full,
}

impl std::str::FromStr for StripLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "none" => Ok(Self::None),
            "partial" => Ok(Self::Partial),
            "full" | "all" => Ok(Self::Full),
            _ => Err(()),
        }
    }
}

impl StripLevel {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Panic behavior for unrecoverable program failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PanicPolicy {
    /// Terminate the current worker or isolate.
    #[default]
    Worker,
    /// Terminate the whole runtime process.
    Process,
    /// Lower panic to a minimal trap.
    Trap,
    /// Abort immediately without diagnostics.
    Abort,
}

impl std::str::FromStr for PanicPolicy {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "worker" | "isolate" => Ok(Self::Worker),
            "process" | "runtime" => Ok(Self::Process),
            "trap" => Ok(Self::Trap),
            "abort" => Ok(Self::Abort),
            _ => Err(()),
        }
    }
}

impl PanicPolicy {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Link time optimization mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum LtoModeJson {
    /// Choose mode based on optimization level.
    #[serde(alias = "auto")]
    Auto,
    /// Disable link time optimization.
    #[serde(alias = "none")]
    #[serde(alias = "off")]
    #[serde(alias = "disabled")]
    None,
    /// Enable thin link time optimization.
    #[serde(alias = "thin")]
    #[serde(alias = "thinlto")]
    #[serde(alias = "thin_lto")]
    Thin,
    /// Enable full link time optimization.
    #[serde(alias = "full")]
    #[serde(alias = "lto")]
    Full,
}

impl From<LtoModeJson> for LtoMode {
    fn from(value: LtoModeJson) -> Self {
        match value {
            LtoModeJson::Auto => LtoMode::Auto,
            LtoModeJson::None => LtoMode::None,
            LtoModeJson::Thin => LtoMode::Thin,
            LtoModeJson::Full => LtoMode::Full,
        }
    }
}

/// Symbol stripping policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum StripLevelJson {
    /// Keep all symbols.
    #[serde(alias = "none")]
    None,
    /// Strip local symbols.
    #[serde(alias = "locals")]
    Partial,
    /// Strip all symbols.
    #[serde(alias = "all")]
    Full,
}

impl From<StripLevelJson> for StripLevel {
    fn from(value: StripLevelJson) -> Self {
        match value {
            StripLevelJson::None => StripLevel::None,
            StripLevelJson::Partial => StripLevel::Partial,
            StripLevelJson::Full => StripLevel::Full,
        }
    }
}

/// Panic policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum PanicPolicyJson {
    /// Terminate the current worker or isolate.
    Worker,
    /// Terminate the whole runtime process.
    Process,
    /// Lower panic to a minimal trap.
    Trap,
    /// Abort immediately without diagnostics.
    Abort,
}

impl From<PanicPolicyJson> for PanicPolicy {
    fn from(value: PanicPolicyJson) -> Self {
        match value {
            PanicPolicyJson::Worker => PanicPolicy::Worker,
            PanicPolicyJson::Process => PanicPolicy::Process,
            PanicPolicyJson::Trap => PanicPolicy::Trap,
            PanicPolicyJson::Abort => PanicPolicy::Abort,
        }
    }
}
