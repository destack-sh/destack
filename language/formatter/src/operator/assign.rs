use super::binary::is_logical_binary_operator;
use super::r#type::{
    expression_has_generic_arguments, write_type_expression_with_inline_prefix_annotations,
};
use crate::annotation::{
    FormatTrailingComments, format_comment, prefix_annotations, write_comment_slice,
    write_inline_prefix_annotations,
};
use crate::chain::{
    MemberChain, assignment_like_parent, is_assignment_chain_tail_lambda,
    transparent_inner_expression,
};
use crate::context::DestackFormatterSpeculationExt;
use crate::declaration::{FormatLambdaDeclarationOptions, format_lambda_declaration_with_options};
use crate::expression::{ExpressionLeftSide, write_expression_without_prefix_annotations};
use crate::{DestackFormatContext, DestackFormatter};
use destack_dir::{
    Argument, AssignOperator, AssignPattern, AssignPatternField, BinaryOperator, Comment,
    Declaration, Declarator, DecoratorPosition, Expression, FunctionDeclaration, FunctionForm,
    GenericArgument, IfCondition, IfForm, LocalNodeId, NodeType, Pattern, PatternField,
    ScalarLiteral, TemplateLiteral, TokenType, TypeExpression,
};
use destack_fir::format::{
    Buffer, Format, FormatError, FormatNode as FirFormatNode, FormatNodes, FormatResult,
    Formatter as FirFormatter, VecBuffer,
};
use destack_fir::prelude::{
    empty_line, format_with, group, hard_line_break, indent, indent_if_group_breaks,
    line_suffix_boundary, soft_line_break_or_space, soft_line_indent_or_space, space, token,
};
use destack_fir::write;
use destack_source::Span;

/// Return whether one argument expression is short enough to keep a call attached.
fn is_short_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
    threshold: u32,
) -> bool {
    let argument_expression_id = match context.tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
        Argument::Error => return false,
    };

    is_short_expression(context, argument_expression_id, threshold)
}

/// Return whether one expression is short enough to keep a call attached.
fn is_short_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    threshold: u32,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Identifier { name } => context.strings.get(*name).len() <= threshold as usize,
        Expression::Unary { right, .. } => is_short_expression(context, *right, threshold),
        Expression::ScalarLiteral(
            ScalarLiteral::Null
            | ScalarLiteral::Boolean(_)
            | ScalarLiteral::Integer(_)
            | ScalarLiteral::Bigint(_)
            | ScalarLiteral::Float(_),
        )
        | Expression::This => true,
        Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
            context.strings.get(*string_id).len() <= threshold as usize
        }
        Expression::ScalarLiteral(ScalarLiteral::RegexString { content, .. }) => {
            context.strings.get(*content).len() <= threshold as usize
        }
        Expression::TemplateExpression { value } => {
            // interpolated templates are not short in the assignment-like rules
            let TemplateLiteral::String { string } = value else {
                return false;
            };

            let content = context.strings.get(*string);

            content.len() <= threshold as usize && !content.contains('\n')
        }
        Expression::Call {
            left, arguments, ..
        } => {
            arguments.is_empty()
                && matches!(
                    context.tree.get(transparent_inner_expression(context, *left)),
                    Expression::Identifier { name }
                        if context.strings.get(*name).len()
                            <= threshold.saturating_sub(2) as usize
                )
        }
        _ => false,
    }
}

/// Return whether one single type argument is complex for assignment-like layout.
fn type_argument_is_complex(
    context: &DestackFormatContext<'_>,
    type_id: LocalNodeId<TypeExpression>,
) -> bool {
    matches!(
        context.tree.get(type_id),
        TypeExpression::Union { .. }
            | TypeExpression::Intersection { .. }
            | TypeExpression::Object { .. }
    )
}

/// Return whether one generic argument list is complex enough to break a call chain.
fn is_complex_generic_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> FormatResult<bool> {
    // multiple arguments always count as complex in the assignment-like layout
    if generic_arguments.len() > 1 {
        return Ok(true);
    }

    // inspect the single argument structurally first
    let Some(argument_id) = generic_arguments.first().copied() else {
        return Ok(false);
    };

    match f.context().tree.get(argument_id) {
        GenericArgument::Type { value } => {
            if type_argument_is_complex(f.context(), *value) {
                return Ok(true);
            }
        }
        GenericArgument::Value { value } => {
            let value = transparent_inner_expression(f.context(), *value);

            // value arguments use the same threshold as complex type arguments
            if matches!(
                f.context().tree.get(value),
                Expression::Binary {
                    operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
                    ..
                }
            ) {
                return Ok(true);
            }

            if let Expression::Type { value } = f.context().tree.get(value)
                && type_argument_is_complex(f.context(), *value)
            {
                return Ok(true);
            }
        }
        GenericArgument::Error => return Ok(false),
    }

    // measure remaining cases with one speculative render
    let argument_span = f.context().span(argument_id);
    let start = f
        .context()
        .previous_non_trivia_token_before_span(argument_span)
        .map_or(argument_span.start, |token| token.span.start);
    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        super::r#type::format_generic_argument_list(f, generic_arguments)
    });

    f.speculate_will_break_after(start, &content)
}

/// Return whether one call or member chain is awkward to break inside an assignment layout.
pub(crate) fn is_poorly_breakable_member_or_call_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let threshold = u32::from(f.context().options.line_width) / 4;
    let root_expression_id = transparent_inner_expression(f.context(), expression_id);
    let mut current_expression_id = root_expression_id;
    let mut is_chain = false;
    let mut has_simple_head = false;
    let mut call_expression_ids = Vec::new();
    let mut call_generic_argument_groups = Vec::<Vec<LocalNodeId<GenericArgument>>>::new();

    loop {
        current_expression_id = match f.context().tree.get(current_expression_id) {
            // call
            Expression::Call {
                left,
                generic_arguments,
                ..
            } => {
                is_chain = true;
                call_expression_ids.push(current_expression_id);
                call_generic_argument_groups.push(generic_arguments.clone());
                transparent_inner_expression(f.context(), *left)
            }

            // instantiation
            Expression::Instantiation {
                left,
                generic_arguments,
            } => {
                is_chain = true;
                if is_complex_generic_arguments(f, generic_arguments)? {
                    return Ok(false);
                }

                transparent_inner_expression(f.context(), *left)
            }

            // member
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => {
                is_chain = true;
                transparent_inner_expression(f.context(), *left)
            }

            // simple heads
            Expression::Identifier { .. } | Expression::This => {
                has_simple_head = true;
                break;
            }

            // non-chains
            _ => break,
        };
    }

    // non-simple chain heads do not use this layout shortcut
    if !is_chain || !has_simple_head {
        return Ok(false);
    }

    // pure member chains are cheap to keep attached
    if call_expression_ids.is_empty() {
        return Ok(true);
    }

    // comments on the outer call break the shortcut
    if f.context()
        .comments()
        .has_comment_in_span(f.context().span(call_expression_ids[0]))
    {
        return Ok(false);
    }

    // breakable calls defeat the shortcut
    for (index, call_expression_id) in call_expression_ids.iter().copied().enumerate() {
        let Expression::Call { arguments, .. } = f.context().tree.get(call_expression_id) else {
            continue;
        };

        let is_breakable_call = match arguments.len() {
            0 => false,
            1 => {
                let argument_id = arguments[0];
                !is_short_argument(f.context(), argument_id, threshold)
            }
            _ => true,
        };
        if is_breakable_call {
            return Ok(false);
        }

        if let Some(generic_arguments) = call_generic_argument_groups.get(index)
            && is_complex_generic_arguments(f, generic_arguments)?
        {
            return Ok(false);
        }
    }

    // member call chains already have enough internal structure
    MemberChain::is_member_call_chain(f.context(), call_expression_ids[0])
        .map(|is_member_call_chain| !is_member_call_chain)
}

