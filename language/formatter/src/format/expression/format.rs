use crate::format::analysis::{
    first_non_trivia_token_in_span, last_non_trivia_token_in_span, timing,
};
use crate::format::chain::{should_expand_static_argument_list, static_argument_list_is_hug_safe};
use crate::format::collection::list_like;
use crate::format::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition,
    comment_node_is_ignore_directive, directive_for_node, write_ignored_node,
};
use crate::format::expression::{
    format_primary_expression, format_statement_expression, parenthesized_has_leading_inner_trivia,
    transparent_inner_expression,
};
use crate::format::operator::{format_operator_expression, union_owns_prefix_annotations};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Argument, BinaryOperator, Declarator, Expression, IfCondition, IfKind,
    LocalNodeId, NodeTree, NodeType, Pattern, Property, ScalarLiteral, TokenType,
    TypeBinaryOperator, TypeUnaryOperator, UnaryOperator,
};
use destack_fir::format::{Buffer, FormatError, FormatResult, space, token};
use destack_fir::write;
use destack_source::Span;

/// Return whether one expression node is the value slot of a declarator.
fn expression_is_declarator_value(
    f: &DestackFormatter<'_, '_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = f.context().parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declarator {
        return false;
    }

    let declarator_id = LocalNodeId::<Declarator>::new(parent_id);
    f.context()
        .tree
        .get(declarator_id)
        .value
        .is_some_and(|value_id| value_id.id == node_id.id)
}

/// Return whether one expression carries only postfix blank annotations.
fn expression_has_only_postfix_blank_annotations(
    f: &DestackFormatter<'_, '_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotation_ids) = f.context().annotations(node_id) else {
        return false;
    };

    let mut has_postfix_blank = false;
    for annotation_id in annotation_ids {
        let annotation = f.context().annotation(annotation_id);
        let is_postfix_position = matches!(
            annotation.position(),
            AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
                | AnnotationPosition::BlockPostfix
        );
        if !is_postfix_position {
            return false;
        }

        if !matches!(annotation, Annotation::Blank { .. }) {
            return false;
        }

        has_postfix_blank = true;
    }

    has_postfix_blank
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

    let formatted = match expression {
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
        | Expression::Comptime { .. } => {
            let _timing = f
                .context()
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT);
            format_statement_expression(f, node_id, expression)?
        }
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
        | Expression::TypePredicate { .. } => {
            let _timing = f.context().timing_scope(timing::FORMAT_EXPRESSION_PRIMARY);
            format_primary_expression(f, node_id, expression)?
        }
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
        | Expression::Error => {
            let _timing = f.context().timing_scope(timing::FORMAT_EXPRESSION_OPERATOR);
            format_operator_expression(f, node_id, expression)?
        }
    };

    if formatted {
        return Ok(());
    }

    Err(FormatError::SyntaxError {
        message: "unsupported expression kind for expression formatter",
    })
}

/// Format one expression while omitting prefix annotation emission.
pub(crate) fn format_expression_without_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let directive = directive_for_node(f.context(), expression_id);
    let expression = f.context().tree.get(expression_id);

    format_expression(f, expression_id, expression, directive)?;

    if !matches!(
        directive,
        Some(FormatterDirective {
            kind: FormatterDirectiveKind::IgnoreFormat,
            position: FormatterDirectivePosition::Postfix { .. },
        })
    ) {
        write!(
            f,
            [f.context().any_infix_or_postfix_annotations(expression_id)]
        )?;
    }

    Ok(())
}

