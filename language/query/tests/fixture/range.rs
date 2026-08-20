use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use super::{display_query_path, parse_query_anchor};

/// One file-qualified named source range in a query fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FixtureRange {
    /// The workspace-relative file path.
    pub(super) file: PathBuf,
    /// The anchor name within the file.
    pub(super) anchor: String,
}

impl FixtureRange {
    /// Parse one file-qualified named source range.
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        let (path, anchor) = parse_query_anchor(value, "range")?;
        if anchor.ends_with("@start") || anchor.ends_with("@end") {
            return Err(format!("query range '{value}' cannot select one edge"));
        }

        Ok(Self {
            file: PathBuf::from(path),
            anchor: anchor.to_string(),
        })
    }
}

impl Display for FixtureRange {
    /// Format one file-qualified named source range.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}#{}",
            display_query_path(&self.file),
            self.anchor
        )
    }
}
