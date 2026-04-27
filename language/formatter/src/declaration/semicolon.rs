use destack_ast::{
    Comment, Declaration, DependencyItem, DependencyMode, Expression, FunctionKind, IfKind,
    LocalNodeId, WhileKind,
};
use destack_core::StringId;
use destack_fir::format::{Buffer, FormatResult, hard_line_break};
use destack_fir::prelude::{block_indent, empty_line, format_with, line_suffix, space, token};
use destack_fir::write;

use crate::annotation::format_comment;
use crate::chain::expression_trivia_anchor_end;
use crate::declaration::dependency::import_source_is_reference_directive;
use crate::file::node_has_trailing_ignore_directive;
use crate::{DestackFormatContext, DestackFormatter};
use destack_source::Span;

/// Return same-line trailing comments that follow one statement terminator anchor.
fn statement_terminator_comments_after(
    context: &DestackFormatContext<'_>,
    mut anchor_end: u32,
) -> Vec<Comment> {
    let comments = context.comments().comments_after(anchor_end);
    let source = context.source_text();

    for (index, comment) in comments.iter().copied().enumerate() {
        if comment.preceded_by_newline() {
            let gap = Span::new(comment.span.file, anchor_end, comment.span.start);
            let is_eof_trailing_comment = !context.has_blank_line(gap)
                && source.all_bytes_match(anchor_end, comment.span.start, |byte| {
                    byte.is_ascii_whitespace() || matches!(byte, b')' | b';')
                })
                && source.all_bytes_match(comment.span.end, source.len() as u32, |byte| {
                    byte.is_ascii_whitespace()
                });

            if is_eof_trailing_comment {
                return comments[..=index].to_vec();
            }

            break;
        }

        if source.all_bytes_match(anchor_end, comment.span.start, |byte| {
            matches!(byte, b'\t' | b' ' | b')' | b';')
        }) {
            if comment.is_line() || comment.followed_by_newline() {
                return comments[..=index].to_vec();
            }

            anchor_end = comment.span.end;
            continue;
        }

        break;
    }

    Vec::new()
}

/// Return trailing statement comments with one explicit following sibling start.
fn statement_terminator_comments_between(
    context: &DestackFormatContext<'_>,
    anchor_end: u32,
    following_span_start: u32,
    allow_own_line_comments: bool,
) -> Vec<Comment> {
    let comments = context.comments();
    let comments_before_following = comments.comments_before(following_span_start);
    let mut cursor = anchor_end;
    let mut collected = Vec::new();

    for comment in comments_before_following.iter().copied() {
        if comment.span.start < anchor_end {
            continue;
        }

        if comment.span.end > following_span_start {
            break;
        }

        // blank line comments belong to the following statement
        if cursor < comment.span.start
            && context.has_blank_line(Span::new(comment.span.file, cursor, comment.span.start))
        {
            break;
        }

        if comment.preceded_by_newline() && !allow_own_line_comments {
            break;
        }

        if !context
            .source_text()
            .all_bytes_match(cursor, comment.span.start, |byte| {
                byte.is_ascii_whitespace() || matches!(byte, b')' | b';')
            })
        {
            break;
        }

        cursor = comment.span.end;
        collected.push(comment);

        if comment.is_line() || comment.followed_by_newline() {
            break;
        }
    }

    collected
}

/// Return whether one variable declaration owns comments before its source semicolon.
fn variable_statement_has_delayed_semicolon_comments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    following_span_start: u32,
) -> bool {
    let Expression::Let { declarators, .. } = context.tree.get(expression_id) else {
        return false;
    };
    let [declarator_id] = declarators.as_slice() else {
        return false;
    };
    if context.tree.get(*declarator_id).value.is_some() {
        return false;
    }

    let declarator_span = context.span(*declarator_id);
    let comments = context
        .comments()
        .comments_in_range(declarator_span.end, following_span_start);
    let Some(first_comment) = comments.first() else {
        return false;
    };

    context
        .source_text()
        .bytes_contain(first_comment.span.end, following_span_start, b';')
}