/// Format static type arguments without multiline trailing commas.
pub(crate) fn format_static_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    if static_argument_list_is_hug_safe(f.context(), static_arguments) {
        write!(f, [token("<")])?;
        for (index, argument_id) in static_arguments.iter().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }
            write!(f, [*argument_id])?;
        }
        write!(f, [token(">")])?;
        return Ok(());
    }

    if static_arguments.len() == 1 {
        let argument_id = static_arguments[0];
        let value_id = match f.context().tree.get(argument_id) {
            Argument::Named { value, .. }
            | Argument::Labeled { value, .. }
            | Argument::Positional { value, .. }
            | Argument::Spread { value, .. } => *value,
        };
        let value_id = transparent_inner_expression(f.context(), value_id);
        let value_is_union_or_intersection = matches!(
            f.context().tree.get(value_id),
            Expression::Binary {
                operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
                ..
            }
        );

        if value_is_union_or_intersection && !f.context().has_annotation(argument_id) {
            write!(f, [token("<"), argument_id, token(">")])?;
            return Ok(());
        }
    }

    let should_expand = should_expand_static_argument_list(f.context(), static_arguments);
    let mut list = list_like("<", ">", ",", static_arguments);
    list.disallow_trailing_separator();
    if should_expand {
        write!(f, [list.as_collection().should_expand(true)])
    } else {
        write!(f, [list])
    }
}

/// Format static type arguments with relational spacing for index-following instantiations.
pub(crate) fn format_static_argument_list_with_relational_spacing<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    write!(f, [space(), token("<"), space()])?;
    for (index, argument_id) in static_arguments.iter().enumerate() {
        if index > 0 {
            write!(f, [token(","), space()])?;
        }
        write!(f, [*argument_id])?;
    }
    write!(f, [space(), token(">"), space()])
}

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        node_id: LocalNodeId<Expression>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let _timing = f.context().timing_scope(timing::FORMAT_EXPRESSION);
        let directive = directive_for_node(f.context(), node_id);
        let directive_is_prefix_ignore = matches!(
            directive,
            Some(FormatterDirective {
                kind: FormatterDirectiveKind::IgnoreFormat,
                position: FormatterDirectivePosition::Prefix { .. },
            })
        );
        let expression_owns_prefix_annotations = matches!(
            self,
            Expression::Binary {
                operator: BinaryOperator::ElementwiseOr,
                ..
            } if union_owns_prefix_annotations(f.context(), node_id) && !directive_is_prefix_ignore
        );
        if !expression_owns_prefix_annotations {
            write!(f, [f.context().any_prefix_annotations(node_id)])?;
        }

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
            let declarator_value_postfix_blanks_are_statement_owned =
                expression_is_declarator_value(f, node_id)
                    && expression_has_only_postfix_blank_annotations(f, node_id);
            if declarator_value_postfix_blanks_are_statement_owned {
                return Ok(());
            }

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
            let collection_handles_empty_infix = f.context().has_infix_annotation(node_id)
                && (matches!(
                    self,
                    Expression::ObjectExpression { properties, .. } if properties.is_empty()
                ) || matches!(
                    self,
                    Expression::ArrayExpression { elements } if elements.is_empty()
                ));
            if matches!(
                self,
                Expression::TypeUnary {
                    operator: TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime,
                    ..
                }
            ) {
                write!(f, [f.context().any_postfix_annotations(node_id)])?;
            } else if call_or_new_handles_empty_infix || collection_handles_empty_infix {
                write!(f, [f.context().any_postfix_annotations(node_id)])?;
            } else {
                write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
            }
        }

        Ok(())
    }
}

