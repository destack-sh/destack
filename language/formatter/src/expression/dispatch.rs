use super::parentheses::{should_preserve_source_parentheses, source_parentheses_span};
use super::ternary::expression_is_ternary_branch;
use crate::annotation::{format_leading_comments, format_trailing_comments, prefix_annotations};
use crate::context::FormatNodeWithoutTrailingComments;
use crate::expression::{
    expression_needs_parentheses_in_parent, expression_requires_parentheses_in_parent,
    format_primary_expression, format_statement_expression,
    write_primary_expression_trailing_annotations, write_statement_expression_trailing_annotations,
};
use crate::file::{node_has_ignore_directive, write_ignored_node};
use crate::operator::{format_operator_expression, write_operator_expression_trailing_annotations};
use crate::{FormatNode, TsppFormatContext, TsppFormatter};
use tspp_core::ensure_sufficient_stack;
use tspp_dir::{Expression, LocalNodeId};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::{format_with, token};
use tspp_fir::write;
use tspp_source::{NodeSpanRegion, NodeSpanType, Span};

/// Return the enclosing and preceding spans for trailing expression comments.
fn expression_trailing_comment_spans(
    context: &TsppFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> (Span, Span) {
    let expression_span = context.span(expression_id);
    let parent = context.parent(expression_id);
    let parent_span = parent.map(|(parent_id, _)| context.span_by_id(parent_id));
    let Some(parentheses_span) = source_parentheses_span(context, expression_id) else {
        return (parent_span.unwrap_or(expression_span), expression_span);
    };

    // comments before the close token belong inside the parentheses
    let interior_comments = context
        .comments()
        .comments_in_range(expression_span.end, parentheses_span.end);
    if !interior_comments.is_empty() {
        return (parentheses_span, expression_span);
    }

    // comments after the close token follow the complete wrapped expression
    let parent_parentheses_span = parent.and_then(|(parent_id, _)| {
        context
            .tree
            .get_side_span_by_id(parent_id, NodeSpanType::Region(NodeSpanRegion::Parentheses))
    });
    let enclosing_span = parent_parentheses_span
        .or(parent_span)
        .unwrap_or(parentheses_span);

    (enclosing_span, parentheses_span)
}

/// Write one expression after prefix annotations are handled externally.
fn write_expression_without_prefix_annotations_inner<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);

    format_expression_body(f, expression_id, expression)
}

/// Write trailing annotations for one expression.
fn write_expression_trailing_annotations<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    match expression {
        Expression::Declaration(_)
        | Expression::Block(_)
        | Expression::Import { .. }
        | Expression::Export { .. }
        | Expression::Let { .. }
        | Expression::LetElse { .. }
        | Expression::Using { .. }
        | Expression::If { .. }
        | Expression::While { .. }
        | Expression::ForEach { .. }
        | Expression::For { .. }
        | Expression::Loop { .. }
        | Expression::Try { .. }
        | Expression::Match { .. }
        | Expression::Switch { .. }
        | Expression::Break { .. }
        | Expression::Continue { .. }
        | Expression::Yield { .. }
        | Expression::Return { .. } => {
            write_statement_expression_trailing_annotations(f, expression_id, expression)
        }
        Expression::Identifier { .. }
        | Expression::ImportMeta
        | Expression::ImportSource
        | Expression::This
        | Expression::Super
        | Expression::Infer { .. }
        | Expression::Type { .. }
        | Expression::Literal(_)
        | Expression::TemplateExpression { .. }
        | Expression::TaggedTemplateExpression { .. }
        | Expression::ArrayExpression { .. }
        | Expression::FixedArrayExpression { .. }
        | Expression::TupleExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::StructExpression { .. }
        | Expression::TreeExpression { .. } => {
            write_primary_expression_trailing_annotations(f, expression_id, expression)
        }
        Expression::Unary { .. }
        | Expression::As { .. }
        | Expression::Satisfies { .. }
        | Expression::Is { .. }
        | Expression::InstanceOf { .. }
        | Expression::BorrowOf { .. }
        | Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::AwaitMust { .. }
        | Expression::Const { .. }
        | Expression::Member { .. }
        | Expression::Index { .. }
        | Expression::Instantiation { .. }
        | Expression::Call { .. }
        | Expression::New { .. }
        | Expression::Chain { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. }
        | Expression::RangeExpression { .. }
        | Expression::Binary { .. }
        | Expression::Assign { .. }
        | Expression::Debugger
        | Expression::Missing
        | Expression::Error => {
            write_operator_expression_trailing_annotations(f, expression_id, expression)
        }
    }
}

