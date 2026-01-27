use destack_ast::{AnnotationPosition, Doc};
use destack_dir::{
    self as dir, DynamicKey, EnumField, GlobalNodeIdAny, LocalNodeId, Member, NodeType, Parameter,
    SymbolType,
};
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::format::{format_local_type, format_symbol_signature};
use crate::query::common::{find_symbol_at_offset, get_canonical_symbol};

/// Hover information for a symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        let doc = doc.into();
        if !doc.is_empty() {
            self.documentation = Some(doc);
        }
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

/// Request hover information at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoverRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for hover queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoverResponse {
    /// Hover information, if available.
    pub hover: Option<HoverInfo>,
}

/// Get hover information for the symbol at the given position.
pub fn hover(session: &Session, file: FileId, offset: u32) -> Option<HoverInfo> {
    let symbol_at = find_symbol_at_offset(session, file, offset)?;
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);
    let profile = session.default_profile_for_module(canonical_id.module_id);

    // get documentation for this symbol
    let documentation = get_symbol_documentation(session, canonical_id);

    // try rich signature formatting first (for top-level declarations)
    if let Some(formatted) =
        format_symbol_signature(canonical_id, &session.modules, &session.strings, profile)
    {
        return Some(
            HoverInfo::signature(formatted.text)
                .with_documentation(documentation.unwrap_or_default())
                .with_range(symbol_at.span),
        );
    }

    let module = session.modules.get(symbol_at.symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_at.symbol_id.local_id);
    let name = symbol.name().map(|id| session.strings.get(id).to_string());

    let mut hover_node_id = symbol_at.node_id;
    if matches!(hover_node_id.ty, NodeType::Expression)
        && let Some(declaration) = symbol.primary_declaration
    {
        hover_node_id = declaration.local_id;
    }

    // get container name for members
    let container_name = get_container_name(session, symbol_at.symbol_id);

    // format based on node type
    let module_id = ctx.module_id;
    let signature = match hover_node_id.ty {
        NodeType::Member => {
            let dir_tree = ctx.tree();
            let types = ctx.types();
            if let Ok(member_id) = hover_node_id.try_into() {
                let member = dir_tree.get::<Member>(member_id);
                format_member_hover(
                    session,
                    member,
                    member_id,
                    module_id,
                    &dir_tree,
                    &types,
                    container_name.as_deref(),
                )
            } else {
                format_simple_signature(symbol.ty, name.as_deref())
            }
        }
        NodeType::EnumField => {
            let dir_tree = ctx.tree();
            let types = ctx.types();
            if let Ok(field_id) = hover_node_id.try_into() {
                let field = dir_tree.get::<EnumField>(field_id);
                format_enum_field_hover(
                    session,
                    field,
                    field_id,
                    module_id,
                    &types,
                    container_name.as_deref(),
                )
            } else {
                format_simple_signature(symbol.ty, name.as_deref())
            }
        }
        NodeType::Parameter => {
            let dir_tree = ctx.tree();
            let types = ctx.types();
            if let Ok(param_id) = hover_node_id.try_into() {
                let param = dir_tree.get::<Parameter>(param_id);
                format_parameter_hover(session, param, param_id, module_id, &types)
            } else {
                format_simple_signature(symbol.ty, name.as_deref())
            }
        }
        NodeType::Pattern => {
            // local variable or destructuring pattern
            let types = ctx.types();
            format_local_variable_hover(
                session,
                name.as_deref(),
                symbol_at.symbol_id,
                &symbols,
                &types,
            )
        }
        _ => format_simple_signature(symbol.ty, name.as_deref()),
    };

    drop(symbols);
    drop(module);

    Some(
        HoverInfo::signature(signature)
            .with_documentation(documentation.unwrap_or_default())
            .with_range(symbol_at.span),
    )
}

/// Get documentation comments for a symbol.
fn get_symbol_documentation(session: &Session, symbol_id: dir::GlobalSymbolId) -> Option<String> {
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    // get the primary declaration to find doc comments
    let declaration_ref = symbol.primary_declaration?;
    drop(symbols);

    // get the AST node for the declaration
    let dir_tree = ctx.tree();
    let ast_node_id = dir_tree.get_source(declaration_ref.local_id.id);

    // get doc annotations attached to this AST node
    let docs = ctx.ast.tree.get_docs_for(ast_node_id);
    if docs.is_empty() {
        return None;
    }

    // collect prefix docs (those that appear before the declaration)
    let doc_strings: Vec<String> = docs
        .into_iter()
        .filter(|(_, pos)| {
            matches!(
                pos,
                AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
            )
        })
        .map(|(doc_id, _)| {
            let doc = ctx.ast.tree.get::<Doc>(doc_id);
            ctx.ast.strings.get(doc.string).to_string()
        })
        .collect();

    if doc_strings.is_empty() {
        None
    } else {
        Some(doc_strings.join("\n\n"))
    }
}

