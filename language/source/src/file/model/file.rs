use destack_serde::Reflect;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_core::StableHasher;
use serde::{Deserialize, Serialize};

use super::hash::{stable_source_id, stable_source_path};
use crate::{ByteRange, FileType, Span, Uri};

const FILE_LOGICAL_DOMAIN: &[u8] = b"destack.source.file.logical.v1";
const FILE_SOURCE_DOMAIN: &[u8] = b"destack.source.file.source.v1";
const CONTENT_DOMAIN: &[u8] = b"destack.content.v1";

/// The id of a File.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
#[serde(transparent)]
pub struct FileId(pub u64);

impl std::fmt::Debug for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "f{:016x}", self.0)
    }
}

impl std::fmt::Display for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "f{:016x}", self.0)
    }
}

impl FileId {
    /// Turn a raw id into a FileId.
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Create a file id from one logical source path string.
    ///
    /// This should be one repository or import relative path for physical files,
    /// or one explicit logical path for virtual files.
    pub fn from_logical_str(path: &str) -> Self {
        debug_assert!(
            !path.starts_with('/'),
            "file identities derive from logical paths, got {path:?}"
        );
        let path = path.replace('\\', "/");
        Self(stable_source_id(FILE_LOGICAL_DOMAIN, &[path.as_bytes()]))
    }

    /// Create a file id from one logical source path.
    pub fn from_logical_path(path: &Path) -> Self {
        let path = stable_source_path(path);

        Self::from_logical_str(&path)
    }

    /// Create a file id from one explicit source payload.
    pub fn from_source_bytes(bytes: &[u8]) -> Self {
        Self(stable_source_id(FILE_SOURCE_DOMAIN, &[bytes]))
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
    pub content: Arc<ContentEntry>,
}

/// One exact content payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Content {
    /// Text content.
    Text { content: String },
    /// Binary content.
    Binary { content: Vec<u8> },
}

impl Content {
    /// Return the payload length in bytes.
    pub fn byte_length(&self) -> usize {
        match self {
            Self::Text { content } => content.len(),
            Self::Binary { content } => content.len(),
        }
    }
}

/// The exact identity of one content payload.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
#[serde(transparent)]
pub struct ContentId(pub u128);

impl std::fmt::Debug for ContentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "c{:016x}", self.0)
    }
}

impl std::fmt::Display for ContentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "c{:032x}", self.0)
    }
}

impl ContentId {
    /// Build one content id from one raw stable value.
    pub const fn new(value: u128) -> Self {
        Self(value)
    }

    /// Build one content id from one exact content payload.
    pub fn for_content(content: &Content) -> Self {
        match content {
            Content::Text { content } => Self::for_text(content),
            Content::Binary { content } => Self::for_binary(content),
        }
    }

    /// Build one content id from one exact text payload.
    pub fn for_text(content: &str) -> Self {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(CONTENT_DOMAIN);

        hasher.update(&[0]);
        hasher.update_len_prefixed(content.as_bytes());

        Self::new(hasher.finish_u128())
    }

    /// Build one content id from one exact binary payload.
    pub fn for_binary(content: &[u8]) -> Self {
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(CONTENT_DOMAIN);

        hasher.update(&[1]);
        hasher.update_len_prefixed(content);

        Self::new(hasher.finish_u128())
    }
}

/// One canonical content payload and its derived data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentEntry {
    /// The raw content payload.
    payload: Content,
    /// The payload length in bytes.
    length: u32,
    /// Shared line index for text content.
    line_index: Option<Arc<[u32]>>,
}

impl ContentEntry {
    /// Build one shared content entry from one payload.
    pub fn new(payload: Content) -> Self {
        let length = payload.byte_length();
        assert!(
            length <= File::MAX_BYTES,
            "content length exceeds source coordinate range"
        );

        // index text lines
        let line_index = match &payload {
            Content::Text { content } => Some(Arc::<[u32]>::from(
                File::precompute_line_start_offsets(content),
            )),
            Content::Binary { .. } => None,
        };

        Self {
            payload,
            length: length as u32,
            line_index,
        }
    }