/// Write one expression's prefix annotations.
fn write_expression_prefix_annotations<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let has_prefix_annotation = f.context().has_prefix_annotation(node_id);

    // prefix annotations
    if has_prefix_annotation {
        write!(f, [prefix_annotations(f.context(), node_id)])?;
    }

    Ok(())
}

/// Write one expression's prefix annotations and positional leading comments.
fn write_expression_prefix_annotations_and_leading_comments<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let expression_span = f.context().span(node_id);
    let token_start = f.context().expression_token_start(node_id);
    let token_start_span = Span::new(expression_span.file, token_start, token_start);

    write_expression_prefix_annotations(f, node_id)?;

    // positional leading comments
    write!(f, [format_leading_comments(token_start_span)])?;

    Ok(())
}

/// Write one expression without derived parentheses.
pub(crate) fn write_expression_without_derived_parentheses<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write_expression_prefix_annotations_and_leading_comments(f, node_id)?;

    // body
    write_expression_without_prefix_annotations_inner(f, node_id)
}

/// Write trailing comments and annotations for one expression node.
fn write_expression_trailing_node_annotations<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);
    let (enclosing_span, preceding_span) =
        expression_trailing_comment_spans(f.context(), expression_id);
    let following_span_start = f.context().following_span_start();

    let is_tree_literal_ternary_branch = matches!(expression, Expression::TreeExpression { .. })
        && expression_is_ternary_branch(f.context(), expression_id);

    if !is_tree_literal_ternary_branch {
        write!(
            f,
            [format_trailing_comments(
                enclosing_span,
                preceding_span,
                following_span_start
            )]
        )?;
    }

    // trailing annotations
    write_expression_trailing_annotations(f, expression_id, expression)
}

/// Write one expression after prefix annotations are handled externally.
pub(crate) fn write_expression_without_prefix_annotations<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write_expression_body_and_trailing_annotations(f, expression_id)
}

/// Write one expression body and its trailing annotations with required parentheses.
fn write_expression_body_and_trailing_annotations<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let preserves_source_parentheses =
        should_preserve_source_parentheses(f.context(), expression_id);
    let needs_derived_parentheses =
        expression_requires_parentheses_in_parent(f.context(), expression_id);
    let needs_parentheses = preserves_source_parentheses || needs_derived_parentheses;

    // retain otherwise redundant parentheses that own trailing comments
    if preserves_source_parentheses && !needs_derived_parentheses {
        write!(f, [token("(")])?;
        write_expression_without_prefix_annotations_inner(f, expression_id)?;
        write_expression_trailing_node_annotations(f, expression_id)?;

        return write!(f, [token(")")]);
    }

    // close required parentheses before trailing comments
    if needs_parentheses {
        write!(f, [token("(")])?;
        write_expression_without_prefix_annotations_inner(f, expression_id)?;
        write!(f, [token(")")])?;
    } else {
        write_expression_without_prefix_annotations_inner(f, expression_id)?;
    }

    write_expression_trailing_node_annotations(f, expression_id)
}

/// Format one expression body without leading comments, prefix annotations, or trailing annotations.
fn format_expression_body<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    ensure_sufficient_stack(|| format_expression_body_inner(f, node_id, expression))
}

