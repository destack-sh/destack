use destack_dir::SymbolType;
use destack_source::{FileId, Span};

use crate::Session;
use crate::format::format_symbol_signature;
use crate::query::common::find_symbol_at_offset;

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
pub fn hover(session: &Session, file: FileId, offset: u32) -> Option<HoverInfo> {
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // try rich signature formatting first (for declarations)
    if let Some(formatted) =
        format_symbol_signature(symbol_at.symbol_id, &session.modules, &session.strings)
    {
        return Some(HoverInfo::signature(formatted.text).with_range(symbol_at.span));
    }

    // fallback: simple formatting for locals and other symbols
    let module = session.modules.get(symbol_at.symbol_id.module_id);
    let module_guard = module.read();
    let symbols = module_guard.dir.symbols.read();
    let symbol = symbols.get_symbol(symbol_at.symbol_id.local_id);

    let name = symbol
        .name()
        .map(|id| module_guard.ast.strings.get(id).to_string());

    let signature = format_simple_signature(symbol.ty, name.as_deref());

    Some(HoverInfo::signature(signature).with_range(symbol_at.span))
}

/// Simple signature formatting for locals and symbols without declarations.
fn format_simple_signature(symbol_type: SymbolType, name: Option<&str>) -> String {
    let name = name.unwrap_or("<anonymous>");

    match symbol_type {
        SymbolType::Void => format!("(local) {name}"),
        SymbolType::Class => format!("class {name}"),
        SymbolType::Struct => format!("struct {name}"),
        SymbolType::Interface => format!("interface {name}"),
        SymbolType::Enum => format!("enum {name}"),
        SymbolType::Function => format!("function {name}"),
        SymbolType::Extension => format!("extension {name}"),
        SymbolType::TypeAlias => format!("type {name}"),
        SymbolType::Newtype => format!("newtype {name}"),
    }
}
