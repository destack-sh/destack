use super::groups::{TailChainGroup, chain_operation_has_leading_gap_comment};
use super::member::{CallChainPosition, build_member_chain_parts, member_has_intervening_comment};
use super::{
    ChainExpression, ChainExpressionBase, ChainExpressionBaseHead, TailChainGroups,
    assignment_like_parent, chain_operation_is_call_like, chain_operation_is_index,
    chain_operation_node_id, expression_has_ternary_ancestor, expression_trivia_anchor_end,
    first_tail_group_operation, is_lambda_expression, is_nested_lambda_expression,
    member_is_private_hash, transparent_inner_expression,
};
use crate::format::annotation::{
    FormatLeadingComments, FormatTrailingComments, format_trailing_comments,
    infix_or_postfix_annotations, postfix_annotations, prefix_annotations,
};
use crate::format::call::{
    expression_is_long_curried_call, format_call_arguments_in_chain, format_call_expression,
};
use crate::format::context::DestackFormatterSpeculationExt;
use crate::format::expression::{
    format_generic_argument_list, format_generic_argument_list_with_relational_spacing,
};
use crate::format::operator::{is_chain_expression, write_postfix_base_expression};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Comment, CommentPosition, DecoratorPosition, Expression, LocalNodeId, NodeType, PostfixPosition,
};
use destack_fir::format::{Buffer, Format, FormatResult};
use destack_fir::prelude::{
    empty_line, expand_parent, format_with, group, hard_line_break, indent, token,
};
use destack_fir::{best_fitting, write};
use destack_source::Span;

/// Check whether an assignment chain ends in a nested lambda expression.
pub(crate) fn is_assignment_chain_tail_lambda(
    context: &DestackFormatContext<'_>,
    assignment_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    if assignment_like_parent(context, assignment_id).is_none() {
        return false;
    }

    if matches!(context.tree.get(right_id), Expression::Assign { .. }) {
        return false;
    }

    is_nested_lambda_expression(context, right_id)
}

/// One normalized chain layout for the chain formatter.
#[derive(Clone)]
pub(crate) struct MemberChain {
    chain: Vec<LocalNodeId<Expression>>,
    base: ChainExpressionBase,
    tail_groups: TailChainGroups,
}

impl MemberChain {
    /// Build the normalized chain layout for one expression.
    fn from_expression(
        context: &DestackFormatContext<'_>,
        node_id: LocalNodeId<Expression>,
    ) -> FormatResult<Self> {
        let (chain, mut base, mut tail_groups) = build_member_chain_parts(context, node_id)?;
        maybe_merge_first_tail_group_with_head(context, node_id, &mut base, &mut tail_groups);

        Ok(Self {
            chain,
            base,
            tail_groups,
        })
    }

    /// Return whether one normalized call chain has multiple tail groups.
    pub(crate) fn is_member_call_chain(
        context: &DestackFormatContext<'_>,
        node_id: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        Self::from_expression(context, node_id).map(|chain| chain.tail_groups.len() > 1)
    }

