use std::borrow::Cow;
use std::collections::HashMap;

use destack_ast::{LocalNodeId, Node, NodeTree, NodeTreeImpl, TokenSpan, TokenType};
use destack_fir::format::{text, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;

use crate::{DestackFormatContext, DestackFormatter};

/// Single-line ignore directive markers supported by formatter behavior.
const IGNORE_DIRECTIVES: &[&str] = &[
    "prettier-ignore",
    "oxfmt-ignore",
    "format-ignore",
    "fmt-ignore",
    "deno-fmt-ignore",
];

/// File-level ignore directive markers supported by formatter behavior.
const IGNORE_FILE_DIRECTIVES: &[&str] = &[
    "prettier-ignore-file",
    "oxfmt-ignore-file",
    "format-ignore-file",
    "fmt-ignore-file",
    "deno-fmt-ignore-file",
];

/// Ignore-range start directive markers supported by formatter behavior.
const IGNORE_START_DIRECTIVES: &[&str] = &[
    "prettier-ignore-start",
    "oxfmt-ignore-start",
    "format-ignore-start",
    "fmt-ignore-start",
];

/// Ignore-range end directive markers supported by formatter behavior.
const IGNORE_END_DIRECTIVES: &[&str] = &[
    "prettier-ignore-end",
    "oxfmt-ignore-end",
    "format-ignore-end",
    "fmt-ignore-end",
];

/// Prefix ignore directive markers supported by formatter behavior.
const IGNORE_PREFIX_DIRECTIVES: &[&str] = &["biome-ignore format"];

/// Formatter suppression directives parsed from comment text.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum IgnoreDirective {
    /// Ignore the next node.
    Ignore,
    /// Ignore the full file.
    IgnoreFile,
    /// Start one ignore range.
    IgnoreStart,
    /// End one ignore range.
    IgnoreEnd,
}

/// Return whether one marker matches any directive alias.
#[inline]
fn marker_matches_any(marker: &str, markers: &[&str]) -> bool {
    markers.contains(&marker)
}

/// Return whether one marker starts with one directive prefix.
#[inline]
fn marker_matches_prefix(marker: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| {
        marker == *prefix
            || marker.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.starts_with(':') || suffix.starts_with(char::is_whitespace)
            })
    })
}

