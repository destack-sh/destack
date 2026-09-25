use std::borrow::Cow;
use std::collections::HashMap;

use super::source::{line_prefix_text, write_source_span};
use tspp_dir::{Comment, LocalNodeId, Node, NodeType, Tree, TreeStore};
use tspp_fir::format::FormatResult;
use tspp_source::Span;

use crate::{TsppFormatContext, TsppFormatter};

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

/// Parse one formatter directive from one comment.
#[inline]
fn parse_comment_directive(
    ctx: &TsppFormatContext<'_>,
    comment: Comment,
) -> Option<IgnoreDirective> {
    parse_comment_text_directive(ctx.source_text().text_for(&comment.span))
}

/// Return the last comment that starts before or at one node offset.
fn last_comment_before(source_comments: &[Comment], node_start: u32) -> Option<Comment> {
    let mut last_comment = None;
    for comment in source_comments {
        if comment.span.start <= node_start {
            last_comment = Some(*comment);
        } else {
            break;
        }
    }

    last_comment
}

/// Return one line-leading comment that prefixes one node span.
fn prefix_comment(
    ctx: &TsppFormatContext<'_>,
    node_span: Span,
    source_comments: &[Comment],
) -> Option<Comment> {
    let comment = last_comment_before(source_comments, node_span.start)?;
    if !comment_starts_line(ctx, comment) {
        return None;
    }

    Some(comment)
}

