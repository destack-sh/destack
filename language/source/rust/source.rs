use crate::{SourceFormat, Span, Uri};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(pub u32);

impl SourceId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A text "source" (like a file or a standalone string).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// The id of the SourceFile.
    pub id: SourceId,
    /// The name of the source (usually the last segment of the URI).
    pub name: String,
    /// The URI of the SourceFile.
    pub uri: Uri,
    /// The format of the source file (ds or dst).
    pub format: SourceFormat,
    /// The content of the source file.
    pub content: String,
    /// The length of the SourceFile in bytes.
    pub len: u32,
    /// Start line byte offsets for fast line -> byte position lookup.
    pub line_start_offsets: Vec<u32>,
}

impl Source {
    /// Create an empty source in some format.
    pub fn empty(format: SourceFormat) -> Self {
        Self::from_string(
            SourceId::new(0),
            "<empty>".to_string(),
            Uri::from_string("<empty>"),
            format,
            String::new(),
        )
    }

    /// Create a new Source.
    /// Precomputes indexing information immediately.
    pub fn from_string(
        id: SourceId,
        name: String,
        uri: Uri,
        format: SourceFormat,
        content: String,
    ) -> Self {
        let len = content.len() as u32;

        // precompute line start byte offsets for O(1) line -> byte lookup
        let mut line_start_offsets = vec![0];
        for (i, ch) in content.char_indices() {
            if ch == '\n' {
                line_start_offsets.push(i as u32 + 1);
            }
        }

        Self {
            id,
            name,
            uri,
            format,
            content,
            len,
            line_start_offsets,
        }
    }

    /// Get the string slice for a given span.
    pub fn get_span_str(&self, span: Span) -> &str {
        &self.content[span.start as usize..span.end as usize]
    }

    /// Get the line number and column for a given byte index.
    /// Returns (line_index, column_index), both 0-based.
    /// Uses binary search for O(log n) performance.
    pub fn get_position(&self, byte_index: u32) -> Option<(u32, u32)> {
        if byte_index > self.len {
            return None;
        }

        // binary search to find the line containing this byte index
        let line_index = match self.line_start_offsets.binary_search(&byte_index) {
            Ok(exact_match) => exact_match as u32,
            Err(insertion_point) => {
                if insertion_point == 0 {
                    0
                } else {
                    (insertion_point - 1) as u32
                }
            }
        };

        let line_start = self.line_start_offsets[line_index as usize];
        let column = byte_index - line_start;

        Some((line_index, column))
    }

    /// Get the byte position for a given line and column.
    /// Returns the byte index, or None if the position is invalid.
    /// Uses precomputed line offsets for O(1) performance.
    pub fn get_byte_position(&self, line_index: u32, column: u32) -> Option<u32> {
        let line_start = *self.line_start_offsets.get(line_index as usize)?;
        let byte_pos = line_start + column;

        // check if the position is within bounds
        if byte_pos > self.len {
            return None;
        }

        // check if the column is within the line bounds
        let next_line_start = self
            .line_start_offsets
            .get(line_index as usize + 1)
            .copied()
            .unwrap_or(self.len + 1); // +1 to account for potential newline

        if byte_pos >= next_line_start {
            return None;
        }

        Some(byte_pos)
    }

    /// Compute the number of lines in the source (at least 1 for empty content).
    pub fn line_count(&self) -> u32 {
        self.line_start_offsets.len() as u32
    }

    /// Get the byte bounds (start, end-exclusive) for a line by 0-based index.
    /// Uses precomputed offsets for O(1) performance.
    pub fn get_line_bounds(&self, line_index: u32) -> Option<(usize, usize)> {
        let line_start = *self.line_start_offsets.get(line_index as usize)? as usize;
        let line_end =
            if let Some(&next_start) = self.line_start_offsets.get(line_index as usize + 1) {
                // subtract 1 to exclude the newline character
                (next_start - 1) as usize
            } else {
                // last line, goes to end of content
                self.content.len()
            };
        Some((line_start, line_end))
    }

    /// Get a line as a string slice by 0-based index.
    pub fn get_line(&self, line_index: u32) -> Option<&str> {
        self.get_line_bounds(line_index)
            .and_then(|(s, e)| self.content.get(s..e))
    }

    /// Get entire source Span.
    pub fn whole_span(&self) -> Span {
        Span::new(self.id, 0, self.len)
    }
}