/// Return whether one expression has an own-line prefix annotation.
fn assign_expression_has_own_line_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .annotation_ids(expression_id)
        .iter()
        .copied()
        .any(|annotation_id| {
            let annotation = context.annotation(annotation_id);
            if !matches!(
                annotation.position,
                DecoratorPosition::LinePrefix | DecoratorPosition::BlockPrefix
            ) {
                return false;
            }

            context.annotation_starts_on_own_line(annotation_id)
        })
}

/// Return whether one token is an assignment operator token.
#[inline]
fn is_assignment_operator_token(token_type: TokenType) -> bool {
    AssignOperator::from_token(token_type).is_some()
}

/// Return whether one expression has an inline prefix comment after an assignment operator.
pub(crate) fn assignment_rhs_has_inline_operator_prefix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    assignment_rhs_has_inline_operator_prefix_annotation_style(context, expression_id, |_| true)
}

/// Return whether one expression has an inline prefix assignment-operator annotation matching one filter.
fn assignment_rhs_has_inline_operator_prefix_annotation_style(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    mut style_filter: impl FnMut(bool) -> bool,
) -> bool {
    let mut left_side = Some(ExpressionLeftSide::new(transparent_inner_expression(
        context,
        expression_id,
    )));

    while let Some(current_left_side) = left_side {
        let current_expression_id = current_left_side.expression_id();

        let has_inline_prefix_comment =
            assignment_rhs_operator_comment_nodes(context, current_expression_id)
                .into_iter()
                .any(|comment| {
                    let Some(previous_token) =
                        context.previous_non_trivia_token_before_span(comment.span)
                    else {
                        return false;
                    };
                    if !is_assignment_operator_token(previous_token.token.ty) {
                        return false;
                    }

                    let assignment_and_comment_share_line = context.file.is_same_line(
                        previous_token.span.end.saturating_sub(1),
                        comment.span.start,
                    );
                    if !assignment_and_comment_share_line {
                        return false;
                    }

                    let is_slash_style = comment.is_line();
                    if !style_filter(is_slash_style) {
                        return false;
                    }

                    if is_slash_style {
                        return true;
                    }

                    !context.has_newline(comment.span)
                        && !context.span_has_newline_before_next_non_whitespace_token(comment.span)
                });
        if has_inline_prefix_comment {
            return true;
        }

        left_side = current_left_side.left(context);
    }

    false
}

/// Return whether one rhs expression is a class declaration.
fn expression_is_class_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(transparent_inner_expression(context, expression_id)),
        Expression::Declaration(declaration_id)
            if matches!(context.tree.get(*declaration_id), Declaration::Class(_))
    )
}

/// Return whether one assignment operator has a slash line comment between left and right.
pub(crate) fn assignment_operator_has_line_comment_between(
    context: &DestackFormatContext<'_>,
    left: LocalNodeId<AssignPattern>,
    right: LocalNodeId<Expression>,
) -> bool {
    let left_span = context.span(left);
    let right_span = context.span(right);
    let Some(between_span) = left_span.gap_to(right_span) else {
        return false;
    };
    let comment_tokens = context.comment_tokens_intersecting_span(between_span);
    comment_tokens.into_iter().any(|comment_token| {
        if !matches!(
            comment_token.token.ty,
            TokenType::LineComment | TokenType::DocLineComment
        ) {
            return false;
        }

        context
            .previous_non_whitespace_token_before_span(comment_token.span)
            .is_some_and(|token| is_assignment_operator_token(token.token.ty))
    })
}

/// Return comments between one assignment operator and rhs expression.
fn assignment_rhs_operator_comment_nodes(
    context: &DestackFormatContext<'_>,
    right: LocalNodeId<Expression>,
) -> Vec<Comment> {
    let right_span = context.span(right);
    let right_token_start = context
        .first_non_trivia_token_in_span(right_span)
        .map_or(right_span.start, |token| token.span.start);
    let mut previous_token = context.previous_non_whitespace_token_before_span(Span::new(
        right_span.file,
        right_token_start,
        right_token_start,
    ));

    // transparent grouping and adjacent comments
    while let Some(token) = previous_token {
        if !matches!(
            token.token.ty,
            TokenType::OpenParenthesis
                | TokenType::LineComment
                | TokenType::BlockComment
                | TokenType::DocLineComment
                | TokenType::DocBlockComment
        ) {
            break;
        }

        previous_token = context.previous_non_whitespace_token_before_span(token.span);
    }

    let Some(previous_token) = previous_token else {
        return Vec::new();
    };

    if !is_assignment_operator_token(previous_token.token.ty)
        || previous_token.span.file != right_span.file
        || previous_token.span.end >= right_token_start
    {
        return Vec::new();
    }

    {
        let comments = context.comments();
        comments
            .comments_in_range(previous_token.span.end, right_token_start)
            .to_vec()
    }
}

/// Return the simple expression target inside one assign-pattern, when one exists.
pub(crate) fn assign_pattern_target_expression(
    context: &DestackFormatContext<'_>,
    pattern_id: LocalNodeId<AssignPattern>,
) -> Option<LocalNodeId<Expression>> {
    match context.tree.get(pattern_id) {
        // direct target
        AssignPattern::Expression { value } => Some(*value),

        // default wrapper
        AssignPattern::Assign { pattern, .. } => {
            assign_pattern_target_expression(context, *pattern)
        }

        // destructuring targets
        AssignPattern::Sequence { .. } | AssignPattern::Object { .. } => None,
    }
}