/// Return the source span used for ignore directive preservation.
fn ignore_target_span<T: Node + Clone>(ctx: &TsppFormatContext<'_>, node_id: LocalNodeId<T>) -> Span
where
    Tree: TreeStore<T>,
{
    if T::TYPE == NodeType::Expression {
        let expression_id = LocalNodeId::new(node_id.id);

        return ctx.expression_statement_extent(expression_id);
    }

    ctx.tree.get_source_extent(node_id)
}

/// Return whether one comment starts at the first non-whitespace position on its line.
fn comment_starts_line(ctx: &TsppFormatContext<'_>, comment: Comment) -> bool {
    let Some(prefix) = line_prefix_text(ctx, comment.span.start) else {
        return false;
    };

    prefix.trim().is_empty()
}

/// Return whether a trailing ignore gap contains only separators.
fn trailing_ignore_gap_is_allowed(ctx: &TsppFormatContext<'_>, span: Span) -> bool {
    ctx.source_text()
        .bytes_range(span.start, span.end)
        .iter()
        .all(|byte| matches!(*byte, b' ' | b'\t' | b'\r' | b'\n' | b';' | b','))
}

/// Return one same-line trailing ignore directive for a node span.
fn trailing_ignore_comment(
    ctx: &TsppFormatContext<'_>,
    node_span: Span,
    source_comments: &[Comment],
) -> Option<Comment> {
    let comment_index =
        source_comments.partition_point(|comment| comment.span.start < node_span.end);

    let comment = source_comments[comment_index..].iter().copied().next()?;

    if ctx.line_distance(node_span.end, comment.span.start) != Some(0) {
        return None;
    }

    let gap_span = Span::new(node_span.file, node_span.end, comment.span.start);
    if !trailing_ignore_gap_is_allowed(ctx, gap_span) {
        return None;
    }

    matches!(
        parse_comment_directive(ctx, comment),
        Some(IgnoreDirective::Ignore)
    )
    .then_some(comment)
}

/// Return one node's same-line trailing ignore directive.
fn trailing_ignore_directive<T: Node + Clone>(
    ctx: &TsppFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> Option<Comment>
where
    Tree: TreeStore<T>,
{
    if !ctx.has_ignore_directive_markers() {
        return None;
    }

    let node_span = ignore_target_span(ctx, node_id);
    let source_comments = ctx.source_comments();

    trailing_ignore_comment(ctx, node_span, source_comments)
}

/// Return whether one node has a same-line trailing ignore directive.
pub fn node_has_trailing_ignore_directive<T: Node + Clone>(
    ctx: &TsppFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> bool
where
    Tree: TreeStore<T>,
{
    trailing_ignore_directive(ctx, node_id).is_some()
}

/// Return whether one node has a same-line trailing line ignore directive.
pub fn node_has_trailing_line_ignore_directive<T: Node + Clone>(
    ctx: &TsppFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> bool
where
    Tree: TreeStore<T>,
{
    trailing_ignore_directive(ctx, node_id).is_some_and(Comment::is_line)
}

/// Return whether one node has a prefix ignore directive.
pub fn node_has_ignore_directive<T: Node + Clone>(
    ctx: &TsppFormatContext<'_>,
    node_id: LocalNodeId<T>,
) -> bool
where
    Tree: TreeStore<T>,
{
    if !ctx.has_ignore_directive_markers() {
        return false;
    }

    let node_span = ignore_target_span(ctx, node_id);
    let source_comments = ctx.source_comments();
    if trailing_ignore_comment(ctx, node_span, source_comments).is_some() {
        return true;
    }

    let Some(comment) = prefix_comment(ctx, node_span, source_comments) else {
        return false;
    };
    let (between_is_whitespace_only, line_distance) = if comment.span.end > node_span.start {
        (true, 0)
    } else {
        let between_is_whitespace_only = comment
            .span
            .gap_to(node_span)
            .is_none_or(|between_span| !ctx.has_non_whitespace_content(between_span));
        let line_distance = ctx
            .line_distance(comment.span.end, node_span.start)
            .map_or(2, |distance| distance as usize);
        (between_is_whitespace_only, line_distance)
    };
    if !between_is_whitespace_only || line_distance > 1 {
        return false;
    }

    matches!(
        parse_comment_directive(ctx, comment),
        Some(IgnoreDirective::Ignore | IgnoreDirective::IgnoreStart)
    )
}

/// Resolve an ignore range directive for a node.
pub fn ignore_range_for_node<T: Node + Clone>(
    ctx: &TsppFormatContext<'_>,
    node_id: LocalNodeId<T>,
    source_comments: &[Comment],
) -> Option<Span>
where
    Tree: TreeStore<T>,
{
    if !ctx.has_ignore_directive_markers() {
        return None;
    }

    let node_span = ignore_target_span(ctx, node_id);
    if let Some(comment) = trailing_ignore_comment(ctx, node_span, source_comments) {
        return Some(Span::new(node_span.file, node_span.start, comment.span.end));
    }

    let comment = prefix_comment(ctx, node_span, source_comments)?;

    let is_adjacent = ctx.line_distance(comment.span.start, node_span.start) == Some(1);
    if !is_adjacent {
        return None;
    }

    match parse_comment_directive(ctx, comment) {
        Some(IgnoreDirective::Ignore) => {
            let range_span = Span::new(node_span.file, comment.span.start, node_span.end);
            Some(ctx.extend_span_with_trailing_line_tokens(range_span))
        }
        Some(IgnoreDirective::IgnoreStart) => {
            let end_comment = find_ignore_range_end(ctx, source_comments, comment.span.end)?;
            let mut end_span = ctx.extend_span_with_trailing_line_tokens(end_comment.span);

            // line end markers should preserve their trailing newline
            if end_comment.is_line()
                && let Some((line_index, _)) = ctx.file.get_position(end_comment.span.start)
                && let Some(next_line_span) = ctx.file.get_line_span(line_index + 1)
            {
                end_span = Span::new(end_span.file, end_span.start, next_line_span.start);
            }

            Some(Span::new(
                comment.span.file,
                comment.span.start,
                end_span.end,
            ))
        }
        Some(IgnoreDirective::IgnoreFile | IgnoreDirective::IgnoreEnd) | None => None,
    }
}

/// Return the source span to preserve for one ignored node.
fn ignored_node_span<T: Node + Clone>(ctx: &TsppFormatContext<'_>, node_id: LocalNodeId<T>) -> Span
where
    Tree: TreeStore<T>,
{
    let node_span = ignore_target_span(ctx, node_id);
    let source_comments = ctx.source_comments();

    trailing_ignore_comment(ctx, node_span, source_comments).map_or(node_span, |comment| {
        Span::new(node_span.file, node_span.start, comment.span.end)
    })
}

/// Collect ignore ranges for a list of nodes keyed by node id.
pub fn ignore_ranges_for_nodes<T: Node + Clone>(
    ctx: &TsppFormatContext<'_>,
    node_ids: &[LocalNodeId<T>],
    source_comments: &[Comment],
) -> HashMap<u32, Span>
where
    Tree: TreeStore<T>,
{
    let mut ignore_ranges = HashMap::new();
    for node_id in node_ids.iter().copied() {
        if let Some(range_span) = ignore_range_for_node(ctx, node_id, source_comments) {
            ignore_ranges.insert(node_id.id, range_span);
        }
    }
    ignore_ranges
}

/// Return whether any node in a list has an ignore range.
pub fn any_ignore_range_for_nodes<T: Node + Clone>(
    ctx: &TsppFormatContext<'_>,
    node_ids: &[LocalNodeId<T>],
    source_comments: &[Comment],
) -> bool
where
    Tree: TreeStore<T>,
{
    node_ids
        .iter()
        .copied()
        .any(|node_id| ignore_range_for_node(ctx, node_id, source_comments).is_some())
}

/// Return whether this file has a formatter ignore-file directive comment.
pub fn has_file_ignore_directive(ctx: &TsppFormatContext<'_>) -> bool {
    if !ctx.has_ignore_directive_markers() {
        return false;
    }

    let Some(first_comment) = ctx.source_comments().first().copied() else {
        return false;
    };

    if !ctx
        .source_text()
        .all_bytes_match(0, first_comment.span.start, |byte| {
            byte.is_ascii_whitespace()
        })
    {
        return false;
    }

    matches!(
        parse_comment_directive(ctx, first_comment),
        Some(IgnoreDirective::IgnoreFile)
    )
}

/// Write one ignored node source range with formatter-managed indentation.
pub fn write_ignored_node<'ast, T: Node + Clone>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
) -> FormatResult<()>
where
    Tree: TreeStore<T>,
{
    let span = ignored_node_span(f.context(), node_id);
    write_source_span(f, span)
}

/// Find the matching ignore range end comment following a start offset.
fn find_ignore_range_end(
    ctx: &TsppFormatContext<'_>,
    source_comments: &[Comment],
    start_offset: u32,
) -> Option<Comment> {
    let mut nested_range_depth = 0usize;

    for comment in source_comments.iter().copied() {
        if comment.span.start < start_offset {
            continue;
        }

        match parse_comment_directive(ctx, comment) {
            Some(IgnoreDirective::IgnoreStart) => {
                nested_range_depth += 1;
            }
            Some(IgnoreDirective::IgnoreEnd) => {
                if nested_range_depth == 0 {
                    return Some(comment);
                }

                nested_range_depth -= 1;
            }
            Some(IgnoreDirective::Ignore | IgnoreDirective::IgnoreFile) | None => {}
        }
    }

    None
}

/// Parse one directive from comment text, including delimiters.
fn parse_comment_text_directive(comment_text: &str) -> Option<IgnoreDirective> {
    let content = strip_comment_markers(comment_text);
    parse_directive(content.as_ref())
}

/// Return whether one comment text contains any recognized ignore directive.
pub(crate) fn comment_text_has_ignore_directive_marker(comment_text: &str) -> bool {
    parse_comment_text_directive(comment_text).is_some()
}

/// Return whether one comment text contains a single-node suppression directive.
pub(crate) fn comment_text_has_suppression_directive(comment_text: &str) -> bool {
    parse_comment_text_directive(comment_text) == Some(IgnoreDirective::Ignore)
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

/// Parse one directive from comment content.
fn parse_directive(comment: &str) -> Option<IgnoreDirective> {
    let mut first_significant_line = None;
    let mut has_additional_significant_line = false;

    // collect the first non-empty directive line and reject multiline payloads
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

    // single-line suppression
    if !has_additional_significant_line
        && marker_matches_any(first_significant_line, IGNORE_DIRECTIVES)
    {
        return Some(IgnoreDirective::Ignore);
    }

    // single-line file suppression
    if !has_additional_significant_line
        && marker_matches_any(first_significant_line, IGNORE_FILE_DIRECTIVES)
    {
        return Some(IgnoreDirective::IgnoreFile);
    }

    // range start
    if !has_additional_significant_line
        && marker_matches_any(first_significant_line, IGNORE_START_DIRECTIVES)
    {
        return Some(IgnoreDirective::IgnoreStart);
    }

    // range end
    if !has_additional_significant_line
        && marker_matches_any(first_significant_line, IGNORE_END_DIRECTIVES)
    {
        return Some(IgnoreDirective::IgnoreEnd);
    }

    // prefix suppression aliases
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

    /// Suppression aliases should map to single-node ignore directives.
    #[test]
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

    /// Directive markers should include single-node, range, and file aliases.
    #[test]
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
