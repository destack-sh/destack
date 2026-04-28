use std::borrow::Cow;

use destack_ast::Comment;
use destack_fir::format::{Buffer, Format, FormatResult, Formatter};
use destack_fir::prelude::{hard_line_break, text, token};
use destack_fir::write;
use destack_source::Span;
use destack_workspace::{JsdocCommentLineStrategy, JsdocOptions, QuoteStyle};

use crate::{DestackFormatContext, DestackFormatOptions};

use super::line::LineBuffer;
use super::md::format_description_markdown;
use super::normalize::normalize_tag_kind;
use super::parse::{Jsdoc, JsdocTag};

/// Result of formatting a Jsdoc comment, ready to be emitted as IR.
#[derive(Debug)]
pub(super) enum FormattedJsdoc {
    /// An empty Jsdoc comment.
    Empty,
    /// A single-line comment body.
    SingleLine(String),
    /// A multiline comment body without wrapper or line prefixes.
    MultiLine(String),
}

impl<'a> Format<DestackFormatContext<'a>> for FormattedJsdoc {
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'a>>) -> FormatResult<()> {
        match self {
            FormattedJsdoc::Empty => {}
            FormattedJsdoc::SingleLine(content) => {
                write!(
                    f,
                    [
                        token("/**"),
                        text(" "),
                        text(content),
                        text(" "),
                        token("*/")
                    ]
                )?;
            }
            FormattedJsdoc::MultiLine(content_str) => {
                write!(f, [token("/**")])?;
                for line in content_str.split('\n') {
                    if line.is_empty() {
                        write!(f, [hard_line_break(), text(" "), token("*")])?;
                    } else {
                        write!(
                            f,
                            [
                                hard_line_break(),
                                text(" "),
                                token("*"),
                                text(" "),
                                text(line)
                            ]
                        )?;
                    }
                }
                write!(f, [hard_line_break(), text(" "), token("*/")])?;
            }
        }

        Ok(())
    }
}

/// The ` * ` prefix used in multiline Jsdoc comments.
const LINE_PREFIX_LEN: usize = 3;

/// Holds shared per-comment Jsdoc formatting state.
pub(super) struct JsdocFormatter<'o> {
    pub(super) options: &'o JsdocOptions,
    pub(super) format_options: &'o DestackFormatOptions,
    pub(super) wrap_width: usize,
    pub(super) content_lines: LineBuffer,
}

impl<'o> JsdocFormatter<'o> {
    fn new(
        options: &'o JsdocOptions,
        format_options: &'o DestackFormatOptions,
        available_width: usize,
    ) -> Self {
        let wrap_width = available_width.saturating_sub(LINE_PREFIX_LEN);
        Self {
            options,
            format_options,
            wrap_width,
            content_lines: LineBuffer::new(),
        }
    }

    /// Format a Jsdoc comment.
    ///
    /// Returns `Some(formatted)` if the comment was modified, `None` if no changes are needed.
    fn format(mut self, comment: &Comment, source_text: &str) -> Option<FormattedJsdoc> {
        let content = &source_text[comment.span.start as usize..comment.span.end as usize];

        // extract inner content between `/**` and `*/`
        let content_span = comment.content_span();
        // skip the extra `*` in `/**`
        let jsdoc_span = Span::new(content_span.file, content_span.start + 1, content_span.end);
        let inner = &source_text[jsdoc_span.start as usize..jsdoc_span.end as usize];
        let jsdoc = Jsdoc::new(inner, jsdoc_span);

        let jsdoc_text = jsdoc.comment();
        let description = jsdoc_text.parsed_preserving_whitespace();

        // empty docs have no description and no tags
        if description.trim().is_empty() && jsdoc.tags().is_empty() {
            return Some(FormattedJsdoc::Empty);
        }

        // collect the logical description and non-description tags
        let (description, effective_tags) =
            collect_effective_tags(description.trim(), jsdoc.tags())?;

        // emit the merged description before tags
        self.format_description(&description);

        // emit all remaining tags
        self.format_tags(&effective_tags, source_text);

        self.finish(content)
    }

    /// Format the merged leading description.
    fn format_description(&mut self, description: &str) {
        if description.is_empty() {
            return;
        }

        let description = format_description_markdown(
            description,
            self.wrap_width,
            0,
            self.options.capitalize_descriptions,
            Some(self.format_options),
        );

        if self.options.description_tag {
            let line = self.content_lines.begin_line();
            line.push_str("@description ");
            line.push_str(&description);
        } else {
            self.content_lines.push(description);
        }
    }

