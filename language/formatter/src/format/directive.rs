use std::borrow::Cow;
use std::collections::HashMap;

use destack_ast::{Comment, LocalNodeId, Node, NodeTree, NodeTreeImpl, TokenSpan, TokenType};
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

/// Single-line ignore directive markers supported by formatter behavior.
const IGNORE_DIRECTIVES: &[&str] = &["prettier-ignore", "oxfmt-ignore"];

/// File-level ignore directive markers supported by formatter behavior.
const IGNORE_FILE_DIRECTIVES: &[&str] = &["prettier-ignore-file", "oxfmt-ignore-file"];

/// Ignore-range start directive markers supported by formatter behavior.
const IGNORE_START_DIRECTIVES: &[&str] = &["oxfmt-ignore-start"];

/// Ignore-range end directive markers supported by formatter behavior.
const IGNORE_END_DIRECTIVES: &[&str] = &["oxfmt-ignore-end"];

/// Return whether one marker matches any directive alias.
#[inline]
fn marker_matches_any(marker: &str, markers: &[&str]) -> bool {
    markers.contains(&marker)
}

/// Parse one directive token from one comment token span.
#[inline]
fn directive_token_for_comment_token(
    ctx: &DestackFormatContext<'_>,
    token: TokenSpan,
) -> Option<FormatterDirectiveToken> {
    parse_directive_token_from_raw(ctx.token_str(token))
}

/// Return the last comment token that starts before or at one node offset.
fn last_prefix_comment_token_before(
    comment_tokens: &[TokenSpan],
    node_start: u32,
) -> Option<TokenSpan> {
    let mut last_prefix_token = None;
    for token in comment_tokens {
        if token.span.start <= node_start {
            last_prefix_token = Some(*token);
        } else {
            break;
        }
    }

    last_prefix_token
}

/// Return one line-leading comment token that prefixes one node span.
fn prefix_comment_token_for_node(
    ctx: &DestackFormatContext<'_>,
    node_span: Span,
    comment_tokens: &[TokenSpan],
) -> Option<TokenSpan> {
    let token = last_prefix_comment_token_before(comment_tokens, node_span.start)?;
    if !comment_token_is_line_leading(ctx, token) {
        return None;
    }

    Some(token)
}

/// Return one formatter directive kind for one parsed directive token.
fn directive_kind_for_token(token: FormatterDirectiveToken) -> Option<FormatterDirectiveKind> {
    match token {
        FormatterDirectiveToken::Ignore | FormatterDirectiveToken::IgnoreStart => {
            Some(FormatterDirectiveKind::IgnoreFormat)
        }
        FormatterDirectiveToken::IgnoreFile | FormatterDirectiveToken::IgnoreEnd => None,
    }
}