    /// Inspect the formatted base and return whether it breaks.
    fn inspect_head_will_break<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        formatted_root_id: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        // speculative formatting
        let start = f.context().expression_token_start(formatted_root_id);
        let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write_chain_base(f, formatted_root_id, &self.base, &self.tail_groups)
        });
        f.speculate_will_break_after(start, &content)
    }

    /// Inspect every tail group and cache whether it breaks.
    fn inspect_tail_groups<'ast>(
        &mut self,
        f: &mut DestackFormatter<'ast, '_>,
        formatted_root_id: LocalNodeId<Expression>,
    ) -> FormatResult<()> {
        let group_count = self.tail_groups.len();

        for group_index in 0..group_count {
            let following_group_first_operation = self
                .tail_groups
                .get(group_index + 1)
                .and_then(|group| group.first())
                .cloned();

            let group = self
                .tail_groups
                .get_mut(group_index)
                .expect("tail chain group inspection should have one group");

            // speculative formatting
            let first_operation = group
                .first()
                .expect("tail chain group inspection should have one first operation");
            let start = f
                .context()
                .expression_token_start(chain_operation_node_id(first_operation));
            let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write_chain_group(
                    f,
                    formatted_root_id,
                    group.as_slice(),
                    following_group_first_operation.as_ref(),
                    false,
                )
            });
            let will_break = f.speculate_will_break_after(start, &content)?;

            group.set_will_break(will_break);
            group.set_needs_empty_line(chain_group_needs_empty_line_before(f.context(), group));
        }

        Ok(())
    }

    /// Return whether the last tail group is a call-like group that breaks.
    fn last_call_breaks(&self) -> bool {
        let Some(last_group) = self.tail_groups.last() else {
            return false;
        };

        last_group.last().is_some_and(chain_operation_is_call_like) && last_group.will_break()
    }

    /// Return whether one call operation has a function-like argument.
    fn call_operation_has_function_like_argument(
        context: &DestackFormatContext<'_>,
        operation: &ChainExpression,
    ) -> bool {
        let ChainExpression::Call { arguments, .. } = operation else {
            return false;
        };

        arguments.iter().copied().any(|argument_id| {
            let Some(argument_value_id) =
                super::argument_value_id_if_present(context.tree, argument_id)
            else {
                return false;
            };

            let argument_value_id = transparent_inner_expression(context, argument_value_id);
            is_lambda_expression(context, argument_value_id)
                || is_nested_lambda_expression(context, argument_value_id)
        })
    }

    /// Return whether the inspected chain groups should force expanded layout.
    fn groups_should_break(
        &self,
        context: &DestackFormatContext<'_>,
        head_will_break: bool,
    ) -> bool {
        let mut has_function_like_argument = false;

        for operation in self
            .base
            .body
            .iter()
            .chain(self.tail_groups.iter().flat_map(|group| group.iter()))
        {
            if !chain_operation_is_call_like(operation) {
                continue;
            }

            has_function_like_argument |=
                Self::call_operation_has_function_like_argument(context, operation);
        }

        if !self.tail_groups.is_empty() && head_will_break {
            return true;
        }

        if self.last_call_breaks() && has_function_like_argument {
            return true;
        }

        self.tail_groups.any_except_last_will_break()
    }

    /// Return whether any member operation owns a comment before its property.
    fn has_member_comment(&self, context: &DestackFormatContext<'_>) -> bool {
        self.base
            .body
            .iter()
            .chain(self.tail_groups.iter().flat_map(|group| group.iter()))
            .any(|operation| {
                let ChainExpression::Member { node_id, .. } = operation else {
                    return false;
                };

                member_has_intervening_comment(context, *node_id)
            })
    }
}

/// Try to merge the first tail group into the chain head.
fn maybe_merge_first_tail_group_with_head(
    context: &DestackFormatContext<'_>,
    _root_id: LocalNodeId<Expression>,
    base: &mut ChainExpressionBase,
    tail_groups: &mut TailChainGroups,
) {
    if !should_merge_first_tail_group_with_head(context, base, tail_groups) {
        return;
    }

    let Some(first_group) = tail_groups.pop_first() else {
        return;
    };

    base.body.extend(first_group.as_slice().iter().cloned());
}

/// Return whether the first tail group should merge into the head.
fn should_merge_first_tail_group_with_head(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
    tail_groups: &TailChainGroups,
) -> bool {
    let Some(first_group) = tail_groups.first() else {
        return false;
    };

    if first_group_has_comment(context, first_group) {
        return false;
    }

    let has_computed_property = first_group
        .first()
        .is_some_and(|operation| matches!(operation, ChainExpression::Index { .. }));

    if base.body.is_empty() {
        match &base.head {
            ChainExpressionBaseHead::Expression(expression_id) => {
                let expression_id = transparent_inner_expression(context, *expression_id);
                match context.tree.get(expression_id) {
                    Expression::Identifier { name, .. } => {
                        has_computed_property
                            || is_factory_name(context, *name)
                            || has_short_name(
                                context.strings.get(*name),
                                context.options.indent_width,
                            )
                    }
                    Expression::QualifiedReference { path, .. } if path.segments.len() == 1 => {
                        has_computed_property || is_factory_name(context, path.segments[0])
                    }
                    Expression::This => true,
                    _ => false,
                }
            }
            ChainExpressionBaseHead::Path { segment, .. } => {
                has_computed_property || is_factory_name(context, *segment)
            }
        }
    } else if let Some(ChainExpression::Member { segment, .. }) = base.body.last() {
        has_computed_property || is_factory_name(context, *segment)
    } else {
        false
    }
}