/// Write one statement terminator after one explicit source anchor.
pub(crate) fn write_statement_terminator_after_anchor<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    anchor_end: u32,
) -> FormatResult<()> {
    write!(f, [token(";")])?;

    let comments = statement_terminator_comments_after(f.context(), anchor_end);
    if comments.is_empty() {
        return Ok(());
    }

    write_statement_terminator_comments(f, anchor_end, &comments, false)
}

/// Write one statement terminator with one explicit following sibling start.
pub(crate) fn write_statement_terminator_with_following_start<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    anchor_end: u32,
    following_span_start: u32,
) -> FormatResult<()> {
    write!(f, [token(";")])?;

    let allow_own_line_comments = variable_statement_has_delayed_semicolon_comments(
        f.context(),
        expression_id,
        following_span_start,
    );
    let comments = statement_terminator_comments_between(
        f.context(),
        anchor_end,
        following_span_start,
        allow_own_line_comments,
    );
    if comments.is_empty() {
        return Ok(());
    }

    write_statement_terminator_comments(f, anchor_end, &comments, true)
}

/// Write statement separator comments after one statement terminator.
fn write_statement_terminator_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    anchor_end: u32,
    comments: &[Comment],
    indent_own_line: bool,
) -> FormatResult<()> {
    if comments.is_empty() {
        return Ok(());
    }

    let first_comment_span = comments[0].span;
    let comment_is_on_own_line = first_comment_span.start > anchor_end
        && f.context().has_newline(Span::new(
            first_comment_span.file,
            anchor_end,
            first_comment_span.start,
        ));

    // statement-separator own-line comments should stay in the statement flow,
    // not in the generic trailing line-suffix path
    if comment_is_on_own_line {
        let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            for (index, comment) in comments.iter().copied().enumerate() {
                format_comment(f, comment)?;

                if index + 1 < comments.len() {
                    write!(f, [hard_line_break()])?;
                }
            }

            Ok(())
        });

        if indent_own_line {
            return write!(f, [block_indent(&content)]);
        }

        return write!(f, [hard_line_break(), content]);
    }

    write_inline_statement_terminator_comments(f, comments)
}

/// Write same-line statement terminator comments.
fn write_inline_statement_terminator_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[Comment],
) -> FormatResult<()> {
    let source = f.context().source_text();
    let mut total_lines_before = 0usize;
    let mut previous_comment = None;

    for comment in comments.iter().copied() {
        let lines_before = {
            let comment_cursor = f.context().comments();
            source.get_lines_before(comment.span, comment_cursor)
        };
        total_lines_before += lines_before;

        if total_lines_before > 0 || previous_comment.is_some_and(Comment::is_line) {
            write!(
                f,
                [line_suffix(&format_with(
                    move |f: &mut DestackFormatter<'ast, '_>| {
                        match lines_before {
                            0 => {
                                if previous_comment.is_some_and(Comment::is_line) {
                                    write!(f, [hard_line_break()])?;
                                } else {
                                    write!(f, [space()])?;
                                }
                            }
                            1 => {
                                write!(f, [hard_line_break()])?;
                            }
                            _ => {
                                write!(f, [empty_line()])?;
                            }
                        }

                        format_comment(f, comment)
                    }
                ))]
            )?;
        } else {
            let content = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                write!(f, [space()])?;
                format_comment(f, comment)
            });

            if comment.is_line() {
                write!(f, [line_suffix(&content)])?;
            } else {
                write!(f, [content])?;
            }
        }

        previous_comment = Some(comment);
    }

    Ok(())
}

/// Return whether one export expression still needs the outer statement terminator.
fn export_expression_needs_statement_terminator(
    context: &DestackFormatContext<'_>,
    items: &[LocalNodeId<DependencyItem>],
    target: Option<StringId>,
) -> bool {
    let first_item = items.first().map(|item_id| context.tree.get(*item_id));

    let writes_own_terminator = items.len() == 1
        && first_item.is_some_and(|item| match item {
            DependencyItem::Item { mode, value, .. } => {
                (*mode == DependencyMode::Default && value.is_some())
                    || (*mode == DependencyMode::Namespace && value.is_some() && target.is_none())
            }
            DependencyItem::Error => false,
        });

    !writes_own_terminator
}

