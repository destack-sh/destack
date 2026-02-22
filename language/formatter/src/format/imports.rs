use destack_ast::{DependencyItem, Expression, ImportSource, ImportTarget, LocalNodeId, NodeTree};
use destack_base::ImmutableStringPool;
use destack_workspace::ImportSortOrder;
use destack_workspace::common::{
    ImportDeclarationKey, categorize_import, sort_dependency_items as sort_items,
    sort_import_declaration_indices,
};

/// Get the inner import expression, unwrapping Statement if needed.
pub fn get_import_expression(
    expr_id: LocalNodeId<Expression>,
    tree: &NodeTree,
) -> Option<&Expression> {
    let expr = tree.get(expr_id);
    match expr {
        Expression::Import {
            source: ImportSource::ImportCall,
            ..
        } => None,
        Expression::Import { target, .. } => {
            if matches!(target, ImportTarget::String(_)) {
                Some(expr)
            } else {
                None
            }
        }
        Expression::Statement(inner_id) => {
            let inner = tree.get(*inner_id);
            if matches!(
                inner,
                Expression::Import {
                    source: ImportSource::ImportStatement | ImportSource::ImportEquals,
                    target: ImportTarget::String(_),
                    ..
                }
            ) {
                Some(inner)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Check if an expression is an import (unwrapping Statement if needed).
pub fn is_import(expr_id: LocalNodeId<Expression>, tree: &NodeTree) -> bool {
    get_import_expression(expr_id, tree).is_some()
}

/// Sort import expressions by group and then alphabetically within each group.
///
/// Side-effect imports (no items) preserve their relative order and stay at the top.
pub fn sort_imports(
    imports: &[LocalNodeId<Expression>],
    tree: &NodeTree,
    strings: &ImmutableStringPool,
) -> Vec<LocalNodeId<Expression>> {
    let mut expression_ids = Vec::new();
    let mut order_keys = Vec::new();

    // collect sortable declaration keys
    for &expr_id in imports {
        if let Some(Expression::Import { items, target, .. }) = get_import_expression(expr_id, tree)
        {
            let ImportTarget::String(target) = target else {
                continue;
            };

            let target_str = strings.get(*target);
            expression_ids.push(expr_id);
            order_keys.push(ImportDeclarationKey {
                target: target_str,
                is_side_effect: items.is_empty(),
            });
        }
    }

    // map declaration order back to expression ids
    let order = sort_import_declaration_indices(&order_keys);
    order
        .into_iter()
        .map(|index| expression_ids[index])
        .collect()
}

/// Sort dependency items by kind and configured key order.
pub fn sort_dependency_items(
    items: &[LocalNodeId<DependencyItem>],
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    sort_order: ImportSortOrder,
) -> Vec<LocalNodeId<DependencyItem>> {
    sort_items(items, tree, strings, sort_order)
}

/// Determine if a blank line should be inserted between two imports.
///
/// Returns true if:
/// - Transitioning from side-effect to regular imports
/// - Different import groups (for regular imports)
pub fn should_insert_blank_between(
    prev_expr_id: LocalNodeId<Expression>,
    curr_expr_id: LocalNodeId<Expression>,
    tree: &NodeTree,
    strings: &ImmutableStringPool,
) -> bool {
    let (prev_is_side_effect, prev_group) = match get_import_expression(prev_expr_id, tree) {
        Some(Expression::Import { items, target, .. }) => {
            let ImportTarget::String(target) = target else {
                return false;
            };
            let target_str = strings.get(*target);
            (items.is_empty(), categorize_import(target_str))
        }
        _ => return false,
    };

    let (curr_is_side_effect, curr_group) = match get_import_expression(curr_expr_id, tree) {
        Some(Expression::Import { items, target, .. }) => {
            let ImportTarget::String(target) = target else {
                return false;
            };
            let target_str = strings.get(*target);
            (items.is_empty(), categorize_import(target_str))
        }
        _ => return false,
    };

    // blank line between side effect and regular imports
    if prev_is_side_effect && !curr_is_side_effect {
        return true;
    }

    // blank line between different groups (for regular imports)
    if !prev_is_side_effect && !curr_is_side_effect && prev_group != curr_group {
        return true;
    }

    false
}
