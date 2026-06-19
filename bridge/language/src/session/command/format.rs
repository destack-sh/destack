use crate::{Module, bridge};

/// One document accepted by formatter operations.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Document {
    /// Repository module at one immutable revision.
    Module {
        /// Loaded source module.
        module: Module,
    },
    /// Ad hoc source text.
    Text {
        /// Display path used for parser language detection.
        path: String,
        /// Source text.
        text: String,
    },
}

/// One formatter request.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatRequest {
    /// Document to format.
    pub document: Document,
}

/// One formatter output.
#[bridge]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatOutput {
    /// Formatted source text.
    pub text: String,
}

impl Document {
    /// Create one module document.
    pub fn module(module: Module) -> Self {
        Self::Module { module }
    }

    /// Create one text document.
    pub fn text(path: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Text {
            path: path.into(),
            text: text.into(),
        }
    }
}

impl FormatRequest {
    /// Create one formatter request.
    pub fn new(document: Document) -> Self {
        Self { document }
    }
}
