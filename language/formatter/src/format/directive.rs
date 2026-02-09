use std::borrow::Cow;

use destack_ast::{
    Annotation, AnnotationPosition, Comment, LocalNodeId, Node, NodeTree, NodeTreeImpl, TokenSpan,
    TokenType,
};
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
    Prefix,
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
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Annotation> + NodeTreeImpl<Comment>,
{
    let annotations = context.get_annotations(node_id).unwrap_or_default();

    // use only prefix directives for ignore behavior
    // postfix comments like `expr(); // oxfmt-ignore` remain regular comments
    for annotation_id in annotations {
        let Annotation::Comment { node, position } = context.tree.get::<Annotation>(annotation_id)
        else {
            continue;
        };
        let comment = context.tree.get::<Comment>(*node);
        let content = context.strings.get(comment.string);
        let span = context.get_span(*node);
        let raw_comment = context.get_span_str(span);
        let Some(token) =
            parse_directive_token(content).or_else(|| parse_directive_token_from_raw(raw_comment))
        else {
            continue;
        };

        let kind = match token {
            FormatterDirectiveToken::Ignore | FormatterDirectiveToken::IgnoreStart => {
                FormatterDirectiveKind::IgnoreFormat
            }
            FormatterDirectiveToken::IgnoreEnd => {
                continue;
            }
        };

        match position {
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix => {
                return Some(FormatterDirective {
                    kind,
                    position: FormatterDirectivePosition::Prefix,
                });
            }
            _ => {}
        }
    }

    let node_span = context.get_span(node_id);
    let comment_tokens = collect_comment_tokens(context);
    let mut last_prefix_token: Option<TokenSpan> = None;
    for token in comment_tokens {
        if token.span.start <= node_span.start {
            last_prefix_token = Some(token);
        } else {
            break;
        }
    }

    if let Some(token) = last_prefix_token {
        if !comment_token_is_line_leading(context, token) {
            return None;
        }

        let raw = context.get_token_str(token);
        let (between, newlines) = if token.span.end > node_span.start {
            ("", 0)
        } else {
            let between_span = Span::new(node_span.file, token.span.end, node_span.start);
            let between = context.get_span_str(between_span);
            let newlines = between.chars().filter(|ch| *ch == '\n').count();
            (between, newlines)
        };
        if between.trim().is_empty() && newlines <= 1 {
            let token = parse_directive_token_from_raw(raw);
            let kind = match token {
                Some(FormatterDirectiveToken::Ignore | FormatterDirectiveToken::IgnoreStart) => {
                    Some(FormatterDirectiveKind::IgnoreFormat)
                }
                _ => None,
            };
            if let Some(kind) = kind {
                return Some(FormatterDirective {
                    kind,
                    position: FormatterDirectivePosition::Prefix,
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
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Annotation> + NodeTreeImpl<Comment>,
{
    let annotations = context.get_annotations(node_id).unwrap_or_default();
    let node_span = context.get_span(node_id);

    for annotation_id in annotations {
        let Annotation::Comment { node, position } = context.tree.get::<Annotation>(annotation_id)
        else {
            continue;
        };
        if !matches!(
            position,
            AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
        ) {
            continue;
        }

        let comment = context.tree.get::<Comment>(*node);
        let content = context.strings.get(comment.string);
        let start_span = context.get_span(*node);
        let raw_comment = context.get_span_str(start_span);
        let token =
            parse_directive_token(content).or_else(|| parse_directive_token_from_raw(raw_comment));
        if matches!(token, Some(FormatterDirectiveToken::Ignore)) {
            let range_span = Span::new(start_span.file, start_span.start, node_span.end);
            return Some(extend_span_with_trailing_tokens(context, range_span));
        }
        if matches!(token, Some(FormatterDirectiveToken::IgnoreStart)) {
            let Some(end_span) = find_ignore_range_end(context, comment_tokens, start_span.end)
            else {
                continue;
            };
            let range_span = Span::new(start_span.file, start_span.start, end_span.start);
            return Some(range_span);
        }
    }

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

        let raw = context.get_token_str(token);
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

/// Return whether a comment token starts at the first non-whitespace position on its line.
fn comment_token_is_line_leading(context: &DestackFormatContext<'_>, token: TokenSpan) -> bool {
    let Some((line_index, _)) = context.file.get_position(token.span.start) else {
        return false;
    };
    let Some(line_span) = context.file.get_line_span(line_index) else {
        return false;
    };

    let prefix_span = Span::new(token.span.file, line_span.start, token.span.start);
    context.get_span_str(prefix_span).trim().is_empty()
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
    for token in tokens {
        if token.span.start < span.end {
            continue;
        }

        match token.token.ty {
            TokenType::Whitespace => {
                let raw = context.get_token_str(token);
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

    if end > span.end {
        Span::new(span.file, span.start, end)
    } else {
        span
    }
}

/// Collect comment tokens sorted by source position.
pub fn collect_comment_tokens(context: &DestackFormatContext<'_>) -> Vec<TokenSpan> {
    let mut tokens: Vec<TokenSpan> = context
        .tokens
        .iter()
        .copied()
        .chain(context.side_tokens.iter().copied())
        .filter(|token| {
            matches!(
                token.token.ty,
                TokenType::LineComment
                    | TokenType::BlockComment
                    | TokenType::DocLineComment
                    | TokenType::DocBlockComment
            )
        })
        .collect();

    tokens.sort_by_key(|token| token.span.start);
    tokens
}

/// Extract the source for an ignored span.
pub fn ignored_span_source(context: &DestackFormatContext<'_>, span: Span) -> String {
    let raw = context.get_span_str(span);
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
    let start_column = f
        .context()
        .file
        .get_position(span.start)
        .map_or(0, |(_, column)| column);

    // preserve exact line structure for top level ignored spans
    if start_column == 0 {
        write!(f, [text(&raw)])?;
        return Ok(());
    }

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

/// Extract the source for an ignored node, removing its leading indentation.
pub fn ignored_node_source<T: Node>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    directive: FormatterDirective,
) -> String
where
    NodeTree: NodeTreeImpl<T>,
{
    let span = context.get_span(node_id);
    let end = match directive.position {
        FormatterDirectivePosition::Prefix => span.end,
        FormatterDirectivePosition::Postfix { comment_span } => comment_span.end.max(span.end),
    };
    ignored_span_source(context, Span::new(span.file, span.start, end))
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
            let raw_comment = context.get_token_str(*token);
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
        if matches!(
            trimmed,
            "prettier-ignore" | "oxfmt-ignore" | "deno-fmt-ignore" | "fmt-ignore" | "format-ignore"
        ) {
            return Some(FormatterDirectiveToken::Ignore);
        }

        // check for range start directives
        if matches!(
            trimmed,
            "prettier-ignore-start"
                | "fmt-ignore-start"
                | "format-ignore-start"
                | "biome-ignore-start"
        ) {
            return Some(FormatterDirectiveToken::IgnoreStart);
        }

        // check for range end directives
        if matches!(
            trimmed,
            "prettier-ignore-end" | "fmt-ignore-end" | "format-ignore-end" | "biome-ignore-end"
        ) {
            return Some(FormatterDirectiveToken::IgnoreEnd);
        }

        // check for biome ignore with format specifier
        if trimmed.starts_with("biome-ignore") && trimmed.contains("format") {
            return Some(FormatterDirectiveToken::Ignore);
        }

        None
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_ast::NodeParentIndex;
    use destack_parser::Parser;
    use destack_source::{File, FileId, FileType, LanguageType, Uri};
    use destack_workspace::FormatterOptions;

    use super::{collect_comment_tokens, ignore_range_for_node};
    use crate::{DestackFormatContext, DestackFormatOptions};

    #[test]
    fn test_format_ignore_range_for_statement() {
        let source = "// format-ignore\ncall(   a, b)";
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
        parser.finish();

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
            &file,
            &parser.tree,
            &tokens,
            &side_tokens,
            &side_span,
            &strings,
            parents,
        );

        let comment_tokens = collect_comment_tokens(&context);
        assert!(!comment_tokens.is_empty());
        let range = ignore_range_for_node(&context, expressions[0], &comment_tokens);
        assert!(range.is_some());
    }
}
