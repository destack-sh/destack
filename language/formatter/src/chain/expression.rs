use super::groups::{MemberChainGroup, chain_member_has_leading_gap_comment};
use super::member::{
    CallExpressionPosition, build_member_chain_parts, member_has_intervening_comment,
};
use super::{
    ChainMember, ChainRoot, TailChainGroups, assignment_like_parent, chain_member_is_call_like,
    chain_member_is_index, chain_member_node_id, expression_has_ternary_ancestor,
    expression_trivia_anchor_end, first_tail_group_member, is_lambda_expression,
    is_nested_lambda_expression, member_is_private_hash, transparent_inner_expression,
};
use crate::annotation::{
    FormatLeadingComments, FormatTrailingComments, format_leading_comments,
    format_trailing_comments, infix_or_postfix_annotations, postfix_annotations,
    prefix_annotations,
};
use crate::call::{expression_is_long_curried_call, format_call_arguments, format_call_expression};
use crate::context::{DestackFormatterSpeculationExt, with_following_span_start};
use crate::expression::{
    format_generic_argument_list, format_generic_argument_list_with_relational_spacing,
    format_if_else_chain, write_index_access,
};
use crate::operator::{is_chain_expression, write_postfix_base_expression};
use crate::{DestackFormatContext, DestackFormatter};
use destack_core::StringId;
use destack_dir::{
    Comment, CommentPosition, Declaration, DecoratorPosition, Expression, FunctionForm, IfForm,
    LocalNodeId, Member, NodeType, PostfixPosition,
};
use destack_fir::format::{Buffer, Format, FormatResult};
use destack_fir::prelude::{
    empty_line, expand_parent, format_with, group, hard_line_break, indent, line_suffix_boundary,
    token,
};
use destack_fir::{best_fitting, format_args, write};
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
    root: ChainRoot,
    head: MemberChainGroup,
    tail_groups: TailChainGroups,
}

impl MemberChain {
    /// Build the normalized chain layout for one expression.
    fn from_expression(
        context: &DestackFormatContext<'_>,
        node_id: LocalNodeId<Expression>,
    ) -> FormatResult<Self> {
        let (chain, root, mut head, mut tail_groups) = build_member_chain_parts(context, node_id)?;
        maybe_merge_with_first_group(context, node_id, &root, &mut head, &mut tail_groups);

        Ok(Self {
            chain,
            root,
            head,
            tail_groups,
        })
    }

    /// Return whether one normalized call chain has multiple tail groups.
    pub(crate) fn is_member_call_chain(
        context: &DestackFormatContext<'_>,
        node_id: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        Self::from_expression(context, node_id)
            .map(|chain| chain.tail_groups.is_member_call_chain())
    }

