use destack_ast as ast;
use destack_core::StringPool;
use destack_dir::{self as dir, Declaration, EnumField, LocalNodeId, Member, TypeMember};
use destack_source::{FileId, Span, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};

pub use crate::SymbolKind;
use crate::ast::{main_span_for_dir_node, span_for_dir_node};
use crate::core::{
    AstQueryContext, QueryContext, with_ast_query_for_file, with_query_context_for_file,
};
use crate::dir::{
    declaration_display_name, declaration_symbol_kind, is_synthetic_function_keyword_field,
    member_key_name, member_symbol_kind, type_member_symbol_kind,
};

/// A symbol in a document (for outline view).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentSymbol {
    /// The symbol's name.
    pub name: String,
    /// Additional detail (e.g., signature).
    pub detail: Option<String>,
    /// The kind of symbol.
    pub kind: SymbolKind,
    /// The full range of the symbol (including body).
    pub range: Span,
    /// The range of the symbol's name.
    pub selection_range: Span,
    /// Children symbols (for hierarchical outline).
    pub children: Vec<DocumentSymbol>,
}

impl DocumentSymbol {
    /// Create a new document symbol.
    pub fn new(name: impl Into<String>, kind: SymbolKind, range: Span) -> Self {
        Self {
            name: name.into(),
            detail: None,
            kind,
            range,
            selection_range: range,
            children: Vec::new(),
        }
    }

    /// Set the detail.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the selection range.
    pub fn with_selection_range(mut self, selection_range: Span) -> Self {
        self.selection_range = selection_range;
        self
    }

    /// Add a child symbol.
    pub fn with_child(mut self, child: DocumentSymbol) -> Self {
        self.children.push(child);
        self
    }
}

/// Request document symbols for a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentSymbolsRequest {
    /// The document URI.
    pub uri: Uri,
}

/// Response payload for document symbols queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentSymbolsResponse {
    /// Document symbols.
    pub symbols: Vec<DocumentSymbol>,
}

/// Get all symbols in a document (for outline view).
pub fn document_symbols(
    repository: &Repository,
    revision: Revision,
    file: FileId,
) -> Vec<DocumentSymbol> {
    // prefer using dir symbols when available
    if let Some(symbols) = document_symbols_with_dir(repository, revision, file) {
        return symbols;
    }

    // fall back to ast only symbols
    document_symbols_with_ast(repository, revision, file)
}

/// Build document symbols using DIR data when available.
fn document_symbols_with_dir(
    repository: &Repository,
    revision: Revision,
    file: FileId,
) -> Option<Vec<DocumentSymbol>> {
    with_query_context_for_file(repository, revision, file, |ctx| {
        // resolve the dir tree
        let dir_tree = ctx.dir().view();

        // collect document symbols
        let mut symbols = Vec::new();

        // iterate through all declarations
        for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<Declaration>() {
            // get the kind based on declaration type
            let kind = declaration_symbol_kind(declaration);

            // get the declaration name
            let name = declaration_display_name(ctx.dir().strings(), declaration);

            // resolve the full range and the main selection range
            let range = span_for_dir_node(ctx.ast(), dir_tree, declaration_id.into());
            let selection_range =
                main_span_for_dir_node(ctx.ast(), dir_tree, declaration_id.into()).unwrap_or(range);

            // build the document symbol
            let mut symbol =
                DocumentSymbol::new(name, kind, range).with_selection_range(selection_range);

            // add children for declarations with members
            if let Some(member_ids) = declaration.member_ids() {
                for member_id in member_ids {
                    if let Some(child) = member_to_document_symbol(dir_tree, *member_id, &ctx) {
                        symbol = symbol.with_child(child);
                    }
                }
            }

            if let Some(member_ids) = declaration.type_member_ids() {
                for member_id in member_ids {
                    if let Some(child) = type_member_to_document_symbol(dir_tree, *member_id, &ctx)
                    {
                        symbol = symbol.with_child(child);
                    }
                }
            }

            // add enum field children
            if let Declaration::Enum(declaration) = declaration {
                for field_id in &declaration.fields {
                    if let Some(child) = enum_field_to_document_symbol(dir_tree, *field_id, &ctx) {
                        symbol = symbol.with_child(child);
                    }
                }
            }

            symbols.push(symbol);
        }

        symbols
    })
}

