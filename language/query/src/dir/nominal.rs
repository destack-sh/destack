use destack_dir as dir;
use destack_dir::{Expression, GlobalNodeIdAny, GlobalSymbolId, Resolution};

use super::{expression_symbol_target, is_type_symbol};
use crate::core::{DirQueryContext, ModuleQueryContext, NominalEntry, NominalRelation};

/// Return the recorded symbol target for one member access.
pub(crate) fn member_access_symbol_target(
    dir: DirQueryContext<'_>,
    expr_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    recorded_member_resolution(dir, expr_id)
}

/// Return the nominal type symbol named by one type expression.
pub(crate) fn resolve_nominal_symbol_from_type_expression(
    dir: DirQueryContext<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    // inspect the expression shape
    let view = dir.view();
    let expression = view.get::<Expression>(expression_id);

    // unwrap type operators and wrappers to the underlying nominal expression
    match expression {
        Expression::BorrowOf { right, .. }
        | Expression::MoveOf { right, .. }
        | Expression::Maybe { left: right, .. }
        | Expression::Must { left: right, .. } => {
            return resolve_nominal_symbol_from_type_expression(dir, *right);
        }
        Expression::Parenthesized { expression } => {
            return resolve_nominal_symbol_from_type_expression(dir, *expression);
        }
        Expression::Instantiation { left, .. } => {
            return resolve_nominal_symbol_from_type_expression(dir, *left);
        }
        Expression::Member { .. } => {
            if let Some(symbol_id) = member_access_symbol_target(dir, expression_id) {
                return Some(symbol_id);
            }
        }
        _ => {}
    }

    if let Some(target_symbol) = expression_symbol_target(dir, expression_id)
        && symbol_is_type_symbol(dir, target_symbol)
    {
        return Some(target_symbol);
    }

    None
}

/// Build nominal index entries for one module.
pub(crate) fn build_nominal_relations_for_module(
    ctx: &ModuleQueryContext<'_>,
) -> Vec<NominalEntry> {
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
    }

    entries
}

/// Check whether a symbol represents a nominal type symbol.
fn symbol_is_type_symbol(dir: DirQueryContext<'_>, symbol_id: GlobalSymbolId) -> bool {
    let Some(ctx) = dir.module_context(symbol_id.module_id) else {
        return false;
    };

    let symbols = ctx.dir().symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);

    is_type_symbol(symbol.form)
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
        Resolution::Dispatch(dir::DispatchResolution::Static { target, .. }) => {
            if !dir.symbol_is_visible(target.symbol) {
                return None;
            }

            Some(target.symbol)
        }
        Resolution::Dispatch(dir::DispatchResolution::Dynamic { targets, .. }) => {
            if targets.len() == 1 {
                let symbol_id = targets[0].symbol;
                if !dir.symbol_is_visible(symbol_id) {
                    return None;
                }

                return Some(symbol_id);
            }

            None
        }
        Resolution::Symbol(symbol_id) => {
            if !dir.symbol_is_visible(*symbol_id) {
                return None;
            }

            Some(*symbol_id)
        }
        Resolution::Dispatch(dir::DispatchResolution::Builtin { .. })
        | Resolution::Dependency(_)
        | Resolution::Label(_) => None,
    }
}