    /// Return the raw payload.
    pub fn payload(&self) -> &Content {
        &self.payload
    }

    /// Return the exact content id for this payload.
    pub fn content_id(&self) -> ContentId {
        ContentId::for_content(&self.payload)
    }

    /// Return shared line start offsets when present.
    pub fn line_index(&self) -> Option<&[u32]> {
        self.line_index.as_deref()
    }
}

impl File {
    /// The greatest representable content length.
    pub const MAX_BYTES: usize = u32::MAX as usize - 1;

    /// Create an empty source in some format.
    pub fn empty_text(ty: FileType) -> Self {
        let file_id = FileId::from_logical_str("<empty>");

        Self::from_text(
            file_id,
            "<empty>".to_string(),
            Uri::from_string("<empty>"),
            None,
            ty,
            String::new(),
        )
    }

    /// Precompute line start byte offsets for constant-time line access.
    fn precompute_line_start_offsets(content: &str) -> Vec<u32> {
        let mut line_start_offsets = vec![0];
        for (offset, character) in content.char_indices() {
            if character == '\n' {
                line_start_offsets.push(offset as u32 + 1);
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
        content: Arc<ContentEntry>,
    ) -> Self {
        let len = content.length;

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
        let content = ContentEntry::new(Content::Text { content });

        Self::from_content(id, name, uri, path, ty, Arc::new(content))
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
        let content = ContentEntry::new(Content::Binary { content });

        Self::from_content(id, name, uri, path, ty, Arc::new(content))
    }

    /// Get the text content of the File (empty if not text).
    #[inline]
    pub fn text(&self) -> &str {
        match self.content.payload() {
            Content::Text { content } => content,
            _ => "",
        }
    }

    /// Return the exact content id for this file image.
    pub fn content_id(&self) -> ContentId {
        self.content.content_id()
    }

    /// Return shared line start offsets when present.
    pub fn line_start_offsets(&self) -> Option<&[u32]> {
        self.content.line_index()
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
        if span.file != self.id {
            return None;
        }

        self.get_range_str(span.range())
    }

    /// Return the string slice for one file-local byte range.
    #[inline]
    pub fn get_range_str(&self, range: ByteRange) -> Option<&str> {
        match self.content.payload() {
            Content::Text { content } => content.get(range.start as usize..range.end as usize),
            Content::Binary { .. } => None,
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
    pub fn is_same_line(&self, first_position: u32, second_position: u32) -> bool {
        let Some(line_start_offsets) = self.line_start_offsets() else {
            return false;
        };
        if first_position > self.len || second_position > self.len {
            return false;
        }

        // find the first position's line
        let line_index = match line_start_offsets.binary_search(&first_position) {
            Ok(exact) => exact,
            Err(insertion) => insertion.saturating_sub(1),
        };

        // check the lower file and line bounds
        let line_start = line_start_offsets[line_index];
        if second_position < line_start {
            return false;
        }

        // check the next line bound when present
        let Some(next_line_start) = line_start_offsets.get(line_index + 1) else {
            return true;
        };

        second_position < *next_line_start
    }

    /// Return the byte position for one zero-based line and byte column.
    pub fn get_byte_position(&self, line_index: u32, column: u32) -> Option<u32> {
        let line_start_offsets = self.line_start_offsets()?;
        let line_start = *line_start_offsets.get(line_index as usize)?;
        let byte_position = line_start.checked_add(column)?;

        // check if the position is within bounds
        if byte_position > self.len {
            return None;
        }

        // check if the column is within the line bounds
        if let Some(next_line_start) = line_start_offsets.get(line_index as usize + 1)
            && byte_position >= *next_line_start
        {
            return None;
        }

        Some(byte_position)
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
