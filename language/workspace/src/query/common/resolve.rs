use destack_dir as dir;
use std::collections::HashSet;
use std::path::Path;

use destack_base::StringId;
use destack_dir::{
    Declaration, Declarator, DependencyItem, DependencyKind, DependencyMode, EnumField, Expression,
    GlobalNodeIdAny, GlobalSymbolId, LocalScopeId, LocalSymbolId, Member, Name, NodeType,
    Resolution, SymbolSpace, SymbolType, Type,
};
use destack_source::{ModuleId, PathExt, Uri};

use super::symbol::{member_key_name, resolve_symbol_name_id};
use super::{QueryContext, for_each_visible_extension};
use crate::Session;

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
            SymbolSpace::TypeValue => is_type_symbol(symbol_type),
            _ => is_type_symbol(symbol_type),
        },
        SymbolSpace::Value => {
            symbol_space == SymbolSpace::Value || symbol_space == SymbolSpace::TypeValue
        }
        SymbolSpace::TypeValue => symbol_space == SymbolSpace::TypeValue,
        SymbolSpace::Label => symbol_space == SymbolSpace::Label,
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

fn symbol_matches_space(session: &Session, symbol_id: GlobalSymbolId, space: SymbolSpace) -> bool {
    // resolve the module and query context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return false;
    };

    // resolve the symbol space
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    matches_symbol_space_filter(symbol.ty, symbol.space, Some(space))
}

/// Check whether a symbol represents a nominal type symbol.
fn symbol_is_type_symbol(session: &Session, symbol_id: GlobalSymbolId) -> bool {
    // resolve the module and query context for the symbol
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return false;
    };

    // read the symbol type
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    is_type_symbol(symbol.ty)
}

/// Resolve a member access symbol when the cursor is on the member name.
pub(crate) fn resolve_member_access_symbol(
    session: &Session,
    ctx: &QueryContext<'_>,
    expr_id: dir::LocalNodeId<Expression>,
    left: dir::LocalNodeId<Expression>,
    name: StringId,
) -> Option<GlobalSymbolId> {
    // resolve member symbols from recorded resolution data when available
    if let Some(target) = recorded_member_resolution(ctx, expr_id) {
        return Some(target);
    }

    // resolve the member name string
    let member_name = session.strings.get(name).to_string();

    // resolve the base nominal symbol for the left expression
    let base_symbol = nominal_symbol_for_expression(session, ctx, left)?;

    // resolve members declared directly on the base type
    if let Some(symbol_id) =
        resolve_member_symbol_from_declaration(session, base_symbol, &member_name)
    {
        return Some(symbol_id);
    }

    // resolve members provided by extensions
    resolve_extension_member_symbol(session, base_symbol, ctx.module_id, &member_name)
}

