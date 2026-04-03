use destack_ast::{Declaration, Expression, FunctionKind, IfKind, LocalNodeId, WhileKind};

use crate::DestackFormatContext;

/// Return whether one block expression needs a trailing statement terminator.
pub(crate) fn expression_needs_statement_terminator(
    context: &DestackFormatContext<'_>,
    expression: &Expression,
    is_expression_context_tail: bool,
) -> bool {
    let always_needs_statement_terminator = matches!(
        expression,
        Expression::Import { .. } | Expression::Let { .. } | Expression::Using { .. }
    ) || matches!(
        expression,
        Expression::While {
            kind: WhileKind::DoWhile,
            ..
        }
    ) || matches!(
        expression,
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function {
                    descriptor,
                    signature,
                    ..
                }
                if descriptor.name.is_none() && signature.kind == FunctionKind::Lambda
            )
    );

    if always_needs_statement_terminator {
        return true;
    }

    if is_expression_context_tail {
        return false;
    }

    !matches!(expression, Expression::Stub | Expression::Error)
        && !expression.ends_statement_on_newline()
}

/// Return whether one statement wrapper should keep its trailing semicolon.
pub(crate) fn statement_wrapper_needs_semicolon(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression = context.tree.get(expression_id);
    if matches!(
        expression,
        Expression::Declaration(_) | Expression::Block(_)
    ) {
        return false;
    }

    if let Expression::Try {
        catch_expression,
        catch_pattern,
        finally_expression,
        ..
    } = expression
        && (catch_expression.is_some() || catch_pattern.is_some() || finally_expression.is_some())
    {
        return false;
    }

    if matches!(
        expression,
        Expression::If {
            kind: IfKind::If,
            ..
        } | Expression::While {
            kind: WhileKind::While,
            ..
        } | Expression::ForEach { .. }
            | Expression::For { .. }
            | Expression::Loop { .. }
            | Expression::Match { .. }
            | Expression::Labelled { .. }
    ) {
        return false;
    }

    if matches!(expression, Expression::Stub | Expression::Error) {
        return false;
    }

    true
}
