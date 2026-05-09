use std::borrow::Cow;

use destack_source::Span;

/// The raw text outside a tag kind and optional type expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::jsdoc) struct JsdocText<'a> {
    /// The raw string content.
    raw: &'a str,

    /// The source span.
    pub(in crate::jsdoc) span: Span,
}

impl<'a> JsdocText<'a> {
    pub(in crate::jsdoc) fn new(raw: &'a str, span: Span) -> Self {
        Self { raw, span }
    }

    // diagnostics point at the first visible line
    #[cfg(test)]
    pub(in crate::jsdoc) fn span_trimmed_first_line(&self) -> Span {
        if self.raw.trim().is_empty() {
            return self.span.subspan(0..0);
        }

        let base_len = self.raw.len();
        if self.raw.lines().count() == 1 {
            let trimmed_start_offset = base_len - self.raw.trim_start().len();
            let trimmed_end_offset = base_len - self.raw.trim_end().len();

            return self
                .span
                .subspan(trimmed_start_offset..base_len - trimmed_end_offset);
        }

        let start_trimmed = self.raw.trim_start();
        let trimmed_start_offset = base_len - start_trimmed.len();
        let first_line_len = start_trimmed
            .split_once('\n')
            .map_or(start_trimmed.len(), |(first_line, _)| first_line.len());
        let trimmed_end_offset = trimmed_start_offset + first_line_len;
        self.span.subspan(trimmed_start_offset..trimmed_end_offset)
    }

    /// Return the comment content while preserving paragraph breaks and indentation.
    pub(in crate::jsdoc) fn parsed_preserving_whitespace(&self) -> String {
        if !self.raw.contains('\n') {
            return self.raw.trim().to_string();
        }

        let mut result = String::with_capacity(self.raw.len());
        for (i, line) in self.raw.lines().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix('*') {
                // strip continuation leaders but leave emphasis markers alone
                let is_emphasis = rest.starts_with(|c: char| c.is_alphanumeric() || c == '_');
                if !is_emphasis {
                    result.push_str(rest.strip_prefix(' ').unwrap_or(rest));
                    continue;
                }
            }
            result.push_str(trimmed);
        }
        result
    }

    /// Return the content without leading comment leaders.
    pub(in crate::jsdoc) fn parsed(&self) -> String {
        if !self.raw.contains('\n') {
            return self.raw.trim().to_string();
        }

        let mut result = String::with_capacity(self.raw.len());
        for line in self.raw.lines() {
            // trim continuation leaders but leave emphasis markers alone
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix('*') {
                let is_emphasis = rest.starts_with(|c: char| c.is_alphanumeric() || c == '_');
                if !is_emphasis {
                    let content = rest.trim();
                    if content.is_empty() {
                        continue;
                    }
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str(content);
                    continue;
                }
            }
            if trimmed.is_empty() {
                continue;
            }
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(trimmed);
        }
        result
    }
}

#[derive(Debug, Clone, Copy)]
pub(in crate::jsdoc) struct JsdocTagKind<'a> {
    raw: &'a str,
    pub(in crate::jsdoc) span: Span,
}
impl<'a> JsdocTagKind<'a> {
    pub(in crate::jsdoc) fn new(raw: &'a str, span: Span) -> Self {
        debug_assert!(raw.starts_with('@'));
        debug_assert!(raw.trim() == raw);

        Self { raw, span }
    }

    /// Return the kind without the leading `@`.
    pub(in crate::jsdoc) fn parsed(&self) -> &'a str {
        &self.raw[1..]
    }
}

/// The raw type content inside a Jsdoc tag's curly braces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::jsdoc) struct JsdocTagType<'a> {
    /// The raw type expression, including braces.
    raw: Cow<'a, str>,

    /// The source span.
    pub(in crate::jsdoc) span: Span,
}

impl<'a> JsdocTagType<'a> {
    pub(in crate::jsdoc) fn new(raw: &'a str, span: Span) -> Self {
        debug_assert!(raw.starts_with('{'));
        debug_assert!(raw.ends_with('}'));

        Self {
            raw: clean_type_text(raw),
            span,
        }
    }

    /// Return the raw type string including braces.
    pub(in crate::jsdoc) fn raw(&self) -> &str {
        &self.raw
    }
}

