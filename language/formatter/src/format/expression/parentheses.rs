use super::super::timing::tags;
use super::*;
use crate::scan::next_non_whitespace_after_span;

/// Parenthesized unwrap policy for expression contexts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ParenthesizedUnwrapPolicy {
    /// Unwrap when the value is used as a member object.
    MemberObject,
    /// Unwrap when the value is a `new` callee wrapper.
    NewMemberCallee,
}

/// Parenthesized drop policy for type contexts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ParenthesizedDropPolicy {
    /// Drop wrappers in generic expression contexts.
    ExpressionWrapper,
    /// Drop wrappers around type-binary left operands.
    TypeBinaryLeft { node_id: LocalNodeId<Expression> },
}

/// Decide whether a parenthesized expression should unwrap under a policy.
pub(super) fn parenthesized_should_unwrap(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    policy: ParenthesizedUnwrapPolicy,
) -> bool {
    match policy {
        ParenthesizedUnwrapPolicy::MemberObject => should_unwrap_parenthesized_member_object(
            context,
            parenthesized_id,
            inner_expression_id,
        ),
        ParenthesizedUnwrapPolicy::NewMemberCallee => {
            should_unwrap_parenthesized_new_member_callee(
                context,
                parenthesized_id,
                inner_expression_id,
            )
        }
    }
}

/// Decide whether a parenthesized expression should drop wrappers under a policy.
pub(super) fn parenthesized_should_drop(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    policy: ParenthesizedDropPolicy,
) -> bool {
    match policy {
        ParenthesizedDropPolicy::ExpressionWrapper => should_drop_parenthesized_expression_wrapper(
            context,
            parenthesized_id,
            inner_expression_id,
        ),
        ParenthesizedDropPolicy::TypeBinaryLeft { node_id } => {
            should_drop_type_binary_left_parentheses(
                context,
                node_id,
                parenthesized_id,
                inner_expression_id,
            )
        }
    }
}

/// Return whether `new` callee formatting should keep member-object parentheses.
pub(super) fn parenthesized_prefers_new_member_callee_parentheses(
    context: &DestackFormatContext<'_>,
    object_id: LocalNodeId<Expression>,
) -> bool {
    member_object_prefers_new_callee_parentheses(context, object_id)
}

/// Collect postfix star comments from an inner expression that should render after `)`.
pub(super) fn collect_parenthesized_boundary_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> Vec<String> {
    let _timing =
        context.timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_PARENTHESES_BOUNDARY_COMMENTS);
    if matches!(
        context.tree.get(inner_expression_id),
        Expression::TreeExpression { .. }
    ) {
        return Vec::new();
    }

    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        return Vec::new();
    }

    let parenthesized_span = context.get_span(parenthesized_id);
    let inner_span = context.get_span(inner_expression_id);
    if parenthesized_span.file != inner_span.file || inner_span.end >= parenthesized_span.end {
        return Vec::new();
    }

    let boundary_span = Span::new(
        parenthesized_span.file,
        inner_span.end,
        parenthesized_span.end,
    );
    if !context.has_comment(boundary_span) {
        return Vec::new();
    }

    let comment_tokens = context.comment_tokens();
    let first_relevant_index =
        comment_tokens.partition_point(|comment_token| comment_token.span.end < inner_span.end);

    let mut comments: Vec<(u32, String)> = Vec::new();
    for comment_token in comment_tokens[first_relevant_index..].iter().copied() {
        if comment_token.span.start > parenthesized_span.end {
            break;
        }
        if !matches!(
            comment_token.token.ty,
            TokenType::BlockComment | TokenType::DocBlockComment
        ) {
            continue;
        }
        if comment_token.span.start < inner_span.end
            || comment_token.span.end > parenthesized_span.end
        {
            continue;
        }

        let comment_source = context.get_token_str(comment_token).trim();
        if comment_source.is_empty() {
            continue;
        }
        if next_non_whitespace_after_span(context, comment_token.span) != Some(')') {
            continue;
        }

        comments.push((comment_token.span.start, comment_source.to_string()));
    }

    comments.sort_by_key(|(start, _)| *start);
    comments.into_iter().map(|(_, source)| source).collect()
}

