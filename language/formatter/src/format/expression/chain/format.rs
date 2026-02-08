use super::*;
use destack_fir::write;

/// Format a member/call/maybe/index chain with prettier-style breaking.
pub(crate) fn format_expression_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let tree = f.context().tree;

    // collect the nodes that belong to this chain
    let mut chain = Vec::new();
    let mut current = node_id;
    loop {
        chain.push(current);
        let next = match tree.get(current) {
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Instantiation { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => Some(*left),
            _ => None,
        };

        if let Some(next_id) = next {
            current = next_id;
        } else {
            break;
        }
    }
    chain.reverse();
    let root_id = *chain
        .first()
        .expect("member/call/maybe/index chain must contain at least one node");
    let mut chain_should_break = should_break_chain(f.context(), &chain);
    let has_chain_intervening_trivia = chain_has_intervening_break_or_comment(f.context(), &chain);
    let chain_call_summaries = summarize_chain_calls(f.context(), &chain);
    let has_multiline_nonhead_call = chain_call_summaries
        .iter()
        .skip(1)
        .any(|summary| summary.has_multiline_argument);
    let has_nonhead_nonlambda_function_call_argument =
        chain_has_nonhead_nonlambda_function_call_argument(f.context(), &chain);
    let has_path_tail_deferred_empty_call_boundary_comment =
        chain_has_deferred_empty_call_boundary_comment_on_path_tail(f.context(), &chain);

    // gather operations while breaking path roots into individual segments
    let mut body: Vec<ChainExpression> = Vec::new();
    let mut base_head = ChainExpressionBaseHead::Expression(root_id);
    let mut deferred_path_boundary_comments: Vec<String> = Vec::new();

    // break leading path expression into individual segments
    if let Expression::Path {
        path,
        static_arguments,
    } = tree.get(root_id)
        && should_split_chain_root_path_segments(f.context(), root_id)
    {
        let mut segments = path.segments.clone().into_iter();
        let static_arguments = static_arguments.clone();
        let first_segment = segments.next().expect("path is not empty");
        let remaining_segments: Vec<StringId> = segments.collect();
        deferred_path_boundary_comments =
            path_deferred_boundary_line_comments(f.context(), root_id, path.segments.len());

        let emit_postfix_on_tail = !remaining_segments.is_empty()
            && path_postfix_annotations_emit_on_tail(f.context(), root_id, path.segments.len());

        // path base (keeps static arguments if there are no remaining segments)
        let base_static_arguments = if remaining_segments.is_empty() {
            static_arguments.clone()
        } else {
            None
        };
        base_head = ChainExpressionBaseHead::Path {
            node_id: root_id,
            segment: first_segment,
            static_arguments: base_static_arguments,
            emit_postfix_annotations: (remaining_segments.is_empty() || !emit_postfix_on_tail)
                && deferred_path_boundary_comments.is_empty(),
        };

        // path rest (tail segments)
        // (path segments are synthetic, they come from a single Path expression,
        //  so we use root_id for annotations, even though they won't have intra-path comments)
        let tail_len = remaining_segments.len();
        for (index, segment) in remaining_segments.into_iter().enumerate() {
            let is_last = tail_len != 0 && index + 1 == tail_len;
            // last segment keeps static arguments if there are any
            let static_args = if is_last {
                static_arguments.clone()
            } else {
                None
            };
            body.push(ChainExpression::Member {
                node_id: root_id,
                segment,
                static_arguments: static_args,
                emit_prefix_annotations: false,
                emit_postfix_annotations: emit_postfix_on_tail && is_last,
            });
        }
    }
    let mut base = ChainExpressionBase {
        head: base_head,
        body: Vec::new(),
    };

    // convert the chain into individual chain expressions, preserving node IDs for annotations
    for &expression_id in &chain[1..] {
        let chain_expression = match tree.get(expression_id) {
            Expression::Member {
                name,
                static_arguments,
                ..
            } => ChainExpression::Member {
                node_id: expression_id,
                segment: *name,
                static_arguments: static_arguments.clone(),
                emit_prefix_annotations: true,
                emit_postfix_annotations: true,
            },
            Expression::PrivateMember {
                name,
                static_arguments,
                ..
            } => ChainExpression::Member {
                node_id: expression_id,
                segment: *name,
                static_arguments: static_arguments.clone(),
                emit_prefix_annotations: true,
                emit_postfix_annotations: true,
            },
            Expression::Call {
                position,
                static_arguments,
                dynamic_arguments,
                ..
            } => ChainExpression::Call {
                node_id: expression_id,
                position: *position,
                static_arguments: static_arguments.clone(),
                dynamic_arguments: dynamic_arguments.clone(),
            },
            Expression::Instantiation {
                static_arguments, ..
            } => ChainExpression::Instantiation {
                node_id: expression_id,
                static_arguments: static_arguments.clone(),
            },
            Expression::Index {
                position, index, ..
            } => ChainExpression::Index {
                node_id: expression_id,
                position: *position,
                index: *index,
            },
            Expression::Maybe { position, .. } => ChainExpression::Maybe {
                node_id: expression_id,
                position: *position,
            },
            Expression::Must { position, .. } => ChainExpression::Must {
                node_id: expression_id,
                position: *position,
            },
            _ => {
                return Err(FormatError::SyntaxError {
                    message: "unexpected expression kind for chain expression",
                });
            }
        };
        body.push(chain_expression);
    }

    // member operations with non-inline annotations should keep one operation per line
    let member_has_non_inline_annotation = body.iter().any(|operation| {
        let ChainExpression::Member { node_id, .. } = operation else {
            return false;
        };
        chain_node_has_breaking_annotation(f.context(), *node_id)
    });
    if member_has_non_inline_annotation {
        chain_should_break = true;
    }
    let root_has_line_postfix_boundary_comment =
        expression_has_line_postfix_boundary_comment(f.context(), root_id);
    let first_member_has_line_postfix_boundary_comment = body.first().is_some_and(|operation| {
        let ChainExpression::Member { node_id, .. } = operation else {
            return false;
        };
        expression_has_line_postfix_boundary_comment(f.context(), *node_id)
    });
    let starts_with_member_operation = matches!(body.first(), Some(ChainExpression::Member { .. }));
    let should_avoid_head_promotion_for_boundary_comment =
        first_member_has_line_postfix_boundary_comment
            || (root_has_line_postfix_boundary_comment && starts_with_member_operation);
    if should_avoid_head_promotion_for_boundary_comment {
        chain_should_break = true;
    }

    // keep a leading call with the base so alignment stays stable
    if let Some(first_op) = body.first()
        && matches!(
            first_op,
            ChainExpression::Call {
                position: PostfixPosition::Direct,
                ..
            } | ChainExpression::Instantiation { .. }
        )
    {
        base.body.push(first_op.clone());
        body.remove(0);
    }

    // keep a small head group with the base for prettier style chains
    let base_len = chain_base_len(f.context(), &base);
    let base_has_leading_call_like = match &base.head {
        ChainExpressionBaseHead::Expression(expression_id) => matches!(
            f.context().tree.get(*expression_id),
            Expression::Call { .. } | Expression::Instantiation { .. }
        ),
        ChainExpressionBaseHead::Path { .. } => base.body.first().is_some_and(|operation| {
            matches!(
                operation,
                ChainExpression::Call { .. } | ChainExpression::Instantiation { .. }
            )
        }),
    };
    let remaining_width = if is_call_like_argument(f.context(), node_id) {
        None
    } else {
        assignment_like_remaining_width(f.context(), node_id)
    };
    let root_has_annotation = f.context().has_annotation(root_id);
    let should_avoid_head_promotion_for_nonhead_callbacks =
        has_nonhead_nonlambda_function_call_argument
            || (has_multiline_nonhead_call && has_chain_intervening_trivia)
            || has_path_tail_deferred_empty_call_boundary_comment
            || root_has_annotation;
    let allow_wide_head = is_call_like_argument(f.context(), node_id);
    let head_ops_count = if should_avoid_head_promotion_for_nonhead_callbacks
        || should_avoid_head_promotion_for_boundary_comment
    {
        0
    } else {
        split_chain_head_operations(
            f.context(),
            base_len,
            base_has_leading_call_like,
            &body,
            remaining_width,
            allow_wide_head,
        )
    };
    if head_ops_count > 0 {
        let head_ops: Vec<_> = body.drain(..head_ops_count).collect();
        base.body.extend(head_ops);
    }

    // group the chain operations into lines
    let mut lines = group_chain_expression_lines(f.context(), body);

    // keep curried call tails attached to an already promoted direct call head:
    // `foo(...)(...)` should not split between the closing and opening parens
    if base.body.last().is_some_and(|operation| {
        matches!(
            operation,
            ChainExpression::Call {
                position: PostfixPosition::Direct,
                ..
            }
        )
    }) && let Some(first_line) = lines.first()
        && first_line.len() == 1
        && let ChainExpression::Call {
            node_id,
            position: PostfixPosition::Direct,
            ..
        } = first_line[0]
        && !chain_node_has_non_inline_annotation(f.context(), node_id)
    {
        let first_line = lines.remove(0);
        base.body.push(first_line[0].clone());
    }

    // keep short member + call heads compact inside argument positions:
    // `foo.bar.get(...)` should not split before `.get(` by default
    if is_call_like_argument(f.context(), node_id)
        && base
            .body
            .last()
            .is_some_and(|operation| matches!(operation, ChainExpression::Member { .. }))
        && let Some(first_line) = lines.first()
        && first_line.len() == 1
        && let ChainExpression::Call {
            node_id: call_node_id,
            ..
        } = first_line[0]
        && !chain_node_has_non_inline_annotation(f.context(), call_node_id)
    {
        let first_line = lines.remove(0);
        base.body.push(first_line[0].clone());
    }

    // keep member + call pairs attached in argument chains:
    // `foo.bar.get(...)` should stay together before optional tails
    if is_call_like_argument(f.context(), node_id)
        && base
            .body
            .last()
            .is_some_and(|operation| matches!(operation, ChainExpression::Member { .. }))
        && let Some(first_line) = lines.first()
        && first_line.len() == 2
        && let (
            ChainExpression::Member {
                node_id: member_node_id,
                ..
            },
            ChainExpression::Call {
                node_id: call_node_id,
                ..
            },
        ) = (&first_line[0], &first_line[1])
        && !chain_node_has_non_inline_annotation(f.context(), *member_node_id)
        && !chain_node_has_non_inline_annotation(f.context(), *call_node_id)
    {
        let first_line = lines.remove(0);
        base.body.extend(first_line);
    }

    // keep short argument chains from fragmenting on their first member hops:
    // `foo.bar` + `.baz(...)` should render as `foo.bar.baz(...)` in argument positions
    if is_call_like_argument(f.context(), node_id)
        && lines.len() >= 2
        && matches!(lines[0].as_slice(), [ChainExpression::Member { .. }])
        && matches!(
            lines[1].first(),
            Some(ChainExpression::Member { .. } | ChainExpression::Call { .. })
        )
    {
        let mut first_line = lines.remove(0);
        let second_line = lines.remove(0);
        first_line.extend(second_line);
        lines.insert(0, first_line);
    }

    // indent chain lines consistently, even in assignment rhs positions
    let should_indent_chain = true;

    // inline variant keeps everything on one line when it fits
    let format_inline = format_with(|f| {
        format_chain_base(f, &base)?;
        for line in &lines {
            format_chain_expression_line(f, line)?;
        }
        Ok(())
    });
    // chain variant breaks each operation onto its own line
    let format_chain = format_with(|f| {
        // encourage the parent to break when the chain is complex
        if chain_should_break {
            write!(f, [expand_parent()])?;
        }

        group(&format_with(|f| {
            // always print the base first so indentation aligns subsequent lines
            format_chain_base(f, &base)?;
            // indent chained entries so each operation sits on its own line
            // use indent with manual line breaks instead of block_indent to avoid trailing newline
            // this keeps semicolons on the same line as the last chain element
            if !lines.is_empty() {
                let format_lines = format_with(|f| {
                    for comment in &deferred_path_boundary_comments {
                        write!(f, [hard_line_break(), text(comment.as_str())])?;
                    }

                    // each chain line renders in isolation to mirror prettier style
                    for (line_index, line) in lines.iter().enumerate() {
                        if line_index == 0
                            || !chain_line_starts_with_block_prefix_annotation(f.context(), line)
                        {
                            write!(f, [hard_line_break()])?;
                        }
                        format_chain_expression_line(f, line)?;
                    }
                    Ok(())
                });
                if should_indent_chain {
                    write!(f, [indent(&format_lines)])?;
                } else {
                    // avoid extra indentation when the parent already indents after `=`
                    write!(f, [format_lines])?;
                }
            }
            Ok(())
        }))
        .format(f)
    });

    let chain_has_calls = !chain_call_summaries.is_empty();
    if expression_is_in_template_literal_interpolation(f.context(), node_id) && !chain_has_calls {
        format_inline.format(f)?;
        return Ok(());
    }

    // chains that are clearly complex should not try the inline layout first
    if chain_should_break {
        write!(f, [group(&format_chain).should_expand(true)])?;
        return Ok(());
    }

    // prefer inline, otherwise chain
    best_fitting![format_inline, format_chain]
        .with_mode(BestFittingMode::AllLines)
        .format(f)
}
fn format_chain_base<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    base: &ChainExpressionBase,
) -> FormatResult<()> {
    match &base.head {
        ChainExpressionBaseHead::Path {
            node_id,
            segment,
            static_arguments,
            emit_postfix_annotations,
        } => {
            write!(f, [f.context().any_prefix_annotations(*node_id)])?;
            write!(f, [*segment])?;
            if let Some(arguments) = static_arguments {
                format_static_argument_list(f, arguments)?;
            }
            if *emit_postfix_annotations {
                write!(f, [f.context().any_infix_or_postfix_annotations(*node_id)])?;
            }
        }
        ChainExpressionBaseHead::Expression(node_id) => {
            let expression = f.context().tree.get(*node_id);
            debug_assert!(
                !matches!(
                    expression,
                    Expression::Member { .. }
                        | Expression::PrivateMember { .. }
                        | Expression::Call { .. }
                        | Expression::Index { .. }
                        | Expression::Maybe { .. }
                ),
                "chain base expression should not be another chain node"
            );
            write_postfix_base_expression(f, *node_id)?;
        }
    }

    for op in &base.body {
        format_chain_expression(f, op)?;
    }

    Ok(())
}

