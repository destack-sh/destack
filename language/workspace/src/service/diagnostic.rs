use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_repository::Revision;
use tspp_serde::Reflect;
use tspp_source::Diagnostic;

use crate::{DiagnosticsRequest, Error, FileDiagnostics, FileImage};

/// Request to read diagnostics from one workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DiagnoseRequest {
    /// Owning workspace root.
    pub root: PathBuf,
    /// Exact semantic revision.
    pub revision: Revision,
    /// Diagnostic selection within the workspace.
    pub request: DiagnosticsRequest,
}

/// Serialized diagnostics for one exact source file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileDiagnosticsResponse {
    /// Semantic revision containing these diagnostics.
    pub revision: Revision,
    /// Source file image used for range conversion.
    pub file: FileImage,
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
            diagnostics: diagnostics.diagnostics.clone(),
        }
    }
}
