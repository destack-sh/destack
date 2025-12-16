use destack_source::{FileId, Span};

use crate::Session;

/// Hover information for a symbol.
#[derive(Debug, Clone)]
pub struct HoverInfo {
    /// The type/signature in code format.
    pub signature: String,
    /// Documentation (markdown).
    pub documentation: Option<String>,
    /// The range of the hovered element.
    pub range: Option<Span>,
}

impl HoverInfo {
    /// Create hover info with just a signature.
    pub fn signature(signature: impl Into<String>) -> Self {
        Self {
            signature: signature.into(),
            documentation: None,
            range: None,
        }
    }

    /// Add documentation.
    pub fn with_documentation(mut self, doc: impl Into<String>) -> Self {
        self.documentation = Some(doc.into());
        self
    }

    /// Add range.
    pub fn with_range(mut self, range: Span) -> Self {
        self.range = Some(range);
        self
    }

    /// Format as markdown for display.
    pub fn to_markdown(&self) -> String {
        let mut result = format!("```destack\n{}\n```", self.signature);
        if let Some(doc) = &self.documentation {
            result.push_str("\n\n---\n\n");
            result.push_str(doc);
        }
        result
    }
}

/// Get hover information for the symbol at the given position.
pub fn hover(_session: &Session, _file: FileId, _offset: u32) -> Option<HoverInfo> {
    // 1. find the symbol/node at offset
    // 2. get its type
    // 3. format the signature
    // 4. get documentation comments
    todo!("#Incomplete: hover")
}
