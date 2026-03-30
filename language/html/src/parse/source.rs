use destack_source::{FileId, Span};

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
    /// The authored self closing slash form when one exists.
    pub(crate) self_closing_style: Option<RawHtmlSelfClosingStyle>,
}

/// One authored doctype match.
#[derive(Debug, Clone)]
pub(crate) struct RawHtmlDoctype {
    /// The full authored doctype span.
    pub(crate) span: Span,
    /// The authored `doctype` keyword spelling.
    pub(crate) doctype_keyword: String,
    /// The authored doctype name spelling.
    pub(crate) name: String,
    /// The authored doctype keyword form.
    pub(crate) kind: RawHtmlDoctypeKind,
    /// The authored `public` or `system` keyword spelling when one exists.
    pub(crate) kind_keyword: Option<String>,
    /// The authored public id quote style when one exists.
    pub(crate) public_id_quote_style: Option<RawHtmlDoctypeQuoteStyle>,
    /// The authored system id quote style when one exists.
    pub(crate) system_id_quote_style: Option<RawHtmlDoctypeQuoteStyle>,
}

/// One authored end tag match.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RawHtmlEndTag {
    /// The full authored end tag span.
    pub(crate) span: Span,
    /// The authored end tag name.
    pub(crate) name_span: Span,
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
    /// The authored value form when one exists.
    pub(crate) value_form: Option<RawHtmlAttributeValueForm>,
}

/// One authored HTML attribute value form.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RawHtmlAttributeValueForm {
    /// One double-quoted value.
    DoubleQuoted,
    /// One single-quoted value.
    SingleQuoted,
    /// One unquoted value.
    Unquoted,
}

/// One authored doctype keyword form.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RawHtmlDoctypeKind {
    /// One bare `<!doctype name>` form.
    NameOnly,
    /// One `PUBLIC` doctype form.
    Public,
    /// One `SYSTEM` doctype form.
    System,
}

/// One authored doctype quote style.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RawHtmlDoctypeQuoteStyle {
    /// One double-quoted id.
    DoubleQuoted,
    /// One single-quoted id.
    SingleQuoted,
}

/// One authored self closing slash form.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RawHtmlSelfClosingStyle {
    /// One compact `/>` close.
    Compact,
    /// One spaced ` />` close.
    Spaced,
}

