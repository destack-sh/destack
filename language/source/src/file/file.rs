use std::path::PathBuf;

use crate::{FileType, Span, Uri, strip_json};

/// The id of a File.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

impl std::fmt::Debug for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Display for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}
impl FileId {
    /// Turn a u32 into a FileId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Version of a file's content (increments on each change).
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct FileVersion(pub u64);

impl std::fmt::Debug for FileVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl std::fmt::Display for FileVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl FileVersion {
    /// Initial version.
    pub const INITIAL: Self = Self(0);

    /// Create a new FileVersion.
    pub fn new(version: u64) -> Self {
        Self(version)
    }

    /// Increment the version, returning the new value.
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// File with content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// The id of the File.
    pub id: FileId,
    /// The version of the File (increments on each change).
    pub version: FileVersion,
    /// The name of the source (usually the last segment of the URI).
    pub name: String,
    /// The URI of the File.
    pub uri: Uri,
    /// The path to the File (may be invalid as a path).
    pub path: Option<PathBuf>,
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
    /// Create a blank unloaded File (content not yet loaded).
    pub fn unloaded(
        id: FileId,
        name: String,
        uri: Uri,
        path: Option<PathBuf>,
        ty: FileType,
    ) -> Self {
        Self {
            id,
            version: FileVersion::INITIAL,
            name,
            uri,
            path,
            ty,
            len: 0,
            content: FileContent::Unloaded,
            line_start_offsets: None,
        }
    }

    /// Create an empty source in some format.
    pub fn empty_text(ty: FileType) -> Self {
        Self::from_text(
            FileId::new(0),
            "<empty>".to_string(),
            Uri::from_string("<empty>"),
            None,
            ty,
            String::new(),
        )
    }

    /// Check if this file has content loaded.
    pub fn is_loaded(&self) -> bool {
        !matches!(self.content, FileContent::Unloaded)
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

    /// Normalize line endings to `\n` (LF).
    ///
    /// Converts `\r\n` (CRLF) and standalone `\r` (CR) to `\n` (LF).
    fn normalize_line_endings(content: String) -> String {
        if content.contains('\r') {
            content.replace("\r\n", "\n").replace('\r', "\n")
        } else {
            content
        }
    }

    /// Create a new File.
    pub fn from_text(
        id: FileId,
        name: String,
        uri: Uri,
        path: Option<PathBuf>,
        ty: FileType,
        content: String,
    ) -> Self {
        let content = Self::normalize_line_endings(content);
        let len = content.len() as u32;
        let line_start_offsets = Self::precompute_line_start_offsets(&content);
        Self {
            id,
            version: FileVersion::INITIAL,
            name,
            uri,
            path,
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
        path: Option<PathBuf>,
        ty: FileType,
        content: String,
    ) -> Result<Self, serde_json::Error> {
        let content = Self::normalize_line_endings(content);
        // strip BOM from content for parsing, but keep original content
        let json_str = content.strip_prefix('\u{feff}').unwrap_or(&content);

        let json = serde_json::from_str(json_str)?;
        let len = content.len() as u32;
        let line_start_offsets = Self::precompute_line_start_offsets(&content);
        let file = Self {
            id,
            version: FileVersion::INITIAL,
            name,
            uri,
            path,
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

    /// Create a new file from text as JSONC (JSON with comments).
    pub fn from_text_as_jsonc(
        id: FileId,
        name: String,
        uri: Uri,
        path: Option<PathBuf>,
        ty: FileType,
        content: String,
    ) -> Result<Self, serde_json::Error> {
        let content = Self::normalize_line_endings(content);
        // strip BOM from content for parsing
        let json_content = content.strip_prefix('\u{feff}').unwrap_or(&content);

        // strip comments from JSONC
        let json_str = strip_json(json_content).map_err(serde_json::Error::io)?;

        // default to empty object if the file is empty
        let json_str = if json_str.trim().is_empty() {
            "{}".to_string()
        } else {
            json_str
        };

        let json = serde_json::from_str(&json_str)?;
        let len = content.len() as u32;
        let line_start_offsets = Self::precompute_line_start_offsets(&content);
        let file = Self {
            id,
            version: FileVersion::INITIAL,
            name,
            uri,
            path,
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

    /// Create a new file from bytes as JSON.
    pub fn from_bytes_as_json(
        id: FileId,
        name: String,
        uri: Uri,
        path: Option<PathBuf>,
        ty: FileType,
        bytes: Vec<u8>,
    ) -> Result<Self, serde_json::Error> {
        let content = String::from_utf8(bytes).unwrap_or_else(|_| String::new());
        Self::from_text_as_json(id, name, uri, path, ty, content)
    }

    /// Create a new file from bytes as JSONC.
    pub fn from_bytes_as_jsonc(
        id: FileId,
        name: String,
        uri: Uri,
        path: Option<PathBuf>,
        ty: FileType,
        bytes: Vec<u8>,
    ) -> Result<Self, serde_json::Error> {
        let content = String::from_utf8(bytes).unwrap_or_else(|_| String::new());
        Self::from_text_as_jsonc(id, name, uri, path, ty, content)
    }

    /// Set the version of the file (builder pattern).
    pub fn with_version(mut self, version: FileVersion) -> Self {
        self.version = version;
        self
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