/// Get the name of the container (class/struct/interface) for a member.
fn get_container_name(session: &Session, symbol_id: dir::GlobalSymbolId) -> Option<String> {
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let dir = module.dir_base_maybe()?;
    let symbols = dir.symbols.read();

    let symbol = symbols.get_symbol(symbol_id.into_local());
    let scope = symbols.get_scope_by_id(symbol.scope.0);

    // get the owner of the scope (the containing type)
    let owner_id = scope.owner_id?;
    let owner = symbols.get_symbol(owner_id);
    let name_id = owner.name()?;
    Some(session.strings.get(name_id).to_string())
}

// NOTE #Architecture: member hover formatting duplicates the signature formatter
// NOTE #Architecture: consolidate into shared member formatting, differences: kind prefixes, declared or inferred types

/// Format hover for a member (field, method, etc).
fn format_member_hover(
    session: &Session,
    member: &Member,
    member_id: LocalNodeId<Member>,
    module_id: destack_source::ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    container: Option<&str>,
) -> String {
    // get member name from key field
    let member_name = member
        .key()
        .and_then(|key| match key {
            DynamicKey::Name(string_id) => Some(session.strings.get(*string_id).to_string()),
            DynamicKey::Number(string_id) => Some(session.strings.get(*string_id).to_string()),
            DynamicKey::NamedExpression { name, .. } => {
                Some(session.strings.get(*name).to_string())
            }
            DynamicKey::Expression(_) => None,
        })
        .unwrap_or_else(|| "<anonymous>".to_string());

    // get type if available
    let node_id = GlobalNodeIdAny {
        module_id,
        local_id: member_id.into(),
    };
    let type_str = types
        .get_declared_or_inferred_type_id(node_id)
        .map(|type_id| format_local_type(type_id, types, &session.modules, &session.strings));

    // format with container prefix
    let qualified_name = match container {
        Some(c) => format!("{c}.{member_name}"),
        None => member_name,
    };

    match member {
        Member::Type { .. } => {
            if let Some(ty) = type_str {
                format!("(type member) {qualified_name} = {ty}")
            } else {
                format!("(type member) {qualified_name}")
            }
        }
        Member::Field { .. } => {
            if let Some(ty) = type_str {
                format!("(property) {qualified_name}: {ty}")
            } else {
                format!("(property) {qualified_name}")
            }
        }
        Member::Method { signature, .. } => format_method_hover(
            &qualified_name,
            signature,
            module_id,
            dir_tree,
            types,
            session,
        ),
        Member::Embed { .. } => format!("(embed) {qualified_name}"),
        Member::StaticBlock { .. } => "(static block)".to_string(),
        Member::ComptimeBlock { .. } => "(comptime block)".to_string(),
    }
}

/// Format hover for a method with full signature.
fn format_method_hover(
    qualified_name: &str,
    signature: &dir::FunctionSignature,
    module_id: destack_source::ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    session: &Session,
) -> String {
    let modules = &session.modules;
    let strings = &session.strings;

    // async prefix
    let async_prefix = match signature.asynchrony {
        dir::Asynchrony::Async => "async ",
        dir::Asynchrony::Sync => "",
    };

    // generic parameters
    let generics_text = signature
        .generics
        .as_ref()
        .map(|g| format_generics_for_hover(g, module_id, dir_tree, types, session))
        .unwrap_or_default();

    // dynamic parameters
    let params_text = format_params_for_hover(
        &signature.dynamic_parameters,
        module_id,
        dir_tree,
        types,
        session,
    );

    // return type
    let return_text = signature
        .return_type
        .and_then(|return_node| {
            let node_id = GlobalNodeIdAny {
                module_id,
                local_id: return_node.into(),
            };
            types.get_declared_or_inferred_type_id(node_id)
        })
        .map(|type_id| format!(": {}", format_local_type(type_id, types, modules, strings)))
        .unwrap_or_default();

    format!("(method) {async_prefix}{qualified_name}{generics_text}({params_text}){return_text}")
}

/// Format generic parameters for hover.
fn format_generics_for_hover(
    generics: &dir::Generics,
    module_id: destack_source::ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    session: &Session,
) -> String {
    let Some(parameters) = &generics.static_parameters else {
        return String::new();
    };
    if parameters.is_empty() {
        return String::new();
    }

    let formatted: Vec<_> = parameters
        .iter()
        .map(|param_id| {
            let param = dir_tree.get::<Parameter>(*param_id);
            format_type_param_for_hover(param, *param_id, module_id, types, session)
        })
        .collect();

    format!("<{}>", formatted.join(", "))
}

