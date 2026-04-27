use std::collections::HashSet;

use destack_dir::{
    self as dir, DependencyItem, GlobalSymbolId, LocalNodeIdAny, LocalSymbolId, Member, NodeType,
    SymbolSpace,
};
use destack_source::{ModuleId, Span};
use destack_workspace::{Repository, Revision};

use super::{container_name_for_node, declaration_display_name, matches_symbol_space_filter};
use crate::ast::{get_node_tree_main_span, get_node_tree_span, try_span_for_dir_node};
use crate::core::{
    AstQuery, QueryContext, query_context, query_context_for_module_id, with_ast_query_for_module,
};
use destack_artifact::DirResolved;
use destack_workspace::{SymbolIndexEntry, SymbolIndexKind};

/// Build a global symbol id from a module and local symbol id.
pub(crate) fn global_symbol(module_id: ModuleId, local_id: LocalSymbolId) -> GlobalSymbolId {
    GlobalSymbolId {
        module_id,
        local_id,
    }
}

/// Resolve a typed global symbol id from one module-local symbol id.
pub fn resolve_global_symbol_id(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    local_symbol_id: u32,
) -> Option<GlobalSymbolId> {
    let ctx = query_context_for_module_id(repository, revision, module_id)?;
    let symbol_entry = ctx.dir().symbols().get_symbol_by_id(local_symbol_id);

    Some(GlobalSymbolId {
        module_id,
        local_id: LocalSymbolId::new_typed(local_symbol_id, symbol_entry.ty),
    })
}

/// Get the canonical symbol for a given symbol id.
pub(crate) fn get_canonical_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> GlobalSymbolId {
    let Some(ctx) = query_context_for_module_id(repository, revision, symbol_id.module_id) else {
        return symbol_id;
    };

    let canonical = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.canonical_symbol
    };
    if let Some(canonical) = canonical
        && canonical != symbol_id
    {
        return get_canonical_symbol(repository, revision, canonical);
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

    let mut current_symbol = symbol_id;
    let mut visited = HashSet::new();
    while visited.insert(current_symbol) {
        let Some(ctx) = query_context_for_module_id(repository, revision, current_symbol.module_id)
        else {
            return false;
        };

        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(current_symbol.local_id);
        let Some(target_symbol) = symbol.target_symbol else {
            return false;
        };

        if target_symbol == target_symbol_id {
            return true;
        }

        if get_canonical_symbol(repository, revision, target_symbol) == target_symbol_id {
            return true;
        }

        current_symbol = target_symbol;
    }

    false
}

/// Resolve a symbol name string when possible.
pub(crate) fn resolve_symbol_name(
    repository: &Repository,
    revision: Revision,
    symbol_id: dir::GlobalSymbolId,
) -> Option<String> {
    let ctx = query_context_for_module_id(repository, revision, symbol_id.module_id)?;

    let symbols = ctx.dir().symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    symbol
        .name()
        .map(|name_id| repository.strings.get(name_id).to_string())
}

