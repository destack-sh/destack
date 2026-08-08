use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Filesystem destination for one target output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    /// Output directory.
    pub directory: PathBuf,
    /// Output file for one-file outputs.
    pub file: Option<PathBuf>,
}

impl Default for Destination {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("dist"),
            file: None,
        }
    }
}
