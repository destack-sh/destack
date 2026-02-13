use super::*;
use crate::annotation::{
    annotation_should_defer, if_head_boundary_annotations, if_then_else_boundary_annotations,
};
use crate::r#match::{MatchCaseStyle, format_match_case_with_style};
use destack_ast::BlockFormat;
use destack_fir::{format_args, write};
use destack_source::Span;

/// Format a statement body block, preserving wrapper semantics.
pub(super) fn format_statement_body_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    block_id: LocalNodeId<Block>,
) -> FormatResult<()> {
    let block = f.context().tree.get(block_id);
    let is_statement_wrapper = is_statement_wrapper_block(f.context(), block_id);
    if !is_statement_wrapper {
        write!(f, [block_id])?;
        return Ok(());
    }

    write!(f, [f.context().any_prefix_annotations(block_id)])?;

    if block.expressions.is_empty() {
        write!(f, [token(";")])?;
    } else if block.expressions.len() == 1 {
        let expression_id = block.expressions[0];
        write!(f, [expression_id])?;

        let expression = f.context().tree.get(expression_id);
        let needs_terminator =
            !matches!(expression, Expression::Statement(_)) && !expression.is_top_level_statement();
        if needs_terminator {
            write!(f, [token(";")])?;
        }
    } else {
        write!(f, [block_id])?;
    }

    write!(f, [f.context().any_infix_or_postfix_annotations(block_id)])?;
    Ok(())
}

/// Return true when this block originated from a statement wrapper instead of braces.
fn is_statement_wrapper_block<'ast>(
    context: &DestackFormatContext<'ast>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    block.format == BlockFormat::Implicit
}

/// Return true when this block is an empty statement wrapper.
pub(super) fn is_empty_statement_block<'ast>(
    context: &DestackFormatContext<'ast>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    is_statement_wrapper_block(context, block_id) && block.expressions.is_empty()
}

/// Detect a source binding keyword for a for each pattern binding.
pub(super) fn detect_for_each_binding_keyword<'ast>(
    context: &DestackFormatContext<'ast>,
    _for_each_id: LocalNodeId<Expression>,
    pattern_id: LocalNodeId<Pattern>,
) -> Option<Keyword> {
    let pattern_span = context.get_span(pattern_id);
    let pattern_source = context.get_span_str(pattern_span);
    let pattern_source = pattern_source.trim_start();
    if pattern_source.starts_with("let ") {
        return Some(Keyword::Let);
    }
    if pattern_source.starts_with("const ") {
        return Some(Keyword::Const);
    }
    if pattern_source.starts_with("var ") {
        return Some(Keyword::Var);
    }
    None
}

/// Format a for each binding pattern without repeating root mutability keywords.
pub(super) fn format_for_each_binding_pattern<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    pattern_id: LocalNodeId<Pattern>,
) -> FormatResult<()> {
    match f.context().tree.get(pattern_id) {
        Pattern::Binding { name, pattern, .. } => {
            write!(f, [name])?;
            if let Some(pattern) = pattern {
                write!(f, [token(":"), space(), pattern])?;
            }
            Ok(())
        }
        _ => write!(f, [pattern_id]),
    }
}

/// Return whether expression annotations include a block prefix annotation.
fn expression_has_block_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.get_annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        matches!(
            context.tree.get::<Annotation>(annotation_id),
            Annotation::Blank {
                position: AnnotationPosition::BlockPrefix,
                ..
            } | Annotation::Doc {
                position: AnnotationPosition::BlockPrefix,
                ..
            } | Annotation::Comment {
                position: AnnotationPosition::BlockPrefix,
                ..
            } | Annotation::Decorator {
                position: AnnotationPosition::BlockPrefix,
                ..
            }
        )
    })
}

/// Return whether expression annotations include a line prefix annotation.
fn expression_has_line_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.get_annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        matches!(
            context.tree.get::<Annotation>(annotation_id),
            Annotation::Blank {
                position: AnnotationPosition::LinePrefix,
                ..
            } | Annotation::Doc {
                position: AnnotationPosition::LinePrefix,
                ..
            } | Annotation::Comment {
                position: AnnotationPosition::LinePrefix,
                ..
            } | Annotation::Decorator {
                position: AnnotationPosition::LinePrefix,
                ..
            }
        )
    })
}

/// Return whether expression has any prefix annotation that is not deferred away.
fn expression_has_effective_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.get_annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let position = context.tree.get::<Annotation>(annotation_id).position();
        let is_prefix = matches!(
            position,
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        );
        if !is_prefix {
            return false;
        }

        !annotation_should_defer(context, expression_id, annotation_id, position)
    })
}