/// Format one chained operation.
fn format_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    op: &ChainExpression,
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
        | ChainExpression::Must { node_id, .. } => (*node_id, true, true),
    };
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
                format_static_argument_list(f, arguments)?;
            }
        }
        ChainExpression::Instantiation {
            static_arguments, ..
        } => {
            format_static_argument_list(f, static_arguments)?;
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
            format_call_dynamic_arguments_with_deferred_comments(
                f,
                *call_node_id,
                dynamic_arguments,
            )?;
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
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
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
    ops: &[ChainExpression],
) -> FormatResult<()> {
    for op in ops {
        format_chain_expression(f, op)?;
    }
    Ok(())
}

/// Return whether an expression has a line postfix boundary comment annotation.
fn expression_has_line_postfix_boundary_comment(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    context.get_annotations(node_id).is_some_and(|annotations| {
        annotations.iter().any(|annotation_id| {
            matches!(
                context.tree.get::<Annotation>(*annotation_id),
                Annotation::Comment {
                    position: AnnotationPosition::LinePostfixBoundary,
                    ..
                }
            )
        })
    })
}

/// Return whether a chain call has exactly one template literal argument.
fn chain_call_has_single_template_literal_argument(
    context: &DestackFormatContext<'_>,
    op: &ChainExpression,
) -> bool {
    let ChainExpression::Call {
        dynamic_arguments, ..
    } = op
    else {
        return false;
    };

    dynamic_arguments.len() == 1 && argument_is_template_literal(context, dynamic_arguments[0])
}

