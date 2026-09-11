use super::conditional::ConditionalLayout;
use crate::annotation::{
    FormatLeadingComments, FormatTrailingComments, format_node_with_trailing_comments,
    format_trailing_comments, infix_or_postfix_annotations, prefix_annotations,
    prefix_annotations_without_comments,
};
use crate::collection::literal::format_scalar_literal;
use crate::collection::{FormatSeparatedIter, TrailingSeparator, separated_entries};
use crate::declaration::signature::{
    ParameterList, default_generic_parameter_trailing_separator, format_where_clause,
    parameter_is_variadic, should_hug_function_parameters, write_function_abstraction_prefix,
    write_function_header_prefix, write_generic_parameter_list,
    write_grouped_parameters_with_return_type, write_signature_hug_parameter_list_with_this,
    write_signature_parameter_list_with_this, write_signature_return_type,
};
use crate::declaration::{write_access_prefix, write_visibility_prefix};
use crate::expression::format_type_template_literal;
use crate::file::{
    ignore_ranges_for_nodes, node_has_ignore_directive, node_has_trailing_line_ignore_directive,
    write_ignored_node, write_source_span,
};
use crate::operator::{
    format_generic_argument_list, write_colon_prefixed_type_annotation, write_range_operator,
    write_type_annotation_prefix, write_type_expression_with_inline_prefix_annotations,
};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_core::ensure_sufficient_stack;
use destack_dir::{
    Comment, ConstructorType, Declaration, Expression, FunctionForm, FunctionSignature,
    FunctionTypeExpression, GenericArgument, GenericParameter, InferForm, Keyword, LocalNodeId,
    MappedTypeModifier, Member, Mutability, Name, Node, NodeType, Parameter, Property, RangeEnd,
    TokenSpan, TokenType, Tree, TreeStore, TupleElement, TupleForm, TypeExpression, TypeLiteral,
    TypeMappedParameter, TypeMember, VarianceBound, WhereClause,
};
use destack_fir::format::{FormatElement as FirElement, FormatError, FormatLayout, FormatResult};
use destack_fir::prelude::{space, token, *};
use destack_fir::{format_args, write};
use destack_repository::TrailingComma;
use destack_source::{NodeSpanBoundary, NodeSpanRegion, NodeSpanType, Span};

/// Return the innermost type that can own one postfix type operator.
fn normalize_postfix_type_operand(
    context: &DestackFormatContext<'_>,
    mut expression_id: LocalNodeId<TypeExpression>,
) -> LocalNodeId<TypeExpression> {
    loop {
        // single member unions and intersections are transparent here
        match context.tree.get(expression_id) {
            TypeExpression::Union { elements } | TypeExpression::Intersection { elements }
                if elements.len() == 1 =>
            {
                expression_id = elements[0];
            }
            _ => return expression_id,
        }
    }
}

/// Return whether one normalized type needs parentheses before postfix operators.
fn type_needs_postfix_parentheses(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(expression_id) {
        TypeExpression::Union { elements } => elements.len() > 1,
        TypeExpression::Intersection { elements } => elements.len() > 1,
        TypeExpression::Conditional { .. }
        | TypeExpression::Extends { .. }
        | TypeExpression::Implements { .. }
        | TypeExpression::Mapped { .. }
        | TypeExpression::Readonly { .. }
        | TypeExpression::Local { .. }
        | TypeExpression::Shared { .. }
        | TypeExpression::KeyOf { .. }
        | TypeExpression::TypeOf { .. }
        | TypeExpression::Must { .. }
        | TypeExpression::Range { .. }
        | TypeExpression::Not { .. }
        | TypeExpression::OwnedOf { .. }
        | TypeExpression::BorrowedOf { .. }
        | TypeExpression::PointerOf { .. }
        | TypeExpression::Infer { .. }
        | TypeExpression::Function(_)
        | TypeExpression::Constructor(_) => true,
        _ => false,
    }
}

/// Return whether one indexed-access object type needs parentheses.
fn type_needs_index_object_parentheses(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(expression_id) {
        TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
            elements.len() > 1
        }
        TypeExpression::Conditional { .. }
        | TypeExpression::Extends { .. }
        | TypeExpression::Implements { .. }
        | TypeExpression::Readonly { .. }
        | TypeExpression::Local { .. }
        | TypeExpression::Shared { .. }
        | TypeExpression::KeyOf { .. }
        | TypeExpression::TypeOf { .. }
        | TypeExpression::Must { .. }
        | TypeExpression::Not { .. }
        | TypeExpression::OwnedOf { .. }
        | TypeExpression::BorrowedOf { .. }
        | TypeExpression::PointerOf { .. }
        | TypeExpression::Infer { .. }
        | TypeExpression::Function(_)
        | TypeExpression::Constructor(_) => true,
        _ => false,
    }
}

/// Leading comments relevant to union layout.
#[derive(Debug, Clone, Copy, Default)]
struct LeadingComments {
    /// Whether any leading comments exist.
    has_comments: bool,
    /// Whether a comment begins on its own line.
    has_own_line_comment: bool,
    /// Whether a comment ends its line.
    has_end_of_line_comment: bool,
    /// Whether a trailing block comment occupies its own line.
    has_trailing_own_line_block_comment: bool,
    /// Whether documentation ends its line.
    has_documentation_line_break: bool,
}

/// The object type body layout for type expression formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectTypeBodyLayout {
    /// Use the object type's local body rules.
    Local,
    /// Use grouped object body formatting.
    Grouped,
}

/// One type expression layout.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TypeExpressionLayout {
    /// The derived-parentheses policy.
    derived_parentheses: DerivedParentheses,
    /// How object type bodies should be formatted.
    object_body: ObjectTypeBodyLayout,
}

impl TypeExpressionLayout {
    /// The standard type expression layout.
    pub(crate) const DEFAULT: Self = Self {
        derived_parentheses: DerivedParentheses::Allowed,
        object_body: ObjectTypeBodyLayout::Local,
    };

    /// The standard layout without derived parentheses.
    const WITHOUT_DERIVED_PARENTHESES: Self = Self {
        derived_parentheses: DerivedParentheses::Suppressed,
        object_body: ObjectTypeBodyLayout::Local,
    };

    /// Return this layout with grouped object type bodies.
    const fn with_grouped_object_body(self) -> Self {
        Self {
            object_body: ObjectTypeBodyLayout::Grouped,
            ..self
        }
    }
}

/// The derived-parentheses policy for a type expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DerivedParentheses {
    /// Emit derived parentheses when precedence requires them.
    Allowed,
    /// Do not emit derived parentheses.
    Suppressed,
}

impl DerivedParentheses {
    /// Return whether derived parentheses may be emitted.
    const fn is_allowed(self) -> bool {
        matches!(self, Self::Allowed)
    }
}

impl LeadingComments {
    /// Classify one sequence of leading comments.
    fn from_comments(comments: &[Comment]) -> Self {
        let mut leading = Self {
            has_comments: !comments.is_empty(),
            ..Self::default()
        };

        // classify every comment
        for comment in comments.iter().copied() {
            leading.has_own_line_comment |= comment.preceded_by_newline();
            leading.has_end_of_line_comment |= comment.followed_by_newline();
            leading.has_trailing_own_line_block_comment |= comment.is_block()
                && comment.is_trailing()
                && comment.followed_by_newline()
                && !comment.is_documentation();
            leading.has_documentation_line_break |=
                comment.is_documentation() && comment.followed_by_newline();
        }

        leading
    }
}

/// Return the content start for one type expression.
fn type_expression_content_start(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
) -> u32 {
    context
        .tree
        .get_head_span(node_id)
        .map_or_else(|| context.span(node_id).start, |span| span.start)
}

/// Write positional leading comments that belong directly before one type node.
pub(crate) fn write_type_expression_leading_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    expression: &TypeExpression,
) -> FormatResult<()> {
    if type_expression_body_owns_leading_comments(expression) {
        return Ok(());
    }

    // union containers own comments before the first arm
    if let Some((parent_id, parent_type)) = f.context().parent(node_id)
        && parent_type == NodeType::TypeExpression
    {
        let parent_id = LocalNodeId::<TypeExpression>::new(parent_id);

        if let TypeExpression::Union { elements } = f.context().tree.get(parent_id)
            && elements.first().copied() == Some(node_id)
        {
            return Ok(());
        }
    }

    let token_start = f.context().node_token_start(node_id);
    let leading_span = f
        .context()
        .tree
        .get_side_span(node_id, NodeSpanType::Boundary(NodeSpanBoundary::Leading));
    let comments = {
        let comments = f.context().comments();

        // node leading range
        if let Some(leading_span) = leading_span {
            comments
                .comments_in_range(leading_span.start, token_start)
                .to_vec()
        } else {
            comments.comments_before(token_start).to_vec()
        }
    };

    if comments.is_empty() {
        return Ok(());
    }

    write!(f, [FormatLeadingComments::Comments(&comments)])
}

/// Return the conditional layout for one type expression.
fn type_conditional_layout(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
) -> ConditionalLayout {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return ConditionalLayout::Root;
    };

    if parent_type != NodeType::TypeExpression {
        return ConditionalLayout::Root;
    }

    let parent_id = LocalNodeId::<TypeExpression>::new(parent_id);
    let TypeExpression::Conditional {
        left,
        extends_type,
        then_type,
        else_type,
    } = context.tree.get(parent_id)
    else {
        return ConditionalLayout::Root;
    };

    if *left == node_id || *extends_type == node_id {
        ConditionalLayout::NestedTest
    } else if *then_type == node_id {
        ConditionalLayout::NestedConsequent
    } else if *else_type == node_id {
        ConditionalLayout::NestedAlternate
    } else {
        ConditionalLayout::Root
    }
}

/// Return whether one type expression is a conditional.
fn type_expression_is_conditional(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> bool {
    matches!(
        context.tree.get(expression_id),
        TypeExpression::Conditional { .. }
    )
}

/// Return the source anchor for trailing comments after one type expression.
fn type_expression_trailing_anchor_end(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> u32 {
    let span = context.span(expression_id);

    context
        .last_token_in_span(span)
        .map_or(span.end, |token| token.span.end)
}

/// Return conditional comments that trail one branch before the next operator.
fn type_conditional_trailing_comments(
    context: &DestackFormatContext<'_>,
    mut start: u32,
    end: u32,
    operator: u8,
) -> Vec<Comment> {
    let comments = context.comments().unprinted_comments();
    if comments.is_empty() {
        return Vec::new();
    }

    let source = context.source_text();
    for (index, comment) in comments.iter().copied().enumerate() {
        if comment.span.end > end {
            return comments[..index].to_vec();
        }

        if source.contains_newline_between(start, comment.span.start) {
            return comments[..index].to_vec();
        } else if source.bytes_contain(start, comment.span.start, operator) {
            if comment.is_line() || comment.followed_by_newline() {
                return comments[..=index].to_vec();
            }

            return comments[..index].to_vec();
        } else if comment.is_line() || comment.followed_by_newline() {
            return comments[..=index].to_vec();
        }

        start = comment.span.end;
    }

    comments.to_vec()
}

/// Write the test layout of one conditional type.
fn write_type_conditional_test<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    layout: ConditionalLayout,
    left: LocalNodeId<TypeExpression>,
    extends_type: LocalNodeId<TypeExpression>,
    then_type: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let format_test = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [left, space(), Keyword::Extends, space(), extends_type])?;

        let trailing_comments = type_conditional_trailing_comments(
            f.context(),
            type_expression_trailing_anchor_end(f.context(), extends_type),
            f.context().node_token_start(then_type),
            b'?',
        );

        if !trailing_comments.is_empty() {
            write!(f, [FormatTrailingComments::Comments(&trailing_comments)])?;
        }

        Ok(())
    });

    if layout.is_nested_alternate() {
        let comments = f
            .context()
            .comments()
            .comments_before(f.context().span(node_id).start);
        if !comments.is_empty() {
            write!(f, [FormatLeadingComments::Comments(comments)])?;
        }

        write!(f, [align(2, &format_test)])?;
    } else {
        write!(f, [format_test])?;
    }

    Ok(())
}

