use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_core::StableHasher;
use serde::{Deserialize, Serialize};

use super::hash::{stable_source_id, stable_source_path};
use crate::{FileType, Span, Uri};

const FILE_LOGICAL_ID_DOMAIN: &[u8] = b"destack.source.file.logical.v1";
const FILE_SOURCE_ID_DOMAIN: &[u8] = b"destack.source.file.source.v1";
const FILE_CONTENT_ID_DOMAIN: &[u8] = b"destack.source.file.content.v1";

/// The id of a File.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FileId(pub u128);

impl std::fmt::Debug for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "f{:032x}", self.0)
    }
}

impl std::fmt::Display for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "f{:032x}", self.0)
    }
}

impl FileId {
    /// Well-known ID for ephemeral files.
    pub const EPHEMERAL: Self = Self(0);

    /// Turn a raw id into a FileId.
    pub const fn new(id: u128) -> Self {
        Self(id)
    }

    /// Create a file id from one logical source path string.
    ///
    /// This should be one repository or import relative path for physical files,
    /// or one explicit namespaced synthetic path for virtual files.
    pub fn from_logical_str(path: &str) -> Self {
        let path = path.replace('\\', "/");
        Self(stable_source_id(FILE_LOGICAL_ID_DOMAIN, &[path.as_bytes()]))
    }

    /// Create a file id from one logical source path.
    pub fn from_logical_path(path: &Path) -> Self {
        let path = stable_source_path(path);

        Self::from_logical_str(&path)
    }

    /// Create a file id from one explicit source payload.
    pub fn from_source_bytes(bytes: &[u8]) -> Self {
        let id = stable_source_id(FILE_SOURCE_ID_DOMAIN, &[bytes]);
        let id = if id == 0 { 1 } else { id };

        Self(id)
    }
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
    /// The path to the File (may be invalid as a path).
    pub path: Option<PathBuf>,
    /// The type of file.
    pub ty: FileType,
    /// The length of the File in bytes.
    pub len: u32,
    /// The shared content entry for the File.
    pub content: Arc<FileContentEntry>,
}

/// The content of a File.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileContent {
    /// Text content.
    Text { content: String },
    /// Binary content.
    Binary { content: Vec<u8> },
}

/// The exact identity of one source content payload.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FileContentId(pub u128);

impl std::fmt::Debug for FileContentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "c{:032x}", self.0)
    }
}

impl std::fmt::Display for FileContentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "c{:032x}", self.0)
    }
}

impl FileContentId {
    /// Build one content id from one raw stable value.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Build one content id from one exact source content payload.
    pub fn for_content(content: &FileContent) -> Self {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(FILE_CONTENT_ID_DOMAIN);

        match content {
            FileContent::Text { content } => {
                hasher.update(&[0]);
                hasher.update_len_prefixed(content.as_bytes());
            }
            FileContent::Binary { content } => {
                hasher.update(&[1]);
                hasher.update_len_prefixed(content);
            }
        }

        Self::new(hasher.finish_u128())
    }
}

/// One canonical content payload and its derived data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileContentEntry {
    /// The raw content payload.
    pub payload: FileContent,
    /// Shared line index for text content.
    line_index: Option<Arc<[u32]>>,
}

impl FileContentEntry {
    /// Build one shared content entry from one payload.
    pub fn new(payload: FileContent) -> Self {
        let line_index = match &payload {
            FileContent::Text { content } => Some(Arc::<[u32]>::from(
                File::precompute_line_start_offsets(content),
            )),
            FileContent::Binary { .. } => None,
        };

        Self {
            payload,
            line_index,
        }
    }

    /// Return the raw payload.
    pub fn payload(&self) -> &FileContent {
        &self.payload
    }

    /// Return shared line start offsets when present.
    pub fn line_index(&self) -> Option<&[u32]> {
        self.line_index.as_deref()
    }

    /// Return true when this entry has one cached line index.
    pub fn has_line_index(&self) -> bool {
        self.line_index.is_some()
    }

    /// Clear any cached line index on this entry.
    pub fn clear_line_index(&mut self) {
        self.line_index = None;
    }
}

