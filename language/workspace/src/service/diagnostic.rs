use std::sync::Arc;

use destack_repository::Revision;
use destack_serde::Reflect;
use destack_source::{Diagnostic, Uri};
use serde::{Deserialize, Serialize};

use crate::{Error, FileDiagnostics, FileImage};

/// Serialized diagnostics for one exact source file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileDiagnosticsResponse {
    /// Semantic revision containing these diagnostics.
    pub revision: Revision,
    /// Source file image used for range conversion.
    pub file: FileImage,
    /// URI published to the editor.
    pub uri: Uri,
    /// Editor document version when the file is open.
    pub version: Option<i32>,
    /// Diagnostics for the source file.
    pub diagnostics: Vec<Diagnostic>,
}

impl TryFrom<FileDiagnosticsResponse> for FileDiagnostics {
    type Error = Error;

    /// Convert one serialized response into runtime diagnostics.
    fn try_from(response: FileDiagnosticsResponse) -> Result<Self, Self::Error> {
        let file = response.file.into_file()?;

        Ok(Self {
            revision: response.revision,
            file: Arc::new(file),
            uri: response.uri,
            version: response.version,
            diagnostics: response.diagnostics,
        })
    }
}

impl From<&FileDiagnostics> for FileDiagnosticsResponse {
    /// Build one serialized response from runtime diagnostics.
    fn from(diagnostics: &FileDiagnostics) -> Self {
        Self {
            revision: diagnostics.revision,
            file: FileImage::from(diagnostics.file.as_ref()),
            uri: diagnostics.uri.clone(),
            version: diagnostics.version,
            diagnostics: diagnostics.diagnostics.clone(),
        }
    }
}