/// Format one expression body.
fn format_expression_body_inner<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    match expression {
        Expression::Declaration(_)
        | Expression::Block(_)
        | Expression::Import { .. }
        | Expression::Export { .. }
        | Expression::Let { .. }
        | Expression::LetElse { .. }
        | Expression::Using { .. }
        | Expression::If { .. }
        | Expression::While { .. }
        | Expression::ForEach { .. }
        | Expression::For { .. }
        | Expression::Loop { .. }
        | Expression::Try { .. }
        | Expression::Match { .. }
        | Expression::Switch { .. }
        | Expression::Break { .. }
        | Expression::Continue { .. }
        | Expression::Yield { .. }
        | Expression::Return { .. } => {
            let is_formatted = format_statement_expression(f, node_id, expression)?;
            debug_assert!(is_formatted);
        }
        Expression::Identifier { .. }
        | Expression::ImportMeta
        | Expression::ImportSource
        | Expression::This
        | Expression::Super
        | Expression::Infer { .. }
        | Expression::Type { .. }
        | Expression::Literal(_)
        | Expression::TemplateExpression { .. }
        | Expression::TaggedTemplateExpression { .. }
        | Expression::ArrayExpression { .. }
        | Expression::FixedArrayExpression { .. }
        | Expression::TupleExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::StructExpression { .. }
        | Expression::TreeExpression { .. } => {
            let is_formatted = format_primary_expression(f, node_id, expression)?;
            debug_assert!(is_formatted);
        }
        Expression::Unary { .. }
        | Expression::As { .. }
        | Expression::Satisfies { .. }
        | Expression::Is { .. }
        | Expression::InstanceOf { .. }
        | Expression::BorrowOf { .. }
        | Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::AwaitMust { .. }
        | Expression::Const { .. }
        | Expression::Member { .. }
        | Expression::Index { .. }
        | Expression::Instantiation { .. }
        | Expression::Call { .. }
        | Expression::New { .. }
        | Expression::Chain { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. }
        | Expression::RangeExpression { .. }
        | Expression::Binary { .. }
        | Expression::Assign { .. }
        | Expression::Debugger
        | Expression::Missing
        | Expression::Error => {
            let is_formatted = format_operator_expression(f, node_id, expression)?;
            debug_assert!(is_formatted);
        }
    }

    Ok(())
}

/// Format an expression without prefix and postfix annotations.
pub(crate) fn format_expression<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
    is_ignored: bool,
) -> FormatResult<()> {
    if is_ignored {
        write_ignored_node(f, node_id)?;
        return Ok(());
    }

    // derived parentheses
    if expression_needs_parentheses_in_parent(f.context(), node_id) {
        let parenthesized_body = format_with(|f: &mut TsppFormatter<'ast, '_>| {
            write!(f, [token("(")])?;
            format_expression_body(f, node_id, expression)?;
            write!(f, [token(")")])
        });

        return write!(f, [parenthesized_body]);
    }

    format_expression_body(f, node_id, expression)
}

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        node_id: LocalNodeId<Expression>,
        f: &mut TsppFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // tree expressions own leading comments
        if matches!(self, Expression::TreeExpression { .. }) {
            write_expression_prefix_annotations(f, node_id)?;
        }
        // default leading comments
        else {
            write_expression_prefix_annotations_and_leading_comments(f, node_id)?;
        }

        if node_has_ignore_directive(f.context(), node_id) {
            write_ignored_node(f, node_id)?;
            return write_expression_trailing_node_annotations(f, node_id);
        }

        write_expression_body_and_trailing_annotations(f, node_id)
    }
}

/// Write one expression without trailing comments.
pub(crate) fn write_expression_without_trailing_comments<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if node_has_ignore_directive(f.context(), expression_id) {
        write_ignored_node(f, expression_id)?;
        return Ok(());
    }

    write!(f, [FormatNodeWithoutTrailingComments(expression_id)])
}