/// Resolve the nominal symbol for an expression, using type info or fallbacks.
pub(crate) fn nominal_symbol_for_expression(
    session: &Session,
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    // resolve the type id from expression or node type tables
    let mut type_id = ctx
        .get_expression_type(expression_id.into())
        .or_else(|| ctx.get_node_type(expression_id.into()));

    // fall back to the expression target symbol when type info is missing
    if type_id.is_none() {
        // resolve the expression and its target symbol
        let dir_tree = ctx.tree();
        let expression = dir_tree.get::<Expression>(expression_id);
        let target_symbol = expression.target_symbol()?;

        // resolve type information for the target symbol
        let types = ctx.types();
        let symbols = ctx.symbols();
        type_id = types.get_type_id_for_symbol(&symbols, target_symbol);

        // fall back to resolving from the target symbol initializer
        if type_id.is_none() {
            return resolve_nominal_symbol_from_initializer(session, ctx, target_symbol);
        }
    }

    // resolve the nominal symbol from the resolved type id
    let type_id = type_id?;
    let types = ctx.types();
    resolve_nominal_symbol_from_type(&types, type_id)
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

fn resolve_member_from_members(
    session: &Session,
    tree: &dir::NodeTree,
    member_ids: &[dir::LocalNodeId<Member>],
    member_name: &str,
) -> Option<LocalSymbolId> {
    // scan members for a matching key name
    for member_id in member_ids {
        let member = tree.get(*member_id);

        // skip members without a static key
        let Some(key) = member.key() else {
            continue;
        };

        // skip members whose key cannot be resolved to a name
        let Some(key_name) = member_key_name(session, key) else {
            continue;
        };

        // return the member symbol when the name matches
        if key_name == member_name {
            return Some(member.symbol());
        }
    }

    None
}

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

fn resolve_member_symbol_from_declaration(
    session: &Session,
    base_symbol: GlobalSymbolId,
    member_name: &str,
) -> Option<GlobalSymbolId> {
    // resolve the module query context
    let module = session.modules.get(base_symbol.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // resolve the primary declaration for the base symbol
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(base_symbol.local_id);
    let declaration = symbol.primary_declaration?;
    drop(symbols);

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
        let symbol_id = resolve_member_from_members(session, &tree, member_ids, member_name);

        // return the member symbol when found
        if let Some(symbol_id) = symbol_id {
            return Some(global_symbol(ctx.module_id, symbol_id));
        }
    }

    // resolve enum fields for enum declarations
    if let Declaration::Enum { fields, .. } = declaration {
        let symbol_id = resolve_member_from_enum_fields(session, &tree, fields, member_name);

        // return the member symbol when found
        if let Some(symbol_id) = symbol_id {
            return Some(global_symbol(ctx.module_id, symbol_id));
        }
    }

    // return none when no member matches
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

fn resolve_nominal_symbol_from_initializer(
    session: &Session,
    ctx: &QueryContext<'_>,
    symbol_id: GlobalSymbolId,
) -> Option<GlobalSymbolId> {
    // resolve the primary declaration for the symbol
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let declaration = symbol.primary_declaration?;
    drop(symbols);

    // find the declarator that owns the declaration
    let dir_tree = ctx.tree();
    let declarator_id = match declaration.local_id.ty {
        NodeType::Declarator => declaration.local_id.try_into().ok(),
        NodeType::Pattern => {
            let parent_id = dir_tree.get_parent(declaration.local_id.id)?;
            if parent_id.ty == NodeType::Declarator {
                parent_id.try_into_typed().ok()
            } else {
                None
            }
        }
        _ => None,
    }?;

    // read the declarator node
    let declarator = dir_tree.get::<Declarator>(declarator_id);

    // resolve the nominal type from the type annotation when present
    let ty_expr_id = declarator.ty;
    if let Some(ty_expr_id) = ty_expr_id {
        let symbol = resolve_nominal_symbol_from_type_expression(session, ctx, ty_expr_id);
        if let Some(symbol) = symbol {
            return Some(symbol);
        }
    }

    // resolve nominal type from initializer when annotation is missing
    if let Some(value_id) = declarator.value {
        return resolve_nominal_symbol_from_value_expression(session, ctx, value_id);
    }

    None
}

fn resolve_nominal_symbol_from_value_expression(
    session: &Session,
    ctx: &QueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let dir_tree = ctx.tree();
    let expression = dir_tree.get::<Expression>(expression_id);
    match expression {
        Expression::New { left, .. } => {
            resolve_nominal_symbol_from_type_expression(session, ctx, *left)
        }
        Expression::Call { left, .. } => {
            resolve_nominal_symbol_from_type_expression(session, ctx, *left)
        }
        Expression::Member { left, name, .. } => {
            resolve_member_access_symbol(session, ctx, expression_id, *left, *name)
        }
        _ => None,
    }
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
    if let Some(target_symbol) = expression.target_symbol() {
        // return the target when it already resolves to a nominal type symbol
        if symbol_is_type_symbol(session, target_symbol) {
            return Some(target_symbol);
        }

        // fall back to resolving dependency aliases into their type targets
        let name_id = resolve_symbol_name_id(session, ctx, target_symbol);
        if let Some(name_id) = name_id
            && let Some(symbol_id) =
                resolve_type_symbol_from_target_symbol(session, ctx, target_symbol, name_id)
        {
            return Some(symbol_id);
        }
    }

    // fall back to resolving through import clauses when unresolved paths persist
    if let Expression::UnresolvedPath { path, .. } = expression
        && let Some(name_id) = path.last_segment()
        && let Some(target_symbol) = resolve_type_symbol_from_imports(session, ctx, name_id)
    {
        return Some(target_symbol);
    }

    // return none when the symbol is not a type
    None
}

fn resolve_type_symbol_from_target_symbol(
    session: &Session,
    ctx: &QueryContext<'_>,
    symbol_id: GlobalSymbolId,
    name_id: StringId,
) -> Option<GlobalSymbolId> {
    // resolve within the current module when the symbol is local
    if symbol_id.module_id == ctx.module_id {
        return resolve_type_symbol_from_target_context(session, ctx, symbol_id, name_id);
    }

    // resolve within the owning module when it differs
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let target_ctx = session.query_context(&module)?;
    resolve_type_symbol_from_target_context(session, &target_ctx, symbol_id, name_id)
}

fn resolve_type_symbol_from_target_context(
    session: &Session,
    ctx: &QueryContext<'_>,
    symbol_id: GlobalSymbolId,
    name_id: StringId,
) -> Option<GlobalSymbolId> {
    // check for dependency items that export the same name
    if let Some(symbol_id) = resolve_type_symbol_from_imports(session, ctx, name_id) {
        return Some(symbol_id);
    }

    // resolve direct dependency targets when the symbol is an import alias
    let symbols = ctx.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let declaration = symbol.primary_declaration?;
    drop(symbols);

    if declaration.local_id.ty != NodeType::DependencyItem {
        return None;
    }

    let Ok(item_id) = declaration.local_id.try_into() else {
        return None;
    };

    let dir_tree = ctx.tree();
    let item = dir_tree.get::<DependencyItem>(item_id);
    let target_symbol = item.target_symbol()?;
    if symbol_matches_space(session, target_symbol, SymbolSpace::Type) {
        return Some(target_symbol);
    }

    // resolve type symbols from dependency target modules
    let name_id = resolve_symbol_name_id(session, ctx, symbol_id)?;
    resolve_type_symbol_from_dependency_symbol(session, ctx, target_symbol, name_id)
}

pub(crate) fn resolve_type_symbol_from_dependency_symbol(
    session: &Session,
    ctx: &QueryContext<'_>,
    symbol_id: GlobalSymbolId,
    name_id: StringId,
) -> Option<GlobalSymbolId> {
    // return direct type symbols
    if symbol_matches_space(session, symbol_id, SymbolSpace::Type) {
        return Some(symbol_id);
    }

    // resolve unresolved dependency targets
    let dir_tree = ctx.tree();
    let declaration = ctx
        .symbols()
        .get_symbol(symbol_id.local_id)
        .primary_declaration?;
    let Ok(item_id) = declaration.local_id.try_into() else {
        return None;
    };
    let item = dir_tree.get::<DependencyItem>(item_id);
    let target_symbol = item.target_symbol()?;
    if symbol_matches_space(session, target_symbol, SymbolSpace::Type) {
        return Some(target_symbol);
    }

    // resolve type symbol from target module
    let target_module_id = item
        .target_module_for_kind(DependencyKind::Type)
        .and_then(|target| target.module_id())?;
    resolve_type_symbol_from_module(session, target_module_id, name_id, &mut HashSet::new())
}

fn resolve_type_symbol_from_imports(
    session: &Session,
    ctx: &QueryContext<'_>,
    name_id: StringId,
) -> Option<GlobalSymbolId> {
    // scan imports for the matching type symbol
    let dir_tree = ctx.tree();
    for item_id in dir_tree.iter_node_ids_of_type::<DependencyItem>() {
        let item = dir_tree.get::<DependencyItem>(item_id);
        let (mode, name, alias, target, target_module) = match item {
            DependencyItem::Remote {
                mode,
                name,
                alias,
                target,
                target_module,
                ..
            } => (*mode, *name, *alias, Some(*target), Some(*target_module)),
            DependencyItem::UnresolvedRemote {
                mode,
                name,
                alias,
                target_module,
                target,
                ..
            } => (*mode, *name, *alias, Some(*target), *target_module),
            _ => continue,
        };

        if !dependency_item_matches_name(name_id, name, alias, mode, false) {
            continue;
        }

        if let Some(target_symbol) = item.target_symbol()
            && symbol_matches_space(session, target_symbol, SymbolSpace::Type)
        {
            return Some(target_symbol);
        }

        let mut target_module_id = target_module
            .and_then(|targets| targets.ty.or(targets.value))
            .and_then(|target| target.module_id());
        if target_module_id.is_none()
            && let Some(target) = target
        {
            let target_text = session.strings.get(target).to_string();
            target_module_id =
                resolve_module_id_for_import_target(session, ctx, target_text.as_str());
        }

        let Some(target_module_id) = target_module_id else {
            continue;
        };

        if let Some(symbol_id) =
            resolve_type_symbol_from_module(session, target_module_id, name_id, &mut HashSet::new())
        {
            return Some(symbol_id);
        }
    }

    None
}

pub(crate) fn resolve_module_id_for_import_target(
    session: &Session,
    ctx: &QueryContext<'_>,
    target: &str,
) -> Option<ModuleId> {
    let candidate_target = if target.starts_with("file://") {
        Path::new(target.trim_start_matches("file://")).to_path_buf()
    } else {
        let path = Path::new(target);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            let source_file = session.files.get(ctx.file_id);
            let source_path = source_file.path.as_ref()?;
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

/// Resolve a type symbol from a module by scanning re-exports and exports.
pub(crate) fn resolve_type_symbol_from_module(
    session: &Session,
    module_id: ModuleId,
    name_id: StringId,
    visited: &mut HashSet<ModuleId>,
) -> Option<GlobalSymbolId> {
    // avoid cycles across re-export chains
    if !visited.insert(module_id) {
        return None;
    }

    let module = session.modules.get(module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    let dir_tree = ctx.tree();
    for (_expr_id, expr) in dir_tree.iter_nodes_of_type::<Expression>() {
        let items = match expr {
            Expression::ReExport { items, .. } | Expression::UnresolvedReExport { items, .. } => {
                items
            }
            Expression::Export { items, .. } => items,
            _ => continue,
        };

        for item_id in items {
            let item = dir_tree.get::<DependencyItem>(*item_id);
            let (mode, name, alias, target, target_module) = match item {
                DependencyItem::Remote {
                    mode,
                    name,
                    alias,
                    target,
                    target_module,
                    ..
                } => (*mode, *name, *alias, Some(*target), Some(*target_module)),
                DependencyItem::UnresolvedRemote {
                    mode,
                    name,
                    alias,
                    target_module,
                    target,
                    ..
                } => (*mode, *name, *alias, Some(*target), *target_module),
                _ => continue,
            };
            if !dependency_item_matches_name(name_id, name, alias, mode, true) {
                continue;
            }

            if let Some(target_symbol) = item.target_symbol()
                && symbol_matches_space(session, target_symbol, SymbolSpace::Type)
            {
                return Some(target_symbol);
            }

            let mut target_module_id = target_module
                .and_then(|targets| targets.ty.or(targets.value))
                .and_then(|target| target.module_id());
            if target_module_id.is_none()
                && let Some(target) = target
            {
                let target_text = session.strings.get(target).to_string();
                target_module_id =
                    resolve_module_id_for_import_target(session, &ctx, target_text.as_str());
            }

            let Some(target_module_id) = target_module_id else {
                continue;
            };

            if let Some(symbol_id) =
                resolve_type_symbol_from_module(session, target_module_id, name_id, visited)
            {
                return Some(symbol_id);
            }
        }
    }

    let exports = ctx.dir.exported_symbols.read();
    for ((space, key), export) in exports.iter() {
        let dir::StaticKey::Name(export_name) = *key else {
            continue;
        };

        if export_name != name_id {
            continue;
        }

        if *space != SymbolSpace::Type && *space != SymbolSpace::TypeValue {
            continue;
        }

        if let Some(target_symbol) = export.target.resolved()
            && symbol_matches_space(session, target_symbol, SymbolSpace::Type)
        {
            return Some(target_symbol);
        }

        if let Some(item_id) = export.item {
            let item = dir_tree.get::<DependencyItem>(item_id);
            let (mode, name, alias, target, target_module) = match item {
                DependencyItem::Remote {
                    mode,
                    name,
                    alias,
                    target,
                    target_module,
                    ..
                } => (*mode, *name, *alias, Some(*target), Some(*target_module)),
                DependencyItem::UnresolvedRemote {
                    mode,
                    name,
                    alias,
                    target_module,
                    target,
                    ..
                } => (*mode, *name, *alias, Some(*target), *target_module),
                _ => continue,
            };

            if !dependency_item_matches_name(name_id, name, alias, mode, true) {
                continue;
            }

            if let Some(target_symbol) = item.target_symbol()
                && symbol_matches_space(session, target_symbol, SymbolSpace::Type)
            {
                return Some(target_symbol);
            }

            let mut target_module_id = target_module
                .and_then(|targets| targets.ty.or(targets.value))
                .and_then(|target| target.module_id());
            if target_module_id.is_none()
                && let Some(target) = target
            {
                let target_text = session.strings.get(target).to_string();
                target_module_id =
                    resolve_module_id_for_import_target(session, &ctx, target_text.as_str());
            }

            let Some(target_module_id) = target_module_id else {
                continue;
            };

            if let Some(symbol_id) =
                resolve_type_symbol_from_module(session, target_module_id, name_id, visited)
            {
                return Some(symbol_id);
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
