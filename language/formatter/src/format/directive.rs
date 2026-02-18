use std::borrow::Cow;
use std::collections::HashMap;

use destack_ast::{LocalNodeId, Node, NodeTree, NodeTreeImpl, TokenSpan, TokenType};
use destack_fir::format::{FormatResult, text};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;

use crate::{DestackFormatContext, DestackFormatter};

/// The formatter directives supported via comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatterDirectiveKind {
    /// Ignore formatting for the next node.
    IgnoreFormat,
}

/// The position of a formatter directive relative to a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatterDirectivePosition {
    /// The directive appears before the node.
    Prefix { comment_span: Span },
    /// The directive appears after the node.
    Postfix { comment_span: Span },
}

/// A formatter directive attached to a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatterDirective {
    /// The directive kind.
    pub kind: FormatterDirectiveKind,
    /// The position of the directive.
    pub position: FormatterDirectivePosition,
}

/// The directive tokens parsed from comment text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormatterDirectiveToken {
    /// Ignore formatting for the next node.
    Ignore,
    /// Ignore formatting for the current file.
    IgnoreFile,
    /// Begin ignoring formatting until the matching end token.
    IgnoreStart,
    /// End the current ignore range.
    IgnoreEnd,
}

/// Resolve the formatter directive for a node, if any.
pub fn directive_for_node<T: Node + Clone>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> Option<FormatterDirective>
where
    NodeTree: NodeTreeImpl<T>,
{
    if !context.has_ignore_directive_markers() {
        return None;
    }

    let node_span = context.span(node_id);

    let comment_tokens = context.comment_tokens();
    let mut last_prefix_token: Option<TokenSpan> = None;
    for token in comment_tokens {
        if token.span.start <= node_span.start {
            last_prefix_token = Some(*token);
        } else {
            break;
        }
    }

    if let Some(token) = last_prefix_token {
        if !comment_token_is_line_leading(context, token) {
            return None;
        }

        let raw = context.token_str(token);
        let (between, newlines) = if token.span.end > node_span.start {
            ("", 0)
        } else {
            let between_span = Span::new(node_span.file, token.span.end, node_span.start);
            let between = context.span_str(between_span);
            let newlines = between.chars().filter(|ch| *ch == '\n').count();
            (between, newlines)
        };
        if between.trim().is_empty() && newlines <= 1 {
            let directive_token = parse_directive_token_from_raw(raw);
            let kind = match directive_token {
                Some(FormatterDirectiveToken::Ignore | FormatterDirectiveToken::IgnoreStart) => {
                    Some(FormatterDirectiveKind::IgnoreFormat)
                }
                _ => None,
            };
            if let Some(kind) = kind {
                return Some(FormatterDirective {
                    kind,
                    position: FormatterDirectivePosition::Prefix {
                        comment_span: token.span,
                    },
                });
            }
        }
    }

    None
}

/// Resolve an ignore range directive for a node.
pub fn ignore_range_for_node<T: Node + Clone>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    comment_tokens: &[TokenSpan],
) -> Option<Span>
where
    NodeTree: NodeTreeImpl<T>,
{
    if !context.has_ignore_directive_markers() {
        return None;
    }

    let node_span = context.span(node_id);

    let mut last_prefix_token: Option<TokenSpan> = None;
    for token in comment_tokens {
        if token.span.start <= node_span.start {
            last_prefix_token = Some(*token);
        } else {
            break;
        }
    }

    if let Some(token) = last_prefix_token {
        if !comment_token_is_line_leading(context, token) {
            return None;
        }

        let raw = context.token_str(token);
        let comment_line = context
            .file
            .get_position(token.span.start)
            .map(|(line, _)| line);
        let node_line = context
            .file
            .get_position(node_span.start)
            .map(|(line, _)| line);
        let is_adjacent = comment_line
            .zip(node_line)
            .is_some_and(|(comment_line, node_line)| node_line == comment_line + 1);
        if is_adjacent {
            match parse_directive_token_from_raw(raw) {
                Some(FormatterDirectiveToken::Ignore) => {
                    let range_span = Span::new(node_span.file, token.span.start, node_span.end);
                    return Some(extend_span_with_trailing_tokens(context, range_span));
                }
                Some(FormatterDirectiveToken::IgnoreStart) => {
                    let end_span = find_ignore_range_end(context, comment_tokens, token.span.end)?;
                    return Some(Span::new(token.span.file, token.span.start, end_span.start));
                }
                _ => {}
            }
        }
    }

    None
}