/// Return whether one assign-pattern contains the expression.
pub(crate) fn assign_pattern_contains_expression(
    context: &DestackFormatContext<'_>,
    pattern_id: LocalNodeId<AssignPattern>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(pattern_id) {
        // direct target
        AssignPattern::Expression { value } => *value == expression_id,

        // default wrapper
        AssignPattern::Assign { pattern, value } => {
            assign_pattern_contains_expression(context, *pattern, expression_id)
                || *value == expression_id
        }

        // destructuring fields
        AssignPattern::Sequence { fields } | AssignPattern::Object { fields } => {
            fields.iter().copied().any(|field_id| {
                assign_pattern_field_contains_expression(context, field_id, expression_id)
            })
        }
    }
}

/// Return whether one assign-pattern field contains the expression.
fn assign_pattern_field_contains_expression(
    context: &DestackFormatContext<'_>,
    field_id: LocalNodeId<AssignPatternField>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(field_id) {
        // named and spread fields
        AssignPatternField::Named { pattern, .. } | AssignPatternField::Spread { pattern } => {
            pattern.is_some_and(|pattern_id| {
                assign_pattern_contains_expression(context, pattern_id, expression_id)
            })
        }

        // keyed and positional fields
        AssignPatternField::Computed { key, pattern } => {
            *key == expression_id
                || assign_pattern_contains_expression(context, *pattern, expression_id)
        }
        AssignPatternField::Positional { pattern } => {
            assign_pattern_contains_expression(context, *pattern, expression_id)
        }

        // elisions
        AssignPatternField::Elision => false,
    }
}

/// Buffer one assignment-expression left-hand side for layout selection.
fn buffer_assignment_expression_layout_left<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<AssignPattern>,
    operator_span: Span,
) -> FormatResult<(Vec<FirFormatNode>, bool, bool)> {
    let left_comments =
        assignment_left_trailing_comments(f.context(), f.context().span(left).end, operator_span);

    let mut buffer = VecBuffer::new(f.state_mut());
    let formatter = &mut FirFormatter::new(&mut buffer);

    // left side
    write!(formatter, [left])?;
    write_assignment_left_trailing_comments(formatter, &left_comments)?;

    let nodes = buffer.into_vec();

    let may_break = nodes.may_directly_break();

    // assignment-expression layout is driven by the rhs
    Ok((nodes, false, may_break))
}

/// Buffer one declarator left-hand side for layout selection.
fn buffer_declarator_layout_left<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    declarator_id: LocalNodeId<Declarator>,
    pattern_id: LocalNodeId<Pattern>,
    type_id: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<(Vec<FirFormatNode>, bool, bool)> {
    let left_comments = if let Some(operator_span) = f.context().tree.get_main_span(declarator_id) {
        let left_end = type_id
            .map(|type_id| f.context().span(type_id).end)
            .unwrap_or_else(|| f.context().span(pattern_id).end);

        assignment_left_trailing_comments(f.context(), left_end, operator_span)
    } else if f.context().tree.get(declarator_id).value.is_some() {
        return Err(FormatError::SyntaxError {
            message: "declarator assignment requires an operator span",
        });
    } else {
        Vec::new()
    };

    let mut buffer = VecBuffer::new(f.state_mut());
    let formatter = &mut FirFormatter::new(&mut buffer);

    // pattern
    write!(formatter, [pattern_id])?;

    // type annotation
    if let Some(type_id) = type_id {
        write!(formatter, [token(":"), space()])?;
        write_type_expression_with_inline_prefix_annotations(formatter, type_id)?;
    }

    write_assignment_left_trailing_comments(formatter, &left_comments)?;

    let nodes = buffer.into_vec();

    let may_break = nodes.may_directly_break();

    // declarator layout is driven by the rhs
    Ok((nodes, false, may_break))
}

/// Return comments that syntactically trail the left side before the assignment operator.
fn assignment_left_trailing_comments(
    context: &DestackFormatContext<'_>,
    left_end: u32,
    operator_span: Span,
) -> Vec<Comment> {
    let comments = context
        .comments()
        .comments_in_range(left_end, operator_span.start);
    if comments.iter().any(|comment| comment.preceded_by_newline()) {
        return Vec::new();
    }

    comments.to_vec()
}

/// Write comments that syntactically trail the left side before the assignment operator.
fn write_assignment_left_trailing_comments<'ast>(
    f: &mut FirFormatter<'_, DestackFormatContext<'ast>>,
    comments: &[Comment],
) -> FormatResult<()> {
    write!(f, [FormatTrailingComments::Comments(comments)])
}

/// Return whether one declarator pattern subtree contains one default assignment.
fn declarator_pattern_has_default_assignment(
    context: &DestackFormatContext<'_>,
    pattern_id: LocalNodeId<Pattern>,
) -> bool {
    match context.tree.get(pattern_id) {
        // direct assignment wrapper
        Pattern::Assign { .. } => true,

        // leaf patterns
        Pattern::Wildcard
        | Pattern::Expression { .. }
        | Pattern::Range { .. }
        | Pattern::TypeExpression { .. } => false,

        // transparent wrappers
        Pattern::Must(inner_pattern_id)
        | Pattern::BorrowOf {
            right: inner_pattern_id,
            ..
        }
        | Pattern::MoveOf {
            right: inner_pattern_id,
            ..
        } => declarator_pattern_has_default_assignment(context, *inner_pattern_id),

        // binding wrapper
        Pattern::Binding { pattern, .. } => pattern.as_ref().is_some_and(|inner_pattern_id| {
            declarator_pattern_has_default_assignment(context, *inner_pattern_id)
        }),

        // field collections
        Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. }
        | Pattern::Sequence { fields }
        | Pattern::Object { fields }
        | Pattern::TaggedObject { fields, .. } => fields
            .iter()
            .copied()
            .any(|field_id| declarator_pattern_field_has_default_assignment(context, field_id)),

        // union members
        Pattern::Union { patterns } => patterns.iter().copied().any(|inner_pattern_id| {
            declarator_pattern_has_default_assignment(context, inner_pattern_id)
        }),
    }
}

/// Return whether one declarator pattern field contains one default assignment.
fn declarator_pattern_field_has_default_assignment(
    context: &DestackFormatContext<'_>,
    pattern_field_id: LocalNodeId<PatternField>,
) -> bool {
    match context.tree.get(pattern_field_id) {
        // optional nested field
        PatternField::Named { pattern, .. } => pattern.as_ref().is_some_and(|pattern_id| {
            declarator_pattern_has_default_assignment(context, *pattern_id)
        }),

        // required nested field
        PatternField::Computed { pattern, .. } => {
            declarator_pattern_has_default_assignment(context, *pattern)
        }
        PatternField::Positional { pattern } => {
            declarator_pattern_has_default_assignment(context, *pattern)
        }

        // spread field
        PatternField::Spread { pattern, .. } => pattern.as_ref().is_some_and(|pattern_id| {
            declarator_pattern_has_default_assignment(context, *pattern_id)
        }),

        // empty slot
        PatternField::Elision => false,
    }
}

