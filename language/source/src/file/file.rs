use crate::{FileType, Span, Uri};

/// The id of a File.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

impl FileId {
    /// Turn a u32 into a FileId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// The source of a File.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileSource {
    /// The file is read from the system.
    System,
    /// The file is read from memory.
    Memory,
}

/// File with content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// The id of the File.
    pub id: FileId,
    /// The name of the source (usually the last segment of the URI).
    pub name: String,
    /// The URI of the File.
    pub uri: Uri,
    /// The type of file.
    pub ty: FileType,
    /// The length of the File in bytes.
    pub len: u32,
    /// The content of the File.
    pub content: FileContent,
    /// Start line byte offsets for fast line -> byte position lookup.
    pub line_start_offsets: Option<Vec<u32>>,
}

/// The content of a File.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileContent {
    /// Text content.
    Text { content: String },
    /// JSON content.
    Json {
        content: String,
        value: serde_json::Value,
    },
    /// Binary content.
    Binary { content: Vec<u8> },
    /// Content not yet loaded.
    Unloaded,
}

impl File {
    /// Create an empty source in some format.
    pub fn empty_text_with_type(ty: FileType) -> Self {
        Self::from_text(
            FileId::new(0),
            "<empty>".to_string(),
            Uri::from_string("<empty>"),
            ty,
            String::new(),
        )
    }

    /// Create an empty source.
    pub fn empty_text_with_id(ty: FileType, id: FileId, uri: Uri) -> Self {
        let name = uri
            .last_segment()
            .unwrap_or_else(|| uri.as_ref())
            .to_string();
        Self::from_text(id, name, uri, ty, String::new())
    }

    /// Precompute line start byte offsets for O(1) line.
    fn precompute_line_start_offsets(content: &str) -> Vec<u32> {
        let mut line_start_offsets = vec![0];
        for (i, ch) in content.char_indices() {
            if ch == '\n' {
                line_start_offsets.push(i as u32 + 1);
            }
        }
        line_start_offsets
    }

    /// Create a new File.
    pub fn from_text(id: FileId, name: String, uri: Uri, ty: FileType, content: String) -> Self {
        let len = content.len() as u32;
        let line_start_offsets = Self::precompute_line_start_offsets(&content);
        Self {
            id,
            name,
            uri,
            ty,
            content: FileContent::Text { content },
            len,
            line_start_offsets: Some(line_start_offsets),
        }
    }

    /// Create a new file from text as JSON.
    pub fn from_text_as_json(
        id: FileId,
        name: String,
        uri: Uri,
        ty: FileType,
        content: String,
    ) -> Result<Self, serde_json::Error> {
        let json = serde_json::from_str(&content)?;
        let len = content.len() as u32;
        let line_start_offsets = Self::precompute_line_start_offsets(&content);
        let file = Self {
            id,
            name,
            uri,
            ty,
            content: FileContent::Json {
                content,
                value: json,
            },
            len,
            line_start_offsets: Some(line_start_offsets),
        };
        Ok(file)
    }

    /// Get the text content of the File (empty if not text).
    #[inline]
    pub fn text(&self) -> &str {
        if let FileContent::Text { content } = &self.content {
            content
        } else {
            ""
        }
    }

    /// Get the string slice for a given span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> Option<&str> {
        match &self.content {
            FileContent::Text { content } => Some(&content[span.start as usize..span.end as usize]),
            FileContent::Json { content, .. } => {
                Some(&content[span.start as usize..span.end as usize])
            }
            FileContent::Binary { .. } => None,
            FileContent::Unloaded => None,
        }
    }

    /// Get a line as a string slice by 0-based index.
    #[inline]
    pub fn get_line_str(&self, line_index: u32) -> Option<&str> {
        self.get_line_span(line_index)
            .and_then(|span| self.get_span_str(span))
    }

    /// Get the byte bounds (start, end-exclusive) for a line by 0-based index.
    /// Uses precomputed offsets for O(1) performance.
    #[inline]
    pub fn get_line_span(&self, line_index: u32) -> Option<Span> {
        let Some(line_start_offsets) = &self.line_start_offsets else {
            return None;
        };
        let line_start = *line_start_offsets.get(line_index as usize)?;
        let line_end = if let Some(&next_start) = line_start_offsets.get(line_index as usize + 1) {
            // subtract 1 to exclude the newline character
            next_start - 1
        } else {
            // last line, goes to end of content
            self.len
        };
        Some(Span::new(self.id, line_start, line_end))
    }

    /// Get the line number and column for a given byte index.
    /// Returns (line_index, column_index), both 0-based.
    /// Uses binary search for O(log n) performance.
    pub fn get_position(&self, byte_index: u32) -> Option<(u32, u32)> {
        let Some(line_start_offsets) = &self.line_start_offsets else {
            return None;
        };
        if byte_index > self.len {
            return None;
        };

        // binary search to find the line containing this byte index
        let line_index = match line_start_offsets.binary_search(&byte_index) {
            Ok(exact_match) => exact_match as u32,
            Err(insertion_point) => {
                if insertion_point == 0 {
                    0
                } else {
                    (insertion_point - 1) as u32
                }
            }
        };

        let line_start = line_start_offsets[line_index as usize];
        let column = byte_index - line_start;

        Some((line_index, column))
    }

    /// Get the byte position for a given line and column.
    /// Returns the byte index, or None if the position is invalid.
    /// Uses precomputed line offsets for O(1) performance.
    pub fn get_byte_position(&self, line_index: u32, column: u32) -> Option<u32> {
        let Some(line_start_offsets) = &self.line_start_offsets else {
            return None;
        };
        let line_start = *line_start_offsets.get(line_index as usize)?;
        let byte_pos = line_start + column;

        // check if the position is within bounds
        if byte_pos > self.len {
            return None;
        }

        // check if the column is within the line bounds
        let next_line_start = line_start_offsets
            .get(line_index as usize + 1)
            .copied()
            .unwrap_or(self.len + 1); // +1 to account for potential newline

        if byte_pos >= next_line_start {
            return None;
        }

        Some(byte_pos)
    }

    /// Get the number of lines in the source (at least 1 for empty content).
    pub fn line_count(&self) -> u32 {
        if let Some(line_start_offsets) = &self.line_start_offsets {
            line_start_offsets.len() as u32
        } else {
            0
        }
    }

    /// Get the span of the file.
    pub fn span(&self) -> Span {
        Span::new(self.id, 0, self.len)
    }
}