/// Return whether an if branch should include a space after the condition head.
fn if_branch_head_requires_space(
    context: &DestackFormatContext<'_>,
    then_expression_id: LocalNodeId<Expression>,
) -> bool {
    let then_is_empty_statement = matches!(
        context.tree.get(then_expression_id),
        Expression::Block(block_id) if is_empty_statement_block(context, *block_id)
    );

    if expression_has_block_prefix_annotation(context, then_expression_id) {
        return false;
    }

    if expression_has_line_prefix_annotation(context, then_expression_id) {
        return true;
    }

    !then_is_empty_statement
}

/// Return whether annotation is a slash comment.
fn annotation_is_slash_comment(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let Annotation::Comment { node, .. } = context.tree.get::<Annotation>(annotation_id) else {
        return false;
    };

    let comment = context.tree.get::<destack_ast::Comment>(*node);
    comment.style == destack_ast::CommentStyle::Slash
}

/// Return whether an annotation starts on its own source line.
fn annotation_starts_on_own_line(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation_span = context.get_span::<Annotation>(annotation_id);
    let head_span = Span::new(annotation_span.file, 0, annotation_span.start);
    let head_source = context.file.get_span_str(head_span).unwrap_or_default();
    let line_start = head_source.rfind('\n').map_or(0, |index| index + 1);
    head_source[line_start..].trim().is_empty()
}

/// Write deferred if boundary annotations with stable separator spacing.
fn write_deferred_if_boundary_annotation_cluster<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    annotation_ids: &[LocalNodeId<Annotation>],
) -> FormatResult<()> {
    if annotation_ids.is_empty() {
        return Ok(());
    }

    let mut previous_was_slash = false;
    for (index, annotation_id) in annotation_ids.iter().copied().enumerate() {
        let is_slash = annotation_is_slash_comment(f.context(), annotation_id);
        let starts_on_own_line = annotation_starts_on_own_line(f.context(), annotation_id);

        // separate each deferred boundary annotation with stable seam spacing
        if starts_on_own_line {
            write!(f, [hard_line_break()])?;
        } else if index == 0 || !previous_was_slash {
            write!(f, [space()])?;
        }

        let annotation_span = f.context().get_span::<Annotation>(annotation_id);
        let annotation_source = f.context().get_span_str(annotation_span);
        write!(f, [text(annotation_source.trim())])?;
        if is_slash {
            write!(f, [hard_line_break()])?;
        }
        previous_was_slash = is_slash;
    }

    if !previous_was_slash {
        write!(f, [space()])?;
    }

    Ok(())
}

/// Write deferred boundary annotations between if head and then body.
fn write_deferred_if_head_boundary_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    if_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let annotation_ids = if_head_boundary_annotations(f.context(), if_id);
    if annotation_ids.is_empty() {
        return Ok(false);
    }

    write_deferred_if_boundary_annotation_cluster(f, annotation_ids.as_slice())?;
    Ok(true)
}

/// Write deferred boundary annotations between then body and else branch.
fn write_deferred_if_then_else_boundary_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    if_id: LocalNodeId<Expression>,
    then_id: LocalNodeId<Expression>,
    else_id: LocalNodeId<Expression>,
) -> FormatResult<bool> {
    let annotation_ids = if_then_else_boundary_annotations(f.context(), if_id, then_id, else_id);
    if annotation_ids.is_empty() {
        return Ok(false);
    }

    write_deferred_if_boundary_annotation_cluster(f, annotation_ids.as_slice())?;
    Ok(true)
}

