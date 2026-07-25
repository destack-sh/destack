use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use super::{FixtureAnchor, display_query_path, parse_query_anchor};

/// One file-qualified named source position in a query fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FixturePosition {
    /// The workspace-relative file path.
    pub(super) file: PathBuf,
    /// The anchor name within the file.
    pub(super) anchor: String,
    /// The selected edge of the anchor.
    edge: FixturePositionEdge,
}

impl FixturePosition {
    /// Parse one file-qualified named source position.
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        let (path, anchor) = parse_query_anchor(value, "position")?;
        let (anchor, edge) = if let Some(anchor) = anchor.strip_suffix("@start") {
            (anchor, FixturePositionEdge::Start)
        } else if let Some(anchor) = anchor.strip_suffix("@end") {
            (anchor, FixturePositionEdge::End)
        } else {
            (anchor, FixturePositionEdge::Start)
        };
        if anchor.is_empty() {
            return Err(format!("query position '{value}' has no anchor name"));
        }

        Ok(Self {
            file: PathBuf::from(path),
            anchor: anchor.to_string(),
            edge,
        })
    }

    /// Return the selected byte offset in one resolved anchor.
    pub(super) fn offset(&self, anchor: FixtureAnchor) -> u32 {
        match self.edge {
            FixturePositionEdge::Start => anchor.start,
            FixturePositionEdge::End => anchor.end,
        }
    }
}

impl Display for FixturePosition {
    /// Format one file-qualified named source position.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}#{}{}",
            display_query_path(&self.file),
            self.anchor,
            self.edge.suffix()
        )
    }
}

/// One selected edge of a source anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FixturePositionEdge {
    /// The inclusive anchor start.
    Start,
    /// The exclusive anchor end.
    End,
}

impl FixturePositionEdge {
    /// Return the fixture suffix for this edge.
    pub(super) fn suffix(self) -> &'static str {
        match self {
            Self::Start => "",
            Self::End => "@end",
        }
    }
}
