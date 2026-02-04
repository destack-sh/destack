use std::collections::HashSet;

use destack_base::StringId;
use destack_source::{FileId, ModuleId, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::query::common::{
    QueryContext, dependency_item_matches_name, find_symbol_at_offset, get_canonical_symbol,
    get_dir_node_span, get_module_by_file_id, get_symbol_definition_span,
    resolve_type_symbol_from_module, type_definition_span_for_symbol,
};
use crate::{ModuleAst, ModuleDir, Session};
use destack_dir::{self as dir, Declarator, Expression, NodeType};

/// Result of a goto definition query.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DefinitionResult {
    /// The definition location(s).
    /// Multiple locations for overloaded symbols or partial definitions.
    pub locations: Vec<Span>,
}

impl DefinitionResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self {
            locations: Vec::new(),
        }
    }

    /// Create a result with a single span.
    pub fn single(span: Span) -> Self {
        Self {
            locations: vec![span],
        }
    }

    /// Whether any definitions were found.
    pub fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }
}

/// Request goto definition at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoDefinitionRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for goto definition queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoDefinitionResponse {
    /// Definition locations, if any.
    pub result: Option<DefinitionResult>,
}

/// Request goto declaration at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoDeclarationRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for goto declaration queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoDeclarationResponse {
    /// Declaration locations, if any.
    pub result: Option<DefinitionResult>,
}

/// Request goto type definition at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoTypeDefinitionRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for goto type definition queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GotoTypeDefinitionResponse {
    /// Type definition locations, if any.
    pub result: Option<DefinitionResult>,
}

/// Find the definition of the symbol at the given position.
///
/// Returns the location(s) where the symbol is defined.
/// For imports, follows to the original definition.
pub fn goto_definition(session: &Session, file: FileId, offset: u32) -> Option<DefinitionResult> {
    // find the symbol at the offset
    let Some(symbol_at) = find_symbol_at_offset(session, file, offset) else {
        if let Some(span) = resolve_type_definition_from_imports(session, file, offset) {
            return Some(DefinitionResult::single(span));
        }

        return None;
    };

    // get the definition span
    let span = get_symbol_definition_span(session, symbol_at.symbol_id)?;

    Some(DefinitionResult::single(span))
}

/// Find the declaration of the symbol at the given position.
///
/// For imports, returns the import statement location.
/// For locals, same as goto_definition.
pub fn goto_declaration(session: &Session, file: FileId, offset: u32) -> Option<DefinitionResult> {
    // find the symbol at offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;

    // get the declaration span
    let span = get_declaration_span(session, symbol_at.symbol_id)?;

    Some(DefinitionResult::single(span))
}

/// Get the declaration span without following canonical_symbol.
fn get_declaration_span(session: &Session, symbol_id: dir::GlobalSymbolId) -> Option<Span> {
    // get module and dir
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    // get primary_declaration directly
    let declaration = symbol.primary_declaration?;

    drop(symbols);

    get_dir_node_span(ctx.ast, ctx.dir, declaration.local_id)
}

/// Find the type definition of the symbol at the given position.
///
/// For a variable, returns the location of its type's definition.
/// For a type, returns the type itself.
pub fn goto_type_definition(
    session: &Session,
    file: FileId,
    offset: u32,
) -> Option<DefinitionResult> {
    // find the symbol at the offset
    let symbol_at = find_symbol_at_offset(session, file, offset)?;
    let symbol_id = symbol_at.symbol_id;

    // use the canonical symbol when it resolves to a type
    let canonical_id = get_canonical_symbol(session, symbol_id);
    if let Some(span) = type_definition_span_for_symbol(session, canonical_id) {
        return Some(DefinitionResult::single(span));
    }

    // get the module to access type table
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    if let Some(span) = type_definition_span_for_symbol(session, symbol_id) {
        drop(module);
        return Some(DefinitionResult::single(span));
    }

    // for non-type symbols (variables, parameters, etc.), look up their value type
    let types = ctx.types();

    // try get_value_type_id first (for inferred types)
    if let Some(type_id) = types.get_value_type_id(symbol_id) {
        let ty = types.get_type(type_id);
        if let Some(type_symbol) = ty.symbol() {
            drop(types);
            drop(module);
            let span = get_symbol_definition_span(session, type_symbol)?;
            return Some(DefinitionResult::single(span));
        }
    }

    // fall back to declared type (type annotation) if inferred not available
    let node_id = symbol_at.node_id.into_global(symbol_id.module_id);
    if let Some(type_id) = types.get_declared_or_inferred_type_id(node_id) {
        let ty = types.get_type(type_id);
        if let Some(type_symbol) = ty.symbol() {
            drop(types);
            drop(module);
            let span = get_symbol_definition_span(session, type_symbol)?;
            return Some(DefinitionResult::single(span));
        }
    }
    drop(types);

    // final fallback: try to get type from AST context
    if let Some(type_symbol) =
        get_type_from_declaration_context(session, ctx.ast, ctx.dir, symbol_at.node_id)
    {
        drop(module);
        let span = get_symbol_definition_span(session, type_symbol)?;
        return Some(DefinitionResult::single(span));
    }

    // fall back to resolving through import and re-export chains for type-only exports
    if let Some(span) = resolve_type_definition_from_imports(session, file, offset) {
        return Some(DefinitionResult::single(span));
    }

    None
}

