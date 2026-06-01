use destack_dir as dir;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::core::{ModuleQueryContext, QueryPosition};
use crate::format::{
    format_enum_field_hover, format_global_type, format_hover_markdown,
    format_local_variable_hover, format_member_hover, format_parameter_hover,
    format_simple_signature, format_symbol_signature,
};

/// Hover payload for a source position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Create hover payload with just a signature.
    pub fn signature(signature: impl Into<String>) -> Self {
        // build the base hover info
        Self {
            signature: signature.into(),
            documentation: None,
            type_text: None,
            location: None,
            range: None,
        }
    }

    /// Add documentation.
    pub fn with_documentation(mut self, doc: impl Into<String>) -> Self {
        // normalize and store documentation
        let doc = doc.into();
        if !doc.is_empty() {
            self.documentation = Some(doc);
        }
        self
    }

    /// Add resolved type text.
    pub fn with_type_text(mut self, type_text: Option<String>) -> Self {
        // store the type text when non empty
        let type_text = type_text.filter(|text| !text.trim().is_empty());
        if let Some(type_text) = type_text {
            self.type_text = Some(type_text);
        }
        self
    }

    /// Add location text.
    pub fn with_location(mut self, location: Option<String>) -> Self {
        // store the location text when non empty
        let location = location.filter(|text| !text.trim().is_empty());
        if let Some(location) = location {
            self.location = Some(location);
        }
        self
    }

    /// Add range.
    pub fn with_range(mut self, range: Span) -> Self {
        // store the hovered range
        self.range = Some(range);
        self
    }

    /// Format as markdown for display.
    pub fn to_markdown(&self) -> String {
        // format the hover into markdown
        format_hover_markdown(
            &self.signature,
            self.type_text.as_deref(),
            self.documentation.as_deref(),
            self.location.as_deref(),
        )
    }
}

/// Request hover information at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoverRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for hover queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoverResponse {
    /// Hover information, if available.
    pub hover: Option<Hover>,
}

impl ModuleQueryContext<'_> {
    /// Get hover information for the symbol at the given position.
    pub fn hover(&self, offset: u32) -> Option<Hover> {
        let ctx = self;
        let symbol_at = ctx.symbol_for_hover_at_offset(offset)?;
        let canonical_id = ctx.canonical_symbol(symbol_at.symbol_id);

        // get documentation for this symbol
        let documentation = ctx.doc_text_for_symbol(canonical_id);

        // try rich signature formatting first (for top level declarations)
        if let Some(formatted) = format_symbol_signature(ctx, canonical_id) {
            // resolve the hover location
            let location = ctx.hover_location(symbol_at.span);

            // return a minimal hover payload
            return Some(
                Hover::signature(formatted.text)
                    .with_documentation(documentation.unwrap_or_default())
                    .with_location(location)
                    .with_range(symbol_at.span),
            );
        }

        // resolve module query context for richer formatting
        // resolve symbol metadata
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_at.symbol_id.local_id);
        let name = symbol
            .name()
            .map(|id| ctx.dir().strings().get(id).to_string());

        // prefer declaration nodes for expression hovers
        let mut hover_node_id = symbol_at.node_id;
        if matches!(hover_node_id.ty, dir::NodeType::Expression) {
            let declaration = symbol.declaration;
            if let Some(declaration) = declaration {
                hover_node_id = declaration.local_id;
            }
        }

        // get container name for members
        let container_name = ctx.container_name_for_symbol(symbol_at.symbol_id);

        // resolve shared dir data for formatting
        let dir_tree = ctx.dir().view();
        let types = ctx.dir().types();

        // format based on node type
        let module_id = ctx.module_id();
        let signature = match hover_node_id.ty {
            dir::NodeType::Member => {
                if let Ok(member_id) = hover_node_id.try_into() {
                    // format member hover with full signature
                    let member = dir_tree.get::<dir::Member>(member_id);
                    format_member_hover(
                        ctx.dir().strings(),
                        ctx,
                        member,
                        member_id,
                        module_id,
                        dir_tree,
                        types,
                        container_name.as_deref(),
                    )
                } else {
                    format_simple_signature(symbol.kind, name.as_deref())
                }
            }
            dir::NodeType::EnumField => {
                if let Ok(field_id) = hover_node_id.try_into() {
                    // format enum field hover
                    let field = dir_tree.get::<dir::EnumField>(field_id);
                    format_enum_field_hover(
                        ctx.dir().strings(),
                        ctx,
                        field,
                        field_id,
                        module_id,
                        types,
                        container_name.as_deref(),
                    )
                } else {
                    format_simple_signature(symbol.kind, name.as_deref())
                }
            }
            dir::NodeType::Parameter => {
                if let Ok(param_id) = hover_node_id.try_into() {
                    // format parameter hover
                    let param = dir_tree.get::<dir::Parameter>(param_id);
                    format_parameter_hover(
                        ctx.dir().strings(),
                        ctx,
                        param,
                        param_id,
                        module_id,
                        types,
                    )
                } else {
                    format_simple_signature(symbol.kind, name.as_deref())
                }
            }
            dir::NodeType::Pattern => {
                // local variable or destructuring pattern
                format_local_variable_hover(name.as_deref(), symbol_at.symbol_id, types, ctx)
            }
            _ => format_simple_signature(symbol.kind, name.as_deref()),
        };

        // resolve type and location metadata
        let type_text = ctx.resolve_hover_type_text(hover_node_id, symbol_at.symbol_id);
        let location = ctx.hover_location(symbol_at.span);
        let range = ctx.hover_range_for_symbol(symbol_at.node_id, symbol_at.span);

        // return the assembled hover payload
        Some(
            Hover::signature(signature)
                .with_documentation(documentation.unwrap_or_default())
                .with_type_text(type_text)
                .with_location(location)
                .with_range(range),
        )
    }

    /// Format a source location string for a hover span.
    fn hover_location(&self, span: Span) -> Option<String> {
        let file = self
            .repository()
            .file(self.revision(), span.file)
            .ok()
            .flatten()?;
        let path = file.path.as_ref()?;
        let (line, col) = file.get_position(span.start)?;

        Some(format!(
            "{}:{}:{}",
            path.to_string_lossy(),
            line + 1,
            col + 1
        ))
    }

    /// Resolve a type string for a hover target when available.
    fn resolve_hover_type_text(
        &self,
        hover_node_id: dir::LocalNodeIdAny,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<String> {
        let ctx = self;
        // resolve the type table
        let types = ctx.dir().types();

        // map the hover node to a type id
        let type_id = match hover_node_id.ty {
            dir::NodeType::Pattern => types.get_symbol_type_id(symbol_id),
            dir::NodeType::Member | dir::NodeType::EnumField | dir::NodeType::Parameter => {
                ctx.dir().node_type_id(hover_node_id)
            }
            _ => None,
        }?;

        // format the checked type for display
        Some(format_global_type(type_id, ctx))
    }

    /// Resolve the visible hover range for a symbol.
    fn hover_range_for_symbol(&self, node_id: dir::LocalNodeIdAny, default_span: Span) -> Span {
        let ctx = self;
        // preserve full declaration ranges for member declarations
        if node_id.ty == dir::NodeType::Member {
            return ctx.dir().span_for_dir_node(ctx.dir().view(), node_id);
        }

        default_span
    }
}