/// Return whether source contains leading trivia between `(` and the inner expression.
pub(super) fn parenthesized_has_leading_inner_trivia(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    parenthesized_has_leading_inner_pattern(context, parenthesized_id, inner_expression_id, true)
}

/// Return whether source contains leading comments between `(` and the inner expression.
pub(super) fn parenthesized_has_leading_inner_comments(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    parenthesized_has_leading_inner_pattern(context, parenthesized_id, inner_expression_id, false)
}

/// Return whether source contains a newline between `(` and the inner expression.
pub(super) fn parenthesized_has_leading_inner_newline(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let parenthesized_span = context.get_span(parenthesized_id);
    let inner_span = context.get_span(inner_expression_id);

    let leading_start = parenthesized_span.start.saturating_add(1);
    if leading_start >= inner_span.start || parenthesized_span.file != inner_span.file {
        return false;
    }

    let leading_span = Span::new(parenthesized_span.file, leading_start, inner_span.start);
    context.has_newline(leading_span)
}

/// Return whether source contains leading comment or newline trivia between `(` and inner.
fn parenthesized_has_leading_inner_pattern(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
    include_newline: bool,
) -> bool {
    let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_PARENTHESES_LEADING_TRIVIA);
    let parenthesized_span = context.get_span(parenthesized_id);
    let inner_span = context.get_span(inner_expression_id);

    let leading_start = parenthesized_span.start.saturating_add(1);
    if leading_start >= inner_span.start || parenthesized_span.file != inner_span.file {
        return false;
    }

    let leading_span = Span::new(parenthesized_span.file, leading_start, inner_span.start);
    if include_newline && context.has_newline(leading_span) {
        return true;
    }

    span_has_comment(context, leading_span)
}

/// Return whether one expression chain contains optional chaining semantics.
fn expression_chain_has_optional_maybe(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id;

    loop {
        match context.tree.get(current_id) {
            Expression::Maybe { .. } => return true,
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Call { left, .. }
            | Expression::Must { left, .. }
            | Expression::Instantiation { left, .. } => current_id = *left,
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                current_id = *expression;
            }
            _ => return false,
        }
    }
}

/// Return whether one member expression contains optional chaining semantics.
pub(super) fn member_expression_has_optional_chain(
    context: &DestackFormatContext<'_>,
    member_id: LocalNodeId<Expression>,
) -> bool {
    expression_chain_has_optional_maybe(context, member_id)
}

/// Decide whether a parenthesized expression can be unwrapped in member object position.
pub(super) fn should_unwrap_parenthesized_member_object(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    // object members require explicit grouping: `({}).x`
    if matches!(
        context.tree.get(inner_expression_id),
        Expression::ObjectExpression { .. }
    ) {
        return false;
    }

    // keep nested grouping in type contexts stable across repeated formatting
    if is_type_context(context, parenthesized_id) {
        return false;
    }

    // decorated class expressions require explicit grouping before member access
    if expression_is_decorated_class_declaration(context, inner_expression_id) {
        return false;
    }

    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        // allow unwrapping only when inner annotations are prefix comments or docs
        if !expression_has_only_prefix_comment_or_doc_annotations(context, inner_expression_id) {
            return false;
        }
    }

    if parenthesized_has_leading_inner_comments(context, parenthesized_id, inner_expression_id)
        && !expression_has_only_prefix_comment_or_doc_annotations(context, inner_expression_id)
    {
        return false;
    }

    !needs_parens_in_postfix_position(context.tree, inner_expression_id)
}

/// Return whether a member object should keep parentheses as a `new` callee.
pub(super) fn member_object_prefers_new_callee_parentheses(
    context: &DestackFormatContext<'_>,
    object_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = object_id;

    while let Expression::Parenthesized { expression } = context.tree.get(current_id) {
        if context.has_annotation(current_id)
            || parenthesized_has_leading_inner_trivia(context, current_id, *expression)
        {
            return false;
        }
        current_id = *expression;
    }

    matches!(
        context.tree.get(current_id),
        Expression::Call { .. } | Expression::Instantiation { .. }
    )
}

