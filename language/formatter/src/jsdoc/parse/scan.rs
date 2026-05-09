use destack_source::Span;

use super::model::{JsdocTagKind, JsdocText};
use super::tag::JsdocTag;

/// Parse the inside of a Jsdoc comment into description and tags.
pub(super) fn parse_jsdoc(
    source_text: &str,
    jsdoc_span: Span,
) -> (JsdocText<'_>, Vec<JsdocTag<'_>>) {
    debug_assert!(!source_text.starts_with("/*"));
    debug_assert!(!source_text.ends_with("*/"));

    // docs have an optional description followed by optional tags
    let mut comment = None;

    let mut tags = vec![];

    // bracket and quote state
    let mut curly_brace_depth: i32 = 0;
    let mut brace_depth: i32 = 0;
    let mut square_brace_depth: i32 = 0;
    let mut backtick_count: u32 = 0;
    let mut in_double_quotes = false;
    let mut in_single_quotes = false;

    // tag markers only count at logical line starts
    let mut at_line_start = true;

    // indented code blocks can contain leading `@`
    let mut line_seen_star = false;
    let mut spaces_after_star: u32 = 0;

    // the first segment is the description, later segments are tags
    let mut comment_found = false;

    let (mut start, mut end) = (0, 0);
    let mut chars = source_text.chars().peekable();

    while let Some(ch) = chars.next() {
        // a tag cannot start inside nested syntax or quotes
        let can_parse = curly_brace_depth == 0
            && square_brace_depth == 0
            && brace_depth == 0
            && backtick_count == 0
            && !in_double_quotes
            && !in_single_quotes;

        match ch {
            // backtick sequences close only with the same length
            '`' if !in_single_quotes && !in_double_quotes => {
                let mut count: u32 = 1;
                while chars.peek() == Some(&'`') {
                    chars.next();
                    end += 1;
                    count += 1;
                }
                if backtick_count == 0 {
                    backtick_count = count;
                } else if backtick_count == count {
                    backtick_count = 0;
                }
            }
            '"' if backtick_count == 0 && !in_single_quotes => {
                in_double_quotes = !in_double_quotes;
            }
            '\'' if backtick_count == 0 && !in_double_quotes => {
                in_single_quotes = !in_single_quotes;
            }
            '\n' => {
                in_double_quotes = false;
                in_single_quotes = false;

                // prose brackets are line-oriented
                brace_depth = 0;
                square_brace_depth = 0;
            }
            '{' if backtick_count == 0 && !in_double_quotes && !in_single_quotes => {
                curly_brace_depth += 1;
            }
            '}' if backtick_count == 0 && !in_double_quotes && !in_single_quotes => {
                curly_brace_depth = (curly_brace_depth - 1).max(0);
            }
            '(' if backtick_count == 0 && !in_double_quotes && !in_single_quotes => {
                brace_depth += 1;
            }
            ')' if backtick_count == 0 && !in_double_quotes && !in_single_quotes => {
                brace_depth = (brace_depth - 1).max(0);
            }
            '[' if backtick_count == 0 && !in_double_quotes && !in_single_quotes => {
                square_brace_depth += 1;
            }
            ']' if backtick_count == 0 && !in_double_quotes && !in_single_quotes => {
                square_brace_depth = (square_brace_depth - 1).max(0);
            }

            '@' if can_parse
                && at_line_start
                && !is_indented_code_block(line_seen_star, spaces_after_star) =>
            {
                let segment = &source_text[start..end];
                let span = jsdoc_span.subspan(start..end);

                if comment_found {
                    tags.push(parse_jsdoc_tag(segment, span));
                } else {
                    comment = Some(JsdocText::new(segment, span));
                    comment_found = true;
                }

                start = end;
            }
            _ => {}
        }

        // update line-start tracking
        if ch == '\n' {
            at_line_start = true;
            line_seen_star = false;
            spaces_after_star = 0;
        } else if at_line_start {
            if ch == '*' {
                line_seen_star = true;
                spaces_after_star = 0;
            } else if matches!(ch, ' ' | '\t' | '\r') {
                if line_seen_star {
                    spaces_after_star += 1;
                }
            } else {
                at_line_start = false;
            }
        }

        end += ch.len_utf8();
    }

    // capture the final segment
    if start != end {
        let segment = &source_text[start..end];
        let span = jsdoc_span.subspan(start..end);

        if comment_found {
            tags.push(parse_jsdoc_tag(segment, span));
        } else {
            comment = Some(JsdocText::new(segment, span));
        }
    }

    (
        comment.unwrap_or(JsdocText::new("", jsdoc_span.subspan(0..0))),
        tags,
    )
}

