use std::cmp::Ordering;

use destack_ast::{DependencyItem, DependencyKind, Expression, LocalNodeId, NodeTree};
use destack_base::ImmutableStringPool;
use destack_workspace::ImportSortOrder;

/// Import group category for sorting.
///
/// Groups are ordered by priority (lower = earlier in file).
/// The ordering is: Builtin → Package → Alias → Relative
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportGroup {
    /// Builtin modules with protocol prefix (e.g. `node:fs`, `bun:test`, `deno:path`)
    Builtin,
    /// External packages (e.g. `lodash`, `@org/pkg`, `react`)
    Package,
    /// Path aliases (e.g. `@/utils`, `~/lib`, `#internal`)
    Alias,
    /// Relative imports (e.g. `./foo`, `../bar`, `/absolute`)
    Relative,
}

/// Categorize an import target path into a group.
pub fn categorize_import(target: &str) -> ImportGroup {
    // builtin protocols: "protocol:module" but not URLs ("protocol://...")
    if let Some(colon_pos) = target.find(':')
        && !target[colon_pos..].starts_with("://")
    {
        return ImportGroup::Builtin;
    }

    // relative imports
    if target.starts_with("./") || target.starts_with("../") {
        return ImportGroup::Relative;
    }

    // absolute path (treated as relative/local)
    if target.starts_with('/') {
        return ImportGroup::Relative;
    }

    // alias imports: @/ ~/ # (but not scoped packages like @org/pkg)
    if target.starts_with("@/") || target.starts_with("~/") || target.starts_with('#') {
        return ImportGroup::Alias;
    }

    // everything else is a package (including scoped packages @org/pkg)
    ImportGroup::Package
}

/// Get the inner import expression, unwrapping Statement if needed.
pub fn get_import_expression(
    expr_id: LocalNodeId<Expression>,
    tree: &NodeTree,
) -> Option<&Expression> {
    let expr = tree.get(expr_id);
    match expr {
        Expression::Import { .. } => Some(expr),
        Expression::Statement(inner_id) => {
            let inner = tree.get(*inner_id);
            if matches!(inner, Expression::Import { .. }) {
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
    // separate side effect imports, preserve order, from regular imports
    let mut side_effects: Vec<LocalNodeId<Expression>> = Vec::new();
    let mut regular: Vec<(ImportGroup, &str, LocalNodeId<Expression>)> = Vec::new();

    for &expr_id in imports {
        if let Some(Expression::Import { items, target, .. }) = get_import_expression(expr_id, tree)
        {
            let target_str = strings.get(*target);
            if items.is_empty() {
                // side effect import: preserve relative order
                side_effects.push(expr_id);
            } else {
                let group = categorize_import(target_str);
                regular.push((group, target_str, expr_id));
            }
        }
    }

    // sort regular imports by group, then alphabetically by target
    regular.sort_by(|a, b| match a.0.cmp(&b.0) {
        Ordering::Equal => a.1.cmp(b.1),
        other => other,
    });

    // build result: side effects first, then sorted regular imports
    let mut result: Vec<LocalNodeId<Expression>> = Vec::with_capacity(imports.len());
    result.extend(side_effects);
    result.extend(regular.into_iter().map(|(_, _, id)| id));
    result
}

/// Compare two strings using natural sort order (numbers ordered as integers).
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ac), Some(bc)) => {
                // compare numeric runs as numbers
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let mut a_number: u64 = 0;
                    while let Some(&character) = a_chars.peek()
                        && character.is_ascii_digit()
                    {
                        a_number = a_number
                            .saturating_mul(10)
                            .saturating_add((character as u64) - ('0' as u64));
                        a_chars.next();
                    }

                    let mut b_number: u64 = 0;
                    while let Some(&character) = b_chars.peek()
                        && character.is_ascii_digit()
                    {
                        b_number = b_number
                            .saturating_mul(10)
                            .saturating_add((character as u64) - ('0' as u64));
                        b_chars.next();
                    }

                    match a_number.cmp(&b_number) {
                        Ordering::Equal => continue,
                        ordering => return ordering,
                    }
                }

                // compare characters case-insensitively
                let ac_lower = ac.to_ascii_lowercase();
                let bc_lower = bc.to_ascii_lowercase();
                match ac_lower.cmp(&bc_lower) {
                    Ordering::Equal => {
                        // if only case differs, keep uppercase first
                        match ac.cmp(bc) {
                            Ordering::Equal => {
                                a_chars.next();
                                b_chars.next();
                            }
                            ordering => return ordering,
                        }
                    }
                    ordering => return ordering,
                }
            }
        }
    }
}

/// Sort dependency items by kind and configured key order.
pub fn sort_dependency_items(
    items: &[LocalNodeId<DependencyItem>],
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    sort_order: ImportSortOrder,
) -> Vec<LocalNodeId<DependencyItem>> {
    let mut sorted_items: Vec<_> = items.to_vec();

    sorted_items.sort_by(|a, b| {
        let a_item = tree.get(*a);
        let b_item = tree.get(*b);

        // type imports come before value imports
        let a_is_type = a_item.kind == Some(DependencyKind::Type);
        let b_is_type = b_item.kind == Some(DependencyKind::Type);
        match (a_is_type, b_is_type) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }

        // sort by alias first when present, otherwise by item name
        let a_key = a_item
            .alias
            .or(a_item.name.map(|name| name.string()))
            .map(|string_id| strings.get(string_id))
            .unwrap_or("");
        let b_key = b_item
            .alias
            .or(b_item.name.map(|name| name.string()))
            .map(|string_id| strings.get(string_id))
            .unwrap_or("");

        match sort_order {
            ImportSortOrder::Natural => natural_cmp(a_key, b_key),
            ImportSortOrder::Alphabetical => a_key.cmp(b_key),
        }
    });

    sorted_items
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
            let target_str = strings.get(*target);
            (items.is_empty(), categorize_import(target_str))
        }
        _ => return false,
    };

    let (curr_is_side_effect, curr_group) = match get_import_expression(curr_expr_id, tree) {
        Some(Expression::Import { items, target, .. }) => {
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
