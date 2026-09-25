use crate::file::comment_text_has_ignore_directive_marker;

use tspp_dir::{Comment, Tree};
use tspp_source::File;

/// Immutable source lookups for one formatter pass.
#[derive(Debug)]
pub struct FormatSourceIndex {
    /// Newline byte offsets in file text.
    newline_offsets: Vec<u32>,
    /// Documented node IDs ordered by documentation start.
    documentation_nodes: Vec<(u32, u32)>,
    /// Whether file text contains formatter ignore directive markers.
    has_ignore_directive_markers: bool,
}

impl FormatSourceIndex {
    /// Build source lookups for one parsed file.
    pub(crate) fn new(file: &File, tree: &Tree, comments: &[Comment]) -> Self {
        // index source positions
        let newline_offsets = collect_newline_offsets(file);
        let mut documentation_nodes = tree
            .iter_documentation()
            .map(|(node_id, documentation)| (documentation.span.start, node_id))
            .collect::<Vec<_>>();
        documentation_nodes.sort_unstable();

        // detect formatter directives
        let has_ignore_directive_markers = comments
            .iter()
            .any(|comment| comment_text_has_ignore_directive_marker(file.span_str(comment.span)));

        Self {
            newline_offsets,
            documentation_nodes,
            has_ignore_directive_markers,
        }
    }

    /// Return byte offsets of all newline characters in the source file.
    #[inline]
    pub(crate) fn newline_offsets(&self) -> &[u32] {
        &self.newline_offsets
    }

    /// Return documented node IDs with one exact documentation start.
    pub(crate) fn documentation_nodes_at(&self, start: u32) -> &[(u32, u32)] {
        // locate the equal start range
        let first = self
            .documentation_nodes
            .partition_point(|(candidate, _)| *candidate < start);
        let count =
            self.documentation_nodes[first..].partition_point(|(candidate, _)| *candidate == start);

        &self.documentation_nodes[first..first + count]
    }

    /// Return whether file text contains formatter ignore directive markers.
    #[inline]
    pub(crate) fn has_ignore_directive_markers(&self) -> bool {
        self.has_ignore_directive_markers
    }
}

/// Collect all newline byte offsets in one source file.
fn collect_newline_offsets(file: &File) -> Vec<u32> {
    if let Some(line_start_offsets) = file.line_start_offsets() {
        return line_start_offsets
            .iter()
            .copied()
            .skip(1)
            .map(|line_start| line_start - 1)
            .collect();
    }

    file.text()
        .bytes()
        .enumerate()
        .filter_map(|(index, byte)| (byte == b'\n').then_some(index as u32))
        .collect()
}
