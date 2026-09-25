use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{error, fmt, str};

use serde::{Deserialize, Serialize};
use tspp_core::{Blob, BlobMemory, SectionEntry};
use tspp_serde::Reflect;

use super::hash::{stable_source_id, stable_source_path};
use crate::{ByteRange, FileType, Span, Uri};

const FILE_LOGICAL_DOMAIN: &[u8] = b"tspp.source.file.logical.v1";
const FILE_SOURCE_DOMAIN: &[u8] = b"tspp.source.file.source.v1";

/// The id of a File.
#[repr(transparent)]
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect, SectionEntry,
)]
#[serde(transparent)]
pub struct FileId(pub u64);

impl fmt::Debug for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "f{:016x}", self.0)
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

/// One loaded source file.
#[derive(Debug, Clone)]
pub struct File {
    /// The stable file identity.
    pub id: FileId,
    /// The source name, usually the last URI segment.
    pub name: String,
    /// The source URI.
    pub uri: Uri,
    /// The physical path when one exists.
    pub path: Option<PathBuf>,
    /// The source format.
    pub ty: FileType,
    /// The byte length.
    pub len: u32,
    /// The exact immutable bytes.
    pub blob: Blob,
    /// Retained immutable byte memory.
    memory: Arc<BlobMemory>,
    /// Precomputed line starts for text content.
    line_index: Option<LineIndex>,
}

/// Precomputed line starts retained by one text File.
#[derive(Debug, Clone)]
enum LineIndex {
    /// Build-owned line starts.
    Static(&'static [u32]),
    /// Runtime-owned line starts.
    Shared(Arc<[u32]>),
}

impl AsRef<[u32]> for LineIndex {
    /// Borrow the line starts.
    fn as_ref(&self) -> &[u32] {
        match self {
            Self::Static(offsets) => offsets,
            Self::Shared(offsets) => offsets,
        }
    }
}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.uri == other.uri
            && self.path == other.path
            && self.ty == other.ty
            && self.blob == other.blob
    }
}

impl Eq for File {}

/// Failure while constructing one loaded File.
#[derive(Debug)]
pub enum FileError {
    /// The Blob exceeds the source coordinate range.
    TooLarge {
        /// The Blob byte length.
        byte_len: u64,
    },
    /// A text File contains invalid UTF-8.
    Utf8(str::Utf8Error),
}

impl fmt::Display for FileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { byte_len } => {
                write!(
                    formatter,
                    "File contains {byte_len} bytes beyond the source limit"
                )
            }
            Self::Utf8(error) => write!(formatter, "File text is not UTF-8: {error}"),
        }
    }
}

impl error::Error for FileError {}

impl File {
    /// The greatest representable content length.
    pub const MAX_BYTES: usize = u32::MAX as usize - 1;

    /// Build one empty text File.
    pub fn empty_text(ty: FileType) -> Self {
        let file_id = FileId::from_logical_str("<empty>");
        let memory = Arc::new(BlobMemory::from_bytes(Vec::new()));

        Self {
            id: file_id,
            name: "<empty>".to_string(),
            uri: Uri::from_string("<empty>"),
            path: None,
            ty,
            len: 0,
            blob: memory.blob(),
            memory,
            line_index: Some(LineIndex::Shared(Arc::from([0_u32]))),
        }
    }