/// Return whether one declarator pattern is complex enough to break its left side first.
fn declarator_pattern_is_complex_destructuring(
    context: &DestackFormatContext<'_>,
    pattern_id: LocalNodeId<Pattern>,
) -> bool {
    match context.tree.get(pattern_id) {
        // binding wrapper
        Pattern::Binding { pattern, .. } => pattern.as_ref().is_some_and(|inner_pattern_id| {
            declarator_pattern_is_complex_destructuring(context, *inner_pattern_id)
        }),

        // transparent wrappers
        Pattern::Must(inner_pattern_id)
        | Pattern::BorrowOf {
            right: inner_pattern_id,
            ..
        }
        | Pattern::MoveOf {
            right: inner_pattern_id,
            ..
        } => declarator_pattern_is_complex_destructuring(context, *inner_pattern_id),

        // wide object destructuring
        Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => {
            if fields.len() <= 2 {
                return false;
            }

            fields.iter().copied().any(|field_id| {
                declarator_pattern_field_is_complex_destructuring(context, field_id)
            })
        }

        // other patterns stay compact
        _ => false,
    }
}

/// Return whether one declarator object-pattern field makes the left side complex.
fn declarator_pattern_field_is_complex_destructuring(
    context: &DestackFormatContext<'_>,
    pattern_field_id: LocalNodeId<PatternField>,
) -> bool {
    match context.tree.get(pattern_field_id) {
        // named nesting expands the lhs
        PatternField::Named { pattern, .. } => pattern.is_some(),

        // computed keys are always complex
        PatternField::Computed { .. } => true,

        // flat fields stay compact
        PatternField::Positional { .. } | PatternField::Spread { .. } | PatternField::Elision => {
            false
        }
    }
}

/// Return whether one declarator rhs is one lambda-like declaration.
fn declarator_value_is_lambda_like(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => matches!(
            context.tree.get(*declaration_id),
            Declaration::Function(function) if function.signature.form == FunctionForm::Lambda
        ),
        _ => false,
    }
}

/// Return whether one type expression contains generic arguments.
fn declaration_type_expression_has_generic_arguments(
    context: &DestackFormatContext<'_>,
    type_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(type_id) {
        TypeExpression::Reference {
            generic_arguments, ..
        }
        | TypeExpression::Member {
            generic_arguments, ..
        } => !generic_arguments.is_empty(),
        _ => false,
    }
}

/// Return whether one declaration heritage clause contains generic arguments.
fn declaration_has_generic_heritage(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    match context.tree.get(declaration_id) {
        Declaration::Struct(declaration) => declaration
            .implements_types
            .iter()
            .copied()
            .any(|type_id| declaration_type_expression_has_generic_arguments(context, type_id)),
        Declaration::Class(declaration) => {
            declaration
                .extends_expression
                .iter()
                .copied()
                .any(|expression_id| expression_has_generic_arguments(context, expression_id))
                || !declaration.extends_generic_arguments.is_empty()
                || declaration.implements_types.iter().copied().any(|type_id| {
                    declaration_type_expression_has_generic_arguments(context, type_id)
                })
        }
        Declaration::Enum(declaration) => declaration
            .implements_types
            .iter()
            .copied()
            .any(|type_id| declaration_type_expression_has_generic_arguments(context, type_id)),
        Declaration::Interface(declaration) => declaration.extends.iter().any(|heritage| {
            !heritage.generic_arguments.is_empty()
                || expression_has_generic_arguments(context, heritage.expression)
        }),
        Declaration::Extension(declaration) => {
            declaration_type_expression_has_generic_arguments(context, declaration.target_type)
                || declaration.implements_types.iter().copied().any(|type_id| {
                    declaration_type_expression_has_generic_arguments(context, type_id)
                })
        }
        _ => false,
    }
}

/// Return whether one declarator rhs wraps a class declaration with generic heritage.
fn declarator_value_has_generic_class_heritage(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => {
            matches!(context.tree.get(*declaration_id), Declaration::Class(_))
                && declaration_has_generic_heritage(context, *declaration_id)
        }
        Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. } => {
            declarator_value_has_generic_class_heritage(context, *left)
        }
        _ => false,
    }
}

/// Write comments between one assignment operator and rhs expression.
pub(crate) fn write_assignment_rhs_operator_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    right: LocalNodeId<Expression>,
    omit_leading_separator: bool,
) -> FormatResult<()> {
    let comment_nodes = assignment_rhs_operator_comment_nodes(f.context(), right);
    if comment_nodes.is_empty() {
        return Ok(());
    }

    let Some(previous_token) = f
        .context()
        .previous_non_trivia_token_before_span(comment_nodes[0].span)
    else {
        return Ok(());
    };

    // leading separator
    let first_comment_span = comment_nodes[0].span;
    let leading_gap = Span::new(
        first_comment_span.file,
        previous_token.span.end,
        first_comment_span.start,
    );
    if !omit_leading_separator {
        if f.context().has_newline(leading_gap)
            || f.context().span_starts_on_own_line(first_comment_span)
        {
            write!(f, [hard_line_break()])?;
        } else {
            write!(f, [space()])?;
        }
    }

    // inline line comments belong to the operator boundary
    if omit_leading_separator && comment_nodes[0].is_line() {
        let first_comment = comment_nodes[0];

        if !f.context().span_starts_on_own_line(first_comment.span) {
            write!(f, [space()])?;
        }

        format_comment(f, first_comment)?;
        write!(f, [hard_line_break()])?;

        if comment_nodes.len() > 1 {
            write_comment_slice(f, &comment_nodes[1..])?;
        }

        return Ok(());
    }

    // comment body
    format_comment(f, comment_nodes[0])?;
    if comment_nodes.len() > 1 {
        write_comment_slice(f, &comment_nodes[1..])?;
    }

    // trailing separator
    let Some(last_comment) = comment_nodes.last().copied() else {
        return Ok(());
    };

    let next_token = f
        .context()
        .next_non_whitespace_token_after_span(last_comment.span);
    let gap_span = next_token.and_then(|next_token| last_comment.span.gap_to(next_token.span));

    if last_comment.is_line() || gap_span.is_some_and(|gap_span| f.context().has_newline(gap_span))
    {
        if gap_span.is_some_and(|gap_span| f.context().has_blank_line(gap_span)) {
            write!(f, [empty_line()])?;
        } else {
            write!(f, [hard_line_break()])?;
        }
    } else {
        write!(f, [space()])?;
    }

    Ok(())
}

