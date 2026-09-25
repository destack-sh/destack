use super::groups::{MemberChainGroup, chain_member_has_leading_gap_comment};
use super::member::{
    CallExpressionPosition, access_marker_position, build_member_chain_parts,
    member_has_intervening_comment,
};
use super::{
    ChainMember, TailChainGroups, assignment_like_parent, expression_has_ternary_ancestor,
    expression_trivia_anchor_end, first_tail_group_member, is_lambda_expression,
    is_nested_lambda_expression, transparent_inner_expression,
};
use crate::annotation::{
    FormatLeadingComments, FormatTrailingComments, format_leading_comments,
    format_trailing_comments, infix_or_postfix_annotations, postfix_annotations,
    prefix_annotations,
};
use crate::call::{expression_is_long_curried_call, format_call_arguments, format_call_expression};
use crate::context::{TsppFormatterSpeculationExt, with_following_span_start};
use crate::expression::{
    format_generic_argument_list, format_generic_argument_list_with_relational_spacing,
    format_if_else_chain, write_index_access,
};
use crate::operator::{is_chain_expression, write_postfix_base_expression};
use crate::{TsppFormatContext, TsppFormatter};
use smallvec::SmallVec;
use tspp_core::StringId;
use tspp_dir::{
    Comment, Declaration, DecoratorPosition, Expression, FunctionForm, IfForm, LocalNodeId, Member,
    NodeType, PostfixPosition,
};
use tspp_fir::format::{Format, FormatError, FormatResult};
use tspp_fir::prelude::{
    empty_line, expand_parent, format_with, group, hard_line_break, indent, line_suffix_boundary,
    token,
};
use tspp_fir::{best_fitting, format_args, write};
use tspp_source::Span;

/// Check whether an assignment chain ends in a nested lambda expression.
pub(crate) fn is_assignment_chain_tail_lambda(
    context: &TsppFormatContext<'_>,
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
pub(crate) struct MemberChain {
    /// The flattened expression path from root to formatted node.
    chain: Vec<LocalNodeId<Expression>>,
    /// The expression at the chain root.
    root_id: LocalNodeId<Expression>,
    /// The operations retained beside the root.
    head: MemberChainGroup,
    /// The operation groups rendered after the head.
    tail_groups: TailChainGroups,
}

/// The measured layout for one member-chain group.
#[derive(Debug, Clone, Copy)]
struct ChainGroupLayout {
    /// Whether the group breaks when formatted alone.
    will_break: bool,
    /// Whether source spacing requires an empty line before the group.
    needs_empty_line: bool,
}

/// The measured layouts for one member chain's tail groups.
struct MemberChainLayout {
    /// The layouts in tail-group order.
    tail_groups: SmallVec<[ChainGroupLayout; 4]>,
}

impl MemberChainLayout {
    /// Return one tail-group layout by index.
    fn tail_group(&self, index: usize) -> Option<ChainGroupLayout> {
        self.tail_groups.get(index).copied()
    }

    /// Return whether the last tail group breaks.
    fn last_will_break(&self) -> bool {
        self.tail_groups
            .last()
            .is_some_and(|layout| layout.will_break)
    }

    /// Return whether any tail group except the last breaks.
    fn any_except_last_will_break(&self) -> bool {
        let count = self.tail_groups.len().saturating_sub(1);
        self.tail_groups
            .iter()
            .take(count)
            .any(|layout| layout.will_break)
    }

    /// Return whether source spacing requires any empty line.
    fn needs_empty_line(&self) -> bool {
        self.tail_groups
            .iter()
            .any(|layout| layout.needs_empty_line)
    }
}

/// One expanded member-chain layout.
#[derive(Debug, Clone, Copy)]
struct ExpandedChainLayout {
    /// The parent expansion behavior.
    parent: ChainParentLayout,
    /// The tail-group formatting behavior.
    tail_groups: ChainTailGroupLayout,
}

impl ExpandedChainLayout {
    /// Return the standard expanded chain layout.
    const fn standard(parent: ChainParentLayout) -> Self {
        Self {
            parent,
            tail_groups: ChainTailGroupLayout::Grouped,
        }
    }

    /// Return the direct expanded layout.
    const fn direct() -> Self {
        Self {
            parent: ChainParentLayout::Preserve,
            tail_groups: ChainTailGroupLayout::Direct,
        }
    }
}

/// The parent layout effect for an expanded chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainParentLayout {
    /// Preserve the parent group mode.
    Preserve,
    /// Expand the parent group.
    Expand,
}

