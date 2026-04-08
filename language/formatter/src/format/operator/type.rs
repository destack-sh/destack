use super::union::{binary_like_is_type_union, flatten_type_binary_expression};
use crate::format::annotation::{prefix_annotations, write_raw_leading_comments};
use crate::format::chain::{static_argument_list_is_hug_safe, transparent_inner_expression};
use crate::format::declaration::{
    parenthesized_wraps_decorated_class_extends_head,
    parenthesized_wraps_prefix_annotated_class_extends_head,
};
use crate::format::expression::{
    write_expression_with_prefix_annotations_after_offset,
    write_expression_without_prefix_annotations,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, BinaryOperator, Comment, Expression, LocalNodeId, NodeType, TokenType,
    TypeBinaryOperator, TypeUnaryOperator,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    format_with, group, soft_block_indent, soft_line_break_or_space, space, token,
};
use destack_fir::{format_args, write};
use destack_source::NodeSpanType;

/// Return whether one raw type-position comment must force a following break.
fn type_position_comment_requires_break_after(
    context: &DestackFormatContext<'_>,
    comment: Comment,
) -> bool {
    if comment.is_line() {
        return true;
    }

    let comment_span = comment.span;
    let comment_token_type = context
        .first_non_trivia_token_in_span(comment_span)
        .map(|token| token.token.ty);

    matches!(
        comment_token_type,
        Some(TokenType::LineComment | TokenType::DocLineComment | TokenType::DocBlockComment)
    )
}

/// Return the leading `|` or `&` token that should render before one grouped type expression.
fn type_expression_leading_separator_token_type(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<TokenType> {
    if !expression_has_type_grouping_semantics(context, expression_id)
        || matches!(
            context.tree.get(expression_id),
            Expression::Binary { .. } | Expression::ReferenceOf { .. }
        )
    {
        return None;
    }

    let leading_separator_span = context
        .tree
        .get_side_span(expression_id, NodeSpanType::Leading)?;
    context
        .first_non_trivia_token_in_span(leading_separator_span)
        .and_then(|token| match token.token.ty {
            TokenType::ElementwiseOr | TokenType::ElementwiseAnd => Some(token.token.ty),
            _ => None,
        })
}

/// Write one colon-prefixed type annotation with group-aware boundary layout.
pub(crate) fn write_colon_prefixed_type_annotation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(f, [token(":"), space()])?;

    write_type_expression_with_inline_prefix_annotations(f, expression_id)
}

/// Write one static type argument, preserving type-position boundary ownership.
fn write_static_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    argument_id: LocalNodeId<Argument>,
) -> FormatResult<()> {
    match f.context().tree.get(argument_id) {
        Argument::Positional { value, .. } => {
            let argument_span = f.context().span(argument_id);
            let value_span = f.context().span(*value);
            if argument_span.file == value_span.file && argument_span.start < value_span.start {
                return write_type_expression_with_inline_prefix_annotations_from(
                    f,
                    *value,
                    argument_span.start,
                );
            }

            write_type_expression_with_inline_prefix_annotations(f, *value)
        }
        _ => write!(f, [argument_id]),
    }
}

/// Run one formatter operation with one explicit type-expression root.
fn with_type_expression_root<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    operation: impl FnOnce(&mut DestackFormatter<'ast, '_>) -> T,
) -> T {
    let context = f.context().clone();
    context.with_type_expression_root(expression_id, || operation(f))
}

/// Run one formatter operation with one explicit type-expression root and leading boundary.
fn with_type_expression_root_from<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    leading_comment_start: u32,
    operation: impl FnOnce(&mut DestackFormatter<'ast, '_>) -> T,
) -> T {
    let context = f.context().clone();
    context
        .with_type_expression_root_from(expression_id, Some(leading_comment_start), || operation(f))
}