/// Return whether the current line position is inside an indented code block.
fn is_indented_code_block(line_seen_star: bool, spaces_after_star: u32) -> bool {
    line_seen_star && spaces_after_star >= 5
}

/// Parse one Jsdoc tag.
fn parse_jsdoc_tag(tag_content: &str, jsdoc_tag_span: Span) -> JsdocTag<'_> {
    debug_assert!(tag_content.starts_with('@'));
    let (kind_start, kind_end) = find_tag_kind_range(tag_content);

    // tag kind
    let kind = JsdocTagKind::new(
        &tag_content[kind_start..kind_end],
        jsdoc_tag_span.subspan(kind_start..kind_end),
    );

    // keep splitter whitespace so comment parsing can distinguish inline stars
    let body = &tag_content[kind_end..];
    let body_span = jsdoc_tag_span.subspan(kind_end..tag_content.len());

    JsdocTag::new(kind, body, body_span)
}

/// Find the tag kind range for a string starting with `@`.
fn find_tag_kind_range(tag_content: &str) -> (usize, usize) {
    let kind_end = tag_content
        .char_indices()
        .find_map(|(index, ch)| (ch.is_whitespace() || ch == '{').then_some(index))
        .unwrap_or(tag_content.len());

    (0, kind_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse Jsdoc and return tag kind strings.
    fn tag_kinds(source: &str) -> Vec<String> {
        let file_id = destack_source::FileId::from_source_bytes(source.as_bytes());
        let span = Span::new(file_id, 0, source.len() as u32);
        let (_, tags) = parse_jsdoc(source, span);
        tags.iter().map(|t| t.kind.parsed().to_string()).collect()
    }

    #[test]
    fn test_backtick_inside_quotes_does_not_prevent_tag_split() {
        // backticks inside quoted types stay inert
        let src = " \n * @param {\"'\" | '\"' | '`'} string_start_char desc\n * @returns {number} The index";
        let kinds = tag_kinds(src);
        assert_eq!(kinds, vec!["param", "returns"]);
    }

    #[test]
    fn test_extra_closing_brace_does_not_prevent_tag_split() {
        // unmatched closing braces stay local to their tag
        let src = " \n * @param {AST.SvelteElement | AST.RegularElement} node}\n * @param {{ stop: () => void }} context";
        let kinds = tag_kinds(src);
        assert_eq!(kinds, vec!["param", "param"]);
    }

    #[test]
    fn test_normal_tags_still_split_correctly() {
        let src = " \n * @param {string} name The name\n * @returns {boolean} True if valid";
        let kinds = tag_kinds(src);
        assert_eq!(kinds, vec!["param", "returns"]);
    }

    #[test]
    fn test_inline_link_does_not_split_tag() {
        // inline links are not tag starts
        let src = " \n * @param {string} name See {@link Foo} for details\n * @returns {void}";
        let kinds = tag_kinds(src);
        assert_eq!(kinds, vec!["param", "returns"]);
    }

    #[test]
    fn test_at_sign_mid_line_does_not_split_tag() {
        // mid-line at signs are not tag starts
        let src = " \n * @param {string} email user@example.com address\n * @returns {void}";
        let kinds = tag_kinds(src);
        assert_eq!(kinds, vec!["param", "returns"]);
    }

    #[test]
    fn test_code_fence_does_not_prevent_tag_split() {
        // code fences do not affect following tags
        let src = " \n * @example\n * ```\n * const x = 1;\n * ```\n * @returns {void}";
        let kinds = tag_kinds(src);
        assert_eq!(kinds, vec!["example", "returns"]);
    }

    #[test]
    fn test_braces_inside_quotes_do_not_prevent_tag_split() {
        // quoted braces in prose stay inert
        let src = " \n * \"props\" of the form \"{ [key: string]: { type?: \"String\" | \"Object\" }\"\n * @param {null} node\n * @returns {never}";
        let kinds = tag_kinds(src);
        assert_eq!(kinds, vec!["param", "returns"]);
    }

    #[test]
    fn test_indented_code_block_at_sign_does_not_split_tag() {
        // indented code at signs are not tag starts
        let src = " \n * @deprecated\n *     @myDecorator\n *     class Foo {}\n * @type {string}";
        let kinds = tag_kinds(src);

        assert_eq!(kinds, vec!["deprecated", "type"]);
    }

    #[test]
    fn test_normal_indent_at_sign_still_splits() {
        // normal indented at signs are tag starts
        let src = " \n * @deprecated\n *    @type {string}";
        let kinds = tag_kinds(src);

        assert_eq!(kinds, vec!["deprecated", "type"]);
    }
}