/// Return whether one block expression needs a trailing statement terminator.
pub(crate) fn expression_needs_statement_terminator(
    context: &DestackFormatContext<'_>,
    expression: &Expression,
    is_expression_context_tail: bool,
) -> bool {
    if let Expression::Import { source, .. } = expression
        && import_source_is_reference_directive(*source)
    {
        return false;
    }

    let always_needs_statement_terminator = matches!(
        expression,
        Expression::Import { .. }
            | Expression::ExportNamespace { .. }
            | Expression::Let { .. }
            | Expression::LetElse { .. }
            | Expression::Using { .. }
    ) || matches!(
        expression,
        Expression::If {
            kind: IfKind::Ternary,
            ..
        }
    ) || matches!(
        expression,
        Expression::While {
            kind: WhileKind::DoWhile,
            ..
        }
    ) || matches!(
        expression,
        Expression::Declaration(declaration_id)
            if matches!(
                context.tree.get(*declaration_id),
                Declaration::Function(function)
                    if function.name.is_none() && function.signature.kind == FunctionKind::Lambda
            )
    ) || matches!(
        expression,
        Expression::Break { .. }
            | Expression::Continue { .. }
            | Expression::Yield { .. }
            | Expression::Return { .. }
            | Expression::Throw { .. }
            | Expression::Debugger
    );

    if always_needs_statement_terminator {
        return true;
    }

    if let Expression::Export { items, target, .. } = expression {
        return export_expression_needs_statement_terminator(context, items, *target);
    }

    if is_expression_context_tail {
        return false;
    }

    !matches!(expression, Expression::Stub | Expression::Error)
        && !expression.ends_statement_on_newline()
}

/// Return whether one statement wrapper should keep its trailing semicolon.
pub(crate) fn statement_wrapper_needs_semicolon(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression = context.tree.get(expression_id);
    if matches!(
        expression,
        Expression::Declaration(_) | Expression::Block(_)
    ) {
        return false;
    }

    if let Expression::Try {
        catch_expression,
        catch_pattern,
        finally_expression,
        ..
    } = expression
        && (catch_expression.is_some() || catch_pattern.is_some() || finally_expression.is_some())
    {
        return false;
    }

    if matches!(
        expression,
        Expression::If {
            kind: IfKind::If,
            ..
        } | Expression::While {
            kind: WhileKind::While,
            ..
        } | Expression::ForEach { .. }
            | Expression::For { .. }
            | Expression::Loop { .. }
            | Expression::Match { .. }
            | Expression::Labelled { .. }
    ) {
        return false;
    }

    if matches!(expression, Expression::Stub | Expression::Error) {
        return false;
    }

    true
}

/// Return the source anchor where same-line statement trailing comments begin.
pub(crate) fn statement_trailing_comment_anchor_end(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> u32 {
    let expression = context.tree.get(expression_id);

    match expression {
        Expression::Return {
            value: Some(value_id),
        }
        | Expression::Yield {
            value: Some(value_id),
            ..
        }
        | Expression::Break {
            value: Some(value_id),
            ..
        } => expression_trivia_anchor_end(context, *value_id),
        Expression::Throw { value } => expression_trivia_anchor_end(context, *value),
        _ => expression_trivia_anchor_end(context, expression_id),
    }
}

/// Write one statement terminator and its same-line trailing comments.
pub(crate) fn write_statement_terminator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if node_has_trailing_ignore_directive(f.context(), expression_id) {
        return Ok(());
    }

    let anchor_end = statement_trailing_comment_anchor_end(f.context(), expression_id);
    write_statement_terminator_after_anchor(f, anchor_end)
}
