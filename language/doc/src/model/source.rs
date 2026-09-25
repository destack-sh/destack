use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// One stable source location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct SourceReference {
    /// The stable logical source path when filesystem backed.
    pub path: Option<String>,
    /// The one-based source line.
    pub line: u32,
    /// The one-based byte column.
    pub column: u32,
    /// The one-based final source line.
    pub end_line: u32,
    /// The one-based final byte column.
    pub end_column: u32,
}
