use super::control::{
    format_break_expression, format_continue_expression, format_for_each_expression,
    format_for_expression, format_if_else_chain, format_loop_expression, format_match,
    format_return_expression, format_statement_body_block, format_throw_expression,
    format_try_expression, format_while_expression, format_yield_expression,
    is_empty_statement_block,
};
use super::format_expression;
use super::ternary::format_ternary;
use crate::format::annotation::write_annotation_sequence_without_trailing_break;
use crate::format::declaration::dependency::format_dependency_statement_expression;
use crate::format::declaration::{
    format_let_statement_expression, format_using_statement_expression,
    statement_wrapper_needs_semicolon,
};
use crate::format::directive::node_has_ignore_directive;
use crate::{Annotation, DestackFormatContext, DestackFormatter};
use destack_ast::{
    AnnotationPosition, Doc, DocumentationStyle, Expression, FunctionKind, IfKind, LocalNodeId,
    TypeBinaryOperator, TypeUnaryOperator,
};
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

/// Decide whether a statement can drop one parenthesized expression wrapper.
pub(crate) fn statement_drops_parenthesized_expression_wrapper(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        return false;
    }

    if crate::format::expression::parenthesized_has_leading_inner_trivia(
        context,
        parenthesized_id,
        inner_expression_id,
    ) {
        return false;
    }

    if !crate::format::expression::parenthesized_boundary_comments(
        context,
        parenthesized_id,
        inner_expression_id,
    )
    .is_empty()
    {
        return false;
    }

    let inner_expression = context.tree.get(inner_expression_id);

    matches!(
        inner_expression,
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    ) || matches!(
        inner_expression,
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                destack_ast::Declaration::Function { signature, .. }
                    if signature.kind == FunctionKind::Lambda
            )
    )
}

/// Write trailing annotations for one statement expression.
pub(crate) fn write_statement_expression_trailing_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    // statement-owned trailers stay inside the statement formatter
    if statement_expression_owns_trailing_annotations(expression) {
        return Ok(());
    }

    write!(
        f,
        [crate::format::annotation::infix_or_postfix_annotations(
            f.context(),
            expression_id
        )]
    )
}

/// Return whether one expression has a multiline block postfix annotation.
fn expression_has_multiline_block_postfix_annotation(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if !ctx.has_postfix_annotation(expression_id) {
        return false;
    }

    ctx.annotation_ids(expression_id)
        .iter()
        .copied()
        .any(|annotation_id| {
            let Annotation::Doc { node, position } = ctx.annotation(annotation_id) else {
                return false;
            };
            if !matches!(
                position,
                AnnotationPosition::LinePostfix
                    | AnnotationPosition::LinePostfixBoundary
                    | AnnotationPosition::BlockPostfix
            ) {
                return false;
            }

            let doc = ctx.tree.get::<Doc>(node);
            if doc.style != DocumentationStyle::Star {
                return false;
            }

            ctx.has_newline(ctx.annotation_span(annotation_id))
        })
}

/// Return whether one statement wrapper should delay semicolon emission to after postfix docs.
fn statement_wrapper_delays_semicolon_for_multiline_as_const_postfix(
    ctx: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
    needs_semicolon: bool,
) -> bool {
    needs_semicolon
        && matches!(
            expression,
            Expression::TypeUnary {
                operator: TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime,
                ..
            }
        )
        && expression_has_multiline_block_postfix_annotation(ctx, node_id)
}