/// Build symbol index entries for one module.
pub(crate) fn build_symbol_index_entries_for_module(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
) -> Vec<SymbolIndexEntry> {
    let Some(ctx) = query_context(repository, revision, module_id) else {
        return Vec::new();
    };

    let dir_tree = ctx.dir().tree();
    let mut entries = Vec::new();

    // declarations, members, enum fields
    for (declaration_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
        let name = declaration_display_name(&repository.strings, declaration);
        let kind = symbol_index_kind_for_declaration(declaration);
        let container_name =
            container_name_for_node(dir_tree, &repository.strings, declaration_id.id);

        let Some(range) = symbol_index_range(&ctx, dir_tree, declaration_id.id) else {
            continue;
        };

        entries.push(SymbolIndexEntry {
            name: name.clone(),
            kind,
            module_id,
            file_id: ctx.file_id(),
            range,
            container_name: container_name.clone(),
        });

        if let Some(member_ids) = declaration.member_ids() {
            for member_id in member_ids {
                let Some(entry) =
                    member_to_symbol_index_entry(repository, &ctx, dir_tree, *member_id, &name)
                else {
                    continue;
                };

                entries.push(entry);
            }
        }

        if let Some(member_ids) = declaration.type_member_ids() {
            for member_id in member_ids {
                let Some(entry) = type_member_to_symbol_index_entry(
                    repository, &ctx, dir_tree, *member_id, &name,
                ) else {
                    continue;
                };

                entries.push(entry);
            }
        }

        if let dir::Declaration::Enum(declaration) = declaration {
            for field_id in &declaration.fields {
                let Some(entry) =
                    enum_field_to_symbol_index_entry(repository, &ctx, dir_tree, *field_id, &name)
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
pub(crate) fn member_key_name(repository: &Repository, key: &dir::Key) -> Option<String> {
    match key {
        dir::Key::Name(name) => Some(repository.strings.get(name.string()).to_string()),
        dir::Key::Private(name) => {
            let name = repository.strings.get(*name).to_string();
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
    with_resolved_symbol_context(
        repository,
        revision,
        symbol_id.module_id,
        |ast, _module_id, resolved| {
            let declaration = {
                let symbols = &resolved.symbols;
                let symbol = symbols.get_symbol(symbol_id.local_id);
                symbol.primary_declaration
            };

            if let Some(declaration) = declaration {
                return Some(get_node_tree_main_span(
                    ast,
                    &resolved.tree,
                    declaration.local_id,
                ));
            }

            None
        },
    )?
}

/// Resolve a definition span when the symbol is a type symbol.
pub(crate) fn type_definition_span_for_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> Option<Span> {
    with_resolved_symbol_context(
        repository,
        revision,
        symbol_id.module_id,
        |_ast, _module_id, resolved| {
            if symbol_id.local_id.id >= resolved.symbols.symbol_count() {
                return None;
            }

            let (is_type_symbol, declaration) = {
                let symbols = &resolved.symbols;
                let symbol = symbols.get_symbol(symbol_id.local_id);
                (
                    matches_symbol_space_filter(symbol.ty, symbol.space, Some(SymbolSpace::Type)),
                    symbol.primary_declaration,
                )
            };

            if !is_type_symbol {
                return None;
            }

            if declaration
                .is_some_and(|declaration| declaration.local_id.ty == NodeType::DependencyItem)
            {
                let target_symbol = dependency_item_target_symbol(resolved, symbol_id)?;
                return get_symbol_span_with(
                    repository,
                    revision,
                    target_symbol,
                    get_node_tree_main_span,
                );
            }

            get_symbol_span_with(repository, revision, symbol_id, get_node_tree_main_span)
        },
    )?
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
fn symbol_index_kind_for_declaration(declaration: &dir::Declaration) -> SymbolIndexKind {
    match declaration {
        dir::Declaration::Global { .. } => SymbolIndexKind::Namespace,
        dir::Declaration::Function { .. } => SymbolIndexKind::Function,
        dir::Declaration::Struct { .. } => SymbolIndexKind::Struct,
        dir::Declaration::Class { .. } => SymbolIndexKind::Class,
        dir::Declaration::Interface { .. } => SymbolIndexKind::Interface,
        dir::Declaration::Enum { .. } => SymbolIndexKind::Enum,
        dir::Declaration::Namespace { .. } => SymbolIndexKind::Namespace,
        dir::Declaration::Type { .. } => SymbolIndexKind::TypeParameter,
        dir::Declaration::ImportAlias { .. } => SymbolIndexKind::Variable,
        dir::Declaration::Extension { .. } => SymbolIndexKind::Class,
    }
}

/// Convert one member to one symbol index entry.
fn member_to_symbol_index_entry(
    repository: &Repository,
    ctx: &QueryContext,
    dir_tree: &dir::Tree,
    member_id: dir::LocalNodeId<dir::Member>,
    container_name: &str,
) -> Option<SymbolIndexEntry> {
    let member = dir_tree.get::<dir::Member>(member_id);

    // member name and kind
    let kind = symbol_index_kind_for_member(member)?;
    let name = if let Some(name) = member.name() {
        repository.strings.get(name).to_string()
    } else {
        let key = member.key()?;
        member_key_name(repository, key)?
    };

    let range = symbol_index_range(ctx, dir_tree, member_id.id)?;

    if is_synthetic_function_keyword_field(member, &name, range) {
        return None;
    }

    Some(SymbolIndexEntry {
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
    repository: &Repository,
    ctx: &QueryContext,
    dir_tree: &dir::Tree,
    member_id: dir::LocalNodeId<dir::TypeMember>,
    container_name: &str,
) -> Option<SymbolIndexEntry> {
    let member = dir_tree.get::<dir::TypeMember>(member_id);

    // type member name and kind
    let kind = symbol_index_kind_for_type_member(member)?;
    let name = if let Some(name) = member.name() {
        repository.strings.get(name).to_string()
    } else {
        let key = member.key()?;
        member_key_name(repository, key)?
    };

    let range = symbol_index_range(ctx, dir_tree, member_id.id)?;

    Some(SymbolIndexEntry {
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
    repository: &Repository,
    ctx: &QueryContext,
    dir_tree: &dir::Tree,
    field_id: dir::LocalNodeId<dir::EnumField>,
    container_name: &str,
) -> Option<SymbolIndexEntry> {
    let field = dir_tree.get::<dir::EnumField>(field_id);
    let range = symbol_index_range(ctx, dir_tree, field_id.id)?;

    Some(SymbolIndexEntry {
        name: repository.strings.get(field.name.string()).to_string(),
        kind: SymbolIndexKind::EnumMember,
        module_id: ctx.module_id(),
        file_id: ctx.file_id(),
        range,
        container_name: Some(container_name.to_string()),
    })
}

/// Map one member to the symbol index kind.
fn symbol_index_kind_for_member(member: &dir::Member) -> Option<SymbolIndexKind> {
    match member {
        dir::Member::AssociatedType { .. } => Some(SymbolIndexKind::TypeParameter),
        dir::Member::AssociatedConst { .. } => Some(SymbolIndexKind::Constant),
        dir::Member::Field { .. } => Some(SymbolIndexKind::Field),
        dir::Member::Method { .. } => Some(SymbolIndexKind::Method),
        dir::Member::Embed { .. }
        | dir::Member::StaticBlock { .. }
        | dir::Member::ComptimeBlock { .. }
        | dir::Member::Error { .. } => None,
    }
}

/// Map one type member to the symbol index kind.
fn symbol_index_kind_for_type_member(member: &dir::TypeMember) -> Option<SymbolIndexKind> {
    match member {
        dir::TypeMember::AssociatedType { .. } => Some(SymbolIndexKind::TypeParameter),
        dir::TypeMember::AssociatedConst { .. } => Some(SymbolIndexKind::Constant),
        dir::TypeMember::Field { .. } => Some(SymbolIndexKind::Field),
        dir::TypeMember::Method { .. } => Some(SymbolIndexKind::Method),
        dir::TypeMember::CallSignature { .. } => Some(SymbolIndexKind::Method),
        dir::TypeMember::ConstructSignature { .. } => Some(SymbolIndexKind::Method),
        dir::TypeMember::IndexSignature { .. } => None,
        dir::TypeMember::Embed { .. } => None,
        dir::TypeMember::Error { .. } => None,
    }
}

/// Resolve one symbol index range without failing the whole query on bad source ids.
fn symbol_index_range(ctx: &QueryContext, dir_tree: &dir::Tree, node_id: u32) -> Option<Span> {
    let node_id = LocalNodeIdAny::new(node_id, dir_tree.get_node_type(node_id));
    try_span_for_dir_node(ctx.ast(), dir_tree, node_id)
}

/// Resolve a symbol span using the provided declaration span strategy.
fn get_symbol_span_with(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
    span_for_declaration: impl Fn(AstQuery<'_>, &dir::Tree, LocalNodeIdAny) -> Span + Copy,
) -> Option<Span> {
    with_resolved_symbol_context(
        repository,
        revision,
        symbol_id.module_id,
        |ast, _module_id, resolved| {
            let (canonical_id, declaration, target_symbol) = {
                let symbols = &resolved.symbols;
                let symbol = symbols.get_symbol(symbol_id.local_id);
                let canonical_id = symbol.canonical_symbol.unwrap_or(symbol_id);

                if canonical_id.module_id != symbol_id.module_id {
                    (canonical_id, None, None)
                } else {
                    let canonical_symbol = symbols.get_symbol(canonical_id.local_id);
                    (
                        canonical_id,
                        canonical_symbol.primary_declaration,
                        canonical_symbol.target_symbol,
                    )
                }
            };
            if canonical_id.module_id != symbol_id.module_id {
                return get_symbol_span_with(
                    repository,
                    revision,
                    canonical_id,
                    span_for_declaration,
                );
            }

            if let Some(declaration) = declaration {
                return Some(span_for_declaration(
                    ast,
                    &resolved.tree,
                    declaration.local_id,
                ));
            }

            if let Some(target) = target_symbol {
                return get_symbol_span_with(repository, revision, target, span_for_declaration);
            }

            None
        },
    )?
}

/// Execute a closure with AST and resolved DIR for one module.
fn with_resolved_symbol_context<T>(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    f: impl FnOnce(AstQuery<'_>, ModuleId, &DirResolved) -> T,
) -> Option<T> {
    let module = repository.module(revision, module_id).ok().flatten()?;
    let profile = repository
        .default_profile_id_for_module(revision, module.id)
        .ok()?;
    let selected_profile =
        repository.available_profile_id_for_module(revision, module.id, profile, true)?;
    let resolved = repository.dir_resolved(revision, module.id, selected_profile)?;

    with_ast_query_for_module(repository, revision, module.id, |ast| {
        f(ast, module.id, resolved.as_ref())
    })
}

/// Resolve the imported target symbol for a dependency-item binding.
fn dependency_item_target_symbol(
    resolved: &DirResolved,
    symbol_id: GlobalSymbolId,
) -> Option<GlobalSymbolId> {
    let declaration = {
        let symbols = &resolved.symbols;
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.primary_declaration?
    };
    if declaration.local_id.ty != NodeType::DependencyItem {
        return None;
    }

    let item_id = declaration.local_id.try_into().ok()?;
    let resolved_item = resolved.tree.get::<DependencyItem>(item_id);
    resolved_item.target_symbol()
}