/// Build document symbols using AST data when DIR is unavailable.
fn document_symbols_with_ast(
    repository: &Repository,
    revision: Revision,
    file: FileId,
) -> Vec<DocumentSymbol> {
    with_ast_query_for_file(repository, revision, file, |ast| {
        // collect document symbols
        let mut symbols = Vec::new();

        // iterate through all declarations
        for declaration_id in ast.tree().iter_nodes::<ast::Declaration>() {
            let declaration = ast.tree().get(declaration_id);

            // resolve kind and name
            let kind = declaration_symbol_kind_ast(declaration);
            let name = declaration_display_name_ast(ast.strings(), declaration);

            // resolve the full range and the main selection range
            let range = ast.source_map().get(declaration_id.id);
            let selection_range = ast.tree().get_main_span(declaration_id).unwrap_or(range);

            // build the document symbol
            let mut symbol =
                DocumentSymbol::new(name, kind, range).with_selection_range(selection_range);

            // declaration members
            if let Some(member_ids) = declaration.member_ids() {
                for member_id in member_ids {
                    if let Some(child) = member_to_document_symbol_ast(ast, *member_id) {
                        symbol = symbol.with_child(child);
                    }
                }
            }

            // type members
            if let Some(member_ids) = declaration.type_member_ids() {
                for member_id in member_ids {
                    if let Some(child) = type_member_to_document_symbol_ast(ast, *member_id) {
                        symbol = symbol.with_child(child);
                    }
                }
            }

            // enum fields
            if let ast::Declaration::Enum(declaration) = declaration {
                for field_id in &declaration.fields {
                    if let Some(child) = enum_field_to_document_symbol_ast(ast, *field_id) {
                        symbol = symbol.with_child(child);
                    }
                }
            }

            // store the symbol
            symbols.push(symbol);
        }

        symbols
    })
    .unwrap_or_default()
}

/// Convert a member to a document symbol.
fn member_to_document_symbol(
    dir_tree: dir::View<'_>,
    member_id: LocalNodeId<Member>,
    ctx: &QueryContext,
) -> Option<DocumentSymbol> {
    let member = dir_tree.get::<Member>(member_id);
    let range = span_for_dir_node(ctx.ast(), dir_tree, member_id.into());

    // get the member name and symbol kind
    let (name, kind) = match member {
        Member::AssociatedType { name, .. } => {
            let name = ctx.dir().strings().get(*name).to_string();
            (name, SymbolKind::TypeParameter)
        }
        Member::AssociatedConst { name, .. } => {
            let name = ctx.dir().strings().get(*name).to_string();
            (name, SymbolKind::Constant)
        }
        _ => {
            let key = member.key()?;
            let name = member_key_name(ctx.dir().strings(), key)?;
            let kind = member_symbol_kind(member)?;
            (name, kind)
        }
    };

    // skip synthetic function keyword fields for methods
    if is_synthetic_function_keyword_field(member, &name, range) {
        return None;
    }

    // get spans
    // get main span (name span) if available
    let selection_range =
        main_span_for_dir_node(ctx.ast(), dir_tree, member_id.into()).unwrap_or(range);

    Some(DocumentSymbol::new(name, kind, range).with_selection_range(selection_range))
}

/// Convert a type member to a document symbol.
fn type_member_to_document_symbol(
    dir_tree: dir::View<'_>,
    member_id: LocalNodeId<TypeMember>,
    ctx: &QueryContext,
) -> Option<DocumentSymbol> {
    let member = dir_tree.get::<TypeMember>(member_id);
    let range = span_for_dir_node(ctx.ast(), dir_tree, member_id.into());

    // resolve the type member name and symbol kind
    let kind = type_member_symbol_kind(member)?;
    let name = if let Some(name) = member.name() {
        ctx.dir().strings().get(name).to_string()
    } else {
        let key = member.key()?;
        member_key_name(ctx.dir().strings(), key)?
    };

    // get spans
    let selection_range =
        main_span_for_dir_node(ctx.ast(), dir_tree, member_id.into()).unwrap_or(range);

    Some(DocumentSymbol::new(name, kind, range).with_selection_range(selection_range))
}

/// Convert an enum field to a document symbol.
fn enum_field_to_document_symbol(
    dir_tree: dir::View<'_>,
    field_id: LocalNodeId<EnumField>,
    ctx: &QueryContext,
) -> Option<DocumentSymbol> {
    let field = dir_tree.get::<EnumField>(field_id);

    // get the field name
    let name = ctx.dir().strings().get(field.name.string()).to_string();

    // get spans
    let range = span_for_dir_node(ctx.ast(), dir_tree, field_id.into());

    // get main span (name span) if available
    let selection_range =
        main_span_for_dir_node(ctx.ast(), dir_tree, field_id.into()).unwrap_or(range);

    Some(
        DocumentSymbol::new(name, SymbolKind::EnumMember, range)
            .with_selection_range(selection_range),
    )
}

/// Resolve the display name for an AST declaration.
fn declaration_display_name_ast(strings: &StringPool, declaration: &ast::Declaration) -> String {
    // default block declarations to keyword labels
    if matches!(declaration, ast::Declaration::Global(_)) {
        return "global".to_string();
    }
    if matches!(declaration, ast::Declaration::Module(_)) {
        return "module".to_string();
    }

    // prefer the explicit declaration name when available
    declaration
        .name()
        .map(|name| strings.get(name.string()).to_string())
        .unwrap_or_else(|| "<anonymous>".to_string())
}

