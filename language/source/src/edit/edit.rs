use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::{File, FileId, Span};

/// Error produced while applying source edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditApplyError {
    /// The edited file cannot be loaded.
    MissingFile {
        /// The missing file.
        file: FileId,
    },
    /// One edit is attached to a different file.
    FileMismatch {
        /// The file being edited.
        file: FileId,
        /// The file carried by the edit span.
        edit_file: FileId,
    },
    /// Edits overlap after sorting.
    OverlappingEdits {
        /// The edited file.
        file: FileId,
    },
    /// One edit span is outside the file.
    OutsideFile {
        /// The edited file.
        file: FileId,
        /// The edit start byte offset.
        start: u32,
        /// The edit end byte offset.
        end: u32,
        /// The file length in bytes.
        len: usize,
    },
    /// One edit span does not land on UTF-8 boundaries.
    Boundary {
        /// The edited file.
        file: FileId,
        /// The edit start byte offset.
        start: u32,
        /// The edit end byte offset.
        end: u32,
    },
}

impl Display for EditApplyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFile { file } => {
                write!(formatter, "missing file for source edit {file:?}")
            }
            Self::FileMismatch { file, edit_file } => {
                write!(
                    formatter,
                    "source edit file {edit_file:?} does not match {file:?}"
                )
            }
            Self::OverlappingEdits { file } => {
                write!(formatter, "source edits overlap in file {file:?}")
            }
            Self::OutsideFile {
                file,
                start,
                end,
                len,
            } => write!(
                formatter,
                "source edit span {start}..{end} is outside file {file:?} with length {len}"
            ),
            Self::Boundary { file, start, end } => write!(
                formatter,
                "source edit span {start}..{end} is not on UTF-8 boundaries in file {file:?}"
            ),
        }
    }
}

impl Error for EditApplyError {}

/// A single edit: replace a span with new text.
///
/// This is the atomic unit of source modification.
/// An empty `new_text` represents deletion; an empty span represents insertion.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edit {
    /// The span to replace.
    pub span: Span,
    /// The replacement text (empty for deletion).
    pub new_text: String,
}

impl Edit {
    /// Create a replacement edit.
    pub fn replace(span: Span, new_text: impl Into<String>) -> Self {
        Self {
            span,
            new_text: new_text.into(),
        }
    }

    /// Create a deletion edit.
    pub fn delete(span: Span) -> Self {
        Self {
            span,
            new_text: String::new(),
        }
    }

    /// Create an insertion edit at a position.
    pub fn insert(file: FileId, position: u32, text: impl Into<String>) -> Self {
        Self {
            span: Span::new(file, position, position),
            new_text: text.into(),
        }
    }

    /// Whether this edit is a pure insertion (zero-width span).
    #[inline]
    pub fn is_insert(&self) -> bool {
        self.span.is_empty()
    }

    /// Whether this edit is a pure deletion (empty replacement).
    #[inline]
    pub fn is_delete(&self) -> bool {
        self.new_text.is_empty() && !self.span.is_empty()
    }

    /// Get the file this edit applies to.
    #[inline]
    pub fn file(&self) -> FileId {
        self.span.file
    }
}

/// Edits for a single file.
///
/// Groups multiple edits together for efficient application.
/// Edits should be non-overlapping and are typically sorted by position.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileEdit {
    /// The file to edit.
    pub file: FileId,
    /// The edits to apply (should be non-overlapping).
    pub edits: Vec<Edit>,
}

impl FileEdit {
    /// Create a new FileEdit for the given file.
    pub fn new(file: FileId) -> Self {
        Self {
            file,
            edits: Vec::new(),
        }
    }

    /// Create a FileEdit with the given edits.
    pub fn with_edits(file: FileId, edits: Vec<Edit>) -> Self {
        Self { file, edits }
    }

    /// Add an edit.
    pub fn push(&mut self, edit: Edit) {
        debug_assert_eq!(edit.file(), self.file, "edit file mismatch");
        self.edits.push(edit);
    }

    /// Add a replacement edit.
    pub fn replace(&mut self, span: Span, new_text: impl Into<String>) {
        self.push(Edit::replace(span, new_text));
    }

    /// Add a deletion edit.
    pub fn delete(&mut self, span: Span) {
        self.push(Edit::delete(span));
    }

    /// Add an insertion edit.
    pub fn insert(&mut self, position: u32, text: impl Into<String>) {
        self.push(Edit::insert(self.file, position, text));
    }