/// Write one declarator rhs while preserving inline operator prefix annotations.
fn write_declarator_assignment_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
    layout: AssignmentLikeLayout,
    rhs_has_inline_operator_prefix_comment: bool,
    rhs_operator_comment_nodes: &[Comment],
) -> FormatResult<()> {
    let prefix_annotation_ids: Vec<_> = f
        .context()
        .annotation_ids(value_id)
        .iter()
        .copied()
        .filter(|annotation_id| {
            matches!(
                f.context().annotation(*annotation_id).position,
                DecoratorPosition::BlockPrefix | DecoratorPosition::LinePrefix
            )
        })
        .collect();

    let mut write_value = || {
        // plain rhs
        if prefix_annotation_ids.is_empty() {
            if !rhs_operator_comment_nodes.is_empty() {
                return write_expression_with_assignment_layout(f, value_id, layout, true);
            }

            write_expression_with_assignment_layout(f, value_id, layout, false)?;
            return Ok(());
        }

        // inline operator prefix annotations
        if rhs_has_inline_operator_prefix_comment {
            write_inline_prefix_annotations(f, &prefix_annotation_ids)?;
            write!(f, [space()])?;
            return write_expression_with_assignment_layout(f, value_id, layout, true);
        }

        // normal prefix annotations
        write!(f, [prefix_annotations(f.context(), value_id)])?;

        if let Some(last_prefix_annotation_id) = prefix_annotation_ids.last().copied()
            && f.context()
                .annotation_next_token_is_on_same_line(last_prefix_annotation_id)
        {
            write!(f, [space()])?;
        }

        write_expression_with_assignment_layout(f, value_id, layout, true)
    };

    if rhs_operator_comment_nodes.is_empty() {
        return write_value();
    }

    write_value()
}

/// Write one expression with assignment-like layout routed into lambda declarations.
fn write_expression_with_assignment_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    layout: AssignmentLikeLayout,
    without_prefix_annotations: bool,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);

    let Expression::Declaration(declaration_id) = expression else {
        if without_prefix_annotations {
            return write_expression_without_prefix_annotations(f, expression_id);
        }

        return write!(f, [expression_id]);
    };

    let Declaration::Function(FunctionDeclaration {
        name,
        export,
        is_ambient,
        signature,
        body,
    }) = f.context().tree.get(*declaration_id)
    else {
        if without_prefix_annotations {
            return write_expression_without_prefix_annotations(f, expression_id);
        }

        return write!(f, [expression_id]);
    };

    if signature.form != FunctionForm::Lambda {
        if without_prefix_annotations {
            return write_expression_without_prefix_annotations(f, expression_id);
        }

        return write!(f, [expression_id]);
    }

    if !without_prefix_annotations {
        write!(f, [prefix_annotations(f.context(), *declaration_id)])?;
    }

    format_lambda_declaration_with_options(
        f,
        *declaration_id,
        *export,
        *is_ambient,
        *name,
        signature,
        body,
        FormatLambdaDeclarationOptions {
            assignment_layout: Some(layout),
            ..FormatLambdaDeclarationOptions::default()
        },
    )
}

/// Format one declarator assignment through the shared assignment-like owner.
pub(crate) fn format_declarator_assignment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    declarator_id: LocalNodeId<Declarator>,
) -> FormatResult<()> {
    AssignmentLike::Declarator(declarator_id).format(f)
}

/// One layout for one assignment-like expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AssignmentLikeLayout {
    /// Break the right-hand side only when it forces the outer group to expand.
    Fluid,

    /// Break after the operator and indent the right-hand side as one unit.
    BreakAfterOperator,

    /// Keep the operator and right-hand side on the same line.
    NeverBreakAfterOperator,

    /// Break the left-hand side first and then group the right-hand side independently.
    BreakLeftHandSide,

    /// Keep a chained assignment head attached to the following assignment.
    Chain,

    /// Indent the final right-hand side of one eligible assignment chain.
    ChainTail,

    /// Keep one arrow-function tail attached to the final chain operator.
    ChainTailArrowFunction,
}

/// One assignment-like formatter owner.
#[derive(Clone, Copy, Debug)]
enum AssignmentLike {
    /// One declarator assignment.
    Declarator(LocalNodeId<Declarator>),

    /// One assignment expression.
    Expression {
        node_id: LocalNodeId<Expression>,
        left: LocalNodeId<AssignPattern>,
        operator: AssignOperator,
        right: LocalNodeId<Expression>,
    },
}

impl AssignmentLike {
    /// Return the right-hand side expression when one exists.
    fn right(self, context: &DestackFormatContext<'_>) -> Option<LocalNodeId<Expression>> {
        match self {
            AssignmentLike::Declarator(declarator_id) => context.tree.get(declarator_id).value,
            AssignmentLike::Expression { right, .. } => Some(right),
        }
    }

    /// Buffer the left-hand side for layout selection.
    fn buffer_left<'ast>(
        self,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<(Vec<FirFormatNode>, bool, bool)> {
        match self {
            AssignmentLike::Declarator(declarator_id) => {
                let declarator = f.context().tree.get(declarator_id);
                buffer_declarator_layout_left(f, declarator_id, declarator.pattern, declarator.ty)
            }
            AssignmentLike::Expression { node_id, left, .. } => {
                let operator_span =
                    f.context()
                        .tree
                        .get_main_span(node_id)
                        .ok_or(FormatError::SyntaxError {
                            message: "assignment expression requires an operator span",
                        })?;

                buffer_assignment_expression_layout_left(f, left, operator_span)
            }
        }
    }

