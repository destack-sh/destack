use crate::format::analysis::{previous_non_whitespace_token_before_span, token_is_keyword};
use crate::format::annotation::statement_wrapper_needs_semicolon;
use crate::format::declaration::statement::format_block_of_statements;
use crate::format::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
};
use crate::format::expression::{
    Annotation, AnnotationPosition, Block, DestackFormatContext, DestackFormatter, Expression,
    FormatResult, IfCondition, Keyword, LetKind, LocalNodeId, MatchKind, Pattern, TokenType,
    block_indent, format_declarator, format_expression, format_with, group, hard_line_break,
    line_postfix_boundary, space, token,
};
use crate::{FormatNode, empty_block_with_infix_annotations};
use destack_ast::{BlockFormat, CommentStyle, MatchCase, MatchSelector};
use destack_fir::format::{Buffer, FormatError};
use destack_fir::{format_args, write};

/// Format one statement-body expression with statement-separator semantics.
fn format_statement_body_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let expression = f.context().tree.get(expression_id);
    let directive = directive_for_node(f.context(), expression_id);

    write!(f, [f.context().any_prefix_annotations(expression_id)])?;
    format_expression(f, expression_id, expression, directive)?;

    if statement_wrapper_needs_semicolon(f.context(), expression_id) {
        write!(f, [token(";")])?;
    }

    let if_chain_handles_annotations = matches!(
        expression,
        Expression::If {
            kind: destack_ast::IfKind::If,
            ..
        }
    );
    let directive_owns_postfix_annotations = matches!(
        directive,
        Some(FormatterDirective {
            kind: FormatterDirectiveKind::IgnoreFormat,
            position: FormatterDirectivePosition::Postfix { .. },
        })
    );
    if !if_chain_handles_annotations && !directive_owns_postfix_annotations {
        write!(
            f,
            [f.context().any_infix_or_postfix_annotations(expression_id)]
        )?;
    }

    Ok(())
}

/// Format a statement body block, preserving wrapper semantics.
pub(crate) fn format_statement_body_block<'ast>(
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
        let has_expression_prefix_annotation = f.context().has_prefix_annotation(expression_id);
        if has_expression_prefix_annotation {
            write!(
                f,
                [
                    hard_line_break(),
                    group(&block_indent(&format_with(|f| {
                        format_statement_body_expression(f, expression_id)
                    })))
                ]
            )?;
        } else {
            format_statement_body_expression(f, expression_id)?;
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
pub(crate) fn is_empty_statement_block<'ast>(
    context: &DestackFormatContext<'ast>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let block = context.tree.get(block_id);
    is_statement_wrapper_block(context, block_id) && block.expressions.is_empty()
}

/// Detect a source binding keyword for a for each pattern binding.
pub(crate) fn detect_for_each_binding_keyword<'ast>(
    context: &DestackFormatContext<'ast>,
    _for_each_id: LocalNodeId<Expression>,
    pattern_id: LocalNodeId<Pattern>,
) -> Option<Keyword> {
    let pattern_span = context.span(pattern_id);
    let keyword_token = previous_non_whitespace_token_before_span(context, pattern_span)?;
    if keyword_token.token.ty != TokenType::Identifier {
        return None;
    }

    if token_is_keyword(context, keyword_token, Keyword::Let) {
        Some(Keyword::Let)
    } else if token_is_keyword(context, keyword_token, Keyword::Const) {
        Some(Keyword::Const)
    } else if token_is_keyword(context, keyword_token, Keyword::Var) {
        Some(Keyword::Var)
    } else {
        None
    }
}

/// Format a for each binding pattern without repeating root mutability keywords.
pub(crate) fn format_for_each_binding_pattern<'ast>(
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

