use serde::{Deserialize, Serialize};
use tspp_dir::GlobalSymbolId;
use tspp_serde::Reflect;
use tspp_source::Diagnostic;

/// One stored diagnostic and its deferred labels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DiagnosticRecord {
    /// The diagnostic with immediate source labels resolved.
    pub diagnostic: Diagnostic,
    /// Diagnostic labels resolved against the current revision.
    pub deferred_labels: Vec<DeferredDiagnosticLabel>,
}

/// One diagnostic label resolved against the current revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DeferredDiagnosticLabel {
    /// The declaration anchoring the label.
    pub anchor: GlobalSymbolId,
    /// The label message.
    pub message: String,
}

impl DiagnosticRecord {
    /// Record one resolved diagnostic without deferred labels.
    pub fn new(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostic,
            deferred_labels: Vec::new(),
        }
    }
}

impl From<&Diagnostic> for DiagnosticRecord {
    /// Record one already-resolved diagnostic.
    fn from(diagnostic: &Diagnostic) -> Self {
        Self::new(diagnostic.clone())
    }
}