/// Return whether a member object is simple enough for `new a.b()` style callee formatting.
pub(super) fn is_simple_new_member_object(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match tree.get(expression_id) {
        Expression::Path { .. }
        | Expression::This
        | Expression::Super
        | Expression::PrivateIdentifier { .. } => true,
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            is_simple_new_member_object(tree, *left)
        }
        Expression::Parenthesized { expression } => is_simple_new_member_object(tree, *expression),
        _ => false,
    }
}

/// Decide whether `new (<member>)()` can unwrap outer parentheses.
pub(super) fn should_unwrap_parenthesized_new_member_callee(
    context: &DestackFormatContext<'_>,
    parenthesized_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    if context.has_annotation(parenthesized_id) || context.has_annotation(inner_expression_id) {
        return false;
    }

    if parenthesized_has_leading_inner_trivia(context, parenthesized_id, inner_expression_id) {
        return false;
    }

    match context.tree.get(inner_expression_id) {
        Expression::Path { .. } => true,
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => {
            if member_expression_has_optional_chain(context, inner_expression_id) {
                return false;
            }

            is_simple_new_member_object(context.tree, *left)
        }
        _ => false,
    }
}

/// Remove one surrounding pair of parentheses from text when present.
pub(super) fn strip_one_wrapping_parentheses(source: &str) -> &str {
    let trimmed = source.trim();
    if trimmed.len() < 2 || !trimmed.starts_with('(') || !trimmed.ends_with(')') {
        return trimmed;
    }
    trimmed[1..trimmed.len() - 1].trim()
}

/// Keep parentheses for cast or satisfies expressions in statement position.
pub(super) fn type_binary_is_statement_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    matches!(
        context.tree.get(parent_id),
        Expression::Statement(inner) if *inner == node_id
    )
}

/// Return whether a type-binary expression is wrapped by one statement parenthesized node.
pub(super) fn type_binary_is_parenthesized_statement_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Parenthesized { expression } = context.tree.get(parent_id) else {
        return false;
    };
    if *expression != node_id {
        return false;
    }

    let Some((grandparent_id, grandparent_type)) = context.get_parent(parent_id) else {
        return false;
    };
    if grandparent_type != NodeType::Expression {
        return false;
    }

    let grandparent_id = LocalNodeId::<Expression>::new(grandparent_id);
    matches!(
        context.tree.get(grandparent_id),
        Expression::Statement(inner_id) if *inner_id == parent_id
    )
}

/// Return whether any parenthesized expression ancestor has leading inner trivia.
pub(super) fn has_parenthesized_ancestor_with_leading_inner_trivia(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = node_id;

    while let Some((parent_id, parent_type)) = context.get_parent(current_id) {
        if parent_type != NodeType::Expression {
            break;
        }

        let parent_id = LocalNodeId::<Expression>::new(parent_id);
        if let Expression::Parenthesized { expression } = context.tree.get(parent_id)
            && *expression == current_id
            && parenthesized_has_leading_inner_trivia(context, parent_id, current_id)
        {
            return true;
        }

        current_id = parent_id;
    }

    false
}

/// Return whether cast or satisfies appears as the parenthesized callee of a `new` expression.
pub(super) fn type_binary_is_parenthesized_new_callee(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::Parenthesized { expression } = context.tree.get(parent_id) else {
        return false;
    };
    if *expression != node_id {
        return false;
    }

    let Some((grandparent_id, grandparent_type)) = context.get_parent(parent_id) else {
        return false;
    };
    if grandparent_type != NodeType::Expression {
        return false;
    }

    let grandparent_id = LocalNodeId::<Expression>::new(grandparent_id);
    matches!(
        context.tree.get(grandparent_id),
        Expression::New { left, .. } if *left == parent_id
    )
}

