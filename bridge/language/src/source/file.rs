use destack_source as source;

use crate::source::parse_u128;
use crate::{SourceIdParseError, bridge};

/// External file id crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileId {
    /// Canonical lowercase hex file id.
    pub id: String,
}

/// External file content id crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileContentId {
    /// Canonical lowercase hex file content id.
    pub id: String,
}

/// Full file content crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileContent {
    /// Text file content.
    Text {
        /// Text content.
        content: String,
    },
    /// Binary file content.
    Binary {
        /// Binary content.
        content: Vec<u8>,
    },
}

impl FileId {
    /// Convert one source file id into one bridge file id.
    pub fn from_source(id: source::FileId) -> Self {
        Self {
            id: format!("{:032x}", id.0),
        }
    }

    /// Convert this bridge file id into one source file id.
    pub fn into_source(self) -> Result<source::FileId, SourceIdParseError> {
        let id = parse_u128("file", &self.id)?;

        Ok(source::FileId::new(id))
    }
}

impl FileContentId {
    /// Convert one source file content id into one bridge file content id.
    pub fn from_source(id: source::FileContentId) -> Self {
        Self {
            id: format!("{:032x}", id.0),
        }
    }

    /// Convert this bridge file content id into one source file content id.
    pub fn into_source(self) -> Result<source::FileContentId, SourceIdParseError> {
        let id = parse_u128("file content", &self.id)?;

        Ok(source::FileContentId::new(id))
    }
}

impl FileContent {
    /// Convert one source file content into one bridge file content.
    pub fn from_source(content: source::FileContent) -> Self {
        match content {
            source::FileContent::Text { content } => Self::Text { content },
            source::FileContent::Binary { content } => Self::Binary { content },
        }
    }

    /// Convert this bridge file content into one source file content.
    pub fn into_source(self) -> source::FileContent {
        match self {
            Self::Text { content } => source::FileContent::Text { content },
            Self::Binary { content } => source::FileContent::Binary { content },
        }
    }
}

impl From<source::FileId> for FileId {
    /// Convert one source file id into one bridge file id.
    fn from(id: source::FileId) -> Self {
        Self::from_source(id)
    }
}

impl TryFrom<FileId> for source::FileId {
    type Error = SourceIdParseError;

    /// Convert one bridge file id into one source file id.
    fn try_from(id: FileId) -> Result<Self, Self::Error> {
        id.into_source()
    }
}

impl From<source::FileContentId> for FileContentId {
    /// Convert one source file content id into one bridge file content id.
    fn from(id: source::FileContentId) -> Self {
        Self::from_source(id)
    }
}

impl TryFrom<FileContentId> for source::FileContentId {
    type Error = SourceIdParseError;

    /// Convert one bridge file content id into one source file content id.
    fn try_from(id: FileContentId) -> Result<Self, Self::Error> {
        id.into_source()
    }
}

impl From<source::FileContent> for FileContent {
    /// Convert one source file content into one bridge file content.
    fn from(content: source::FileContent) -> Self {
        Self::from_source(content)
    }
}

impl From<FileContent> for source::FileContent {
    /// Convert one bridge file content into one source file content.
    fn from(content: FileContent) -> Self {
        content.into_source()
    }
}
