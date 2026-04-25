use crate::expression::format_expression;
use crate::file::node_has_ignore_directive;
use crate::{DestackFormatter, FormatNode};
use destack_ast::{Decorator, Expression, LocalNodeId, NodeTree};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::token;
use destack_fir::write;

impl<'ast> FormatNode<'ast, Decorator> for Decorator {
    /// Format one decorator annotation.
    fn format_node(
        &self,
        _node_id: LocalNodeId<Decorator>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let tree = f.context().tree;
        let needs_parentheses = decorator_needs_parentheses(tree, self.expression);
        let expression_id = self.expression;
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
}

/// Return whether a decorator expression requires parentheses.
fn decorator_needs_parentheses(tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> bool {
    match tree.get(expression_id) {
        Expression::Identifier { .. } => false,
        Expression::QualifiedReference {
            generic_arguments, ..
        } => !generic_arguments.is_empty(),
        Expression::Call { left, .. } => !is_identifier_or_static_member_only(tree, *left),
        Expression::Member { left, .. } => !is_identifier_or_static_member_only(tree, *left),
        _ => true,
    }
}

/// Return whether an expression is an identifier or static-member-only path.
fn is_identifier_or_static_member_only(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Identifier { .. } => true,
        Expression::QualifiedReference {
            generic_arguments, ..
        } => generic_arguments.is_empty(),
        Expression::Member { left, .. } => is_identifier_or_static_member_only(tree, *left),
        _ => false,
    }
}