/// Write one expression body without prefix annotations while owning local type comments.
fn write_expression_body_with_owned_type_position_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    has_owned_leading_raw_comments: bool,
) -> FormatResult<()> {
    if has_owned_leading_raw_comments {
        let expression_start = f.context().type_expression_token_start(expression_id);

        return write_expression_with_prefix_annotations_after_offset(
            f,
            expression_id,
            expression_start,
        );
    }

    write_expression_without_prefix_annotations(f, expression_id)
}

/// Write one expression with inline prefix annotations in explicit type role.
pub(crate) fn write_type_expression_with_inline_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    with_type_expression_root(f, expression_id, |f| {
        write_type_expression_inline_body(f, expression_id)
    })
}

/// Write one expression with inline prefix annotations in explicit type role from one boundary.
pub(crate) fn write_type_expression_with_inline_prefix_annotations_from<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    leading_comment_start: u32,
) -> FormatResult<()> {
    with_type_expression_root_from(f, expression_id, leading_comment_start, |f| {
        write_type_expression_inline_body(f, expression_id)
    })
}

/// Write one expression without prefix annotations in explicit type role.
pub(crate) fn write_type_expression_without_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    with_type_expression_root(f, expression_id, |f| {
        write_expression_without_prefix_annotations(f, expression_id)
    })
}

/// Write one type-position expression while preserving inline prefix annotation ownership.
fn write_type_expression_inline_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // leading trivia state
    let leading_raw_comment_ids = f.context().raw_type_position_comments_for(expression_id);
    let leading_separator_token_type =
        type_expression_leading_separator_token_type(f.context(), expression_id);
    let has_prefix_annotations = f.context().has_prefix_annotation(expression_id);
    let expression_is_type_union_root =
        binary_like_is_type_union(f.context(), expression_id, BinaryOperator::ElementwiseOr);
    let suppress_leading_raw_comment_owner = !expression_is_type_union_root
        && !leading_raw_comment_ids.is_empty()
        && leading_raw_comment_ids
            .iter()
            .copied()
            .all(|comment| !type_position_comment_requires_break_after(f.context(), comment));
    let owned_type_position_comments: &[Comment] = if suppress_leading_raw_comment_owner {
        &leading_raw_comment_ids
    } else {
        &[]
    };

    let write_expression_body = |f: &mut DestackFormatter<'ast, '_>| -> FormatResult<()> {
        let expression_start = f.context().type_expression_token_start(expression_id);
        let should_skip_generic_prefix_owner =
            expression_is_type_union_root && leading_separator_token_type.is_none();

        // leading separator
        if let Some(leading_separator_token_type) = leading_separator_token_type {
            // shared prefix owner
            if has_prefix_annotations {
                write!(f, [prefix_annotations(f.context(), expression_id)])?;
            }

            match leading_separator_token_type {
                TokenType::ElementwiseOr => write!(f, [token("|"), space()])?,
                TokenType::ElementwiseAnd => write!(f, [token("&")])?,
                _ => unreachable!("unexpected leading separator token"),
            }

            write_expression_body_with_owned_type_position_comments(
                f,
                expression_id,
                !owned_type_position_comments.is_empty(),
            )?;

            return Ok(());
        }

        // default owner
        let result = if should_skip_generic_prefix_owner {
            write_expression_body_with_owned_type_position_comments(
                f,
                expression_id,
                !owned_type_position_comments.is_empty(),
            )
        } else if suppress_leading_raw_comment_owner {
            write_expression_with_prefix_annotations_after_offset(
                f,
                expression_id,
                expression_start,
            )
        } else {
            write!(f, [expression_id])
        };

        result?;
        Ok(())
    };

    // type unions own their leading raw comments directly
    if expression_is_type_union_root && !leading_raw_comment_ids.is_empty() {
        return write_expression_body(f);
    }

    // raw type-position comments keep the multiline owner
    if suppress_leading_raw_comment_owner {
        write_raw_leading_comments(f, &leading_raw_comment_ids)?;
        return write_expression_body(f);
    }

    // breaking raw type-position comments
    if !leading_raw_comment_ids.is_empty() {
        write_raw_leading_comments(f, &leading_raw_comment_ids)?;
        return write_expression_body(f);
    }

    write_expression_body(f)
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
            write_static_argument(f, *argument_id)?;
        }
        write!(f, [token(">")])?;
        return Ok(());
    }

    let has_line_suffix_boundary = static_arguments.iter().copied().any(|argument_id| {
        !f.context()
            .end_of_line_raw_doc_comments_after(f.context().span(argument_id).end)
            .is_empty()
    });
    let format_arguments = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        for (index, argument_id) in static_arguments.iter().copied().enumerate() {
            if index > 0 {
                write!(f, [token(","), soft_line_break_or_space()])?;
            }

            write_static_argument(f, argument_id)?;
        }

        Ok(())
    });
    write!(
        f,
        [group(&format_args![
            token("<"),
            soft_block_indent(&format_arguments),
            token(">")
        ])
        .should_expand(has_line_suffix_boundary)]
    )
}