/// Strip block comment leaders from multiline type parts.
fn clean_type_text(raw: &str) -> Cow<'_, str> {
    if !raw.contains('\n') {
        return Cow::Borrowed(raw);
    }

    let mut result = String::with_capacity(raw.len());

    for (index, line) in raw.lines().enumerate() {
        if index > 0 {
            result.push('\n');
        }

        if index == 0 {
            result.push_str(line);
            continue;
        }

        let line = strip_comment_leader(line);
        result.push_str(line);
    }

    Cow::Owned(result)
}

/// Strip one conventional Jsdoc `*` leader from a continuation line.
fn strip_comment_leader(line: &str) -> &str {
    let line = line.trim_start_matches([' ', '\t']);

    if let Some(line) = line.strip_prefix('*') {
        return line.strip_prefix(' ').unwrap_or(line);
    }

    line
}

/// A raw tag name with optional/default metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::jsdoc) struct JsdocTagName<'a> {
    raw: &'a str,

    /// The source span.
    pub(in crate::jsdoc) span: Span,

    /// Whether the name is optional.
    pub(in crate::jsdoc) optional: bool,

    /// Whether the name has a default value.
    pub(in crate::jsdoc) default: bool,
}

impl<'a> JsdocTagName<'a> {
    pub(in crate::jsdoc) fn new(raw: &'a str, span: Span) -> Self {
        debug_assert!(raw.trim() == raw);

        let optional = raw.starts_with('[') && raw.ends_with(']');
        let default = optional && raw.contains('=');

        Self {
            raw,
            span,
            optional,
            default,
        }
    }

    /// Return the raw text including optional/default brackets.
    pub(in crate::jsdoc) fn raw(&self) -> &'a str {
        self.raw
    }
}

#[cfg(test)]
mod test {
    use destack_source::{FileId, Span};

    use super::{JsdocTagKind, JsdocTagType, JsdocText};

    #[test]
    fn text_parsed() {
        for (actual, expect) in [
            ("", ""),
            ("hello  ", "hello"),
            ("  * single line", "* single line"),
            (" * ", "*"),
            (" * * ", "* *"),
            ("***", "***"),
            (
                "
      trim
    ",
                "trim",
            ),
            (
                "

    ", "",
            ),
            (
                "
                    *
                    *
                    ",
                "",
            ),
            (
                "
     * asterisk
    ",
                "asterisk",
            ),
            (
                "
     * * li
     * * li
    ",
                "* li\n* li",
            ),
            (
                "
    * list
    * list
    ",
                "list\nlist",
            ),
            (
                "
     * * 1
     ** 2
    ",
                "* 1\n* 2",
            ),
            (
                "
    1

    2

    3
                ",
                "1\n2\n3",
            ),
        ] {
            let file_id = FileId::from_source_bytes(actual.as_bytes());
            let text = JsdocText::new(actual, Span::empty(file_id));
            assert_eq!(text.parsed(), expect);
        }
    }

    #[test]
    fn text_span_trimmed() {
        for (actual, expect) in [
            ("", ""),
            ("\n", ""),
            ("\n\n\n", ""),
            ("...", "..."),
            ("c1\n", "c1"),
            ("\nc2\n", "c2"),
            (" c 3\n", "c 3"),
            ("\nc4\n * ...\n ", "c4"),
            (
                "
 extra text
*
",
                "extra text",
            ),
            (
                "
 * foo
 * bar
",
                "* foo",
            ),
        ] {
            let file_id = FileId::from_source_bytes(actual.as_bytes());
            let text = JsdocText::new(actual, Span::new(file_id, 0, actual.len() as u32));
            let span = text.span_trimmed_first_line();
            let actual = &actual[span.start as usize..span.end as usize];

            assert_eq!(actual, expect);
        }
    }

    #[test]
    fn tag_kind_parsed() {
        for (actual, expect) in [("@foo", "foo"), ("@", ""), ("@かいんど", "かいんど")] {
            let file_id = FileId::from_source_bytes(actual.as_bytes());
            let kind = JsdocTagKind::new(actual, Span::empty(file_id));
            assert_eq!(kind.parsed(), expect);
        }
    }

    #[test]
    fn tag_type_raw_strips_comment_leaders() {
        let source = "{{
 * \tfailed?: () => void;
 * }}";

        let file_id = FileId::from_source_bytes(source.as_bytes());
        let tag_type = JsdocTagType::new(source, Span::empty(file_id));

        assert_eq!(tag_type.raw(), "{{\n\tfailed?: () => void;\n}}");
    }
}
