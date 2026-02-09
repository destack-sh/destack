use super::*;
use crate::r#match::{MatchCaseStyle, format_match_case_with_style};
use destack_ast::BlockFormat;
use destack_fir::{format_args, write};

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

/// Return whether node has a line postfix slash comment annotation.
fn has_line_postfix_slash_comment<'ast, T>(
    context: &DestackFormatContext<'ast>,
    node_id: LocalNodeId<T>,
) -> bool
where
    T: destack_ast::Node,
    NodeTree: destack_ast::NodeTreeImpl<T>,
{
    let Some(annotations) = context.get_annotations(node_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        let annotation = context.tree.get::<Annotation>(annotation_id);
        let Annotation::Comment { node, position } = annotation else {
            return false;
        };
        if !matches!(
            position,
            AnnotationPosition::LinePostfix | AnnotationPosition::LinePostfixBoundary
        ) {
            return false;
        }
        let comment = context.tree.get::<destack_ast::Comment>(*node);
        comment.style == destack_ast::CommentStyle::Slash
    })
}

/// Return an inline block comment source for an else boundary when safe.
fn inline_else_boundary_block_comment<'ast>(
    context: &DestackFormatContext<'ast>,
    then_expression_id: LocalNodeId<Expression>,
    else_expression_id: LocalNodeId<Expression>,
) -> Option<(String, bool)> {
    let annotations = context.get_annotations(else_expression_id)?;
    let mut prefix_comment_annotation: Option<LocalNodeId<Annotation>> = None;

    for annotation_id in annotations {
        let annotation = context.tree.get::<Annotation>(annotation_id);
        match annotation {
            Annotation::Comment { node, position } => {
                if !matches!(
                    position,
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) {
                    continue;
                }
                let comment = context.tree.get::<destack_ast::Comment>(*node);
                if comment.style != destack_ast::CommentStyle::Star {
                    return None;
                }
                if prefix_comment_annotation.replace(annotation_id).is_some() {
                    return None;
                }
            }
            Annotation::Blank { position, .. }
            | Annotation::Doc { position, .. }
            | Annotation::Decorator { position, .. } => {
                if matches!(
                    position,
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) {
                    return None;
                }
            }
        }
    }

    let prefix_comment_annotation = prefix_comment_annotation?;
    let comment_span = context.get_span::<Annotation>(prefix_comment_annotation);
    let comment_source = context.get_span_str(comment_span);
    let comment_source = comment_source.trim();
    if !comment_source.starts_with("/*") || !comment_source.ends_with("*/") {
        return None;
    }

    let then_span = context.get_span(then_expression_id);
    let else_span = context.get_span(else_expression_id);
    if then_span.file != else_span.file || then_span.end > else_span.start {
        return None;
    }
    let between_span = Span::new(then_span.file, then_span.end, else_span.start);
    let between_source = context.get_span_str(between_span);
    let comment_index = between_source.find(comment_source)?;
    let comment_is_on_new_line = between_source[..comment_index].contains('\n');

    Some((comment_source.to_string(), comment_is_on_new_line))
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
                let then_is_empty_statement = matches!(
                    f.context().tree.get(*then_expression_id),
                    Expression::Block(block_id) if is_empty_statement_block(f.context(), *block_id)
                );

                // if <condition>
                match condition {
                    IfCondition::Expression { condition } => {
                        write!(
                            f,
                            [Keyword::If, space(), token("("), *condition, token(")")]
                        )?;
                        if !then_is_empty_statement {
                            write!(f, [space()])?;
                        }
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
                        if !then_is_empty_statement {
                            write!(f, [space()])?;
                        }
                    }
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
                    let has_line_postfix_slash_on_boundary =
                        has_line_postfix_slash_comment(f.context(), next_if_id)
                            || has_line_postfix_slash_comment(f.context(), *then_expression_id);
                    let inline_else_block_comment = inline_else_boundary_block_comment(
                        f.context(),
                        *then_expression_id,
                        *else_expression,
                    );
                    let consume_else_prefix_annotations = inline_else_block_comment.is_some();
                    if has_line_postfix_slash_on_boundary {
                        write!(f, [line_postfix_boundary()])?;
                    }

                    if let Some((comment_source, comment_is_on_new_line)) =
                        inline_else_block_comment
                    {
                        if comment_is_on_new_line {
                            write!(
                                f,
                                [hard_line_break(), text(comment_source.as_str()), space()]
                            )?;
                        } else {
                            write!(f, [space(), text(comment_source.as_str()), space()])?;
                        }
                    } else if !has_line_postfix_slash_on_boundary
                        && !f.context().has_prefix_annotation(*else_expression)
                    {
                        write!(f, [space()])?;
                    }
                    match f.context().tree.get(*else_expression) {
                        // else if
                        Expression::If { .. } => {
                            if !consume_else_prefix_annotations {
                                write!(f, [f.context().any_prefix_annotations(*else_expression)])?;
                            }
                            write!(f, [Keyword::Else, space()])?;
                            // (postfix is covered by the next if above)
                            next_if_id = *else_expression;
                        }
                        // else
                        Expression::Block(else_block_id) => {
                            if !consume_else_prefix_annotations {
                                write!(f, [f.context().any_prefix_annotations(*else_expression)])?;
                            }
                            write!(f, [Keyword::Else, space()])?;
                            format_statement_body_block(f, *else_block_id)?;
                            write!(f, [f.context().any_postfix_annotations(*else_expression)])?;
                            break;
                        }
                        // something else
                        _ => {
                            let directive = directive_for_node(f.context(), *else_expression);
                            if !consume_else_prefix_annotations {
                                write!(f, [f.context().any_prefix_annotations(*else_expression)])?;
                            }
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