/// Return the expression parent of one expression node, if any.
fn expression_parent_expression_id(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let (parent_id, parent_type) = context.parent(expression_id)?;
    if parent_type != NodeType::Expression {
        return None;
    }

    Some(LocalNodeId::<Expression>::new(parent_id))
}

/// Return the declaration parent of one expression node, if any.
fn expression_parent_declaration_id(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<destack_ast::Declaration>> {
    let (parent_id, parent_type) = context.parent(expression_id)?;
    if parent_type != NodeType::Declaration {
        return None;
    }

    Some(LocalNodeId::<destack_ast::Declaration>::new(parent_id))
}

/// Return the normalized inner expression used for parenthesized type drop checks.
fn normalized_parenthesized_type_drop_inner_expression(
    context: &DestackFormatContext<'_>,
    inner_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    transparent_inner_expression(context, inner_id)
}

/// Return the effective parent expression for one type wrapper.
fn effective_parenthesized_type_parent_expression_id(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let mut current_id = node_id;

    loop {
        let parent_expression_id = expression_parent_expression_id(context, current_id)?;

        let Expression::Binary { operator, .. } = context.tree.get(parent_expression_id) else {
            return Some(parent_expression_id);
        };
        if !matches!(
            operator,
            BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
        ) || !expression_has_type_grouping_semantics(context, parent_expression_id)
        {
            return Some(parent_expression_id);
        }

        let operands = flatten_type_binary_expression(context, parent_expression_id, *operator);
        if operands.len() > 1 {
            return Some(parent_expression_id);
        }

        current_id = parent_expression_id;
    }
}

/// Return whether one parenthesized type sits in a function return type slot.
fn parenthesized_type_parent_is_function_return_type(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some(declaration_id) = expression_parent_declaration_id(context, node_id) else {
        return false;
    };

    matches!(
        context.tree.get(declaration_id),
        destack_ast::Declaration::Function { signature, .. } if signature.return_type == Some(node_id)
    )
}

/// Return whether one parent expression forces parentheses for operator-like type nodes.
fn operator_type_or_higher_needs_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    parent_expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(parent_expression_id) {
        Expression::Index { left, index, .. } => *left == node_id && index.is_none(),
        Expression::TypeUnary { right, .. } => *right == node_id,
        Expression::TypeIndex { left, .. } => *left == node_id,
        _ => false,
    }
}

/// Return whether one function-like type needs parentheses in its parent slot.
fn function_like_type_needs_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if parenthesized_type_parent_is_function_return_type(context, node_id) {
        return true;
    }

    let Some(parent_expression_id) =
        effective_parenthesized_type_parent_expression_id(context, node_id)
    else {
        return false;
    };

    match context.tree.get(parent_expression_id) {
        Expression::TypeConditional { left, right, .. } => *left == node_id || *right == node_id,
        Expression::Binary { .. }
            if expression_has_type_grouping_semantics(context, parent_expression_id) =>
        {
            true
        }
        _ => operator_type_or_higher_needs_parentheses(context, node_id, parent_expression_id),
    }
}