/// Write the `? ... : ...` tail of one conditional type.
fn write_type_conditional_tail<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    then_type: LocalNodeId<TypeExpression>,
    else_type: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let format_then_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let then_leading_comments = f.context().comments_after_previous_token(then_type);

        if !then_leading_comments.is_empty() {
            write!(f, [FormatLeadingComments::Comments(&then_leading_comments)])?;
        }

        write_type_expression_without_prefix_annotations(f, then_type)?;

        let trailing_comments = type_conditional_trailing_comments(
            f.context(),
            type_expression_trailing_anchor_end(f.context(), then_type),
            f.context().node_token_start(else_type),
            b':',
        );

        if !trailing_comments.is_empty() {
            write!(f, [FormatTrailingComments::Comments(&trailing_comments)])?;
        }

        Ok(())
    });

    let format_then_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let format_then_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if f.options().indent_style.is_space() {
                write!(f, [align(2, &format_then_type)])?;
            } else {
                write!(f, [indent(&format_then_type)])?;
            }

            Ok(())
        });

        if type_expression_is_conditional(f.context(), then_type) {
            write!(
                f,
                [
                    if_group_fits_on_line(&token("(")),
                    format_then_type,
                    if_group_fits_on_line(&token(")"))
                ]
            )?;
        } else {
            write!(f, [format_then_type])?;
        }

        Ok(())
    });

    let format_else_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if !type_expression_is_conditional(f.context(), else_type) {
            let else_leading_comments = f.context().comments_after_previous_token(else_type);

            if !else_leading_comments.is_empty() {
                write!(f, [FormatLeadingComments::Comments(&else_leading_comments)])?;
            }
        }

        write_type_expression_without_prefix_annotations(f, else_type)
    });

    let format_else_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if f.options().indent_style.is_space() {
            write!(f, [align(2, &format_else_type)])?;
        } else {
            write!(f, [indent(&format_else_type)])?;
        }

        Ok(())
    });

    write!(
        f,
        [format_args![
            soft_line_break_or_space(),
            token("?"),
            space(),
            format_then_type,
            soft_line_break_or_space(),
            token(":"),
            space(),
            format_else_type
        ]]
    )?;

    Ok(())
}

/// Format one conditional type with the nested layout rules.
fn write_conditional_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    left: LocalNodeId<TypeExpression>,
    extends_type: LocalNodeId<TypeExpression>,
    then_type: LocalNodeId<TypeExpression>,
    else_type: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let layout = type_conditional_layout(f.context(), node_id);

    let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_type_conditional_test(f, node_id, layout, left, extends_type, then_type)?;

        let format_tail = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write_type_conditional_tail(f, then_type, else_type)
        });

        match layout {
            ConditionalLayout::Root | ConditionalLayout::NestedTest => {
                write!(f, [indent(&format_tail)])?;
            }
            ConditionalLayout::NestedConsequent => {
                // remove the parent consequent alignment before applying one tab indent
                write!(f, [dedent(&indent(&format_tail))])?;
            }
            ConditionalLayout::NestedAlternate => {
                write!(f, [format_tail])?;
            }
        }

        Ok(())
    });

    let grouped = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if layout.groups_at_root() {
            write!(f, [group(&format_inner)])?;
        } else {
            write!(f, [format_inner])?;
        }

        Ok(())
    });

    if layout.is_nested_test() {
        write!(f, [group(&soft_block_indent(&grouped))])?;
    } else {
        write!(f, [grouped])?;
    }

    Ok(())
}

/// Write one static boolean type relation.
fn write_type_relation(
    f: &mut DestackFormatter<'_, '_>,
    left: LocalNodeId<TypeExpression>,
    keyword: Keyword,
    right: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    write!(f, [left, space(), keyword, space(), right])
}

/// Return comments after one mapped opening brace.
fn mapped_type_leading_body_comments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    span: Span,
) -> Vec<Comment> {
    let comments = context.comments();

    if let Some(name_span) = context.tree.get_main_span(node_id)
        && comments.has_leading_own_line_comment(name_span.start)
    {
        return comments.comments_before(name_span.start).to_vec();
    }

    comments
        .comments_before_character(span.start, b'[')
        .to_vec()
}

/// Return whether one type is object-like for intersection layout.
fn intersection_type_is_object_like(expression: &TypeExpression) -> bool {
    matches!(
        expression,
        TypeExpression::Object { .. } | TypeExpression::Mapped { .. }
    )
}

/// Return the object body layout for object arms in one intersection.
fn intersection_object_body_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<TypeExpression>],
) -> FormatResult<ObjectTypeBodyLayout> {
    for element_id in elements.iter().copied() {
        let TypeExpression::Object { members } = f.context().tree.get(element_id) else {
            continue;
        };

        if prepare_inline_type_object(f, element_id, members, TypeExpressionLayout::DEFAULT)?
            .is_none()
        {
            return Ok(ObjectTypeBodyLayout::Grouped);
        }
    }

    Ok(ObjectTypeBodyLayout::Local)
}

/// Write one type expression with an explicit layout and no trailing comments.
fn write_type_expression_without_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    layout: TypeExpressionLayout,
) -> FormatResult<()> {
    if !f.context().comments().has_comments() {
        let expression = f.context().tree.get(node_id);

        return write_type_expression_node(f, node_id, expression, layout);
    }

    let node_end = f.context().span(node_id).end;
    let previous_limit = f
        .context_mut()
        .comments_mut()
        .limit_comments_up_to(node_end);
    let expression = f.context().tree.get(node_id);
    let result = write_type_expression_node(f, node_id, expression, layout);

    f.context_mut()
        .comments_mut()
        .restore_view_limit(previous_limit);

    result
}

/// Write one intersection member with generated-node-style trailing comments.
fn write_intersection_member<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    intersection_id: LocalNodeId<TypeExpression>,
    element_id: LocalNodeId<TypeExpression>,
    next_element_id: Option<LocalNodeId<TypeExpression>>,
    object_body: ObjectTypeBodyLayout,
) -> FormatResult<()> {
    let layout = match object_body {
        ObjectTypeBodyLayout::Local => TypeExpressionLayout::DEFAULT,
        ObjectTypeBodyLayout::Grouped => TypeExpressionLayout::DEFAULT.with_grouped_object_body(),
    };

    if let Some(next_element_id) = next_element_id {
        let enclosing_span = f.context().span(intersection_id);
        let element_span = f.context().span(element_id);
        let following_span_start = f.context().span(next_element_id).start;

        return write!(
            f,
            [
                format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                    write_type_expression_without_trailing_comments(f, element_id, layout)
                }),
                format_trailing_comments(enclosing_span, element_span, Some(following_span_start))
            ]
        );
    }

    write_type_expression_without_trailing_comments(f, element_id, layout)
}

/// Write one intersection type with object-chain layout.
fn write_intersection_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    elements: &[LocalNodeId<TypeExpression>],
) -> FormatResult<()> {
    // one annotated intersection keeps its explicit leading `&`
    if elements.len() == 1 && f.context().has_prefix_annotation(node_id) {
        return write!(f, [token("&"), elements[0]]);
    }

    let object_body = intersection_object_body_layout(f, elements)?;

    let format_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let last_index = elements.len().saturating_sub(1);
        let mut previous_is_object_like = false;
        let mut is_chain_indented = false;

        for (index, element_id) in elements.iter().copied().enumerate() {
            let next_element_id = elements.get(index + 1).copied();
            let element = f.context().tree.get(element_id);
            let is_object_like = intersection_type_is_object_like(element);

            // first element stays inline
            if index == 0 {
                write_intersection_member(f, node_id, element_id, next_element_id, object_body)?;
            }
            // non-object edges use the standard breakable layout
            else if !(previous_is_object_like || is_object_like)
                || f.context()
                    .comments()
                    .has_leading_own_line_comment(f.context().span(element_id).start)
            {
                let content = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                    write_intersection_member(f, node_id, element_id, next_element_id, object_body)
                });

                write!(f, [soft_line_indent_or_space(&content)])?;
            }
            // object-like chains stay tighter, with one indentation step on mixed chains
            else {
                write!(f, [space()])?;

                if !previous_is_object_like || !is_object_like {
                    is_chain_indented = index > 1;
                }

                if is_chain_indented {
                    let content = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                        write_intersection_member(
                            f,
                            node_id,
                            element_id,
                            next_element_id,
                            object_body,
                        )
                    });

                    write!(f, [indent(&content)])?;
                } else {
                    write_intersection_member(
                        f,
                        node_id,
                        element_id,
                        next_element_id,
                        object_body,
                    )?;
                }
            }

            if index < last_index {
                write!(f, [space(), token("&")])?;
            }

            previous_is_object_like = is_object_like;
        }

        Ok(())
    });

    write!(f, [group(&format_content)])
}

/// Return whether a type object body starts on its own line in source.
fn type_object_members_have_leading_newline(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    members: &[LocalNodeId<TypeMember>],
) -> bool {
    let Some(first_member_id) = members.first().copied() else {
        return false;
    };

    let object_span = context.span(node_id);
    let first_member_span = context.span(first_member_id);

    if object_span.file != first_member_span.file || object_span.start >= first_member_span.start {
        return false;
    }

    context.has_newline(Span::new(
        object_span.file,
        object_span.start,
        first_member_span.start,
    ))
}

/// Return whether one parameter directly owns a declared type.
fn parameter_declared_type_is(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
    type_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(parameter_id) {
        Parameter::Named { declared_type, .. }
        | Parameter::Pattern { declared_type, .. }
        | Parameter::VariadicNamed { declared_type, .. }
        | Parameter::VariadicPattern { declared_type, .. } => {
            declared_type.is_some_and(|declared_type| declared_type.id == type_id.id)
        }
        Parameter::Error => false,
    }
}

/// Return whether one parameter has a default expression.
fn parameter_has_default_value(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    match context.tree.get(parameter_id) {
        Parameter::Named { default, .. } | Parameter::Pattern { default, .. } => default.is_some(),
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } | Parameter::Error => {
            false
        }
    }
}

