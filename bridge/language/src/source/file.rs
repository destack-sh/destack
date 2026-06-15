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

/// External content id crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentId {
    /// Canonical lowercase hex content id.
    pub id: String,
}

/// Full content crossing bridge boundaries.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    /// Text content.
    Text {
        /// Text content.
        content: String,
    },
    /// Binary content.
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

impl ContentId {
    /// Convert one source content id into one bridge content id.
    pub fn from_source(id: source::ContentId) -> Self {
        Self {
            id: format!("{:032x}", id.0),
        }
    }

    /// Convert this bridge content id into one source content id.
    pub fn into_source(self) -> Result<source::ContentId, SourceIdParseError> {
        let id = parse_u128("content", &self.id)?;

        Ok(source::ContentId::new(id))
    }
}

impl Content {
    /// Convert one source content into one bridge content.
    pub fn from_source(content: source::Content) -> Self {
        match content {
            source::Content::Text { content } => Self::Text { content },
            source::Content::Binary { content } => Self::Binary { content },
        }
    }

    /// Convert this bridge content into one source content.
    pub fn into_source(self) -> source::Content {
        match self {
            Self::Text { content } => source::Content::Text { content },
            Self::Binary { content } => source::Content::Binary { content },
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

impl From<source::ContentId> for ContentId {
    /// Convert one source content id into one bridge content id.
    fn from(id: source::ContentId) -> Self {
        Self::from_source(id)
    }
}

impl TryFrom<ContentId> for source::ContentId {
    type Error = SourceIdParseError;

    /// Convert one bridge content id into one source content id.
    fn try_from(id: ContentId) -> Result<Self, Self::Error> {
        id.into_source()
    }
}

impl From<source::Content> for Content {
    /// Convert one source content into one bridge content.
    fn from(content: source::Content) -> Self {
        Self::from_source(content)
    }
}

impl From<Content> for source::Content {
    /// Convert one bridge content into one source content.
    fn from(content: Content) -> Self {
        content.into_source()
    }
}
