use destack_dir as dir;
use destack_dir::{Expression, GlobalNodeIdAny, GlobalSymbolId, Resolution};
use destack_source::{ModuleId, ProfileId};
use destack_workspace::{Repository, Revision};

use super::{expression_symbol_target, is_type_symbol};
use crate::core::{
    DirQueryContext, NominalEntry, NominalRelation, query_context_for_profile,
    with_query_context_for_module,
};

/// Return the recorded symbol target for one member access.
pub(crate) fn member_access_symbol_target(
    dir: DirQueryContext<'_>,
    expr_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    recorded_member_resolution(dir, expr_id)
}

/// Return the nominal type symbol named by one type expression.
pub(crate) fn resolve_nominal_symbol_from_type_expression(
    repository: &Repository,
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    // inspect the expression shape
    let dir_tree = dir.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    // unwrap type operators and wrappers to the underlying nominal expression
    match expression {
        Expression::BorrowOf { right, .. }
        | Expression::PointerOf { right, .. }
        | Expression::MoveOf { right, .. }
        | Expression::Maybe { left: right }
        | Expression::Must { left: right } => {
            return resolve_nominal_symbol_from_type_expression(repository, dir, *right);
        }
        Expression::Parenthesized { expression } => {
            return resolve_nominal_symbol_from_type_expression(repository, dir, *expression);
        }
        Expression::Instantiation { left, .. } => {
            return resolve_nominal_symbol_from_type_expression(repository, dir, *left);
        }
        Expression::Member { .. } => {
            if let Some(symbol_id) = member_access_symbol_target(dir, expression_id) {
                return Some(symbol_id);
            }
        }
        _ => {}
    }

    if let Some(target_symbol) = expression_symbol_target(dir, expression_id)
        && symbol_is_type_symbol(repository, dir.revision(), target_symbol)
    {
        return Some(target_symbol);
    }

    None
}

/// Build nominal index entries for one module.
pub(crate) fn build_nominal_relations_for_module(
    repository: &Repository,
    revision: Revision,
    module_id: ModuleId,
    profile_id: ProfileId,
) -> Vec<NominalEntry> {
    let Some(ctx) = query_context_for_profile(repository, revision, module_id, profile_id) else {
        return Vec::new();
    };

    let mut entries = Vec::new();

    // collect direct nominal edges from stored lineages
    for (source_symbol, lineage) in ctx.dir().types().iter_lineages() {
        if let Some(target_symbol) = lineage.extends {
            entries.push(NominalEntry {
                source_symbol,
                target_symbol,
                relation: NominalRelation::Extends,
            });
        }

        for target_symbol in lineage.implements.iter().copied() {
            entries.push(NominalEntry {
                source_symbol,
                target_symbol,
                relation: NominalRelation::Implements,
            });
        }

        for target_symbol in lineage.embedded.iter().copied() {
            entries.push(NominalEntry {
                source_symbol,
                target_symbol,
                relation: NominalRelation::Embeds,
            });
        }
    }

    entries
}

/// Check whether a symbol represents a nominal type symbol.
fn symbol_is_type_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
) -> bool {
    with_query_context_for_module(repository, revision, symbol_id.module_id, |ctx| {
        if symbol_id.local_id.id >= ctx.dir().symbols().symbol_count() {
            return false;
        }

        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        is_type_symbol(symbol.form)
    })
    .unwrap_or(false)
}

/// Return one unambiguous symbol target from a recorded expression resolution.
fn recorded_member_resolution(
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let types = dir.types();
    let node_id = GlobalNodeIdAny {
        module_id: dir.module_id(),
        local_id: expression_id.into(),
    };
    let resolution = types.resolution(node_id)?;
    match resolution {
        Resolution::Dispatch(dir::DispatchResolution::Static { target, .. }) => Some(target.symbol),
        Resolution::Dispatch(dir::DispatchResolution::Dynamic { targets, .. }) => {
            if targets.len() == 1 {
                return Some(targets[0].symbol);
            }

            None
        }
        Resolution::Symbol(dir::SymbolResolution::Target(symbol_id)) => Some(*symbol_id),
        Resolution::Symbol(dir::SymbolResolution::Candidates(symbols)) => {
            if symbols.len() == 1 {
                return Some(symbols[0]);
            }

            None
        }
        Resolution::Dispatch(dir::DispatchResolution::Builtin { .. })
        | Resolution::Dependency(_)
        | Resolution::Control(_) => None,
    }
}