/// Group chain operations into the segments that should share lines.
fn group_chain_expression_lines(
    context: &DestackFormatContext<'_>,
    operations: Vec<ChainExpression>,
) -> Vec<SmallVec<[ChainExpression; 2]>> {
    let mut lines = Vec::new();
    let mut iter = operations.into_iter().peekable();
    while let Some(op) = iter.next() {
        let mut line = smallvec![op.clone()];
        match op {
            // (maybe)
            ChainExpression::Maybe { .. } => {
                match iter.peek() {
                    // (maybe, member)
                    Some(ChainExpression::Member { .. }) => {
                        line.push(iter.next().unwrap());
                        // (maybe, member, must?)
                        if let Some(ChainExpression::Must { .. }) = iter.peek() {
                            line.push(iter.next().unwrap());
                        }
                        // (maybe, member, index | call)
                        if let Some(
                            ChainExpression::Index { .. }
                            | ChainExpression::Call { .. }
                            | ChainExpression::Instantiation { .. },
                        ) = iter.peek()
                        {
                            line.push(iter.next().unwrap());
                        }

                        // keep short member tails with optional call chains:
                        // `?.foo().bar.baz`
                        let mut merged_member_count = 0usize;
                        while merged_member_count < 2 {
                            let Some(ChainExpression::Member { node_id, .. }) = iter.peek() else {
                                break;
                            };
                            if chain_node_has_non_inline_annotation(context, *node_id) {
                                break;
                            }
                            line.push(iter.next().unwrap());
                            merged_member_count += 1;
                        }
                    }
                    // (maybe, index | call)
                    Some(
                        ChainExpression::Index { .. }
                        | ChainExpression::Call { .. }
                        | ChainExpression::Instantiation { .. },
                    ) => {
                        line.push(iter.next().unwrap());
                        // (maybe, index | call, must?)
                        if let Some(ChainExpression::Must { .. }) = iter.peek() {
                            line.push(iter.next().unwrap());
                        }
                    }
                    _ => {}
                }
            }
            // (member)
            ChainExpression::Member { .. } => {
                // (member, must)
                if let Some(ChainExpression::Must { .. }) = iter.peek() {
                    line.push(iter.next().unwrap());
                }
                // (member, index | call)
                if let Some(
                    ChainExpression::Call { .. }
                    | ChainExpression::Index { .. }
                    | ChainExpression::Instantiation { .. },
                ) = iter.peek()
                {
                    line.push(iter.next().unwrap());
                }

                // keep direct curried calls attached: `foo(...)(...)`
                while let Some(ChainExpression::Call {
                    node_id,
                    position: PostfixPosition::Direct,
                    ..
                }) = iter.peek()
                {
                    if chain_node_has_non_inline_annotation(context, *node_id) {
                        break;
                    }
                    line.push(iter.next().unwrap());
                }

                // merge member runs that end in a call like op
                // this keeps tails like `.property.test.only(...)` together
                let line_has_call_like = line.iter().any(|operation| {
                    matches!(
                        operation,
                        ChainExpression::Call { .. }
                            | ChainExpression::Index { .. }
                            | ChainExpression::Instantiation { .. }
                    )
                });
                let should_merge_member_run = if line_has_call_like {
                    false
                } else {
                    let lookahead = iter.clone();
                    let mut member_run_count = 0usize;
                    let mut terminal_is_call = false;
                    let mut should_merge = false;
                    for next_operation in lookahead {
                        match next_operation {
                            ChainExpression::Member { .. } => {
                                member_run_count += 1;
                            }
                            ChainExpression::Must { .. } => {}
                            ChainExpression::Maybe { .. } => {
                                break;
                            }
                            ChainExpression::Call { .. } => {
                                terminal_is_call = true;
                                should_merge = member_run_count >= 2;
                                break;
                            }
                            ChainExpression::Index { .. }
                            | ChainExpression::Instantiation { .. } => {
                                break;
                            }
                        }
                    }
                    should_merge && terminal_is_call
                };
                if should_merge_member_run {
                    while let Some(ChainExpression::Member { .. }) = iter.peek() {
                        line.push(iter.next().unwrap());
                    }
                    if let Some(ChainExpression::Must { .. }) = iter.peek() {
                        line.push(iter.next().unwrap());
                    }
                    if let Some(ChainExpression::Call { .. }) = iter.peek() {
                        line.push(iter.next().unwrap());
                    }
                }

                // keep short member tails together before terminal call-like operations
                let should_merge_member_tail = line
                    .last()
                    .is_some_and(|op| chain_call_has_single_template_literal_argument(context, op));

                if should_merge_member_tail {
                    let mut merged_member_count = 0usize;
                    while merged_member_count < 2 {
                        let Some(ChainExpression::Member { .. }) = iter.peek() else {
                            break;
                        };

                        line.push(iter.next().unwrap());
                        merged_member_count += 1;
                    }

                    if let Some(ChainExpression::Must { .. }) = iter.peek() {
                        line.push(iter.next().unwrap());
                    }

                    if let Some(
                        ChainExpression::Call { .. }
                        | ChainExpression::Index { .. }
                        | ChainExpression::Instantiation { .. },
                    ) = iter.peek()
                    {
                        line.push(iter.next().unwrap());
                    }
                }
            }
            // (index | call | instantiation)
            ChainExpression::Index { .. }
            | ChainExpression::Call { .. }
            | ChainExpression::Instantiation { .. } => {
                // (index | call | instantiation, must)
                if let Some(ChainExpression::Must { .. }) = iter.peek() {
                    line.push(iter.next().unwrap());
                }

                // (call, call, ...) for curried call tails
                while let Some(ChainExpression::Call {
                    node_id,
                    position: PostfixPosition::Direct,
                    ..
                }) = iter.peek()
                {
                    if chain_node_has_non_inline_annotation(context, *node_id) {
                        break;
                    }
                    line.push(iter.next().unwrap());
                }
            }
            // (must)
            ChainExpression::Must { .. } => {
                // (must, member)
                if let Some(ChainExpression::Member { .. }) = iter.peek() {
                    line.push(iter.next().unwrap());
                }
                // (must, index | call)
                if let Some(
                    ChainExpression::Call { .. }
                    | ChainExpression::Index { .. }
                    | ChainExpression::Instantiation { .. },
                ) = iter.peek()
                {
                    line.push(iter.next().unwrap());
                }
            }
        }
        lines.push(line);
    }

    lines
}
