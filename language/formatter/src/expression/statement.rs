use super::control::{
    format_break_expression, format_continue_expression, format_for_each_expression,
    format_for_expression, format_if_else_chain, format_loop_expression, format_match,
    format_return_expression, format_statement_body_block, format_throw_expression,
    format_try_expression, format_while_expression, format_yield_expression,
    is_empty_statement_block,
};
use super::ternary::format_ternary;
use crate::DestackFormatter;
use crate::annotation::{
    FormatLeadingComments, FormatTrailingComments, infix_or_postfix_annotations,
};
use crate::declaration::dependency::format_dependency_statement_expression;
use crate::declaration::sequence::{
    expression_is_in_statement_position, expression_is_value_block_tail,
};
use crate::declaration::{
    format_let_else_statement_expression, format_let_statement_expression,
    format_using_statement_expression, statement_wrapper_needs_semicolon,
    write_statement_terminator,
};
use crate::tree::tree_control_child_should_expand;
use destack_ast::{Comment, Expression, IfForm, LocalNodeId, TokenType};
use destack_fir::format::{Buffer, Format, FormatResult};
use destack_fir::prelude::{format_with, group, space, token};
use destack_fir::write;

/// Return whether one statement expression owns its own trailing annotations.
pub(crate) fn statement_expression_owns_trailing_annotations(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::If {
            form: IfForm::If,
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

/// Return whether one value-capable control expression should expand explicit branch blocks.
fn value_branch_expression_should_expand<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if expression_is_value_block_tail(f.context(), node_id) {
        return true;
    }

    if tree_control_child_should_expand(f.context(), node_id) {
        return true;
    }

    f.context().options.language_type.is_destack()
        && expression_is_in_statement_position(f.context(), node_id)
}

/// Return whether one try expression should expand explicit branch blocks.
fn try_expression_should_expand<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    catch_expression: Option<LocalNodeId<Expression>>,
    finally_expression: Option<LocalNodeId<Expression>>,
) -> bool {
    if catch_expression.is_some() || finally_expression.is_some() {
        return true;
    }

    value_branch_expression_should_expand(f, node_id)
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
                    if statement_wrapper_needs_semicolon(f.context(), *body) {
                        write_statement_terminator(f, *body)?;
                    }
                }
            }
        }

        // import and export family
        Expression::Import { .. } | Expression::Export { .. } => {
            format_dependency_statement_expression(f, node_id, expression)?;
        }

        // let
        Expression::Let {
            kind,
            export,
            is_ambient,
            declarators,
            ..
        } => {
            format_let_statement_expression(f, *kind, *export, *is_ambient, declarators)?;
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
            is_ambient,
            declarators,
            ..
        } => {
            format_using_statement_expression(f, *asynchrony, *export, *is_ambient, declarators)?;
        }

        // if (ternary)
        Expression::If {
            form: IfForm::Ternary,
            ..
        } => {
            format_ternary(f, node_id)?;
        }

        // if (regular)
        Expression::If {
            form: IfForm::If, ..
        } => {
            let expand_branches = value_branch_expression_should_expand(f, node_id);
            let if_chain = format_with(|f| format_if_else_chain(f, node_id, expand_branches));
            if expand_branches {
                write!(f, [group(&if_chain).should_expand(true)])?;
                return Ok(true);
            }

            write!(f, [group(&if_chain)])?;
        }

        // while
        Expression::While {
            form,
            condition,
            body,
        } => {
            format_while_expression(f, *form, *condition, *body)?;
        }

        // for each
        Expression::ForEach {
            asynchrony,
            operator,
            binding,
            iterator,
            body,
        } => {
            format_for_each_expression(
                f,
                node_id,
                *asynchrony,
                *operator,
                binding,
                *iterator,
                *body,
            )?;
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
            let expand_try_branches =
                try_expression_should_expand(f, node_id, *catch_expression, *finally_expression);
            format_try_expression(
                f,
                *try_expression,
                *catch_pattern,
                *catch_ty,
                *catch_expression,
                *finally_expression,
                expand_try_branches,
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
