use crate::format::chain::normalize::chain_layout;
use crate::format::chain::{
    ChainExpression, ChainExpressionBase, ChainExpressionBaseHead,
    chain_line_starts_with_block_prefix_annotation, expression_is_in_conditional_branch,
    format_call_arguments, member_is_private_hash, transparent_inner_expression,
};
use crate::format::expression::{
    AnnotationPosition, Argument, Declaration, DestackFormatContext, DestackFormatter, Expression,
    FormatResult, FunctionKind, LocalNodeId, NodeType, PostfixPosition, SmallVec, expand_parent,
    format_static_argument_list, format_static_argument_list_with_relational_spacing, format_with,
    group, indent, is_call_like_argument, soft_line_break, token, write_postfix_base_expression,
};
use destack_fir::format::Buffer;
use destack_fir::write;

/// Return whether one call operation should emit its prefix annotations.
fn call_operation_should_emit_prefix_annotations(
    context: &DestackFormatContext<'_>,
    call_node_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Call { left, .. } = context.tree.get(call_node_id) else {
        return context.has_prefix_annotation(call_node_id);
    };

    if !context.has_prefix_annotation(call_node_id) {
        return false;
    }

    if !context.has_prefix_annotation(*left) {
        return true;
    }

    let left_span = context.span(*left);
    let has_leading_prefix_before_left = context
        .visit_annotations(call_node_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.annotation(*annotation_id);
                if !matches!(
                    annotation.position(),
                    AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
                ) {
                    return false;
                }

                let annotation_span = context.annotation_span(*annotation_id);
                annotation_span.start < left_span.start
            })
        })
        .unwrap_or(false);

    !has_leading_prefix_before_left
}

/// Return whether one chain operation is an index access.
fn chain_operation_is_index(operation: &ChainExpression) -> bool {
    matches!(operation, ChainExpression::Index { .. })
}

/// Return the first operation of the first grouped line.
fn first_grouped_line_operation(
    lines: &[SmallVec<[ChainExpression; 2]>],
) -> Option<&ChainExpression> {
    lines.first().and_then(|line| line.first())
}

/// Return one expression node id for one chain operation.
fn chain_operation_node_id(operation: &ChainExpression) -> LocalNodeId<Expression> {
    match operation {
        ChainExpression::Member { node_id, .. }
        | ChainExpression::Instantiation { node_id, .. }
        | ChainExpression::Call { node_id, .. }
        | ChainExpression::Index { node_id, .. }
        | ChainExpression::Maybe { node_id, .. }
        | ChainExpression::Must { node_id, .. } => *node_id,
    }
}

/// Return the trailing node of one chain base.
fn chain_base_trailing_node_id(base: &ChainExpressionBase) -> Option<LocalNodeId<Expression>> {
    if let Some(last_operation) = base.body.last() {
        return Some(chain_operation_node_id(last_operation));
    }

    let ChainExpressionBaseHead::Path { node_id, .. } = base.head else {
        return None;
    };
    Some(node_id)
}

/// Return whether one node has only own-line boundary postfix annotations.
fn node_has_only_own_line_boundary_postfix_annotations(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(node_id, |annotations| {
            let mut has_boundary_postfix_annotation = false;

            for annotation_id in annotations {
                let annotation_id = *annotation_id;
                let annotation = context.annotation(annotation_id);
                if annotation.position() != AnnotationPosition::LinePostfixBoundary {
                    continue;
                }

                has_boundary_postfix_annotation = true;
                if !context.span_starts_on_own_line(context.annotation_span(annotation_id)) {
                    return false;
                }
            }

            has_boundary_postfix_annotation
        })
        .unwrap_or(false)
}

/// Return one base node whose boundary postfix annotations should render with the first chain line.
fn deferred_base_boundary_owner_node_id(
    context: &DestackFormatContext<'_>,
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
    first_line_attaches_to_base: bool,
) -> Option<LocalNodeId<Expression>> {
    if first_line_attaches_to_base || lines.is_empty() {
        return None;
    }

    let base_trailing_node_id = chain_base_trailing_node_id(base)?;
    if !node_has_only_own_line_boundary_postfix_annotations(context, base_trailing_node_id) {
        return None;
    }

    Some(base_trailing_node_id)
}

