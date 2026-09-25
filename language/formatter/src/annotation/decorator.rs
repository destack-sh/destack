use crate::expression::format_expression;
use crate::file::node_has_ignore_directive;
use crate::{FormatNode, TsppFormatter};
use tspp_dir::{Decorator, Expression, LocalNodeId, Tree};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::token;
use tspp_fir::write;

impl<'ast> FormatNode<'ast, Decorator> for Decorator {
    /// Format one decorator annotation.
    fn format_node(
        &self,
        _node_id: LocalNodeId<Decorator>,
        f: &mut TsppFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write_decorator_expression(f, self.expression)
    }
}

/// Write one decorator by node id.
pub(crate) fn write_decorator<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    decorator_id: LocalNodeId<Decorator>,
) -> FormatResult<()> {
    let expression_id = f.context().tree.get(decorator_id).expression;

    write_decorator_expression(f, expression_id)
}

/// Write one decorator expression.
fn write_decorator_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let needs_parentheses = decorator_needs_parentheses(tree, expression_id);
    let expression = tree.get(expression_id);
    let is_ignored = node_has_ignore_directive(f.context(), expression_id);

    write!(f, [token("@")])?;
    if needs_parentheses {
        write!(f, [token("(")])?;
    }

    format_expression(f, expression_id, expression, is_ignored)?;

    if needs_parentheses {
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Return whether a decorator expression requires parentheses.
fn decorator_needs_parentheses(tree: &Tree, expression_id: LocalNodeId<Expression>) -> bool {
    match tree.get(expression_id) {
        Expression::Identifier { .. } => false,
        Expression::Call { left, .. } => !is_identifier_or_static_member_only(tree, *left),
        Expression::Member { left, .. } => !is_identifier_or_static_member_only(tree, *left),
        _ => true,
    }
}

/// Return whether an expression is an identifier or static-member-only path.
fn is_identifier_or_static_member_only(
    tree: &Tree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Identifier { .. } => true,
        Expression::Member { left, .. } => is_identifier_or_static_member_only(tree, *left),
        _ => false,
    }
}