/// The tail-group layout for an expanded chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChainTailGroupLayout {
    /// Keep each tail group in a layout group.
    Grouped,
    /// Write each tail group directly.
    Direct,
}

impl MemberChain {
    /// Build the normalized chain layout for one expression.
    fn from_expression(
        context: &TsppFormatContext<'_>,
        node_id: LocalNodeId<Expression>,
    ) -> FormatResult<Self> {
        let (chain, root_id, mut head, mut tail_groups) =
            build_member_chain_parts(context, node_id)?;
        maybe_merge_with_first_group(context, node_id, root_id, &mut head, &mut tail_groups)?;

        Ok(Self {
            chain,
            root_id,
            head,
            tail_groups,
        })
    }

    /// Return whether one normalized call chain has multiple tail groups.
    pub(crate) fn is_member_call_chain(
        context: &TsppFormatContext<'_>,
        node_id: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        Self::from_expression(context, node_id)
            .map(|chain| chain.tail_groups.is_member_call_chain())
    }

    /// Return whether this chain's minimum width exceeds line width.
    fn minimum_width_exceeds_line_width(&self, context: &TsppFormatContext<'_>) -> bool {
        let operation_count = self.chain.len().saturating_sub(1);
        let minimum_width = 1 + operation_count.saturating_mul(2);

        minimum_width > context.options.line_width as usize
    }

