use destack_source::{ContentId, FileType};
use serde::{Deserialize, Serialize};

use crate::SourceMap;

/// One compiled-code linker input for a target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    /// The compiled-code object format.
    pub format: ObjectFormat,
    /// The encoded object content identity.
    pub content: ContentId,
    /// The source map when one exists.
    pub map: Option<SourceMap>,
}

impl Object {
    /// Create one compiled-code object.
    pub fn new(format: ObjectFormat, content: ContentId, map: Option<SourceMap>) -> Self {
        Self {
            format,
            content,
            map,
        }
    }

    /// Create one native object.
    pub fn native(content: ContentId) -> Self {
        Self {
            format: ObjectFormat::Object,
            content,
            map: None,
        }
    }

    /// Create one wasm object.
    pub fn wasm(content: ContentId, map: Option<SourceMap>) -> Self {
        Self {
            format: ObjectFormat::Wasm,
            content,
            map,
        }
    }

    /// Return this object as an emitted file type.
    pub fn file_type(&self) -> FileType {
        self.format.file_type()
    }

    /// Return all content ids referenced by this object.
    pub fn content_ids(&self) -> Vec<ContentId> {
        vec![self.content]
    }
}

/// One compiled-code object format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectFormat {
    /// Native relocatable object file.
    Object,
    /// WebAssembly object or module payload.
    Wasm,
}

impl ObjectFormat {
    /// Return the emitted file type for this object format.
    pub fn file_type(self) -> FileType {
        match self {
            Self::Object => FileType::Object,
            Self::Wasm => FileType::Wasm,
        }
    }

    /// Return the object format for one emitted file type.
    pub fn from_file_type(file_type: FileType) -> Option<Self> {
        match file_type {
            FileType::Object => Some(Self::Object),
            FileType::Wasm => Some(Self::Wasm),
            _ => None,
        }
    }
}
