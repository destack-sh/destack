use crate::Name;
use destack_source::{FileId, Span};

/// One lowered raw HTML attribute before source span attachment.
#[derive(Debug, Clone)]
pub(crate) struct RawHtmlAttribute {
    /// The parsed attribute name.
    pub(crate) name: Name,
    /// The authored attribute value.
    pub(crate) value: String,
}

/// One authored start tag match.
#[derive(Debug, Clone)]
pub(crate) struct RawHtmlStartTag {
    /// The full authored start tag span.
    pub(crate) span: Span,
    /// The authored tag name.
    pub(crate) name: String,
    /// The authored attributes in source order.
    pub(crate) attributes: Vec<RawHtmlSourceAttribute>,
    /// The first byte after the tag.
    pub(crate) end: usize,
    /// Whether the tag used self closing syntax.
    pub(crate) is_self_closing: bool,
}

/// One authored end tag match.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RawHtmlEndTag {
    /// The full authored end tag span.
    pub(crate) span: Span,
    /// The byte offset where the end tag begins.
    pub(crate) start: usize,
}

/// One authored source attribute match.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RawHtmlSourceAttribute {
    /// The full authored attribute span.
    pub(crate) span: Span,
    /// The authored name span.
    pub(crate) name_span: Span,
    /// The authored value span when one exists.
    pub(crate) value_span: Option<Span>,
}

/// One source cursor over authored HTML.
#[derive(Debug, Clone)]
pub(crate) struct HtmlSourceCursor<'a> {
    /// The authored source text.
    pub(crate) source: &'a str,
    /// The current byte offset.
    pub(crate) offset: usize,
    /// The owning source file.
    pub(crate) file_id: FileId,
}

impl<'a> HtmlSourceCursor<'a> {
    /// Build one new source cursor.
    pub(crate) fn new(source: &'a str, file_id: FileId) -> Self {
        Self {
            source,
            offset: 0,
            file_id,
        }
    }

    /// Match the next authored doctype.
    pub(crate) fn match_doctype(&mut self) -> Option<Span> {
        let bytes = self.source.as_bytes();
        let mut index = self.offset;

        // next declaration
        while index < bytes.len() {
            if bytes[index] != b'<' {
                index += 1;
                continue;
            }

            if has_ascii_prefix_ignore_case(&self.source[index..], "<!doctype") {
                let end = advance_after_markup_declaration(self.source, index);
                self.offset = end;

                return Some(Span::new(self.file_id, index as u32, end as u32));
            }

            index += 1;
        }

        None
    }

    /// Match the next authored text span.
    pub(crate) fn match_text(&mut self) -> Span {
        let bytes = self.source.as_bytes();
        let start = self.offset;
        let mut end = start;

        // next markup boundary
        while end < bytes.len() && bytes[end] != b'<' {
            end += 1;
        }

        self.offset = end;

        Span::new(self.file_id, start as u32, end as u32)
    }

    /// Match the next authored comment span.
    pub(crate) fn match_comment(&mut self) -> Option<Span> {
        let start = self.source[self.offset..].find("<!--")? + self.offset;
        let end = advance_after_comment(self.source, start);
        self.offset = end;

        Some(Span::new(self.file_id, start as u32, end as u32))
    }

    /// Match the next authored instruction span.
    pub(crate) fn match_instruction(&mut self) -> Option<Span> {
        let start = self.source[self.offset..].find("<?")? + self.offset;
        let end = advance_after_processing_instruction(self.source, start);
        self.offset = end;

        Some(Span::new(self.file_id, start as u32, end as u32))
    }

    /// Match the next authored start tag for one expected element name.
    pub(crate) fn match_start_tag(&mut self, expected_local_name: &str) -> Option<RawHtmlStartTag> {
        let bytes = self.source.as_bytes();
        let mut index = self.offset;

        // next matching start tag
        while index < bytes.len() {
            if bytes[index] != b'<' {
                index += 1;
                continue;
            }

            if self.source[index..].starts_with("<!--") {
                index = advance_after_comment(self.source, index);
                continue;
            }

            if self.source[index..].starts_with("<!") {
                index = advance_after_markup_declaration(self.source, index);
                continue;
            }

            if self.source[index..].starts_with("<?") {
                index = advance_after_processing_instruction(self.source, index);
                continue;
            }

            if self.source[index..].starts_with("</") {
                index = advance_after_end_tag(self.source, index);
                continue;
            }

            let Some(tag) = parse_start_tag(self.source, self.file_id, index) else {
                index += 1;
                continue;
            };

            if tag_name_matches(&tag.name, expected_local_name) {
                self.offset = tag.end;

                return Some(tag);
            }

            index = tag.end;
        }

        None
    }

    /// Advance past the next authored end tag for one element name.
    pub(crate) fn advance_past_end_tag(
        &mut self,
        expected_local_name: &str,
    ) -> Option<RawHtmlEndTag> {
        let bytes = self.source.as_bytes();
        let mut index = self.offset;

        // next matching end tag
        while index < bytes.len() {
            if bytes[index] != b'<' {
                index += 1;
                continue;
            }

            if self.source[index..].starts_with("<!--") {
                index = advance_after_comment(self.source, index);
                continue;
            }

            if !self.source[index..].starts_with("</") {
                index += 1;
                continue;
            }

            let Some((name, end)) = parse_end_tag(self.source, index) else {
                index += 1;
                continue;
            };

            if tag_name_matches(&name, expected_local_name) {
                self.offset = end;

                return Some(RawHtmlEndTag {
                    span: Span::new(self.file_id, index as u32, end as u32),
                    start: index,
                });
            }

            index = end;
        }

        None
    }
}