    /// Format all normalized tags into the content buffer.
    fn format_tags(&mut self, effective_tags: &[(&JsdocTag<'_>, &str)], source_text: &str) {
        let mut prev_normalized_kind: Option<&str> = None;
        let mut prev_tag_had_trailing_blank = false;

        for (tag_index, &(tag, normalized_kind)) in effective_tags.iter().enumerate() {
            let is_first_tag = tag_index == 0;
            let should_capitalize = self.options.capitalize_descriptions
                && !should_skip_capitalize(normalized_kind)
                && is_known_tag(normalized_kind);

            // add a blank line between description and first tag
            if is_first_tag && !self.content_lines.is_empty() && !self.content_lines.last_is_empty()
            {
                self.content_lines.push_empty();
            }

            // add blank lines between tag groups
            if !is_first_tag
                && self.should_separate_tag(
                    prev_normalized_kind,
                    normalized_kind,
                    prev_tag_had_trailing_blank,
                )
                && !self.content_lines.last_is_empty()
            {
                self.content_lines.push_empty();
            }

            prev_normalized_kind = Some(normalized_kind);

            // detect a source blank line after this tag
            let source_has_trailing_blank = {
                let raw_ws = tag.comment().parsed_preserving_whitespace();
                raw_ws.ends_with("\n\n")
            };
            prev_tag_had_trailing_blank = source_has_trailing_blank;

            // track content before formatting this tag
            let tag_start = self.content_lines.byte_len();

            // preserve missing space between tag kind and `{type}`
            let has_no_space_before_type = {
                let kind_end = tag.kind.span.end as usize;
                kind_end < source_text.len() && source_text.as_bytes()[kind_end] == b'{'
            };

            if normalized_kind == "example" {
                self.format_example_tag(normalized_kind, tag);
            } else if is_type_name_comment_tag(normalized_kind) {
                self.format_type_name_comment_tag(
                    normalized_kind,
                    tag,
                    should_capitalize,
                    has_no_space_before_type,
                );
            } else if is_type_comment_tag(normalized_kind) {
                self.format_type_comment_tag(
                    normalized_kind,
                    tag,
                    should_capitalize,
                    has_no_space_before_type,
                );
            } else {
                self.format_generic_tag(normalized_kind, tag, should_capitalize);
            }

            // add trailing blanks only when the formatted tag or source needs one
            let tag_newline_count = self.content_lines.line_count_since(tag_start);
            let needs_trailing_blank = if normalized_kind == "example" && tag_newline_count > 1 {
                // separate multiline @example from a following different tag kind
                effective_tags
                    .get(tag_index + 1)
                    .is_some_and(|&(_, next_kind)| next_kind != normalized_kind)
            } else {
                // preserve source blank lines after block descriptions
                effective_tags.get(tag_index + 1).is_some()
                    && source_has_trailing_blank
                    && self.content_lines.last_line_is_block_end()
            };
            if needs_trailing_blank && !self.content_lines.last_is_empty() {
                self.content_lines.push_empty();
            }
        }
    }

    /// Return whether two adjacent tags need an empty line between them.
    fn should_separate_tag(
        &self,
        previous_kind: Option<&str>,
        kind: &str,
        previous_had_trailing_blank: bool,
    ) -> bool {
        // consecutive examples stay visually independent
        if previous_kind.is_some_and(|previous| previous == "example") && kind == "example" {
            return true;
        }

        // configured tag grouping separates every kind boundary
        if self.options.separate_tag_groups {
            return previous_kind.is_some_and(|previous| previous != kind);
        }

        // return tags can be separated from parameter tags
        if self.options.separate_returns_from_param
            && matches!(kind, "returns" | "yields")
            && previous_kind.is_some_and(|previous| !matches!(previous, "returns" | "yields"))
        {
            return true;
        }

        previous_had_trailing_blank && previous_kind.is_some_and(|previous| !is_known_tag(previous))
    }

    /// Return the final formatted comment, or `None` when it matches source.
    fn finish(self, content: &str) -> Option<FormattedJsdoc> {
        // trim leading and trailing blank content lines
        let content_str = self.content_lines.into_string();
        let content_str = content_str.trim_end_matches('\n');
        let mut iter = content_str.split('\n').skip_while(|l| l.is_empty());

        let Some(first) = iter.next() else {
            return Some(FormattedJsdoc::Empty);
        };

        // use single-line output when the final content has one line
        let second = iter.next();
        let use_single_line = match self.options.comment_line_strategy {
            JsdocCommentLineStrategy::SingleLine => second.is_none(),
            JsdocCommentLineStrategy::Multiline => false,
            JsdocCommentLineStrategy::Keep => {
                // preserve original single-line shape
                second.is_none() && !content.contains('\n')
            }
        };
        if use_single_line {
            // compare against the original wrapper before emitting a change
            let mut tmp = String::with_capacity(4 + first.len() + 3);
            tmp.push_str("/** ");
            tmp.push_str(first);
            tmp.push_str(" */");
            if tmp == content {
                return None;
            }
            return Some(FormattedJsdoc::SingleLine(first.to_string()));
        }

        // compare the wrapped multiline output against the original
        let capacity =
            content_str.len() + content_str.bytes().filter(|&b| b == b'\n').count() * 4 + 10;
        let mut tmp = String::with_capacity(capacity);
        tmp.push_str("/**");

        for line in std::iter::once(first).chain(second).chain(iter) {
            tmp.push('\n');
            if line.is_empty() {
                tmp.push_str(" *");
            } else {
                tmp.push_str(" * ");
                tmp.push_str(line);
            }
        }
        tmp.push('\n');
        tmp.push_str(" */");

        // skip unchanged comments
        if tmp == content {
            return None;
        }

        Some(FormattedJsdoc::MultiLine(content_str.to_string()))
    }

    /// Push a (possibly multi-line) description into `content_lines` as a single string,
    /// prepending `indent` to each non-empty line. When indent is empty, moves `desc` directly.
    pub(super) fn push_indented_desc(&mut self, indent: &str, mut desc: String) {
        if desc.is_empty() {
            return;
        }
        if indent.is_empty() {
            self.content_lines.push(desc);
            return;
        }
        if !desc.contains('\n') {
            desc.insert_str(0, indent);
            self.content_lines.push(desc);
            return;
        }
        // allocate once and scan forward with `find`
        let mut s = String::with_capacity(desc.len() + indent.len() * 4);
        let mut rest = desc.as_str();
        while let Some(nl) = rest.find('\n') {
            // skip indent for blank lines
            if nl > 0 {
                s.push_str(indent);
            }
            s.push_str(&rest[..=nl]);
            rest = &rest[nl + 1..];
        }
        if !rest.is_empty() {
            s.push_str(indent);
            s.push_str(rest);
        }
        self.content_lines.push(s);
    }

    /// Return the continuation indent string for description wrapping.
    pub(super) fn continuation_indent() -> &'static str {
        "  "
    }

    /// Return the width of the description continuation indent.
    pub(super) fn continuation_indent_width() -> usize {
        2
    }

    /// Returns the continuation indent string for code blocks (`@example`) and
    /// type continuation lines (union `|` wrapping, generic `<...>` wrapping).
    /// Uses `"\t"` when tabs are enabled, otherwise spaces equal to `indent_width`.
    pub(super) fn code_indent(&self) -> &'static str {
        if self.format_options.indent_style.is_tab() {
            "\t"
        } else {
            match self.format_options.indent_width {
                0 => "",
                1 => " ",
                3 => "   ",
                4 => "    ",
                5 => "     ",
                6 => "      ",
                7 => "       ",
                8 => "        ",
                _ => "  ",
            }
        }
    }