/// Return whether the first tail group member owns a separator comment.
fn first_group_has_comment(
    context: &DestackFormatContext<'_>,
    first_group: &TailChainGroup,
) -> bool {
    let Some(first_operation) = first_group.first() else {
        return false;
    };

    match first_operation {
        ChainExpression::Member { node_id, .. } => {
            member_has_intervening_comment(context, *node_id)
        }
        _ => false,
    }
}

/// Return whether one identifier name follows the factory-style merge rule.
fn is_factory_name(context: &DestackFormatContext<'_>, string_id: destack_core::StringId) -> bool {
    let name = context.strings.get(string_id);
    let mut bytes = name.bytes();

    match bytes.next() {
        Some(b'_' | b'$') => bytes.all(|byte| matches!(byte, b'_' | b'$')),
        Some(byte) => byte.is_ascii_uppercase(),
        None => false,
    }
}

/// Return whether one identifier fits within the indent-width short-name rule.
fn has_short_name(name: &str, indent_width: u8) -> bool {
    name.len() <= usize::from(indent_width)
}

/// Return whether source preserves an empty line before one tail group.
fn chain_group_needs_empty_line_before(
    context: &DestackFormatContext<'_>,
    group: &TailChainGroup,
) -> bool {
    let Some(first_operation) = group.first() else {
        return false;
    };

    let node_id = chain_operation_node_id(first_operation);
    let Some(left_id) = super::member::chain_node_left_id(context.tree, node_id) else {
        return false;
    };

    // terminal calls that are not already part of a chain do not preserve blank lines
    if let Expression::Call { left, .. } = context.tree.get(left_id)
        && !is_chain_expression(context.tree.get(*left))
    {
        return false;
    }

    let operator_character = match first_operation {
        ChainExpression::Member { .. } => b'.',
        ChainExpression::Index { .. } => b'[',
        _ => return false,
    };

    let start = expression_trivia_anchor_end(context, left_id);
    let mut end = chain_operation_start(context, first_operation).unwrap_or(start);

    if let Some(first_comment) = context
        .comments()
        .comments_before_character(start, operator_character)
        .first()
    {
        end = first_comment.span.start;
    }

    for (index, byte) in context
        .source_text()
        .bytes_range(start, end)
        .iter()
        .enumerate()
    {
        if matches!(byte, b'\n' | b'\r')
            && context.source_text().lines_after(start + index as u32) > 1
        {
            return true;
        }
    }

    false
}

impl<'ast> Format<DestackFormatContext<'ast>> for MemberChain {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let mut chain = self.clone();
        let formatted_root_id = chain.chain[chain.chain.len() - 1];
        let head_will_break = chain.inspect_head_will_break(f, formatted_root_id)?;
        chain.inspect_tail_groups(f, formatted_root_id)?;
        let groups_should_break = chain.groups_should_break(f.context(), head_will_break);
        let has_member_comment = self.has_member_comment(f.context());
        let has_new_line_or_comment_between = chain
            .tail_groups
            .iter()
            .any(|group| group.needs_empty_line());

        let format_one_line_chain = format_with(|f| {
            write_one_line_chain(f, formatted_root_id, &chain.base, &chain.tail_groups)
        });