/// Walk a chain of if expressions and collect the if/else if/else nodes.
pub(crate) fn format_if_else_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // walk the chain
    let mut next_if_id = node_id;
    loop {
        let if_node = f.context().tree.get(next_if_id);
        match if_node {
            // if or else if
            Expression::If {
                kind: _, // we turn everything into regular ifs
                condition,
                then_expression: then_expression_id,
                else_expression: else_expression_id,
            } => {
                let then_requires_head_space =
                    if_branch_head_requires_space(f.context(), *then_expression_id);

                // if <condition>
                match condition {
                    IfCondition::Expression { condition } => {
                        write!(
                            f,
                            [Keyword::If, space(), token("("), *condition, token(")")]
                        )?;
                    }
                    IfCondition::Let {
                        kind,
                        mutability: _,
                        declarator,
                    } => {
                        write!(f, [Keyword::If, space()])?;
                        match kind {
                            LetKind::Let => write!(f, [Keyword::Let])?,
                            LetKind::Var => write!(f, [Keyword::Var])?,
                            LetKind::Const => write!(f, [Keyword::Const])?,
                        }
                        write!(f, [space()])?;
                        format_declarator(f, f.context().tree, *declarator)?;
                    }
                }

                // write deferred boundary annotations between the condition head and body
                let wrote_deferred_head_boundary_annotations =
                    write_deferred_if_head_boundary_annotations(f, next_if_id)?;

                // insert canonical spacing before the then expression when no deferred boundary exists
                if then_requires_head_space && !wrote_deferred_head_boundary_annotations {
                    write!(f, [space()])?;
                }

                // then block
                let then_expression = f.context().tree.get(*then_expression_id);
                match then_expression {
                    Expression::Block(block_id) => {
                        write!(f, [f.context().any_prefix_annotations(*then_expression_id)])?;
                        format_statement_body_block(f, *block_id)?;
                        write!(
                            f,
                            [f.context()
                                .any_infix_or_postfix_annotations(*then_expression_id)]
                        )?;
                    }
                    // something else
                    _ => write!(f, [*then_expression_id])?,
                }

                // postfix annotations
                if else_expression_id.is_some() || next_if_id != node_id {
                    write!(f, [f.context().any_postfix_annotations(next_if_id)])?;
                }

                // next node
                if let Some(else_expression) = else_expression_id {
                    // write deferred boundary annotations between then and else
                    let wrote_deferred_then_else_boundary_annotations =
                        write_deferred_if_then_else_boundary_annotations(
                            f,
                            next_if_id,
                            *then_expression_id,
                            *else_expression,
                        )?;

                    // keep compact spacing when no deferred boundary annotations or else prefixes force layout
                    let else_has_effective_prefix_annotation =
                        expression_has_effective_prefix_annotation(f.context(), *else_expression);
                    if !wrote_deferred_then_else_boundary_annotations
                        && !else_has_effective_prefix_annotation
                    {
                        write!(f, [space()])?;
                    }
                    match f.context().tree.get(*else_expression) {
                        // else if
                        Expression::If { .. } => {
                            write!(f, [f.context().any_prefix_annotations(*else_expression)])?;
                            write!(f, [Keyword::Else, space()])?;
                            // (postfix is covered by the next if above)
                            next_if_id = *else_expression;
                        }
                        // else
                        Expression::Block(else_block_id) => {
                            write!(f, [f.context().any_prefix_annotations(*else_expression)])?;
                            write!(f, [Keyword::Else, space()])?;
                            format_statement_body_block(f, *else_block_id)?;
                            write!(f, [f.context().any_postfix_annotations(*else_expression)])?;
                            break;
                        }
                        // something else
                        _ => {
                            let directive = directive_for_node(f.context(), *else_expression);
                            write!(f, [f.context().any_prefix_annotations(*else_expression)])?;
                            write!(f, [Keyword::Else, space()])?;
                            format_expression(
                                f,
                                *else_expression,
                                f.context().tree.get(*else_expression),
                                directive,
                            )?;
                            if !matches!(
                                directive,
                                Some(FormatterDirective {
                                    kind: FormatterDirectiveKind::IgnoreFormat,
                                    position: FormatterDirectivePosition::Postfix { .. },
                                })
                            ) {
                                write!(
                                    f,
                                    [f.context()
                                        .any_infix_or_postfix_annotations(*else_expression)]
                                )?;
                            }
                            break;
                        }
                    }
                } else {
                    // bare if
                    break;
                }
            }
            // shouldn't be anything else
            _ => {
                return Err(FormatError::SyntaxError {
                    message: "unexpected expression kind for if chain",
                });
            }
        }
    }
    Ok(())
}
/// Format a match expression.
pub(crate) fn format_match<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    include_prefix: bool,
) -> FormatResult<()> {
    let match_node = f.context().tree.get(node_id);
    let Expression::Match { kind, value, cases } = &match_node else {
        return Err(FormatError::SyntaxError {
            message: "invalid match expression",
        });
    };
    let kind = *kind;
    let case_style = match kind {
        MatchKind::Match => MatchCaseStyle::Match,
        MatchKind::Switch => MatchCaseStyle::Switch,
    };

    if include_prefix {
        // match/switch <expression>
        let keyword = match kind {
            MatchKind::Match => Keyword::Match,
            MatchKind::Switch => Keyword::Switch,
        };
        write!(f, [keyword, space()])?;
    }

    write!(f, [token("("), value, token(")")])?;

    // empty match body
    if cases.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        return Ok(());
    }

    // match/switch cases
    write!(f, [space(), token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            let mut first = true;
            for case_id in cases {
                if !first {
                    write!(f, [hard_line_break()])?;
                }
                first = false;
                format_match_case_with_style(f, *case_id, case_style)?;
            }
            Ok(())
        })),])]
    )?;
    write!(f, [f.context().block_infix_annotations(node_id)])?;
    write!(f, [hard_line_break(), token("}")])?;

    Ok(())
}