/// Return whether one callable should hug a parameter-owned object type.
fn parameters_should_hug_type(
    context: &DestackFormatContext<'_>,
    parameters: ParameterList,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    parameters
        .iter()
        .any(|current| current.id == parameter_id.id)
        && should_hug_function_parameters(context, &parameters, false)
}

/// Return whether one object type should use parameter hugging layout.
fn type_object_should_hug(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
) -> bool {
    let Some((parameter_parent_id, NodeType::Parameter)) = context.parent(node_id) else {
        return false;
    };
    let parameter_id = LocalNodeId::<Parameter>::new(parameter_parent_id);
    if !parameter_declared_type_is(context, parameter_id, node_id) {
        return false;
    }
    if parameter_has_default_value(context, parameter_id) {
        return false;
    }

    let Some((owner_id, owner_type)) = context.parent(parameter_id) else {
        return false;
    };

    match owner_type {
        NodeType::Declaration => {
            match context.tree.get(LocalNodeId::<Declaration>::new(owner_id)) {
                Declaration::Function(function) => {
                    let parameters = ParameterList::from_signature(&function.signature);
                    parameters_should_hug_type(context, parameters, parameter_id)
                }
                _ => false,
            }
        }
        NodeType::Property => match context.tree.get(LocalNodeId::<Property>::new(owner_id)) {
            Property::Method { signature, .. } => {
                let parameters = ParameterList::from_signature(signature);
                parameters_should_hug_type(context, parameters, parameter_id)
            }
            _ => false,
        },
        NodeType::Member => match context.tree.get(LocalNodeId::<Member>::new(owner_id)) {
            Member::Method { signature, .. } => {
                let parameters = ParameterList::from_signature(signature);
                parameters_should_hug_type(context, parameters, parameter_id)
            }
            _ => false,
        },
        NodeType::TypeExpression => {
            match context
                .tree
                .get(LocalNodeId::<TypeExpression>::new(owner_id))
            {
                TypeExpression::Function(function) => {
                    let parameters =
                        ParameterList::new(function.this_parameter, &function.parameters);
                    parameters_should_hug_type(context, parameters, parameter_id)
                }
                TypeExpression::Constructor(function) => {
                    let parameters = ParameterList::new(None, &function.parameters);
                    parameters_should_hug_type(context, parameters, parameter_id)
                }
                _ => false,
            }
        }
        NodeType::TypeMember => match context.tree.get(LocalNodeId::<TypeMember>::new(owner_id)) {
            TypeMember::Method { signature, .. } => {
                let parameters = ParameterList::from_signature(signature);
                parameters_should_hug_type(context, parameters, parameter_id)
            }
            TypeMember::CallSignature { signature } => {
                let parameters =
                    ParameterList::new(signature.this_parameter, &signature.parameters);
                parameters_should_hug_type(context, parameters, parameter_id)
            }
            TypeMember::ConstructSignature { signature } => {
                let parameters = ParameterList::new(None, &signature.parameters);
                parameters_should_hug_type(context, parameters, parameter_id)
            }
            _ => false,
        },
        _ => false,
    }
}

/// Return whether one object type is structurally eligible for direct inline formatting.
fn type_object_allows_inline_body(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    members: &[LocalNodeId<TypeMember>],
) -> bool {
    if members.len() != 1 {
        return false;
    }

    let span = context.span(node_id);
    if context
        .source_text()
        .has_newline_after_opening_brace(span.start)
    {
        return false;
    }

    if context.has_ignore_directive_markers() || context.comments().has_comment_in_span(span) {
        return false;
    }

    true
}

/// Prepare a direct inline object body when it is measurable and fits.
fn prepare_inline_type_object<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    members: &[LocalNodeId<TypeMember>],
    layout: TypeExpressionLayout,
) -> FormatResult<Option<FirElement<'ast>>> {
    if layout.object_body != ObjectTypeBodyLayout::Local {
        return Ok(None);
    }

    if !type_object_allows_inline_body(f.context(), node_id, members) {
        return Ok(None);
    }

    let content =
        format_with(|f: &mut DestackFormatter<'ast, '_>| write_inline_type_object(f, members[0]));

    let snapshot = f.context().comments().snapshot();
    let element = f.capture(&content);
    f.context_mut().comments_mut().restore(snapshot);
    let element = element?;

    let Some(element) = element else {
        return Ok(None);
    };

    let Some(width) = element.single_line_width() else {
        return Ok(None);
    };

    if width <= f.context().options.line_width as u32 {
        Ok(Some(element))
    } else {
        Ok(None)
    }
}

/// Write one direct inline object type body.
fn write_inline_type_object<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    member_id: LocalNodeId<TypeMember>,
) -> FormatResult<()> {
    write!(f, [token("{")])?;

    if f.context().options.bracket_spacing {
        write!(f, [space()])?;
    }

    write!(f, [member_id])?;

    if f.context().options.bracket_spacing {
        write!(f, [space()])?;
    }

    write!(f, [token("}")])
}

/// Return whether one union should stay inline when it fits.
fn union_should_hug(
    f: &DestackFormatter<'_, '_>,
    node_id: LocalNodeId<TypeExpression>,
    elements: &[LocalNodeId<TypeExpression>],
) -> bool {
    if elements.len() <= 1 {
        return true;
    }

    let mut has_object_type = false;
    let mut non_nullish_count = 0usize;

    // find the single non-nullish arm required by the hugging policy
    for element_id in elements.iter().copied() {
        match f.context().tree.get(element_id) {
            TypeExpression::Keyword {
                value: TypeLiteral::Void | TypeLiteral::Null,
            } => {}
            TypeExpression::Object { .. } | TypeExpression::Reference { .. } => {
                has_object_type = true;
                non_nullish_count += 1;
            }
            _ => {
                non_nullish_count += 1;
            }
        }

        if non_nullish_count > 1 {
            return false;
        }
    }

    // reject pure nullish unions
    if !has_object_type {
        return false;
    }

    let mut start = f.context().span(node_id).start;

    for element_id in elements.iter().copied() {
        let element_span = f.context().span(element_id);

        if f.context()
            .comments()
            .has_comment_in_range(start, element_span.start)
        {
            return false;
        }

        start = element_span.end;
    }

    true
}

/// Return whether one multiline union should own one extra indent.
fn union_should_indent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    leading_comments: LeadingComments,
) -> bool {
    let Some((parent_id, parent_type)) = union_indent_parent(context, node_id) else {
        return false;
    };

    match parent_type {
        NodeType::Declaration => {
            let declaration_id = LocalNodeId::<Declaration>::new(parent_id);

            match context.tree.get(declaration_id) {
                // type aliases have one comment-sensitive layout
                Declaration::Type(_) => {
                    type_alias_union_should_indent(context, declaration_id, leading_comments)
                }

                // other declarations follow the default union layout
                _ => true,
            }
        }

        // tuple elements already indent their body
        NodeType::TupleElement => false,

        // direct type arguments already indent their value wrapper
        NodeType::GenericArgument => false,

        // expression parents follow their own layout
        NodeType::Expression => {
            let expression_id = LocalNodeId::<Expression>::new(parent_id);

            union_expression_should_indent(context, expression_id)
        }

        // other parents use the default union layout
        _ => true,
    }
}

/// Return whether one type-alias union should keep its extra union indent.
fn type_alias_union_should_indent(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
    leading_comments: LeadingComments,
) -> bool {
    // keep documented union arms at the assignment indentation
    if leading_comments.has_documentation_line_break {
        return false;
    }

    let head_end = context
        .tree
        .get_side_span(
            declaration_id,
            NodeSpanType::Region(NodeSpanRegion::GenericParameters),
        )
        .or(context.tree.get_main_span(declaration_id))
        .unwrap_or_else(|| context.span(declaration_id))
        .end;

    !context
        .comments()
        .printed_comments()
        .last()
        .is_some_and(|comment| comment.span.start > head_end && comment.followed_by_newline())
}

/// Return the outer parent that decides one union indent.
fn union_indent_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
) -> Option<(u32, NodeType)> {
    let (parent_id, parent_type, _) = effective_type_parent(context, node_id)?;

    Some((parent_id, parent_type))
}

/// The head of one singleton union chain.
#[derive(Clone, Copy, Debug)]
struct UnionChainHead {
    /// The outer parent that owns the chain.
    parent: Option<(u32, NodeType)>,
    /// The number of elements at the chain head.
    element_count: usize,
}

/// Return the union node that carries the printable arms.
fn union_print_node(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    elements: &[LocalNodeId<TypeExpression>],
) -> LocalNodeId<TypeExpression> {
    let mut current_id = node_id;
    let mut current_elements = elements;

    loop {
        if current_elements.len() != 1 {
            return current_id;
        }

        let inner_id = current_elements[0];
        let TypeExpression::Union {
            elements: inner_elements,
        } = context.tree.get(inner_id)
        else {
            return current_id;
        };

        current_id = inner_id;
        current_elements = inner_elements;
    }
}

/// Return the head information for one union chain.
fn union_chain_head(
    context: &DestackFormatContext<'_>,
    mut node_id: LocalNodeId<TypeExpression>,
    elements_len: usize,
) -> UnionChainHead {
    let mut element_count = elements_len;

    loop {
        let Some((parent_id, parent_type)) = context.parent(node_id) else {
            return UnionChainHead {
                parent: None,
                element_count,
            };
        };

        if parent_type != NodeType::TypeExpression {
            return UnionChainHead {
                parent: Some((parent_id, parent_type)),
                element_count,
            };
        }

        let parent_type_id = LocalNodeId::<TypeExpression>::new(parent_id);
        let TypeExpression::Union { elements } = context.tree.get(parent_type_id) else {
            return UnionChainHead {
                parent: Some((parent_id, parent_type)),
                element_count,
            };
        };

        if elements.len() != 1 {
            return UnionChainHead {
                parent: Some((parent_id, parent_type)),
                element_count,
            };
        }

        element_count = 1;
        node_id = parent_type_id;
    }
}

/// Return whether one expression parent should add one union indent.
fn union_expression_should_indent(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        // generic arguments already indent their explicit type wrapper
        Expression::Type { .. } => context
            .parent(expression_id)
            .is_none_or(|(_, parent_type)| parent_type != NodeType::GenericArgument),

        // cast-like expressions already indent their type side
        Expression::As { .. } | Expression::Satisfies { .. } => false,

        // other expression parents use the default union layout
        _ => true,
    }
}

/// Write one inline union body without multiline grouping logic.
fn write_inline_union_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<TypeExpression>],
    emit_last_arm_trailing_comments: bool,
) -> FormatResult<()> {
    for (index, element_id) in elements.iter().copied().enumerate() {
        // separator
        if index > 0 {
            write!(f, [space(), token("|"), space()])?;
        }

        // operand
        write!(f, [element_id])?;
    }

    // trailing comments inside derived parens
    if emit_last_arm_trailing_comments && let Some(last_element_id) = elements.last().copied() {
        let trailing_comments = {
            let comments = f.context().comments();
            comments
                .end_of_line_comments_after(f.context().span(last_element_id).end)
                .to_vec()
        };

        if !trailing_comments.is_empty() {
            write!(f, [FormatTrailingComments::Comments(&trailing_comments)])?;
        }
    }

    Ok(())
}

