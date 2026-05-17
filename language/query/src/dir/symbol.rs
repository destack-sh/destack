use destack_dir as dir;
use std::collections::HashSet;

use destack_core::StringPool;
use destack_dir::{GlobalSymbolId, LocalNodeIdAny, LocalSymbolId, Member, NodeType, SymbolSpace};
use destack_source::{ModuleId, Span};
use destack_workspace::{Repository, Revision};

use super::{
    container_name_for_node, declaration_display_name, dependency_symbol_target,
    matches_symbol_space_filter,
};
use crate::core::{
    DirQueryContext, QueryContext, SourceQueryContext, SymbolEntry, SymbolEntryKind, query_context,
};
use crate::source::{get_node_tree_main_span, get_node_tree_span, try_span_for_dir_node};

/// Build a global symbol id from a module and local symbol id.
pub(crate) fn global_symbol(module_id: ModuleId, local_id: LocalSymbolId) -> GlobalSymbolId {
    GlobalSymbolId {
        module_id,
        local_id,
    }
}

/// Resolve the local symbol declared by one DIR node.
pub(crate) fn local_symbol_for_node(
    dir: DirQueryContext<'_>,
    node_id: LocalNodeIdAny,
) -> Option<LocalSymbolId> {
    dir.symbol_for_node(node_id)
}

/// Resolve the global symbol declared by one DIR node.
pub(crate) fn global_symbol_for_node(
    dir: DirQueryContext<'_>,
    node_id: LocalNodeIdAny,
) -> Option<GlobalSymbolId> {
    local_symbol_for_node(dir, node_id).map(|symbol_id| global_symbol(dir.module_id(), symbol_id))
}

/// Resolve a global symbol id from one module-local symbol id.
pub fn resolve_global_symbol_id(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    local_symbol_id: u32,
) -> Option<GlobalSymbolId> {
    let ctx = query_context(repository, revision, module_id)?;
    let symbols = ctx.dir().symbols();
    if local_symbol_id >= symbols.symbol_count() {
        return None;
    }

    Some(GlobalSymbolId {
        module_id,
        local_id: LocalSymbolId::new(local_symbol_id),
    })
}

/// Get the canonical symbol for a given symbol id.
pub(crate) fn get_canonical_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> GlobalSymbolId {
    let mut symbol_id = symbol_id;
    let mut seen = HashSet::new();

    // follow import bindings through recorded dependency resolutions
    while seen.insert(symbol_id) {
        let Some(ctx) = query_context(repository, revision, symbol_id.module_id) else {
            return symbol_id;
        };

        let symbols = ctx.dir().symbols();
        if symbol_id.local_id.id >= symbols.symbol_count() {
            return symbol_id;
        }

        let symbol = symbols.get_symbol(symbol_id.local_id);
        let Some(declaration) = symbol.declaration else {
            return symbol_id;
        };

        if declaration.local_id.ty != NodeType::DependencyItem {
            return symbol_id;
        }

        let Ok(item_id) = declaration.local_id.try_into() else {
            return symbol_id;
        };

        let Some(target_symbol) = dependency_symbol_target(ctx.dir(), item_id) else {
            return symbol_id;
        };

        symbol_id = target_symbol;
    }

    symbol_id
}

/// Check whether one symbol still refers to one target for reference queries.
pub(crate) fn symbol_matches_reference_target(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
    target_symbol_id: GlobalSymbolId,
) -> bool {
    if symbol_id == target_symbol_id {
        return true;
    }

    if get_canonical_symbol(repository, revision, symbol_id) == target_symbol_id {
        return true;
    }

    false
}

/// Resolve a symbol name string when possible.
pub(crate) fn resolve_symbol_name(
    repository: &Repository,
    revision: Revision,
    symbol_id: dir::GlobalSymbolId,
) -> Option<String> {
    let ctx = query_context(repository, revision, symbol_id.module_id)?;

    let symbols = ctx.dir().symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    symbol
        .name()
        .map(|name_id| ctx.dir().strings().get(name_id).to_string())
}

