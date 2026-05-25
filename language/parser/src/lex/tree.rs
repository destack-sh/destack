use super::html::HTML_NAMED_ENTITIES;
use super::lexer::Lexer;
use destack_dir::{
    Token, TokenLiteral, TokenSpan, TokenType, is_identifier_continue, is_identifier_start,
};
use destack_source::{File, Span};
use memchr::memchr;

impl Lexer {
    /// Return one tree child token from the live lexer cursor.
    pub(crate) fn next_tree_child_token(&mut self) -> TokenSpan {
        let is_on_new_line = self.pending_line_terminator_before_next;
        let token = Self::tree_child_token(self.file(), self.position() as u32, is_on_new_line);
        self.set_position(token.span.end as usize);

        self.prepare_semantic_token(token)
    }

    /// Return one tree tag token from the live lexer cursor.
    pub(crate) fn next_tree_tag_token(&mut self) -> TokenSpan {
        self.eat_tree_tag_trivia();

        let is_on_new_line = self.pending_line_terminator_before_next;
        let token = Self::tree_tag_token(self.file(), self.position() as u32, is_on_new_line);
        self.set_position(token.span.end as usize);

        self.prepare_semantic_token(token)
    }

    /// Return one tree attribute value token from the live lexer cursor.
    pub(crate) fn next_tree_attribute_value_token(&mut self) -> Option<TokenSpan> {
        let is_on_new_line = self.pending_line_terminator_before_next;
        let token =
            Self::tree_attribute_value_token(self.file(), self.position() as u32, is_on_new_line)?;
        self.set_position(token.span.end as usize);

        Some(self.prepare_semantic_token(token))
    }

    /// Return one contextual tree child token from source.
    pub(crate) fn tree_child_token(file: &File, start: u32, is_on_new_line: bool) -> TokenSpan {
        let source = file.text();
        let start = start as usize;
        let bytes = source.as_bytes();

        if start >= bytes.len() {
            return TokenSpan {
                token: Token::end(),
                span: Span::new(file.id, file.len, file.len),
            };
        }

        let byte = bytes[start];
        if byte == b'<' {
            return Self::source_token(file, TokenType::LessThan, start, 1, is_on_new_line, None);
        }

        if byte == b'{' {
            return Self::source_token(file, TokenType::OpenBrace, start, 1, is_on_new_line, None);
        }

        let end = Self::tree_child_text_end(bytes, start);
        if byte == b'&'
            && let Some(semicolon) = memchr(b';', &bytes[start..end])
        {
            let entity_end = start + semicolon + 1;
            let span = Span::new(file.id, start as u32, entity_end as u32);
            if decode_html_entity(file.span_str(span)).is_some() {
                let literal = TokenLiteral::Character {
                    is_terminated: true,
                    is_html_entity: true,
                };

                return Self::source_token(
                    file,
                    TokenType::Literal,
                    start,
                    entity_end - start,
                    is_on_new_line,
                    Some(literal),
                );
            }
        }

        Self::source_token(
            file,
            TokenType::Literal,
            start,
            end - start,
            is_on_new_line,
            Some(TokenLiteral::TreeString),
        )
    }