    /// Returns the width (in columns) of the code indent.
    /// Tabs count as `indent_width` columns for width calculations.
    pub(super) fn code_indent_width(&self) -> usize {
        self.format_options.indent_width as usize
    }

    /// The configured quote style.
    pub(super) fn quote_style(&self) -> QuoteStyle {
        self.format_options.quote_style
    }
}

/// Trim trailing whitespace from an owned `String` in place, avoiding a reallocation.
pub(super) fn truncate_trim_end(s: &mut String) {
    let trimmed_len = s.trim_end().len();
    s.truncate(trimmed_len);
}

/// Return whether a tag description should not be capitalized.
fn should_skip_capitalize(tag_kind: &str) -> bool {
    matches!(
        tag_kind,
        "borrows" | "default" | "defaultValue" | "deprecated" | "memberof" | "module" | "see"
    )
}

/// Return whether a tag description should not be formatted.
pub(super) fn should_skip_description_formatting(tag_kind: &str) -> bool {
    matches!(
        tag_kind,
        "borrows"
            | "default"
            | "defaultValue"
            | "deprecated"
            | "internal"
            | "memberof"
            | "module"
            | "see"
    )
}

/// Return whether a tag description should be preserved verbatim.
pub(super) fn should_preserve_description_verbatim(tag_kind: &str) -> bool {
    matches!(
        tag_kind,
        "borrows" | "default" | "defaultValue" | "memberof" | "module" | "see"
    )
}