/// Collect ignore ranges for a list of nodes keyed by node id.
pub fn collect_ignore_ranges_for_nodes<T: Node + Clone>(
    context: &DestackFormatContext<'_>,
    node_ids: &[LocalNodeId<T>],
    comment_tokens: &[TokenSpan],
) -> HashMap<u32, Span>
where
    NodeTree: NodeTreeImpl<T>,
{
    let mut ignore_ranges = HashMap::new();
    for node_id in node_ids.iter().copied() {
        if let Some(range_span) = ignore_range_for_node(context, node_id, comment_tokens) {
            ignore_ranges.insert(node_id.id, range_span);
        }
    }
    ignore_ranges
}

/// Return whether any node in a list has an ignore range.
pub fn any_ignore_range_for_nodes<T: Node + Clone>(
    context: &DestackFormatContext<'_>,
    node_ids: &[LocalNodeId<T>],
    comment_tokens: &[TokenSpan],
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    node_ids
        .iter()
        .copied()
        .any(|node_id| ignore_range_for_node(context, node_id, comment_tokens).is_some())
}

/// Return whether a comment token starts at the first non-whitespace position on its line.
fn comment_token_is_line_leading(context: &DestackFormatContext<'_>, token: TokenSpan) -> bool {
    let Some((line_index, _)) = context.file.get_position(token.span.start) else {
        return false;
    };
    let Some(line_span) = context.file.get_line_span(line_index) else {
        return false;
    };

    let prefix_span = Span::new(token.span.file, line_span.start, token.span.start);
    context.span_str(prefix_span).trim().is_empty()
}

/// Extend an ignored span to include trailing content on the same line.
fn extend_span_with_trailing_tokens(context: &DestackFormatContext<'_>, span: Span) -> Span {
    let mut tokens: Vec<TokenSpan> = context
        .tokens
        .iter()
        .copied()
        .chain(context.side_tokens.iter().copied())
        .collect();
    tokens.sort_by_key(|token| token.span.start);

    let mut end = span.end;
    for token in tokens.iter().copied() {
        if token.span.start < span.end {
            continue;
        }

        match token.token.ty {
            TokenType::Whitespace => {
                let raw = context.token_str(token);
                if raw.contains(['\n', '\r']) {
                    break;
                }
                end = token.span.end;
                continue;
            }
            TokenType::Newline => {
                break;
            }
            _ => {
                end = token.span.end;
            }
        }
    }

    let extended = if end > span.end {
        Span::new(span.file, span.start, end)
    } else {
        span
    };

    extend_span_with_trailing_statement_terminator(context, &tokens, extended)
}

/// Extend an ignored span to include a standalone trailing statement terminator.
fn extend_span_with_trailing_statement_terminator(
    context: &DestackFormatContext<'_>,
    tokens: &[TokenSpan],
    span: Span,
) -> Span {
    let mut token_index = tokens.partition_point(|token| token.span.start < span.end);

    // skip pure whitespace before the next meaningful token
    while let Some(token) = tokens.get(token_index).copied() {
        if matches!(token.token.ty, TokenType::Whitespace | TokenType::Newline) {
            token_index += 1;
            continue;
        }

        break;
    }

    let Some(candidate) = tokens.get(token_index).copied() else {
        return span;
    };
    if candidate.token.ty != TokenType::Semicolon {
        return span;
    }

    let mut lookahead_index = token_index + 1;

    // accept only standalone semicolons: no trailing code on the same line
    while let Some(token) = tokens.get(lookahead_index).copied() {
        match token.token.ty {
            TokenType::Whitespace => {
                let raw = context.token_str(token);
                if raw.contains(['\n', '\r']) {
                    return Span::new(span.file, span.start, candidate.span.end);
                }
            }
            TokenType::Newline | TokenType::End => {
                return Span::new(span.file, span.start, candidate.span.end);
            }
            _ => {
                return span;
            }
        }

        lookahead_index += 1;
    }

    Span::new(span.file, span.start, candidate.span.end)
}

