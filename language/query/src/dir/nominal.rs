use destack_dir as dir;
use destack_dir::{Expression, GlobalNodeIdAny, GlobalSymbolId, Resolution};
use destack_source::SourcePartKey;

use super::{is_type_symbol, resolve_expression_symbol};
use crate::core::DirQuery;
use destack_workspace::Session;

/// Resolve a member access symbol when the cursor is on the member name.
pub(crate) fn resolve_member_access_symbol(
    dir: DirQuery<'_>,
    expr_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    // prefer the compiler-recorded member target for this source part
    let source_id = dir.tree().get_source(expr_id.id);
    let span_type = Expression::member_source_part(dir.tree(), expr_id);
    if let Some(target_symbol) = dir
        .types()
        .get_symbol_target_for_source_part(SourcePartKey::new(source_id, span_type))
    {
        return Some(target_symbol);
    }

    // prefer the direct member target recorded on the expression
    if let Some(target_symbol) = dir.tree().get::<Expression>(expr_id).target_symbol() {
        return Some(target_symbol);
    }

    // otherwise read the recorded resolution candidate
    if let Some(target_symbol) = recorded_member_resolution(dir, expr_id) {
        return Some(target_symbol);
    }

    None
}

/// Resolve a nominal type symbol from a type expression.
pub(crate) fn resolve_nominal_symbol_from_type_expression(
    session: &Session,
    dir: DirQuery<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    // resolve the expression and target symbol
    let dir_tree = dir.tree();
    let expression = dir_tree.get::<Expression>(expression_id);

    // unwrap type operators and wrappers to the underlying nominal expression
    match expression {
        Expression::ReferenceOf { right, .. }
        | Expression::PointerOf { right, .. }
        | Expression::ValueOf { right, .. }
        | Expression::Maybe { left: right }
        | Expression::Must { left: right } => {
            return resolve_nominal_symbol_from_type_expression(session, dir, *right);
        }
        Expression::Parenthesized { expression } => {
            return resolve_nominal_symbol_from_type_expression(session, dir, *expression);
        }
        Expression::Instantiation { left, .. } => {
            return resolve_nominal_symbol_from_type_expression(session, dir, *left);
        }
        Expression::Member { .. } => {
            if let Some(symbol_id) = resolve_member_access_symbol(dir, expression_id) {
                return Some(symbol_id);
            }
        }
        _ => {}
    }

    if let Some(target_symbol) = resolve_expression_symbol(dir, expression_id)
        && symbol_is_type_symbol(session, target_symbol)
    {
        return Some(target_symbol);
    }

    None
}

/// Check whether a symbol represents a nominal type symbol.
fn symbol_is_type_symbol(session: &Session, symbol_id: GlobalSymbolId) -> bool {
    let module = session.modules.get(symbol_id.module_id);
    let module = module.as_ref();
    crate::core::with_query_context_for_module(session, module, |ctx| {
        if symbol_id.local_id.id >= ctx.dir().resolved_symbols().symbol_count() {
            return false;
        }

        let symbols = ctx.dir().resolved_symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        is_type_symbol(symbol.ty)
    })
    .unwrap_or(false)
}

/// Resolve the recorded member target for an expression resolution.
fn recorded_member_resolution(
    dir: DirQuery<'_>,
    expression_id: dir::LocalNodeId<Expression>,
) -> Option<GlobalSymbolId> {
    let types = dir.types();
    let node_id = GlobalNodeIdAny {
        module_id: dir.module_id(),
        local_id: expression_id.into(),
    };
    let resolution_id = types.get_resolution_for_node(node_id)?;

    let resolution = types.get_resolution(resolution_id);
    match resolution {
        Resolution::Static { candidate, .. } => Some(candidate.target_symbol),
        Resolution::Dynamic { .. } => None,
        Resolution::Unresolved { candidates, .. } => {
            if candidates.len() == 1 {
                return Some(candidates[0].target_symbol);
            }

            None
        }
        Resolution::Builtin { .. } => None,
    }
}