        let format_expanded_chain = format_with(|f| {
            write_expanded_chain(
                f,
                formatted_root_id,
                groups_should_break,
                &chain.base,
                &chain.tail_groups,
            )
        });

        if chain.tail_groups.len() <= 1 && !has_member_comment && !has_new_line_or_comment_between {
            let should_keep_one_line_without_group =
                leading_call_expression_id(f.context(), &chain.base).is_some_and(|expression_id| {
                    expression_is_long_curried_call(f.context(), expression_id)
                });

            if should_keep_one_line_without_group {
                return write!(f, [format_one_line_chain]);
            }

            return write!(f, [group(&format_one_line_chain)]);
        }

        if has_member_comment || has_new_line_or_comment_between || groups_should_break {
            return write!(f, [group(&format_expanded_chain)]);
        }

        if chain
            .tail_groups
            .last()
            .is_some_and(|group| group.will_break())
        {
            write!(f, [expand_parent()])?;
        }

        write!(
            f,
            [best_fitting![format_one_line_chain, format_expanded_chain]]
        )
    }
}

/// Return the leading call expression in one normalized chain, if any.
fn leading_call_expression_id(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
) -> Option<LocalNodeId<Expression>> {
    match &base.head {
        ChainExpressionBaseHead::Expression(expression_id) => {
            let expression_id = transparent_inner_expression(context, *expression_id);

            if matches!(context.tree.get(expression_id), Expression::Call { .. }) {
                return Some(expression_id);
            }
        }
        ChainExpressionBaseHead::Path { .. } => {}
    }

    base.body.iter().find_map(|operation| match operation {
        ChainExpression::Call { node_id, .. } => Some(*node_id),
        _ => None,
    })
}

/// Format a member/call/maybe/index chain with OXC-shaped breaking.
pub(crate) fn format_expression_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let chain = MemberChain::from_expression(f.context(), node_id)?;
    write!(f, [chain])
}

/// Write the one-line variant of one member chain.
fn write_one_line_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    base: &ChainExpressionBase,
    tail_groups: &TailChainGroups,
) -> FormatResult<()> {
    write_chain_base(f, formatted_root_id, base, tail_groups)?;

    let groups = tail_groups.iter().collect::<Vec<_>>();

    for (group_index, group) in groups.iter().enumerate() {
        let following_group_first_operation =
            groups.get(group_index + 1).and_then(|group| group.first());

        write_chain_group(
            f,
            formatted_root_id,
            group.as_slice(),
            following_group_first_operation,
            false,
        )?;
    }

    Ok(())
}

/// Write the expanded variant of one member chain.
fn write_expanded_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    should_expand_parent: bool,
    base: &ChainExpressionBase,
    tail_groups: &TailChainGroups,
) -> FormatResult<()> {
    // parent expansion
    if should_expand_parent {
        write!(f, [expand_parent()])?;
    }

    // base
    write_chain_base(f, formatted_root_id, base, tail_groups)?;

    // tail groups
    if tail_groups.is_empty() {
        return Ok(());
    }

    let skip_first_soft_break_for_conditional_head =
        expression_has_ternary_ancestor(f.context(), formatted_root_id)
            && base.body.last().is_some_and(chain_operation_is_call_like)
            && tail_groups.first().is_some_and(|group| {
                matches!(
                    group.as_slice(),
                    [
                        ChainExpression::Member { .. },
                        operation
                    ]
                    if chain_operation_is_call_like(operation)
                )
            });
    let format_groups = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        for (group_index, group) in tail_groups.iter().enumerate() {
            let should_skip_first_break =
                group_index == 0 && skip_first_soft_break_for_conditional_head;
            let should_insert_break = !should_skip_first_break
                && (group_index == 0
                    || !group.first().is_some_and(|operation| {
                        let node_id = chain_operation_node_id(operation);

                        f.context()
                            .annotation_ids(node_id)
                            .iter()
                            .any(|annotation_id| {
                                matches!(
                                    f.context().annotation(*annotation_id).position,
                                    DecoratorPosition::BlockPrefix
                                )
                            })
                    }));
            if should_insert_break {
                if group.needs_empty_line() {
                    write!(f, [empty_line()])?;
                } else {
                    write!(f, [hard_line_break()])?;
                }
            }

            write_chain_group(
                f,
                formatted_root_id,
                group.as_slice(),
                tail_groups
                    .iter()
                    .nth(group_index + 1)
                    .and_then(|group| group.first()),
                should_insert_break,
            )?;
        }

        Ok(())
    });

    write!(f, [indent(&format_groups)])?;

    Ok(())
}