/// Return whether the first grouped chain line can attach directly to the base.
fn first_grouped_line_attaches_to_base(
    _context: &DestackFormatContext<'_>,
    _node_id: LocalNodeId<Expression>,
    _base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
) -> bool {
    if matches!(
        lines.first().and_then(|line| line.first()),
        Some(ChainExpression::Must { .. })
    ) {
        return true;
    }

    if lines.len() != 1 {
        return false;
    }

    matches!(
        lines[0].as_slice(),
        [
            ChainExpression::Maybe { .. },
            ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
        ]
    )
}

/// Return whether formatting should skip the first soft break for conditional chain heads.
fn should_skip_first_soft_break_for_conditional_head(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
) -> bool {
    expression_is_in_conditional_branch(context, node_id)
        && base.body.last().is_some_and(|operation| {
            matches!(
                operation,
                ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
            )
        })
        && lines.first().is_some_and(|line| {
            matches!(
                line.as_slice(),
                [
                    ChainExpression::Member { .. },
                    ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
                ]
            )
        })
}

/// Return whether this chain is the expression body of a lambda call argument.
fn chain_is_lambda_call_argument_body(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let Declaration::Function {
        signature, body, ..
    } = context.tree.get(declaration_id)
    else {
        return false;
    };
    if signature.kind != FunctionKind::Lambda || *body != Some(node_id) {
        return false;
    }

    let Some((declaration_parent_id, declaration_parent_type)) = context.parent(declaration_id)
    else {
        return false;
    };
    if declaration_parent_type != NodeType::Expression {
        return false;
    }

    let lambda_expression_id = LocalNodeId::<Expression>::new(declaration_parent_id);
    is_call_like_argument(context, lambda_expression_id)
}

/// Return whether statement formatting owns postfix emission for one expression.
fn statement_context_owns_expression_postfix(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return true;
    };

    if parent_type == NodeType::Block {
        return true;
    }

    if parent_type == NodeType::Expression {
        let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
        if matches!(
            context.tree.get(parent_expression_id),
            Expression::Statement(inner_id) if inner_id.id == expression_id.id
        ) {
            return true;
        }
    }

    false
}

/// Format static arguments for chain operations that may need relational spacing.
fn format_chain_static_argument_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    static_arguments: &[LocalNodeId<Argument>],
    next_operation: Option<&ChainExpression>,
) -> FormatResult<()> {
    if next_operation.is_some_and(chain_operation_is_index) {
        return format_static_argument_list_with_relational_spacing(f, static_arguments);
    }

    format_static_argument_list(f, static_arguments)
}