/// Collect comment tokens sorted by source position.
pub fn collect_comment_tokens(context: &DestackFormatContext<'_>) -> Vec<TokenSpan> {
    context.comment_tokens().to_vec()
}

/// Return whether this file has a formatter ignore-file directive comment.
pub fn has_file_ignore_directive(context: &DestackFormatContext<'_>) -> bool {
    if !context.has_ignore_directive_markers() {
        return false;
    }

    let mut tokens = context
        .tokens
        .iter()
        .copied()
        .chain(context.side_tokens.iter().copied())
        .collect::<Vec<_>>();
    tokens.sort_by_key(|token| token.span.start);

    for token in tokens {
        match token.token.ty {
            TokenType::Whitespace | TokenType::Newline => {
                continue;
            }
            TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment => {
                let raw_comment = context.token_str(token);
                if matches!(
                    parse_directive_token_from_raw(raw_comment),
                    Some(FormatterDirectiveToken::IgnoreFile)
                ) {
                    return true;
                }
                continue;
            }
            _ => return false,
        }
    }

    false
}

/// Extract the source for an ignored span.
pub fn ignored_span_source(context: &DestackFormatContext<'_>, span: Span) -> String {
    let raw = context.span_str(span);
    let Some((line_index, column)) = context.file.get_position(span.start) else {
        return raw.to_owned();
    };
    let Some(line_span) = context.file.get_line_span(line_index) else {
        return raw.to_owned();
    };
    let line_str = context.file.get_span_str(line_span).unwrap_or_default();
    let Some(prefix) = line_str.get(..column as usize) else {
        return raw.to_owned();
    };
    if prefix.is_empty() || !prefix.trim().is_empty() {
        return raw.to_owned();
    }

    raw.split('\n')
        .map(|line| line.strip_prefix(prefix).unwrap_or(line).to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Write a raw ignored span with formatter-managed indentation.
pub fn write_ignored_span<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    span: Span,
) -> FormatResult<()> {
    let raw = ignored_span_source(f.context(), span);
    let raw = dedent_common_leading_whitespace(raw.as_str());

    let mut lines: Vec<&str> = raw.split('\n').collect();
    if raw.ends_with('\n') {
        lines.pop();
    }
    let mut lines = lines.into_iter();
    if let Some(first) = lines.next() {
        write!(f, [text(first)])?;
    }
    for line in lines {
        write!(f, [hard_line_break(), text(line)])?;
    }

    Ok(())
}

/// Remove shared leading indentation from non-empty lines.
fn dedent_common_leading_whitespace(raw: &str) -> String {
    // collect all non-empty lines that contribute indentation
    let lines = raw.lines().collect::<Vec<_>>();
    let mut common_prefix: Option<&str> = None;
    for line in &lines {
        if line.trim().is_empty() {
            continue;
        }

        let prefix_end = line
            .char_indices()
            .find_map(|(index, character)| {
                if character == ' ' || character == '\t' {
                    None
                } else {
                    Some(index)
                }
            })
            .unwrap_or(line.len());
        let prefix = &line[..prefix_end];

        match common_prefix {
            None => common_prefix = Some(prefix),
            Some(current_prefix) => {
                let mut shared_len = 0usize;
                let mut current_iter = current_prefix.chars();
                let mut next_iter = prefix.chars();
                loop {
                    let Some(current_character) = current_iter.next() else {
                        break;
                    };
                    let Some(next_character) = next_iter.next() else {
                        break;
                    };
                    if current_character != next_character {
                        break;
                    }
                    shared_len += current_character.len_utf8();
                }
                common_prefix = Some(&current_prefix[..shared_len]);
            }
        }
    }

    let Some(common_prefix) = common_prefix else {
        return raw.to_owned();
    };
    if common_prefix.is_empty() {
        return raw.to_owned();
    }

    lines
        .into_iter()
        .map(|line| line.strip_prefix(common_prefix).unwrap_or(line).to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract the source for an ignored node.
pub fn ignored_node_source<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    directive: FormatterDirective,
) -> String
where
    NodeTree: NodeTreeImpl<T>,
{
    let span = context.span(node_id);
    let end = match directive.position {
        FormatterDirectivePosition::Prefix { .. } => span.end,
        FormatterDirectivePosition::Postfix { comment_span } => comment_span.end.max(span.end),
    };
    context
        .span_str(Span::new(span.file, span.start, end))
        .to_owned()
}

/// Find the matching ignore range end comment following a start offset.
fn find_ignore_range_end(
    context: &DestackFormatContext<'_>,
    comment_tokens: &[TokenSpan],
    start_offset: u32,
) -> Option<Span> {
    comment_tokens
        .iter()
        .filter(|token| token.span.start >= start_offset)
        .find_map(|token| {
            let raw_comment = context.token_str(*token);
            if parse_directive_token_from_raw(raw_comment)
                == Some(FormatterDirectiveToken::IgnoreEnd)
            {
                Some(token.span)
            } else {
                None
            }
        })
}

/// Parse a directive token from a raw comment string (including markers).
fn parse_directive_token_from_raw(raw: &str) -> Option<FormatterDirectiveToken> {
    let content = strip_comment_markers(raw);
    parse_directive_token(content.as_ref())
}

/// Return whether raw comment text is an ignore directive.
pub(crate) fn is_ignore_directive_comment(raw: &str) -> bool {
    matches!(
        parse_directive_token_from_raw(raw),
        Some(FormatterDirectiveToken::Ignore | FormatterDirectiveToken::IgnoreStart)
    )
}

/// Return whether raw comment text is any ignore directive token.
pub(crate) fn is_any_ignore_directive_comment(raw: &str) -> bool {
    matches!(
        parse_directive_token_from_raw(raw),
        Some(
            FormatterDirectiveToken::Ignore
                | FormatterDirectiveToken::IgnoreFile
                | FormatterDirectiveToken::IgnoreStart
                | FormatterDirectiveToken::IgnoreEnd
        )
    )
}

/// Strip comment markers from a raw comment string.
fn strip_comment_markers(raw: &str) -> Cow<'_, str> {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("//") {
        return Cow::Owned(rest.trim_start_matches('/').trim().to_owned());
    }
    if let Some(rest) = trimmed.strip_prefix("/*") {
        let rest = rest.strip_suffix("*/").unwrap_or(rest);
        return Cow::Owned(rest.trim().trim_start_matches('*').trim().to_owned());
    }
    Cow::Borrowed(trimmed)
}