/// Write one union type with break-aware leading separators.
pub(crate) fn write_union_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    elements: &[LocalNodeId<TypeExpression>],
    is_in_explicit_parentheses: bool,
) -> FormatResult<()> {
    // empty union
    if elements.is_empty() {
        return Ok(());
    }

    // leading comments
    let leading_separator_span = f.context().tree.get_side_span(
        node_id,
        NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator),
    );
    let format_node_id = union_print_node(f.context(), node_id, elements);
    let TypeExpression::Union {
        elements: format_elements,
    } = f.context().tree.get(format_node_id)
    else {
        return Err(FormatError::SyntaxError {
            message: "union print node is not a union expression",
        });
    };
    let union_content_start = type_expression_content_start(f.context(), node_id);
    let leading_separator_comments = {
        let comments = f.context().comments();

        leading_separator_span.map_or_else(Vec::new, |span| {
            comments
                .comments_in_range(span.start, union_content_start)
                .to_vec()
        })
    };
    let has_leading_separator_prefix_comment = leading_separator_comments
        .iter()
        .copied()
        .any(|comment| comment.is_line() || comment.is_multiline_block());

    let union_leading_comments = {
        let comments = f.context().comments();

        if has_leading_separator_prefix_comment
            && let Some(leading_separator_span) = leading_separator_span
        {
            comments
                .comments_before(leading_separator_span.start)
                .to_vec()
        } else {
            comments.comments_before(union_content_start).to_vec()
        }
    };

    // inline unions
    let leading_comments = LeadingComments::from_comments(&union_leading_comments);
    let should_hug = union_should_hug(f, format_node_id, format_elements)
        && !has_leading_separator_prefix_comment
        && !format_elements
            .iter()
            .copied()
            .any(|element_id| node_has_trailing_line_ignore_directive(f.context(), element_id));
    let parent_needs_parentheses =
        type_expression_needs_parentheses_in_parent(f.context(), format_node_id);
    let emit_last_arm_trailing_comments = parent_needs_parentheses;

    if should_hug {
        return write_inline_union_type(f, format_elements, emit_last_arm_trailing_comments);
    }

    // multiline indent
    let should_indent = union_should_indent(f.context(), node_id, leading_comments);
    let needs_parentheses = parent_needs_parentheses && !is_in_explicit_parentheses;
    let chain_head = union_chain_head(f.context(), format_node_id, format_elements.len());
    let only_type = chain_head.element_count == 1;
    let content_group_id = f.group_id();

    // grouped content
    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let leading_separator = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if should_indent && !leading_comments.has_comments {
                write!(f, [soft_line_break_or_space()])?;
            }

            write!(f, [token("|"), space()])?;

            Ok(())
        });

        if has_leading_separator_prefix_comment {
            write!(f, [leading_separator])?;
        } else {
            write!(
                f,
                [if_group_breaks(&leading_separator).with_group_id(Some(content_group_id))]
            )?;
        }

        let mut element_iter = format_elements.iter().copied().peekable();
        let mut element_index = 0usize;

        while let Some(element_id) = element_iter.next() {
            // operand
            if should_hug {
                write!(f, [element_id])?;
            } else if element_index == 0 && has_leading_separator_prefix_comment {
                let first_element = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    write!(
                        f,
                        [FormatLeadingComments::Comments(&leading_separator_comments)]
                    )?;
                    write!(f, [element_id])
                });

                write!(f, [align(2, &first_element)])?;
            } else {
                write!(f, [align(2, &element_id)])?;
            }

            // following separators
            if let Some(next_element_id) = element_iter.peek().copied() {
                let current_element_span = f.context().span(element_id);
                let next_element_span = f.context().span(next_element_id);

                let comments_before_separator = {
                    let comments = f.context().comments();
                    comments
                        .comments_before_character(current_element_span.end, b'|')
                        .to_vec()
                };
                if !comments_before_separator.is_empty() {
                    write!(
                        f,
                        [FormatTrailingComments::Comments(&comments_before_separator)]
                    )?;
                }

                if f.context()
                    .comments()
                    .has_leading_own_line_comment(next_element_span.start)
                {
                    let comments_before_next_element = {
                        let comments = f.context().comments();
                        comments.comments_before(next_element_span.start).to_vec()
                    };

                    if !comments_before_next_element.is_empty() {
                        write!(
                            f,
                            [FormatTrailingComments::Comments(
                                &comments_before_next_element
                            )]
                        )?;
                    }
                }

                if node_has_trailing_line_ignore_directive(f.context(), element_id) {
                    write!(f, [hard_line_break()])?;
                } else if should_hug {
                    write!(f, [space()])?;
                } else {
                    write!(f, [soft_line_break_or_space()])?;
                }

                write!(f, [token("|"), space()])?;
            }
            // trailing comments inside derived parens
            else if emit_last_arm_trailing_comments {
                let trailing_comments = {
                    let comments = f.context().comments();
                    comments
                        .end_of_line_comments_after(f.context().span(element_id).end)
                        .to_vec()
                };

                if !trailing_comments.is_empty() {
                    write!(f, [FormatTrailingComments::Comments(&trailing_comments)])?;
                }
            }

            element_index += 1;
        }

        Ok(())
    });

    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if needs_parentheses {
            return write!(f, [indent(&content), soft_line_break()]);
        }

        write!(f, [content])
    });

    let format_inner_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let has_own_line_comment = leading_comments.has_own_line_comment
            || matches!(
                chain_head.parent,
                Some((parent_id, NodeType::Declaration))
                    if matches!(
                        f.context()
                            .tree
                            .get(LocalNodeId::<Declaration>::new(parent_id)),
                        Declaration::Type(_)
                    )
            ) && leading_comments.has_trailing_own_line_block_comment;

        if (has_own_line_comment && !only_type)
            || (leading_comments.has_end_of_line_comment && only_type)
        {
            write!(f, [soft_line_break()])?;
        }

        if !union_leading_comments.is_empty() {
            write!(
                f,
                [FormatLeadingComments::Comments(&union_leading_comments)]
            )?;
        }

        if !leading_comments.has_end_of_line_comment && has_own_line_comment && only_type {
            write!(f, [soft_line_break()])?;
        }

        write!(f, [group(&content).with_id(Some(content_group_id))])
    });

    if should_indent && !needs_parentheses {
        write!(f, [group(&indent(&format_inner_content))])
    } else {
        write!(f, [group(&format_inner_content)])
    }
}

/// Return whether one type body formats its own leading comments.
fn type_expression_body_owns_leading_comments(expression: &TypeExpression) -> bool {
    matches!(expression, TypeExpression::Union { .. })
}

/// Write prefix annotations for one type expression.
pub(crate) fn write_type_expression_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    write!(
        f,
        [prefix_annotations_without_comments(f.context(), node_id)]
    )
}

/// Write one type expression without emitting prefix annotations.
pub(crate) fn write_type_expression_without_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(node_id);
    write_type_expression_body(f, node_id, expression, false, TypeExpressionLayout::DEFAULT)?;
    write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
}

/// Return whether one prefix type operand needs grouping.
fn prefix_type_operand_needs_grouping(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
    target_type: LocalNodeId<TypeExpression>,
) -> bool {
    let target_span = context.span(target_type);
    let target_start = context.node_token_start(target_type);
    let prefix_span = context.span(node_id);
    let comments = context.comments();

    comments.has_comment_before(target_start)
        || comments.has_comment_in_range(target_span.end, prefix_span.end)
}

/// Write one prefix type operand with grouped boundary comments.
fn write_prefix_type_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    target_type: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    if prefix_type_operand_needs_grouping(f.context(), node_id, target_type) {
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(f, [token("("), soft_block_indent(&target_type), token(")")])
            }))]
        )?;
    } else {
        write!(f, [target_type])?;
    }

    Ok(())
}

/// Write one type expression without positional leading comments.
fn write_type_expression_without_leading_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(node_id);
    let layout = TypeExpressionLayout::DEFAULT;
    let needs_parentheses = layout.derived_parentheses.is_allowed()
        && type_expression_needs_parentheses_in_parent(f.context(), node_id);

    write_type_expression_prefix_annotations(f, node_id)?;

    if needs_parentheses {
        write!(f, [token("(")])?;
    }

    write_type_expression_body(f, node_id, expression, false, layout)?;

    if needs_parentheses {
        write!(f, [token(")")])?;
    }

    write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
}

/// Write one type expression node with optional derived parentheses.
pub(crate) fn write_type_expression_node<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    expression: &TypeExpression,
    layout: TypeExpressionLayout,
) -> FormatResult<()> {
    let needs_parentheses = layout.derived_parentheses.is_allowed()
        && type_expression_needs_parentheses_in_parent(f.context(), node_id);

    // prefix annotations
    write_type_expression_prefix_annotations(f, node_id)?;

    // positional leading comments
    write_type_expression_leading_comments(f, node_id, expression)?;

    // derived parentheses
    if needs_parentheses {
        write!(f, [token("(")])?;
    }

    // body
    write_type_expression_body(
        f,
        node_id,
        expression,
        layout.derived_parentheses == DerivedParentheses::Suppressed,
        layout,
    )?;

    // derived parentheses
    if needs_parentheses {
        write!(f, [token(")")])?;
    }

    // trailing annotations
    write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
}

/// Write one type with parentheses when one postfix operator needs them.
fn write_postfix_type_operand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let expression_id = normalize_postfix_type_operand(f.context(), expression_id);

    // postfix grouping
    if type_needs_postfix_parentheses(f.context(), expression_id) {
        let expression = f.context().tree.get(expression_id);

        write!(
            f,
            [group(&format_args![
                token("("),
                format_with(|f| {
                    write_type_expression_node(
                        f,
                        expression_id,
                        expression,
                        TypeExpressionLayout::WITHOUT_DERIVED_PARENTHESES,
                    )
                }),
                soft_line_break(),
                token(")")
            ])]
        )
    } else {
        write!(f, [expression_id])
    }
}

/// Write one range type expression.
fn write_range_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    start: Option<LocalNodeId<TypeExpression>>,
    end: Option<LocalNodeId<TypeExpression>>,
    end_kind: RangeEnd,
) -> FormatResult<()> {
    let start_end = start.map(|start| f.context().node_token_end(start));
    let end_start = end.map(|end| f.context().node_token_start(end));

    // start bound
    if let Some(start) = start {
        write!(f, [start])?;
    }

    // range operator
    write_range_operator(f, f.context().span(node_id), start_end, end_start, end_kind)?;

    // end bound
    if let Some(end) = end {
        write!(f, [end])?;
    }

    Ok(())
}