/// Decide whether cast or satisfies can drop a parenthesized left side.
pub(super) fn should_drop_type_binary_left_parentheses(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    parenthesized_id: LocalNodeId<Expression>,
    left_id: LocalNodeId<Expression>,
) -> bool {
    let left_is_cast_chain = matches!(
        context.tree.get(left_id),
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    );

    if context.has_annotation(parenthesized_id) {
        return false;
    }

    if context.has_annotation(left_id) && !left_is_cast_chain {
        return false;
    }

    if parenthesized_has_leading_inner_trivia(context, parenthesized_id, left_id)
        && !left_is_cast_chain
    {
        return false;
    }

    if type_binary_is_statement_expression(context, node_id)
        || type_binary_is_parenthesized_statement_expression(context, node_id)
    {
        return false;
    }

    if has_parenthesized_ancestor_with_leading_inner_trivia(context, node_id) && !left_is_cast_chain
    {
        return false;
    }

    is_simple_type_binary_left_expression(context.tree, left_id)
}

/// Return whether this parenthesized expression is a top-level type alias value.
pub(super) fn parenthesized_is_top_level_type_alias_value(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    matches!(
        context.tree.get(declaration_id),
        Declaration::Type { value, .. } if *value == node_id
    )
}

/// Return whether an expression starts with a type union or intersection chain.
pub(super) fn expression_is_type_binary_chain_head(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::Parenthesized { expression } => {
            expression_is_type_binary_chain_head(context, *expression)
        }
        Expression::Binary { operator, .. } => {
            matches!(
                operator,
                BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
            ) && is_type_context(context, expression_id)
        }
        _ => false,
    }
}

/// Return whether parenthesized source starts with a leading `|` or `&` operator.
pub(super) fn parenthesized_source_leading_type_grouping_operator(
    source: &str,
) -> Option<BinaryOperator> {
    let mut remaining = source.trim_start();
    if let Some(after_parenthesis) = remaining.strip_prefix('(') {
        remaining = after_parenthesis;
    }

    loop {
        remaining = remaining.trim_start();

        if let Some(after_block_comment) = remaining.strip_prefix("/*") {
            let comment_end = after_block_comment.find("*/")?;
            remaining = &after_block_comment[comment_end + 2..];
            continue;
        }

        if let Some(after_line_comment) = remaining.strip_prefix("//") {
            if let Some(line_end) = after_line_comment.find('\n') {
                remaining = &after_line_comment[line_end + 1..];
                continue;
            }
            return None;
        }

        break;
    }

    match remaining.chars().next() {
        Some('|') => Some(BinaryOperator::ElementwiseOr),
        Some('&') => Some(BinaryOperator::ElementwiseAnd),
        _ => None,
    }
}

/// Decide whether a parenthesized type expression can drop wrappers.
pub(super) fn should_drop_parenthesized_type_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_id: LocalNodeId<Expression>,
) -> bool {
    let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_PARENTHESES_TYPE_DROP);

    // type wrappers with annotations are semantic boundaries, not redundant parens
    if context.has_annotation(node_id) || context.has_annotation(inner_id) {
        return false;
    }

    // decorated class extends heads must preserve explicit grouping
    if parenthesized_wraps_decorated_class_extends_head(context, node_id, inner_id) {
        return false;
    }

    if !is_type_context(context, node_id) {
        return false;
    }

    // drop redundant simple wrappers in array element position: `(number)[]` -> `number[]`
    if let Some((parent_id, parent_type)) = context.get_parent(node_id)
        && parent_type == NodeType::Expression
    {
        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        if let Expression::Index { left, index, .. } = context.tree.get(parent_expression_id)
            && *left == node_id
            && index.is_none()
            && !context.has_annotation(node_id)
            && !context.has_annotation(inner_id)
            && is_simple_type_binary_left_expression(context.tree, inner_id)
        {
            return true;
        }
    }

    if parenthesized_associative_type_binary_can_drop(context, node_id, inner_id) {
        return true;
    }

    if parenthesized_is_top_level_type_alias_value(context, node_id)
        && expression_is_type_binary_chain_head(context, inner_id)
    {
        return true;
    }

    if !parenthesized_type_grouping_drop_is_safe_in_parent(context, node_id) {
        return false;
    }

    let node_span = context.get_span(node_id);
    let node_source = context.get_span_str(node_span);
    let leading_grouping_operator =
        parenthesized_source_leading_type_grouping_operator(node_source);
    let has_leading_union_source = node_source.trim_start().starts_with('|');
    if previous_non_whitespace_before_span(context, node_span) != Some('|')
        && !has_leading_union_source
        && leading_grouping_operator.is_none()
    {
        return false;
    }

    if let Some(leading_operator) = leading_grouping_operator {
        if let Expression::Binary { operator, .. } = context.tree.get(inner_id)
            && *operator == leading_operator
            && is_type_context(context, inner_id)
        {
            return true;
        } else if let Expression::Parenthesized { expression } = context.tree.get(inner_id)
            && (matches!(
                context.tree.get(*expression),
                Expression::Binary { operator, .. }
                    if *operator == leading_operator && is_type_context(context, *expression)
            ) || is_simple_type_binary_left_expression(context.tree, *expression))
        {
            return true;
        } else if is_simple_type_binary_left_expression(context.tree, inner_id) {
            return true;
        }
    }

    match context.tree.get(inner_id) {
        Expression::TypeConditional { .. } => true,
        Expression::Declaration(declaration_id) => matches!(
            context.tree.get(*declaration_id),
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
        ),
        _ => false,
    }
}

