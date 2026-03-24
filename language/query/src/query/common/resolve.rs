use std::collections::HashSet;
use std::path::Path;
use {destack_ast as ast, destack_dir as dir};

use destack_core::StringId;
use destack_dir::{
    Declaration, DependencyItem, DependencyKind, DependencyMode, EnumField, Expression,
    GlobalNodeIdAny, GlobalSymbolId, LocalScopeId, LocalSymbolId, Member, Name, NodeType,
    Resolution, StaticKey, SymbolKind, SymbolSpace, SymbolType, Type,
};
use destack_source::{ModuleId, PathExt, Uri};

use super::symbol::member_key_name;
use super::{
    DirResolvedContext, QueryContext, for_each_visible_extension, visible_symbols,
    with_dir_resolved_context_for_module,
};
use destack_workspace::Session;

/// Check whether a symbol type participates in the type namespace.
pub(crate) fn is_type_symbol(symbol_type: SymbolType) -> bool {
    matches!(
        symbol_type,
        SymbolType::Class
            | SymbolType::Struct
            | SymbolType::Interface
            | SymbolType::Enum
            | SymbolType::TypeAlias
            | SymbolType::Newtype
    )
}

/// Check whether a symbol matches a requested symbol space filter.
pub(crate) fn matches_symbol_space_filter(
    symbol_type: SymbolType,
    symbol_space: SymbolSpace,
    filter: Option<SymbolSpace>,
) -> bool {
    // allow all symbols when no filter is present
    let Some(filter) = filter else {
        return true;
    };

    // return whether the symbol matches the filter
    match filter {
        SymbolSpace::Type => match symbol_space {
            SymbolSpace::Type => true,
            SymbolSpace::TypeValue => true,
            _ => is_type_symbol(symbol_type),
        },
        SymbolSpace::Value => {
            symbol_space == SymbolSpace::Value || symbol_space == SymbolSpace::TypeValue
        }
        SymbolSpace::TypeValue => symbol_space == SymbolSpace::TypeValue,
        SymbolSpace::Label => symbol_space == SymbolSpace::Label,
    }
}

/// Check whether a symbol matches an explicit import-clause space filter.
pub(crate) fn matches_import_clause_space_filter(
    symbol_type: SymbolType,
    symbol_space: SymbolSpace,
    filter: Option<SymbolSpace>,
) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    match filter {
        // `import type` only exposes symbols that are intrinsically type-side.
        //
        // destack allows some `TypeValue` symbols, such as functions, in general
        // type-space resolution. Import clauses are stricter: they should offer
        // declarations that remain meaningful after type-only erasure.
        SymbolSpace::Type => symbol_space == SymbolSpace::Type || is_type_symbol(symbol_type),
        _ => matches_symbol_space_filter(symbol_type, symbol_space, Some(filter)),
    }
}

/// Build a global symbol id from a module and local symbol id.
pub(crate) fn global_symbol(module_id: ModuleId, local_id: LocalSymbolId) -> GlobalSymbolId {
    // assemble the global symbol id
    GlobalSymbolId {
        module_id,
        local_id,
    }
}

/// Find the scope owned by a symbol when it declares one.
pub(crate) fn owned_scope_for_symbol(
    symbols: &dir::SymbolTable,
    symbol_id: LocalSymbolId,
) -> Option<LocalScopeId> {
    // scan scopes for the owner id
    for (idx, scope) in symbols.scopes().enumerate() {
        if scope.owner_id == Some(symbol_id) {
            return Some(LocalScopeId::new(idx as u32));
        }
    }

    // return none when no scope is owned
    None
}

/// Check whether a symbol resolves in the requested symbol space.
pub(crate) fn symbol_matches_space(
    session: &Session,
    symbol_id: GlobalSymbolId,
    space: SymbolSpace,
) -> bool {
    // resolve the module and query context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.as_ref();
    with_dir_resolved_context_for_module(session, module, |ctx| {
        if symbol_id.local_id.id >= ctx.symbols().symbol_count() {
            return false;
        }

        // resolve the symbol space
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        matches_symbol_space_filter(symbol.ty, symbol.space, Some(space))
    })
    .unwrap_or(false)
}

/// Check whether a symbol represents a nominal type symbol.
fn symbol_is_type_symbol(session: &Session, symbol_id: GlobalSymbolId) -> bool {
    // resolve the module and query context for the symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.as_ref();
    with_dir_resolved_context_for_module(session, module, |ctx| {
        if symbol_id.local_id.id >= ctx.symbols().symbol_count() {
            return false;
        }

        // read the symbol type
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        is_type_symbol(symbol.ty)
    })
    .unwrap_or(false)
}

