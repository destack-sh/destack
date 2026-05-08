use serde::{Deserialize, Serialize};

/// Execution mode for runtime scheduling and replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Fast execution without determinism guarantees.
    #[default]
    Fast,
    /// Deterministic scheduling with controlled randomness.
    Deterministic,
    /// Record external effects for replay.
    Record,
    /// Replay external effects from the log.
    Replay,
}

impl std::str::FromStr for ExecutionMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "fast" => Ok(Self::Fast),
            "deterministic" => Ok(Self::Deterministic),
            "record" => Ok(Self::Record),
            "replay" => Ok(Self::Replay),
            _ => Err(()),
        }
    }
}

impl ExecutionMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Replay payload selection for record/replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ReplayPayloadMode {
    /// Record only the result value.
    #[default]
    ResultsOnly,
    /// Record arguments and results for verification.
    ArgumentsAndResults,
}

impl std::str::FromStr for ReplayPayloadMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "results" | "results_only" => Ok(Self::ResultsOnly),
            "args" | "args_and_results" => Ok(Self::ArgumentsAndResults),
            _ => Err(()),
        }
    }
}

impl ReplayPayloadMode {
    /// Parse from a string value.
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

/// Execution mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ExecutionModeJson {
    /// Fast execution without determinism guarantees.
    Fast,
    /// Deterministic scheduling with controlled randomness.
    Deterministic,
    /// Record external effects for replay.
    Record,
    /// Replay external effects from the log.
    Replay,
}

impl From<ExecutionModeJson> for ExecutionMode {
    fn from(value: ExecutionModeJson) -> Self {
        match value {
            ExecutionModeJson::Fast => ExecutionMode::Fast,
            ExecutionModeJson::Deterministic => ExecutionMode::Deterministic,
            ExecutionModeJson::Record => ExecutionMode::Record,
            ExecutionModeJson::Replay => ExecutionMode::Replay,
        }
    }
}

/// Replay payload mode for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ReplayPayloadModeJson {
    /// Record only the result value.
    ResultsOnly,
    /// Record arguments and results for verification.
    ArgumentsAndResults,
}

impl From<ReplayPayloadModeJson> for ReplayPayloadMode {
    fn from(value: ReplayPayloadModeJson) -> Self {
        match value {
            ReplayPayloadModeJson::ResultsOnly => ReplayPayloadMode::ResultsOnly,
            ReplayPayloadModeJson::ArgumentsAndResults => ReplayPayloadMode::ArgumentsAndResults,
        }
    }
}
