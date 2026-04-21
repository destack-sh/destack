use super::control::{
    format_break_expression, format_continue_expression, format_for_each_expression,
    format_for_expression, format_if_else_chain, format_loop_expression, format_match,
    format_return_expression, format_statement_body_block, format_throw_expression,
    format_try_expression, format_while_expression, format_yield_expression,
    is_empty_statement_block,
};
use super::ternary::format_ternary;
use crate::DestackFormatter;
use crate::format::annotation::{
    FormatLeadingComments, FormatTrailingComments, infix_or_postfix_annotations,
};
use crate::format::declaration::dependency::format_dependency_statement_expression;
use crate::format::declaration::{
    format_let_else_statement_expression, format_let_statement_expression,
    format_using_statement_expression,
};
use destack_ast::{Comment, Expression, IfKind, LocalNodeId, TokenType};
use destack_fir::format::{Buffer, Format, FormatResult};
use destack_fir::prelude::{format_with, group, space, token};
use destack_fir::write;

/// Return whether one statement expression owns its own trailing annotations.
pub(crate) fn statement_expression_owns_trailing_annotations(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::If {
            kind: IfKind::If,
            ..
        }
    )
}

/// Write trailing annotations for one statement expression.
pub(crate) fn write_statement_expression_trailing_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    // statement trailing annotations stay inside the statement formatter
    if statement_expression_owns_trailing_annotations(expression) {
        return Ok(());
    }

    write!(
        f,
        [infix_or_postfix_annotations(f.context(), expression_id)]
    )
}

/// Format statement-like expression variants.
pub(crate) fn format_statement_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    match expression {
        // declaration
        Expression::Declaration(node) => node.format(f)?,

        // block
        Expression::Block(node) => node.format(f)?,

        // labelled statement
        Expression::Labelled { label, body } => {
            let body_span = f.context().span(*body);
            let separator_comments = if let Some(separator_token) =
                f.context().previous_non_trivia_token_before_span(body_span)
            {
                if separator_token.token.ty == TokenType::Colon {
                    let comments = f.context().comments();
                    comments
                        .comments_in_range(separator_token.span.end, body_span.start)
                        .to_vec()
                } else {
                    Vec::<Comment>::new()
                }
            } else {
                Vec::<Comment>::new()
            };
            let has_line_comment = separator_comments.iter().any(|comment| comment.is_line());

            if has_line_comment {
                write!(f, [FormatLeadingComments::Comments(&separator_comments)])?;
            }

            write!(f, [label, token(":")])?;
            if !has_line_comment && !separator_comments.is_empty() {
                write!(f, [FormatTrailingComments::Comments(&separator_comments)])?;
            }

            let body_expression = f.context().tree.get(*body);
            let body_is_empty_statement = matches!(
                body_expression,
                Expression::Block(block_id) if is_empty_statement_block(f.context(), *block_id)
            );
            let body_has_prefix_annotation = f.context().has_prefix_annotation(*body);
            if !body_is_empty_statement || body_has_prefix_annotation {
                write!(f, [space()])?;
            }

            match body_expression {
                Expression::Block(block_id) => {
                    format_statement_body_block(f, *block_id)?;
                }
                _ => {
                    write!(f, [*body])?;
                }
            }
        }

        // import and export family
        Expression::Import { .. }
        | Expression::Export { .. }
        | Expression::ExportNamespace { .. } => {
            format_dependency_statement_expression(f, node_id, expression)?;
        }

        // let
        Expression::Let {
            kind,
            export,
            ambient,
            declarators,
            ..
        } => {
            format_let_statement_expression(f, *kind, *export, *ambient, declarators)?;
        }

        // let else
        Expression::LetElse {
            kind,
            declarator,
            else_branch,
            ..
        } => {
            format_let_else_statement_expression(f, *kind, *declarator, *else_branch)?;
        }

        // using
        Expression::Using {
            asynchrony,
            export,
            ambient,
            declarators,
            ..
        } => {
            format_using_statement_expression(f, *asynchrony, *export, *ambient, declarators)?;
        }

        // if (ternary)
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => {
            format_ternary(f, node_id)?;
        }

        // if (regular)
        Expression::If {
            kind: IfKind::If, ..
        } => {
            write!(
                f,
                [group(&format_with(|f| format_if_else_chain(f, node_id)))]
            )?;
        }

        // while
        Expression::While {
            kind,
            condition,
            body,
        } => {
            format_while_expression(f, *kind, *condition, *body)?;
        }

        // for each
        Expression::ForEach {
            asynchrony,
            kind,
            binding,
            iterator,
            body,
        } => {
            format_for_each_expression(f, node_id, *asynchrony, *kind, binding, *iterator, *body)?;
        }

        // for condition
        Expression::For {
            initialization,
            condition,
            increment,
            body,
        } => {
            format_for_expression(f, *initialization, *condition, *increment, *body)?;
        }

        // loop
        Expression::Loop { body } => {
            format_loop_expression(f, *body)?;
        }

        // try
        Expression::Try {
            try_expression,
            catch_pattern,
            catch_ty,
            catch_expression,
            finally_expression,
        } => {
            format_try_expression(
                f,
                *try_expression,
                *catch_pattern,
                *catch_ty,
                *catch_expression,
                *finally_expression,
            )?;
        }

        // match
        Expression::Match { .. } => {
            format_match(f, node_id, true)?;
        }

        // break
        Expression::Break { label, value } => {
            format_break_expression(f, node_id, label, value)?;
        }

        // continue
        Expression::Continue { label } => {
            format_continue_expression(f, node_id, label)?;
        }

        // yield
        Expression::Yield { cardinality, value } => {
            format_yield_expression(f, node_id, *cardinality, *value)?;
        }

        // throw
        Expression::Throw { value } => {
            format_throw_expression(f, node_id, *value)?;
        }

        // return
        Expression::Return { value } => {
            format_return_expression(f, node_id, *value)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