/// Return one effective parent plus the outermost transparent child it sees.
fn effective_type_parent(
    context: &DestackFormatContext<'_>,
    mut node_id: LocalNodeId<TypeExpression>,
) -> Option<(u32, NodeType, LocalNodeId<TypeExpression>)> {
    let mut parent_child_id = node_id;

    loop {
        let (parent_id, parent_type) = context.parent(node_id)?;

        if parent_type != NodeType::TypeExpression {
            return Some((parent_id, parent_type, parent_child_id));
        }

        let parent_type_id = LocalNodeId::<TypeExpression>::new(parent_id);
        match context.tree.get(parent_type_id) {
            // single member unions and intersections are also transparent here
            TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
                if elements.len() <= 1 && elements.first().copied() == Some(node_id) {
                    parent_child_id = parent_type_id;
                    node_id = parent_type_id;
                } else {
                    return Some((parent_id, parent_type, parent_child_id));
                }
            }

            // other type parents decide precedence directly
            _ => return Some((parent_id, parent_type, parent_child_id)),
        }
    }
}

/// Return whether one prefix operand needs grouping.
fn type_needs_prefix_operand_parentheses(
    context: &DestackFormatContext<'_>,
    child_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(child_id) {
        TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
            elements.len() > 1
        }
        TypeExpression::Conditional { .. }
        | TypeExpression::Extends { .. }
        | TypeExpression::Implements { .. }
        | TypeExpression::Mapped { .. }
        | TypeExpression::Range { .. } => true,
        TypeExpression::Infer { constraint, .. } => constraint.is_some(),
        TypeExpression::Function(_) | TypeExpression::Constructor(_) => true,
        _ => false,
    }
}

/// Return whether one parent type forces parentheses around its child position.
fn type_parent_requires_parentheses(
    context: &DestackFormatContext<'_>,
    parent_id: LocalNodeId<TypeExpression>,
    child_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(parent_id) {
        // postfix and indexed object positions need precedence grouping
        TypeExpression::Array { element } => *element == child_id,
        TypeExpression::Index { left, .. } => {
            *left == child_id && type_needs_index_object_parentheses(context, child_id)
        }

        // prefix type operators only group lower precedence operands
        TypeExpression::Readonly { target_type }
        | TypeExpression::Local { target_type }
        | TypeExpression::Shared { target_type }
        | TypeExpression::KeyOf { target_type }
        | TypeExpression::Not { target_type }
        | TypeExpression::OwnedOf { target_type, .. }
        | TypeExpression::BorrowedOf { target_type, .. }
        | TypeExpression::PointerOf { target_type, .. } => {
            *target_type == child_id && type_needs_prefix_operand_parentheses(context, child_id)
        }
        TypeExpression::Must { target_type } => *target_type == child_id,

        // value space typeof keeps its own precedence
        TypeExpression::TypeOf { .. } => false,

        // relations keep composite operands grouped
        TypeExpression::Extends { left, right } | TypeExpression::Implements { left, right } => {
            (*left == child_id || *right == child_id)
                && matches!(
                    context.tree.get(child_id),
                    TypeExpression::Union { elements } | TypeExpression::Intersection { elements }
                        if elements.len() > 1
                )
        }
        TypeExpression::Range { .. } => true,

        _ => false,
    }
}

/// One callable type expression.
#[derive(Clone, Copy)]
struct CallableType {
    /// Whether the type is constructor-like.
    is_constructor: bool,

    /// The declared return type.
    return_type: Option<LocalNodeId<TypeExpression>>,
}

/// Return whether one conditional `extends` branch needs function-like type parentheses.
fn conditional_extends_branch_needs_callable_parentheses(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<TypeExpression>>,
) -> bool {
    let Some(return_type) = return_type else {
        return false;
    };

    match context.tree.get(return_type) {
        TypeExpression::Infer { constraint, .. } => constraint.is_some(),
        _ => false,
    }
}

/// Return one callable type expression.
fn type_expression_callable(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
) -> Option<CallableType> {
    match context.tree.get(node_id) {
        TypeExpression::Function(function) => Some(CallableType {
            is_constructor: false,
            return_type: function.return_type,
        }),
        TypeExpression::Constructor(function) => Some(CallableType {
            is_constructor: true,
            return_type: function.return_type,
        }),
        _ => None,
    }
}

/// Return whether one callable type needs parentheses in one type-expression parent.
fn callable_type_needs_parentheses_in_type_parent(
    context: &DestackFormatContext<'_>,
    callable: CallableType,
    parent_id: LocalNodeId<TypeExpression>,
    child_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(parent_id) {
        TypeExpression::Conditional {
            left, extends_type, ..
        } => {
            if *left == child_id {
                return true;
            }

            *extends_type == child_id
                && conditional_extends_branch_needs_callable_parentheses(
                    context,
                    callable.return_type,
                )
        }
        TypeExpression::Extends { left, right } | TypeExpression::Implements { left, right } => {
            *left == child_id || *right == child_id
        }

        TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
            elements.len() > 1
        }

        _ => type_parent_requires_parentheses(context, parent_id, child_id),
    }
}

/// Return whether one callable type needs parentheses in one declaration parent.
fn callable_type_needs_parentheses_in_declaration_parent(
    context: &DestackFormatContext<'_>,
    callable: CallableType,
    parent_id: LocalNodeId<Declaration>,
    child_id: LocalNodeId<TypeExpression>,
) -> bool {
    let Declaration::Function(parent_function) = context.tree.get(parent_id) else {
        return false;
    };

    if parent_function.signature.return_type != Some(child_id) {
        return false;
    }

    !callable.is_constructor && parent_function.signature.form == FunctionForm::Lambda
}

/// Unwrap the value term one type expression carries in type space.
pub(crate) fn static_value_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
) -> Option<LocalNodeId<Expression>> {
    match context.tree.get(node_id) {
        TypeExpression::StaticValue { expression } => Some(*expression),
        _ => None,
    }
}

/// Return whether one type expression needs derived parentheses in its effective parent.
pub(crate) fn type_expression_needs_parentheses_in_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<TypeExpression>,
) -> bool {
    let Some((parent_id, parent_type, parent_child_id)) = effective_type_parent(context, node_id)
    else {
        return false;
    };

    if let Some(callable) = type_expression_callable(context, node_id) {
        return match parent_type {
            NodeType::TypeExpression => {
                let parent_id = LocalNodeId::<TypeExpression>::new(parent_id);

                callable_type_needs_parentheses_in_type_parent(
                    context,
                    callable,
                    parent_id,
                    parent_child_id,
                )
            }
            NodeType::Declaration => {
                let parent_id = LocalNodeId::<Declaration>::new(parent_id);

                callable_type_needs_parentheses_in_declaration_parent(
                    context,
                    callable,
                    parent_id,
                    parent_child_id,
                )
            }
            _ => false,
        };
    }

    if parent_type != NodeType::TypeExpression {
        return false;
    }

    let parent_id = LocalNodeId::<TypeExpression>::new(parent_id);
    match context.tree.get(node_id) {
        TypeExpression::Union { elements } => {
            if elements.len() <= 1 {
                return false;
            }

            match context.tree.get(parent_id) {
                TypeExpression::Intersection { elements } => elements.len() > 1,
                _ => type_parent_requires_parentheses(context, parent_id, parent_child_id),
            }
        }
        TypeExpression::Intersection { elements } => {
            if elements.len() <= 1 {
                return false;
            }

            match context.tree.get(parent_id) {
                TypeExpression::Union { .. } | TypeExpression::Intersection { .. } => false,
                _ => type_parent_requires_parentheses(context, parent_id, parent_child_id),
            }
        }
        TypeExpression::Conditional { .. } => match context.tree.get(parent_id) {
            TypeExpression::Conditional {
                left, extends_type, ..
            } => *left == parent_child_id || *extends_type == parent_child_id,
            TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
                elements.len() > 1
            }
            _ => type_parent_requires_parentheses(context, parent_id, parent_child_id),
        },
        TypeExpression::Extends { .. } | TypeExpression::Implements { .. } => {
            match context.tree.get(parent_id) {
                TypeExpression::Conditional {
                    left, extends_type, ..
                } => *left == parent_child_id || *extends_type == parent_child_id,
                TypeExpression::Extends { left, right }
                | TypeExpression::Implements { left, right } => {
                    *left == parent_child_id || *right == parent_child_id
                }
                TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
                    elements.len() > 1
                }
                _ => type_parent_requires_parentheses(context, parent_id, parent_child_id),
            }
        }
        TypeExpression::Range { .. } => match context.tree.get(parent_id) {
            TypeExpression::Range { .. } => true,
            _ => type_parent_requires_parentheses(context, parent_id, parent_child_id),
        },
        TypeExpression::Readonly { .. }
        | TypeExpression::Local { .. }
        | TypeExpression::Shared { .. }
        | TypeExpression::KeyOf { .. }
        | TypeExpression::TypeOf { .. }
        | TypeExpression::Must { .. }
        | TypeExpression::Not { .. }
        | TypeExpression::OwnedOf { .. }
        | TypeExpression::BorrowedOf { .. }
        | TypeExpression::PointerOf { .. } => {
            type_parent_requires_parentheses(context, parent_id, parent_child_id)
        }
        TypeExpression::Infer {
            constraint: Some(_),
            ..
        } => match context.tree.get(parent_id) {
            TypeExpression::Union { elements } | TypeExpression::Intersection { elements } => {
                elements.len() > 1
            }
            _ => type_parent_requires_parentheses(context, parent_id, parent_child_id),
        },
        _ => false,
    }
}

/// Write one list of value parameters in type position.
fn write_value_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    this_form: Option<destack_dir::ThisForm>,
    this_parameter: Option<LocalNodeId<Parameter>>,
    parameter_ids: &[LocalNodeId<Parameter>],
) -> FormatResult<()> {
    let parameters = ParameterList::new(this_parameter, parameter_ids);

    if parameters.is_empty() {
        return write!(f, [token("("), token(")")]);
    }

    if should_hug_function_parameters(f.context(), &parameters, false) {
        return write_signature_hug_parameter_list_with_this(
            f,
            this_form,
            this_parameter,
            parameter_ids,
        );
    }

    let disallow_trailing_parameter_separator = parameters
        .last()
        .is_some_and(|parameter_id| parameter_is_variadic(f.context(), *parameter_id));

    write_signature_parameter_list_with_this(
        f,
        this_form,
        this_parameter,
        parameter_ids,
        disallow_trailing_parameter_separator,
    )
}

/// Write one complete callable declaration in type position.
fn write_type_callable<'ast, H, R>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_parameters: &[LocalNodeId<GenericParameter>],
    where_clauses: &[LocalNodeId<WhereClause>],
    this_form: Option<destack_dir::ThisForm>,
    this_parameter: Option<LocalNodeId<Parameter>>,
    parameters: &[LocalNodeId<Parameter>],
    return_type: Option<LocalNodeId<TypeExpression>>,
    format_parameter_head: H,
    format_return_type: R,
    should_group_return_type: bool,
) -> FormatResult<()>
where
    H: Format<'ast, DestackFormatContext<'ast>>,
    R: Format<'ast, DestackFormatContext<'ast>>,
{
    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let format_parameters = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write_value_parameters(f, this_form, this_parameter, parameters)
        });

        let parameter_count = parameters.len() + usize::from(this_parameter.is_some());
        write_grouped_parameters_with_return_type(
            f,
            generic_parameters,
            parameter_count,
            return_type,
            &format_parameter_head,
            format_parameters,
            &format_return_type,
            false,
            should_group_return_type,
        )?;

        write_type_callable_where_clauses(f, where_clauses)
    });

    write!(f, [group(&content)])
}