impl File {
    /// Create an empty source in some format.
    pub fn empty_text(ty: FileType) -> Self {
        Self::from_text(
            FileId::EPHEMERAL,
            "<empty>".to_string(),
            Uri::from_string("<empty>"),
            None,
            ty,
            String::new(),
        )
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

    /// Build one file from shared content.
    pub fn from_content(
        id: FileId,
        name: String,
        uri: Uri,
        path: Option<PathBuf>,
        ty: FileType,
        content: Arc<FileContentEntry>,
    ) -> Self {
        let len = match content.payload() {
            FileContent::Text { content } => content.len() as u32,
            FileContent::Binary { content } => content.len() as u32,
        };

        Self {
            id,
            name,
            uri,
            path,
            ty,
            len,
            content,
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
        Self::from_content(
            id,
            name,
            uri,
            path,
            ty,
            Arc::new(FileContentEntry::new(FileContent::Text { content })),
        )
    }

    /// Create a new binary file from bytes.
    pub fn from_binary(
        id: FileId,
        name: String,
        uri: Uri,
        path: Option<PathBuf>,
        ty: FileType,
        content: Vec<u8>,
    ) -> Self {
        Self::from_content(
            id,
            name,
            uri,
            path,
            ty,
            Arc::new(FileContentEntry::new(FileContent::Binary { content })),
        )
    }

    /// Get the text content of the File (empty if not text).
    #[inline]
    pub fn text(&self) -> &str {
        match self.content.payload() {
            FileContent::Text { content } => content,
            _ => "",
        }
    }

    /// Return shared line start offsets when present.
    pub fn line_start_offsets(&self) -> Option<&[u32]> {
        self.content.line_index()
    }

    /// Return true when this file has one cached line index.
    pub fn has_line_index(&self) -> bool {
        self.content.has_line_index()
    }

    /// Clear any cached line index on this file view.
    pub fn clear_line_index(&mut self) {
        Arc::make_mut(&mut self.content).clear_line_index();
    }

    /// Return whether the file starts with a hashbang line.
    #[inline]
    pub fn has_hashbang(&self) -> bool {
        self.text()
            .strip_prefix('\u{feff}')
            .unwrap_or(self.text())
            .starts_with("#!")
    }

    /// Get the string slice for a given span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> Option<&str> {
        match self.content.payload() {
            FileContent::Text { content } => Some(&content[span.start as usize..span.end as usize]),
            FileContent::Binary { .. } => None,
        }
    }

    /// Get the string slice for a given span, or empty string if unavailable.
    #[inline]
    pub fn span_str(&self, span: Span) -> &str {
        self.get_span_str(span).unwrap_or_default()
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
        let line_start_offsets = self.line_start_offsets()?;
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
        let line_start_offsets = self.line_start_offsets()?;
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

    /// Check if two byte positions are on the same line.
    #[inline]
    pub fn is_same_line(&self, pos_a: u32, pos_b: u32) -> bool {
        let Some(line_start_offsets) = self.line_start_offsets() else {
            return false;
        };

        // find line for pos_a using binary search
        let line_idx = match line_start_offsets.binary_search(&pos_a) {
            Ok(exact) => exact,
            Err(insert) => insert.saturating_sub(1),
        };

        // check if pos_b is within the same line
        let line_start = line_start_offsets[line_idx];
        let line_end = line_start_offsets
            .get(line_idx + 1)
            .copied()
            .unwrap_or(self.len + 1);

        pos_b >= line_start && pos_b < line_end
    }

    /// Get the byte position for a given line and column.
    /// Returns the byte index, or None if the position is invalid.
    /// Uses precomputed line offsets for O(1) performance.
    pub fn get_byte_position(&self, line_index: u32, column: u32) -> Option<u32> {
        let line_start_offsets = self.line_start_offsets()?;
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

    /// Compute byte offset from 1-indexed line and column in raw content.
    ///
    /// Useful when the file is not loaded but we have content (e.g., from filesystem read).
    /// Line and column are 1-indexed (as typical from parser error messages).
    /// Returns the byte offset, clamped to content length if out of bounds.
    pub fn byte_offset_from_position(content: &str, line: usize, column: usize) -> u32 {
        let mut current_line = 1;
        let mut line_start = 0;

        for (i, ch) in content.char_indices() {
            if current_line == line {
                // found the target line, compute column offset
                let col_offset = content[line_start..]
                    .char_indices()
                    .take(column.saturating_sub(1))
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(0);
                return (line_start + col_offset) as u32;
            }
            if ch == '\n' {
                current_line += 1;
                line_start = i + 1;
            }
        }

        // if line not found, return end of content
        content.len() as u32
    }

    /// Get the number of lines in the source (at least 1 for empty content).
    pub fn line_count(&self) -> u32 {
        if let Some(line_start_offsets) = self.line_start_offsets() {
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