/// Resolve a member access symbol when the cursor is on the member name.
pub(crate) fn resolve_member_access_symbol(
    session: &Session,
    ctx: &QueryContext<'_>,
    expr_id: dir::LocalNodeId<Expression>,
    left: dir::LocalNodeId<Expression>,
    name: StringId,
) -> Option<GlobalSymbolId> {
    // prefer the direct member target recorded on the expression
    if let Some(target_symbol) = ctx.tree().get::<Expression>(expr_id).target_symbol() {
        return Some(target_symbol);
    }

    // resolve member symbols from recorded resolution data when available
    if let Some(target) = recorded_member_resolution(ctx, expr_id) {
        return Some(target);
    }

    // resolve the member name string
    let member_name = session.strings.get(name).to_string();

    // resolve nominal members when the receiver is a nominal type
    if let Some(base_symbol) = nominal_symbol_for_expression(ctx, left) {
        if let Some(symbol_id) =
            resolve_member_symbol_from_declaration(session, base_symbol, &member_name)
        {
            return Some(symbol_id);
        }

        if let Some(symbol_id) =
            resolve_member_symbol_from_lineage(session, base_symbol, &member_name)
        {
            return Some(symbol_id);
        }

        if let Some(symbol_id) =
            resolve_extension_member_symbol(session, base_symbol, ctx.module_id, &member_name)
        {
            return Some(symbol_id);
        }
    }

    // resolve namespace members through namespace imports and re-exports
    let member_space = if expression_is_type_position(ctx, expr_id) {
        SymbolSpace::Type
    } else {
        SymbolSpace::Value
    };
    if let Some(namespace_symbol) = resolve_namespace_receiver_symbol(ctx, left)
        && let Some(symbol_id) =
            resolve_namespace_member_symbol(session, namespace_symbol, name, member_space)
    {
        return Some(symbol_id);
    }

    None
}

/// Resolve the namespace receiver symbol for a member access.
pub(crate) fn resolve_namespace_receiver_symbol(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let dir_tree = ctx.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    match expression {
        Expression::Parenthesized { expression } => {
            resolve_namespace_receiver_symbol(ctx, *expression)
        }
        Expression::UnresolvedPath { path, .. }
        | Expression::LocalReference { path, .. }
        | Expression::ModuleReference { path, .. }
        | Expression::GlobalReference { path, .. } => {
            let name_id = path.last_segment()?;
            resolve_visible_symbol_in_scope(ctx, expression_id, name_id, SymbolSpace::Value)
                .or_else(|| resolve_expression_binding_symbol(ctx, expression_id))
                .or_else(|| expression.target_symbol())
                .or_else(|| resolve_expression_symbol(ctx, expression_id))
        }
        _ => resolve_expression_symbol(ctx, expression_id),
    }
}

/// Resolve the leading namespace binding for a multi segment path expression.
pub(crate) fn resolve_namespace_path_receiver_symbol(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let dir_tree = ctx.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    let path = match expression {
        Expression::UnresolvedPath { path, .. }
        | Expression::LocalReference { path, .. }
        | Expression::ModuleReference { path, .. }
        | Expression::GlobalReference { path, .. } => path,
        _ => return None,
    };

    if path.segments.len() < 2 {
        return None;
    }

    let name_id = path.first_segment()?;
    resolve_visible_symbol_in_scope(ctx, expression_id, name_id, SymbolSpace::Value)
}

/// Resolve the best symbol for an expression from DIR data.
pub(crate) fn resolve_expression_symbol(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let dir_tree = ctx.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    // prefer recorded expression targets
    if let Some(target_symbol) = expression.target_symbol() {
        return Some(target_symbol);
    }

    match expression {
        // preserve the underlying symbol through wrappers
        Expression::Parenthesized { expression } => resolve_expression_symbol(ctx, *expression),

        // resolve visible path bindings from the current scope
        Expression::UnresolvedPath { path, .. }
        | Expression::LocalReference { path, .. }
        | Expression::ModuleReference { path, .. }
        | Expression::GlobalReference { path, .. } => {
            let name_id = path.last_segment()?;
            let symbol_space = if expression_is_type_position(ctx, expression_id) {
                SymbolSpace::Type
            } else {
                SymbolSpace::Value
            };

            resolve_visible_symbol_in_scope(ctx, expression_id, name_id, symbol_space)
        }

        _ => None,
    }
}

