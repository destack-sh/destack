use std::collections::HashSet;

use destack_core::StringId;
use destack_source::{FileId, ModuleId, NodeSpanType, Span, Uri};
use serde::{Deserialize, Serialize};

use crate::common::{
    QueryContext, SymbolAtOffset, dependency_item_matches_name, find_symbol_at_offset,
    get_canonical_symbol, get_dir_node_main_span, get_dir_node_span, get_module_by_file_id,
    get_symbol_definition_span, resolve_nominal_symbol_from_type_expression,
    resolve_type_symbol_from_module, type_definition_span_for_symbol,
};
use destack_dir::{
    self as dir, Declarator, DependencyItem, Expression, GlobalNodeIdAny, NodeType, Resolution,
};
use destack_workspace::Session;

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
    // resolve import-specifier definitions before generic symbol lookup
    if let Some(span) = resolve_import_definition_at_offset(session, file, offset) {
        return Some(DefinitionResult::single(span));
    }

    // find the symbol at the offset
    let Some(symbol_at) = find_symbol_at_offset(session, file, offset) else {
        if let Some(span) = resolve_type_definition_from_imports(session, file, offset) {
            return Some(DefinitionResult::single(span));
        }

        return None;
    };

    // prefer overload declaration spans when call resolution selected a concrete signature
    if let Some(span) = overload_definition_span_for_call_site(session, file, &symbol_at) {
        return Some(DefinitionResult::single(span));
    }

    // get the definition span
    let span = get_symbol_definition_span(session, symbol_at.symbol_id)?;

    Some(DefinitionResult::single(span))
}

/// Resolve a definition span when the cursor is on an import dependency item.
fn resolve_import_definition_at_offset(
    session: &Session,
    file: FileId,
    offset: u32,
) -> Option<Span> {
    // resolve the module and query context for this file
    let module = get_module_by_file_id(session, file)?;
    let module = module.as_ref();
    let ctx = crate::query_context(session, &module)?;
    let dir_tree = ctx.tree();

    // scan dependency items and select the one at the cursor
    for item_id in dir_tree.iter_node_ids_of_type::<DependencyItem>() {
        let item = dir_tree.get::<DependencyItem>(item_id);

        // resolve the main declaration span for coarse overlap checks
        let fallback_span = get_dir_node_main_span(&ctx.ast, &ctx.dir, item_id.into())
            .or_else(|| get_dir_node_span(&ctx.ast, &ctx.dir, item_id.into()));

        // skip items that do not cover the cursor
        if !fallback_span.is_some_and(|span| span.contains(offset)) {
            continue;
        }

        // resolve side spans for imported-name and alias positions
        let source_id = dir_tree.get_source(item_id.id);
        let imported_name_span = ctx
            .ast
            .tree
            .get_side_span_by_id(source_id, NodeSpanType::Type)
            .map(|span| Span::new(ctx.file_id, span.start, span.end));
        let local_alias_span = ctx
            .ast
            .tree
            .get_side_span_by_id(source_id, NodeSpanType::Main)
            .map(|span| Span::new(ctx.file_id, span.start, span.end));

        // import-side positions should prefer the imported target symbol
        let prefer_target_symbol = imported_name_span.is_some_and(|span| span.contains(offset))
            || local_alias_span.is_some_and(|span| span.contains(offset));
        let target_symbol =
            resolve_dependency_item_target_symbol_for_definition(session, &ctx, item)?;

        if prefer_target_symbol {
            return get_symbol_definition_span(session, target_symbol);
        }

        return get_symbol_definition_span(session, target_symbol);
    }

    None
}