/// Return the first token start that begins one chain operation.
fn chain_operation_start(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> Option<u32> {
    match operation {
        ChainExpression::Member { node_id, .. } => {
            return super::member::member_property_start(context, *node_id);
        }
        ChainExpression::Index {
            index: Some(index), ..
        } => {
            return Some(context.span(*index).start);
        }
        _ => {}
    }

    let node_id = chain_operation_node_id(operation);
    let left_id = super::member::chain_node_left_id(context.tree, node_id)?;
    let left_end = expression_trivia_anchor_end(context, left_id);
    let node_span = context.span(node_id);

    context
        .first_non_trivia_token_between(left_end, node_span.end)
        .map(|token| token.span.start)
}

/// Return whether the following call owns the gap comments before its arguments.
fn next_operation_owns_callee_gap_comments(next_operation: Option<&ChainExpression>) -> bool {
    matches!(
        next_operation,
        Some(ChainExpression::Call {
            optional_position: None,
            position: PostfixPosition::Direct,
            generic_arguments,
            arguments,
            ..
        }) if generic_arguments.is_empty() && !arguments.is_empty()
    )
}

/// Return structural trailing comments emitted after one chain segment.
fn chain_structural_trailing_comments(
    context: &DestackFormatContext<'_>,
    preceding_node_id: LocalNodeId<Expression>,
    next_operation: Option<&ChainExpression>,
) -> Vec<Comment> {
    let full_span = context.span(preceding_node_id);

    // structural trailing comments
    let mut comments = context
        .tree
        .get_main_span(preceding_node_id)
        .map(|main_span| {
            context
                .comments()
                .unprinted_comments()
                .iter()
                .copied()
                .filter(|comment| comment.position == CommentPosition::Trailing)
                .filter(|comment| comment.span.file == full_span.file)
                .filter(|comment| {
                    comment.span.start >= main_span.end && comment.span.end <= full_span.end
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let Some(next_operation) = next_operation else {
        comments.sort_by_key(|comment| (comment.span.start, comment.span.end));
        return comments;
    };

    // separator comments before the next hop
    let next_separator_comments = match next_operation {
        ChainExpression::Member { node_id, .. } | ChainExpression::Index { node_id, .. } => {
            let receiver_id = match context.tree.get(*node_id) {
                Expression::Member { left, .. } | Expression::Index { left, .. } => *left,
                _ => return comments,
            };

            context.comments_before_next_non_trivia_token_after_span(context.span(receiver_id))
        }
        _ => Vec::new(),
    };

    comments.extend(
        next_separator_comments
            .into_iter()
            .filter(|comment| !comment.preceded_by_newline()),
    );

    // normalized order
    comments.sort_by_key(|comment| (comment.span.start, comment.span.end));
    comments
}

/// Write separator comments that start on their own line before one chain hop.
fn write_chain_operation_leading_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    op: &ChainExpression,
) -> FormatResult<()> {
    let node_id = match op {
        ChainExpression::Member { node_id, .. } | ChainExpression::Index { node_id, .. } => {
            *node_id
        }
        _ => return Ok(()),
    };

    let receiver_id = match f.context().tree.get(node_id) {
        Expression::Member { left, .. } | Expression::Index { left, .. } => *left,
        _ => return Ok(()),
    };
    let leading_comments = f
        .context()
        .comments_before_next_non_trivia_token_after_span(f.context().span(receiver_id))
        .into_iter()
        .filter(|comment| comment.preceded_by_newline())
        .collect::<Vec<_>>();
    if leading_comments.is_empty() {
        return Ok(());
    }

    write!(f, [FormatLeadingComments::Comments(&leading_comments)])
}

/// Write trailing comments between one formatted node and its following operation.
fn write_chain_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    preceding_node_id: LocalNodeId<Expression>,
    preceding_span: Span,
    next_operation: Option<&ChainExpression>,
) -> FormatResult<()> {
    let Some(next_operation) = next_operation else {
        let structural_comments =
            chain_structural_trailing_comments(f.context(), preceding_node_id, None);
        if structural_comments.is_empty() {
            return Ok(());
        }

        return write!(f, [FormatTrailingComments::Comments(&structural_comments)]);
    };

    if next_operation_owns_callee_gap_comments(Some(next_operation)) {
        return Ok(());
    }

    let structural_comments =
        chain_structural_trailing_comments(f.context(), preceding_node_id, Some(next_operation));
    if !structural_comments.is_empty() {
        return write!(f, [FormatTrailingComments::Comments(&structural_comments)]);
    }

    let following_span_start = chain_operation_start(f.context(), next_operation).unwrap_or(0);

    write!(
        f,
        [format_trailing_comments(
            f.context().span(formatted_root_id),
            preceding_span,
            following_span_start,
        )]
    )
}

/// Format the unwrapped base segment of a chain.
fn write_chain_base<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    base: &ChainExpressionBase,
    tail_groups: &TailChainGroups,
) -> FormatResult<()> {
    let root_is_decorator_expression = f
        .context()
        .parent(formatted_root_id)
        .is_some_and(|(_, parent_type)| matches!(parent_type, NodeType::Decorator));
    let skip_base_head_for_start_call = base_head_is_owned_by_start_call(base);

    if !skip_base_head_for_start_call {
        match &base.head {
            ChainExpressionBaseHead::Path {
                node_id,
                segment,
                generic_arguments,
                emit_postfix_annotations,
            } => {
                if !root_is_decorator_expression {
                    write!(f, [prefix_annotations(f.context(), *node_id)])?;
                }
                write!(f, [*segment])?;
                if !generic_arguments.is_empty() {
                    let next_operation = base
                        .body
                        .first()
                        .or_else(|| first_tail_group_operation(tail_groups));
                    if next_operation.is_some_and(chain_operation_is_index) {
                        format_generic_argument_list_with_relational_spacing(f, generic_arguments)?;
                    } else {
                        format_generic_argument_list(f, generic_arguments)?;
                    }
                }
                if *emit_postfix_annotations {
                    write!(f, [infix_or_postfix_annotations(f.context(), *node_id)])?;
                }
            }
            ChainExpressionBaseHead::Expression(node_id) => {
                // nested chain nodes can still appear as the base here
                write_postfix_base_expression(f, *node_id)?;
                write!(f, [infix_or_postfix_annotations(f.context(), *node_id)])?;

                let first_continuation = base
                    .body
                    .first()
                    .or_else(|| first_tail_group_operation(tail_groups));

                write_chain_trailing_comments(
                    f,
                    formatted_root_id,
                    *node_id,
                    f.context().span(*node_id),
                    first_continuation,
                )?;
            }
        }
    }

    for (index, op) in base.body.iter().enumerate() {
        let next_operation = base
            .body
            .get(index + 1)
            .or_else(|| first_tail_group_operation(tail_groups));
        write_chain_operation(f, formatted_root_id, op, next_operation)?;
    }

    Ok(())
}

/// Return whether the first call operation prints the normalized base itself.
fn base_head_is_owned_by_start_call(base: &ChainExpressionBase) -> bool {
    matches!(
        (&base.head, base.body.first()),
        (
            ChainExpressionBaseHead::Expression(_),
            Some(ChainExpression::Call {
                call_position: CallChainPosition::Start,
                ..
            })
        )
    )
}

/// Return prefix and postfix emission flags for one chain operation.
fn chain_operation_annotation_emit_flags(
    context: &DestackFormatContext<'_>,
    formatted_root_id: LocalNodeId<Expression>,
    operation: &ChainExpression,
) -> (LocalNodeId<Expression>, bool, bool) {
    let (node_id, emit_prefix_annotations, emit_postfix_annotations) = match operation {
        ChainExpression::Member {
            node_id,
            emit_prefix_annotations,
            emit_postfix_annotations,
            ..
        } => (
            *node_id,
            *emit_prefix_annotations,
            *emit_postfix_annotations,
        ),
        ChainExpression::Instantiation { node_id, .. }
        | ChainExpression::Call { node_id, .. }
        | ChainExpression::Index { node_id, .. }
        | ChainExpression::Maybe { node_id, .. }
        | ChainExpression::Must { node_id, .. } => {
            let should_emit_prefix_annotations = match operation {
                ChainExpression::Call { node_id, .. } => match context.tree.get(*node_id) {
                    Expression::Call { left, .. } => {
                        if !context.has_prefix_annotation(*node_id) {
                            false
                        } else if !context.has_prefix_annotation(*left) {
                            true
                        } else {
                            let left_span = context.span(*left);
                            let has_leading_prefix_before_left = context
                                .annotation_ids(*node_id)
                                .iter()
                                .any(|annotation_id| {
                                    let annotation = context.annotation(*annotation_id);
                                    if !matches!(
                                        annotation.position,
                                        DecoratorPosition::BlockPrefix
                                            | DecoratorPosition::LinePrefix
                                    ) {
                                        return false;
                                    }

                                    let annotation_span = context.annotation_span(*annotation_id);
                                    annotation_span.start < left_span.start
                                });

                            !has_leading_prefix_before_left
                        }
                    }
                    _ => context.has_prefix_annotation(*node_id),
                },
                _ => true,
            };
            (*node_id, should_emit_prefix_annotations, true)
        }
    };

    // separator comments between chain hops belong to the chain layout
    let emit_prefix_annotations =
        emit_prefix_annotations && !chain_operation_has_leading_gap_comment(context, operation);

    let emit_prefix_annotations = emit_prefix_annotations && node_id != formatted_root_id;
    let root_postfix_owned_by_outer_context = node_id == formatted_root_id;
    let emit_postfix_annotations = emit_postfix_annotations && !root_postfix_owned_by_outer_context;

    (node_id, emit_prefix_annotations, emit_postfix_annotations)
}

/// Write prefix annotations for one chain operation when it owns them.
fn write_chain_operation_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    emit_prefix_annotations: bool,
) -> FormatResult<()> {
    if emit_prefix_annotations {
        write!(f, [prefix_annotations(f.context(), node_id)])?;
    }

    Ok(())
}

/// Write one absorbed optional-chain marker before its owning operation.
fn write_chain_operation_optional_marker<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _node_id: LocalNodeId<Expression>,
    optional_position: Option<PostfixPosition>,
) -> FormatResult<()> {
    let Some(optional_position) = optional_position else {
        return Ok(());
    };

    match optional_position {
        PostfixPosition::Direct => write!(f, [token("?")])?,
        PostfixPosition::Indirect => write!(f, [token("."), token("?")])?,
    }

    Ok(())
}

