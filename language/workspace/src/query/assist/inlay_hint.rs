use destack_source::{FileId, Span};

use crate::Session;

/// Kind of inlay hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InlayHintKind {
    /// Type annotation hint (e.g., `: string`).
    Type,
    /// Parameter name hint (e.g., `name:`).
    Parameter,
}

/// An inlay hint (virtual text shown inline).
#[derive(Debug, Clone)]
pub struct InlayHint {
    /// Position where the hint should be displayed.
    pub position: u32,
    /// The hint text.
    pub label: String,
    /// The kind of hint.
    pub kind: InlayHintKind,
    /// Whether there should be padding before the hint.
    pub padding_left: bool,
    /// Whether there should be padding after the hint.
    pub padding_right: bool,
}

impl InlayHint {
    /// Create a type hint.
    pub fn type_hint(position: u32, type_name: impl Into<String>) -> Self {
        Self {
            position,
            label: format!(": {}", type_name.into()),
            kind: InlayHintKind::Type,
            padding_left: false,
            padding_right: false,
        }
    }

    /// Create a parameter hint.
    pub fn parameter_hint(position: u32, param_name: impl Into<String>) -> Self {
        Self {
            position,
            label: format!("{}:", param_name.into()),
            kind: InlayHintKind::Parameter,
            padding_left: false,
            padding_right: true,
        }
    }
}

/// Get inlay hints for a range in a file.
pub fn inlay_hints(
    _session: &Session,
    _file: FileId,
    _range: Span,
) -> Vec<InlayHint> {
    // 1. walk the AST in the given range
    // 2. for variable declarations without explicit type: add type hint
    // 3. for function calls: add parameter name hints
    todo!("#Incomplete: inlay_hints")
}