    /// Return one contextual tree tag token from source.
    pub(crate) fn tree_tag_token(file: &File, start: u32, is_on_new_line: bool) -> TokenSpan {
        let source = file.text();
        let bytes = source.as_bytes();
        let offset = start as usize;
        let Some(byte) = bytes.get(offset).copied() else {
            return TokenSpan {
                token: Token::end(),
                span: Span::new(file.id, file.len, file.len),
            };
        };

        match byte {
            b'<' => Self::source_token(file, TokenType::LessThan, offset, 1, is_on_new_line, None),
            b'/' => Self::source_token(file, TokenType::Divide, offset, 1, is_on_new_line, None),
            b'>' => Self::source_token(
                file,
                TokenType::GreaterThan,
                offset,
                1,
                is_on_new_line,
                None,
            ),
            b':' => Self::source_token(file, TokenType::Colon, offset, 1, is_on_new_line, None),
            b',' => Self::source_token(file, TokenType::Comma, offset, 1, is_on_new_line, None),
            b'.' => Self::source_token(file, TokenType::Dot, offset, 1, is_on_new_line, None),
            b'-' => Self::source_token(file, TokenType::Subtract, offset, 1, is_on_new_line, None),
            b'=' => Self::source_token(file, TokenType::Assign, offset, 1, is_on_new_line, None),
            b'{' => Self::source_token(file, TokenType::OpenBrace, offset, 1, is_on_new_line, None),
            b'}' => {
                Self::source_token(file, TokenType::CloseBrace, offset, 1, is_on_new_line, None)
            }
            b'[' => Self::source_token(
                file,
                TokenType::OpenBracket,
                offset,
                1,
                is_on_new_line,
                None,
            ),
            b']' => Self::source_token(
                file,
                TokenType::CloseBracket,
                offset,
                1,
                is_on_new_line,
                None,
            ),
            b'\'' | b'"' => {
                let (end, literal) = Self::tree_attribute_string(file.text(), offset, byte);
                Self::source_token(
                    file,
                    TokenType::Literal,
                    offset,
                    end - offset,
                    is_on_new_line,
                    Some(literal),
                )
            }
            byte if Self::byte_starts_tree_tag_identifier(byte) => {
                let end = Self::tree_tag_identifier_end(source, offset);
                Self::source_token(
                    file,
                    TokenType::Identifier,
                    offset,
                    end - offset,
                    is_on_new_line,
                    None,
                )
            }
            _ => {
                let character = source[offset..].chars().next().unwrap_or('\0');
                if is_identifier_start(character) {
                    let end = Self::tree_tag_identifier_end(source, offset);
                    Self::source_token(
                        file,
                        TokenType::Identifier,
                        offset,
                        end - offset,
                        is_on_new_line,
                        None,
                    )
                } else {
                    Self::source_token(
                        file,
                        TokenType::Unknown,
                        offset,
                        character.len_utf8(),
                        is_on_new_line,
                        None,
                    )
                }
            }
        }
    }

    /// Return one contextual tree attribute value token from source.
    pub(crate) fn tree_attribute_value_token(
        file: &File,
        start: u32,
        is_on_new_line: bool,
    ) -> Option<TokenSpan> {
        let start = start as usize;
        let byte = file.text().as_bytes().get(start).copied()?;

        if !matches!(byte, b'\'' | b'"') {
            return None;
        }

        let (end, literal) = Self::tree_attribute_string(file.text(), start, byte);
        Some(Self::source_token(
            file,
            TokenType::Literal,
            start,
            end - start,
            is_on_new_line,
            Some(literal),
        ))
    }

    /// Return the end offset for one tree child text token.
    fn tree_child_text_end(bytes: &[u8], start: usize) -> usize {
        let mut end = start + 1;

        while end < bytes.len() && !matches!(bytes[end], b'<' | b'{' | b'}' | b'>' | b'&') {
            end += 1;
        }

        end
    }

    /// Return one tree attribute string extent and literal state.
    fn tree_attribute_string(source: &str, start: usize, quote: u8) -> (usize, TokenLiteral) {
        let bytes = source.as_bytes();
        let mut offset = start + 1;
        while offset < bytes.len() {
            let byte = bytes[offset];

            if byte == quote {
                let literal = TokenLiteral::String {
                    is_terminated: true,
                    has_invalid_escape: false,
                };
                return (offset + 1, literal);
            }

            if byte == b'\\' {
                offset += 2;
                continue;
            }

            offset += 1;
        }

        let literal = TokenLiteral::String {
            is_terminated: false,
            has_invalid_escape: false,
        };
        (offset, literal)
    }