/// Write postfix annotations for one chain operation when it owns them.
fn write_chain_operation_postfix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    emit_postfix_annotations: bool,
    call_or_new_handles_empty_infix: bool,
) -> FormatResult<()> {
    if !emit_postfix_annotations {
        return Ok(());
    }

    if call_or_new_handles_empty_infix {
        write!(f, [postfix_annotations(f.context(), node_id)])?;
    } else {
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;
    }

    Ok(())
}

/// Format one chained operation.
fn write_chain_operation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    op: &ChainExpression,
    next_operation: Option<&ChainExpression>,
) -> FormatResult<()> {
    let (node_id, emit_prefix_annotations, emit_postfix_annotations) =
        chain_operation_annotation_emit_flags(f.context(), formatted_root_id, op);
    let operation_span = f.context().span(node_id);
    let call_or_new_handles_empty_infix = matches!(
        op,
        ChainExpression::Call {
            node_id,
            arguments,
            ..
        } if arguments.is_empty() && f.context().has_infix_annotation(*node_id)
    );
    write_chain_operation_prefix(f, node_id, emit_prefix_annotations)?;
    write_chain_operation_leading_comments(f, op)?;

    match op {
        ChainExpression::Member {
            node_id,
            optional_position,
            segment,
            generic_arguments,
            ..
        } => {
            write_chain_operation_optional_marker(f, *node_id, *optional_position)?;

            let is_private_hash = member_is_private_hash(f.context(), *node_id);
            write!(f, [token(".")])?;
            if is_private_hash {
                write!(f, [token("#")])?;
            }
            write!(f, [*segment])?;
            if !generic_arguments.is_empty() {
                if next_operation.is_some_and(chain_operation_is_index) {
                    format_generic_argument_list_with_relational_spacing(f, generic_arguments)?;
                } else {
                    format_generic_argument_list(f, generic_arguments)?;
                }
            }
        }
        ChainExpression::Instantiation {
            generic_arguments, ..
        } => {
            if next_operation.is_some_and(chain_operation_is_index) {
                format_generic_argument_list_with_relational_spacing(f, generic_arguments)?;
            } else {
                format_generic_argument_list(f, generic_arguments)?;
            }
        }
        ChainExpression::Call {
            node_id: call_node_id,
            call_position,
            optional_position,
            position,
            generic_arguments,
            arguments,
        } => {
            if *call_position == CallChainPosition::Start {
                format_call_expression(f, *call_node_id)?;
            } else {
                write_chain_operation_optional_marker(f, *call_node_id, *optional_position)?;
                if *position == PostfixPosition::Indirect {
                    write!(f, [token(".")])?;
                }
                if !generic_arguments.is_empty() {
                    format_generic_argument_list(f, generic_arguments)?;
                }
                format_call_arguments_in_chain(f, *call_node_id, arguments)?;
            }
        }
        ChainExpression::Index {
            node_id,
            optional_position,
            position,
            index,
            ..
        } => {
            write_chain_operation_optional_marker(f, *node_id, *optional_position)?;
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            if let Some(index) = index {
                let inner_index_id = transparent_inner_expression(f.context(), *index);
                let should_parenthesize = matches!(
                    f.context().tree.get(inner_index_id),
                    Expression::Assign { .. }
                );
                if should_parenthesize {
                    write!(f, [token("["), token("("), *index, token(")"), token("]")])?;
                } else {
                    write!(f, [token("["), *index, token("]")])?;
                }
            } else {
                write!(f, [token("[]")])?;
            }
        }
        ChainExpression::Maybe { position, .. } => match position {
            PostfixPosition::Direct => write!(f, [token("?")])?,
            PostfixPosition::Indirect => {
                write!(f, [token("."), token("?")])?;
            }
        },
        ChainExpression::Must { position, .. } => match position {
            PostfixPosition::Direct => write!(f, [token("!")])?,
            PostfixPosition::Indirect => {
                write!(f, [token("."), token("!")])?;
            }
        },
    }

    write_chain_operation_postfix(
        f,
        node_id,
        emit_postfix_annotations,
        call_or_new_handles_empty_infix,
    )?;

    write_chain_trailing_comments(
        f,
        formatted_root_id,
        node_id,
        operation_span,
        next_operation,
    )
}

/// Format all operations for one chain line.
fn write_chain_group<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    ops: &[ChainExpression],
    following_group_first_operation: Option<&ChainExpression>,
    _skip_first_member_gap_comments: bool,
) -> FormatResult<()> {
    for (index, op) in ops.iter().enumerate() {
        let next_operation = ops.get(index + 1).or(following_group_first_operation);
        write_chain_operation(f, formatted_root_id, op, next_operation)?;
    }
    Ok(())
}
