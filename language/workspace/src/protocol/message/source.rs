use std::sync::Arc;

use destack_serde::Reflect;
use destack_source::Patch;
use serde::{Deserialize, Serialize};

use crate::{Error, FileEdit, FileImage};

/// Wire representation of one patch over an exact source file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileEditPayload {
    /// The exact source file image interpreted by the patch.
    pub file: FileImage,
    /// The source replacement.
    pub patch: Patch,
}

impl TryFrom<FileEditPayload> for FileEdit {
    type Error = Error;

    /// Convert one wire payload into a runtime file edit.
    fn try_from(payload: FileEditPayload) -> Result<Self, Self::Error> {
        let file = payload.file.into_file()?;

        Ok(Self {
            file: Arc::new(file),
            patch: payload.patch,
        })
    }
}

impl From<&FileEdit> for FileEditPayload {
    /// Build one wire payload from a runtime file edit.
    fn from(edit: &FileEdit) -> Self {
        Self {
            file: FileImage::from(edit.file.as_ref()),
            patch: edit.patch.clone(),
        }
    }
}