/// One source cursor over authored HTML.
#[derive(Debug, Clone)]
pub(crate) struct HtmlSourceCursor<'a> {
    /// The authored source text.
    source: &'a str,
    /// The current byte offset.
    offset: usize,
    /// The owning source file.
    file_id: FileId,
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

    /// Return the owning source file id.
    pub(crate) fn file_id(&self) -> FileId {
        self.file_id
    }

    /// Return the current byte offset.
    pub(crate) fn offset(&self) -> usize {
        self.offset
    }

    /// Set the current byte offset.
    pub(crate) fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    /// Return the authored source length in bytes.
    pub(crate) fn source_len(&self) -> usize {
        self.source.len()
    }

    /// Slice one authored source span.
    pub(crate) fn slice(&self, span: Span) -> &str {
        let start = span.start as usize;
        let end = span.end as usize;

        &self.source[start..end]
    }

    /// Return one empty span in the owning file.
    pub(crate) fn empty_span(&self) -> Span {
        Span::new(self.file_id, 0, 0)
    }

    /// Match the next authored doctype.
    pub(crate) fn match_doctype(&mut self) -> Option<RawHtmlDoctype> {
        let bytes = self.source.as_bytes();
        let mut index = self.offset;

        // next declaration
        while index < bytes.len() {
            if bytes[index] != b'<' {
                index += 1;
                continue;
            }

            if Self::has_ascii_prefix_ignore_case(&self.source[index..], "<!doctype") {
                let doctype = self.parse_doctype(index)?;
                self.offset = doctype.span.end as usize;

                return Some(doctype);
            }

            index += 1;
        }

        None
    }

    /// Parse one authored doctype at one known `<!doctype` position.
    fn parse_doctype(&self, start: usize) -> Option<RawHtmlDoctype> {
        let bytes = self.source.as_bytes();
        let doctype_keyword_start = start + 2;
        let doctype_keyword_end = doctype_keyword_start + "doctype".len();
        let mut index = doctype_keyword_end;

        index = Self::skip_whitespace(bytes, index);
        let name_start = index;
        index = Self::advance_name(bytes, index);
        let name_end = index;
        index = Self::skip_whitespace(bytes, index);

        let mut kind = RawHtmlDoctypeKind::NameOnly;
        let mut kind_keyword = None;
        let mut public_id_quote_style = None;
        let mut system_id_quote_style = None;

        // keyword
        if Self::has_ascii_prefix_ignore_case(&self.source[index..], "public") {
            let keyword_start = index;
            kind = RawHtmlDoctypeKind::Public;
            index += "public".len();
            kind_keyword = Some(self.source[keyword_start..index].to_string());
            index = Self::skip_whitespace(bytes, index);
            let (quote_style, next_index) = self.parse_doctype_quoted_id(index)?;
            public_id_quote_style = Some(quote_style);
            let next_index = Self::skip_whitespace(bytes, next_index);

            if let Some((quote_style, _)) = self.parse_optional_doctype_quoted_id(next_index) {
                system_id_quote_style = Some(quote_style);
            }
        } else if Self::has_ascii_prefix_ignore_case(&self.source[index..], "system") {
            let keyword_start = index;
            kind = RawHtmlDoctypeKind::System;
            index += "system".len();
            kind_keyword = Some(self.source[keyword_start..index].to_string());
            index = Self::skip_whitespace(bytes, index);
            let (quote_style, _) = self.parse_doctype_quoted_id(index)?;
            system_id_quote_style = Some(quote_style);
        }

        let end = self.advance_after_markup_declaration(start);

        Some(RawHtmlDoctype {
            span: Span::new(self.file_id, start as u32, end as u32),
            doctype_keyword: self.source[doctype_keyword_start..doctype_keyword_end].to_string(),
            name: self.source[name_start..name_end].to_string(),
            kind,
            kind_keyword,
            public_id_quote_style,
            system_id_quote_style,
        })
    }

    /// Parse one required quoted doctype id.
    fn parse_doctype_quoted_id(&self, index: usize) -> Option<(RawHtmlDoctypeQuoteStyle, usize)> {
        let bytes = self.source.as_bytes();
        let quote = *bytes.get(index)?;

        if quote != b'"' && quote != b'\'' {
            return None;
        }

        let mut next_index = index + 1;

        while next_index < bytes.len() && bytes[next_index] != quote {
            next_index += 1;
        }

        if next_index >= bytes.len() {
            return None;
        }

        let quote_style = if quote == b'"' {
            RawHtmlDoctypeQuoteStyle::DoubleQuoted
        } else {
            RawHtmlDoctypeQuoteStyle::SingleQuoted
        };

        Some((quote_style, next_index + 1))
    }

    /// Parse one optional quoted doctype id.
    fn parse_optional_doctype_quoted_id(
        &self,
        index: usize,
    ) -> Option<(RawHtmlDoctypeQuoteStyle, usize)> {
        self.parse_doctype_quoted_id(index)
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
        let end = self.advance_after_comment(start);
        self.offset = end;

        Some(Span::new(self.file_id, start as u32, end as u32))
    }

    /// Match the next authored instruction span.
    pub(crate) fn match_instruction(&mut self) -> Option<Span> {
        let start = self.source[self.offset..].find("<?")? + self.offset;
        let end = self.advance_after_processing_instruction(start);
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
                index = self.advance_after_comment(index);
                continue;
            }

            if self.source[index..].starts_with("<!") {
                index = self.advance_after_markup_declaration(index);
                continue;
            }

            if self.source[index..].starts_with("<?") {
                index = self.advance_after_processing_instruction(index);
                continue;
            }

            if self.source[index..].starts_with("</") {
                index = self.advance_after_end_tag(index);
                continue;
            }

            let Some(tag) = self.parse_start_tag(index) else {
                index += 1;
                continue;
            };

            if Self::tag_name_matches(&tag.name, expected_local_name) {
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
                index = self.advance_after_comment(index);
                continue;
            }

            if !self.source[index..].starts_with("</") {
                index += 1;
                continue;
            }

            let Some((name, end)) = self.parse_end_tag(index) else {
                index += 1;
                continue;
            };

            if Self::tag_name_matches(&name, expected_local_name) {
                self.offset = end;

                return Some(RawHtmlEndTag {
                    span: Span::new(self.file_id, index as u32, end as u32),
                    name_span: Span::new(
                        self.file_id,
                        index as u32 + 2,
                        (index + 2 + name.len()) as u32,
                    ),
                    start: index,
                });
            }

            index = end;
        }

        None
    }

    /// Parse one authored start tag at one known `<` position.
    fn parse_start_tag(&self, start: usize) -> Option<RawHtmlStartTag> {
        let bytes = self.source.as_bytes();
        let mut index = start + 1;

        if index >= bytes.len() || !Self::is_name_start(bytes[index]) {
            return None;
        }

        let name_start = index;
        index = Self::advance_name(bytes, index);
        let name_end = index;
        let mut attributes = Vec::new();

        loop {
            index = Self::skip_whitespace(bytes, index);

            if index >= bytes.len() {
                return None;
            }

            // normal close
            if bytes[index] == b'>' {
                let end = index + 1;

                return Some(RawHtmlStartTag {
                    span: Span::new(self.file_id, start as u32, end as u32),
                    name: self.source[name_start..name_end].to_string(),
                    attributes,
                    end,
                    is_self_closing: false,
                    self_closing_style: None,
                });
            }

            // self closing close
            if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'>') {
                let end = index + 2;
                let self_closing_style = if index > start && bytes[index - 1].is_ascii_whitespace()
                {
                    RawHtmlSelfClosingStyle::Spaced
                } else {
                    RawHtmlSelfClosingStyle::Compact
                };

                return Some(RawHtmlStartTag {
                    span: Span::new(self.file_id, start as u32, end as u32),
                    name: self.source[name_start..name_end].to_string(),
                    attributes,
                    end,
                    is_self_closing: true,
                    self_closing_style: Some(self_closing_style),
                });
            }

            let attribute_start = index;
            let attribute_name_start = index;
            index = Self::advance_attribute_name(bytes, index);

            if attribute_name_start == index {
                index += 1;
                continue;
            }

            let attribute_name_end = index;
            index = Self::skip_whitespace(bytes, index);

            // optional attribute value
            let (value_span, value_form) = if bytes.get(index) == Some(&b'=') {
                index += 1;
                index = Self::skip_whitespace(bytes, index);

                if index >= bytes.len() {
                    (None, None)
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

                    (
                        Some(Span::new(
                            self.file_id,
                            value_start as u32,
                            value_end as u32,
                        )),
                        Some(if quote == b'"' {
                            RawHtmlAttributeValueForm::DoubleQuoted
                        } else {
                            RawHtmlAttributeValueForm::SingleQuoted
                        }),
                    )
                } else {
                    let value_start = index;

                    while index < bytes.len()
                        && !bytes[index].is_ascii_whitespace()
                        && !matches!(bytes[index], b'>' | b'/')
                    {
                        index += 1;
                    }

                    (
                        Some(Span::new(self.file_id, value_start as u32, index as u32)),
                        Some(RawHtmlAttributeValueForm::Unquoted),
                    )
                }
            } else {
                (None, None)
            };
            let attribute_end = value_span
                .map(|span| span.end as usize)
                .unwrap_or(attribute_name_end);

            attributes.push(RawHtmlSourceAttribute {
                span: Span::new(self.file_id, attribute_start as u32, attribute_end as u32),
                name_span: Span::new(
                    self.file_id,
                    attribute_name_start as u32,
                    attribute_name_end as u32,
                ),
                value_span,
                value_form,
            });
        }
    }

    /// Parse one authored end tag at one known `</` position.
    fn parse_end_tag(&self, start: usize) -> Option<(String, usize)> {
        let bytes = self.source.as_bytes();
        let mut index = start + 2;

        if index >= bytes.len() || !Self::is_name_start(bytes[index]) {
            return None;
        }

        let name_start = index;
        index = Self::advance_name(bytes, index);
        let name_end = index;
        index = Self::skip_whitespace(bytes, index);

        if bytes.get(index) != Some(&b'>') {
            return None;
        }

        Some((self.source[name_start..name_end].to_string(), index + 1))
    }

    /// Advance after one HTML comment.
    fn advance_after_comment(&self, start: usize) -> usize {
        self.source[start + 4..]
            .find("-->")
            .map(|index| start + 4 + index + 3)
            .unwrap_or(self.source.len())
    }

    /// Advance after one markup declaration.
    fn advance_after_markup_declaration(&self, start: usize) -> usize {
        self.source[start + 2..]
            .find('>')
            .map(|index| start + 2 + index + 1)
            .unwrap_or(self.source.len())
    }

    /// Advance after one processing instruction.
    fn advance_after_processing_instruction(&self, start: usize) -> usize {
        self.source[start + 2..]
            .find("?>")
            .map(|index| start + 2 + index + 2)
            .or_else(|| {
                self.source[start + 2..]
                    .find('>')
                    .map(|index| start + 2 + index + 1)
            })
            .unwrap_or(self.source.len())
    }

    /// Advance after one end tag without validating its name.
    fn advance_after_end_tag(&self, start: usize) -> usize {
        self.source[start + 2..]
            .find('>')
            .map(|index| start + 2 + index + 1)
            .unwrap_or(self.source.len())
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
        while index < bytes.len() && Self::is_name_continue(bytes[index]) {
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
}