    /// Return whether the formatted head breaks.
    fn head_will_break<'ast>(
        &self,
        f: &mut TsppFormatter<'ast, '_>,
        formatted_root_id: LocalNodeId<Expression>,
    ) -> FormatResult<bool> {
        let start = f.context().expression_token_start(formatted_root_id);
        let content = format_with(|f: &mut TsppFormatter<'ast, '_>| {
            write_chain_head(
                f,
                formatted_root_id,
                self.root_id,
                &self.head,
                &self.tail_groups,
                false,
            )
        });
        f.speculate_will_break_after(start, &content)
    }

    /// Measure every tail group's isolated layout.
    fn inspect_layout<'ast>(
        &self,
        f: &mut TsppFormatter<'ast, '_>,
        formatted_root_id: LocalNodeId<Expression>,
    ) -> FormatResult<MemberChainLayout> {
        let mut group_layouts = SmallVec::with_capacity(self.tail_groups.len());
        let mut groups = self.tail_groups.iter().peekable();

        while let Some(group) = groups.next() {
            let Some(first_member) = group.first() else {
                return Err(FormatError::SyntaxError {
                    message: "empty member chain group",
                });
            };
            let following_group_first_member = groups.peek().and_then(|group| group.first());
            let start = f.context().expression_token_start(first_member.node_id());
            let content = format_with(|f: &mut TsppFormatter<'ast, '_>| {
                write_chain_group(
                    f,
                    formatted_root_id,
                    group.members(),
                    following_group_first_member,
                )
            });
            let will_break = f.speculate_will_break_after(start, &content)?;
            let needs_empty_line = chain_group_needs_empty_line_before(f.context(), group);

            group_layouts.push(ChainGroupLayout {
                will_break,
                needs_empty_line,
            });
        }

        Ok(MemberChainLayout {
            tail_groups: group_layouts,
        })
    }

    /// Return whether the last tail group is a call-like group that breaks.
    fn last_call_breaks(&self, layout: &MemberChainLayout) -> bool {
        let Some(last_group) = self.tail_groups.last() else {
            return false;
        };

        last_group.last().is_some_and(ChainMember::is_call_like) && layout.last_will_break()
    }

    /// Return whether one call operation has a function-like argument.
    fn call_operation_has_function_like_argument(
        context: &TsppFormatContext<'_>,
        operation: &ChainMember,
    ) -> FormatResult<bool> {
        let ChainMember::Call { .. } = operation else {
            return Ok(false);
        };
        let expression = operation.expression(context.tree)?;
        let Expression::Call { arguments, .. } = expression else {
            return Err(FormatError::SyntaxError {
                message: "call chain member does not contain a call expression",
            });
        };
        let has_function_like_argument = arguments.iter().copied().any(|argument_id| {
            let Some(argument_value_id) =
                super::argument_value_id_if_present(context.tree, argument_id)
            else {
                return false;
            };

            let argument_value_id = transparent_inner_expression(context, argument_value_id);
            is_lambda_expression(context, argument_value_id)
                || is_nested_lambda_expression(context, argument_value_id)
        });

        Ok(has_function_like_argument)
    }

    /// Return whether the inspected chain groups should force expanded layout.
    fn groups_should_break(
        &self,
        layout: &MemberChainLayout,
        context: &TsppFormatContext<'_>,
        head_will_break: bool,
    ) -> FormatResult<bool> {
        let mut has_function_like_argument = false;

        for operation in self
            .head
            .iter()
            .chain(self.tail_groups.iter().flat_map(|group| group.iter()))
        {
            if !operation.is_call_like() {
                continue;
            }

            has_function_like_argument |=
                Self::call_operation_has_function_like_argument(context, operation)?;
        }

        if !self.tail_groups.is_empty() && head_will_break {
            return Ok(true);
        }

        if self.last_call_breaks(layout) && has_function_like_argument {
            return Ok(true);
        }

        Ok(layout.any_except_last_will_break())
    }

    /// Return whether any member operation owns a comment before its property.
    fn has_member_comment(&self, context: &TsppFormatContext<'_>) -> bool {
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

/// Return whether one chain carries comments or blank lines that affect chain layout.
fn chain_has_layout_trivia(
    context: &TsppFormatContext<'_>,
    formatted_root_id: LocalNodeId<Expression>,
) -> bool {
    let span = context.span(formatted_root_id);

    context.comments().has_comment_in_span(span) || context.has_blank_line(span)
}

/// Try to merge the first tail group into the chain head.
fn maybe_merge_with_first_group(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    root_id: LocalNodeId<Expression>,
    head: &mut MemberChainGroup,
    tail_groups: &mut TailChainGroups,
) -> FormatResult<()> {
    if !should_merge_tail_with_head(context, node_id, root_id, head, tail_groups)? {
        return Ok(());
    }

    let Some(first_group) = tail_groups.pop_first() else {
        return Ok(());
    };

    head.extend_members(first_group.into_members());

    Ok(())
}

/// Return whether the first tail group should merge into the head.
fn should_merge_tail_with_head(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    root_id: LocalNodeId<Expression>,
    head: &MemberChainGroup,
    tail_groups: &TailChainGroups,
) -> FormatResult<bool> {
    let Some(first_group) = tail_groups.first() else {
        return Ok(false);
    };

    if first_group_has_comment(context, first_group) {
        return Ok(false);
    }

    let has_computed_property = first_group
        .first()
        .is_some_and(|operation| matches!(operation, ChainMember::Index { .. }));

    if head.members().is_empty() {
        let root_id = transparent_inner_expression(context, root_id);
        let is_standalone_statement = expression_is_standalone_statement(context, node_id);

        let should_merge = match context.tree.get(root_id) {
            Expression::Identifier { name, .. } => {
                has_computed_property
                    || is_factory_name(context, *name)
                    || (is_standalone_statement
                        && has_short_name(context.strings.get(*name), context.options.indent_width))
            }
            Expression::This => true,
            _ => false,
        };

        Ok(should_merge)
    } else if let Some(member) = head.last()
        && matches!(member, ChainMember::Member { .. })
    {
        let expression = member.expression(context.tree)?;
        let Expression::Member {
            name: Some(segment),
            ..
        } = expression
        else {
            return Err(FormatError::SyntaxError {
                message: "member chain operation does not contain a named member expression",
            });
        };
        Ok(has_computed_property || is_factory_name(context, *segment))
    } else {
        Ok(false)
    }
}

/// Return whether the first tail group member owns a separator comment.
fn first_group_has_comment(
    context: &TsppFormatContext<'_>,
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
    context: &TsppFormatContext<'_>,
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
            Declaration::Module(module) => module.expressions.contains(&expression_id),

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
fn is_factory_name(context: &TsppFormatContext<'_>, string_id: StringId) -> bool {
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
    context: &TsppFormatContext<'_>,
    group: &MemberChainGroup,
) -> bool {
    let Some(first_operation) = group.first() else {
        return false;
    };

    let node_id = first_operation.node_id();
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

impl<'ast> Format<'ast, TsppFormatContext<'ast>> for MemberChain {
    fn format(&self, f: &mut TsppFormatter<'ast, '_>) -> FormatResult<()> {
        let formatted_root_id = self.chain[self.chain.len() - 1];
        let chain_span_end = f.context().span(formatted_root_id).end;
        let minimum_width_exceeds_line_width = self.minimum_width_exceeds_line_width(f.context());
        let has_layout_trivia = chain_has_layout_trivia(f.context(), formatted_root_id);

        // write guaranteed expanded chains directly
        if minimum_width_exceeds_line_width && !has_layout_trivia {
            let format_expanded_chain = format_with(|f| {
                write_expanded_chain(
                    f,
                    formatted_root_id,
                    ExpandedChainLayout::direct(),
                    None,
                    self.root_id,
                    &self.head,
                    &self.tail_groups,
                )
            });

            write!(f, [format_expanded_chain])?;
            f.context_mut()
                .comments_mut()
                .skip_comments_before(chain_span_end);

            return Ok(());
        }

        let head_will_break = self.head_will_break(f, formatted_root_id)?;
        let chain_layout = self.inspect_layout(f, formatted_root_id)?;
        let groups_should_break =
            self.groups_should_break(&chain_layout, f.context(), head_will_break)?;
        let parent_layout = if groups_should_break {
            ChainParentLayout::Expand
        } else {
            ChainParentLayout::Preserve
        };
        let has_member_comment = self.has_member_comment(f.context());
        let has_new_line_or_comment_between = chain_layout.needs_empty_line();

        let format_one_line_chain = format_with(|f| {
            write_one_line_chain(
                f,
                formatted_root_id,
                self.root_id,
                &self.head,
                &self.tail_groups,
            )
        });

        let format_expanded_chain = format_with(|f| {
            write_expanded_chain(
                f,
                formatted_root_id,
                ExpandedChainLayout::standard(parent_layout),
                Some(&chain_layout),
                self.root_id,
                &self.head,
                &self.tail_groups,
            )
        });

        if self.tail_groups.len() <= 1
            && !has_member_comment
            && !has_new_line_or_comment_between
            && !minimum_width_exceeds_line_width
        {
            let is_long_curried_call =
                first_call_expression_id(f.context(), self.root_id, &self.head, &self.tail_groups)
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

        if has_member_comment
            || has_new_line_or_comment_between
            || groups_should_break
            || minimum_width_exceeds_line_width
        {
            write!(f, [group(&format_expanded_chain)])?;
            f.context_mut()
                .comments_mut()
                .skip_comments_before(chain_span_end);

            return Ok(());
        }

        let last_group_breaks = chain_layout.last_will_break();

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
    context: &TsppFormatContext<'_>,
    root_id: LocalNodeId<Expression>,
    head: &MemberChainGroup,
    tail_groups: &TailChainGroups,
) -> Option<LocalNodeId<Expression>> {
    let root_id = transparent_inner_expression(context, root_id);
    if matches!(context.tree.get(root_id), Expression::Call { .. }) {
        return Some(root_id);
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
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let chain = MemberChain::from_expression(f.context(), node_id)?;
    write!(f, [chain])
}

/// Write the one-line variant of one member chain.
fn write_one_line_chain<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    root_id: LocalNodeId<Expression>,
    head: &MemberChainGroup,
    tail_groups: &TailChainGroups,
) -> FormatResult<()> {
    write_chain_head(f, formatted_root_id, root_id, head, tail_groups, false)?;
    skip_comments_after_chain_head(f, root_id, head)?;

    let mut groups = tail_groups.iter().peekable();

    while let Some(group) = groups.next() {
        let following_group_first_operation = groups.peek().and_then(|group| group.first());

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
    f: &mut TsppFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    layout: ExpandedChainLayout,
    chain_layout: Option<&MemberChainLayout>,
    root_id: LocalNodeId<Expression>,
    head: &MemberChainGroup,
    tail_groups: &TailChainGroups,
) -> FormatResult<()> {
    // parent expansion
    if layout.parent == ChainParentLayout::Expand {
        write!(f, [expand_parent()])?;
    }

    // base
    write_chain_head(f, formatted_root_id, root_id, head, tail_groups, true)?;
    skip_comments_after_chain_head(f, root_id, head)?;

    // tail groups
    if tail_groups.is_empty() {
        return Ok(());
    }

    let skip_first_soft_break_for_conditional_head =
        expression_has_ternary_ancestor(f.context(), formatted_root_id)
            && head.last().is_some_and(ChainMember::is_call_like)
            && tail_groups.first().is_some_and(|group| {
                matches!(
                    group.members(),
                    [
                        ChainMember::Member { .. },
                        operation
                    ]
                    if operation.is_call_like()
                )
            });
    let format_groups = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        let mut tail_groups = tail_groups.iter().enumerate().peekable();

        while let Some((group_index, tail_group)) = tail_groups.next() {
            let should_skip_first_break =
                group_index == 0 && skip_first_soft_break_for_conditional_head;
            let should_insert_break = !should_skip_first_break
                && (group_index == 0
                    || !tail_group.first().is_some_and(|operation| {
                        let node_id = operation.node_id();

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
                let needs_empty_line = chain_layout
                    .and_then(|layout| layout.tail_group(group_index))
                    .is_some_and(|layout| layout.needs_empty_line);

                if needs_empty_line {
                    write!(f, [empty_line()])?;
                } else {
                    write!(f, [hard_line_break()])?;
                }
            }

            let following_group_first_member =
                tail_groups.peek().and_then(|(_, group)| group.first());
            let group_content = format_with(|f: &mut TsppFormatter<'ast, '_>| {
                write_chain_group(
                    f,
                    formatted_root_id,
                    tail_group.members(),
                    following_group_first_member,
                )
            });

            match layout.tail_groups {
                ChainTailGroupLayout::Grouped => write!(f, [group(&group_content)])?,
                ChainTailGroupLayout::Direct => write!(f, [group_content])?,
            }

            skip_comments_after_chain_group(f, tail_group)?;
        }

        Ok(())
    });

    write!(f, [indent(&format_groups)])?;

    Ok(())
}

/// Return the first token start that begins one chain operation.
fn chain_operation_start(context: &TsppFormatContext<'_>, operation: &ChainMember) -> Option<u32> {
    match operation {
        ChainMember::Member { node_id, .. } => {
            return super::member::member_property_start(context, *node_id);
        }
        ChainMember::Index { node_id } => {
            if let Expression::Index {
                index: Some(index), ..
            } = context.tree.get(*node_id)
            {
                return Some(context.span(*index).start);
            }
        }
        _ => {}
    }

    let node_id = operation.node_id();
    let left_id = super::member::chain_node_left_id(context.tree, node_id)?;
    let left_end = expression_trivia_anchor_end(context, left_id);
    let node_span = context.span(node_id);

    context
        .first_token_between(left_end, node_span.end)
        .map(|token| token.span.start)
}

/// Return whether the following call owns the gap comments before its arguments.
fn next_operation_owns_callee_gap_comments(
    context: &TsppFormatContext<'_>,
    next_operation: Option<&ChainMember>,
) -> bool {
    let Some(ChainMember::Call { node_id, .. }) = next_operation else {
        return false;
    };
    let Expression::Call {
        left,
        position,
        generic_arguments,
        arguments,
        is_optional,
    } = context.tree.get(*node_id)
    else {
        return false;
    };

    access_marker_position(context.tree, *left, *is_optional).is_none()
        && *position == PostfixPosition::Direct
        && generic_arguments.is_empty()
        && !arguments.is_empty()
}

/// Return structural trailing comments emitted after one chain segment.
fn chain_structural_trailing_comments(
    context: &TsppFormatContext<'_>,
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
                .filter(|comment| comment.is_trailing())
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

            context.comments_before_next_token_after_span(context.span(receiver_id))
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
    f: &mut TsppFormatter<'ast, '_>,
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
        .comments_before_next_token_after_span(f.context().span(receiver_id))
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
    f: &mut TsppFormatter<'ast, '_>,
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

    if next_operation_owns_callee_gap_comments(f.context(), Some(next_operation)) {
        return Ok(());
    }

    let structural_comments =
        chain_structural_trailing_comments(f.context(), preceding_node_id, Some(next_operation));
    if !structural_comments.is_empty() {
        return write!(f, [FormatTrailingComments::Comments(&structural_comments)]);
    }

    let following_span_start = chain_operation_start(f.context(), next_operation);

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
    f: &mut TsppFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    root_id: LocalNodeId<Expression>,
    head: &MemberChainGroup,
    tail_groups: &TailChainGroups,
    expand_if_value_root: bool,
) -> FormatResult<()> {
    let skip_root_for_start_call = root_is_owned_by_start_call(head);

    if !skip_root_for_start_call {
        let first_continuation = head
            .first()
            .or_else(|| first_tail_group_member(tail_groups));
        let following_span_start =
            first_continuation.and_then(|operation| chain_operation_start(f.context(), operation));

        // format the chain base with its owned annotations and comments
        with_following_span_start(f, following_span_start, |f| {
            write_chain_root_expression(f, root_id, expand_if_value_root)?;
            write!(f, [infix_or_postfix_annotations(f.context(), root_id)])
        })?;

        write_chain_trailing_comments(
            f,
            formatted_root_id,
            root_id,
            f.context().span(root_id),
            first_continuation,
        )?;
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
    f: &mut TsppFormatter<'ast, '_>,
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
    f: &mut TsppFormatter<'ast, '_>,
    root_id: LocalNodeId<Expression>,
    head: &MemberChainGroup,
) -> FormatResult<()> {
    let head_end = if let Some(last_member) = head.last() {
        let node_id = last_member.node_id();
        f.context().span(node_id).end
    } else {
        f.context().span(root_id).end
    };

    f.context_mut()
        .comments_mut()
        .skip_comments_before(head_end);

    Ok(())
}

/// Advance the comment cursor past one chain group owner span.
fn skip_comments_after_chain_group<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    group: &MemberChainGroup,
) -> FormatResult<()> {
    let Some(last_member) = group.last() else {
        return Ok(());
    };

    let node_id = last_member.node_id();
    let group_end = f.context().span(node_id).end;

    f.context_mut()
        .comments_mut()
        .skip_comments_before(group_end);

    Ok(())
}

/// Return whether the first call operation prints the normalized base itself.
fn root_is_owned_by_start_call(head: &MemberChainGroup) -> bool {
    matches!(
        head.first(),
        Some(ChainMember::Call {
            call_position: CallExpressionPosition::Start,
            ..
        })
    )
}

/// Return prefix and postfix emission flags for one chain operation.
fn chain_operation_annotation_emit_flags(
    context: &TsppFormatContext<'_>,
    formatted_root_id: LocalNodeId<Expression>,
    operation: &ChainMember,
) -> (LocalNodeId<Expression>, bool, bool) {
    let node_id = operation.node_id();

    // direct members only own postfix annotations
    let (emit_prefix_annotations, emit_postfix_annotations) = match operation {
        ChainMember::Member { .. } => (false, true),
        _ => {
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
            (should_emit_prefix_annotations, true)
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
    f: &mut TsppFormatter<'ast, '_>,
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
    f: &mut TsppFormatter<'ast, '_>,
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
    f: &mut TsppFormatter<'ast, '_>,
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
    f: &mut TsppFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    op: &ChainMember,
    next_operation: Option<&ChainMember>,
) -> FormatResult<()> {
    let (node_id, emit_prefix_annotations, emit_postfix_annotations) =
        chain_operation_annotation_emit_flags(f.context(), formatted_root_id, op);
    let operation_span = f.context().span(node_id);
    let expression = f.context().tree.get(node_id);
    let call_or_new_handles_empty_infix = matches!(
        (op, expression),
        (ChainMember::Call { .. }, Expression::Call { arguments, .. })
            if arguments.is_empty() && f.context().has_infix_annotation(node_id)
    );
    write_chain_operation_prefix(f, node_id, emit_prefix_annotations)?;
    write_chain_operation_leading_comments(f, op)?;

    match (op, expression) {
        (
            ChainMember::Member { .. },
            Expression::Member {
                left,
                name: Some(segment),
                is_optional,
            },
        ) => {
            let optional_position = access_marker_position(f.context().tree, *left, *is_optional);
            write_chain_operation_optional_marker(f, node_id, optional_position)?;

            write!(f, [token(".")])?;
            write!(f, [*segment])?;
        }
        (
            ChainMember::Instantiation { .. },
            Expression::Instantiation {
                generic_arguments, ..
            },
        ) => {
            if next_operation.is_some_and(ChainMember::is_index) {
                format_generic_argument_list_with_relational_spacing(f, generic_arguments)?;
            } else {
                format_generic_argument_list(f, generic_arguments)?;
            }
        }
        (
            ChainMember::Call { call_position, .. },
            Expression::Call {
                left,
                position,
                generic_arguments,
                arguments,
                is_optional,
            },
        ) => {
            if *call_position == CallExpressionPosition::Start {
                format_call_expression(f, node_id)?;
            } else {
                let optional_position =
                    access_marker_position(f.context().tree, *left, *is_optional);
                write_chain_operation_optional_marker(f, node_id, optional_position)?;
                if *position == PostfixPosition::Indirect {
                    write!(f, [token(".")])?;
                }
                if !generic_arguments.is_empty() {
                    format_generic_argument_list(f, generic_arguments)?;
                }
                format_call_arguments(f, node_id, arguments)?;
            }
        }
        (
            ChainMember::Index { .. },
            Expression::Index {
                left,
                position,
                index,
                is_optional,
            },
        ) => {
            write!(f, [line_suffix_boundary()])?;
            let optional_position = access_marker_position(f.context().tree, *left, *is_optional);
            write_chain_operation_optional_marker(f, node_id, optional_position)?;
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            if let Some(index) = index {
                write_index_access(f, node_id, *index, false)?;
            } else {
                write!(f, [token("[]")])?;
            }
        }
        (ChainMember::Maybe { .. }, Expression::Maybe { position, .. }) => match position {
            PostfixPosition::Direct => write!(f, [token("?")])?,
            PostfixPosition::Indirect => {
                write!(f, [token("."), token("?")])?;
            }
        },
        (ChainMember::Must { .. }, Expression::Must { position, .. }) => match position {
            PostfixPosition::Direct => write!(f, [token("!")])?,
            PostfixPosition::Indirect => {
                write!(f, [token("."), token("!")])?;
            }
        },
        _ => {
            return Err(FormatError::SyntaxError {
                message: "chain member does not match its source expression",
            });
        }
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
    f: &mut TsppFormatter<'ast, '_>,
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
