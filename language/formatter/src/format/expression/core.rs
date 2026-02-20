use crate::analysis::timing::tags;
use crate::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
    write_ignored_node,
};
use crate::expression::{format_primary_expression, format_statement_expression};
use crate::operator::format_operator_expression;
use crate::{DestackFormatter, FormatNode};
use destack_ast::{Expression, IfKind, LocalNodeId, TypeUnaryOperator};
use destack_fir::format::{Buffer, FormatError, FormatResult};
use destack_fir::write;

enum ExpressionFormatRoute {
    Statement,
    Primary,
    Operator,
}

fn expression_format_route(expression: &Expression) -> ExpressionFormatRoute {
    match expression {
        Expression::Declaration(_)
        | Expression::Block(_)
        | Expression::Statement(_)
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
        | Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::Yield { .. }
        | Expression::Throw { .. }
        | Expression::Return { .. }
        | Expression::Comptime { .. } => ExpressionFormatRoute::Statement,

        Expression::Path { .. }
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
        | Expression::TypePredicate { .. } => ExpressionFormatRoute::Primary,

        Expression::Unary { .. }
        | Expression::TypeUnary { .. }
        | Expression::TypeBinary { .. }
        | Expression::ValueOf { .. }
        | Expression::ReferenceOf { .. }
        | Expression::PointerOf { .. }
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
        | Expression::Stub
        | Expression::Error => ExpressionFormatRoute::Operator,
    }
}

/// Format an expression without prefix and postfix annotations.
pub(crate) fn format_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
    directive: Option<FormatterDirective>,
) -> FormatResult<()> {
    if let Some(directive) = directive
        && directive.kind == FormatterDirectiveKind::IgnoreFormat
    {
        write_ignored_node(f, node_id, directive)?;
        return Ok(());
    }

    match expression_format_route(expression) {
        ExpressionFormatRoute::Statement => {
            let _timing = f.context().timing_scope(tags::FORMAT_EXPRESSION_STATEMENT);
            let statement_formatted = format_statement_expression(f, node_id, expression)?;
            if statement_formatted {
                return Ok(());
            }
        }
        ExpressionFormatRoute::Primary => {
            let _timing = f.context().timing_scope(tags::FORMAT_EXPRESSION_PRIMARY);
            let primary_formatted = format_primary_expression(f, node_id, expression)?;
            if primary_formatted {
                return Ok(());
            }
        }
        ExpressionFormatRoute::Operator => {
            let _timing = f.context().timing_scope(tags::FORMAT_EXPRESSION_OPERATOR);
            let operator_formatted = format_operator_expression(f, node_id, expression)?;
            if operator_formatted {
                return Ok(());
            }
        }
    }

    Err(FormatError::SyntaxError {
        message: "unsupported expression kind for expression core formatter",
    })
}

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        node_id: LocalNodeId<Expression>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let _timing = f.context().timing_scope(tags::FORMAT_EXPRESSION);
        let directive = directive_for_node(f.context(), node_id);
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        format_expression(f, node_id, self, directive)?;

        // regular if chains emit their own edge annotations in control formatter
        let if_chain_handles_annotations = matches!(
            self,
            Expression::If {
                kind: IfKind::If,
                ..
            }
        );

        if !if_chain_handles_annotations
            && !matches!(
                directive,
                Some(FormatterDirective {
                    kind: FormatterDirectiveKind::IgnoreFormat,
                    position: FormatterDirectivePosition::Postfix { .. },
                })
            )
        {
            let call_or_new_handles_empty_infix = matches!(
                self,
                Expression::Call {
                    dynamic_arguments,
                    ..
                }
                | Expression::New {
                    dynamic_arguments,
                    ..
                } if dynamic_arguments.is_empty() && f.context().has_infix_annotation(node_id)
            );
            if matches!(
                self,
                Expression::TypeUnary {
                    operator: TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime,
                    ..
                }
            ) {
                write!(f, [f.context().any_postfix_annotations(node_id)])?;
            } else if call_or_new_handles_empty_infix {
                write!(f, [f.context().any_postfix_annotations(node_id)])?;
            } else {
                write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
            }
        }

        Ok(())
    }
}