/// Return whether dropping a parenthesized type grouping is safe in the parent expression context.
fn parenthesized_type_grouping_drop_is_safe_in_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return true;
    };
    if parent_type != NodeType::Expression {
        return true;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_expression_id) {
        Expression::Index { left, .. } | Expression::TypeIndex { left, .. } => *left != node_id,
        _ => true,
    }
}

/// Return whether a binary operator is associative in type contexts.
pub(super) fn is_associative_type_binary_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
    )
}

/// Decide whether an associative type binary can drop redundant wrappers.
pub(super) fn parenthesized_associative_type_binary_can_drop(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Binary {
        operator: inner_operator,
        ..
    } = context.tree.get(inner_id)
    else {
        return false;
    };
    if !is_associative_type_binary_operator(*inner_operator) {
        return false;
    }

    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_id) {
        Expression::Parenthesized { expression } => *expression == node_id,
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            (*left == node_id || *right == node_id)
                && *operator == *inner_operator
                && is_type_context(context, parent_id)
        }
        _ => false,
    }
}

/// Decide whether a parenthesized expression should drop wrappers in generic expression contexts.
fn should_drop_parenthesized_expression_wrapper(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let _timing = context.timing_scope(tags::FORMAT_EXPRESSION_PRIMARY_PARENTHESES_DROP_POLICY);
    // decorated class extends heads must keep explicit grouping
    if parenthesized_wraps_decorated_class_extends_head(context, node_id, inner_expression_id) {
        return false;
    }

    // closure-style cast wrappers in class heritage should stay explicit
    if parenthesized_wraps_prefix_annotated_class_extends_head(
        context,
        node_id,
        inner_expression_id,
    ) {
        return false;
    }

    let should_drop_type_parentheses =
        should_drop_parenthesized_type_expression(context, node_id, inner_expression_id);

    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return should_drop_type_parentheses;
    };
    if parent_type != NodeType::Expression {
        // call/new arguments can unwrap decorated class expressions
        let should_drop_argument_decorated_class_wrapper = parent_type == NodeType::Argument
            && !context.has_annotation(node_id)
            && !parenthesized_has_leading_inner_newline(context, node_id, inner_expression_id)
            && expression_is_decorated_class_declaration(context, inner_expression_id);
        if should_drop_argument_decorated_class_wrapper {
            return true;
        }

        // declarator wrappers can drop when left spine carries prefix comment/doc annotations
        let should_drop_declarator_prefix_wrapper = parent_type == NodeType::Declarator
            && !context.has_annotation(node_id)
            && !parenthesized_has_leading_inner_newline(context, node_id, inner_expression_id)
            && expression_has_prefix_comment_or_doc_annotation_in_left_spine(
                context,
                inner_expression_id,
            );
        if should_drop_declarator_prefix_wrapper {
            return true;
        }

        return should_drop_type_parentheses;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let parent_expression = context.tree.get(parent_id);
    let inner_expression = context.tree.get(inner_expression_id);
    let should_drop_statement_type_binary_wrapper = matches!(
        parent_expression,
        Expression::Statement(inner_id) if inner_id.id == node_id.id
    ) && matches!(
        inner_expression,
        Expression::TypeBinary {
            operator: TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies,
            ..
        }
    ) && !context.has_annotation(node_id);
    let should_drop_assignment_must = matches!(
        parent_expression,
        Expression::Assign { left, .. } if *left == node_id
    ) && matches!(inner_expression, Expression::Must { .. });
    let should_drop_statement_lambda = matches!(
        parent_expression,
        Expression::Statement(inner_id) if inner_id.id == node_id.id
    ) && matches!(
        inner_expression,
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
            )
    ) && !context.has_annotation(node_id);
    should_drop_statement_type_binary_wrapper
        || should_drop_assignment_must
        || should_drop_statement_lambda
        || should_drop_type_parentheses
}

