use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Vendored dependency resolution options.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Vendor {
    /// Vendored dependency resolution mode.
    pub mode: VendorMode,
    /// Project-owned vendor directory.
    pub path: PathBuf,
    /// Package patterns included in the vendor mirror.
    pub include: Vec<String>,
    /// Package patterns excluded from the vendor mirror.
    pub exclude: Vec<String>,
    /// Whether vendored packages must match the lock.
    pub verify: bool,
}

impl Default for Vendor {
    fn default() -> Self {
        Self {
            mode: VendorMode::Auto,
            path: PathBuf::from("vendor"),
            include: Vec::new(),
            exclude: Vec::new(),
            verify: true,
        }
    }
}

/// Vendored dependency resolution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum VendorMode {
    /// Use the vendor directory when it is complete.
    #[default]
    Auto,
    /// Use vendored packages when present and fall back to normal resolution.
    Prefer,
    /// Require every resolved package to be vendored.
    Require,
    /// Ignore vendored packages.
    Ignore,
}