/// Resolve the visible binding symbol for a path-like expression.
pub(crate) fn resolve_expression_binding_symbol(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let dir_tree = ctx.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    match expression {
        Expression::Parenthesized { expression } => {
            resolve_expression_binding_symbol(ctx, *expression)
        }
        Expression::UnresolvedPath { path, .. }
        | Expression::LocalReference { path, .. }
        | Expression::ModuleReference { path, .. }
        | Expression::GlobalReference { path, .. } => {
            let name_id = path.last_segment()?;
            let symbol_space = if expression_is_type_position(ctx, expression_id) {
                SymbolSpace::Type
            } else {
                SymbolSpace::Value
            };

            resolve_visible_symbol_in_scope(ctx, expression_id, name_id, symbol_space).or_else(
                || {
                    resolve_visible_binding_symbol_for_target(
                        ctx,
                        expression_id,
                        name_id,
                        symbol_space,
                        expression.target_symbol(),
                    )
                },
            )
        }
        _ => None,
    }
}

/// Resolve a visible symbol by name from the expression scope.
pub(crate) fn resolve_visible_symbol_in_scope(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    name_id: StringId,
    symbol_space: SymbolSpace,
) -> Option<GlobalSymbolId> {
    let dir_tree = ctx.tree();
    let (scope_id, mark) = dir_tree.get_scope(expression_id);
    let symbols = ctx.symbols();

    for visible in visible_symbols(symbols, scope_id, mark, Some(symbol_space)) {
        let dir::StaticKey::Name(visible_name) = visible.key else {
            continue;
        };
        if visible_name != name_id {
            continue;
        }

        return Some(GlobalSymbolId::new(ctx.module_id, visible.id));
    }

    None
}

/// Resolve a visible binding symbol by matching an already resolved target symbol.
fn resolve_visible_binding_symbol_for_target(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
    name_id: StringId,
    symbol_space: SymbolSpace,
    target_symbol: Option<GlobalSymbolId>,
) -> Option<GlobalSymbolId> {
    let target_symbol = target_symbol?;

    let dir_tree = ctx.tree();
    let symbols = ctx.symbols();
    let (scope_id, mark) = dir_tree.get_scope(expression_id);

    for visible in visible_symbols(symbols, scope_id, mark, Some(symbol_space)) {
        let dir::StaticKey::Name(visible_name) = visible.key else {
            continue;
        };
        if visible_name != name_id {
            continue;
        }

        let visible_symbol_id = GlobalSymbolId::new(ctx.module_id, visible.id);
        if visible_symbol_id == target_symbol {
            return Some(visible_symbol_id);
        }

        let visible_symbol = symbols.get_symbol(visible.id);
        if visible_symbol.target_symbol == Some(target_symbol)
            || visible_symbol.canonical_symbol == Some(target_symbol)
        {
            return Some(visible_symbol_id);
        }
    }

    None
}