/// Return whether wrapper annotation emission should use postfix-only output.
fn statement_wrapper_uses_postfix_only_annotations(
    ctx: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> bool {
    if matches!(
        expression,
        Expression::TypeUnary {
            operator: TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime,
            ..
        }
    ) {
        return true;
    }

    let call_or_new_handles_empty_infix = matches!(
        expression,
        Expression::Call {
            dynamic_arguments,
            ..
        }
        | Expression::New {
            dynamic_arguments,
            ..
        } if dynamic_arguments.is_empty() && ctx.has_infix_annotation(node_id)
    );
    if call_or_new_handles_empty_infix {
        return true;
    }

    ctx.has_infix_annotation(node_id)
        && (matches!(
            expression,
            Expression::ObjectExpression { properties, .. } if properties.is_empty()
        ) || matches!(
            expression,
            Expression::ArrayExpression { elements } if elements.is_empty()
        ))
}

/// Format one statement wrapper inner expression with an optional trailing semicolon.
fn format_statement_wrapped_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    needs_semicolon: bool,
) -> FormatResult<()> {
    let expression = f.context().tree.get(node_id);
    let is_ignored = node_has_ignore_directive(f.context(), node_id);

    // prefix annotations and core expression
    write!(
        f,
        [crate::format::annotation::prefix_annotations(
            f.context(),
            node_id
        )]
    )?;
    format_expression(f, node_id, expression, is_ignored)?;

    let semicolon_after_multiline_as_const_postfix =
        statement_wrapper_delays_semicolon_for_multiline_as_const_postfix(
            f.context(),
            node_id,
            expression,
            needs_semicolon,
        );

    // statement terminator
    if needs_semicolon && !semicolon_after_multiline_as_const_postfix {
        write!(f, [token(";")])?;
    }

    // boundary annotations
    write!(
        f,
        [crate::format::annotation::line_postfix_boundary_annotations(f.context(), node_id)]
    )?;

    // non-boundary annotations
    let expression_handles_its_own_edge_annotations =
        statement_expression_owns_trailing_annotations(expression);
    let uses_postfix_only_annotations =
        statement_wrapper_uses_postfix_only_annotations(f.context(), node_id, expression);
    if !expression_handles_its_own_edge_annotations {
        if semicolon_after_multiline_as_const_postfix && uses_postfix_only_annotations {
            let mut items = Vec::new();
            for annotation_id in f.context().annotation_ids(node_id).iter().copied() {
                if matches!(
                    f.context().annotation(annotation_id).position(),
                    AnnotationPosition::BlockPostfix | AnnotationPosition::LinePostfix
                ) {
                    items.push(annotation_id);
                }
            }
            write_annotation_sequence_without_trailing_break(f, &items)?;
        } else if uses_postfix_only_annotations {
            write!(
                f,
                [
                    crate::format::annotation::postfix_annotations_without_line_postfix_boundary(
                        f.context(),
                        node_id
                    )
                ]
            )?;
        } else {
            write!(
                f,
                [crate::format::annotation::infix_or_postfix_annotations_without_line_postfix_boundary(
                    f.context(),
                    node_id
                )]
            )?;
        }
    }

    // delayed semicolon for multiline `as const` postfix comments
    if semicolon_after_multiline_as_const_postfix {
        write!(f, [token(";")])?;
    }

    Ok(())
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

        // statement
        Expression::Statement(node) => {
            let needs_semicolon = statement_wrapper_needs_semicolon(f.context(), *node);
            format_statement_wrapped_expression(f, *node, needs_semicolon)?;
        }

        // labelled statement
        Expression::Labelled { label, body } => {
            write!(f, [label, token(":")])?;

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
            descriptor,
            declarators,
            ..
        } => {
            format_let_statement_expression(f, *kind, descriptor, declarators)?;
        }

        // using
        Expression::Using {
            asynchrony,
            descriptor,
            declarators,
        } => {
            format_using_statement_expression(f, *asynchrony, descriptor, declarators)?;
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
            format_break_expression(f, label, value)?;
        }

        // continue
        Expression::Continue { label } => {
            format_continue_expression(f, label)?;
        }

        // yield
        Expression::Yield { cardinality, value } => {
            format_yield_expression(f, node_id, *cardinality, *value)?;
        }

        // throw
        Expression::Throw { value } => {
            format_throw_expression(f, *value)?;
        }

        // return
        Expression::Return { value } => {
            format_return_expression(f, node_id, *value)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}
