use destack_source as source;

use crate::{FileId, SourceIdParseError, Span, bridge};

/// One source replacement crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Replacement {
    /// Source span to replace.
    pub span: Span,
    /// Replacement text.
    pub new_text: String,
}

/// Edits for a single file.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FilePatch {
    /// Edited file.
    pub file: FileId,
    /// Source replacements.
    pub replacements: Vec<Replacement>,
}

/// Edits across multiple files.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BatchEdit {
    /// Per-file edits.
    pub files: Vec<FilePatch>,
}

impl Replacement {
    /// Convert one source edit into one bridge replacement.
    pub fn from_source(edit: source::Edit) -> Self {
        Self {
            span: edit.span.into(),
            new_text: edit.new_text,
        }
    }

    /// Convert this bridge replacement into one source edit.
    pub fn into_source(self) -> Result<source::Edit, SourceIdParseError> {
        Ok(source::Edit::replace(
            self.span.into_source()?,
            self.new_text,
        ))
    }
}

impl FilePatch {
    /// Convert one source file edit into one bridge file patch.
    pub fn from_source(edit: source::FileEdit) -> Self {
        Self {
            file: edit.file.into(),
            replacements: edit
                .edits
                .into_iter()
                .map(Replacement::from_source)
                .collect(),
        }
    }

    /// Convert this bridge file patch into one source file edit.
    pub fn into_source(self) -> Result<source::FileEdit, SourceIdParseError> {
        let file = self.file.into_source()?;
        let edits = self
            .replacements
            .into_iter()
            .map(Replacement::into_source)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(source::FileEdit::with_edits(file, edits))
    }
}

impl BatchEdit {
    /// Convert one source batch edit into one bridge batch edit.
    pub fn from_source(edit: source::BatchEdit) -> Self {
        Self {
            files: edit.files.into_iter().map(FilePatch::from_source).collect(),
        }
    }

    /// Convert this bridge batch edit into one source batch edit.
    pub fn into_source(self) -> Result<source::BatchEdit, SourceIdParseError> {
        let files = self
            .files
            .into_iter()
            .map(FilePatch::into_source)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(source::BatchEdit::from_files(files))
    }
}

impl From<source::Edit> for Replacement {
    /// Convert one source edit into one bridge replacement.
    fn from(edit: source::Edit) -> Self {
        Self::from_source(edit)
    }
}

impl TryFrom<Replacement> for source::Edit {
    type Error = SourceIdParseError;

    /// Convert one bridge replacement into one source edit.
    fn try_from(replacement: Replacement) -> Result<Self, Self::Error> {
        replacement.into_source()
    }
}

impl From<source::FileEdit> for FilePatch {
    /// Convert one source file edit into one bridge file patch.
    fn from(edit: source::FileEdit) -> Self {
        Self::from_source(edit)
    }
}

impl TryFrom<FilePatch> for source::FileEdit {
    type Error = SourceIdParseError;

    /// Convert one bridge file patch into one source file edit.
    fn try_from(patch: FilePatch) -> Result<Self, Self::Error> {
        patch.into_source()
    }
}

impl From<source::BatchEdit> for BatchEdit {
    /// Convert one source batch edit into one bridge batch edit.
    fn from(edit: source::BatchEdit) -> Self {
        Self::from_source(edit)
    }
}

impl TryFrom<BatchEdit> for source::BatchEdit {
    type Error = SourceIdParseError;

    /// Convert one bridge batch edit into one source batch edit.
    fn try_from(edit: BatchEdit) -> Result<Self, Self::Error> {
        edit.into_source()
    }
}
