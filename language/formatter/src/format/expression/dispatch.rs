use crate::format::annotation::{format_leading_comments, prefix_annotations};
use crate::format::expression::{
    expression_needs_parentheses_in_parent, format_primary_expression, format_statement_expression,
    write_primary_expression_trailing_annotations, write_statement_expression_trailing_annotations,
};
use crate::format::file::write_ignored_node;
use crate::format::operator::{
    format_operator_expression, write_operator_expression_trailing_annotations,
};
use crate::{DestackFormatter, FormatNode};
use destack_ast::{Expression, LocalNodeId};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{format_with, token};
use destack_fir::write;

/// Write one expression after prefix annotations are handled externally.
fn write_expression_without_prefix_annotations_inner<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);

    match expression {
        Expression::Declaration(_)
        | Expression::Block(_)
        | Expression::Labelled { .. }
        | Expression::Import { .. }
        | Expression::Export { .. }
        | Expression::ExportNamespace { .. }
        | Expression::Let { .. }
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
            format_statement_expression(f, expression_id, expression)?;
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
            format_primary_expression(f, expression_id, expression)?;
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
            format_operator_expression(f, expression_id, expression)?;
            write_operator_expression_trailing_annotations(f, expression_id, expression)
        }
    }
}

/// Write one expression after prefix annotations are handled externally.
pub(crate) fn write_expression_without_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write_expression_without_prefix_annotations_inner(f, expression_id)
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

    format_expression_body(f, node_id, expression)
}

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        node_id: LocalNodeId<Expression>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let needs_parentheses = expression_needs_parentheses_in_parent(f.context(), node_id);
        let expression_span = f.context().span(node_id);
        let has_prefix_annotation = f.context().has_prefix_annotation(node_id);

        // prefix annotations
        if has_prefix_annotation {
            write!(f, [prefix_annotations(f.context(), node_id)])?;
        }

        // positional leading comments
        write!(f, [format_leading_comments(expression_span)])?;

        if needs_parentheses {
            let parenthesized_body = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                write!(f, [token("(")])?;
                write_expression_without_prefix_annotations_inner(f, node_id)?;
                write!(f, [token(")")])
            });

            write!(f, [parenthesized_body])?;
            return Ok(());
        }

        // body
        write_expression_without_prefix_annotations_inner(f, node_id)?;

        Ok(())
    }
}

/// Write one expression without trailing comments.
pub(crate) fn write_expression_without_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let expression_end = f.context().span(expression_id).end;
    let previous_limit = f
        .context_mut()
        .comments_mut()
        .limit_comments_up_to(expression_end);
    let result = write!(f, [expression_id]);
    f.context_mut()
        .comments_mut()
        .restore_view_limit(previous_limit);
    result
}