/// Resolve the formatter directive for a node, if any.
pub fn directive_for_node<T: Node + Clone>(
    ctx: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> Option<FormatterDirective>
where
    NodeTree: NodeTreeImpl<T>,
{
    if !ctx.has_ignore_directive_markers() {
        return None;
    }

    let node_span = ctx.span(node_id);
    let comment_tokens = ctx.comment_tokens();
    let token = prefix_comment_token_for_node(ctx, node_span, comment_tokens)?;
    let (between_is_whitespace_only, line_distance) = if token.span.end > node_span.start {
        (true, 0)
    } else {
        let between_is_whitespace_only = token
            .span
            .gap_to(node_span)
            .is_none_or(|between_span| !ctx.has_non_whitespace_content(between_span));
        let line_distance = ctx
            .source_line_distance(token.span.end, node_span.start)
            .map_or(2, |distance| distance as usize);
        (between_is_whitespace_only, line_distance)
    };
    if !between_is_whitespace_only || line_distance > 1 {
        return None;
    }

    let directive_token = directive_token_for_comment_token(ctx, token)?;
    let kind = directive_kind_for_token(directive_token)?;
    Some(FormatterDirective {
        kind,
        position: FormatterDirectivePosition::Prefix {
            comment_span: token.span,
        },
    })
}

/// Resolve an ignore range directive for a node.
pub fn ignore_range_for_node<T: Node + Clone>(
    ctx: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    comment_tokens: &[TokenSpan],
) -> Option<Span>
where
    NodeTree: NodeTreeImpl<T>,
{
    if !ctx.has_ignore_directive_markers() {
        return None;
    }

    let node_span = ctx.span(node_id);
    let token = prefix_comment_token_for_node(ctx, node_span, comment_tokens)?;

    let is_adjacent = ctx.source_line_distance(token.span.start, node_span.start) == Some(1);
    if !is_adjacent {
        return None;
    }

    match directive_token_for_comment_token(ctx, token) {
        Some(FormatterDirectiveToken::Ignore) => {
            let range_span = Span::new(node_span.file, token.span.start, node_span.end);
            Some(ctx.extend_span_with_trailing_line_tokens(range_span))
        }
        Some(FormatterDirectiveToken::IgnoreStart) => {
            let end_span = find_ignore_range_end(ctx, comment_tokens, token.span.end)?;
            Some(Span::new(token.span.file, token.span.start, end_span.start))
        }
        Some(FormatterDirectiveToken::IgnoreFile | FormatterDirectiveToken::IgnoreEnd) | None => {
            None
        }
    }
}

/// Collect ignore ranges for a list of nodes keyed by node id.
pub fn ignore_ranges_for_nodes<T: Node + Clone>(
    ctx: &DestackFormatContext<'_>,
    node_ids: &[LocalNodeId<T>],
    comment_tokens: &[TokenSpan],
) -> HashMap<u32, Span>
where
    NodeTree: NodeTreeImpl<T>,
{
    let mut ignore_ranges = HashMap::new();
    for node_id in node_ids.iter().copied() {
        if let Some(range_span) = ignore_range_for_node(ctx, node_id, comment_tokens) {
            ignore_ranges.insert(node_id.id, range_span);
        }
    }
    ignore_ranges
}

/// Return whether any node in a list has an ignore range.
pub fn any_ignore_range_for_nodes<T: Node + Clone>(
    ctx: &DestackFormatContext<'_>,
    node_ids: &[LocalNodeId<T>],
    comment_tokens: &[TokenSpan],
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    node_ids
        .iter()
        .copied()
        .any(|node_id| ignore_range_for_node(ctx, node_id, comment_tokens).is_some())
}

/// Return whether a comment token starts at the first non-whitespace position on its line.
fn comment_token_is_line_leading(ctx: &DestackFormatContext<'_>, token: TokenSpan) -> bool {
    ctx.line_prefix_is_whitespace(token.span.start)
}

/// Collect comment tokens sorted by source position.
pub fn comment_tokens(ctx: &DestackFormatContext<'_>) -> Vec<TokenSpan> {
    ctx.comment_tokens().to_vec()
}

/// Return whether this file has a formatter ignore-file directive comment.
pub fn has_file_ignore_directive(ctx: &DestackFormatContext<'_>) -> bool {
    if !ctx.has_ignore_directive_markers() {
        return false;
    }

    let tokens = ctx.source_tokens_sorted();

    for token in tokens {
        match token.token.ty {
            TokenType::Whitespace | TokenType::Newline => {
                continue;
            }
            TokenType::LineComment
            | TokenType::BlockComment
            | TokenType::DocLineComment
            | TokenType::DocBlockComment => {
                if matches!(
                    directive_token_for_comment_token(ctx, token),
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
pub fn ignored_span_source(ctx: &DestackFormatContext<'_>, span: Span) -> String {
    let raw = ctx.span_str(span);
    if ctx.source_position(span.start).is_none() {
        return raw.to_owned();
    }
    let Some(prefix) = ctx.line_prefix_text(span.start) else {
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
    let raw = if !f.context().span_starts_on_own_line(span) {
        dedent_common_leading_whitespace_after_first_line(raw.as_str())
    } else {
        dedent_common_leading_whitespace(raw.as_str())
    };
    let raw = if raw.trim().is_empty() {
        raw.chars()
            .filter(|character| *character == '\n')
            .collect::<String>()
    } else {
        raw
    };

    let mut segment_start = 0usize;
    while segment_start < raw.len() {
        let Some(relative_newline_index) = raw[segment_start..].find('\n') else {
            write!(f, [text(&raw[segment_start..])])?;
            break;
        };

        let newline_index = segment_start + relative_newline_index;
        if segment_start < newline_index {
            write!(f, [text(&raw[segment_start..newline_index])])?;
        }

        let mut newline_run_end = newline_index;
        let raw_bytes = raw.as_bytes();
        while newline_run_end < raw_bytes.len() && raw_bytes[newline_run_end] == b'\n' {
            newline_run_end += 1;
        }

        let mut newline_count = newline_run_end - newline_index;
        while newline_count >= 2 {
            write!(f, [empty_line()])?;
            newline_count -= 2;
        }
        if newline_count == 1 {
            write!(f, [hard_line_break()])?;
        }

        segment_start = newline_run_end;
    }

    Ok(())
}

/// Write one ignored node source range with formatter-managed indentation.
pub fn write_ignored_node<'ast, T: Node>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    directive: FormatterDirective,
) -> FormatResult<()>
where
    NodeTree: NodeTreeImpl<T>,
{
    let span = ignored_node_span(f.context(), node_id, directive);
    write_ignored_span(f, span)
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

/// Remove shared leading indentation from non-empty lines after the first line.
fn dedent_common_leading_whitespace_after_first_line(raw: &str) -> String {
    let lines = raw.lines().collect::<Vec<_>>();
    if lines.len() <= 1 {
        return raw.to_owned();
    }

    let mut common_prefix: Option<&str> = None;
    for line in lines.iter().skip(1) {
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

    let mut normalized_lines = Vec::with_capacity(lines.len());
    normalized_lines.push(lines[0].to_owned());
    normalized_lines.extend(
        lines
            .iter()
            .skip(1)
            .map(|line| line.strip_prefix(common_prefix).unwrap_or(line).to_owned()),
    );
    normalized_lines.join("\n")
}

/// Return the source range that should be preserved for one ignored node.
pub fn ignored_node_span<T: Node>(
    ctx: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    directive: FormatterDirective,
) -> Span
where
    NodeTree: NodeTreeImpl<T>,
{
    let span = ctx.span(node_id);
    let (start, end) = match directive.position {
        FormatterDirectivePosition::Prefix { .. } => (span.start, span.end),
        FormatterDirectivePosition::Postfix { comment_span } => {
            (span.start, comment_span.end.max(span.end))
        }
    };

    Span::new(span.file, start, end)
}

/// Find the matching ignore range end comment following a start offset.
fn find_ignore_range_end(
    ctx: &DestackFormatContext<'_>,
    comment_tokens: &[TokenSpan],
    start_offset: u32,
) -> Option<Span> {
    comment_tokens
        .iter()
        .filter(|token| token.span.start >= start_offset)
        .find_map(|token| {
            if directive_token_for_comment_token(ctx, *token)
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

/// Return whether one comment node is an ignore directive.
pub(crate) fn comment_node_is_ignore_directive(
    ctx: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Comment>,
) -> bool {
    let comment_source = ctx.comment_text(node_id);
    is_ignore_directive_comment(comment_source.as_ref())
}

/// Return whether one comment node is any ignore directive token.
pub(crate) fn comment_node_is_any_ignore_directive(
    ctx: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Comment>,
) -> bool {
    let comment_source = ctx.comment_text(node_id);
    is_any_ignore_directive_comment(comment_source.as_ref())
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
    let mut first_significant_line = None;
    let mut has_additional_significant_line = false;
    for line in comment.lines() {
        let trimmed = line.trim().trim_start_matches('*').trim();
        if trimmed.is_empty() {
            continue;
        }
        if first_significant_line.is_none() {
            first_significant_line = Some(trimmed);
        } else {
            has_additional_significant_line = true;
            break;
        }
    }
    let first_significant_line = first_significant_line?;

    // NOTE #Architecture: behavior is keyed on concrete directive spellings
    // parser directive enums collapse aliases and do not retain which marker appeared
    if !has_additional_significant_line
        && marker_matches_any(first_significant_line, IGNORE_DIRECTIVES)
    {
        return Some(FormatterDirectiveToken::Ignore);
    }

    if !has_additional_significant_line
        && marker_matches_any(first_significant_line, IGNORE_FILE_DIRECTIVES)
    {
        return Some(FormatterDirectiveToken::IgnoreFile);
    }

    if !has_additional_significant_line
        && marker_matches_any(first_significant_line, IGNORE_START_DIRECTIVES)
    {
        return Some(FormatterDirectiveToken::IgnoreStart);
    }

    if !has_additional_significant_line
        && marker_matches_any(first_significant_line, IGNORE_END_DIRECTIVES)
    {
        return Some(FormatterDirectiveToken::IgnoreEnd);
    }

    None
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_ast::{Expression, NodeParentIndex};
    use destack_parser::Parser;
    use destack_source::{File, FileId, FileType, LanguageType, Uri};
    use destack_workspace::FormatterOptions;

    use crate::format::directive::{
        comment_tokens, dedent_common_leading_whitespace_after_first_line, directive_for_node,
        ignore_range_for_node, ignored_node_span, ignored_span_source,
    };
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
        let ctx = DestackFormatContext::new(
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

        let comment_tokens = comment_tokens(&ctx);
        assert!(!comment_tokens.is_empty());
        let range = ignore_range_for_node(&ctx, expressions[0], &comment_tokens);
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
        let ctx = DestackFormatContext::new(
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

        let comment_tokens = comment_tokens(&ctx);
        let range = ignore_range_for_node(&ctx, second_argument, &comment_tokens)
            .expect("expected ignore range for second argument");

        let (_, start_column) = ctx
            .file
            .get_position(range.start)
            .expect("expected range start position");
        assert_eq!(start_column, 4);

        let raw = ignored_span_source(&ctx, range);
        assert!(raw.starts_with("// oxfmt-ignore-start"));
    }

    #[test]
    fn test_ignored_node_span_prefix_uses_node_source_only() {
        let source = "// prettier-ignore\ncall(   a, b)\n";
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

        let statement_expression_id = expressions[0];
        let expression_id = match parser.tree.get(statement_expression_id) {
            Expression::Statement(expression_id) => *expression_id,
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
        let ctx = DestackFormatContext::new(
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

        let directive =
            directive_for_node(&ctx, expression_id).expect("expected prefix ignore directive");
        let ignored_span = ignored_node_span(&ctx, expression_id, directive);
        let raw = ignored_span_source(&ctx, ignored_span);

        assert_eq!(raw, "call(   a, b)");
    }

    #[test]
    fn test_dedent_common_leading_whitespace_after_first_line_dedents_nested_lines() {
        let raw = "{\n    [A in B]: C | D;\n  };";
        let dedented = dedent_common_leading_whitespace_after_first_line(raw);

        assert_eq!(dedented, "{\n  [A in B]: C | D;\n};");
    }
}