    /// Return whether the formatted head breaks.
    fn head_will_break<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
        formatted_root_id: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        let start = f.context().expression_token_start(formatted_root_id);
        let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write_chain_head(
                f,
                formatted_root_id,
                &self.root,
                &self.head,
                &self.tail_groups,
                false,
            )
        });
        f.speculate_will_break_after(start, &content)
    }

    /// Inspect every tail group and cache whether it breaks.
    fn inspect_member_chain_groups<'ast>(
        &mut self,
        f: &mut DestackFormatter<'ast, '_>,
        formatted_root_id: LocalNodeId<Expression>,
    ) -> FormatResult<()> {
        let group_count = self.tail_groups.len();

        for group_index in 0..group_count {
            let following_group_first_member = self
                .tail_groups
                .get(group_index + 1)
                .and_then(|group| group.first())
                .cloned();

            let group = self
                .tail_groups
                .get_mut(group_index)
                .expect("tail chain group inspection should have one group");
            let first_member = group
                .first()
                .expect("tail chain group inspection should have one first member");
            let start = f
                .context()
                .expression_token_start(chain_member_node_id(first_member));
            let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write_chain_group(
                    f,
                    formatted_root_id,
                    group.members(),
                    following_group_first_member.as_ref(),
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

        last_group.last().is_some_and(chain_member_is_call_like) && last_group.will_break()
    }

    /// Return whether one call operation has a function-like argument.
    fn call_operation_has_function_like_argument(
        context: &DestackFormatContext<'_>,
        operation: &ChainMember,
    ) -> bool {
        let ChainMember::Call { arguments, .. } = operation else {
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
            .head
            .iter()
            .chain(self.tail_groups.iter().flat_map(|group| group.iter()))
        {
            if !chain_member_is_call_like(operation) {
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
        self.head
            .iter()
            .chain(self.tail_groups.iter().flat_map(|group| group.iter()))
            .any(|operation| {
                let ChainMember::Member { node_id, .. } = operation else {
                    return false;
                };

                member_has_intervening_comment(context, *node_id)
            })
    }
}

/// Try to merge the first tail group into the chain head.
fn maybe_merge_with_first_group(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    root: &ChainRoot,
    head: &mut MemberChainGroup,
    tail_groups: &mut TailChainGroups,
) {
    if !should_merge_tail_with_head(context, node_id, root, head, tail_groups) {
        return;
    }

    let Some(first_group) = tail_groups.pop_first() else {
        return;
    };

    head.extend_members(first_group.into_members());
}

/// Return whether the first tail group should merge into the head.
fn should_merge_tail_with_head(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    root: &ChainRoot,
    head: &MemberChainGroup,
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
        .is_some_and(|operation| matches!(operation, ChainMember::Index { .. }));

    if head.members().is_empty() {
        match root {
            ChainRoot::Expression(expression_id) => {
                let expression_id = transparent_inner_expression(context, *expression_id);
                let is_standalone_statement = expression_is_standalone_statement(context, node_id);

                match context.tree.get(expression_id) {
                    Expression::Identifier { name, .. } => {
                        has_computed_property
                            || is_factory_name(context, *name)
                            || (is_standalone_statement
                                && has_short_name(
                                    context.strings.get(*name),
                                    context.options.indent_width,
                                ))
                    }
                    Expression::QualifiedReference { path, .. } if path.segments.len() == 1 => {
                        has_computed_property || is_factory_name(context, path.segments[0])
                    }
                    Expression::This => true,
                    _ => false,
                }
            }
            ChainRoot::Path { segment, .. } => {
                has_computed_property || is_factory_name(context, *segment)
            }
        }
    } else if let Some(ChainMember::Member { segment, .. }) = head.last() {
        has_computed_property || is_factory_name(context, *segment)
    } else {
        false
    }
}

/// Return whether the first tail group member owns a separator comment.
fn first_group_has_comment(
    context: &DestackFormatContext<'_>,
    first_group: &MemberChainGroup,
) -> bool {
    let Some(first_operation) = first_group.first() else {
        return false;
    };

    match first_operation {
        ChainMember::Member { node_id, .. } => member_has_intervening_comment(context, *node_id),
        _ => false,
    }
}

/// Return whether one expression is a standalone expression statement.
fn expression_is_standalone_statement(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return true;
    };

    // block statements
    if parent_type == NodeType::Block {
        return true;
    }

    // declaration expression lists
    if parent_type == NodeType::Declaration {
        let declaration_id = LocalNodeId::<Declaration>::new(parent_id);

        return match context.tree.get(declaration_id) {
            Declaration::Global(global) => global.expressions.contains(&expression_id),
            Declaration::Namespace(namespace) => namespace.expressions.contains(&expression_id),

            // lambda bodies are not standalone statements
            Declaration::Function(function) => {
                function
                    .body
                    .is_some_and(|body_id| body_id == expression_id)
                    && function.signature.form != FunctionForm::Lambda
            }
            _ => false,
        };
    }

    // method bodies
    if parent_type == NodeType::Member {
        let member_id = LocalNodeId::<Member>::new(parent_id);

        return match context.tree.get(member_id) {
            Member::Method { body, .. } => body.is_some_and(|body_id| body_id == expression_id),
            _ => false,
        };
    }

    false
}

/// Return whether one identifier name follows the factory-style merge rule.
fn is_factory_name(context: &DestackFormatContext<'_>, string_id: StringId) -> bool {
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
    group: &MemberChainGroup,
) -> bool {
    let Some(first_operation) = group.first() else {
        return false;
    };

    let node_id = chain_member_node_id(first_operation);
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
        ChainMember::Member { .. } => b'.',
        ChainMember::Index { .. } => b'[',
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
        let chain_span_end = f.context().span(formatted_root_id).end;
        let head_will_break = chain.head_will_break(f, formatted_root_id)?;
        chain.inspect_member_chain_groups(f, formatted_root_id)?;
        let groups_should_break = chain.groups_should_break(f.context(), head_will_break);
        let has_member_comment = self.has_member_comment(f.context());
        let has_new_line_or_comment_between = chain
            .tail_groups
            .iter()
            .any(|group| group.needs_empty_line());

        let format_one_line_chain = format_with(|f| {
            write_one_line_chain(
                f,
                formatted_root_id,
                &chain.root,
                &chain.head,
                &chain.tail_groups,
            )
        });

        let format_expanded_chain = format_with(|f| {
            write_expanded_chain(
                f,
                formatted_root_id,
                groups_should_break,
                &chain.root,
                &chain.head,
                &chain.tail_groups,
                true,
            )
        });

        if chain.tail_groups.len() <= 1 && !has_member_comment && !has_new_line_or_comment_between {
            let is_long_curried_call =
                first_call_expression_id(f.context(), &chain.root, &chain.head, &chain.tail_groups)
                    .is_some_and(|call_expression_id| {
                        expression_is_long_curried_call(f.context(), call_expression_id)
                    });

            if is_long_curried_call {
                write!(f, [format_one_line_chain])?;
            } else {
                write!(f, [group(&format_one_line_chain)])?;
            }

            f.context_mut()
                .comments_mut()
                .skip_comments_before(chain_span_end);

            return Ok(());
        }

        if has_member_comment || has_new_line_or_comment_between || groups_should_break {
            write!(f, [group(&format_expanded_chain)])?;
            f.context_mut()
                .comments_mut()
                .skip_comments_before(chain_span_end);

            return Ok(());
        }

        let last_group_breaks = chain
            .tail_groups
            .last()
            .is_some_and(|group| group.will_break());

        if last_group_breaks {
            write!(f, [expand_parent()])?;
        }

        write!(
            f,
            [best_fitting![format_one_line_chain, format_expanded_chain]]
        )?;

        f.context_mut()
            .comments_mut()
            .skip_comments_before(chain_span_end);

        Ok(())
    }
}

/// Return the leading call expression in one normalized chain, if any.
fn first_call_expression_id(
    context: &DestackFormatContext<'_>,
    root: &ChainRoot,
    head: &MemberChainGroup,
    tail_groups: &TailChainGroups,
) -> Option<LocalNodeId<Expression>> {
    match root {
        ChainRoot::Expression(expression_id) => {
            let expression_id = transparent_inner_expression(context, *expression_id);

            if matches!(context.tree.get(expression_id), Expression::Call { .. }) {
                return Some(expression_id);
            }
        }
        ChainRoot::Path { .. } => {}
    }

    head.iter()
        .chain(tail_groups.iter().flat_map(|group| group.iter()))
        .find_map(|operation| match operation {
            ChainMember::Call { node_id, .. } => Some(*node_id),
            _ => None,
        })
}

/// Format a member/call/maybe/index chain with the standard breaking layout.
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
    root: &ChainRoot,
    head: &MemberChainGroup,
    tail_groups: &TailChainGroups,
) -> FormatResult<()> {
    write_chain_head(f, formatted_root_id, root, head, tail_groups, false)?;
    skip_comments_after_chain_head(f, root, head)?;

    let groups = tail_groups.iter().collect::<Vec<_>>();

    for (group_index, group) in groups.iter().enumerate() {
        let following_group_first_operation =
            groups.get(group_index + 1).and_then(|group| group.first());

        write_chain_group(
            f,
            formatted_root_id,
            group.members(),
            following_group_first_operation,
        )?;
        skip_comments_after_chain_group(f, group)?;
    }

    Ok(())
}

/// Write the expanded variant of one member chain.
fn write_expanded_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    should_expand_parent: bool,
    root: &ChainRoot,
    head: &MemberChainGroup,
    tail_groups: &TailChainGroups,
    expand_if_value_root: bool,
) -> FormatResult<()> {
    // parent expansion
    if should_expand_parent {
        write!(f, [expand_parent()])?;
    }

    // base
    write_chain_head(
        f,
        formatted_root_id,
        root,
        head,
        tail_groups,
        expand_if_value_root,
    )?;
    skip_comments_after_chain_head(f, root, head)?;

    // tail groups
    if tail_groups.is_empty() {
        return Ok(());
    }

    let skip_first_soft_break_for_conditional_head =
        expression_has_ternary_ancestor(f.context(), formatted_root_id)
            && head.last().is_some_and(chain_member_is_call_like)
            && tail_groups.first().is_some_and(|group| {
                matches!(
                    group.members(),
                    [
                        ChainMember::Member { .. },
                        operation
                    ]
                    if chain_member_is_call_like(operation)
                )
            });
    let format_groups = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        for (group_index, tail_group) in tail_groups.iter().enumerate() {
            let should_skip_first_break =
                group_index == 0 && skip_first_soft_break_for_conditional_head;
            let should_insert_break = !should_skip_first_break
                && (group_index == 0
                    || !tail_group.first().is_some_and(|operation| {
                        let node_id = chain_member_node_id(operation);

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
                if tail_group.needs_empty_line() {
                    write!(f, [empty_line()])?;
                } else {
                    write!(f, [hard_line_break()])?;
                }
            }

            let group_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write_chain_group(
                    f,
                    formatted_root_id,
                    tail_group.members(),
                    tail_groups
                        .iter()
                        .nth(group_index + 1)
                        .and_then(|group| group.first()),
                )
            });
            write!(f, [group(&group_content)])?;
            skip_comments_after_chain_group(f, tail_group)?;
        }

        Ok(())
    });

    write!(f, [indent(&format_groups)])?;

    Ok(())
}