/// Map an AST declaration to a symbol kind.
fn declaration_symbol_kind_ast(declaration: &ast::Declaration) -> SymbolKind {
    match declaration {
        ast::Declaration::Global(_) => SymbolKind::Namespace,
        ast::Declaration::Module(_) => SymbolKind::Namespace,
        ast::Declaration::Function(_) => SymbolKind::Function,
        ast::Declaration::Struct(_) => SymbolKind::Struct,
        ast::Declaration::Class(_) => SymbolKind::Class,
        ast::Declaration::Interface(_) => SymbolKind::Interface,
        ast::Declaration::Enum(_) => SymbolKind::Enum,
        ast::Declaration::Namespace(_) => SymbolKind::Namespace,
        ast::Declaration::Type(_) => SymbolKind::TypeParameter,
        ast::Declaration::Extension(_) => SymbolKind::Class,
    }
}

/// Convert an AST member to a document symbol.
fn member_to_document_symbol_ast(
    ast: AstQueryContext<'_>,
    member_id: ast::LocalNodeId<ast::Member>,
) -> Option<DocumentSymbol> {
    // resolve the member node
    let member = ast.tree().get(member_id);

    // resolve the member name and kind
    let kind = member_symbol_kind_ast(member)?;
    let name = if let Some(name) = member.name() {
        ast.strings().get(name).to_string()
    } else {
        let key = member.key()?;
        member_key_name_ast(ast, key)?
    };

    // resolve spans
    let range = ast.source_map().get(member_id.id);
    let selection_range = ast.tree().get_main_span(member_id).unwrap_or(range);

    Some(DocumentSymbol::new(name, kind, range).with_selection_range(selection_range))
}

/// Convert an AST type member to a document symbol.
fn type_member_to_document_symbol_ast(
    ast: AstQueryContext<'_>,
    member_id: ast::LocalNodeId<ast::TypeMember>,
) -> Option<DocumentSymbol> {
    // resolve the type member node
    let member = ast.tree().get(member_id);

    // resolve the type member name and kind
    let kind = type_member_symbol_kind_ast(member)?;
    let name = if let Some(name) = member.name() {
        ast.strings().get(name).to_string()
    } else {
        let key = member.key()?;
        member_key_name_ast(ast, key)?
    };

    // resolve spans
    let range = ast.source_map().get(member_id.id);
    let selection_range = ast.tree().get_main_span(member_id).unwrap_or(range);

    Some(DocumentSymbol::new(name, kind, range).with_selection_range(selection_range))
}

/// Convert an AST enum field to a document symbol.
fn enum_field_to_document_symbol_ast(
    ast: AstQueryContext<'_>,
    field_id: ast::LocalNodeId<ast::EnumField>,
) -> Option<DocumentSymbol> {
    // resolve the enum field node
    let field = ast.tree().get(field_id);

    // resolve the field name
    let name = ast.strings().get(field.name.string()).to_string();

    // resolve spans
    let range = ast.source_map().get(field_id.id);
    let selection_range = ast.tree().get_main_span(field_id).unwrap_or(range);

    Some(
        DocumentSymbol::new(name, SymbolKind::EnumMember, range)
            .with_selection_range(selection_range),
    )
}

/// Resolve a display name for a member key.
fn member_key_name_ast(ast: AstQueryContext<'_>, key: &ast::Key) -> Option<String> {
    match key {
        ast::Key::Name(name) => Some(ast.strings().get(name.string()).to_string()),
        ast::Key::Private(name) => {
            let name = ast.strings().get(*name).to_string();
            Some(format!("#{name}"))
        }
        ast::Key::Expression(_) => None,
    }
}

/// Map an AST member to a symbol kind.
fn member_symbol_kind_ast(member: &ast::Member) -> Option<SymbolKind> {
    match member {
        ast::Member::AssociatedType { .. } => Some(SymbolKind::TypeParameter),
        ast::Member::AssociatedConst { .. } => Some(SymbolKind::Constant),
        ast::Member::Field { .. } => Some(SymbolKind::Field),
        ast::Member::Method { .. } => Some(SymbolKind::Method),
        ast::Member::Embed { .. } => None,
        ast::Member::StaticBlock { .. } => None,
        ast::Member::ComptimeBlock { .. } => None,
        ast::Member::Error => None,
    }
}

/// Map an AST type member to a symbol kind.
fn type_member_symbol_kind_ast(member: &ast::TypeMember) -> Option<SymbolKind> {
    match member {
        ast::TypeMember::AssociatedType { .. } => Some(SymbolKind::TypeParameter),
        ast::TypeMember::AssociatedConst { .. } => Some(SymbolKind::Constant),
        ast::TypeMember::Field { .. } => Some(SymbolKind::Field),
        ast::TypeMember::Method { .. } => Some(SymbolKind::Method),
        ast::TypeMember::CallSignature { .. } => Some(SymbolKind::Method),
        ast::TypeMember::ConstructSignature { .. } => Some(SymbolKind::Method),
        ast::TypeMember::IndexSignature { .. } => None,
        ast::TypeMember::Embed { .. } => None,
        ast::TypeMember::Error => None,
    }
}
