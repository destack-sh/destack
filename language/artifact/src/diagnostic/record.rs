use destack_dir::GlobalSymbolId;
use destack_serde::Reflect;
use destack_source::Diagnostic;
use serde::{Deserialize, Serialize};

use crate::DiagnosticAnchor;

/// One stored diagnostic and its cross-module declaration references.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DiagnosticRecord {
    /// The diagnostic as emitted, with own-module labels resolved.
    pub diagnostic: Diagnostic,
    /// Cross-module references, resolved to labels when read.
    pub references: Vec<DeclarationReference>,
}

/// One cross-module declaration reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct DeclarationReference {
    /// The referenced declaration.
    pub symbol: GlobalSymbolId,
    /// The optional label message.
    pub message: Option<String>,
}

impl DiagnosticRecord {
    /// Record one resolved diagnostic without references.
    pub fn new(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostic,
            references: Vec::new(),
        }
    }

    /// Add one declaration reference.
    pub fn reference(mut self, symbol: GlobalSymbolId, message: Option<String>) -> Self {
        self.references
            .push(DeclarationReference { symbol, message });
        self
    }
}

impl From<&Diagnostic> for DiagnosticRecord {
    /// Record one already-resolved diagnostic.
    fn from(diagnostic: &Diagnostic) -> Self {
        Self::new(diagnostic.clone())
    }
}

impl DiagnosticAnchor {
    /// Return the referenced declaration when this anchor names one.
    pub fn declaration(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Symbol(symbol) => Some(*symbol),
            _ => None,
        }
    }
}