    /// Whether there are no edits.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Number of edits.
    #[inline]
    pub fn len(&self) -> usize {
        self.edits.len()
    }

    /// Sort edits by position (start, then end).
    pub fn sort(&mut self) {
        self.edits.sort_by_key(|e| (e.span.start, e.span.end));
    }
}

/// Edits across multiple files.
///
/// Used for refactoring operations that touch multiple files (like rename).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct BatchEdit {
    /// Per-file edits.
    pub files: Vec<FileEdit>,
}

impl BatchEdit {
    /// Create an empty BatchEdit.
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    /// Create a BatchEdit from file edits.
    pub fn from_files(files: Vec<FileEdit>) -> Self {
        Self { files }
    }

    /// Create a BatchEdit for a single file.
    pub fn single(file_edit: FileEdit) -> Self {
        Self {
            files: vec![file_edit],
        }
    }

    /// Add a FileEdit.
    pub fn push(&mut self, file_edit: FileEdit) {
        self.files.push(file_edit);
    }

    /// Get or create a FileEdit for the given file.
    pub fn file_mut(&mut self, file: FileId) -> &mut FileEdit {
        // find existing or create new
        let index = self.files.iter().position(|f| f.file == file);
        match index {
            Some(i) => &mut self.files[i],
            None => {
                self.files.push(FileEdit::new(file));
                self.files.last_mut().unwrap()
            }
        }
    }

    /// Add an edit to the appropriate file.
    pub fn add(&mut self, edit: Edit) {
        self.file_mut(edit.file()).push(edit);
    }

    /// Whether there are no edits.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() || self.files.iter().all(|f| f.is_empty())
    }

    /// Total number of edits across all files.
    pub fn total_edits(&self) -> usize {
        self.files.iter().map(|f| f.len()).sum()
    }

    /// Number of files affected.
    #[inline]
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Iterate over all edits with their file.
    pub fn iter(&self) -> impl Iterator<Item = &Edit> {
        self.files.iter().flat_map(|f| f.edits.iter())
    }
}

impl FromIterator<FileEdit> for BatchEdit {
    fn from_iter<T: IntoIterator<Item = FileEdit>>(iter: T) -> Self {
        Self {
            files: iter.into_iter().collect(),
        }
    }
}

/// Apply one file edit to source text.
pub fn apply_file_edit(file: &File, file_edit: &FileEdit) -> Result<String, EditApplyError> {
    let mut edits = file_edit.edits.clone();
    edits.sort_by_key(|edit| (edit.span.start, edit.span.end));

    // validate edit order and ownership before mutating text
    let mut previous_end = 0;
    for edit in &edits {
        // reject edits attached to a different file
        if edit.span.file != file_edit.file {
            return Err(EditApplyError::FileMismatch {
                file: file_edit.file,
                edit_file: edit.span.file,
            });
        }

        // reject edits that would rewrite the same byte twice
        if edit.span.start < previous_end {
            return Err(EditApplyError::OverlappingEdits {
                file: file_edit.file,
            });
        }

        previous_end = edit.span.end;
    }

    // apply from the back so byte offsets stay stable
    let mut text = file.text().to_string();
    for edit in edits.iter().rev() {
        let start = edit.span.start as usize;
        let end = edit.span.end as usize;

        // reject byte ranges outside the source text
        if start > end || end > text.len() {
            return Err(EditApplyError::OutsideFile {
                file: file_edit.file,
                start: edit.span.start,
                end: edit.span.end,
                len: text.len(),
            });
        }

        // reject byte ranges that split unicode scalars
        if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            return Err(EditApplyError::Boundary {
                file: file_edit.file,
                start: edit.span.start,
                end: edit.span.end,
            });
        }

        // replace after validation
        text.replace_range(start..end, &edit.new_text);
    }

    Ok(text)
}

/// Apply one batch edit to loaded source files.
pub fn apply_batch_edit<'a, F>(
    edits: &BatchEdit,
    file_for_id: F,
) -> Result<HashMap<FileId, String>, EditApplyError>
where
    F: Fn(FileId) -> Option<&'a File>,
{
    let mut updates = HashMap::new();

    for file_edit in &edits.files {
        // require every edited file to be available
        let file = file_for_id(file_edit.file).ok_or(EditApplyError::MissingFile {
            file: file_edit.file,
        })?;

        // apply one file edit independently
        let text = apply_file_edit(file, file_edit)?;

        updates.insert(file_edit.file, text);
    }

    Ok(updates)
}