/// Return whether a declaration expression is a decorated class declaration.
fn expression_is_decorated_class_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };
    let Declaration::Class { .. } = context.tree.get(*declaration_id) else {
        return false;
    };

    let expression_has_decorator = context.with_annotations(expression_id, |annotations| {
        annotations.iter().any(|annotation_id| {
            matches!(
                context.get_annotation(*annotation_id),
                Annotation::Decorator { .. }
            )
        })
    });
    if expression_has_decorator.unwrap_or(false) {
        return true;
    }

    context
        .with_annotations(*declaration_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                matches!(
                    context.get_annotation(*annotation_id),
                    Annotation::Decorator { .. }
                )
            })
        })
        .unwrap_or(false)
}

/// Return whether a parenthesized expression wraps a decorated class in `extends`.
fn parenthesized_wraps_decorated_class_extends_head(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let extends_types = match context.tree.get(declaration_id) {
        Declaration::Class { heritage, .. } => heritage.extends_types.as_deref(),
        _ => None,
    };
    let Some(extends_types) = extends_types else {
        return false;
    };

    extends_types.contains(&node_id)
        && expression_is_decorated_class_declaration(context, inner_expression_id)
}

/// Return whether a parenthesized extends head carries prefix comment/doc annotations.
fn parenthesized_wraps_prefix_annotated_class_extends_head(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    inner_expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.get_parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let extends_types = match context.tree.get(declaration_id) {
        Declaration::Class { heritage, .. } => heritage.extends_types.as_deref(),
        _ => None,
    };
    let Some(extends_types) = extends_types else {
        return false;
    };
    if !extends_types.contains(&node_id) {
        return false;
    }

    expression_has_prefix_comment_or_doc_annotation_in_left_spine(context, inner_expression_id)
}

/// Return whether annotations are only prefix comment/doc markers for this expression.
fn expression_has_only_prefix_comment_or_doc_annotations(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .with_annotations(expression_id, |annotations| {
            !annotations.is_empty()
                && annotations.iter().all(|annotation_id| {
                    matches!(
                        context.get_annotation(*annotation_id),
                        Annotation::Comment {
                            position: AnnotationPosition::LinePrefix
                                | AnnotationPosition::BlockPrefix,
                            ..
                        } | Annotation::Doc {
                            position: AnnotationPosition::LinePrefix
                                | AnnotationPosition::BlockPrefix,
                            ..
                        }
                    )
                })
        })
        .unwrap_or(false)
}

/// Return whether any expression on the left spine has a prefix comment/doc annotation.
fn expression_has_prefix_comment_or_doc_annotation_in_left_spine(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id;

    loop {
        if expression_has_only_prefix_comment_or_doc_annotations(context, current_id) {
            return true;
        }

        let next_id = match context.tree.get(current_id) {
            Expression::Parenthesized { expression } => Some(*expression),
            Expression::Call { left, .. }
            | Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. }
            | Expression::TypeBinary { left, .. }
            | Expression::Binary { left, .. } => Some(*left),
            _ => None,
        };

        let Some(next_id) = next_id else {
            break;
        };
        current_id = next_id;
    }

    false
}