    /// Select one layout for one assignment-like expression.
    fn layout<'ast>(
        self,
        f: &mut DestackFormatter<'ast, '_>,
        is_left_short: bool,
        left_may_break: bool,
    ) -> FormatResult<AssignmentLikeLayout> {
        let right = self
            .right(f.context())
            .expect("assignment-like layout requires a right-hand side");

        // assignment chains
        if let Some(layout) = self.chain_layout(f.context()) {
            return Ok(layout);
        }

        // left side pressure
        if self.should_break_left_hand_side(f.context(), left_may_break) {
            return Ok(AssignmentLikeLayout::BreakLeftHandSide);
        }

        // operator-bound trivia and rhs pressure
        if self.should_break_after_operator(f, is_left_short)? {
            return Ok(AssignmentLikeLayout::BreakAfterOperator);
        }

        // operator-bound line comments keep the rhs on the next line
        if assignment_rhs_has_inline_operator_prefix_comment(f.context(), right) {
            return Ok(AssignmentLikeLayout::BreakAfterOperator);
        }

        // compact rhs
        match self {
            // declarator compact rhs
            AssignmentLike::Declarator(declarator_id) => {
                let context = f.context();
                let declarator = context.tree.get(declarator_id);
                let value_inner_id = transparent_inner_expression(context, right);
                let value_inner_expr = context.tree.get(value_inner_id);

                let rhs_is_template_expression =
                    matches!(value_inner_expr, Expression::TemplateExpression { .. });
                let rhs_is_keyword_expression = matches!(
                    context.tree.get(right),
                    Expression::Await { .. }
                        | Expression::AwaitMaybe { .. }
                        | Expression::AwaitMust { .. }
                        | Expression::Comptime { .. }
                );
                let rhs_is_class_declaration =
                    expression_is_class_declaration(context, value_inner_id);
                let rhs_has_generic_class_heritage =
                    declarator_value_has_generic_class_heritage(context, value_inner_id);
                let rhs_has_inline_operator_prefix_comment =
                    assignment_rhs_has_inline_operator_prefix_comment(context, right);
                let rhs_has_prefix_annotation_that_forces_break =
                    context.has_prefix_annotation(right) && !rhs_has_inline_operator_prefix_comment;

                let header_end = declarator
                    .ty
                    .map(|type_id| context.span(type_id).end)
                    .unwrap_or(context.span(declarator.pattern).end);
                let value_span = context.span(right);
                let between_span = if header_end < value_span.start {
                    Some(Span::new(value_span.file, header_end, value_span.start))
                } else {
                    None
                };
                let rhs_has_between_comment = between_span.is_some_and(|span| {
                    !context
                        .comment_tokens_in_range(span.start, span.end)
                        .is_empty()
                }) && !rhs_has_inline_operator_prefix_comment;
                let rhs_has_own_line_prefix_annotation =
                    assign_expression_has_own_line_prefix_annotation(context, right);

                let rhs_is_compact_keyword = rhs_is_keyword_expression
                    && !rhs_has_between_comment
                    && !rhs_has_prefix_annotation_that_forces_break;
                let rhs_is_compact_class = rhs_is_class_declaration
                    && !rhs_has_generic_class_heritage
                    && !rhs_has_prefix_annotation_that_forces_break
                    && !rhs_has_between_comment
                    && !rhs_has_own_line_prefix_annotation;
                if (rhs_is_template_expression || rhs_is_compact_keyword || rhs_is_compact_class)
                    && !rhs_has_inline_operator_prefix_comment
                {
                    return Ok(AssignmentLikeLayout::NeverBreakAfterOperator);
                }

                if !left_may_break
                    && (is_left_short || rhs_is_template_expression || rhs_is_class_declaration)
                    && !rhs_has_inline_operator_prefix_comment
                {
                    return Ok(AssignmentLikeLayout::NeverBreakAfterOperator);
                }
            }

            // assignment-expression compact rhs
            AssignmentLike::Expression { .. } => {
                let context = f.context();
                let rhs_has_inline_operator_prefix_comment =
                    assignment_rhs_has_inline_operator_prefix_comment(context, right);

                if !left_may_break
                    && (is_left_short || assignment_rhs_is_compact(context, right))
                    && !rhs_has_inline_operator_prefix_comment
                {
                    return Ok(AssignmentLikeLayout::NeverBreakAfterOperator);
                }
            }
        }

        Ok(AssignmentLikeLayout::Fluid)
    }

    /// Write the operator for one assignment-like expression.
    fn write_operator<'ast>(self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            AssignmentLike::Declarator(_) => write!(f, [space(), token("=")]),
            AssignmentLike::Expression { left, operator, .. } => {
                let has_left_postfix = assign_pattern_target_expression(f.context(), left)
                    .is_some_and(|left_expression_id| {
                        f.context().has_postfix_annotation(left_expression_id)
                    });

                if !has_left_postfix {
                    write!(f, [space()])?;
                }

                write!(f, [operator])
            }
        }
    }

    /// Write the right-hand side for one assignment-like expression.
    fn write_right<'ast>(
        self,
        f: &mut DestackFormatter<'ast, '_>,
        layout: AssignmentLikeLayout,
    ) -> FormatResult<()> {
        match self {
            AssignmentLike::Declarator(declarator_id) => {
                let declarator = f.context().tree.get(declarator_id);
                let value = declarator
                    .value
                    .expect("declarator rhs requires an initializer");

                let rhs_operator_comment_nodes =
                    assignment_rhs_operator_comment_nodes(f.context(), value);
                let rhs_has_inline_operator_prefix_comment =
                    assignment_rhs_has_inline_operator_prefix_comment(f.context(), value);
                let has_inline_line_operator_comment = rhs_has_inline_operator_prefix_comment
                    && rhs_operator_comment_nodes
                        .first()
                        .is_some_and(|comment| comment.is_line());

                if has_inline_line_operator_comment {
                    let first_comment = rhs_operator_comment_nodes[0];
                    let formatted_right = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write_declarator_assignment_value(f, value, layout, true, &[])
                    });
                    let indented_right = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(f, [hard_line_break(), formatted_right])
                    });

                    write!(f, [space()])?;
                    format_comment(f, first_comment)?;

                    return write!(f, [indent(&indented_right)]);
                }

                let formatted_right = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    write_assignment_rhs_operator_comments(f, value, true)?;
                    write_declarator_assignment_value(
                        f,
                        value,
                        layout,
                        rhs_has_inline_operator_prefix_comment,
                        &rhs_operator_comment_nodes,
                    )
                });

                write_assignment_like_right(f, layout, &formatted_right)
            }
            AssignmentLike::Expression { right, .. } => {
                let rhs_operator_comment_nodes =
                    assignment_rhs_operator_comment_nodes(f.context(), right);
                let rhs_has_inline_operator_prefix_comment =
                    assignment_rhs_has_inline_operator_prefix_comment(f.context(), right);
                let has_inline_line_operator_comment = rhs_has_inline_operator_prefix_comment
                    && rhs_operator_comment_nodes
                        .first()
                        .is_some_and(|comment| comment.is_line());

                if has_inline_line_operator_comment {
                    let first_comment = rhs_operator_comment_nodes[0];
                    let formatted_right = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write_expression_with_assignment_layout(
                            f,
                            right,
                            layout,
                            rhs_has_inline_operator_prefix_comment,
                        )
                    });
                    let indented_right = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(f, [hard_line_break(), formatted_right])
                    });

                    write!(f, [space()])?;
                    format_comment(f, first_comment)?;

                    return write!(f, [indent(&indented_right)]);
                }

                let formatted_right = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    if !rhs_operator_comment_nodes.is_empty() {
                        write_assignment_rhs_operator_comments(f, right, true)?;
                    }

                    write_expression_with_assignment_layout(
                        f,
                        right,
                        layout,
                        rhs_has_inline_operator_prefix_comment,
                    )
                });

                write_assignment_like_right(f, layout, &formatted_right)
            }
        }
    }

    /// Return one explicit chain layout when this expression is eligible.
    fn chain_layout(self, context: &DestackFormatContext<'_>) -> Option<AssignmentLikeLayout> {
        let AssignmentLike::Expression { node_id, right, .. } = self else {
            return None;
        };

        let right = transparent_inner_expression(context, right);
        let right_is_tail = !matches!(context.tree.get(right), Expression::Assign { .. });
        let (parent_type, parent_id) = assignment_like_parent(context, node_id)?;

        // eligible chain layouts
        let is_eligible = match parent_type {
            NodeType::Declarator => !right_is_tail,
            NodeType::Expression => {
                let parent_id = LocalNodeId::<Expression>::new(parent_id);
                if !matches!(context.tree.get(parent_id), Expression::Assign { .. }) {
                    return None;
                }

                !right_is_tail || assignment_like_parent(context, parent_id).is_some()
            }
            _ => false,
        };
        if !is_eligible {
            return None;
        }

        // tail layout
        if right_is_tail {
            if is_assignment_chain_tail_lambda(context, node_id, right) {
                return Some(AssignmentLikeLayout::ChainTailArrowFunction);
            }

            return Some(AssignmentLikeLayout::ChainTail);
        }

        Some(AssignmentLikeLayout::Chain)
    }

    /// Return whether the left side should break before the operator.
    fn should_break_left_hand_side(
        self,
        context: &DestackFormatContext<'_>,
        left_may_break: bool,
    ) -> bool {
        match self {
            // declarator lhs shape
            AssignmentLike::Declarator(declarator_id) => {
                let declarator = context.tree.get(declarator_id);
                let pattern = declarator.pattern;
                let pattern_span = context.span(pattern);

                let pattern_has_newline = context.has_newline(pattern_span);
                let pattern_has_comments_or_annotations = context.has_annotation(pattern)
                    || !context
                        .comment_tokens_in_range(pattern_span.start, pattern_span.end)
                        .is_empty();
                if pattern_has_newline
                    || pattern_has_comments_or_annotations
                    || declarator_pattern_is_complex_destructuring(context, pattern)
                {
                    return true;
                }

                let Some(value) = declarator.value else {
                    return false;
                };

                left_may_break && declarator_value_is_lambda_like(context, value)
            }

            // assignment target shape
            AssignmentLike::Expression { left, .. } => {
                assignment_target_is_complex_destructuring(context, left)
            }
        }
    }

    /// Return whether the operator should break before the rhs.
    fn should_break_after_operator<'ast>(
        self,
        f: &mut DestackFormatter<'ast, '_>,
        is_left_short: bool,
    ) -> FormatResult<bool> {
        let context = f.context();
        let right = self
            .right(context)
            .expect("assignment-like break check requires a right-hand side");

        match self {
            // declarator rhs pressure
            AssignmentLike::Declarator(declarator_id) => {
                let declarator = context.tree.get(declarator_id);
                let pattern = declarator.pattern;
                let type_id = declarator.ty;
                let value_span = context.span(right);
                let value_inner_id = transparent_inner_expression(context, right);
                let value_inner_expr = context.tree.get(value_inner_id);

                let rhs_operator_comment_nodes =
                    assignment_rhs_operator_comment_nodes(context, right);
                let rhs_has_inline_operator_prefix_comment =
                    assignment_rhs_has_inline_operator_prefix_comment(context, right);
                let rhs_has_prefix_annotation_that_forces_break =
                    context.has_prefix_annotation(right) && !rhs_has_inline_operator_prefix_comment;
                let rhs_has_own_line_prefix_annotation =
                    assign_expression_has_own_line_prefix_annotation(context, right);

                let header_end = type_id
                    .map(|type_id| context.span(type_id).end)
                    .unwrap_or(context.span(pattern).end);
                let between_span = if header_end < value_span.start {
                    Some(Span::new(value_span.file, header_end, value_span.start))
                } else {
                    None
                };
                let rhs_has_between_comment = between_span.is_some_and(|span| {
                    !context
                        .comment_tokens_in_range(span.start, span.end)
                        .is_empty()
                }) && !rhs_has_inline_operator_prefix_comment;

                let rhs_is_call_like = matches!(
                    value_inner_expr,
                    Expression::Call { .. }
                        | Expression::New { .. }
                        | Expression::Instantiation { .. }
                );
                let rhs_has_generic_class_heritage =
                    declarator_value_has_generic_class_heritage(context, value_inner_id);
                let pattern_has_default_assignment =
                    declarator_pattern_has_default_assignment(context, pattern);

                Ok(!rhs_operator_comment_nodes.is_empty()
                    || rhs_has_prefix_annotation_that_forces_break
                    || rhs_has_between_comment
                    || rhs_has_own_line_prefix_annotation
                    || rhs_has_generic_class_heritage
                    || (rhs_is_call_like && pattern_has_default_assignment)
                    || assignment_rhs_prefers_break_after_operator(f, right, is_left_short)?)
            }

            // assignment-expression rhs pressure
            AssignmentLike::Expression { left, .. } => Ok(
                assignment_rhs_has_inline_operator_prefix_comment(context, right)
                    || assignment_operator_has_line_comment_between(context, left, right)
                    || assignment_expression_rhs_has_forcing_leading_trivia(context, left, right)
                    || assignment_rhs_prefers_break_after_operator(f, right, is_left_short)?,
            ),
        }
    }

    /// Format one assignment-like expression.
    fn format<'ast>(self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        // left side only
        let Some(_) = self.right(f.context()) else {
            let (left_nodes, _, _) = self.buffer_left(f)?;
            let left_nodes = f.intern_vec(left_nodes);

            if let Some(left_nodes) = left_nodes {
                f.write_node(left_nodes);
            }

            return Ok(());
        };

        // buffered left side
        let (left_nodes, is_left_short, left_may_break) = self.buffer_left(f)?;
        let layout = self.layout(f, is_left_short, left_may_break)?;
        let left_nodes = f.intern_vec(left_nodes);
        let formatted_left = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            if let Some(left_nodes) = &left_nodes {
                f.write_node(left_nodes.clone());
            }

            Ok(())
        });

        // content
        let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if layout == AssignmentLikeLayout::BreakLeftHandSide {
                write!(f, [formatted_left])?;
            } else {
                write!(f, [group(&formatted_left)])?;
            }

            self.write_operator(f)?;
            self.write_right(f, layout)
        });

        match layout {
            AssignmentLikeLayout::Chain
            | AssignmentLikeLayout::ChainTail
            | AssignmentLikeLayout::ChainTailArrowFunction => write!(f, [content]),
            _ => write!(f, [group(&content)]),
        }
    }
}