/// Check whether an expression is used in a type position.
pub(crate) fn expression_is_type_position(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> bool {
    let ast = ctx.ast_context();
    let mut current_id = ctx.tree().get_source(expression_id.id);

    loop {
        let Some(parent_id) = ast.parents().get_by_id(current_id) else {
            return false;
        };

        match ast.tree().get_node_type(parent_id) {
            ast::NodeType::Expression => {
                current_id = parent_id;
            }
            ast::NodeType::Declarator => {
                let declarator = ast
                    .tree()
                    .get(ast::LocalNodeId::<ast::Declarator>::new(parent_id));
                return declarator.ty.is_some_and(|ty| ty.id == current_id);
            }
            ast::NodeType::Parameter => {
                let parameter = ast
                    .tree()
                    .get(ast::LocalNodeId::<ast::Parameter>::new(parent_id));
                return match parameter {
                    ast::Parameter::Named { ty, .. }
                    | ast::Parameter::Pattern { ty, .. }
                    | ast::Parameter::VariadicNamed { ty, .. }
                    | ast::Parameter::VariadicPattern { ty, .. } => {
                        ty.is_some_and(|ty| ty.id == current_id)
                    }
                    ast::Parameter::Error => false,
                };
            }
            ast::NodeType::Member => {
                let member = ast
                    .tree()
                    .get(ast::LocalNodeId::<ast::Member>::new(parent_id));
                return match member {
                    ast::Member::Type { ty, value, .. } => {
                        ty.is_some_and(|ty| ty.id == current_id)
                            || value.is_some_and(|value| value.id == current_id)
                    }
                    ast::Member::ComptimeConst { ty, .. } => {
                        ty.is_some_and(|ty| ty.id == current_id)
                    }
                    ast::Member::Field { value, .. } => {
                        value.is_some_and(|value| value.id == current_id)
                    }
                    ast::Member::Embed { value, .. } => value.id == current_id,
                    _ => false,
                };
            }
            ast::NodeType::Declaration => {
                let declaration = ast
                    .tree()
                    .get(ast::LocalNodeId::<ast::Declaration>::new(parent_id));
                return match declaration {
                    ast::Declaration::Type { value, .. } => value.id == current_id,
                    _ => false,
                };
            }
            _ => return false,
        }
    }
}

/// Resolve the nominal symbol for an expression from DIR type information.
pub(crate) fn nominal_symbol_for_expression(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    // resolve the type id from expression or node type tables
    let type_id = ctx
        .get_expression_type(expression_id.into())
        .or_else(|| ctx.get_node_type(expression_id.into()));
    let type_id = type_id?;

    // resolve the nominal symbol from the resolved type id
    let types = ctx.types();
    resolve_nominal_symbol_from_type(types, type_id)
}

/// Resolve the nominal symbol for a type id.
fn resolve_nominal_symbol_from_type(
    types: &dir::TypeTable,
    type_id: dir::LocalTypeId,
) -> Option<GlobalSymbolId> {
    // walk the type structure to find a nominal symbol
    match types.get_type(type_id) {
        Type::Reference { symbol, .. } => Some(*symbol),
        Type::Object { .. } => types.symbol_for_instance_type(type_id),
        Type::Value { value } => resolve_nominal_symbol_from_type(types, *value),
        Type::ValueOf { right, .. } => resolve_nominal_symbol_from_type(types, *right),
        Type::ReferenceOf { right, .. } => resolve_nominal_symbol_from_type(types, *right),
        Type::PointerOf { right, .. } => resolve_nominal_symbol_from_type(types, *right),
        Type::Unary { right, .. } => resolve_nominal_symbol_from_type(types, *right),
        Type::Binary { left, right, .. } => resolve_nominal_symbol_from_type(types, *left)
            .or_else(|| resolve_nominal_symbol_from_type(types, *right)),
        Type::Conditional {
            left,
            right,
            then_type,
            else_type,
            ..
        } => resolve_nominal_symbol_from_type(types, *left)
            .or_else(|| resolve_nominal_symbol_from_type(types, *right))
            .or_else(|| resolve_nominal_symbol_from_type(types, *then_type))
            .or_else(|| resolve_nominal_symbol_from_type(types, *else_type)),
        Type::Union { elements } | Type::Intersection { elements } => {
            for element in elements {
                if let Some(symbol_id) = resolve_nominal_symbol_from_type(types, *element) {
                    return Some(symbol_id);
                }
            }
            None
        }
        _ => None,
    }
}

/// Resolve a member symbol id from declaration members by name.
fn resolve_member_from_members(
    session: &Session,
    tree: &dir::NodeTree,
    member_ids: &[dir::LocalNodeId<Member>],
    member_name: &str,
) -> Option<LocalSymbolId> {
    // scan members for a matching key name
    for member_id in member_ids {
        let member = tree.get(*member_id);

        match member {
            Member::Type { name, symbol, .. } | Member::ComptimeConst { name, symbol, .. } => {
                let key_name = session.strings.get(*name).to_string();
                if key_name == member_name {
                    return Some(*symbol);
                }
            }
            _ => {
                let Some(key) = member.key() else {
                    continue;
                };

                let Some(key_name) = member_key_name(session, key) else {
                    continue;
                };

                if key_name == member_name {
                    return Some(member.symbol());
                }
            }
        }
    }

    None
}

/// Resolve an enum field symbol id by field name.
fn resolve_member_from_enum_fields(
    session: &Session,
    tree: &dir::NodeTree,
    fields: &[dir::LocalNodeId<EnumField>],
    member_name: &str,
) -> Option<LocalSymbolId> {
    // scan enum fields for a matching name
    for field_id in fields {
        let field = tree.get(*field_id);
        let name = session.strings.get(field.name).to_string();

        // return the field symbol when the name matches
        if name == member_name {
            return Some(field.symbol);
        }
    }

    None
}

/// Resolve a member symbol from a declaration owned by a base symbol.
fn resolve_member_symbol_from_declaration(
    session: &Session,
    base_symbol: GlobalSymbolId,
    member_name: &str,
) -> Option<GlobalSymbolId> {
    // resolve the module query context
    let module = session.modules.get(base_symbol.module_id);
    let module = module.as_ref();
    let ctx = crate::query_context(session, module)?;

    // resolve the primary declaration for the base symbol
    let declaration = {
        let symbols = ctx.symbols();
        let symbol = symbols.get_symbol(base_symbol.local_id);
        symbol.primary_declaration?
    };

    // convert the declaration into a declaration node id
    let Ok(local_decl_id): Result<dir::LocalNodeId<Declaration>, _> = declaration.try_into() else {
        return None;
    };

    // read the declaration node from the tree
    let tree = ctx.tree();
    let declaration = tree.get(local_decl_id);

    // resolve members declared on the type
    let member_ids = declaration.member_ids();
    if let Some(member_ids) = member_ids {
        let symbol_id = resolve_member_from_members(session, tree, member_ids, member_name);

        // return the member symbol when found
        if let Some(symbol_id) = symbol_id {
            return Some(global_symbol(ctx.module_id, symbol_id));
        }
    }

    // resolve enum fields for enum declarations
    if let Declaration::Enum { fields, .. } = declaration {
        let symbol_id = resolve_member_from_enum_fields(session, tree, fields, member_name);

        // return the member symbol when found
        if let Some(symbol_id) = symbol_id {
            return Some(global_symbol(ctx.module_id, symbol_id));
        }
    }

    // return none when no member matches
    None
}

/// Resolve a member symbol through lineage relationships.
fn resolve_member_symbol_from_lineage(
    session: &Session,
    base_symbol: GlobalSymbolId,
    member_name: &str,
) -> Option<GlobalSymbolId> {
    // resolve the module query context for lineage lookup
    let module = session.modules.get(base_symbol.module_id);
    let module = module.as_ref();
    let ctx = crate::query_context(session, module)?;

    // collect direct lineage targets for inheritance and implementation
    let (extends, implements, embedded) = {
        let types = ctx.types();
        let lineage = types.get_lineage_for_symbol(base_symbol)?;
        Some((
            lineage.extends,
            lineage.implements.clone(),
            lineage.embedded.clone(),
        ))
    }?;

    // prefer direct parent first for deterministic results
    if let Some(parent_symbol) = extends
        && let Some(symbol_id) =
            resolve_member_symbol_from_declaration(session, parent_symbol, member_name)
    {
        return Some(symbol_id);
    }

    for interface_symbol in implements {
        if let Some(symbol_id) =
            resolve_member_symbol_from_declaration(session, interface_symbol, member_name)
        {
            return Some(symbol_id);
        }
    }

    for embedded_symbol in embedded {
        if let Some(symbol_id) =
            resolve_member_symbol_from_declaration(session, embedded_symbol, member_name)
        {
            return Some(symbol_id);
        }
    }

    None
}

/// Resolve a member symbol from extensions for a target type.
pub(crate) fn resolve_extension_member_symbol(
    session: &Session,
    target_symbol: GlobalSymbolId,
    current_module_id: ModuleId,
    member_name: &str,
) -> Option<GlobalSymbolId> {
    // track the first matching extension member
    let mut resolved = None;

    for_each_visible_extension(session, target_symbol, current_module_id, |_, extension| {
        // resolve the member within the extension declaration
        if let Some(symbol_id) =
            resolve_member_symbol_from_declaration(session, extension.symbol, member_name)
        {
            resolved = Some(symbol_id);
            return true;
        }

        false
    });

    resolved
}

/// Resolve a nominal type symbol from a type expression.
pub(crate) fn resolve_nominal_symbol_from_type_expression(
    session: &Session,
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    // resolve the expression and target symbol
    let dir_tree = ctx.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    // unwrap type operators and wrappers to the underlying nominal expression
    match expression {
        Expression::ReferenceOf { right, .. }
        | Expression::PointerOf { right, .. }
        | Expression::ValueOf { right, .. }
        | Expression::Maybe { left: right }
        | Expression::Must { left: right } => {
            return resolve_nominal_symbol_from_type_expression(session, ctx, *right);
        }
        Expression::Parenthesized { expression } => {
            return resolve_nominal_symbol_from_type_expression(session, ctx, *expression);
        }
        Expression::Instantiation { left, .. } => {
            return resolve_nominal_symbol_from_type_expression(session, ctx, *left);
        }
        Expression::Member { left, name, .. } => {
            let Some(name) = *name else {
                return None;
            };
            if let Some(symbol_id) =
                resolve_member_access_symbol(session, ctx, expression_id, *left, name)
            {
                return Some(symbol_id);
            }
        }
        _ => {}
    }

    if let Some(target_symbol) = resolve_expression_symbol(ctx, expression_id) {
        // return the target when it already resolves to a nominal type symbol
        if symbol_is_type_symbol(session, target_symbol) {
            return Some(target_symbol);
        }
    }

    // return none when the symbol is not a type
    None
}

/// Resolve an exported symbol from a module in the requested symbol space.
fn resolve_symbol_from_module(
    session: &Session,
    module_id: ModuleId,
    name_id: StringId,
    symbol_space: SymbolSpace,
    visited: &mut HashSet<ModuleId>,
) -> Option<GlobalSymbolId> {
    // avoid cycles across re-export chains
    if !visited.insert(module_id) {
        return None;
    }

    let module = session.modules.get(module_id);
    let module = module.as_ref();
    let ctx = crate::query_context(session, module)?;

    // prefer direct exported declarations before walking re export edges
    {
        let symbols = ctx.symbols();
        for symbol_index in 0..symbols.symbol_count() {
            let local_symbol_id = LocalSymbolId::new(symbol_index);
            let symbol = symbols.get_symbol(local_symbol_id);
            if !symbol.is_active() {
                continue;
            }
            if symbol.export.is_none() {
                continue;
            }
            if symbol.name() != Some(name_id) {
                continue;
            }
            if !matches_symbol_space_filter(symbol.ty, symbol.space, Some(symbol_space)) {
                continue;
            }

            return Some(GlobalSymbolId::new(module_id, local_symbol_id));
        }
    }

    let dir_tree = ctx.tree();
    for (_expr_id, expr) in dir_tree.iter_nodes_of_type::<Expression>() {
        let items = match expr {
            Expression::ReExport { items, .. } => items,
            Expression::Export { items, .. } => items,
            _ => continue,
        };

        for item_id in items {
            let item = dir_tree.get::<DependencyItem>(*item_id);
            let (mode, name, alias, target_module) = match item {
                DependencyItem::Remote {
                    mode,
                    name,
                    alias,
                    target_module,
                    ..
                } => (*mode, *name, *alias, Some(*target_module)),
                _ => continue,
            };
            if !dependency_item_matches_name(name_id, name, alias, mode, true) {
                continue;
            }

            if let Some(target_symbol) = item.target_symbol()
                && symbol_matches_space(session, target_symbol, symbol_space)
            {
                return Some(target_symbol);
            }

            let target_module_id = target_module
                .and_then(|targets| targets.value.or(targets.ty))
                .and_then(|target| target.module_id());
            let Some(target_module_id) = target_module_id else {
                continue;
            };

            if let Some(symbol_id) = resolve_symbol_from_module(
                session,
                target_module_id,
                name_id,
                symbol_space,
                visited,
            ) {
                return Some(symbol_id);
            }
        }
    }

    None
}

/// Resolve a namespace member symbol for a namespace alias symbol.
pub(crate) fn resolve_namespace_member_symbol(
    session: &Session,
    alias_symbol: GlobalSymbolId,
    member_name: StringId,
    symbol_space: SymbolSpace,
) -> Option<GlobalSymbolId> {
    let mut visited = HashSet::new();
    resolve_namespace_member_symbol_inner(
        session,
        alias_symbol,
        member_name,
        symbol_space,
        &mut visited,
    )
}

/// Resolve a namespace member symbol by following alias chains.
fn resolve_namespace_member_symbol_inner(
    session: &Session,
    alias_symbol: GlobalSymbolId,
    member_name: StringId,
    symbol_space: SymbolSpace,
    visited: &mut HashSet<GlobalSymbolId>,
) -> Option<GlobalSymbolId> {
    // avoid symbol cycles across alias chains
    if !visited.insert(alias_symbol) {
        return None;
    }

    // resolve the alias symbol in its owning module
    let module = session.modules.get(alias_symbol.module_id);
    let module = module.as_ref();
    let (forwarded_symbol, direct_scope_symbol, dependency_target) =
        with_dir_resolved_context_for_module(session, module, |alias_ctx| {
            let direct_scope_symbol = resolve_namespace_scope_member_symbol(
                alias_ctx,
                alias_symbol,
                member_name,
                symbol_space,
            );

            let symbols = alias_ctx.symbols();
            let symbol = symbols.get_symbol(alias_symbol.local_id);
            let dependency_target = symbol.primary_declaration.and_then(|declaration| {
                if declaration.local_id.ty != NodeType::DependencyItem {
                    return None;
                }

                let item_id = declaration.local_id.try_into().ok()?;
                let item = alias_ctx.tree().get::<DependencyItem>(item_id);

                match item {
                    DependencyItem::Remote {
                        mode,
                        kind,
                        name,
                        target_module,
                        target_symbol,
                        ..
                    } => Some((
                        *mode,
                        *kind,
                        name.map(|name| name.string()),
                        *target_module,
                        *target_symbol,
                    )),
                    _ => None,
                }
            });

            (
                symbol.target_symbol.or_else(|| {
                    symbol
                        .canonical_symbol
                        .filter(|canonical| *canonical != alias_symbol)
                }),
                direct_scope_symbol,
                dependency_target,
            )
        })?;

    // resolve direct namespace scope members before following alias declarations
    if let Some(symbol_id) = direct_scope_symbol {
        return Some(symbol_id);
    }

    let Some((mode, kind, imported_name, target_module, target_symbol)) = dependency_target else {
        if let Some(next_symbol) = forwarded_symbol {
            return resolve_namespace_member_symbol_inner(
                session,
                next_symbol,
                member_name,
                symbol_space,
                visited,
            );
        }

        return None;
    };

    // follow resolved namespace targets directly before falling back to module export lookup
    if mode == DependencyMode::Namespace {
        return resolve_namespace_member_symbol_inner(
            session,
            target_symbol,
            member_name,
            symbol_space,
            visited,
        );
    }

    // follow named imports through their target module before treating them as namespaces
    if mode != DependencyMode::Namespace || kind != DependencyKind::Value {
        let target_module_id = target_module
            .value
            .or(target_module.ty)
            .and_then(|target| target.module_id());
        if let Some(target_module_id) = target_module_id
            && let Some(imported_name) = imported_name
        {
            let mut visited_modules = HashSet::new();
            if let Some(next_symbol) = resolve_symbol_from_module(
                session,
                target_module_id,
                imported_name,
                SymbolSpace::Value,
                &mut visited_modules,
            ) {
                return resolve_namespace_member_symbol_inner(
                    session,
                    next_symbol,
                    member_name,
                    symbol_space,
                    visited,
                );
            }
        }

        if let Some(next_symbol) = forwarded_symbol {
            return resolve_namespace_member_symbol_inner(
                session,
                next_symbol,
                member_name,
                symbol_space,
                visited,
            );
        }

        return None;
    }

    let target_module_id = target_module
        .value
        .or(target_module.ty)
        .and_then(|target| target.module_id());
    let target_module_id = target_module_id?;
    let mut visited = HashSet::new();
    resolve_symbol_from_module(
        session,
        target_module_id,
        member_name,
        symbol_space,
        &mut visited,
    )
}

/// Resolve one member from a concrete namespace symbol scope.
fn resolve_namespace_scope_member_symbol(
    ctx: DirResolvedContext<'_>,
    namespace_symbol: GlobalSymbolId,
    member_name: StringId,
    symbol_space: SymbolSpace,
) -> Option<GlobalSymbolId> {
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(namespace_symbol.local_id);

    // resolve members from an owned scope first
    if let Some(scope_id) = owned_scope_for_symbol(symbols, namespace_symbol.local_id)
        && let Some(symbol_id) =
            resolve_symbol_from_namespace_scope(symbols, scope_id, member_name, symbol_space)
    {
        return Some(global_symbol(ctx.module_id, symbol_id));
    }

    // resolve direct namespace scopes next
    if symbol.kind == SymbolKind::Namespace
        && let Some(symbol_id) =
            resolve_symbol_from_namespace_scope(symbols, symbol.scope.0, member_name, symbol_space)
    {
        return Some(global_symbol(ctx.module_id, symbol_id));
    }

    // resolve merged namespace scopes for class or type like namespace merges
    let name_id = symbol.name()?;
    let scope = symbols.get_scope_by_id(symbol.scope.0);
    let key = StaticKey::Name(name_id);

    for (candidate_key, candidate_id) in symbols.active_named_symbols(scope) {
        if candidate_key != key {
            continue;
        }

        let candidate = symbols.get_symbol(candidate_id);
        if candidate.kind != SymbolKind::Namespace {
            continue;
        }

        let Some(symbol_id) = resolve_symbol_from_namespace_scope(
            symbols,
            candidate.scope.0,
            member_name,
            symbol_space,
        ) else {
            continue;
        };

        return Some(global_symbol(ctx.module_id, symbol_id));
    }

    None
}

/// Resolve one symbol from one namespace scope by name and symbol space.
fn resolve_symbol_from_namespace_scope(
    symbols: &dir::SymbolTable,
    scope_id: LocalScopeId,
    member_name: StringId,
    symbol_space: SymbolSpace,
) -> Option<LocalSymbolId> {
    let scope = symbols.get_scope_by_id(scope_id);
    let key = StaticKey::Name(member_name);

    for (candidate_key, candidate_id) in symbols.active_named_symbols(scope) {
        if candidate_key != key {
            continue;
        }

        let symbol = symbols.get_symbol(candidate_id);
        if matches_symbol_space_filter(symbol.ty, symbol.space, Some(symbol_space)) {
            return Some(candidate_id);
        }
    }

    None
}

/// Resolve a module id from an import target string.
pub(crate) fn resolve_module_id_for_import_target(
    session: &Session,
    ctx: &QueryContext<'_>,
    target: &str,
) -> Option<ModuleId> {
    let source_file = session.files.get(ctx.file_id);
    let source_path = source_file.path.as_ref()?;
    resolve_module_id_for_import_target_path(session, source_path, target)
}

/// Resolve a module id from an import target path and source file path.
pub(crate) fn resolve_module_id_for_import_target_path(
    session: &Session,
    source_path: &Path,
    target: &str,
) -> Option<ModuleId> {
    let candidate_target = if target.starts_with("file://") {
        Path::new(target.trim_start_matches("file://")).to_path_buf()
    } else {
        let path = Path::new(target);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            source_path.parent()?.join(path)
        }
    };

    // allow module resolution to normalize paths and extensions
    let candidate_target = candidate_target.normalize();
    if let Some(module_id) = session.modules.get_id_by_path(&candidate_target) {
        return Some(module_id);
    }
    let candidate_uri = Uri::from_path(candidate_target.clone());
    if let Some(module_id) = session.modules.get_id_by_uri(&candidate_uri) {
        return Some(module_id);
    }

    // try alternative extensions when initial lookup fails
    let extensions = [
        ".d.ts", ".d.mts", ".d.cts", ".d.ds", ".ts", ".mts", ".cts", ".ds",
    ];
    if let Some(stem) = candidate_target.file_stem() {
        let stem = stem.to_string_lossy();
        let base = candidate_target.parent().unwrap_or_else(|| Path::new(""));
        for extension in extensions {
            let candidate = base.join(format!("{stem}{extension}"));
            if let Some(module_id) = session.modules.get_id_by_path(&candidate) {
                return Some(module_id);
            }

            let candidate_uri = Uri::from_path(candidate.clone());
            if let Some(module_id) = session.modules.get_id_by_uri(&candidate_uri) {
                return Some(module_id);
            }
        }
    }

    None
}

