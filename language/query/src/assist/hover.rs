use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::format::{format_global_type, format_hover_markdown, format_simple_signature};
use crate::{ModuleQueryContext, Position, SymbolHit, format_symbol_signature};

/// Hover payload for a source position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Hover {
    /// The type/signature in code format.
    pub signature: String,
    /// Documentation (markdown).
    pub documentation: Option<String>,
    /// Resolved type information when available.
    pub type_text: Option<String>,
    /// Source location text when available.
    pub location: Option<String>,
    /// The range of the hovered element.
    pub range: Option<Span>,
}

impl Hover {
    /// Create hover payload with a signature.
    pub fn new(signature: impl Into<String>) -> Self {
        Self {
            signature: signature.into(),
            documentation: None,
            type_text: None,
            location: None,
            range: None,
        }
    }

    /// Insert documentation when it exists.
    fn insert_documentation(&mut self, documentation: Option<String>) {
        self.documentation = Self::text(documentation);
    }

    /// Insert resolved type text when it exists.
    fn insert_type_text(&mut self, type_text: Option<String>) {
        self.type_text = Self::text(type_text);
    }

    /// Insert location text when it exists.
    fn insert_location(&mut self, location: Option<String>) {
        self.location = Self::text(location);
    }

    /// Insert the hovered range.
    fn insert_range(&mut self, range: Span) {
        self.range = Some(range);
    }

    /// Format as markdown for display.
    pub fn to_markdown(&self) -> String {
        format_hover_markdown(
            &self.signature,
            self.type_text.as_deref(),
            self.documentation.as_deref(),
            self.location.as_deref(),
        )
    }

    /// Return non-empty text.
    fn text(text: Option<String>) -> Option<String> {
        text.filter(|text| !text.trim().is_empty())
    }
}

/// Request hover information at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct HoverRequest {
    /// The queried position.
    pub position: Position,
}

/// Response payload for hover queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct HoverResponse {
    /// Hover information, if available.
    pub hover: Option<Hover>,
}

impl ModuleQueryContext<'_> {
    /// Return hover information for the symbol at the given position.
    pub fn hover(&self, offset: u32) -> Option<Hover> {
        let symbol_at = self.hover_symbol_at_offset(offset)?;
        let canonical_id = self.canonical_symbol(symbol_at.symbol_id);

        // get documentation for this symbol
        let documentation = self.symbol_doc_text(canonical_id);

        // try rich signature formatting first (for top level declarations)
        if let Some(formatted) = format_symbol_signature(self, canonical_id) {
            let mut hover = Hover::new(formatted.text);
            hover.insert_documentation(documentation);
            hover.insert_location(self.hover_location(symbol_at.span));
            hover.insert_range(symbol_at.span);

            return Some(hover);
        }

        // resolve symbol metadata
        let symbols = self.symbols();
        let symbol = symbols.get_symbol(symbol_at.symbol_id.local_id);

        // format symbol hover metadata
        let hover_node_id = self.hover_node_id(&symbol_at, symbol);
        let signature = self.hover_signature(hover_node_id, symbol_at.symbol_id, symbol);
        let type_text = self.hover_type_text(hover_node_id, symbol_at.symbol_id);
        let location = self.hover_location(symbol_at.span);
        let range = self.hover_range(symbol_at.node_id, symbol_at.span);

        let mut hover = Hover::new(signature);
        hover.insert_documentation(documentation);
        hover.insert_type_text(type_text);
        hover.insert_location(location);
        hover.insert_range(range);

        Some(hover)
    }

    /// Format a source location string for a hover span.
    fn hover_location(&self, span: Span) -> Option<String> {
        let file = self.read_file(span.file);
        let path = file.path.as_ref()?;
        let (line, col) = file.get_position(span.start)?;

        Some(format!(
            "{}:{}:{}",
            path.to_string_lossy(),
            line + 1,
            col + 1
        ))
    }

    /// Return the DIR node used for hover display.
    fn hover_node_id(&self, symbol_at: &SymbolHit, symbol: &dir::Symbol) -> dir::LocalNodeIdAny {
        if !matches!(symbol_at.node_id.ty, dir::NodeType::Expression) {
            return symbol_at.node_id;
        }

        if let Some(declaration) = symbol.declaration {
            declaration.local_id
        } else {
            symbol_at.node_id
        }
    }

    /// Return the signature text for a hover target.
    fn hover_signature(
        &self,
        hover_node_id: dir::LocalNodeIdAny,
        symbol_id: dir::GlobalSymbolId,
        symbol: &dir::Symbol,
    ) -> String {
        let name = symbol.name().map(|id| self.strings().get(id).to_string());
        let container_name = self.symbol_container_name(symbol_id);

        match hover_node_id.ty {
            dir::NodeType::Member => {
                let member_id = hover_node_id.try_into().unwrap_or_else(|_| {
                    panic!(
                        "hover node has member type but invalid id {}",
                        hover_node_id.id
                    )
                });

                self.member_hover(member_id, container_name.as_deref())
            }
            dir::NodeType::EnumField => {
                let field_id = hover_node_id.try_into().unwrap_or_else(|_| {
                    panic!(
                        "hover node has enum field type but invalid id {}",
                        hover_node_id.id
                    )
                });

                self.enum_field_hover(field_id, container_name.as_deref())
            }
            dir::NodeType::Parameter => {
                let parameter_id = hover_node_id.try_into().unwrap_or_else(|_| {
                    panic!(
                        "hover node has parameter type but invalid id {}",
                        hover_node_id.id
                    )
                });

                self.parameter_hover(parameter_id)
            }
            dir::NodeType::Pattern => self.local_variable_hover(name.as_deref(), symbol_id),
            _ => format_simple_signature(symbol.kind, name.as_deref()),
        }
    }

    /// Return type text for a hover target when available.
    fn hover_type_text(
        &self,
        hover_node_id: dir::LocalNodeIdAny,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<String> {
        // resolve the type table
        let types = self.types();

        // map the hover node to a type id
        let type_id = match hover_node_id.ty {
            dir::NodeType::Pattern => Some(
                types
                    .get_symbol_type_id(symbol_id)
                    .unwrap_or_else(|| panic!("missing checked hover type for {symbol_id:?}")),
            ),
            dir::NodeType::Member | dir::NodeType::EnumField | dir::NodeType::Parameter => Some(
                self.node_type_id(hover_node_id)
                    .unwrap_or_else(|| panic!("missing checked hover type for {hover_node_id:?}")),
            ),
            _ => None,
        }?;

        // format the checked type for display
        Some(
            format_global_type(type_id, self)
                .unwrap_or_else(|| panic!("unable to format checked hover type {type_id:?}")),
        )
    }

    /// Resolve the visible hover range for a symbol.
    fn hover_range(&self, node_id: dir::LocalNodeIdAny, default_span: Span) -> Span {
        // preserve full declaration ranges for member declarations
        if node_id.ty == dir::NodeType::Member {
            return self.get_span(self.view(), node_id);
        }

        default_span
    }
}