/// Return whether a tag uses `@tag {type} name description`.
pub(super) fn is_type_name_comment_tag(tag_kind: &str) -> bool {
    matches!(tag_kind, "param" | "property" | "fires")
}

/// Return whether a tag uses `@tag {type} description`.
pub(super) fn is_type_comment_tag(tag_kind: &str) -> bool {
    matches!(
        tag_kind,
        "returns" | "yields" | "throws" | "this" | "extends"
    )
}

/// Return whether a tag belongs to the Jsdoc type system.
fn is_type_system_tag(kind: &str) -> bool {
    matches!(
        kind,
        "callback" | "import" | "satisfies" | "template" | "type" | "typedef"
    )
}

/// Return the merged description and normalized tags to format.
fn collect_effective_tags<'a>(
    description: &str,
    tags: &'a [JsdocTag<'a>],
) -> Option<(String, Vec<(&'a JsdocTag<'a>, &'a str)>)> {
    let mut description = description.to_string();
    let mut effective_tags = Vec::with_capacity(tags.len());

    for tag in tags {
        let normalized_kind = normalize_tag_kind(tag.kind.parsed());
        if should_remove_empty_tag(normalized_kind) && !tag_has_content(tag) {
            continue;
        }

        if normalized_kind == "description" {
            let tag_description = tag.comment().parsed();
            let tag_description = tag_description.trim();
            if !tag_description.is_empty() {
                if !description.is_empty() {
                    description.push_str("\n\n");
                }
                description.push_str(tag_description);
            }
            continue;
        }

        effective_tags.push((tag, normalized_kind));
    }

    if effective_tags
        .iter()
        .any(|(_, kind)| is_type_system_tag(kind))
    {
        return None;
    }

    Some((description, effective_tags))
}

/// Return whether a tag kind has formatter behavior.
pub(super) fn is_known_tag(kind: &str) -> bool {
    matches!(
        kind,
        "abstract"
            | "async"
            | "augments"
            | "author"
            | "borrows"
            | "category"
            | "class"
            | "constant"
            | "default"
            | "defaultValue"
            | "deprecated"
            | "description"
            | "example"
            | "extends"
            | "external"
            | "file"
            | "fires"
            | "flow"
            | "function"
            | "ignore"
            | "license"
            | "member"
            | "memberof"
            | "module"
            | "namespace"
            | "overload"
            | "param"
            | "private"
            | "privateRemarks"
            | "property"
            | "providesModule"
            | "remarks"
            | "returns"
            | "see"
            | "since"
            | "this"
            | "throws"
            | "todo"
            | "typeParam"
            | "version"
            | "yields"
    )
}

/// Return whether a generic tag has a name before the description.
pub(super) fn is_named_generic_tag(kind: &str) -> bool {
    matches!(
        kind,
        "abstract"
            | "async"
            | "augments"
            | "author"
            | "categoryDescription"
            | "class"
            | "constant"
            | "external"
            | "flow"
            | "function"
            | "groupDescription"
            | "ignore"
            | "member"
            | "memberof"
            | "private"
            | "see"
            | "version"
            | "typeParam"
    )
}

/// Return whether a tag has meaningful content.
fn tag_has_content(tag: &JsdocTag<'_>) -> bool {
    let comment = tag.comment().parsed();
    !comment.trim().is_empty()
}

/// Return whether an empty tag should be removed.
fn should_remove_empty_tag(kind: &str) -> bool {
    matches!(
        kind,
        "borrows"
            | "category"
            | "description"
            | "example"
            | "privateRemarks"
            | "remarks"
            | "since"
            | "todo"
    )
}

/// Return whether the byte at `pos` is escaped.
/// An odd number of preceding backslashes means the character is escaped.
fn is_escaped(bytes: &[u8], pos: usize) -> bool {
    let mut count = 0;
    let mut j = pos;
    while j > 0 {
        j -= 1;
        if bytes[j] == b'\\' {
            count += 1;
        } else {
            break;
        }
    }
    count % 2 != 0
}

/// Format a `@default` / `@defaultValue` value.
/// Handles JSON-like formatting: spaces after `:` and `,`, inside `{}`.
/// Converts quotes based on the `quote_style` option.
/// Non-JSON values (code, plain text) are returned as-is.
pub(super) fn format_default_value(value: &str, quote_style: QuoteStyle) -> Cow<'_, str> {
    let trimmed = value.trim();
    // detect json-like values
    let first_byte = trimmed.as_bytes().first().copied().unwrap_or(b' ');
    if !matches!(first_byte, b'{' | b'[' | b'"' | b'\'') {
        return Cow::Borrowed(trimmed);
    }

    // convert only the leading quoted value
    if matches!(first_byte, b'"' | b'\'') {
        let quote = first_byte;
        let bytes = trimmed.as_bytes();
        let mut i = 1;
        while i < bytes.len() {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == quote {
                let inner = &trimmed[1..i];
                let rest = &trimmed[i + 1..];
                let (target, _other) = match quote_style {
                    QuoteStyle::Double | QuoteStyle::Semantic => ('"', '\''),
                    QuoteStyle::Single => ('\'', '"'),
                };
                if quote == target as u8 {
                    return Cow::Borrowed(trimmed);
                }
                let mut result = String::with_capacity(trimmed.len());
                result.push(target);
                for ch in inner.chars() {
                    if ch == target {
                        result.push('\\');
                    }
                    result.push(ch);
                }
                result.push(target);
                result.push_str(rest);
                return Cow::Owned(result);
            }
            i += 1;
        }
        return Cow::Borrowed(trimmed);
    }

    // choose target and alternate quote characters
    let (target_quote, other_quote) = match quote_style {
        QuoteStyle::Double | QuoteStyle::Semantic => (b'"', b'\''),
        QuoteStyle::Single => (b'\'', b'"'),
    };

    // normalize spacing and quotes
    let bytes = trimmed.as_bytes();
    let len = bytes.len();
    let mut result = String::with_capacity(len + 16);
    let mut i = 0;
    let mut in_target_quote = false;
    let mut in_other_quote = false;

    while i < len {
        let b = bytes[i];

        if in_target_quote {
            if b.is_ascii() {
                result.push(b as char);
            } else {
                let Some(ch) = trimmed[i..].chars().next() else {
                    break;
                };
                result.push(ch);
                i += ch.len_utf8();
                continue;
            }
            if b == target_quote && !is_escaped(bytes, i) {
                in_target_quote = false;
            }
            i += 1;
            continue;
        }

        if in_other_quote {
            if b == other_quote && !is_escaped(bytes, i) {
                result.push(target_quote as char);
                in_other_quote = false;
            } else if b.is_ascii() {
                result.push(b as char);
            } else {
                let Some(ch) = trimmed[i..].chars().next() else {
                    break;
                };
                result.push(ch);
                i += ch.len_utf8();
                continue;
            }
            i += 1;
            continue;
        }

        match b {
            _ if b == target_quote => {
                result.push(target_quote as char);
                in_target_quote = true;
                i += 1;
            }
            _ if b == other_quote => {
                result.push(target_quote as char);
                in_other_quote = true;
                i += 1;
            }
            b':' => {
                result.push(':');
                if i + 1 < len && bytes[i + 1] != b' ' {
                    result.push(' ');
                }
                i += 1;
            }
            b',' => {
                result.push(',');
                if i + 1 < len && bytes[i + 1] != b' ' {
                    result.push(' ');
                }
                i += 1;
            }
            b'{' => {
                result.push('{');
                if i + 1 < len && bytes[i + 1] != b'}' && bytes[i + 1] != b' ' {
                    result.push(' ');
                }
                i += 1;
            }
            b'}' => {
                if !result.is_empty() {
                    let last = result.as_bytes().last().copied().unwrap_or(b' ');
                    if last != b'{' && last != b' ' {
                        result.push(' ');
                    }
                }
                result.push('}');
                i += 1;
            }
            b'[' => {
                result.push('[');
                i += 1;
            }
            _ if b.is_ascii() => {
                result.push(b as char);
                i += 1;
            }
            _ => {
                let Some(ch) = trimmed[i..].chars().next() else {
                    break;
                };
                result.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    Cow::Owned(result)
}

/// Strip an existing "Default is `...`" or "Default is ..." suffix from a description.
/// Handles cases where "Default is" may be split across lines (e.g., "Default\nis `X`")
/// by searching a whitespace-normalized version of the text and mapping the position
/// back to the original.
pub(super) fn strip_default_is_suffix(desc: &str) -> Cow<'_, str> {
    // try the common single-line form first
    if let Some(pos) = desc.find("Default is ") {
        let before = desc[..pos].trim_end();
        let before = before.strip_suffix('.').unwrap_or(before);
        return Cow::Borrowed(before.trim_end());
    }

    // handle a suffix without a value
    if desc.trim_end().ends_with("Default is") {
        let trimmed = desc.trim_end();
        let before = trimmed[..trimmed.len() - "Default is".len()].trim_end();
        let before = before.strip_suffix('.').unwrap_or(before);
        return Cow::Borrowed(before.trim_end());
    }

    // map collapsed whitespace offsets back to source offsets
    let bytes = desc.as_bytes();
    let mut norm_pos_to_orig: Vec<usize> = Vec::with_capacity(desc.len());
    let mut prev_was_ws = false;
    for (i, &b) in bytes.iter().enumerate() {
        let is_ws = b == b' ' || b == b'\n' || b == b'\r' || b == b'\t';
        if is_ws {
            if !prev_was_ws {
                norm_pos_to_orig.push(i);
            }
            prev_was_ws = true;
        } else {
            norm_pos_to_orig.push(i);
            prev_was_ws = false;
        }
    }

    // build the normalized string
    let normalized: String = {
        let mut s = String::with_capacity(norm_pos_to_orig.len());
        let mut prev_ws = false;
        for &b in bytes {
            let is_ws = b == b' ' || b == b'\n' || b == b'\r' || b == b'\t';
            if is_ws {
                if !prev_ws {
                    s.push(' ');
                }
                prev_ws = true;
            } else {
                s.push(b as char);
                prev_ws = false;
            }
        }
        s
    };

    if let Some(norm_pos) = normalized.find("Default is ") {
        // map back to original position
        let orig_pos = norm_pos_to_orig[norm_pos];
        let before = desc[..orig_pos].trim_end();
        let before = before.strip_suffix('.').unwrap_or(before);
        return Cow::Borrowed(before.trim_end());
    }

    Cow::Borrowed(desc)
}

/// Format a Jsdoc comment and return whether it changed.
///
/// The returned `FormattedJsdoc` implements `Format` and emits the `/** ... */`
/// wrapper directly as IR tokens.
pub(super) fn format_jsdoc_body(
    comment: &Comment,
    options: &JsdocOptions,
    source_text: &str,
    available_width: usize,
    format_options: &DestackFormatOptions,
) -> Option<FormattedJsdoc> {
    let fmt = JsdocFormatter::new(options, format_options, available_width);
    fmt.format(comment, source_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_known_tag() {
        assert!(is_known_tag("param"));
        assert!(is_known_tag("returns"));
        assert!(is_known_tag("this"));
        assert!(!is_known_tag("custom"));
        assert!(!is_known_tag("typedef"));
        assert!(!is_known_tag("override"));
        assert!(!is_known_tag("link"));
    }

    #[test]
    fn test_should_skip_capitalize() {
        assert!(should_skip_capitalize("borrows"));
        assert!(should_skip_capitalize("default"));
        assert!(should_skip_capitalize("defaultValue"));
        assert!(should_skip_capitalize("memberof"));
        assert!(should_skip_capitalize("module"));
        assert!(should_skip_capitalize("see"));

        assert!(!should_skip_capitalize("param"));
        assert!(!should_skip_capitalize("returns"));
        assert!(should_skip_capitalize("deprecated"));
        assert!(!should_skip_capitalize("function"));
        assert!(!should_skip_capitalize("class"));
    }

    #[test]
    fn test_should_remove_empty_tag() {
        assert!(should_remove_empty_tag("borrows"));
        assert!(should_remove_empty_tag("category"));
        assert!(should_remove_empty_tag("description"));
        assert!(should_remove_empty_tag("example"));
        assert!(should_remove_empty_tag("privateRemarks"));
        assert!(should_remove_empty_tag("remarks"));
        assert!(should_remove_empty_tag("since"));
        assert!(should_remove_empty_tag("todo"));

        assert!(!should_remove_empty_tag("param"));
        assert!(!should_remove_empty_tag("returns"));
        assert!(!should_remove_empty_tag("deprecated"));
        assert!(!should_remove_empty_tag("abstract"));
    }

    #[test]
    fn test_format_default_value_keeps_empty_array_compact() {
        assert_eq!(format_default_value("[]", QuoteStyle::Double), "[]");
    }
}
