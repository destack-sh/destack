use super::control::{
    format_break_expression, format_continue_expression, format_for_each_expression,
    format_for_expression, format_if_else_chain, format_loop_expression, format_match_expression,
    format_return_expression, format_switch_statement, format_try_expression,
    format_while_expression, format_yield_expression,
};
use super::ternary::format_ternary;
use crate::DestackFormatter;
use crate::annotation::{
    FormatLeadingComments, FormatTrailingComments, infix_or_postfix_annotations,
};
use crate::declaration::dependency::format_dependency_statement_expression;
use crate::declaration::sequence::{
    expression_is_in_statement_context, expression_is_value_block_tail,
};
use crate::declaration::{
    format_let_else_statement_expression, format_let_statement_expression,
    format_using_statement_expression,
};
use crate::tree::tree_control_child_should_expand;
use destack_dir::{Catch, Expression, IfForm, LocalNodeId, StringId, TokenType};
use destack_fir::format::{Format, FormatResult};
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

    expression_is_in_statement_context(f.context(), node_id)
}

/// Return whether one try expression should expand explicit branch blocks.
fn try_expression_should_expand<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    catch: Option<LocalNodeId<Catch>>,
    finally: Option<LocalNodeId<Expression>>,
) -> bool {
    if catch.is_some() || finally.is_some() {
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

        // import and export family
        Expression::Import { .. } | Expression::Export { .. } => {
            format_dependency_statement_expression(f, node_id, expression)?;
        }

        // let
        Expression::Let {
            kind,
            export,
            is_ambient,
            is_shared,
            declarators,
            ..
        } => {
            format_let_statement_expression(
                f,
                *kind,
                *export,
                *is_ambient,
                *is_shared,
                declarators,
            )?;
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
            label,
            form,
            condition,
            body,
        } => {
            format_loop_label(f, label.as_ref(), node_id)?;
            format_while_expression(f, *form, condition, *body)?;
        }

        // for each
        Expression::ForEach {
            label,
            asynchrony,
            binding,
            iterator,
            body,
        } => {
            format_loop_label(f, label.as_ref(), node_id)?;
            format_for_each_expression(f, node_id, *asynchrony, binding, *iterator, *body)?;
        }

        // for condition
        Expression::For {
            label,
            initialization,
            condition,
            increment,
            body,
        } => {
            format_loop_label(f, label.as_ref(), node_id)?;
            format_for_expression(f, *initialization, *condition, *increment, *body)?;
        }

        // loop
        Expression::Loop { label, body } => {
            format_loop_label(f, label.as_ref(), node_id)?;
            format_loop_expression(f, *body)?;
        }

        // try
        Expression::Try {
            body,
            catch,
            finally,
        } => {
            let expand_try_branches = try_expression_should_expand(f, node_id, *catch, *finally);
            format_try_expression(f, *body, *catch, *finally, expand_try_branches)?;
        }

        // match expression
        Expression::Match { value, arms } => {
            format_match_expression(f, node_id, *value, arms)?;
        }

        // switch statement
        Expression::Switch { value, cases } => {
            format_switch_statement(f, node_id, *value, cases)?;
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

        // return
        Expression::Return { value } => {
            format_return_expression(f, node_id, *value)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Format one loop label ahead of its loop keyword.
fn format_loop_label<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    label: Option<&StringId>,
    target: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // unlabeled loops print nothing
    let Some(label) = label else {
        return Ok(());
    };

    format_statement_label(f, label, target)?;
    write!(f, [space()])?;

    Ok(())
}

/// Format one statement label with its colon and separator comments.
fn format_statement_label<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    label: &StringId,
    target: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // collect the comments between the label colon and its target
    let context = f.context();
    let separator_comments = context
        .tree
        .get_main_span(target)
        .and_then(|label_span| context.next_token_after_span(label_span))
        .filter(|separator_token| separator_token.token.ty() == TokenType::Colon)
        .and_then(|separator_token| {
            let target_token = context.next_token_after_span(separator_token.span)?;

            Some(
                context
                    .comments()
                    .comments_in_range(separator_token.span.end, target_token.span.start)
                    .to_vec(),
            )
        })
        .unwrap_or_default();
    let has_line_comment = separator_comments.iter().any(|comment| comment.is_line());

    // keep line comments ahead of the label, trailing comments after the colon
    if has_line_comment {
        write!(f, [FormatLeadingComments::Comments(&separator_comments)])?;
    }
    write!(f, [label, token(":")])?;
    if !has_line_comment && !separator_comments.is_empty() {
        write!(f, [FormatTrailingComments::Comments(&separator_comments)])?;
    }

    Ok(())
}