/// Write generic parameters for one type-space callable.
fn write_type_callable_generic_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_parameters: &[LocalNodeId<GenericParameter>],
    has_leading_space: bool,
) -> FormatResult<()> {
    if generic_parameters.is_empty() {
        return Ok(());
    }

    if has_leading_space {
        write!(f, [space()])?;
    }

    write_generic_parameter_list(
        f,
        generic_parameters,
        default_generic_parameter_trailing_separator(f),
    )?;

    Ok(())
}

/// Write comments in one generated child boundary.
fn write_generated_boundary_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    start: u32,
    end: u32,
) -> FormatResult<bool> {
    let comments = f
        .context()
        .comments()
        .comments_in_range(start, end)
        .to_vec();

    if comments.is_empty() {
        return Ok(false);
    }

    write!(f, [FormatTrailingComments::Comments(&comments)])?;

    Ok(true)
}

/// Return the parameter-list source span for one function-like node.
fn function_like_parameters_span<T>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> Option<Span>
where
    T: Node,
    Tree: TreeStore<T>,
{
    context
        .tree
        .get_side_span(node_id, NodeSpanType::Region(NodeSpanRegion::Parameters))
}

/// Write the constructor-type boundary between `new` and parameters.
fn write_constructor_type_parameter_boundary<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
) -> FormatResult<()>
where
    T: Node,
    Tree: TreeStore<T>,
{
    let Some(parameters_span) = function_like_parameters_span(f.context(), node_id) else {
        write!(f, [space()])?;
        return Ok(());
    };

    let Some(previous_token) = f.context().previous_token_before_span(parameters_span) else {
        write!(f, [space()])?;
        return Ok(());
    };

    if !write_generated_boundary_comments(f, previous_token.span.end, parameters_span.start)? {
        write!(f, [space()])?;
    }

    Ok(())
}

/// Write the arrow return section for one function-like type expression.
fn write_type_callable_arrow_return<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    return_type: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<()> {
    if let Some(return_type) = return_type {
        if let Some(parameters_span) = function_like_parameters_span(f.context(), node_id)
            && let Some(arrow_token) = f
                .context()
                .previous_token_before_span(f.context().span(return_type))
            && arrow_token.token.ty() == TokenType::ArrowWide
        {
            write_generated_boundary_comments(f, parameters_span.end, arrow_token.span.start)?;
        }

        write!(f, [space()])?;

        write!(f, [token("=>"), space()])?;
        write_type_expression_with_inline_prefix_annotations(f, return_type)?;
    }

    Ok(())
}

/// Write the where-clause suffix for one type-space callable.
fn write_type_callable_where_clauses<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    where_clauses: &[LocalNodeId<WhereClause>],
) -> FormatResult<()> {
    if !where_clauses.is_empty() {
        format_where_clause(f, where_clauses)?;
    }

    Ok(())
}

/// Write one function-like type declaration directly in type space.
fn write_function_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    function: &FunctionTypeExpression,
) -> FormatResult<()> {
    let generic_parameters = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_type_callable_generic_parameters(f, &function.generic_parameters, false)
    });
    let return_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_type_callable_arrow_return(f, node_id, function.return_type)
    });

    write_type_callable(
        f,
        &function.generic_parameters,
        &function.where_clauses,
        function.this_form,
        function.this_parameter,
        &function.parameters,
        function.return_type,
        generic_parameters,
        return_type,
        false,
    )
}

/// Write one constructor type declaration directly in type space.
fn write_constructor_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    function: &ConstructorType,
) -> FormatResult<()> {
    let generic_parameters = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_function_abstraction_prefix(f, function.is_abstract, false)?;
        write!(f, [Keyword::New])?;
        write_type_callable_generic_parameters(f, &function.generic_parameters, true)?;

        if function.generic_parameters.is_empty() {
            write_constructor_type_parameter_boundary(f, node_id)?;
        }

        Ok(())
    });
    let return_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_type_callable_arrow_return(f, node_id, function.return_type)
    });

    write_type_callable(
        f,
        &function.generic_parameters,
        &function.where_clauses,
        None,
        None,
        &function.parameters,
        function.return_type,
        generic_parameters,
        return_type,
        false,
    )
}

/// Write one type-space function signature.
fn write_type_signature<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeMember>,
    signature: &FunctionSignature,
    name: Name,
    is_optional: bool,
) -> FormatResult<()> {
    let generic_parameters = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_function_header_prefix(f, signature, false, true)?;
        write!(f, [name])?;

        if is_optional {
            write_optional_method_marker(f, node_id)?;
        }

        write_type_callable_generic_parameters(f, &signature.generic_parameters, false)
    });
    let return_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_type_member_return(f, node_id, signature.return_type)
    });

    write_type_callable(
        f,
        &signature.generic_parameters,
        &signature.where_clauses,
        signature.this_form,
        signature.this_parameter,
        &signature.parameters,
        signature.return_type,
        generic_parameters,
        return_type,
        true,
    )
}

/// Write one call signature declaration in type position.
fn write_call_signature<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeMember>,
    signature: &FunctionTypeExpression,
) -> FormatResult<()> {
    let generic_parameters = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_type_callable_generic_parameters(f, &signature.generic_parameters, false)
    });
    let return_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_type_member_return(f, node_id, signature.return_type)
    });

    write_type_callable(
        f,
        &signature.generic_parameters,
        &signature.where_clauses,
        signature.this_form,
        signature.this_parameter,
        &signature.parameters,
        signature.return_type,
        generic_parameters,
        return_type,
        false,
    )
}

/// Write one construct signature declaration in type position.
fn write_construct_signature<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeMember>,
    signature: &ConstructorType,
) -> FormatResult<()> {
    let generic_parameters = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_function_abstraction_prefix(f, signature.is_abstract, false)?;
        write!(f, [Keyword::New])?;
        write_type_callable_generic_parameters(f, &signature.generic_parameters, true)?;

        if signature.generic_parameters.is_empty() {
            write!(f, [space()])?;
        }

        Ok(())
    });
    let return_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_type_member_return(f, node_id, signature.return_type)
    });

    write_type_callable(
        f,
        &signature.generic_parameters,
        &signature.where_clauses,
        None,
        None,
        &signature.parameters,
        signature.return_type,
        generic_parameters,
        return_type,
        false,
    )
}

/// Write the optional marker and adjacent comments for one method.
fn write_optional_method_marker<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeMember>,
) -> FormatResult<()> {
    let name_end = f
        .context()
        .tree
        .get_main_span(node_id)
        .map_or_else(|| f.context().span(node_id).start, |span| span.end);
    let parameter_start = function_like_parameters_span(f.context(), node_id)
        .map_or_else(|| f.context().span(node_id).end, |span| span.start);
    let optional_token = f
        .context()
        .first_token_between(name_end, parameter_start)
        .filter(|token| token.token.ty() == TokenType::Maybe);

    if let Some(optional_token) = optional_token {
        write_generated_boundary_comments(f, name_end, optional_token.span.start)?;
    }

    write!(f, [token("?")])?;

    if let Some(optional_token) = optional_token {
        write_generated_boundary_comments(f, optional_token.span.end, parameter_start)?;
    }

    Ok(())
}

/// Write an optional type-member return annotation.
fn write_type_member_return<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeMember>,
    return_type: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<()> {
    if let Some(return_type) = return_type {
        write_signature_return_type(f, node_id, return_type)?;
    }

    Ok(())
}

/// Write one mapped-type modifier prefix.
fn write_mapped_modifier_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifier: MappedTypeModifier,
    keyword: &'static str,
) -> FormatResult<()> {
    // modifier
    match modifier {
        MappedTypeModifier::Present => write!(f, [token(keyword), space()]),
        MappedTypeModifier::Add => write!(f, [token("+"), token(keyword), space()]),
        MappedTypeModifier::Remove => write!(f, [token("-"), token(keyword), space()]),
        MappedTypeModifier::None => Ok(()),
    }
}

/// Write one mapped-type modifier suffix.
fn write_mapped_modifier_suffix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifier: MappedTypeModifier,
) -> FormatResult<()> {
    // modifier
    match modifier {
        MappedTypeModifier::Present => write!(f, [token("?")]),
        MappedTypeModifier::Add => write!(f, [token("+?")]),
        MappedTypeModifier::Remove => write!(f, [token("-?")]),
        MappedTypeModifier::None => Ok(()),
    }
}

impl<'ast> FormatNode<'ast, TypeMappedParameter> for TypeMappedParameter {
    fn format_node(
        &self,
        node_id: LocalNodeId<TypeMappedParameter>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let source_type_comment_end =
            mapped_source_type_trailing_comment_end(f.context(), self.source_type, self.key_remap);

        write!(
            f,
            [
                token("["),
                self.name,
                space(),
                Keyword::In,
                space(),
                format_node_with_trailing_comments(
                    f.context().span(node_id),
                    self.source_type,
                    Some(source_type_comment_end)
                )
            ]
        )?;

        if let Some(key_remap) = self.key_remap {
            write_mapped_key_remap(f, key_remap)?;
        }

        write!(f, [token("]")])
    }
}

/// Return the `as` token that starts one mapped-type key remap.
fn mapped_key_remap_as_token(
    context: &DestackFormatContext<'_>,
    key_remap: LocalNodeId<TypeExpression>,
) -> Option<TokenSpan> {
    let remap_span = context.span(key_remap);
    let as_token = context.previous_token_before_span(remap_span)?;

    if context.token_keyword(as_token) != Some(Keyword::As) {
        return None;
    }

    Some(as_token)
}

/// Return the offset where mapped source-type trailing comments stop.
fn mapped_source_type_trailing_comment_end(
    context: &DestackFormatContext<'_>,
    source_type: LocalNodeId<TypeExpression>,
    key_remap: Option<LocalNodeId<TypeExpression>>,
) -> u32 {
    key_remap
        .and_then(|key_remap| mapped_key_remap_as_token(context, key_remap))
        .map(|as_token| as_token.span.start)
        .or_else(|| key_remap.map(|key_remap| context.span(key_remap).start))
        .unwrap_or_else(|| context.span(source_type).end)
}

/// Write one mapped-type key remap after the source type.
fn write_mapped_key_remap<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    key_remap: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let Some(as_token) = mapped_key_remap_as_token(f.context(), key_remap) else {
        write!(f, [space(), Keyword::As, space(), key_remap])?;
        return Ok(());
    };

    let remap_start = f.context().node_token_start(key_remap);
    let leading_comments = f
        .context()
        .comments()
        .comments_in_range(as_token.span.end, remap_start)
        .to_vec();
    let has_breaking_comment = leading_comments
        .iter()
        .any(|comment| comment.is_line() || comment.followed_by_newline());

    write!(f, [space(), Keyword::As, space()])?;

    let format_remap = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if !leading_comments.is_empty() {
            write!(f, [FormatLeadingComments::Comments(&leading_comments)])?;
        }

        write_type_expression_without_leading_comments(f, key_remap)
    });

    if has_breaking_comment {
        write!(f, [indent(&format_remap)])
    } else {
        write!(f, [format_remap])
    }
}