/// Return whether one associative type binary needs parentheses in its parent slot.
fn associative_type_binary_needs_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    normalized_inner_id: LocalNodeId<Expression>,
) -> bool {
    let Some(parent_expression_id) =
        effective_parenthesized_type_parent_expression_id(context, node_id)
    else {
        return false;
    };

    match context.tree.get(parent_expression_id) {
        Expression::Binary {
            left,
            operator: parent_operator,
            right,
        } if (*left == node_id || *right == node_id)
            && expression_has_type_grouping_semantics(context, parent_expression_id) =>
        {
            let Expression::Binary {
                operator: inner_operator,
                ..
            } = context.tree.get(normalized_inner_id)
            else {
                return false;
            };

            parent_operator != inner_operator
        }
        _ => operator_type_or_higher_needs_parentheses(context, node_id, parent_expression_id),
    }
}

/// Return whether one conditional type needs parentheses in its parent slot.
fn conditional_type_needs_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some(parent_expression_id) =
        effective_parenthesized_type_parent_expression_id(context, node_id)
    else {
        return false;
    };

    match context.tree.get(parent_expression_id) {
        Expression::TypeConditional { left, right, .. } => *left == node_id || *right == node_id,
        Expression::Binary { .. }
            if expression_has_type_grouping_semantics(context, parent_expression_id) =>
        {
            true
        }
        _ => operator_type_or_higher_needs_parentheses(context, node_id, parent_expression_id),
    }
}

/// Return whether one normalized type expression needs parentheses in its parent slot.
fn normalized_type_expression_needs_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    normalized_inner_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(normalized_inner_id) {
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                destack_ast::Declaration::Function { signature, .. }
                    if signature.kind == destack_ast::FunctionKind::Lambda
            ) =>
        {
            function_like_type_needs_parentheses(context, node_id)
        }
        Expression::Binary { .. }
            if expression_has_type_grouping_semantics(context, normalized_inner_id) =>
        {
            associative_type_binary_needs_parentheses(context, node_id, normalized_inner_id)
        }
        Expression::TypeConditional { .. } => conditional_type_needs_parentheses(context, node_id),
        Expression::TypeInfer { .. } => {
            let Some(parent_expression_id) =
                effective_parenthesized_type_parent_expression_id(context, node_id)
            else {
                return false;
            };

            if matches!(
                context.tree.get(parent_expression_id),
                Expression::Binary { .. }
            ) && expression_has_type_grouping_semantics(context, parent_expression_id)
            {
                return true;
            }

            operator_type_or_higher_needs_parentheses(context, node_id, parent_expression_id)
        }
        Expression::TypeUnary { .. } => {
            let Some(parent_expression_id) =
                effective_parenthesized_type_parent_expression_id(context, node_id)
            else {
                return false;
            };

            operator_type_or_higher_needs_parentheses(context, node_id, parent_expression_id)
        }
        _ => false,
    }
}

