use destack_dir::{
    self as dir, DynamicKey, EnumField, GlobalNodeIdAny, LocalNodeId, Member, NodeType, Parameter,
    SymbolType,
};
use destack_source::{FileId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::Session;
use crate::format::{
    format_call_signature, format_hover_markdown, format_local_type, format_symbol_signature,
};
use crate::query::common::{doc_text_for_node, find_symbol_at_offset, get_canonical_symbol};

/// Hover information for a symbol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoverInfo {
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

impl HoverInfo {
    /// Create hover info with just a signature.
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
    // resolve the hovered symbol and profile
    let symbol_at = find_symbol_at_offset(session, file, offset)?;
    let canonical_id = get_canonical_symbol(session, symbol_at.symbol_id);
    let profile = session.default_profile_for_module(canonical_id.module_id);

    // get documentation for this symbol
    let documentation = get_symbol_documentation(session, canonical_id);

    // try rich signature formatting first (for top level declarations)
    if let Some(formatted) =
        format_symbol_signature(canonical_id, &session.modules, &session.strings, profile)
    {
        // resolve the hover location
        let location = hover_location(session, symbol_at.span);

        // return a minimal hover payload
        return Some(
            HoverInfo::signature(formatted.text)
                .with_documentation(documentation.unwrap_or_default())
                .with_location(location)
                .with_range(symbol_at.span),
        );
    }

    // resolve module query context for richer formatting
    let module = session.modules.get(symbol_at.symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // resolve symbol metadata
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_at.symbol_id.local_id);
    let name = symbol.name().map(|id| session.strings.get(id).to_string());

    // prefer declaration nodes for expression hovers
    let mut hover_node_id = symbol_at.node_id;
    if matches!(hover_node_id.ty, NodeType::Expression) {
        let declaration = symbol.primary_declaration;
        if let Some(declaration) = declaration {
            hover_node_id = declaration.local_id;
        }
    }

    // get container name for members
    let container_name = get_container_name(session, symbol_at.symbol_id);

    // resolve shared dir data for formatting
    let dir_tree = ctx.tree();
    let types = ctx.types();

    // format based on node type
    let module_id = ctx.module_id;
    let signature = match hover_node_id.ty {
        NodeType::Member => {
            if let Ok(member_id) = hover_node_id.try_into() {
                // format member hover with full signature
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
            if let Ok(field_id) = hover_node_id.try_into() {
                // format enum field hover
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
            if let Ok(param_id) = hover_node_id.try_into() {
                // format parameter hover
                let param = dir_tree.get::<Parameter>(param_id);
                format_parameter_hover(session, param, param_id, module_id, &types)
            } else {
                format_simple_signature(symbol.ty, name.as_deref())
            }
        }
        NodeType::Pattern => {
            // local variable or destructuring pattern
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

    // resolve type and location metadata
    let type_text =
        resolve_hover_type_text(session, &ctx, &symbols, hover_node_id, symbol_at.symbol_id);
    let location = hover_location(session, symbol_at.span);

    // return the assembled hover payload
    Some(
        HoverInfo::signature(signature)
            .with_documentation(documentation.unwrap_or_default())
            .with_type_text(type_text)
            .with_location(location)
            .with_range(symbol_at.span),
    )
}

/// Get documentation comments for a symbol.
fn get_symbol_documentation(session: &Session, symbol_id: dir::GlobalSymbolId) -> Option<String> {
    // resolve the module query context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // resolve the target symbol
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    // get the primary declaration to find doc comments
    let declaration_ref = symbol.primary_declaration?;
    drop(symbols);

    // get the AST node for the declaration
    let dir_tree = ctx.tree();
    let ast_node_id = dir_tree.get_source(declaration_ref.local_id.id);

    // try to collect docs on the declaration node first
    if let Some(doc_text) = doc_text_for_node(ctx.ast, ast_node_id) {
        return Some(doc_text);
    }

    // fall back to enclosing nodes when docs are attached to wrapper expressions
    let declaration_span = ctx.ast.tree.source_map.get_main_or_enclosing(ast_node_id);
    let mut enclosing = ctx.ast.tree.source_map.get_enclosing_spans(
        declaration_span.start,
        declaration_span.end.saturating_sub(1),
    );

    // sort so the innermost nodes are checked first
    enclosing.sort_by_key(|span| span.length);

    // walk enclosing spans until documentation is found
    for span in enclosing {
        if span.idx == ast_node_id {
            continue;
        }

        if let Some(doc_text) = doc_text_for_node(ctx.ast, span.idx) {
            return Some(doc_text);
        }
    }

    None
}

/// Get the name of the container (class/struct/interface) for a member.
fn get_container_name(session: &Session, symbol_id: dir::GlobalSymbolId) -> Option<String> {
    // resolve the base dir for the module
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let dir = module.dir_base_maybe()?;
    let symbols = dir.symbols.read();

    // resolve the symbol and owning scope
    let symbol = symbols.get_symbol(symbol_id.into_local());
    let scope = symbols.get_scope_by_id(symbol.scope.0);

    // get the owner of the scope (the containing type)
    let owner_id = scope.owner_id?;
    let owner = symbols.get_symbol(owner_id);
    let name_id = owner.name()?;

    // return the container name
    Some(session.strings.get(name_id).to_string())
}

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
    // resolve the member name from its key
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

    // format the hover text by member kind
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
    // format a call signature label for the method
    let formatted = format_call_signature(
        qualified_name,
        signature,
        module_id,
        dir_tree,
        types,
        &session.modules,
        &session.strings,
        false,
    );

    // prefix the label with the member kind
    format!("(method) {}", formatted.label)
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
    // resolve the enum field name
    let field_name = session.strings.get(field.name).to_string();

    // build a qualified name when a container is available
    let qualified_name = match container {
        Some(c) => format!("{c}.{field_name}"),
        None => field_name,
    };

    // try to get the value or discriminant
    let node_id = GlobalNodeIdAny {
        module_id,
        local_id: field_id.into(),
    };
    if let Some(type_id) = types.get_declared_or_inferred_type_id(node_id) {
        // format the enum field with its value type
        let type_text = format_local_type(type_id, types, &session.modules, &session.strings);
        format!("(enum member) {qualified_name} = {type_text}")
    } else {
        // format the enum field without a value
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
    // derive a display name for the parameter
    let name = match param {
        Parameter::Named { name, .. } => session.strings.get(*name).to_string(),
        Parameter::Pattern { .. } => "_".to_string(),
        Parameter::Variadic { name, .. } => format!("...{}", session.strings.get(*name).as_str()),
    };

    // resolve the parameter node id for type lookup
    let node_id = GlobalNodeIdAny {
        module_id,
        local_id: param_id.into(),
    };
    if let Some(type_id) = types.get_declared_or_inferred_type_id(node_id) {
        // format the parameter with its type
        let type_text = format_local_type(type_id, types, &session.modules, &session.strings);
        format!("(parameter) {name}: {type_text}")
    } else {
        // format the parameter without a type
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
    // resolve the display name
    let name = name.unwrap_or("<anonymous>");

    // local variable types are stored as value types on the symbol, not on the node
    if let Some(type_id) = types.get_type_id_for_symbol(symbols, symbol_id) {
        // format the local binding with its inferred type
        let type_text = format_local_type(type_id, types, &session.modules, &session.strings);
        format!("let {name}: {type_text}")
    } else {
        // format the local binding without a type
        format!("let {name}")
    }
}

/// Simple signature formatting for symbols without richer context.
fn format_simple_signature(symbol_type: SymbolType, name: Option<&str>) -> String {
    // resolve the display name
    let name = name.unwrap_or("<anonymous>");

    // map the symbol type to a simple signature
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

/// Resolve a type string for a hover target when available.
fn resolve_hover_type_text(
    session: &Session,
    ctx: &crate::query::common::QueryContext<'_>,
    symbols: &dir::SymbolTable,
    hover_node_id: dir::LocalNodeIdAny,
    symbol_id: dir::GlobalSymbolId,
) -> Option<String> {
    // resolve the type table
    let types = ctx.types();

    // map the hover node to a type id
    let type_id = match hover_node_id.ty {
        NodeType::Pattern => types.get_type_id_for_symbol(symbols, symbol_id),
        NodeType::Member | NodeType::EnumField | NodeType::Parameter => {
            ctx.get_node_type(hover_node_id)
        }
        _ => None,
    }?;

    // format the local type for display
    Some(format_local_type(
        type_id,
        &types,
        &session.modules,
        &session.strings,
    ))
}

/// Format a source location string for a hover span.
fn hover_location(session: &Session, span: Span) -> Option<String> {
    // resolve the source file and path
    let file = session.files.get(span.file);
    let path = file.path.as_ref()?;
    let (line, col) = file.get_position(span.start)?;

    // format as path and 1 based coordinates
    Some(format!(
        "{}:{}:{}",
        path.to_string_lossy(),
        line + 1,
        col + 1
    ))
}