/// Check whether a dependency item matches a name or wildcard export.
pub(crate) fn dependency_item_matches_name(
    name_id: StringId,
    name: Option<Name>,
    alias: Option<StringId>,
    mode: DependencyMode,
    allow_wildcard: bool,
) -> bool {
    // alias takes precedence when present
    if alias == Some(name_id) {
        return true;
    }

    // match the original name when no alias is used
    if alias.is_none() && name.map(|name| name.string()) == Some(name_id) {
        return true;
    }

    // treat export star as a match when allowed
    if allow_wildcard && mode == DependencyMode::Namespace && name.is_none() && alias.is_none() {
        return true;
    }

    false
}

/// Resolve the recorded member target for an expression resolution.
fn recorded_member_resolution(
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    // read the recorded resolution for the expression node
    let types = ctx.types();
    let node_id = GlobalNodeIdAny {
        module_id: ctx.module_id,
        local_id: expression_id.into(),
    };
    let resolution_id = types.get_resolution_for_node(node_id)?;

    // extract a target symbol from the recorded resolution
    let resolution = types.get_resolution(resolution_id);
    match resolution {
        Resolution::Static { candidate, .. } => Some(candidate.target_symbol),
        Resolution::Dynamic { candidates, .. } => {
            candidates.first().map(|candidate| candidate.target_symbol)
        }
        Resolution::Unresolved { candidates, .. } => {
            if candidates.len() == 1 {
                return Some(candidates[0].target_symbol);
            }
            None
        }
        Resolution::Builtin { .. } => None,
    }
}
