use std::sync::Arc;

use destack_repository::Revision;
use destack_serde::Reflect;
use destack_source::{Diagnostic, Uri};
use serde::{Deserialize, Serialize};

use crate::{Error, FileDiagnostics, FileImage};

/// Wire representation of diagnostics for one source file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileDiagnosticsPayload {
    /// The semantic revision containing these diagnostics.
    pub revision: Revision,
    /// The file image used for range conversion.
    pub file: FileImage,
    /// The URI published to the editor.
    pub uri: Uri,
    /// The editor document version when the file is open.
    pub version: Option<i32>,
    /// The diagnostics for the file.
    pub diagnostics: Vec<Diagnostic>,
}

impl TryFrom<FileDiagnosticsPayload> for FileDiagnostics {
    type Error = Error;

    /// Convert one wire payload into runtime file diagnostics.
    fn try_from(payload: FileDiagnosticsPayload) -> Result<Self, Self::Error> {
        let file = payload.file.into_file()?;

        Ok(Self {
            revision: payload.revision,
            file: Arc::new(file),
            uri: payload.uri,
            version: payload.version,
            diagnostics: payload.diagnostics,
        })
    }
}

impl From<&FileDiagnostics> for FileDiagnosticsPayload {
    /// Build one wire payload from runtime file diagnostics.
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
