use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use super::{QueryAnchor, display_query_path, parse_query_anchor};

/// One file-qualified named source position in a query fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct QueryPosition {
    /// The workspace-relative file path.
    pub(super) file: PathBuf,
    /// The anchor name within the file.
    pub(super) anchor: String,
    /// The selected edge of the anchor.
    edge: QueryPositionEdge,
}

impl QueryPosition {
    /// Parse one file-qualified named source position.
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        let (path, anchor) = parse_query_anchor(value, "position")?;
        let (anchor, edge) = if let Some(anchor) = anchor.strip_suffix("@start") {
            (anchor, QueryPositionEdge::Start)
        } else if let Some(anchor) = anchor.strip_suffix("@end") {
            (anchor, QueryPositionEdge::End)
        } else {
            (anchor, QueryPositionEdge::Start)
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
    pub(super) fn offset(&self, anchor: QueryAnchor) -> u32 {
        match self.edge {
            QueryPositionEdge::Start => anchor.start,
            QueryPositionEdge::End => anchor.end,
        }
    }
}

impl Display for QueryPosition {
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
pub(super) enum QueryPositionEdge {
    /// The inclusive anchor start.
    Start,
    /// The exclusive anchor end.
    End,
}

impl QueryPositionEdge {
    /// Return the fixture suffix for this edge.
    pub(super) fn suffix(self) -> &'static str {
        match self {
            Self::Start => "",
            Self::End => "@end",
        }
    }
}