/// Parse one directive token from one comment token span.
#[inline]
fn directive_token_for_comment_token(
    ctx: &DestackFormatContext<'_>,
    token: TokenSpan,
) -> Option<IgnoreDirective> {
    parse_directive_token_from_comment_text(ctx.token_str(token))
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

/// Return whether one node has a prefix ignore directive.
pub fn node_has_ignore_directive<T: Node + Clone>(
    ctx: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> bool
where
    NodeTree: NodeTreeImpl<T>,
{
    if !ctx.has_ignore_directive_markers() {
        return false;
    }

    let node_span = ctx.span(node_id);
    let comment_tokens = ctx.comment_tokens();
    let Some(token) = prefix_comment_token_for_node(ctx, node_span, comment_tokens) else {
        return false;
    };
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
        return false;
    }

    matches!(
        directive_token_for_comment_token(ctx, token),
        Some(IgnoreDirective::Ignore | IgnoreDirective::IgnoreStart)
    )
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
        Some(IgnoreDirective::Ignore) => {
            let range_span = Span::new(node_span.file, token.span.start, node_span.end);
            Some(ctx.extend_span_with_trailing_line_tokens(range_span))
        }
        Some(IgnoreDirective::IgnoreStart) => {
            let end_token = find_ignore_range_end(ctx, comment_tokens, token.span.end)?;
            let mut end_span = ctx.extend_span_with_trailing_line_tokens(end_token.span);

            // line end markers should preserve their trailing newline
            if ctx.comment_is_line(end_token) {
                if let Some((line_index, _)) = ctx.source_position(end_token.span.start) {
                    if let Some(next_line_span) = ctx.source_line_span(line_index + 1) {
                        end_span = Span::new(end_span.file, end_span.start, next_line_span.start);
                    }
                }
            }

            Some(Span::new(token.span.file, token.span.start, end_span.end))
        }
        Some(IgnoreDirective::IgnoreFile | IgnoreDirective::IgnoreEnd) | None => None,
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

/// Return whether this file has a formatter ignore-file directive comment.
pub fn has_file_ignore_directive(ctx: &DestackFormatContext<'_>) -> bool {
    if !ctx.has_ignore_directive_markers() {
        return false;
    }

    for token in ctx.all_tokens().iter().copied() {
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
                    Some(IgnoreDirective::IgnoreFile)
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
    let source = ctx.span_str(span);
    if ctx.source_position(span.start).is_none() {
        return source.to_owned();
    }
    let Some(prefix) = ctx.line_prefix_text(span.start) else {
        return source.to_owned();
    };
    if prefix.is_empty() || !prefix.trim().is_empty() {
        return source.to_owned();
    }

    source
        .split('\n')
        .map(|line| line.strip_prefix(prefix).unwrap_or(line).to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Write one ignored span with formatter-managed indentation.
pub fn write_ignored_span<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    span: Span,
) -> FormatResult<()> {
    // ignored spans already contain their own comments
    f.context_mut()
        .comments_mut()
        .skip_comments_before(span.end);

    let source = ignored_span_source(f.context(), span);
    let source = if !f.context().span_starts_on_own_line(span) {
        dedent_common_leading_whitespace_after_first_line(source.as_str())
    } else {
        dedent_common_leading_whitespace(source.as_str())
    };
    let source = if source.trim().is_empty() {
        source
            .chars()
            .filter(|character| *character == '\n')
            .collect::<String>()
    } else {
        source
    };

    let mut segment_start = 0usize;
    while segment_start < source.len() {
        let Some(relative_newline_index) = source[segment_start..].find('\n') else {
            write!(f, [text(&source[segment_start..])])?;
            break;
        };

        let newline_index = segment_start + relative_newline_index;
        if segment_start < newline_index {
            write!(f, [text(&source[segment_start..newline_index])])?;
        }

        let mut newline_run_end = newline_index;
        let bytes = source.as_bytes();
        while newline_run_end < bytes.len() && bytes[newline_run_end] == b'\n' {
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
) -> FormatResult<()>
where
    NodeTree: NodeTreeImpl<T>,
{
    let span = f.context().span(node_id);
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

/// Find the matching ignore range end comment following a start offset.
fn find_ignore_range_end(
    ctx: &DestackFormatContext<'_>,
    comment_tokens: &[TokenSpan],
    start_offset: u32,
) -> Option<TokenSpan> {
    let mut nested_range_depth = 0usize;

    for token in comment_tokens.iter().copied() {
        if token.span.start < start_offset {
            continue;
        }

        match directive_token_for_comment_token(ctx, token) {
            Some(IgnoreDirective::IgnoreStart) => {
                nested_range_depth += 1;
            }
            Some(IgnoreDirective::IgnoreEnd) => {
                if nested_range_depth == 0 {
                    return Some(token);
                }

                nested_range_depth -= 1;
            }
            Some(IgnoreDirective::Ignore | IgnoreDirective::IgnoreFile) | None => {}
        }
    }

    None
}

/// Parse a directive token from comment text, including markers.
fn parse_directive_token_from_comment_text(comment_text: &str) -> Option<IgnoreDirective> {
    let content = strip_comment_markers(comment_text);
    parse_directive_token(content.as_ref())
}

/// Return whether one comment text contains any recognized ignore directive.
pub(crate) fn comment_text_has_ignore_directive_marker(comment_text: &str) -> bool {
    parse_directive_token_from_comment_text(comment_text).is_some()
}

/// Return whether one comment text contains a single-node suppression directive.
pub(crate) fn comment_text_has_suppression_directive(comment_text: &str) -> bool {
    parse_directive_token_from_comment_text(comment_text) == Some(IgnoreDirective::Ignore)
}

/// Strip comment markers from comment text.
fn strip_comment_markers(comment_text: &str) -> Cow<'_, str> {
    let trimmed = comment_text.trim();
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
fn parse_directive_token(comment: &str) -> Option<IgnoreDirective> {
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

    if !has_additional_significant_line
        && marker_matches_any(first_significant_line, IGNORE_DIRECTIVES)
    {
        return Some(IgnoreDirective::Ignore);
    }

    if !has_additional_significant_line
        && marker_matches_any(first_significant_line, IGNORE_FILE_DIRECTIVES)
    {
        return Some(IgnoreDirective::IgnoreFile);
    }

    if !has_additional_significant_line
        && marker_matches_any(first_significant_line, IGNORE_START_DIRECTIVES)
    {
        return Some(IgnoreDirective::IgnoreStart);
    }

    if !has_additional_significant_line
        && marker_matches_any(first_significant_line, IGNORE_END_DIRECTIVES)
    {
        return Some(IgnoreDirective::IgnoreEnd);
    }

    if !has_additional_significant_line
        && marker_matches_prefix(first_significant_line, IGNORE_PREFIX_DIRECTIVES)
    {
        return Some(IgnoreDirective::Ignore);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{comment_text_has_ignore_directive_marker, comment_text_has_suppression_directive};

    #[test]
    /// Suppression aliases should map to single-node ignore directives.
    fn test_comment_text_has_suppression_directive_aliases() {
        // accepted aliases
        assert!(comment_text_has_suppression_directive("// fmt-ignore"));
        assert!(comment_text_has_suppression_directive("// format-ignore"));
        assert!(comment_text_has_suppression_directive("// prettier-ignore"));
        assert!(comment_text_has_suppression_directive("// deno-fmt-ignore"));
        assert!(comment_text_has_suppression_directive(
            "/* biome-ignore format: keep raw */"
        ));

        // reject non suppression directives
        assert!(!comment_text_has_suppression_directive(
            "// fmt-ignore-start"
        ));
        assert!(!comment_text_has_suppression_directive("// fmt-ignore-end"));
        assert!(!comment_text_has_suppression_directive(
            "// fmt-ignore-file"
        ));
    }

    #[test]
    /// Directive markers should include single-node, range, and file aliases.
    fn test_comment_text_has_ignore_directive_marker_aliases() {
        // accepted directives
        assert!(comment_text_has_ignore_directive_marker("// fmt-ignore"));
        assert!(comment_text_has_ignore_directive_marker("// format-ignore"));
        assert!(comment_text_has_ignore_directive_marker(
            "// fmt-ignore-start"
        ));
        assert!(comment_text_has_ignore_directive_marker(
            "// fmt-ignore-end"
        ));
        assert!(comment_text_has_ignore_directive_marker(
            "// fmt-ignore-file"
        ));
        assert!(comment_text_has_ignore_directive_marker(
            "/* biome-ignore format: keep raw */"
        ));

        // reject unrelated comments
        assert!(!comment_text_has_ignore_directive_marker("// fmt: ignore"));
        assert!(!comment_text_has_ignore_directive_marker("// format"));
        assert!(!comment_text_has_ignore_directive_marker(
            "// no formatter directive"
        ));
    }
}
