use std::sync::Arc;

use destack_source::{Diagnostic, File, Uri};

/// Diagnostic snapshot for one file.
#[derive(Debug, Clone)]
pub struct DiagnosticSnapshot {
    /// The current file image used for range conversion.
    pub file: Arc<File>,
    /// Diagnostic uri for this snapshot.
    pub diagnostic_uri: Uri,
    /// Protocol file version for diagnostics when the file is open.
    pub diagnostic_version: Option<i32>,
    /// The diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}