    /// Eat whitespace and comments inside a tree tag.
    fn eat_tree_tag_trivia(&mut self) {
        loop {
            let bytes = self.remaining_text().as_bytes();
            let starts_trivia = bytes
                .first()
                .copied()
                .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r'))
                || matches!(bytes, [b'/', b'*', ..] | [b'/', b'/', ..]);

            if !starts_trivia {
                return;
            }

            self.lex_one();
        }
    }

    /// Return whether a source byte starts a tree tag identifier.
    fn byte_starts_tree_tag_identifier(byte: u8) -> bool {
        byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$')
    }

    /// Return the end offset of a tree tag identifier.
    fn tree_tag_identifier_end(source: &str, start: usize) -> usize {
        let mut end = start;
        for character in source[start..].chars() {
            if !is_identifier_continue(character) {
                break;
            }

            end += character.len_utf8();
        }

        end
    }

    /// Build one token from a source byte range.
    fn source_token(
        file: &File,
        token_type: TokenType,
        start: usize,
        len: usize,
        is_on_new_line: bool,
        literal: Option<TokenLiteral>,
    ) -> TokenSpan {
        let span = Span::new(file.id, start as u32, start as u32 + len as u32);

        TokenSpan {
            token: Token::new(token_type, len as u32, literal).with_on_new_line(is_on_new_line),
            span,
        }
    }
}

/// Decodes an HTML entity into a character.
pub fn decode_html_entity(entity: &str) -> Option<char> {
    if !entity.starts_with('&') || !entity.ends_with(';') {
        return None;
    }

    let body = &entity[1..entity.len() - 1];
    if body.is_empty() {
        return None;
    }

    if let Some(codepoint) = body.strip_prefix('#') {
        return decode_numeric_entity(codepoint);
    }

    decode_named_entity(body)
}

/// Decode HTML entities in text, returning an owned string only when text changes.
pub fn decode_html_entities(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut cursor = 0;
    let mut search = 0;
    let mut decoded = None;

    while let Some(relative_ampersand) = memchr(b'&', &bytes[search..]) {
        let ampersand = search + relative_ampersand;
        let Some(relative_semicolon) = memchr(b';', &bytes[ampersand..]) else {
            break;
        };

        // decode only valid semicolon terminated entities
        let entity_end = ampersand + relative_semicolon + 1;
        let entity = &text[ampersand..entity_end];
        let Some(character) = decode_html_entity(entity) else {
            search = ampersand + 1;
            continue;
        };

        // allocate lazily after the first real replacement
        let output = decoded.get_or_insert_with(|| String::with_capacity(text.len()));
        output.push_str(&text[cursor..ampersand]);
        output.push(character);

        cursor = entity_end;
        search = entity_end;
    }

    if let Some(output) = decoded.as_mut() {
        output.push_str(&text[cursor..]);
    }

    decoded
}

/// Decodes a numeric entity into a character (like `&#x1234;` or `&#1234;`).
#[inline]
fn decode_numeric_entity(codepoint: &str) -> Option<char> {
    let (radix, digits) = if let Some(hex_digits) = codepoint.strip_prefix(['x', 'X']) {
        (16, hex_digits)
    } else {
        (10, codepoint)
    };

    if digits.is_empty() {
        return None;
    }

    let is_valid_digits = if radix == 16 {
        digits.chars().all(|c| c.is_ascii_hexdigit())
    } else {
        digits.chars().all(|c| c.is_ascii_digit())
    };

    if !is_valid_digits {
        return None;
    }

    let value = u32::from_str_radix(digits, radix).ok()?;
    char::from_u32(value)
}

/// Decodes a named entity into a character (like `&lt;` or `&amp;`).
#[inline]
fn decode_named_entity(name: &str) -> Option<char> {
    HTML_NAMED_ENTITIES
        .binary_search_by(|(entity_name, _)| (*entity_name).cmp(name))
        .ok()
        .map(|idx| HTML_NAMED_ENTITIES[idx].1)
}