/// Format a member/call/maybe/index chain with prettier-style breaking.
pub(crate) fn format_expression_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let (base, lines, chain_should_break, instantiation_prefix_wrap_body_ops) =
        chain_layout(f.context(), node_id)?;

    // indent chain lines consistently, even in assignment rhs positions
    let should_indent_chain = true;
    let should_force_inline_lambda_argument_chain =
        !chain_should_break && chain_is_lambda_call_argument_body(f.context(), node_id);

    // chain content uses soft line breaks so the enclosing group decides fit vs break
    let format_chain = format_with(|f| {
        // encourage the parent to break when the chain is complex
        if chain_should_break {
            write!(f, [expand_parent()])?;
        }

        let first_line_attaches_to_base =
            first_grouped_line_attaches_to_base(f.context(), node_id, &base, &lines);
        let deferred_base_boundary_owner_node_id = deferred_base_boundary_owner_node_id(
            f.context(),
            &base,
            &lines,
            first_line_attaches_to_base,
        );

        // always print the base first so indentation aligns subsequent lines
        format_chain_base(
            f,
            node_id,
            &base,
            &lines,
            instantiation_prefix_wrap_body_ops,
            deferred_base_boundary_owner_node_id,
        )?;

        // indent chained entries so each operation sits on its own line when expanded
        // use soft line breaks so the enclosing group decides inline vs multiline
        if !lines.is_empty() {
            let skip_first_soft_break_for_conditional_head =
                should_skip_first_soft_break_for_conditional_head(
                    f.context(),
                    node_id,
                    &base,
                    &lines,
                );
            let format_lines = format_with(|f| {
                // each chain line renders in isolation to mirror prettier style
                for (line_index, line) in lines.iter().enumerate() {
                    let is_first_attached_line = line_index == 0 && first_line_attaches_to_base;
                    let should_skip_first_soft_break =
                        line_index == 0 && skip_first_soft_break_for_conditional_head;
                    let should_insert_soft_break = !is_first_attached_line
                        && !should_skip_first_soft_break
                        && (line_index == 0
                            || !chain_line_starts_with_block_prefix_annotation(f.context(), line));
                    if should_insert_soft_break {
                        write!(f, [soft_line_break()])?;
                    }

                    if line_index == 0
                        && let Some(owner_node_id) = deferred_base_boundary_owner_node_id
                    {
                        write!(
                            f,
                            [f.context().line_postfix_boundary_annotations(owner_node_id)]
                        )?;
                    }

                    format_chain_expression_line(
                        f,
                        node_id,
                        line,
                        deferred_base_boundary_owner_node_id,
                    )?;
                }
                Ok(())
            });
            let should_wrap_lines_in_indent = should_indent_chain && !first_line_attaches_to_base;
            if should_wrap_lines_in_indent {
                write!(f, [indent(&format_lines)])?;
            } else {
                // avoid extra indentation when the parent already indents after `=`
                write!(f, [format_lines])?;
            }
        }

        Ok(())
    });

    if should_force_inline_lambda_argument_chain {
        let inline_chain = format_with(|f| {
            format_chain_base(
                f,
                node_id,
                &base,
                &lines,
                instantiation_prefix_wrap_body_ops,
                None,
            )?;
            for line in &lines {
                format_chain_expression_line(f, node_id, line, None)?;
            }
            Ok(())
        });

        return write!(f, [inline_chain]);
    }

    if chain_should_break {
        return write!(f, [group(&format_chain).should_expand(true)]);
    }

    write!(f, [group(&format_chain)])
}
/// Format the base segment of a chain.
fn format_chain_base<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
    instantiation_prefix_wrap_body_ops: Option<usize>,
    deferred_base_boundary_owner_node_id: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    format_chain_base_content(
        f,
        formatted_root_id,
        base,
        lines,
        instantiation_prefix_wrap_body_ops,
        deferred_base_boundary_owner_node_id,
    )
}

/// Format the unwrapped base segment of a chain.
fn format_chain_base_content<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    base: &ChainExpressionBase,
    lines: &[SmallVec<[ChainExpression; 2]>],
    instantiation_prefix_wrap_body_ops: Option<usize>,
    deferred_base_boundary_owner_node_id: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let root_is_decorator_expression =
        f.context()
            .parent(formatted_root_id)
            .is_some_and(|(_, parent_type)| {
                matches!(parent_type, NodeType::Decorator | NodeType::Annotation)
            });
    let mut has_open_prefix_wrap = false;
    let mut has_closed_prefix_wrap = false;
    if instantiation_prefix_wrap_body_ops.is_some() {
        write!(f, [token("(")])?;
        has_open_prefix_wrap = true;
    }

    match &base.head {
        ChainExpressionBaseHead::Path {
            node_id,
            segment,
            static_arguments,
            emit_postfix_annotations,
        } => {
            if !root_is_decorator_expression {
                write!(f, [f.context().any_prefix_annotations(*node_id)])?;
            }
            write!(f, [*segment])?;
            if let Some(arguments) = static_arguments {
                let next_operation = base
                    .body
                    .first()
                    .or_else(|| first_grouped_line_operation(lines));
                format_chain_static_argument_list(f, arguments, next_operation)?;
            }
            if *emit_postfix_annotations {
                let should_defer_boundary_annotations =
                    deferred_base_boundary_owner_node_id == Some(*node_id);
                if should_defer_boundary_annotations {
                    write!(
                        f,
                        [f.context()
                            .any_infix_or_postfix_except_line_postfix_boundary_annotations(
                                *node_id
                            )]
                    )?;
                } else {
                    write!(f, [f.context().any_infix_or_postfix_annotations(*node_id)])?;
                }
            }
        }
        ChainExpressionBaseHead::Expression(node_id) => {
            // chain normalization can still surface nested chain nodes as a base
            // write the base expression directly instead of panicking in debug mode
            write_postfix_base_expression(f, *node_id)?;
        }
    }

    if instantiation_prefix_wrap_body_ops == Some(0) {
        write!(f, [token(")")])?;
        has_closed_prefix_wrap = true;
    }

    for (index, op) in base.body.iter().enumerate() {
        let next_operation = base
            .body
            .get(index + 1)
            .or_else(|| first_grouped_line_operation(lines));
        format_chain_expression(
            f,
            formatted_root_id,
            op,
            next_operation,
            deferred_base_boundary_owner_node_id,
        )?;
        if !has_closed_prefix_wrap && instantiation_prefix_wrap_body_ops == Some(index + 1) {
            write!(f, [token(")")])?;
            has_closed_prefix_wrap = true;
        }
    }

    if has_open_prefix_wrap && !has_closed_prefix_wrap {
        write!(f, [token(")")])?;
    }

    Ok(())
}