/// Parse a directive token from comment content.
fn parse_directive_token(comment: &str) -> Option<FormatterDirectiveToken> {
    comment.lines().find_map(|line| {
        let trimmed = line.trim().trim_start_matches('*').trim();
        if trimmed.is_empty() {
            return None;
        }

        // check for single line ignore directives
        if matches!(trimmed, "prettier-ignore" | "oxfmt-ignore") {
            return Some(FormatterDirectiveToken::Ignore);
        }

        // check for file-level ignore directives
        if matches!(trimmed, "prettier-ignore-file" | "oxfmt-ignore-file") {
            return Some(FormatterDirectiveToken::IgnoreFile);
        }

        // check for range start directives
        if matches!(trimmed, "oxfmt-ignore-start") {
            return Some(FormatterDirectiveToken::IgnoreStart);
        }

        // check for range end directives
        if matches!(trimmed, "oxfmt-ignore-end") {
            return Some(FormatterDirectiveToken::IgnoreEnd);
        }

        None
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_ast::{Expression, NodeParentIndex};
    use destack_parser::Parser;
    use destack_source::{File, FileId, FileType, LanguageType, Uri};
    use destack_workspace::FormatterOptions;

    use super::{collect_comment_tokens, ignore_range_for_node, ignored_span_source};
    use crate::{DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions};

    #[test]
    fn test_format_ignore_range_for_statement() {
        let source = "// prettier-ignore\ncall(   a, b)";
        let file = Arc::new(File::from_text(
            FileId::new(0),
            "main.ts".to_string(),
            Uri::from_string("file://main.ts"),
            None,
            FileType::TypeScript,
            source.to_string(),
        ));

        let mut parser = Parser::lex_file(file.clone(), LanguageType::TypeScript);
        let expressions = parser.parse();

        let side_span = parser.compute_side_span();
        let (tokens, side_tokens) = parser.take_tokens();
        let strings = parser.strings.into_immutable();
        let parents = NodeParentIndex::from_tree(&parser.tree);
        let options = DestackFormatOptions::from_formatter_options(
            FormatterOptions::default(),
            LanguageType::TypeScript,
        );
        let context = DestackFormatContext::new(
            options,
            DestackFormatArtifacts {
                file: &file,
                tree: &parser.tree,
                tokens: &tokens,
                side_tokens: &side_tokens,
                side_span: &side_span,
                strings: &strings,
                parents,
            },
        );

        let comment_tokens = collect_comment_tokens(&context);
        assert!(!comment_tokens.is_empty());
        let range = ignore_range_for_node(&context, expressions[0], &comment_tokens);
        assert!(range.is_some());
    }

    #[test]
    fn test_ignore_range_for_call_arguments_uses_comment_column_start() {
        let source = r#"doThing(
    1,
    // oxfmt-ignore-start
    foo ( 1 ,2 ),
    bar(3),
    // oxfmt-ignore-end
    4,
)"#;
        let file = Arc::new(File::from_text(
            FileId::new(0),
            "main.ts".to_string(),
            Uri::from_string("file://main.ts"),
            None,
            FileType::TypeScript,
            source.to_string(),
        ));

        let mut parser = Parser::lex_file(file.clone(), LanguageType::TypeScript);
        let expressions = parser.parse();
        assert_eq!(expressions.len(), 1);

        let second_argument = match parser.tree.get(expressions[0]) {
            Expression::Statement(call_expression_id) => match parser.tree.get(*call_expression_id)
            {
                Expression::Call {
                    dynamic_arguments, ..
                } => dynamic_arguments[1],
                _ => panic!("expected call expression"),
            },
            _ => panic!("expected statement expression"),
        };

        let side_span = parser.compute_side_span();
        let (tokens, side_tokens) = parser.take_tokens();
        let strings = parser.strings.into_immutable();
        let parents = NodeParentIndex::from_tree(&parser.tree);
        let options = DestackFormatOptions::from_formatter_options(
            FormatterOptions::default(),
            LanguageType::TypeScript,
        );
        let context = DestackFormatContext::new(
            options,
            DestackFormatArtifacts {
                file: &file,
                tree: &parser.tree,
                tokens: &tokens,
                side_tokens: &side_tokens,
                side_span: &side_span,
                strings: &strings,
                parents,
            },
        );

        let comment_tokens = collect_comment_tokens(&context);
        let range = ignore_range_for_node(&context, second_argument, &comment_tokens)
            .expect("expected ignore range for second argument");

        let (_, start_column) = context
            .file
            .get_position(range.start)
            .expect("expected range start position");
        assert_eq!(start_column, 4);

        let raw = ignored_span_source(&context, range);
        assert!(raw.starts_with("// oxfmt-ignore-start"));
    }
}
