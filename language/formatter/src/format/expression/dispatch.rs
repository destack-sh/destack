use crate::format::directive::{node_has_ignore_directive, write_ignored_node};
use crate::format::expression::{
    format_primary_expression, format_statement_expression,
    write_primary_expression_trailing_annotations, write_statement_expression_trailing_annotations,
};
use crate::format::operator::{
    format_operator_expression, operator_expression_owns_prefix_annotations,
    write_operator_expression_trailing_annotations,
};
use crate::{DestackFormatter, FormatNode};
use destack_ast::{Expression, LocalNodeId};
use destack_fir::format::{Buffer, FormatError, FormatResult};
use destack_fir::write;

/// Write one expression after prefix ownership is external.
pub(crate) fn write_expression_without_prefix_annotations<'ast>(
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
        | Expression::This
        | Expression::Super
        | Expression::ScalarLiteral(_)
        | Expression::TypeLiteral(_)
        | Expression::TemplateExpression { .. }
        | Expression::TaggedTemplateExpression { .. }
        | Expression::ArrayExpression { .. }
        | Expression::TupleExpression { .. }
        | Expression::SequenceExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::TreeExpression { .. }
        | Expression::Parenthesized { .. }
        | Expression::TypeConditional { .. }
        | Expression::TypeMapped { .. }
        | Expression::TypeIndex { .. }
        | Expression::TypeTemplateLiteral { .. }
        | Expression::TypeImport { .. }
        | Expression::TypeInfer { .. }
        | Expression::TypePredicate { .. } => {
            format_primary_expression(f, expression_id, expression)?;
            write_primary_expression_trailing_annotations(f, expression_id, expression)
        }
        Expression::Unary { .. }
        | Expression::TypeUnary { .. }
        | Expression::TypeBinary { .. }
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

    let formatted = match expression {
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
        | Expression::Return { .. } => format_statement_expression(f, node_id, expression)?,
        Expression::Identifier { .. }
        | Expression::QualifiedReference { .. }
        | Expression::PrivateIdentifier { .. }
        | Expression::This
        | Expression::Super
        | Expression::ScalarLiteral(_)
        | Expression::TypeLiteral(_)
        | Expression::TemplateExpression { .. }
        | Expression::TaggedTemplateExpression { .. }
        | Expression::ArrayExpression { .. }
        | Expression::TupleExpression { .. }
        | Expression::SequenceExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::TreeExpression { .. }
        | Expression::Parenthesized { .. }
        | Expression::TypeConditional { .. }
        | Expression::TypeMapped { .. }
        | Expression::TypeIndex { .. }
        | Expression::TypeTemplateLiteral { .. }
        | Expression::TypeImport { .. }
        | Expression::TypeInfer { .. }
        | Expression::TypePredicate { .. } => format_primary_expression(f, node_id, expression)?,
        Expression::Unary { .. }
        | Expression::TypeUnary { .. }
        | Expression::TypeBinary { .. }
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
        | Expression::Error => format_operator_expression(f, node_id, expression)?,
    };

    if formatted {
        return Ok(());
    }

    Err(FormatError::SyntaxError {
        message: "unsupported expression kind for expression formatter",
    })
}

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        node_id: LocalNodeId<Expression>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let is_ignored = node_has_ignore_directive(f.context(), node_id);
        let type_cast_node_owns_prefix = matches!(self, Expression::Parenthesized { .. })
            && crate::format::expression::is_type_cast_comment_node(f.context(), node_id);
        if !operator_expression_owns_prefix_annotations(f.context(), node_id, self, is_ignored)
            && !type_cast_node_owns_prefix
        {
            write!(
                f,
                [crate::format::annotation::prefix_annotations(
                    f.context(),
                    node_id
                )]
            )?;
        }

        write_expression_without_prefix_annotations(f, node_id)?;

        Ok(())
    }
}

/// Write one expression body without formatter-owned trailing annotations.
pub(crate) fn write_expression_without_trailing_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let is_ignored = node_has_ignore_directive(f.context(), expression_id);
    let expression = f.context().tree.get(expression_id);

    format_expression(f, expression_id, expression, is_ignored)
}