/// Format a single type parameter for hover.
fn format_type_param_for_hover(
    param: &Parameter,
    param_id: LocalNodeId<Parameter>,
    module_id: destack_source::ModuleId,
    types: &dir::TypeTable,
    session: &Session,
) -> String {
    let name = match param {
        Parameter::Named { name, .. } => session.strings.get(*name).to_string(),
        Parameter::Pattern { .. } => "_".to_string(),
        Parameter::Variadic { name, .. } => format!("...{}", session.strings.get(*name).as_str()),
    };

    // try to get constraint type
    let node_id = GlobalNodeIdAny {
        module_id,
        local_id: param_id.into(),
    };

    if let Some(type_id) = types.get_declared_or_inferred_type_id(node_id) {
        let type_text = format_local_type(type_id, types, &session.modules, &session.strings);
        format!("{name} extends {type_text}")
    } else {
        name
    }
}

/// Format function parameters for hover.
fn format_params_for_hover(
    parameters: &[LocalNodeId<Parameter>],
    module_id: destack_source::ModuleId,
    dir_tree: &dir::NodeTree,
    types: &dir::TypeTable,
    session: &Session,
) -> String {
    let formatted: Vec<_> = parameters
        .iter()
        .map(|param_id| {
            let param = dir_tree.get::<Parameter>(*param_id);
            format_param_for_hover(param, *param_id, module_id, types, session)
        })
        .collect();

    formatted.join(", ")
}

/// Format a single parameter for hover.
fn format_param_for_hover(
    param: &Parameter,
    param_id: LocalNodeId<Parameter>,
    module_id: destack_source::ModuleId,
    types: &dir::TypeTable,
    session: &Session,
) -> String {
    let name = match param {
        Parameter::Named { name, .. } => session.strings.get(*name).to_string(),
        Parameter::Pattern { .. } => "_".to_string(),
        Parameter::Variadic { name, .. } => format!("...{}", session.strings.get(*name).as_str()),
    };

    let node_id = GlobalNodeIdAny {
        module_id,
        local_id: param_id.into(),
    };

    // use declared type (from annotation) or inferred type
    if let Some(type_id) = types.get_declared_or_inferred_type_id(node_id) {
        let type_text = format_local_type(type_id, types, &session.modules, &session.strings);
        format!("{name}: {type_text}")
    } else {
        name
    }
}

/// Format hover for an enum field.
fn format_enum_field_hover(
    session: &Session,
    field: &EnumField,
    field_id: LocalNodeId<EnumField>,
    module_id: destack_source::ModuleId,
    types: &dir::TypeTable,
    container: Option<&str>,
) -> String {
    let field_name = session.strings.get(field.name).to_string();

    let qualified_name = match container {
        Some(c) => format!("{c}.{field_name}"),
        None => field_name,
    };

    // try to get the value/discriminant
    let node_id = GlobalNodeIdAny {
        module_id,
        local_id: field_id.into(),
    };

    if let Some(type_id) = types.get_declared_or_inferred_type_id(node_id) {
        let type_text = format_local_type(type_id, types, &session.modules, &session.strings);
        format!("(enum member) {qualified_name} = {type_text}")
    } else {
        format!("(enum member) {qualified_name}")
    }
}

/// Format hover for a parameter.
fn format_parameter_hover(
    session: &Session,
    param: &Parameter,
    param_id: LocalNodeId<Parameter>,
    module_id: destack_source::ModuleId,
    types: &dir::TypeTable,
) -> String {
    let name = match param {
        Parameter::Named { name, .. } => session.strings.get(*name).to_string(),
        Parameter::Pattern { .. } => "_".to_string(),
        Parameter::Variadic { name, .. } => format!("...{}", session.strings.get(*name).as_str()),
    };

    let node_id = GlobalNodeIdAny {
        module_id,
        local_id: param_id.into(),
    };

    if let Some(type_id) = types.get_declared_or_inferred_type_id(node_id) {
        let type_text = format_local_type(type_id, types, &session.modules, &session.strings);
        format!("(parameter) {name}: {type_text}")
    } else {
        format!("(parameter) {name}")
    }
}

/// Format hover for a local variable.
fn format_local_variable_hover(
    session: &Session,
    name: Option<&str>,
    symbol_id: dir::GlobalSymbolId,
    symbols: &dir::SymbolTable,
    types: &dir::TypeTable,
) -> String {
    let name = name.unwrap_or("<anonymous>");

    // local variable types are stored as value types on the symbol, not on the node
    if let Some(type_id) = types.get_type_id_for_symbol(symbols, symbol_id) {
        let type_text = format_local_type(type_id, types, &session.modules, &session.strings);
        format!("let {name}: {type_text}")
    } else {
        format!("let {name}")
    }
}

/// Simple signature formatting for symbols without richer context.
fn format_simple_signature(symbol_type: SymbolType, name: Option<&str>) -> String {
    let name = name.unwrap_or("<anonymous>");

    match symbol_type {
        SymbolType::Void => format!("let {name}"),
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