/// Write the right-hand side for one assignment-like layout.
pub(crate) fn write_assignment_like_right<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    layout: AssignmentLikeLayout,
    right: &impl Format<DestackFormatContext<'ast>>,
) -> FormatResult<()> {
    match layout {
        AssignmentLikeLayout::Fluid => {
            let group_id = f.group_id("assignment_like");

            write!(
                f,
                [
                    group(&indent(&soft_line_break_or_space())).with_id(Some(group_id)),
                    line_suffix_boundary(),
                    indent_if_group_breaks(right, group_id)
                ]
            )
        }
        AssignmentLikeLayout::BreakAfterOperator => {
            write!(f, [group(&soft_line_indent_or_space(right))])
        }
        AssignmentLikeLayout::NeverBreakAfterOperator => write!(f, [space(), right]),
        AssignmentLikeLayout::BreakLeftHandSide => write!(f, [space(), group(right)]),
        AssignmentLikeLayout::Chain => write!(f, [soft_line_break_or_space(), right]),
        AssignmentLikeLayout::ChainTail => write!(f, [soft_line_indent_or_space(right)]),
        AssignmentLikeLayout::ChainTailArrowFunction => write!(f, [space(), right]),
    }
}

/// Return whether one rhs shape should prefer breaking after the operator.
pub(crate) fn assignment_rhs_prefers_break_after_operator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    right: LocalNodeId<Expression>,
    is_left_short: bool,
) -> FormatResult<bool> {
    let context = f.context();
    let right = transparent_inner_expression(context, right);

    // comments
    for comment in context
        .comments()
        .comments_before_iter(context.span(right).start)
    {
        if comment.preceded_by_newline() && comment.followed_by_newline() {
            return Ok(true);
        }
    }

    let should_break = match context.tree.get(right) {
        // assignment chains break after `=`
        Expression::Assign {
            right: nested_right,
            ..
        } => matches!(
            context
                .tree
                .get(transparent_inner_expression(context, *nested_right)),
            Expression::Assign { .. }
        ),

        // binary-like rhs values first break after `=`
        Expression::Binary { .. } | Expression::SequenceExpression { .. } => true,

        // ternary rhs values only break after `=` when the test is binary-like
        Expression::If {
            form: IfForm::Ternary,
            condition,
            ..
        } => match condition {
            IfCondition::Expression { condition } => {
                let condition = transparent_inner_expression(context, *condition);

                match context.tree.get(condition) {
                    Expression::Binary {
                        operator, right, ..
                    } => {
                        if !is_logical_binary_operator(*operator) {
                            return Ok(true);
                        }

                        let logical_right = transparent_inner_expression(context, *right);
                        let right_stays_inline = match context.tree.get(logical_right) {
                            Expression::ObjectExpression { properties, .. }
                            | Expression::StructExpression { properties, .. } => {
                                !properties.is_empty()
                            }
                            Expression::ArrayExpression { elements } => !elements.is_empty(),
                            Expression::TreeExpression { .. } => true,
                            _ => false,
                        };

                        !right_stays_inline
                    }
                    _ => false,
                }
            }
            IfCondition::Let { .. } => false,
        },

        _ if matches!(
            assignment_rhs_innermost_expression(context, right),
            Expression::ScalarLiteral(ScalarLiteral::String(_))
        ) =>
        {
            true
        }
        _ if is_left_short => false,
        _ => is_poorly_breakable_member_or_call_chain(f, right)?,
    };

    Ok(should_break)
}