/// Return whether an if branch should include a space after the condition head.
fn expression_has_block_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        matches!(
            context.annotation(annotation_id),
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
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        matches!(
            context.annotation(annotation_id),
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

/// Return whether expression has any prefix annotation.
fn expression_has_effective_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotations) = context.annotations(expression_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        matches!(
            context.annotation(annotation_id).position(),
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        )
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
        return false;
    }

    !then_is_empty_statement
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
                            [
                                Keyword::If,
                                space(),
                                token("("),
                                *condition,
                                line_postfix_boundary(),
                                token(")")
                            ]
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

                // insert canonical spacing before the then expression
                if then_requires_head_space {
                    write!(f, [space()])?;
                }

                // then block
                let then_expression = f.context().tree.get(*then_expression_id);
                let then_is_empty_statement = matches!(
                    then_expression,
                    Expression::Block(block_id) if is_empty_statement_block(f.context(), *block_id)
                );
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

                // next node
                if let Some(else_expression) = else_expression_id {
                    let else_has_effective_prefix_annotation =
                        expression_has_effective_prefix_annotation(f.context(), *else_expression);

                    // keep if-else seams tight: only non-blank postfix stays before else
                    if else_has_effective_prefix_annotation
                        || f.context().has_non_blank_postfix_annotation(next_if_id)
                    {
                        write!(f, [f.context().any_postfix_annotations(next_if_id)])?;
                    }

                    // keep compact spacing when else prefixes do not force layout
                    let if_has_postfix_annotation = f.context().has_postfix_annotation(next_if_id);
                    let then_has_postfix_annotation =
                        f.context().has_postfix_annotation(*then_expression_id);
                    if else_has_effective_prefix_annotation
                        || if_has_postfix_annotation
                        || then_has_postfix_annotation
                        || then_is_empty_statement
                    {
                        write!(f, [hard_line_break()])?;
                    } else {
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
                            if f.context()
                                .has_non_blank_postfix_annotation(*else_expression)
                            {
                                write!(f, [f.context().any_postfix_annotations(*else_expression)])?;
                            }
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
                    write!(f, [f.context().any_postfix_annotations(next_if_id)])?;
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

/// Match case rendering style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MatchCaseStyle {
    /// Emit `pattern => body`.
    Match,
    /// Emit `case pattern: body` and `default: body`.
    Switch,
}

/// Return whether a match case has line postfix boundary annotations.
fn match_case_has_line_postfix_boundary_annotation(
    context: &DestackFormatContext<'_>,
    case_id: LocalNodeId<MatchCase>,
) -> bool {
    context.has_boundary_comment_annotation(case_id)
}

/// Return whether one match case has an inline block boundary comment.
fn match_case_has_inline_star_line_postfix_boundary_comment(
    context: &DestackFormatContext<'_>,
    case_id: LocalNodeId<MatchCase>,
) -> bool {
    if !context.has_boundary_comment_annotation(case_id) {
        return false;
    }

    context
        .visit_annotations(case_id, |annotation_ids| {
            annotation_ids.iter().copied().any(|annotation_id| {
                let Annotation::Comment {
                    node,
                    position: AnnotationPosition::LinePostfixBoundary,
                } = context.annotation(annotation_id)
                else {
                    return false;
                };

                let comment = context.tree.get(node);
                comment.style == CommentStyle::Star
                    && !context.annotation_starts_on_own_line(annotation_id)
            })
        })
        .unwrap_or(false)
}

/// Format a match selector according to the selected case style.
fn format_selector_with_style(
    f: &mut DestackFormatter<'_, '_>,
    selector: &MatchSelector,
    style: MatchCaseStyle,
) -> FormatResult<()> {
    match style {
        MatchCaseStyle::Match => match selector {
            MatchSelector::Pattern { pattern, guard } => {
                write!(f, [*pattern])?;
                if let Some(guard) = guard {
                    write!(
                        f,
                        [
                            space(),
                            Keyword::If,
                            space(),
                            token("("),
                            *guard,
                            token(")")
                        ]
                    )?;
                }
            }
            MatchSelector::Default => {
                write!(f, [token("_")])?;
            }
        },
        MatchCaseStyle::Switch => match selector {
            MatchSelector::Pattern { pattern, guard } => {
                write!(f, [Keyword::Case, space(), *pattern, token(":")])?;
                if let Some(guard) = guard {
                    write!(
                        f,
                        [
                            space(),
                            Keyword::If,
                            space(),
                            token("("),
                            *guard,
                            token(")")
                        ]
                    )?;
                }
            }
            MatchSelector::Default => {
                write!(f, [Keyword::Default, token(":")])?;
            }
        },
    }

    Ok(())
}

/// Format one match case with the selected style.
pub(crate) fn format_match_case_with_style<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    case_id: LocalNodeId<MatchCase>,
    style: MatchCaseStyle,
) -> FormatResult<()> {
    let case = f.context().tree.get(case_id);
    let has_line_postfix_boundary_annotation =
        match_case_has_line_postfix_boundary_annotation(f.context(), case_id);
    let has_inline_star_line_postfix_boundary_comment =
        match_case_has_inline_star_line_postfix_boundary_comment(f.context(), case_id);

    // case prefix
    write!(f, [f.context().any_prefix_annotations(case_id)])?;

    // selector, separator, and body
    match case {
        MatchCase::Expression { selector, body } => {
            format_selector_with_style(f, selector, style)?;
            if has_line_postfix_boundary_annotation {
                write!(f, [f.context().line_postfix_boundary_annotations(case_id)])?;
            }
            match style {
                MatchCaseStyle::Match => {
                    write!(f, [space(), token("=>"), space(), *body])?;
                }
                MatchCaseStyle::Switch => {
                    if let Some(explicit_block_expression) =
                        switch_case_expression_body_collapsed_explicit_block_expression(
                            f.context(),
                            *body,
                        )
                    {
                        if has_line_postfix_boundary_annotation {
                            if has_inline_star_line_postfix_boundary_comment {
                                write!(f, [space(), explicit_block_expression])?;
                            } else {
                                write!(f, [explicit_block_expression])?;
                            }
                        } else {
                            write!(f, [space(), explicit_block_expression])?;
                        }
                    } else if switch_case_expression_body_should_break(f, *body) {
                        if has_line_postfix_boundary_annotation {
                            write!(f, [block_indent(body)])?;
                        } else {
                            write!(f, [hard_line_break(), block_indent(body)])?;
                        }
                    } else if has_line_postfix_boundary_annotation {
                        if has_inline_star_line_postfix_boundary_comment {
                            write!(f, [space(), *body])?;
                        } else {
                            write!(f, [*body])?;
                        }
                    } else {
                        write!(f, [space(), *body])?;
                    }
                }
            }
        }
        MatchCase::Block { selector, body } => {
            format_selector_with_style(f, selector, style)?;
            if has_line_postfix_boundary_annotation {
                write!(f, [f.context().line_postfix_boundary_annotations(case_id)])?;
            }
            match style {
                MatchCaseStyle::Match => {
                    write!(f, [space(), token("=>"), space(), *body])?;
                }
                MatchCaseStyle::Switch => {
                    let block = f.context().tree.get(*body);
                    if block.format == BlockFormat::Implicit {
                        if let Some(explicit_block_expression) =
                            switch_case_implicit_body_single_explicit_block_expression(
                                f.context(),
                                *body,
                            )
                        {
                            if has_line_postfix_boundary_annotation {
                                if has_inline_star_line_postfix_boundary_comment {
                                    write!(f, [space(), explicit_block_expression])?;
                                } else {
                                    write!(f, [explicit_block_expression])?;
                                }
                            } else {
                                write!(f, [space(), explicit_block_expression])?;
                            }
                        } else if !block.expressions.is_empty() {
                            if !has_line_postfix_boundary_annotation {
                                write!(f, [hard_line_break()])?;
                            }
                            write!(
                                f,
                                [block_indent(&format_with(|f| {
                                    format_block_of_statements(f, &block.expressions, false)
                                }))]
                            )?;
                        }
                    } else if has_line_postfix_boundary_annotation {
                        if has_inline_star_line_postfix_boundary_comment {
                            write!(f, [space(), *body])?;
                        } else {
                            write!(f, [*body])?;
                        }
                    } else {
                        write!(f, [space(), *body])?;
                    }
                }
            }
        }
    }

    // case postfix
    write!(
        f,
        [f.context()
            .any_infix_or_postfix_except_line_postfix_boundary_annotations(case_id)]
    )?;

    Ok(())
}

/// Return whether a switch case expression body should render on its own line.
fn switch_case_expression_body_should_break(
    f: &DestackFormatter<'_, '_>,
    body_expression_id: LocalNodeId<Expression>,
) -> bool {
    f.context().has_annotation(body_expression_id)
        || f.context().node_has_newline(body_expression_id)
}

/// Return one expression id with statement wrappers removed.
fn switch_case_expression_without_statement_wrapper(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut expression_id = expression_id;

    loop {
        let Expression::Statement(inner_expression_id) = context.tree.get(expression_id) else {
            return expression_id;
        };
        expression_id = *inner_expression_id;
    }
}

/// Return whether one switch case expression body is one explicit block expression.
fn switch_case_expression_body_is_explicit_block(
    context: &DestackFormatContext<'_>,
    body_expression_id: LocalNodeId<Expression>,
) -> bool {
    let body_expression_id =
        switch_case_expression_without_statement_wrapper(context, body_expression_id);

    let Expression::Block(block_id) = context.tree.get(body_expression_id) else {
        return false;
    };

    let block = context.tree.get(*block_id);
    block.format == BlockFormat::Explicit
}

/// Return whether one expression is an empty statement block wrapper.
fn expression_is_empty_statement_block(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = switch_case_expression_without_statement_wrapper(context, expression_id);

    let Expression::Block(block_id) = context.tree.get(expression_id) else {
        return false;
    };

    let block = context.tree.get(*block_id);
    block.format == BlockFormat::Implicit && block.expressions.is_empty()
}

/// Return one explicit block expression for one switch case body when collapsible.
fn switch_case_expression_body_collapsed_explicit_block_expression(
    context: &DestackFormatContext<'_>,
    body_expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    let body_expression_id =
        switch_case_expression_without_statement_wrapper(context, body_expression_id);

    if switch_case_expression_body_is_explicit_block(context, body_expression_id) {
        return Some(body_expression_id);
    }

    let Expression::Block(block_id) = context.tree.get(body_expression_id) else {
        return None;
    };

    switch_case_implicit_body_single_explicit_block_expression(context, *block_id)
}

/// Return one explicit block expression when one implicit switch body only wraps that block.
fn switch_case_implicit_body_single_explicit_block_expression(
    context: &DestackFormatContext<'_>,
    implicit_block_id: LocalNodeId<Block>,
) -> Option<LocalNodeId<Expression>> {
    let implicit_block = context.tree.get(implicit_block_id);
    if implicit_block.format != BlockFormat::Implicit {
        return None;
    }

    let mut explicit_block_expression = None;
    for expression_id in implicit_block.expressions.iter().copied() {
        if expression_is_empty_statement_block(context, expression_id) {
            continue;
        }

        if explicit_block_expression.is_some() {
            return None;
        }

        if !switch_case_expression_body_is_explicit_block(context, expression_id) {
            return None;
        }

        explicit_block_expression = Some(expression_id);
    }

    explicit_block_expression
}

impl<'ast> FormatNode<'ast, MatchCase> for MatchCase {
    fn format_node(
        &self,
        node_id: LocalNodeId<MatchCase>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        format_match_case_with_style(f, node_id, MatchCaseStyle::Match)
    }
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

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    #[test]
    fn test_format_match_expression_cases() {
        assert_format!(
            "match (x) { 1 => 2; 3 => 4 }",
            "match (x) {\n\t1 => 2\n\t3 => 4\n}",
            |p| p.eat_match(),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_match_with_block_case_and_guard() {
        assert_format!(
            "match (value) { Pattern if (cond) => { const X = 1; } }",
            "match (value) {\n\tPattern if (cond) => {\n\t\tconst X = 1;\n\t}\n}",
            |p| p.eat_match(),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_switch_expression_cases() {
        assert_format!(
            "switch (x) { case 1: 2; case 3: 4 }",
            "switch (x) {\n\tcase 1: 2\n\tcase 3: 4\n}",
            |p| p.eat_match(),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_switch_with_default_case() {
        assert_format!(
            "switch (x) { case 1: \"one\"; default: \"other\" }",
            "switch (x) {\n\tcase 1: \"one\"\n\tdefault: \"other\"\n}",
            |p| p.eat_match(),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_switch_with_block() {
        assert_format!(
            "switch (value) { case 1: { const x = 1; } }",
            "switch (value) {\n\tcase 1: {\n\t\tconst x = 1;\n\t}\n}",
            |p| p.eat_match(),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_switch_case_implicit_block_without_extra_braces() {
        assert_format!(
            "switch (state) { case \"ready\": start() // ready-tail\nbreak\n default: stop() // default-tail\n }",
            "switch (state) {\n\tcase \"ready\":\n\t\tstart(); // ready-tail\n\t\tbreak\n\tdefault:\n\t\tstop() // default-tail\n}",
            |p| p.eat_match(),
            DestackFormatOptions::default_tab()
        );
    }
}