    /// Return every line start byte offset in source text.
    pub fn line_starts(content: &str) -> Vec<u32> {
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

    /// Build one File from retained byte memory and an optional line index.
    fn from_memory(
        id: FileId,
        name: String,
        uri: Uri,
        path: Option<PathBuf>,
        ty: FileType,
        memory: Arc<BlobMemory>,
        line_index: Option<LineIndex>,
    ) -> Result<Self, FileError> {
        // enforce source coordinate bounds
        let blob = memory.blob();
        if blob.byte_len > Self::MAX_BYTES as u64 {
            return Err(FileError::TooLarge {
                byte_len: blob.byte_len,
            });
        }

        Ok(Self {
            id,
            name,
            uri,
            path,
            ty,
            len: blob.byte_len as u32,
            blob,
            memory,
            line_index,
        })
    }

    /// Build one File from an exact Blob using its format's default representation.
    pub fn from_blob(
        id: FileId,
        name: String,
        uri: Uri,
        path: Option<PathBuf>,
        ty: FileType,
        memory: Arc<BlobMemory>,
    ) -> Result<Self, FileError> {
        // validate and index source text once
        let line_index = if ty.is_opaque() {
            None
        } else {
            let text = str::from_utf8(memory.bytes()).map_err(FileError::Utf8)?;

            Some(LineIndex::Shared(Arc::<[u32]>::from(Self::line_starts(
                text,
            ))))
        };

        Self::from_memory(id, name, uri, path, ty, memory, line_index)
    }

    /// Build one text File from retained memory and build-owned line starts.
    ///
    /// # Safety
    ///
    /// The retained bytes must be valid UTF-8 and `line_start_offsets` must describe them exactly.
    pub unsafe fn from_indexed_text(
        id: FileId,
        name: String,
        uri: Uri,
        path: Option<PathBuf>,
        ty: FileType,
        memory: Arc<BlobMemory>,
        line_start_offsets: &'static [u32],
    ) -> Result<Self, FileError> {
        let line_index = Some(LineIndex::Static(line_start_offsets));

        Self::from_memory(id, name, uri, path, ty, memory, line_index)
    }

    /// Build one text File from owned source text.
    pub fn from_text(
        id: FileId,
        name: String,
        uri: Uri,
        path: Option<PathBuf>,
        ty: FileType,
        content: String,
    ) -> Result<Self, FileError> {
        // normalize and index the provided text
        let content = Self::normalize_line_endings(content);
        let line_index = Arc::<[u32]>::from(Self::line_starts(&content));

        // retain the normalized bytes
        let bytes = content.into_bytes();
        let memory = Arc::new(BlobMemory::from_bytes(bytes));

        let line_index = LineIndex::Shared(line_index);

        Self::from_memory(id, name, uri, path, ty, memory, Some(line_index))
    }

    /// Build one binary File from owned bytes.
    pub fn from_binary(
        id: FileId,
        name: String,
        uri: Uri,
        path: Option<PathBuf>,
        ty: FileType,
        content: Vec<u8>,
    ) -> Result<Self, FileError> {
        let memory = Arc::new(BlobMemory::from_bytes(content));

        Self::from_blob(id, name, uri, path, ty, memory)
    }

    /// Return the text content of the File.
    ///
    /// # Panics
    ///
    /// Panics when this File has no text representation.
    #[inline]
    pub fn text(&self) -> &str {
        assert!(self.is_text(), "File has no text representation");

        // constructors attach line indexes only to validated UTF-8
        unsafe { str::from_utf8_unchecked(self.bytes()) }
    }

    /// Return the exact immutable bytes.
    pub fn bytes(&self) -> &[u8] {
        self.memory.bytes()
    }

    /// Return whether this File has validated text content.
    pub fn is_text(&self) -> bool {
        self.line_index.is_some()
    }

    /// Return the exact Blob descriptor.
    pub const fn blob(&self) -> Blob {
        self.blob
    }

    /// Return shared line start offsets when present.
    pub fn line_start_offsets(&self) -> Option<&[u32]> {
        self.line_index.as_ref().map(AsRef::as_ref)
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
        self.line_index.as_ref()?;

        self.text().get(range.start as usize..range.end as usize)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_text_with_opaque_file_type() {
        let file = File::from_text(
            FileId::new(1),
            "shader.glsl".to_string(),
            Uri::from_string("memory:/shader.glsl"),
            None,
            FileType::Binary,
            "first\nsecond".to_string(),
        )
        .expect("text file should build");

        assert!(file.is_text());
        assert_eq!(file.text(), "first\nsecond");
        assert_eq!(file.line_start_offsets(), Some([0, 6].as_slice()));
    }
}