/// Whether an expression is "trivial" (prefers to be fully inline).
pub fn is_trivial_expression(tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::ScalarLiteral(_)
        | Expression::TypeLiteral(_)
        | Expression::PrivateIdentifier { .. } => true,
        Expression::ObjectExpression { ty, properties, .. } => {
            ty.is_none()
                && properties.len() <= 5
                && properties
                    .iter()
                    .all(|property| is_trivial_property(tree, tree.get(*property)))
        }
        Expression::Unary { operator: _, right } => is_trivial_expression(tree, tree.get(*right)),
        Expression::Index { left, index, .. } => index
            .as_ref()
            .map_or(is_trivial_expression(tree, tree.get(*left)), |index_id| {
                is_trivial_expression(tree, tree.get(*index_id))
            }),
        Expression::ReferenceOf {
            mutability: _,
            variance: _,
            right,
        } => is_trivial_expression(tree, tree.get(*right)),
        Expression::PointerOf {
            mutability: _,
            right,
        } => is_trivial_expression(tree, tree.get(*right)),
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            is_trivial_expression(tree, tree.get(*left))
        }
        Expression::Path {
            path,
            static_arguments,
        } => {
            path.segments.len() <= 3
                && static_arguments.as_deref().is_none_or(|static_arguments| {
                    static_arguments_are_trivial(tree, static_arguments)
                })
        }
        Expression::TypeBinary {
            left,
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            right,
        } => {
            is_trivial_expression(tree, tree.get(*left))
                && is_trivial_expression(tree, tree.get(*right))
        }
        _ => false,
    }
}

/// Return whether static arguments stay concise when inlined.
fn static_arguments_are_trivial(
    tree: &NodeTree,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    static_arguments
        .iter()
        .all(|argument_id| is_trivial_argument(tree, tree.get(*argument_id)))
}

/// Whether an expression is "complex" (prefers to be multiline).
pub fn is_complex_expression(_tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::Statement { .. } => true,
        // only expand objects with many (>3) properties by default
        // let best_fitting handle the rest based on line width
        Expression::ObjectExpression { properties, .. } => properties.len() > 3,
        Expression::TreeExpression { .. } => true,
        _ => false,
    }
}

/// Whether an argument is "trivial" (prefers to be inline).
pub fn is_trivial_argument(tree: &NodeTree, argument: &Argument) -> bool {
    match argument {
        Argument::Named { name: _, value, .. }
        | Argument::Labeled {
            label: _, value, ..
        }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => is_trivial_expression(tree, tree.get(*value)),
    }
}

/// Whether a property is "trivial" (prefers to be inline).
pub fn is_trivial_property(tree: &NodeTree, property: &Property) -> bool {
    match property {
        Property::Field { value, default, .. } => {
            value.is_none_or(|value| is_trivial_expression(tree, tree.get(value)))
                && default.is_none_or(|default| is_trivial_expression(tree, tree.get(default)))
        }
        Property::Method { body, .. } => {
            body.is_none_or(|body| is_trivial_expression(tree, tree.get(body)))
        }
        Property::Spread { value, .. } => is_trivial_expression(tree, tree.get(*value)),
    }
}

/// Whether an argument is "complex" (prefers to be multiline).
pub fn is_complex_argument(tree: &NodeTree, argument: &Argument) -> bool {
    match argument {
        Argument::Named { name: _, value, .. }
        | Argument::Labeled {
            label: _, value, ..
        }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => is_complex_expression(tree, tree.get(*value)),
    }
}

/// Whether the expression can break itself across multiple lines.
pub fn is_expression_breakable(tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::ArrayExpression { elements, .. } => !elements.is_empty(),
        Expression::TupleExpression { elements, .. } => !elements.is_empty(),
        Expression::SequenceExpression { expressions, .. } => !expressions.is_empty(),
        Expression::ObjectExpression { ty, properties, .. } => {
            ty.is_some_and(|ty| is_expression_breakable(tree, tree.get(ty)))
                || !properties.is_empty()
        }
        Expression::TreeExpression {
            arguments,
            elements,
            ..
        } => {
            arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
                || elements
                    .as_ref()
                    .is_some_and(|elements| !elements.is_empty())
        }
        Expression::Call {
            dynamic_arguments, ..
        } => !dynamic_arguments.is_empty(),
        Expression::New {
            dynamic_arguments, ..
        } => !dynamic_arguments.is_empty(),
        Expression::Match { .. } => true,
        Expression::Path {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|static_arguments| !static_arguments.is_empty()),
        Expression::If { .. }
        | Expression::Loop { .. }
        | Expression::Try { .. }
        | Expression::Block { .. }
        | Expression::ForEach { .. }
        | Expression::For { .. }
        | Expression::While { .. }
        | Expression::Import { .. }
        | Expression::Export { .. } => true,
        Expression::Binary { .. }
        | Expression::TypeBinary { .. }
        | Expression::TypeConditional { .. }
        | Expression::TypeMapped { .. }
        | Expression::TypeTemplateLiteral { .. } => true,
        _ => false,
    }
}

