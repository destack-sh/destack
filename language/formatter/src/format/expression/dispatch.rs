use crate::format::annotation::{
    format_leading_comments, format_trailing_comments, prefix_annotations,
};
use crate::format::context::FormatNodeWithoutTrailingComments;
use crate::format::expression::{
    expression_needs_parentheses_in_parent, format_primary_expression, format_statement_expression,
    write_primary_expression_trailing_annotations, write_statement_expression_trailing_annotations,
};
use crate::format::file::{node_has_ignore_directive, write_ignored_node};
use crate::format::operator::{
    format_operator_expression, write_operator_expression_trailing_annotations,
};
use crate::format::tree::tree_literal_uses_conditional_trailing_comments;
use crate::{DestackFormatter, FormatNode};
use destack_ast::{Expression, LocalNodeId};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{format_with, token};
use destack_fir::write;
use destack_source::Span;

/// Write one expression after prefix annotations are handled externally.
fn write_expression_without_prefix_annotations_inner<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);

    format_expression_body(f, expression_id, expression)
}

/// Write trailing annotations for one expression.
fn write_expression_trailing_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    match expression {
        Expression::Declaration(_)
        | Expression::Block(_)
        | Expression::Labelled { .. }
        | Expression::Import { .. }
        | Expression::Export { .. }
        | Expression::ExportNamespace { .. }
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
        | Expression::Break { .. }
        | Expression::Continue { .. }
        | Expression::Yield { .. }
        | Expression::Throw { .. }
        | Expression::Return { .. } => {
            write_statement_expression_trailing_annotations(f, expression_id, expression)
        }
        Expression::Identifier { .. }
        | Expression::QualifiedReference { .. }
        | Expression::PrivateIdentifier { .. }
        | Expression::ImportMeta
        | Expression::NewTarget
        | Expression::This
        | Expression::Super
        | Expression::Type { .. }
        | Expression::ScalarLiteral(_)
        | Expression::TemplateExpression { .. }
        | Expression::TaggedTemplateExpression { .. }
        | Expression::ArrayExpression { .. }
        | Expression::TupleExpression { .. }
        | Expression::SequenceExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::TreeExpression { .. }
        | Expression::Parenthesized { .. } => {
            write_primary_expression_trailing_annotations(f, expression_id, expression)
        }
        Expression::Unary { .. }
        | Expression::As { .. }
        | Expression::Satisfies { .. }
        | Expression::Is { .. }
        | Expression::InstanceOf { .. }
        | Expression::ValueOf { .. }
        | Expression::ReferenceOf { .. }
        | Expression::PointerOf { .. }
        | Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::Comptime { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Instantiation { .. }
        | Expression::Call { .. }
        | Expression::New { .. }
        | Expression::Delete { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. }
        | Expression::Binary { .. }
        | Expression::Assign { .. }
        | Expression::Debugger
        | Expression::Missing
        | Expression::Stub
        | Expression::Error => {
            write_operator_expression_trailing_annotations(f, expression_id, expression)
        }
    }
}

/// Write one expression's prefix annotations.
fn write_expression_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
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
    f: &mut DestackFormatter<'ast, '_>,
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
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write_expression_prefix_annotations_and_leading_comments(f, node_id)?;

    // body
    write_expression_without_prefix_annotations_inner(f, node_id)
}

/// Write trailing comments and annotations for one expression node.
fn write_expression_trailing_node_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);
    let expression_span = f.context().span(expression_id);
    let enclosing_span = f
        .context()
        .parent(expression_id)
        .map(|(parent_id, _)| f.context().span_by_id(parent_id))
        .unwrap_or(expression_span);
    let following_span_start = f.context().following_span_start();

    let has_tree_literal_owned_trailing_comments =
        matches!(expression, Expression::TreeExpression { .. })
            && tree_literal_uses_conditional_trailing_comments(f.context(), expression_id);

    if !has_tree_literal_owned_trailing_comments {
        write!(
            f,
            [format_trailing_comments(
                enclosing_span,
                expression_span,
                following_span_start
            )]
        )?;
    }

    // trailing annotations
    write_expression_trailing_annotations(f, expression_id, expression)
}

/// Write one expression after prefix annotations are handled externally.
pub(crate) fn write_expression_without_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write_expression_without_prefix_annotations_inner(f, expression_id)?;
    write_expression_trailing_node_annotations(f, expression_id)
}

/// Format one expression body without leading comments, prefix annotations, or trailing annotations.
fn format_expression_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    match expression {
        Expression::Declaration(_)
        | Expression::Block(_)
        | Expression::Labelled { .. }
        | Expression::Import { .. }
        | Expression::Export { .. }
        | Expression::ExportNamespace { .. }
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
        | Expression::Break { .. }
        | Expression::Continue { .. }
        | Expression::Yield { .. }
        | Expression::Throw { .. }
        | Expression::Return { .. } => {
            let is_formatted = format_statement_expression(f, node_id, expression)?;
            debug_assert!(is_formatted);
        }
        Expression::Identifier { .. }
        | Expression::QualifiedReference { .. }
        | Expression::PrivateIdentifier { .. }
        | Expression::ImportMeta
        | Expression::NewTarget
        | Expression::This
        | Expression::Super
        | Expression::Type { .. }
        | Expression::ScalarLiteral(_)
        | Expression::TemplateExpression { .. }
        | Expression::TaggedTemplateExpression { .. }
        | Expression::ArrayExpression { .. }
        | Expression::TupleExpression { .. }
        | Expression::SequenceExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::TreeExpression { .. }
        | Expression::Parenthesized { .. } => {
            let is_formatted = format_primary_expression(f, node_id, expression)?;
            debug_assert!(is_formatted);
        }
        Expression::Unary { .. }
        | Expression::As { .. }
        | Expression::Satisfies { .. }
        | Expression::Is { .. }
        | Expression::InstanceOf { .. }
        | Expression::ValueOf { .. }
        | Expression::ReferenceOf { .. }
        | Expression::PointerOf { .. }
        | Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::Comptime { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Instantiation { .. }
        | Expression::Call { .. }
        | Expression::New { .. }
        | Expression::Delete { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. }
        | Expression::Binary { .. }
        | Expression::Assign { .. }
        | Expression::Debugger
        | Expression::Missing
        | Expression::Stub
        | Expression::Error => {
            let is_formatted = format_operator_expression(f, node_id, expression)?;
            debug_assert!(is_formatted);
        }
    }

    Ok(())
}

/// Format an expression without prefix and postfix annotations.
pub(crate) fn format_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
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
        let parenthesized_body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
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
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // derived parentheses
        let needs_parentheses = expression_needs_parentheses_in_parent(f.context(), node_id);

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

        if needs_parentheses {
            let parenthesized_body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(f, [token("(")])?;
                write_expression_without_prefix_annotations_inner(f, node_id)?;
                write!(f, [token(")")])
            });

            write!(f, [parenthesized_body])?;
        } else {
            write_expression_without_prefix_annotations_inner(f, node_id)?;
        }

        // trailing node annotations
        write_expression_trailing_node_annotations(f, node_id)
    }
}

/// Write one expression without trailing comments.
pub(crate) fn write_expression_without_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if node_has_ignore_directive(f.context(), expression_id) {
        write_ignored_node(f, expression_id)?;
        return Ok(());
    }

    write!(f, [FormatNodeWithoutTrailingComments(expression_id)])
}