/// Format one chained operation.
fn format_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    op: &ChainExpression,
    next_operation: Option<&ChainExpression>,
    deferred_base_boundary_owner_node_id: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    // output any line prefix annotations before the operation
    let (node_id, emit_prefix_annotations, emit_postfix_annotations) = match op {
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
            let should_emit_prefix_annotations = match op {
                ChainExpression::Call { node_id, .. } => {
                    call_operation_should_emit_prefix_annotations(f.context(), *node_id)
                }
                _ => true,
            };
            (*node_id, should_emit_prefix_annotations, true)
        }
    };
    let call_or_new_handles_empty_infix = matches!(
        op,
        ChainExpression::Call {
            node_id,
            dynamic_arguments,
            ..
        } if dynamic_arguments.is_empty() && f.context().has_infix_annotation(*node_id)
    );
    let emit_prefix_annotations = emit_prefix_annotations && node_id != formatted_root_id;
    let root_postfix_owned_by_statement_context = node_id == formatted_root_id
        && statement_context_owns_expression_postfix(f.context(), formatted_root_id);
    let emit_postfix_annotations =
        emit_postfix_annotations && !root_postfix_owned_by_statement_context;
    if emit_prefix_annotations {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
    }

    match op {
        ChainExpression::Member {
            node_id,
            segment,
            static_arguments,
            ..
        } => {
            let is_private_hash = member_is_private_hash(f.context(), *node_id);
            write!(f, [token(".")])?;
            if is_private_hash {
                write!(f, [token("#")])?;
            }
            write!(f, [*segment])?;
            if let Some(arguments) = static_arguments {
                format_chain_static_argument_list(f, arguments, next_operation)?;
            }
        }
        ChainExpression::Instantiation {
            static_arguments, ..
        } => {
            format_chain_static_argument_list(f, static_arguments, next_operation)?;
        }
        ChainExpression::Call {
            node_id: call_node_id,
            position,
            static_arguments,
            dynamic_arguments,
        } => {
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            if let Some(arguments) = static_arguments {
                format_static_argument_list(f, arguments)?;
            }
            format_call_arguments(f, *call_node_id, dynamic_arguments)?;
        }
        ChainExpression::Index {
            position, index, ..
        } => {
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            if let Some(index) = index {
                let should_parenthesize = should_parenthesize_index_expression(f.context(), *index);
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

    // output any line postfix annotations after the operation
    if emit_postfix_annotations {
        let should_defer_boundary_annotations =
            deferred_base_boundary_owner_node_id == Some(node_id);
        if call_or_new_handles_empty_infix {
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
        } else if should_defer_boundary_annotations {
            write!(
                f,
                [f.context()
                    .any_infix_or_postfix_except_line_postfix_boundary_annotations(node_id)]
            )?;
        } else {
            write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        }
    }

    Ok(())
}

/// Decide whether an index expression should be wrapped in parentheses.
pub(crate) fn should_parenthesize_index_expression(
    context: &DestackFormatContext<'_>,
    index_id: LocalNodeId<Expression>,
) -> bool {
    if matches!(context.tree.get(index_id), Expression::Parenthesized { .. }) {
        return false;
    }

    let inner_index_id = transparent_inner_expression(context, index_id);
    matches!(context.tree.get(inner_index_id), Expression::Assign { .. })
}

/// Format all operations for one chain line.
fn format_chain_expression_line<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    formatted_root_id: LocalNodeId<Expression>,
    ops: &[ChainExpression],
    deferred_base_boundary_owner_node_id: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    for (index, op) in ops.iter().enumerate() {
        let next_operation = ops.get(index + 1);
        format_chain_expression(
            f,
            formatted_root_id,
            op,
            next_operation,
            deferred_base_boundary_owner_node_id,
        )?;
    }
    Ok(())
}