/// Check if a pattern can expand to multiple lines (object, array, tuple patterns).
pub fn is_pattern_breakable(tree: &NodeTree, pattern_id: LocalNodeId<Pattern>) -> bool {
    let pattern = tree.get(pattern_id);
    match pattern {
        Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => !fields.is_empty(),
        Pattern::Array { fields }
        | Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. } => !fields.is_empty(),
        _ => false,
    }
}

/// Return whether array elements are simple enough for concise fill formatting.
pub(crate) fn array_elements_are_fill_candidates(
    tree: &NodeTree,
    elements: &[LocalNodeId<Argument>],
) -> bool {
    elements
        .iter()
        .all(|element_id| array_element_is_fill_candidate(tree, *element_id))
}

/// Return whether comments in an array appear only before the first or after the last element.
pub(crate) fn array_has_only_boundary_comments(
    context: &DestackFormatContext<'_>,
    array_span: Span,
    elements: &[LocalNodeId<Argument>],
) -> bool {
    let (Some(first_element), Some(last_element)) = (elements.first(), elements.last()) else {
        return false;
    };

    let first_span = context.span(*first_element);
    let last_span = context.span(*last_element);

    let has_internal_comment =
        context
            .tokens
            .iter()
            .chain(context.side_tokens.iter())
            .any(|token| {
                if !array_span.intersects(token.span) || !is_comment_token_type(token.token.ty) {
                    return false;
                }

                let is_before_first = token.span.end <= first_span.start;
                let is_after_last = token.span.start >= last_span.end;
                !(is_before_first || is_after_last)
            });

    !has_internal_comment
}

/// Return whether a token type is a comment token.
fn is_comment_token_type(token_type: TokenType) -> bool {
    matches!(
        token_type,
        TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment
    )
}

/// Return whether a single array element is a concise fill candidate.
fn array_element_is_fill_candidate(tree: &NodeTree, element_id: LocalNodeId<Argument>) -> bool {
    let value_id = match tree.get(element_id) {
        Argument::Positional { value, .. } => *value,
        _ => return false,
    };

    match tree.get(value_id) {
        Expression::ScalarLiteral(
            ScalarLiteral::Integer(_) | ScalarLiteral::Bigint(_) | ScalarLiteral::Float(_),
        ) => true,
        Expression::Unary { operator, right } => {
            let mut right_id = *right;
            while let Expression::Parenthesized { expression } = tree.get(right_id) {
                right_id = *expression;
            }

            matches!(operator, UnaryOperator::Plus | UnaryOperator::Negate)
                && matches!(
                    tree.get(right_id),
                    Expression::ScalarLiteral(
                        ScalarLiteral::Integer(_)
                            | ScalarLiteral::Bigint(_)
                            | ScalarLiteral::Float(_)
                    )
                )
        }
        _ => false,
    }
}

/// Return whether an expression tree contains static type arguments.
pub(crate) fn expression_has_static_type_arguments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Path {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Member {
            left,
            static_arguments,
            ..
        }
        | Expression::PrivateMember {
            left,
            static_arguments,
            ..
        } => {
            static_arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
                || expression_has_static_type_arguments(context, *left)
        }
        Expression::Call {
            left,
            static_arguments,
            ..
        }
        | Expression::New {
            left,
            static_arguments,
            ..
        } => {
            static_arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
                || expression_has_static_type_arguments(context, *left)
        }
        Expression::Instantiation {
            left,
            static_arguments,
        } => !static_arguments.is_empty() || expression_has_static_type_arguments(context, *left),
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            expression_has_static_type_arguments(context, *expression)
        }
        _ => false,
    }
}