/// Decide whether a parenthesized type expression can drop wrappers.
pub(crate) fn should_drop_parenthesized_type_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_id: LocalNodeId<Expression>,
) -> bool {
    if parenthesized_wraps_decorated_class_extends_head(context, node_id, inner_id) {
        return false;
    }

    if parenthesized_wraps_prefix_annotated_class_extends_head(context, node_id, inner_id) {
        return false;
    }

    let is_parenthesized_type_position = expression_is_type_position(context, node_id)
        || expression_is_type_position(context, inner_id);
    if !is_parenthesized_type_position {
        return false;
    }

    let normalized_inner_id =
        normalized_parenthesized_type_drop_inner_expression(context, inner_id);
    !normalized_type_expression_needs_parentheses(context, node_id, normalized_inner_id)
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

/// Return static argument slots for expression variants that support type arguments.
pub(crate) fn expression_static_arguments(
    expression: &Expression,
) -> Option<&[LocalNodeId<Argument>]> {
    match expression {
        Expression::QualifiedReference {
            static_arguments, ..
        }
        | Expression::Member {
            static_arguments, ..
        }
        | Expression::PrivateMember {
            static_arguments, ..
        }
        | Expression::Call {
            static_arguments, ..
        }
        | Expression::New {
            static_arguments, ..
        }
        | Expression::TypeImport {
            static_arguments, ..
        } => static_arguments.as_deref(),
        Expression::Instantiation {
            static_arguments, ..
        } => Some(static_arguments.as_slice()),
        _ => None,
    }
}

/// Return whether an expression tree contains static type arguments.
pub(crate) fn expression_has_static_type_arguments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::QualifiedReference {
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
        Expression::Parenthesized { expression } => {
            expression_has_static_type_arguments(context, *expression)
        }
        _ => false,
    }
}

/// Return whether one parent expression propagates type position upward.
fn parent_expression_propagates_type_position(
    context: &DestackFormatContext<'_>,
    current_expression_id: LocalNodeId<Expression>,
    parent_expression_id: LocalNodeId<Expression>,
) -> bool {
    let parent_expression = context.tree.get(parent_expression_id);

    if let Expression::TypeUnary { operator, right } = parent_expression
        && *operator == TypeUnaryOperator::AsConst
        && *right == current_expression_id
    {
        return true;
    }

    if let Expression::Index { left, index, .. } = parent_expression
        && *left == current_expression_id
        && index.is_none()
    {
        return true;
    }

    if let Expression::TypeBinary { left, operator, .. } = parent_expression
        && *left == current_expression_id
        && matches!(
            operator,
            TypeBinaryOperator::Cast
                | TypeBinaryOperator::Satisfies
                | TypeBinaryOperator::Is
                | TypeBinaryOperator::InstanceOf
                | TypeBinaryOperator::In
        )
    {
        return true;
    }

    if let Expression::Binary { left, right, .. } = parent_expression
        && (*left == current_expression_id || *right == current_expression_id)
        && expression_has_type_grouping_semantics(context, parent_expression_id)
    {
        return true;
    }

    false
}

/// Return whether one parent expression establishes type position.
fn parent_expression_establishes_type_position(parent_expression: &Expression) -> bool {
    matches!(
        parent_expression,
        Expression::TypeUnary { .. }
            | Expression::TypeBinary { .. }
            | Expression::TypeConditional { .. }
            | Expression::TypeMapped { .. }
            | Expression::TypeIndex { .. }
            | Expression::TypeTemplateLiteral { .. }
            | Expression::TypeImport { .. }
            | Expression::TypeInfer { .. }
            | Expression::TypePredicate { .. }
    )
}

/// Return whether one expression sits in one explicit or nested type position.
pub(crate) fn expression_is_type_position(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    if context.is_in_type_expression_root(node_id) {
        return true;
    }

    let mut current_id = node_id.id;

    while let Some((parent_id, parent_type)) = context.parent_by_id(current_id) {
        if parent_type != NodeType::Expression {
            return false;
        }

        let current_expression_id = LocalNodeId::<Expression>::new(current_id);
        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        if parent_expression_propagates_type_position(
            context,
            current_expression_id,
            parent_expression_id,
        ) {
            current_id = parent_id;
            continue;
        }

        return parent_expression_establishes_type_position(context.tree.get(parent_expression_id));
    }

    false
}

/// Return whether one expression participates in `|` or `&` type grouping semantics.
pub(crate) fn expression_has_type_grouping_semantics(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    matches!(
        context.tree.get(expression_id),
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
            ..
        }
    ) && expression_is_type_position(context, expression_id)
}
