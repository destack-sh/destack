use destack_dir as dir;

/// Resolve the target symbol for a reference expression.
pub fn expression_target_symbol(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalSymbolId> {
    // unwrap parenthesized expressions first
    let expression = tree.get(expression_id);
    if let dir::Expression::Parenthesized { expression } = expression {
        return expression_target_symbol(tree, *expression);
    }

    // return the reference target symbol when present
    expression.target_symbol()
}