/// Return whether expression source is wrapped in a top-level parenthesis pair.
pub(crate) fn expression_has_outer_parentheses_tokens(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let span = context.span(node_id);
    let Some(first_token) = first_non_trivia_token_in_span(context, span) else {
        return false;
    };
    let Some(last_token) = last_non_trivia_token_in_span(context, span) else {
        return false;
    };

    first_token.token.ty == TokenType::OpenParenthesis
        && last_token.token.ty == TokenType::CloseParenthesis
}

/// Return whether an expression has a prefix comment annotation.
pub(crate) fn expression_has_prefix_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(expression_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Comment {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    } | Annotation::Doc {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false)
}

/// Return whether an expression has a prefix ignore-directive comment annotation.
pub(crate) fn expression_has_prefix_ignore_directive_comment_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(expression_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let Annotation::Comment { position, node } = context.annotation(*annotation_id)
                else {
                    return false;
                };

                if !matches!(
                    position,
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) {
                    return false;
                }

                if comment_node_is_ignore_directive(context, node) {
                    return true;
                }
                false
            })
        })
        .unwrap_or(false)
}

/// Return whether an expression has a leading prefix comment in its left spine.
pub(crate) fn expression_has_leading_prefix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current = expression_id;

    loop {
        if expression_has_prefix_comment_annotation(context, current) {
            return true;
        }

        let next = match context.tree.get(current) {
            Expression::Parenthesized { expression } => Some(*expression),
            Expression::Binary { left, .. } | Expression::TypeBinary { left, .. } => Some(*left),
            Expression::If {
                kind: IfKind::Ternary,
                condition:
                    IfCondition::Expression {
                        condition: expression_id,
                    },
                ..
            } => Some(*expression_id),
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => Some(*left),
            Expression::TaggedTemplateExpression { tag, .. } => Some(*tag),
            _ => None,
        };

        let Some(next) = next else {
            break;
        };
        current = next;
    }

    false
}

/// Return whether parenthesized cast comments should be hoisted before `(`.
pub(crate) fn should_hoist_parenthesized_inner_cast_prefix_comments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_id: LocalNodeId<Expression>,
) -> bool {
    let is_parent_yield_value = context
        .parent(node_id)
        .is_some_and(|(parent_id, parent_type)| {
            parent_type == NodeType::Expression
                && matches!(
                    context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::Yield { value: Some(value_id), .. } if *value_id == node_id
                )
        });
    if is_parent_yield_value {
        return false;
    }

    let has_doc_like_prefix_annotation = context
        .visit_annotations(inner_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.annotation(*annotation_id),
                    Annotation::Doc {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    }
                )
            })
        })
        .unwrap_or(false);

    parenthesized_has_leading_inner_trivia(context, node_id, inner_id)
        && has_doc_like_prefix_annotation
        && !expression_has_prefix_ignore_directive_comment_annotation(context, inner_id)
}

/// Decide whether a sequence expression needs parentheses in its parent context.
pub(crate) fn sequence_expression_needs_parens(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };

    if parent_type != NodeType::Expression {
        return true;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_id) {
        Expression::Statement(_) => true,
        Expression::Return { value } => value.is_some_and(|value_id| value_id != node_id),
        Expression::Throw { value } => *value != node_id,
        Expression::Parenthesized { expression } => *expression != node_id,
        Expression::For {
            initialization,
            increment,
            ..
        } => {
            !initialization.is_some_and(|value_id| value_id == node_id)
                && !increment.is_some_and(|value_id| value_id == node_id)
        }
        // preserve explicit nested grouping: `(1, (2, 3), 4)`
        Expression::SequenceExpression { .. } => {
            expression_has_outer_parentheses_tokens(context, node_id)
        }
        _ => true,
    }
}
