use serde::{Deserialize, Serialize};

/// Execution mode for runtime scheduling and replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Strict execution through runtime-owned facts and effect boundaries.
    #[default]
    Strict,
    /// Fast execution without determinism guarantees.
    Fast,
    /// Record external effects for replay.
    Record,
    /// Replay external effects from the log.
    Replay,
}

/// Runtime execution configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionOptions {
    /// Execution mode for scheduling and effect handling.
    pub mode: ExecutionMode,
}

impl std::str::FromStr for ExecutionMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "strict" => Ok(Self::Strict),
            "fast" => Ok(Self::Fast),
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