/// Return the innermost rhs expression after unwrapping unary-like expressions.
fn assignment_rhs_innermost_expression<'a>(
    context: &'a DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> &'a Expression {
    let mut current_expression_id = expression_id;

    loop {
        current_expression_id = match context.tree.get(current_expression_id) {
            Expression::Unary { right, .. } => *right,
            Expression::Await { expression }
            | Expression::AwaitMaybe { expression }
            | Expression::AwaitMust { expression } => *expression,
            Expression::Yield {
                value: Some(value), ..
            } => *value,
            Expression::Must { left, .. } => *left,
            _ => break,
        };
    }

    context.tree.get(current_expression_id)
}

/// Return whether one assignment target is complex enough to break the left side.
fn assignment_target_is_complex_destructuring(
    context: &DestackFormatContext<'_>,
    left: LocalNodeId<AssignPattern>,
) -> bool {
    let fields = match context.tree.get(left) {
        // default wrappers
        AssignPattern::Assign { pattern, .. } => {
            return assignment_target_is_complex_destructuring(context, *pattern);
        }

        // object destructuring
        AssignPattern::Object { fields } => fields,

        // non-object targets
        AssignPattern::Expression { .. } | AssignPattern::Sequence { .. } => {
            return false;
        }
    };

    if fields.len() <= 2 {
        return false;
    }

    fields
        .iter()
        .copied()
        .any(|field_id| match context.tree.get(field_id) {
            // expanded fields
            AssignPatternField::Named {
                is_shorthand,
                pattern,
                ..
            } => !is_shorthand || pattern.is_some(),

            // computed keys
            AssignPatternField::Computed { .. } => true,

            // flat fields
            AssignPatternField::Positional { .. }
            | AssignPatternField::Spread { .. }
            | AssignPatternField::Elision => false,
        })
}

/// Return whether one rhs expression stays attached to the assignment operator.
fn assignment_rhs_is_compact(
    context: &DestackFormatContext<'_>,
    right: LocalNodeId<Expression>,
) -> bool {
    let right = transparent_inner_expression(context, right);
    let right_expression = context.tree.get(right);

    matches!(
        right_expression,
        Expression::ScalarLiteral(
            ScalarLiteral::Boolean(_) | ScalarLiteral::Integer(_) | ScalarLiteral::Float(_)
        ) | Expression::TemplateExpression { .. }
            | Expression::TaggedTemplateExpression { .. }
    ) || expression_is_class_declaration(context, right)
}

/// Return whether one assignment rhs carries leading trivia that forces break-after-operator.
fn assignment_expression_rhs_has_forcing_leading_trivia(
    context: &DestackFormatContext<'_>,
    left: LocalNodeId<AssignPattern>,
    right: LocalNodeId<Expression>,
) -> bool {
    // annotations
    let has_prefix_annotation = context.has_prefix_annotation(right);
    let has_own_line_prefix_annotation =
        assign_expression_has_own_line_prefix_annotation(context, right);

    // between comments
    let left_span = context.span(left);
    let right_span = context.span(right);
    let has_between_comment = left_span
        .gap_to(right_span)
        .is_some_and(|between_span| context.has_own_line_or_multiline_comment(between_span));

    has_prefix_annotation || has_between_comment || has_own_line_prefix_annotation
}

/// Format an assignment expression with one selected layout.
pub(crate) fn format_assign_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    left: LocalNodeId<AssignPattern>,
    operator: &AssignOperator,
    right: LocalNodeId<Expression>,
) -> FormatResult<()> {
    AssignmentLike::Expression {
        node_id,
        left,
        operator: *operator,
        right,
    }
    .format(f)
}