/// Resolve a dependency item target symbol for goto-definition.
fn resolve_dependency_item_target_symbol_for_definition(
    _session: &Session,
    ctx: &QueryContext<'_>,
    item: &DependencyItem,
) -> Option<dir::GlobalSymbolId> {
    // resolved items expose direct target symbols
    if let Some(target_symbol) = item.target_symbol() {
        return Some(target_symbol);
    }

    // unresolved items may still expose a local symbol that canonicalizes to the target
    if let Some(local_symbol) = item.symbol() {
        return Some(dir::GlobalSymbolId::new(ctx.module_id, local_symbol));
    }

    // unresolved import targets are treated as not-ready state
    None
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
    let module = module.as_ref();
    let ctx = crate::query_context(session, &module)?;
    let declaration = {
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.primary_declaration?
    };

    get_dir_node_main_span(&ctx.ast, &ctx.dir, declaration.local_id)
        .or_else(|| get_dir_node_span(&ctx.ast, &ctx.dir, declaration.local_id))
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
    let module = module.as_ref();
    let ctx = crate::query_context(session, &module)?;

    if let Some(span) = type_definition_span_for_symbol(session, symbol_id) {
        return Some(DefinitionResult::single(span));
    }

    // for non-type symbols (variables, parameters, etc.), look up their value type
    let resolved_type_symbol = {
        let types = ctx.types();

        // try get_value_type_id first
        if let Some(type_id) = types.get_value_type_id(symbol_id) {
            let ty = types.get_type(type_id);
            ty.symbol()
        }
        // otherwise fall back to declared or inferred type
        else {
            let node_id = symbol_at.node_id.into_global(symbol_id.module_id);
            let type_id = types.get_declared_or_inferred_type_id(node_id)?;
            let ty = types.get_type(type_id);
            ty.symbol()
        }
    };
    if let Some(type_symbol) = resolved_type_symbol {
        let span = get_symbol_definition_span(session, type_symbol)?;
        return Some(DefinitionResult::single(span));
    }

    // final fallback: try to get type from declaration context
    if let Some(type_symbol) = get_type_from_declaration_context(session, &ctx, symbol_at.node_id) {
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
    ctx: &QueryContext<'_>,
    node_id: dir::LocalNodeIdAny,
) -> Option<dir::GlobalSymbolId> {
    let dir_tree = ctx.tree();
    let types = ctx.types();

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
                if let Some(ty_expr_id) = declarator.ty
                    && let Some(type_symbol) =
                        resolve_nominal_symbol_from_type_expression(session, ctx, ty_expr_id)
                {
                    return Some(type_symbol);
                }
            }
        }
        // for parameters, get the declared type
        NodeType::Parameter => {
            // parameters have their declared type stored in TypeTable
            let global_node_id = dir::GlobalNodeIdAny {
                module_id: ctx.module_id,
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
                // verify target is a type symbol
                let target_module = session.modules.get(target.module_id);
                let target_module = target_module.as_ref();
                let target_ctx = crate::query_context(session, &target_module)?;
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

/// Resolve an overload definition span for the selected call site candidate.
fn overload_definition_span_for_call_site(
    session: &Session,
    file: FileId,
    symbol_at: &SymbolAtOffset,
) -> Option<Span> {
    // require a local expression node at the cursor
    if symbol_at.node_id.ty != NodeType::Expression {
        return None;
    }

    let expression_id: dir::LocalNodeId<Expression> = symbol_at.node_id.try_into().ok()?;

    // resolve the query context for this file
    let module = get_module_by_file_id(session, file)?;
    let module = module.as_ref();
    let ctx = crate::query_context(session, &module)?;
    let dir_tree = ctx.tree();

    // require a call/new parent where this expression is the callee
    let parent = dir_tree.get_parent(expression_id.id)?;
    if parent.ty != NodeType::Expression {
        return None;
    }
    let parent_expression_id: dir::LocalNodeId<Expression> = parent.try_into().ok()?;
    let parent_expression = dir_tree.get::<Expression>(parent_expression_id);
    let is_callee = matches!(
        parent_expression,
        Expression::Call { left, .. } | Expression::New { left, .. } if *left == expression_id
    );
    if !is_callee {
        return None;
    }

    // resolve the selected call candidate signature
    let (target_symbol, dynamic_parameters) = {
        let types = ctx.types();
        let node_id = GlobalNodeIdAny {
            module_id: ctx.module_id,
            local_id: parent_expression_id.into(),
        };
        let resolution_id = types.get_resolution_for_node(node_id)?;
        let resolution = types.get_resolution(resolution_id);
        match resolution {
            Resolution::Static { candidate, .. } => Some((
                get_canonical_symbol(session, candidate.target_symbol),
                candidate
                    .resolved_signature
                    .as_ref()?
                    .dynamic_parameters
                    .clone(),
            )),
            _ => None,
        }
    }?;

    // only match declaration signatures within the target symbol module
    if target_symbol.module_id != ctx.module_id {
        return None;
    }

    overload_declaration_span_for_signature(session, target_symbol, &dynamic_parameters)
}

/// Resolve an overload declaration span by matching dynamic parameter type ids.
fn overload_declaration_span_for_signature(
    session: &Session,
    symbol_id: dir::GlobalSymbolId,
    dynamic_parameter_types: &[dir::LocalTypeId],
) -> Option<Span> {
    // resolve the symbol context and declarations
    let module = session.modules.get(symbol_id.module_id);
    let module = module.as_ref();
    let ctx = crate::query_context(session, &module)?;
    let (primary_declaration, secondary_declarations) = {
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        (
            symbol.primary_declaration,
            symbol.secondary_declarations.clone(),
        )
    };

    let mut declarations = Vec::new();
    if let Some(primary_declaration) = primary_declaration {
        declarations.push(primary_declaration);
    }
    if let Some(secondary_declarations) = secondary_declarations {
        declarations.extend(secondary_declarations.iter().copied());
    }

    // find the declaration whose parameter type ids match the resolved signature
    for declaration in declarations {
        let Some(parameter_types) = declaration_parameter_type_ids(&ctx, declaration.local_id)
        else {
            continue;
        };

        if parameter_types.len() != dynamic_parameter_types.len() {
            continue;
        }

        if parameter_types
            .iter()
            .zip(dynamic_parameter_types.iter())
            .all(|(left, right)| left == right)
        {
            return get_dir_node_main_span(&ctx.ast, &ctx.dir, declaration.local_id)
                .or_else(|| get_dir_node_span(&ctx.ast, &ctx.dir, declaration.local_id));
        }
    }

    None
}

/// Resolve declared dynamic parameter type ids for a declaration or method member.
fn declaration_parameter_type_ids(
    ctx: &QueryContext<'_>,
    declaration_id: dir::LocalNodeIdAny,
) -> Option<Vec<dir::LocalTypeId>> {
    let dir_tree = ctx.tree();
    let types = ctx.types();

    let dynamic_parameters = match declaration_id.ty {
        NodeType::Declaration => {
            let declaration_id = declaration_id.try_into().ok()?;
            let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
            let dir::Declaration::Function { signature, .. } = declaration else {
                return None;
            };
            signature.dynamic_parameters.clone()
        }
        NodeType::Member => {
            let member_id = declaration_id.try_into().ok()?;
            let member = dir_tree.get::<dir::Member>(member_id);
            let dir::Member::Method { signature, .. } = member else {
                return None;
            };
            signature.dynamic_parameters.clone()
        }
        _ => return None,
    };

    let mut parameter_types = Vec::with_capacity(dynamic_parameters.len());
    for parameter_id in dynamic_parameters {
        let global_parameter_id = parameter_id.into_global_any(ctx.module_id);
        let type_id = types.get_declared_type_id(global_parameter_id)?;
        parameter_types.push(type_id);
    }

    Some(parameter_types)
}

/// Resolve a type definition span by walking import and re-export chains.
fn resolve_type_definition_from_imports(
    session: &Session,
    file: FileId,
    offset: u32,
) -> Option<Span> {
    // resolve the module and query context for this file
    let module = get_module_by_file_id(session, file)?;
    let module = module.as_ref();
    let ctx = crate::query_context(session, &module)?;

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
        let name_id = match expr {
            Expression::Member { left, name, .. } => {
                let Some(namespace_name_id) =
                    namespace_alias_name_id_from_expression(dir_tree, *left)
                else {
                    continue;
                };
                if let Some(span) = resolve_type_export_from_namespace_import(
                    session,
                    &ctx,
                    namespace_name_id,
                    *name,
                ) {
                    return Some(span);
                }

                *name
            }
            Expression::UnresolvedPath { path, .. }
            | Expression::LocalReference { path, .. }
            | Expression::ModuleReference { path, .. }
            | Expression::GlobalReference { path, .. } => {
                let Some(name_id) = path.last_segment() else {
                    continue;
                };

                // resolve namespaced type references like `models.Settings`
                if path.segments.len() > 1
                    && let Some(namespace_name_id) = path.first_segment()
                    && let Some(span) = resolve_type_export_from_namespace_import(
                        session,
                        &ctx,
                        namespace_name_id,
                        name_id,
                    )
                {
                    return Some(span);
                }

                name_id
            }
            _ => continue,
        };

        if let Some(span) = resolve_type_export_from_imports(session, &ctx, name_id) {
            return Some(span);
        }
    }

    None
}

/// Resolve a namespace alias name from a path-like expression.
fn namespace_alias_name_id_from_expression(
    dir_tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<StringId> {
    let expression = dir_tree.get::<Expression>(expression_id);
    match expression {
        Expression::UnresolvedPath { path, .. }
        | Expression::LocalReference { path, .. }
        | Expression::ModuleReference { path, .. }
        | Expression::GlobalReference { path, .. } => path.first_segment(),
        Expression::Parenthesized { expression } => {
            namespace_alias_name_id_from_expression(dir_tree, *expression)
        }
        _ => None,
    }
}

/// Resolve a type export through a namespace import alias and member name.
fn resolve_type_export_from_namespace_import(
    session: &Session,
    ctx: &QueryContext<'_>,
    namespace_name_id: StringId,
    type_name_id: StringId,
) -> Option<Span> {
    let dir_tree = ctx.tree();
    for item_id in dir_tree.iter_node_ids_of_type::<dir::DependencyItem>() {
        let item = dir_tree.get::<dir::DependencyItem>(item_id);
        let (mode, name, alias, target_module) = match item {
            dir::DependencyItem::Remote {
                mode,
                name,
                alias,
                target_module,
                ..
            } => (*mode, *name, *alias, Some(*target_module)),
            _ => continue,
        };

        if !dependency_item_matches_name(namespace_name_id, name, alias, mode, false) {
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
            type_name_id,
            &mut visited,
        ) {
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