/// Write the value annotation for one mapped type.
fn write_mapped_value_type_annotation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    value: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let value_start = f.context().node_token_start(value);
    let type_span = f
        .context()
        .tree
        .get_side_span(node_id, NodeSpanType::Region(NodeSpanRegion::Type))
        .ok_or(FormatError::SyntaxError {
            message: "mapped type value requires a type span",
        })?;
    let separator_comments = f
        .context()
        .comments()
        .comments_in_range(type_span.start + 1, value_start);
    let has_line_separator_comment = separator_comments.iter().any(|comment| comment.is_line());
    let trailing_comments = f
        .context()
        .tree
        .get_side_span(value, NodeSpanType::Boundary(NodeSpanBoundary::Trailing))
        .map(|span| {
            f.context()
                .comments()
                .comments_in_range(span.start, span.end)
                .to_vec()
        })
        .unwrap_or_default();
    let trailing_block_comments = trailing_comments
        .iter()
        .copied()
        .filter(|comment| comment.is_block())
        .collect::<Vec<_>>();
    let trailing_line_comments = trailing_comments
        .iter()
        .copied()
        .filter(|comment| comment.is_line())
        .collect::<Vec<_>>();

    write!(f, [token(":"), space()])?;

    if has_line_separator_comment {
        write_type_expression_without_leading_comments(f, value)?;
        write!(f, [if_group_breaks(&token(";"))])?;
        write!(f, [FormatTrailingComments::Comments(separator_comments)])?;

        if !trailing_comments.is_empty() {
            write!(f, [FormatTrailingComments::Comments(&trailing_comments)])?;
        }

        return Ok(());
    }

    if !separator_comments.is_empty() {
        write!(f, [FormatLeadingComments::Comments(separator_comments)])?;
    }

    write!(f, [value])?;

    if !trailing_block_comments.is_empty() {
        write!(
            f,
            [FormatTrailingComments::Comments(&trailing_block_comments)]
        )?;
    }

    write!(f, [if_group_breaks(&token(";"))])?;

    if !trailing_line_comments.is_empty() {
        write!(
            f,
            [FormatTrailingComments::Comments(&trailing_line_comments)]
        )
    } else {
        Ok(())
    }
}

/// Format one type-member list with group-aware separators.
pub(crate) fn format_type_member_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    members: &[LocalNodeId<TypeMember>],
) -> FormatResult<()> {
    let entries = FormatSeparatedIter::new(members.iter().copied(), ";")
        .with_trailing_separator(TrailingSeparator::Allowed);

    for (index, entry) in entries.enumerate() {
        let member_id = entry.element();

        if index > 0 {
            if type_members_have_blank_line_between(f, member_id) {
                write!(f, [empty_line()])?;
            } else {
                write!(f, [soft_line_break_or_space()])?;
            }
        }

        write!(f, [entry])?;
    }

    Ok(())
}

/// Return whether source preserves an empty line between two type members.
fn type_members_have_blank_line_between(
    f: &DestackFormatter<'_, '_>,
    next_id: LocalNodeId<TypeMember>,
) -> bool {
    let next_span = f.context().span(next_id);
    let source_text = f.context().source_text();
    let comments = f.context().comments();

    source_text.get_lines_before(next_span, comments) > 1
}

/// Format one expanded type-member block.
pub(crate) fn format_type_member_block_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    members: &[LocalNodeId<TypeMember>],
) -> FormatResult<()> {
    let ignore_ranges = if f.context().has_ignore_directive_markers() {
        let source_comments = f.context().source_comments();
        ignore_ranges_for_nodes(f.context(), members, source_comments)
    } else {
        std::collections::HashMap::new()
    };
    let entries = FormatSeparatedIter::new(members.iter().copied(), ";")
        .with_trailing_separator(TrailingSeparator::Allowed);
    let mut skip_until = None;

    for (index, entry) in entries.enumerate() {
        let member_id = entry.element();
        let member_span = f.context().span(member_id);

        if let Some(skip_end) = skip_until {
            if member_span.start < skip_end {
                continue;
            }

            skip_until = None;
        }

        if index > 0 {
            if type_members_have_blank_line_between(f, member_id) {
                write!(f, [empty_line()])?;
            } else {
                write!(f, [hard_line_break()])?;
            }
        }

        if let Some(range_span) = ignore_ranges.get(&member_id.id) {
            let comments = f.context().comments().comments_before(range_span.start);
            if !comments.is_empty() {
                write!(f, [FormatLeadingComments::Comments(comments)])?;
            }

            write_source_span(f, *range_span)?;
            skip_until = Some(range_span.end);
            continue;
        }

        // default method bodies end their member without a separator
        let member = f.context().tree.get(member_id);
        if matches!(member, TypeMember::Method { body: Some(_), .. }) {
            write!(f, [member_id])?;
        } else {
            write!(f, [entry])?;
        }
    }

    Ok(())
}

/// Return the optional trailing separator style for tuple types.
fn optional_tuple_trailing_separator(f: &DestackFormatter<'_, '_>) -> TrailingSeparator {
    match f.context().options.trailing_comma {
        TrailingComma::None => TrailingSeparator::Omit,
        TrailingComma::Es5 | TrailingComma::All => TrailingSeparator::Allowed,
    }
}

/// Write one tuple type with explicit delimiters.
fn write_tuple_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    elements: &[LocalNodeId<TupleElement>],
    open_token: &'static str,
    close_token: &'static str,
    trailing_separator: TrailingSeparator,
) -> FormatResult<()> {
    let body = separated_entries(",", elements, trailing_separator, None);

    write!(
        f,
        [group(&format_args![
            token(open_token),
            soft_block_indent(&body),
            token(close_token)
        ])]
    )
}

/// Write one type body without prefix annotations.
pub(crate) fn write_type_expression_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    expression: &TypeExpression,
    is_in_explicit_parentheses: bool,
    layout: TypeExpressionLayout,
) -> FormatResult<()> {
    ensure_sufficient_stack(|| {
        write_type_expression_body_inner(f, node_id, expression, is_in_explicit_parentheses, layout)
    })
}