/// Build symbol index entries for one module.
pub(crate) fn build_workspace_symbol_candidates_for_module(
    ctx: &QueryContext<'_>,
) -> Vec<SymbolEntry> {
    let dir_tree = ctx.dir().view();
    let mut entries = Vec::new();
    let module_id = ctx.module_id();

    // declarations, members, enum fields
    for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
        let name = declaration_display_name(ctx.dir().strings(), declaration);
        let kind = symbol_index_kind_for_declaration(declaration);
        let container_name =
            container_name_for_node(dir_tree, ctx.dir().strings(), declaration_id.into());

        let Some(range) = symbol_index_range(&ctx, dir_tree, declaration_id.id) else {
            continue;
        };

        entries.push(SymbolEntry {
            name: name.clone(),
            kind,
            module_id,
            file_id: ctx.file_id(),
            range,
            container_name: container_name.clone(),
        });

        if let Some(member_ids) = declaration.member_ids() {
            for member_id in member_ids {
                let Some(entry) = member_to_symbol_index_entry(&ctx, dir_tree, *member_id, &name)
                else {
                    continue;
                };

                entries.push(entry);
            }
        }

        if let Some(member_ids) = declaration.type_member_ids() {
            for member_id in member_ids {
                let Some(entry) =
                    type_member_to_symbol_index_entry(&ctx, dir_tree, *member_id, &name)
                else {
                    continue;
                };

                entries.push(entry);
            }
        }

        if let dir::Declaration::Enum(declaration) = declaration {
            for field_id in &declaration.fields {
                let Some(entry) =
                    enum_field_to_symbol_index_entry(&ctx, dir_tree, *field_id, &name)
                else {
                    continue;
                };

                entries.push(entry);
            }
        }
    }

    entries
}

/// Resolve a member name from a key.
pub(crate) fn member_key_name(strings: &StringPool, key: &dir::Key) -> Option<String> {
    match key {
        dir::Key::Name(name) => Some(strings.get(name.string()).to_string()),
        dir::Key::Private(name) => {
            let name = strings.get(*name).to_string();
            Some(format!("#{name}"))
        }
        dir::Key::Expression(_) => None,
    }
}

/// Check whether a member field is the synthetic `function` keyword placeholder.
pub(crate) fn is_synthetic_function_keyword_field(
    member: &Member,
    name: &str,
    range: Span,
) -> bool {
    if !matches!(member, Member::Field { .. }) || name != "function" {
        return false;
    }

    let full_len = range.end.saturating_sub(range.start);
    full_len == 8
}