/// Get the type symbol from the declaration context of a node.
/// (we can remove this once type inference populates value types for all symbols, see goto_type_definition)
fn get_type_from_declaration_context(
    session: &Session,
    _ast: &ModuleAst,
    dir: &ModuleDir,
    node_id: dir::LocalNodeIdAny,
) -> Option<dir::GlobalSymbolId> {
    let dir_tree = dir.tree.read();
    let types = dir.types.read();

    match node_id.ty {
        // for patterns, find parent declarator and get its type annotation
        NodeType::Pattern => {
            // look for a parent Declarator
            if let Some(parent_node_id) = dir_tree.get_parent(node_id.id)
                && parent_node_id.ty == NodeType::Declarator
            {
                let declarator_id = parent_node_id.try_into_typed().ok()?;
                let declarator = dir_tree.get::<Declarator>(declarator_id);

                // get the type expression
                if let Some(ty_expr_id) = declarator.ty {
                    let ty_expr = dir_tree.get::<Expression>(ty_expr_id);
                    // get target_symbol from the type expression
                    return ty_expr.target_symbol();
                }
            }
        }
        // for parameters, get the declared type
        NodeType::Parameter => {
            // parameters have their declared type stored in TypeTable
            let global_node_id = dir::GlobalNodeIdAny {
                module_id: dir.id,
                local_id: node_id,
            };
            if let Some(type_id) = types.get_declared_type_id(global_node_id) {
                let ty = types.get_type(type_id);
                return ty.symbol();
            }
        }
        // for expressions that are type references, get the target_symbol
        NodeType::Expression => {
            let expr_id = node_id.try_into_typed().ok()?;
            let expr = dir_tree.get::<Expression>(expr_id);

            // check if this is a type reference (GlobalReference, etc.)
            if let Some(target) = expr.target_symbol() {
                drop(types);
                drop(dir_tree);
                // verify target is a type symbol
                let target_module = session.modules.get(target.module_id);
                let target_module = target_module.read();
                let target_ctx = session.query_context(&target_module)?;
                let symbols = target_ctx.symbols();
                let symbol = symbols.get_symbol(target.local_id);

                if symbol.space == dir::SymbolSpace::Type
                    || symbol.space == dir::SymbolSpace::TypeValue
                {
                    return Some(target);
                }
            }
        }
        _ => {}
    }

    None
}

/// Resolve a type definition span by walking import and re-export chains.
fn resolve_type_definition_from_imports(
    session: &Session,
    file: FileId,
    offset: u32,
) -> Option<Span> {
    // resolve the module and query context for this file
    let module = get_module_by_file_id(session, file)?;
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // find the smallest enclosing AST node at the cursor
    let mut enclosing = ctx.ast.tree.source_map.get_enclosing_spans(offset, offset);
    if enclosing.is_empty() {
        return None;
    }

    enclosing.sort_by_key(|span| (span.length, -(span.idx as i64)));

    // check unresolved paths for import based type resolution
    let dir_tree = ctx.tree();
    for enclosing_span in &enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enclosing_span.idx) else {
            continue;
        };

        if dir_node_id.ty != NodeType::Expression {
            continue;
        }

        let Ok(expr_id) = dir_node_id.try_into() else {
            continue;
        };

        let expr = dir_tree.get::<Expression>(expr_id);
        let path = match expr {
            Expression::UnresolvedPath { path, .. }
            | Expression::LocalReference { path, .. }
            | Expression::ModuleReference { path, .. }
            | Expression::GlobalReference { path, .. } => path,
            _ => continue,
        };

        let Some(name_id) = path.last_segment() else {
            continue;
        };

        if let Some(span) = resolve_type_export_from_imports(session, &ctx, name_id) {
            return Some(span);
        }
    }

    None
}

/// Resolve a type export by following imports in the current module.
fn resolve_type_export_from_imports(
    session: &Session,
    ctx: &QueryContext<'_>,
    name_id: StringId,
) -> Option<Span> {
    // search import expressions for a matching imported name
    let dir_tree = ctx.tree();
    for (_expr_id, expr) in dir_tree.iter_nodes_of_type::<Expression>() {
        let items = match expr {
            Expression::Import { items, .. } | Expression::UnresolvedImport { items, .. } => items,
            _ => continue,
        };

        for item_id in items {
            let item = dir_tree.get::<dir::DependencyItem>(*item_id);
            let (mode, name, alias, target_module) = match item {
                dir::DependencyItem::Remote {
                    mode,
                    name,
                    alias,
                    target_module,
                    ..
                } => (*mode, *name, *alias, Some(*target_module)),
                dir::DependencyItem::UnresolvedRemote {
                    mode,
                    name,
                    alias,
                    target_module,
                    ..
                } => (*mode, *name, *alias, *target_module),
                _ => continue,
            };

            if !dependency_item_matches_name(name_id, name, alias, mode, false) {
                continue;
            }

            let Some(target_module_id) = target_module
                .and_then(|targets| targets.ty.or(targets.value))
                .and_then(|target| target.module_id())
            else {
                continue;
            };

            let mut visited = HashSet::new();
            if let Some(span) = resolve_type_definition_from_module(
                session,
                target_module_id,
                name_id,
                &mut visited,
            ) {
                return Some(span);
            }
        }
    }

    None
}

/// Resolve a type definition span for a module export.
fn resolve_type_definition_from_module(
    session: &Session,
    module_id: ModuleId,
    name_id: StringId,
    visited: &mut HashSet<ModuleId>,
) -> Option<Span> {
    let symbol_id = resolve_type_symbol_from_module(session, module_id, name_id, visited)?;
    type_definition_span_for_symbol(session, symbol_id)
}