/// Return the first token start that begins one chain operation.
fn chain_operation_start(
    context: &DestackFormatContext<'_>,
    operation: &ChainMember,
) -> Option<u32> {
    match operation {
        ChainMember::Member { node_id, .. } => {
            return super::member::member_property_start(context, *node_id);
        }
        ChainMember::Index {
            index: Some(index), ..
        } => {
            return Some(context.span(*index).start);
        }
        _ => {}
    }

    let node_id = chain_member_node_id(operation);
    let left_id = super::member::chain_node_left_id(context.tree, node_id)?;
    let left_end = expression_trivia_anchor_end(context, left_id);
    let node_span = context.span(node_id);

    context
        .first_non_trivia_token_between(left_end, node_span.end)
        .map(|token| token.span.start)
}

/// Return whether the following call owns the gap comments before its arguments.
fn next_operation_owns_callee_gap_comments(next_operation: Option<&ChainMember>) -> bool {
    matches!(
        next_operation,
        Some(ChainMember::Call {
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
    next_operation: Option<&ChainMember>,
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
        ChainMember::Member { node_id, .. } | ChainMember::Index { node_id, .. } => {
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
    op: &ChainMember,
) -> FormatResult<()> {
    let node_id = match op {
        ChainMember::Member { node_id, .. } | ChainMember::Index { node_id, .. } => *node_id,
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
    next_operation: Option<&ChainMember>,
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
fn write_chain_head<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    root: &ChainRoot,
    head: &MemberChainGroup,
    tail_groups: &TailChainGroups,
    expand_if_value_root: bool,
) -> FormatResult<()> {
    let root_is_decorator_expression = f
        .context()
        .parent(formatted_root_id)
        .is_some_and(|(_, parent_type)| matches!(parent_type, NodeType::Decorator));
    let skip_root_for_start_call = root_is_owned_by_start_call(root, head);

    if !skip_root_for_start_call {
        match root {
            ChainRoot::Path {
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
                    let next_operation = head
                        .first()
                        .or_else(|| first_tail_group_member(tail_groups));
                    if next_operation.is_some_and(chain_member_is_index) {
                        format_generic_argument_list_with_relational_spacing(f, generic_arguments)?;
                    } else {
                        format_generic_argument_list(f, generic_arguments)?;
                    }
                }
                if *emit_postfix_annotations {
                    write!(f, [infix_or_postfix_annotations(f.context(), *node_id)])?;
                }
            }
            ChainRoot::Expression(node_id) => {
                let first_continuation = head
                    .first()
                    .or_else(|| first_tail_group_member(tail_groups));
                let following_span_start = first_continuation
                    .and_then(|operation| chain_operation_start(f.context(), operation))
                    .unwrap_or(0);

                // the root expression still owns its own base formatting
                with_following_span_start(f, following_span_start, |f| {
                    write_chain_root_expression(f, *node_id, expand_if_value_root)?;
                    write!(f, [infix_or_postfix_annotations(f.context(), *node_id)])
                })?;

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

    for (index, op) in head.iter().enumerate() {
        let next_operation = head
            .iter()
            .nth(index + 1)
            .or_else(|| first_tail_group_member(tail_groups));
        write_chain_operation(f, formatted_root_id, op, next_operation)?;
    }

    Ok(())
}

/// Write one chain root expression.
fn write_chain_root_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    expand_if_value_root: bool,
) -> FormatResult<()> {
    // regular if roots expand with the chain
    let should_expand_if_root = expand_if_value_root
        && matches!(
            f.context().tree.get(expression_id),
            Expression::If {
                form: IfForm::If,
                ..
            }
        );
    if should_expand_if_root {
        let expression_span = f.context().span(expression_id);
        let token_start = f.context().expression_token_start(expression_id);
        let token_start_span = Span::new(expression_span.file, token_start, token_start);

        // preserve the same leading trivia boundary as regular expressions
        let expanded_if = format_with(|f| {
            write!(f, [prefix_annotations(f.context(), expression_id)])?;
            write!(f, [format_leading_comments(token_start_span)])?;
            format_if_else_chain(f, expression_id, true)
        });

        write!(
            f,
            [group(&format_args![token("("), &expanded_if, token(")")])]
        )?;
        return Ok(());
    }

    write_postfix_base_expression(f, expression_id)
}

/// Advance the comment cursor past the head owner span.
fn skip_comments_after_chain_head<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    root: &ChainRoot,
    head: &MemberChainGroup,
) -> FormatResult<()> {
    let head_end = if let Some(last_member) = head.last() {
        let node_id = chain_member_node_id(last_member);
        f.context().span(node_id).end
    } else {
        match root {
            ChainRoot::Expression(node_id) => f.context().span(*node_id).end,
            ChainRoot::Path { node_id, .. } => f.context().span(*node_id).end,
        }
    };

    f.context_mut()
        .comments_mut()
        .skip_comments_before(head_end);

    Ok(())
}

/// Advance the comment cursor past one chain group owner span.
fn skip_comments_after_chain_group<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    group: &MemberChainGroup,
) -> FormatResult<()> {
    let Some(last_member) = group.last() else {
        return Ok(());
    };

    let node_id = chain_member_node_id(last_member);
    let group_end = f.context().span(node_id).end;

    f.context_mut()
        .comments_mut()
        .skip_comments_before(group_end);

    Ok(())
}

/// Return whether the first call operation prints the normalized base itself.
fn root_is_owned_by_start_call(root: &ChainRoot, head: &MemberChainGroup) -> bool {
    matches!(
        (root, head.first()),
        (
            ChainRoot::Expression(_),
            Some(ChainMember::Call {
                call_position: CallExpressionPosition::Start,
                ..
            })
        )
    )
}

/// Return prefix and postfix emission flags for one chain operation.
fn chain_operation_annotation_emit_flags(
    context: &DestackFormatContext<'_>,
    formatted_root_id: LocalNodeId<Expression>,
    operation: &ChainMember,
) -> (LocalNodeId<Expression>, bool, bool) {
    // member-chain analysis already computed exact ownership for direct members
    let (node_id, emit_prefix_annotations, emit_postfix_annotations) = match operation {
        ChainMember::Member {
            node_id,
            emit_prefix_annotations,
            emit_postfix_annotations,
            ..
        } => (
            *node_id,
            *emit_prefix_annotations,
            *emit_postfix_annotations,
        ),
        ChainMember::Instantiation { node_id, .. }
        | ChainMember::Call { node_id, .. }
        | ChainMember::Index { node_id, .. }
        | ChainMember::Maybe { node_id, .. }
        | ChainMember::Must { node_id, .. } => {
            // call hops may inherit prefix annotations from their callee path, so the
            // formatter must suppress duplicates when the left side already owns them
            let should_emit_prefix_annotations = match operation {
                ChainMember::Call { node_id, .. } => match context.tree.get(*node_id) {
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
        emit_prefix_annotations && !chain_member_has_leading_gap_comment(context, operation);

    // the formatted root is owned by the outer caller
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
    op: &ChainMember,
    next_operation: Option<&ChainMember>,
) -> FormatResult<()> {
    let (node_id, emit_prefix_annotations, emit_postfix_annotations) =
        chain_operation_annotation_emit_flags(f.context(), formatted_root_id, op);
    let operation_span = f.context().span(node_id);
    let call_or_new_handles_empty_infix = matches!(
        op,
        ChainMember::Call {
            node_id,
            arguments,
            ..
        } if arguments.is_empty() && f.context().has_infix_annotation(*node_id)
    );
    write_chain_operation_prefix(f, node_id, emit_prefix_annotations)?;
    write_chain_operation_leading_comments(f, op)?;

    match op {
        ChainMember::Member {
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
                if next_operation.is_some_and(chain_member_is_index) {
                    format_generic_argument_list_with_relational_spacing(f, generic_arguments)?;
                } else {
                    format_generic_argument_list(f, generic_arguments)?;
                }
            }
        }
        ChainMember::Instantiation {
            generic_arguments, ..
        } => {
            if next_operation.is_some_and(chain_member_is_index) {
                format_generic_argument_list_with_relational_spacing(f, generic_arguments)?;
            } else {
                format_generic_argument_list(f, generic_arguments)?;
            }
        }
        ChainMember::Call {
            node_id: call_node_id,
            call_position,
            optional_position,
            position,
            generic_arguments,
            arguments,
        } => {
            if *call_position == CallExpressionPosition::Start {
                format_call_expression(f, *call_node_id)?;
            } else {
                write_chain_operation_optional_marker(f, *call_node_id, *optional_position)?;
                if *position == PostfixPosition::Indirect {
                    write!(f, [token(".")])?;
                }
                if !generic_arguments.is_empty() {
                    format_generic_argument_list(f, generic_arguments)?;
                }
                format_call_arguments(f, *call_node_id, arguments)?;
            }
        }
        ChainMember::Index {
            node_id,
            optional_position,
            position,
            index,
            ..
        } => {
            write!(f, [line_suffix_boundary()])?;
            write_chain_operation_optional_marker(f, *node_id, *optional_position)?;
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            if let Some(index) = index {
                write_index_access(f, *node_id, *index, false)?;
            } else {
                write!(f, [token("[]")])?;
            }
        }
        ChainMember::Maybe { position, .. } => match position {
            PostfixPosition::Direct => write!(f, [token("?")])?,
            PostfixPosition::Indirect => {
                write!(f, [token("."), token("?")])?;
            }
        },
        ChainMember::Must { position, .. } => match position {
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
    ops: &[ChainMember],
    following_group_first_operation: Option<&ChainMember>,
) -> FormatResult<()> {
    for (index, op) in ops.iter().enumerate() {
        let next_operation = ops.get(index + 1).or(following_group_first_operation);
        write_chain_operation(f, formatted_root_id, op, next_operation)?;
    }
    Ok(())
}