/// Parse one authored start tag at one known '<' position.
fn parse_start_tag(source: &str, file_id: FileId, start: usize) -> Option<RawHtmlStartTag> {
    let bytes = source.as_bytes();
    let mut index = start + 1;

    if index >= bytes.len() || !is_name_start(bytes[index]) {
        return None;
    }

    let name_start = index;
    index = advance_name(bytes, index);
    let name_end = index;
    let mut attributes = Vec::new();

    loop {
        index = skip_whitespace(bytes, index);

        if index >= bytes.len() {
            return None;
        }

        // normal close
        if bytes[index] == b'>' {
            let end = index + 1;

            return Some(RawHtmlStartTag {
                span: Span::new(file_id, start as u32, end as u32),
                name: source[name_start..name_end].to_string(),
                attributes,
                end,
                is_self_closing: false,
            });
        }

        // self closing close
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'>') {
            let end = index + 2;

            return Some(RawHtmlStartTag {
                span: Span::new(file_id, start as u32, end as u32),
                name: source[name_start..name_end].to_string(),
                attributes,
                end,
                is_self_closing: true,
            });
        }

        let attribute_start = index;
        let attribute_name_start = index;
        index = advance_attribute_name(bytes, index);

        if attribute_name_start == index {
            index += 1;
            continue;
        }

        let attribute_name_end = index;
        index = skip_whitespace(bytes, index);

        // optional attribute value
        let value_span = if bytes.get(index) == Some(&b'=') {
            index += 1;
            index = skip_whitespace(bytes, index);

            if index >= bytes.len() {
                None
            } else if bytes[index] == b'"' || bytes[index] == b'\'' {
                let quote = bytes[index];
                let value_start = index + 1;
                index += 1;

                while index < bytes.len() && bytes[index] != quote {
                    index += 1;
                }

                let value_end = index.min(bytes.len());

                if index < bytes.len() {
                    index += 1;
                }

                Some(Span::new(file_id, value_start as u32, value_end as u32))
            } else {
                let value_start = index;

                while index < bytes.len()
                    && !bytes[index].is_ascii_whitespace()
                    && !matches!(bytes[index], b'>' | b'/')
                {
                    index += 1;
                }

                Some(Span::new(file_id, value_start as u32, index as u32))
            }
        } else {
            None
        };
        let attribute_end = value_span
            .map(|span| span.end as usize)
            .unwrap_or(attribute_name_end);

        attributes.push(RawHtmlSourceAttribute {
            span: Span::new(file_id, attribute_start as u32, attribute_end as u32),
            name_span: Span::new(
                file_id,
                attribute_name_start as u32,
                attribute_name_end as u32,
            ),
            value_span,
        });
    }
}

/// Parse one authored end tag at one known '</' position.
fn parse_end_tag(source: &str, start: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let mut index = start + 2;

    if index >= bytes.len() || !is_name_start(bytes[index]) {
        return None;
    }

    let name_start = index;
    index = advance_name(bytes, index);
    let name_end = index;
    index = skip_whitespace(bytes, index);

    if bytes.get(index) != Some(&b'>') {
        return None;
    }

    Some((source[name_start..name_end].to_string(), index + 1))
}

/// Advance after one HTML comment.
fn advance_after_comment(source: &str, start: usize) -> usize {
    source[start + 4..]
        .find("-->")
        .map(|index| start + 4 + index + 3)
        .unwrap_or(source.len())
}

/// Advance after one markup declaration.
fn advance_after_markup_declaration(source: &str, start: usize) -> usize {
    source[start + 2..]
        .find('>')
        .map(|index| start + 2 + index + 1)
        .unwrap_or(source.len())
}

/// Advance after one processing instruction.
fn advance_after_processing_instruction(source: &str, start: usize) -> usize {
    source[start + 2..]
        .find("?>")
        .map(|index| start + 2 + index + 2)
        .or_else(|| {
            source[start + 2..]
                .find('>')
                .map(|index| start + 2 + index + 1)
        })
        .unwrap_or(source.len())
}

/// Advance after one end tag without validating its name.
fn advance_after_end_tag(source: &str, start: usize) -> usize {
    source[start + 2..]
        .find('>')
        .map(|index| start + 2 + index + 1)
        .unwrap_or(source.len())
}

/// Return whether two authored tag names match semantically.
fn tag_name_matches(authored_name: &str, expected_local_name: &str) -> bool {
    authored_name
        .rsplit(':')
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case(expected_local_name))
}

/// Return whether one source prefix matches ASCII case insensitively.
fn has_ascii_prefix_ignore_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|value| value.eq_ignore_ascii_case(prefix))
}

/// Skip one run of ASCII whitespace.
fn skip_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
        index += 1;
    }

    index
}

/// Advance over one HTML tag name.
fn advance_name(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && is_name_continue(bytes[index]) {
        index += 1;
    }

    index
}

/// Advance over one HTML attribute name.
fn advance_attribute_name(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len()
        && !bytes[index].is_ascii_whitespace()
        && !matches!(bytes[index], b'=' | b'>' | b'/')
    {
        index += 1;
    }

    index
}

/// Return whether one byte can start one HTML name.
fn is_name_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic()
}

/// Return whether one byte can continue one HTML name.
fn is_name_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
}
