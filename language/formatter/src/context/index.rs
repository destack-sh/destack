use crate::file::comment_text_has_ignore_directive_marker;

use destack_dir::Comment;
use destack_source::File;

/// Immutable source lookups for one formatter pass.
#[derive(Debug)]
pub struct FormatSourceIndex {
    /// Newline byte offsets in file text.
    newline_offsets: Vec<u32>,
    /// Whether file text contains formatter ignore directive markers.
    has_ignore_directive_markers: bool,
}

impl FormatSourceIndex {
    /// Build source lookups for one parsed file.
    pub(crate) fn new(file: &File, comments: &[Comment]) -> Self {
        let newline_offsets = collect_newline_offsets(file);
        let has_ignore_directive_markers = comments
            .iter()
            .any(|comment| comment_text_has_ignore_directive_marker(file.span_str(comment.span)));

        Self {
            newline_offsets,
            has_ignore_directive_markers,
        }
    }

    /// Return byte offsets of all newline characters in the source file.
    #[inline]
    pub(crate) fn newline_offsets(&self) -> &[u32] {
        &self.newline_offsets
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