/// Write one type body.
fn write_type_expression_body_inner<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<TypeExpression>,
    expression: &TypeExpression,
    is_in_explicit_parentheses: bool,
    layout: TypeExpressionLayout,
) -> FormatResult<()> {
    match expression {
        TypeExpression::Literal { value } => {
            format_scalar_literal(value, f.context().span(node_id), f)?;
        }
        TypeExpression::Lifetime { name } => {
            write!(f, [*name])?;
        }
        TypeExpression::Keyword { value } => {
            write!(f, [value])?;
        }
        TypeExpression::Intrinsic => {
            write!(f, [token("intrinsic")])?;
        }
        TypeExpression::Tuple { form, elements } => {
            let trailing_separator = if elements.len() == 1 {
                TrailingSeparator::Mandatory
            } else {
                optional_tuple_trailing_separator(f)
            };
            let (open, close) = match form {
                TupleForm::Tuple => ("(", ")"),
                TupleForm::Array => ("[", "]"),
            };

            write_tuple_type(f, elements, open, close, trailing_separator)?;
        }
        TypeExpression::Array { element } => {
            write_postfix_type_operand(f, *element)?;
            write!(f, [token("[]")])?;
        }
        TypeExpression::Slice { element } => {
            write!(f, [token("["), element, token("]")])?;
        }
        TypeExpression::FixedArray { element, length } => {
            write!(
                f,
                [token("["), element, token(";"), space(), length, token("]")]
            )?;
        }
        TypeExpression::Object { members } => {
            // empty body
            if members.is_empty() {
                write!(f, [token("{}")])?;
                return Ok(());
            }

            // direct inline body
            if let Some(node) = prepare_inline_type_object(f, node_id, members, layout)? {
                f.write_element(node);
                return Ok(());
            }

            // grouped body
            let should_expand =
                type_object_members_have_leading_newline(f.context(), node_id, members);
            let should_hug = type_object_should_hug(f.context(), node_id);
            let inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                if f.context().options.bracket_spacing {
                    write!(f, [if_group_fits_on_line(&space())])?;
                }

                if should_hug {
                    format_type_member_list(f, members)?;
                } else if should_expand {
                    format_type_member_block_list(f, members)?;
                } else {
                    format_type_member_list(f, members)?;
                }

                if f.context().options.bracket_spacing {
                    write!(f, [if_group_fits_on_line(&space())])?;
                }

                Ok(())
            });

            write!(f, [token("{")])?;

            if should_hug {
                write!(f, [soft_block_indent(&inner)])?;
            } else {
                write!(
                    f,
                    [group(&soft_block_indent(&inner)).should_expand(should_expand)]
                )?;
            }

            write!(f, [token("}")])?;
        }
        TypeExpression::Function(function) => {
            write_function_type(f, node_id, function)?;
        }
        TypeExpression::Constructor(function) => {
            write_constructor_type(f, node_id, function)?;
        }
        TypeExpression::Reference {
            path,
            generic_arguments,
        } => {
            write!(f, [path])?;

            if !generic_arguments.is_empty() {
                format_generic_argument_list(f, generic_arguments)?;
            }
        }
        TypeExpression::Member {
            left,
            name,
            generic_arguments,
        } => {
            write_postfix_type_operand(f, *left)?;
            write!(f, [token("."), *name])?;

            if !generic_arguments.is_empty() {
                format_generic_argument_list(f, generic_arguments)?;
            }
        }
        TypeExpression::Range {
            start,
            end,
            end_kind,
        } => {
            write_range_type(f, node_id, *start, *end, *end_kind)?;
        }
        TypeExpression::Const => {
            write!(f, [Keyword::Const])?;
        }
        TypeExpression::This => {
            write!(f, [Keyword::This])?;
        }
        TypeExpression::Readonly { target_type } => {
            write!(f, [Keyword::Readonly, space(), target_type])?;
        }
        TypeExpression::Local { target_type } => {
            write!(f, [Keyword::Local, space(), target_type])?;
        }
        TypeExpression::Shared { target_type } => {
            write!(f, [token("shared"), space(), target_type])?;
        }
        TypeExpression::KeyOf { target_type } => {
            write!(f, [Keyword::Keyof, space(), target_type])?;
        }
        TypeExpression::TypeOf { value } => {
            write!(f, [Keyword::Typeof, space(), value])?;
        }
        TypeExpression::StaticValue { expression } => {
            write!(f, [expression])?;
        }
        TypeExpression::Must { target_type } => {
            write!(f, [target_type, token("!")])?;
        }
        TypeExpression::Not { target_type } => {
            write!(f, [token("!"), target_type])?;
        }
        TypeExpression::OwnedOf {
            mutability,
            variance,
            target_type,
        } => {
            write!(f, [token("^")])?;

            if let Some(mutability) = mutability {
                match mutability {
                    Mutability::Immutable => write!(f, [Keyword::Readonly, space()])?,
                    Mutability::Mutable => {}
                }
            }

            if let Some(variance) = variance {
                match variance {
                    VarianceBound::Implements => write!(f, [Keyword::Implements, space()])?,
                    VarianceBound::Extends => write!(f, [Keyword::Extends, space()])?,
                    VarianceBound::Super => write!(f, [Keyword::Super, space()])?,
                }
            }

            write_prefix_type_operand(f, node_id, *target_type)?;
        }
        TypeExpression::BorrowedOf {
            lifetime,
            access,
            variance,
            target_type,
        } => {
            write!(f, [token("&")])?;

            if let Some(lifetime) = lifetime {
                write!(f, [lifetime, space()])?;
            }

            write_access_prefix(f, *access)?;

            if let Some(variance) = variance {
                match variance {
                    VarianceBound::Implements => write!(f, [Keyword::Implements, space()])?,
                    VarianceBound::Extends => write!(f, [Keyword::Extends, space()])?,
                    VarianceBound::Super => write!(f, [Keyword::Super, space()])?,
                }
            }

            write_prefix_type_operand(f, node_id, *target_type)?;
        }
        TypeExpression::PointerOf {
            mutability,
            target_type,
        } => {
            write!(f, [token("*")])?;

            if let Some(mutability) = mutability {
                match mutability {
                    Mutability::Immutable => write!(f, [Keyword::Readonly, space()])?,
                    Mutability::Mutable => {}
                }
            }

            write_prefix_type_operand(f, node_id, *target_type)?;
        }
        TypeExpression::Union { elements } => {
            write_union_type(f, node_id, elements, is_in_explicit_parentheses)?;
        }
        TypeExpression::Intersection { elements } => {
            write_intersection_type(f, node_id, elements)?;
        }
        TypeExpression::Conditional {
            left,
            extends_type,
            then_type,
            else_type,
        } => {
            write_conditional_type(f, node_id, *left, *extends_type, *then_type, *else_type)?;
        }
        TypeExpression::Extends { left, right } => {
            write_type_relation(f, *left, Keyword::Extends, *right)?;
        }
        TypeExpression::Implements { left, right } => {
            write_type_relation(f, *left, Keyword::Implements, *right)?;
        }
        TypeExpression::Mapped {
            parameter,
            readonly,
            optional,
            value,
        } => {
            let parameter = f.context().tree.get(*parameter);
            let span = f.context().span(node_id);
            let should_expand = f
                .context()
                .source_text()
                .has_newline_after_opening_brace(span.start);
            let leading_comments = mapped_type_leading_body_comments(f.context(), node_id, span);

            // mapped body
            let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                // comments after `{`
                if should_expand && !leading_comments.is_empty() {
                    write!(f, [FormatLeadingComments::Comments(&leading_comments)])?;
                }

                // modifiers
                write_mapped_modifier_prefix(f, *readonly, "readonly")?;

                // key head
                let format_key = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    let source_type_comment_end = mapped_source_type_trailing_comment_end(
                        f.context(),
                        parameter.source_type,
                        parameter.key_remap,
                    );

                    write!(
                        f,
                        [
                            token("["),
                            parameter.name,
                            space(),
                            Keyword::In,
                            space(),
                            format_node_with_trailing_comments(
                                f.context().span(node_id),
                                parameter.source_type,
                                Some(source_type_comment_end)
                            )
                        ]
                    )?;

                    if let Some(key_remap) = parameter.key_remap {
                        write_mapped_key_remap(f, key_remap)?;
                    }

                    write!(f, [token("]")])?;
                    write_mapped_modifier_suffix(f, *optional)
                });

                // value
                write!(f, [group(&format_key)])?;
                if let Some(value) = value {
                    write_mapped_value_type_annotation(f, node_id, *value)?;
                }

                Ok(())
            });

            // grouped block
            write!(
                f,
                [group(&format_args![
                    token("{"),
                    soft_block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        if f.context().options.bracket_spacing {
                            write!(f, [if_group_fits_on_line(&space())])?;
                        }

                        write!(f, [format_inner])?;

                        if f.context().options.bracket_spacing {
                            write!(f, [if_group_fits_on_line(&space())])?;
                        }

                        Ok(())
                    })),
                    token("}")
                ])
                .should_expand(should_expand)]
            )?;
        }
        TypeExpression::Index { left, index } => {
            write!(f, [left, token("["), index, token("]")])?;
        }
        TypeExpression::TemplateLiteral { strings, spans } => {
            format_type_template_literal(node_id, strings, spans, f)?;
        }
        TypeExpression::Infer {
            form,
            name,
            constraint,
        } => match form {
            InferForm::Hole => {
                write!(f, [token("_")])?;
            }
            InferForm::Infer => {
                write!(f, [Keyword::Infer, space()])?;
                if let Some(name) = name {
                    write!(f, [*name])?;
                } else {
                    write!(f, [token("_")])?;
                }

                if let Some(constraint) = constraint {
                    write!(f, [space(), Keyword::Extends, space(), constraint])?;
                }
            }
        },
        TypeExpression::Missing => {}
        TypeExpression::Error => {
            write_source_span(f, f.context().span(node_id))?;
        }
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, TypeExpression> for TypeExpression {
    fn format_node(
        &self,
        node_id: LocalNodeId<TypeExpression>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if node_has_ignore_directive(f.context(), node_id) {
            return write_ignored_node(f, node_id);
        }

        write_type_expression_node(f, node_id, self, TypeExpressionLayout::DEFAULT)
    }
}

impl<'ast> FormatNode<'ast, TypeMember> for TypeMember {
    fn format_node(
        &self,
        node_id: LocalNodeId<TypeMember>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // prefix annotations
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        // body
        match self {
            TypeMember::Field {
                is_static,
                is_optional,
                is_readonly,
                visibility,
                name,
                declared_type,
                ..
            } => {
                write_visibility_prefix(f, *visibility)?;

                if *is_static {
                    write!(f, [Keyword::Static, space()])?;
                }

                if *is_readonly {
                    write!(f, [Keyword::Readonly, space()])?;
                }

                write!(f, [name])?;

                if *is_optional {
                    write!(f, [token("?")])?;
                }

                if let Some(declared_type) = declared_type {
                    if let Some(type_span) = f
                        .context()
                        .tree
                        .get_side_span(node_id, NodeSpanType::Region(NodeSpanRegion::Type))
                    {
                        write_type_annotation_prefix(f, type_span.start)?;
                        write_type_expression_with_inline_prefix_annotations(f, *declared_type)?;
                    } else {
                        write_colon_prefixed_type_annotation(f, *declared_type)?;
                    }
                }
            }
            TypeMember::Method {
                is_static,
                is_optional,
                visibility,
                name,
                signature,
                body,
            } => {
                write_visibility_prefix(f, *visibility)?;

                if *is_static {
                    write!(f, [Keyword::Static, space()])?;
                }

                write_type_signature(f, node_id, signature, *name, *is_optional)?;

                // default method bodies print after the signature
                if let Some(body) = body {
                    write!(f, [space(), *body])?;
                }
            }
            TypeMember::CallSignature { signature } => {
                write_call_signature(f, node_id, signature)?;
            }
            TypeMember::ConstructSignature { signature } => {
                write_construct_signature(f, node_id, signature)?;
            }
            TypeMember::IndexSignature {
                is_optional,
                is_readonly,
                name,
                key_type,
                value_type,
                ..
            } => {
                if *is_readonly {
                    write!(f, [Keyword::Readonly, space()])?;
                }

                write!(
                    f,
                    [token("["), *name, token(":"), space(), key_type, token("]")]
                )?;

                if *is_optional {
                    write!(f, [token("?")])?;
                }

                write!(f, [token(":"), space(), value_type])?;
            }
            TypeMember::AssociatedType {
                name,
                generic_parameters,
                where_clauses,
                constraint,
                value,
                is_abstract,
                is_override,
                ..
            } => {
                if *is_abstract {
                    write!(f, [Keyword::Abstract, space()])?;
                }
                if *is_override {
                    write!(f, [Keyword::Override, space()])?;
                }

                write!(f, [Keyword::Type, space(), *name])?;

                if !generic_parameters.is_empty() {
                    write_generic_parameter_list(
                        f,
                        generic_parameters,
                        default_generic_parameter_trailing_separator(f),
                    )?;
                }

                if !where_clauses.is_empty() {
                    format_where_clause(f, where_clauses)?;
                }

                if let Some(constraint) = constraint {
                    write!(f, [token(":"), space(), *constraint])?;
                }

                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), *value])?;
                }
            }
            TypeMember::AssociatedConst {
                name,
                declared_type,
                value,
                is_abstract,
                is_override,
                ..
            } => {
                if *is_abstract {
                    write!(f, [Keyword::Abstract, space()])?;
                }
                if *is_override {
                    write!(f, [Keyword::Override, space()])?;
                }

                write!(f, [Keyword::Const, space(), *name])?;

                if let Some(declared_type) = declared_type {
                    write!(f, [token(":"), space(), *declared_type])?;
                }

                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), *value])?;
                }
            }
            TypeMember::Error => {
                write_source_span(f, f.context().span(node_id))?;
            }
        }

        // trailing annotations
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}

impl<'ast> FormatNode<'ast, GenericArgument> for GenericArgument {
    fn format_node(
        &self,
        node_id: LocalNodeId<GenericArgument>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // prefix annotations
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        // body
        match self {
            GenericArgument::Type { value } | GenericArgument::SpreadType { value } => {
                if matches!(self, GenericArgument::SpreadType { .. }) {
                    write!(f, [token("...")])?;
                }

                write!(f, [value])?;
            }
            GenericArgument::AssociatedType { name, value } => {
                write!(
                    f,
                    [
                        Keyword::Type,
                        space(),
                        *name,
                        space(),
                        token("="),
                        space(),
                        value
                    ]
                )?;
            }
            GenericArgument::AssociatedConst { name, value } => {
                write!(
                    f,
                    [
                        Keyword::Const,
                        space(),
                        *name,
                        space(),
                        token("="),
                        space(),
                        value
                    ]
                )?;
            }
            GenericArgument::Error => {
                write_source_span(f, f.context().span(node_id))?;
            }
        }

        // trailing annotations
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}

impl<'ast> FormatNode<'ast, TupleElement> for TupleElement {
    fn format_node(
        &self,
        node_id: LocalNodeId<TupleElement>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // prefix annotations
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        // body
        match self {
            TupleElement::Element {
                label,
                value,
                is_optional,
                is_readonly,
            } => {
                if *is_readonly {
                    write!(f, [Keyword::Readonly, space()])?;
                }

                if let Some(label) = label {
                    write!(f, [*label])?;
                    if *is_optional {
                        write!(f, [token("?")])?;
                    }
                    write!(f, [token(":"), space(), value])?;
                } else {
                    write!(f, [value])?;
                    if *is_optional {
                        write!(f, [token("?")])?;
                    }
                }
            }
            TupleElement::Spread { label, value } => {
                if let Some(label) = label {
                    write!(f, [*label, token(":"), space()])?;

                    write!(f, [token("..."), value])?;
                } else {
                    write!(f, [token("..."), value])?;
                }
            }
            TupleElement::Error => {
                write_source_span(f, f.context().span(node_id))?;
            }
        }

        // trailing annotations
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}
