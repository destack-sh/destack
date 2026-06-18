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