/// Get the definition span of a symbol.
pub(crate) fn get_symbol_definition_span(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> Option<Span> {
    get_symbol_span_with(repository, revision, symbol_id, get_node_tree_main_span)
}

/// Get the local definition span of a symbol without canonical expansion.
pub(crate) fn get_symbol_local_definition_span(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> Option<Span> {
    with_symbol_context(repository, revision, symbol_id.module_id, |ctx| {
        let declaration = {
            let symbols = ctx.dir().symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration
        };

        if let Some(declaration) = declaration {
            return Some(get_node_tree_main_span(
                ctx.source(),
                ctx.dir().view(),
                declaration.local_id,
            ));
        }

        None
    })?
}

/// Resolve a definition span when the symbol is a type symbol.
pub(crate) fn type_definition_span_for_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> Option<Span> {
    with_symbol_context(repository, revision, symbol_id.module_id, |ctx| {
        if symbol_id.local_id.id >= ctx.dir().symbols().symbol_count() {
            return None;
        }

        let (is_type_symbol, declaration) = {
            let symbols = ctx.dir().symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            (
                matches_symbol_space_filter(symbol.form, Some(SymbolSpace::Type)),
                symbol.declaration,
            )
        };

        if !is_type_symbol {
            return None;
        }

        if declaration
            .is_some_and(|declaration| declaration.local_id.ty == NodeType::DependencyItem)
        {
            let target_symbol = dependency_item_target_symbol(ctx.dir(), symbol_id)?;
            return get_symbol_span_with(
                repository,
                revision,
                target_symbol,
                get_node_tree_main_span,
            );
        }

        get_symbol_span_with(repository, revision, symbol_id, get_node_tree_main_span)
    })?
}

/// Get the full declaration span of a symbol.
pub(crate) fn get_symbol_declaration_span(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> Option<Span> {
    get_symbol_span_with(repository, revision, symbol_id, get_node_tree_span)
}

/// Map one declaration to the symbol index kind.
fn symbol_index_kind_for_declaration(declaration: &dir::Declaration) -> SymbolEntryKind {
    match declaration {
        dir::Declaration::Global { .. } => SymbolEntryKind::Namespace,
        dir::Declaration::Module { .. } => SymbolEntryKind::Namespace,
        dir::Declaration::Function { .. } => SymbolEntryKind::Function,
        dir::Declaration::Struct { .. } => SymbolEntryKind::Struct,
        dir::Declaration::Class { .. } => SymbolEntryKind::Class,
        dir::Declaration::Interface { .. } => SymbolEntryKind::Interface,
        dir::Declaration::Enum { .. } => SymbolEntryKind::Enum,
        dir::Declaration::Type { .. } => SymbolEntryKind::TypeParameter,
        dir::Declaration::Extension { .. } => SymbolEntryKind::Class,
    }
}

/// Convert one member to one symbol index entry.
fn member_to_symbol_index_entry(
    ctx: &QueryContext<'_>,
    dir_tree: dir::View<'_>,
    member_id: dir::LocalNodeId<dir::Member>,
    container_name: &str,
) -> Option<SymbolEntry> {
    let member = dir_tree.get::<dir::Member>(member_id);

    // member name and kind
    let kind = symbol_index_kind_for_member(member)?;
    let name = if let Some(name) = member.name() {
        ctx.dir().strings().get(name).to_string()
    } else {
        let key = member.key()?;
        member_key_name(ctx.dir().strings(), key)?
    };

    let range = symbol_index_range(ctx, dir_tree, member_id.id)?;

    if is_synthetic_function_keyword_field(member, &name, range) {
        return None;
    }

    Some(SymbolEntry {
        name,
        kind,
        module_id: ctx.module_id(),
        file_id: ctx.file_id(),
        range,
        container_name: Some(container_name.to_string()),
    })
}

/// Convert one type member to one symbol index entry.
fn type_member_to_symbol_index_entry(
    ctx: &QueryContext<'_>,
    dir_tree: dir::View<'_>,
    member_id: dir::LocalNodeId<dir::TypeMember>,
    container_name: &str,
) -> Option<SymbolEntry> {
    let member = dir_tree.get::<dir::TypeMember>(member_id);

    // type member name and kind
    let kind = symbol_index_kind_for_type_member(member)?;
    let name = if let Some(name) = member.name() {
        ctx.dir().strings().get(name).to_string()
    } else {
        let key = member.key()?;
        member_key_name(ctx.dir().strings(), key)?
    };

    let range = symbol_index_range(ctx, dir_tree, member_id.id)?;

    Some(SymbolEntry {
        name,
        kind,
        module_id: ctx.module_id(),
        file_id: ctx.file_id(),
        range,
        container_name: Some(container_name.to_string()),
    })
}

/// Convert one enum field to one symbol index entry.
fn enum_field_to_symbol_index_entry(
    ctx: &QueryContext<'_>,
    dir_tree: dir::View<'_>,
    field_id: dir::LocalNodeId<dir::EnumField>,
    container_name: &str,
) -> Option<SymbolEntry> {
    let field = dir_tree.get::<dir::EnumField>(field_id);
    let range = symbol_index_range(ctx, dir_tree, field_id.id)?;

    Some(SymbolEntry {
        name: ctx.dir().strings().get(field.name.string()).to_string(),
        kind: SymbolEntryKind::EnumMember,
        module_id: ctx.module_id(),
        file_id: ctx.file_id(),
        range,
        container_name: Some(container_name.to_string()),
    })
}

/// Map one member to the symbol index kind.
fn symbol_index_kind_for_member(member: &dir::Member) -> Option<SymbolEntryKind> {
    match member {
        dir::Member::AssociatedType { .. } => Some(SymbolEntryKind::TypeParameter),
        dir::Member::AssociatedConst { .. } => Some(SymbolEntryKind::Constant),
        dir::Member::Field { .. } => Some(SymbolEntryKind::Field),
        dir::Member::Method { .. } => Some(SymbolEntryKind::Method),
        dir::Member::StaticBlock { .. }
        | dir::Member::ComptimeBlock { .. }
        | dir::Member::Error => None,
    }
}

/// Map one type member to the symbol index kind.
fn symbol_index_kind_for_type_member(member: &dir::TypeMember) -> Option<SymbolEntryKind> {
    match member {
        dir::TypeMember::AssociatedType { .. } => Some(SymbolEntryKind::TypeParameter),
        dir::TypeMember::AssociatedConst { .. } => Some(SymbolEntryKind::Constant),
        dir::TypeMember::Field { .. } => Some(SymbolEntryKind::Field),
        dir::TypeMember::Method { .. } => Some(SymbolEntryKind::Method),
        dir::TypeMember::CallSignature { .. } => Some(SymbolEntryKind::Method),
        dir::TypeMember::ConstructSignature { .. } => Some(SymbolEntryKind::Method),
        dir::TypeMember::IndexSignature { .. } => None,
        dir::TypeMember::Error => None,
    }
}

/// Resolve one symbol index range without failing the whole query on bad source ids.
fn symbol_index_range(
    ctx: &QueryContext<'_>,
    dir_tree: dir::View<'_>,
    node_id: u32,
) -> Option<Span> {
    let node_id = LocalNodeIdAny::new(node_id, dir_tree.get_node_type(node_id));
    try_span_for_dir_node(ctx.source(), dir_tree, node_id)
}

/// Resolve a symbol span using the provided declaration span strategy.
fn get_symbol_span_with(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
    span_for_declaration: impl Fn(SourceQueryContext<'_>, dir::View<'_>, LocalNodeIdAny) -> Span + Copy,
) -> Option<Span> {
    with_symbol_context(repository, revision, symbol_id.module_id, |ctx| {
        let (canonical_id, declaration) = {
            let symbols = ctx.dir().symbols();
            let canonical_id = get_canonical_symbol(repository, revision, symbol_id);

            if canonical_id.module_id != symbol_id.module_id {
                (canonical_id, None)
            } else {
                let canonical_symbol = symbols.get_symbol(canonical_id.local_id);
                (canonical_id, canonical_symbol.declaration)
            }
        };
        if canonical_id.module_id != symbol_id.module_id {
            return get_symbol_span_with(repository, revision, canonical_id, span_for_declaration);
        }

        if let Some(declaration) = declaration {
            return Some(span_for_declaration(
                ctx.source(),
                ctx.dir().view(),
                declaration.local_id,
            ));
        }

        None
    })?
}

/// Execute a closure with one query context for a symbol module.
fn with_symbol_context<T>(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    f: impl FnOnce(QueryContext<'_>) -> T,
) -> Option<T> {
    let ctx = query_context(repository, revision, module_id)?;

    Some(f(ctx))
}

/// Return the imported target symbol for a dependency-item binding.
fn dependency_item_target_symbol(
    dir: DirQueryContext<'_>,
    symbol_id: GlobalSymbolId,
) -> Option<GlobalSymbolId> {
    let declaration = {
        let symbols = dir.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.declaration?
    };
    if declaration.local_id.ty != NodeType::DependencyItem {
        return None;
    }

    let item_id = declaration.local_id.try_into().ok()?;
    dependency_symbol_target(dir, item_id)
}
