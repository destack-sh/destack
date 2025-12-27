use destack_dir::{
    DynamicKey, EnumField, GlobalNodeIdAny, Member, NodeType, Parameter, SymbolType,
};
use destack_source::{FileId, Span};

use crate::Session;
use crate::format::{format_local_type, format_symbol_signature};
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
    let profile = session.default_profile_for_module(symbol_at.symbol_id.module_id);

    // try rich signature formatting first (for declarations)
    if let Some(formatted) = format_symbol_signature(
        symbol_at.symbol_id,
        &session.modules,
        &session.strings,
        profile,
    ) {
        return Some(HoverInfo::signature(formatted.text).with_range(symbol_at.span));
    }

    // fallback: simple formatting for locals and other symbols
    let module = session.modules.get(symbol_at.symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_at.symbol_id.local_id);
    let name = symbol.name().map(|id| ctx.ast.strings.get(id).to_string());

    // use node type to provide better context for members/fields/parameters
    let module_id = ctx.module_id;
    let signature = match symbol_at.node_id.ty {
        NodeType::Member => {
            let dir_tree = ctx.tree();
            let types = ctx.types();
            if let Ok(member_id) = symbol_at.node_id.try_into() {
                let member = dir_tree.get::<Member>(member_id);

                // get member name from key field
                let member_name = member.key().and_then(|key| match key {
                    DynamicKey::Name(string_id) => Some(ctx.ast.strings.get(*string_id).to_string()),
                    DynamicKey::NamedExpression { name, .. } => {
                        Some(ctx.ast.strings.get(*name).to_string())
                    }
                    DynamicKey::Expression(_) => None,
                });

                // get type if available
                let node_id = GlobalNodeIdAny {
                    module_id,
                    local_id: member_id.into(),
                };
                let type_str = types.get_inferred_type_id(node_id).map(|type_id| {
                    format_local_type(type_id, &types, &session.modules, &session.strings)
                });

                format_member_signature(member, member_name.as_deref(), type_str.as_deref())
            } else {
                format_simple_signature(symbol.ty, name.as_deref())
            }
        }
        NodeType::EnumField => {
            let dir_tree = ctx.tree();
            if let Ok(field_id) = symbol_at.node_id.try_into() {
                let field = dir_tree.get::<EnumField>(field_id);
                let field_name = ctx.ast.strings.get(field.name).to_string();
                format!("enum field {field_name}")
            } else {
                format_simple_signature(symbol.ty, name.as_deref())
            }
        }
        NodeType::Parameter => {
            let dir_tree = ctx.tree();
            let types = ctx.types();
            if let Ok(param_id) = symbol_at.node_id.try_into() {
                let _param = dir_tree.get::<Parameter>(param_id);
                let param_name = name.as_deref().unwrap_or("<anonymous>");

                // get type if available
                let node_id = GlobalNodeIdAny {
                    module_id,
                    local_id: param_id.into(),
                };
                let type_str = types.get_inferred_type_id(node_id).map(|type_id| {
                    format!(
                        ": {}",
                        format_local_type(type_id, &types, &session.modules, &session.strings)
                    )
                });

                format!("parameter {param_name}{}", type_str.unwrap_or_default())
            } else {
                format_simple_signature(symbol.ty, name.as_deref())
            }
        }
        _ => format_simple_signature(symbol.ty, name.as_deref()),
    };

    Some(HoverInfo::signature(signature).with_range(symbol_at.span))
}

/// Simple signature formatting for locals and symbols without declarations.
fn format_simple_signature(symbol_type: SymbolType, name: Option<&str>) -> String {
    let name = name.unwrap_or("<anonymous>");

    match symbol_type {
        SymbolType::Void => format!("void {name}"),
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

/// Format signature for a member (field, method, embed, static block).
fn format_member_signature(member: &Member, name: Option<&str>, type_str: Option<&str>) -> String {
    let name = name.unwrap_or("<anonymous>");
    match member {
        Member::Field { .. } => {
            if let Some(ty) = type_str {
                format!("field {name}: {ty}")
            } else {
                format!("field {name}")
            }
        }
        Member::Method { .. } => format!("method {name}"),
        Member::Embed { .. } => format!("embed {name}"),
        Member::StaticBlock { .. } => "static block".to_string(),
    }
}
