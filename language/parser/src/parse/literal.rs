use std::borrow::Cow;

use crate::lex::decode_html_entity;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

use destack_ast::{
    Argument, Expression, Keyword, LiteralType, LocalNodeId, NodeType, NumberBase, Path, Property,
    ScalarLiteral, StringId, TemplateLiteral, TokenSpan, TokenType,
};

impl Parser {
    /// Return true when the current token starts a template literal.
    #[inline]
    pub fn is_template_literal_start(&self) -> bool {
        self.peek_is(TokenType::TemplateString) || self.peek_is(TokenType::TemplateStringStart)
    }

    /// Return true when the current token starts a scalar literal.
    #[inline]
    pub fn is_scalar_literal_start(&self) -> bool {
        self.peek_is(TokenType::Literal)
    }

    /// Return true when the current token starts a tree literal.
    #[inline]
    pub fn is_tree_literal_start(&self) -> bool {
        if !self.peek_is(TokenType::LessThan) {
            return false;
        }

        let pos = self.pos_index();
        let len = self.tokens.len();
        let next = self
            .next_non_newline
            .get(pos)
            .copied()
            .unwrap_or(len as u32) as usize;
        let next_token = match self.tokens.get(next) {
            Some(token) => token,
            None => return false,
        };
        if next_token.token.ty == TokenType::Divide && !self.options.in_tree_literal {
            return false;
        }
        if !matches!(
            next_token.token.ty,
            TokenType::GreaterThan | TokenType::Divide | TokenType::Identifier
        ) {
            return false;
        }

        if next_token.token.ty == TokenType::Identifier {
            let after_identifier = self
                .next_non_newline
                .get(next)
                .copied()
                .unwrap_or(len as u32) as usize;
            if self
                .tokens
                .get(after_identifier)
                .is_some_and(|token| token.token.ty == TokenType::Comma)
            {
                return false;
            }
        }

        true
    }

    /// Peek a scalar literal token.
    #[inline]
    pub fn peek_scalar_literal(&self) -> ParseResult<&TokenSpan> {
        if self.peek_is(TokenType::Literal) {
            Ok(self.peek()?)
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a scalar literal and return its value.
    ///
    /// Examples:
    /// ```
    /// true
    /// false
    /// 1
    /// 1n
    /// 1.0
    /// 0x1234
    /// "Hello, world!"
    /// 'a'
    /// b'a'
    /// b"abc
    /// /abc/
    /// /abc/g
    /// ```
    pub fn eat_scalar_literal(&mut self) -> ParseResult<ScalarLiteral> {
        let _timing = self.timing_scope(tags::PARSE_LITERAL);
        let literal_span = *self.eat()?;
        let Some(body) = literal_span.token.literal else {
            return Err(ParseError::unexpected(literal_span.span));
        };
        let literal_str = self.file.span_str(literal_span.span);

        match body {
            // boolean literal
            LiteralType::Boolean { value } => Ok(ScalarLiteral::Boolean(value)),

            // int literal
            LiteralType::Int {
                base,
                is_empty,
                is_bigint,
            } => {
                if is_empty {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // strip underscores for parsing
                let content: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };

                // strip bigint suffix
                let content = if is_bigint {
                    Cow::Borrowed(content.trim_end_matches("n"))
                } else {
                    content
                };

                // handle base-specific prefixes
                let digits = match base {
                    NumberBase::Decimal => content,
                    NumberBase::Binary => Cow::Borrowed(content.trim_start_matches("0b")),
                    NumberBase::Octal => Cow::Borrowed(content.trim_start_matches("0o")),
                    NumberBase::Hexadecimal => Cow::Borrowed(content.trim_start_matches("0x")),
                };

                // parse with saturation to avoid overflow errors in parsing
                let parsed = self.parse_int_literal_saturating(&digits, base);

                // int
                if is_bigint {
                    Ok(ScalarLiteral::Bigint(parsed))
                } else {
                    Ok(ScalarLiteral::Integer(parsed))
                }
            }

            // float literal
            LiteralType::Float {
                base: _,
                is_empty_exponent,
            } => {
                if is_empty_exponent {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                let content: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };

                content
                    .parse::<f64>()
                    .map(ScalarLiteral::Float)
                    .map_err(|_| {
                        ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
            }

            // character literal (ignore quotes)
            LiteralType::Character {
                is_terminated,
                is_html_entity,
            } => {
                if is_html_entity {
                    decode_html_entity(literal_str)
                        .map(ScalarLiteral::Character)
                        .ok_or_else(|| {
                            ParseError::expected_for(
                                literal_span.span,
                                TokenType::Literal,
                                NodeType::Expression,
                            )
                        })
                } else {
                    if !is_terminated {
                        return Err(ParseError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        ));
                    }
                    let content = literal_str.trim_start_matches('\'').trim_end_matches('\'');
                    content
                        .chars()
                        .next()
                        .map(ScalarLiteral::Character)
                        .ok_or_else(|| {
                            ParseError::expected_for(
                                literal_span.span,
                                TokenType::Literal,
                                NodeType::Expression,
                            )
                        })
                }
            }

            // string literal (ignore quotes)
            // supports both '...' and "..." delimited string literals
            LiteralType::String {
                is_terminated,
                has_invalid_escape,
            } => {
                if !is_terminated || has_invalid_escape {
                    return Err(ParseError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }
                let content = {
                    // "..." or '...'
                    if literal_str.len() >= 2
                        && ((literal_str.starts_with('"') && literal_str.ends_with('"'))
                            || (literal_str.starts_with('\'') && literal_str.ends_with('\'')))
                    {
                        &literal_str[1..literal_str.len() - 1]
                    }
                    // unexpected (error elsewhere)
                    else {
                        literal_str
                    }
                };
                let string_id = self.strings.intern(content);
                Ok(ScalarLiteral::String(string_id))
            }

            // regex string literal (ignore quotes)
            LiteralType::RegexString { has_flags } => {
                // regex without flags
                if !has_flags {
                    let content = literal_str.trim_start_matches("/").trim_end_matches("/");
                    let string_id = self.strings.intern(content);
                    Ok(ScalarLiteral::RegexString {
                        content: string_id,
                        flags: None,
                    })
                }
                // regex with flags
                else {
                    let last_slash_index = literal_str.rfind('/').unwrap();
                    let content = &literal_str[1..last_slash_index];
                    let flags = &literal_str[last_slash_index + 1..];
                    let string_id = self.strings.intern(content);
                    let flags_id = self.strings.intern(flags);
                    Ok(ScalarLiteral::RegexString {
                        content: string_id,
                        flags: Some(flags_id),
                    })
                }
            }

            // tree text content (raw text inside tree literals, like JSX text)
            LiteralType::TreeString => {
                let string_id = self.strings.intern(literal_str);
                Ok(ScalarLiteral::String(string_id))
            }
        }
    }

    /// Parse an integer literal with saturation for values outside i64.
    fn parse_int_literal_saturating(&self, digits: &str, base: NumberBase) -> i64 {
        // select radix for the literal base
        let radix = match base {
            NumberBase::Decimal => 10,
            NumberBase::Binary => 2,
            NumberBase::Octal => 8,
            NumberBase::Hexadecimal => 16,
        };

        // accumulate digits with overflow detection
        let mut value: u128 = 0;
        let max_value = i64::MAX as u128;
        for ch in digits.chars() {
            let Some(digit) = ch.to_digit(radix) else {
                return i64::MAX;
            };
            let digit = digit as u128;
            let next = value
                .checked_mul(radix as u128)
                .and_then(|value| value.checked_add(digit));
            let Some(next) = next else {
                return i64::MAX;
            };
            if next > max_value {
                return i64::MAX;
            }
            value = next;
        }

        // finalize with saturation
        value as i64
    }

    /// Peek a template literal.
    #[inline]
    pub fn peek_template_literal(&self) -> ParseResult<&TokenSpan> {
        if self.peek_is(TokenType::TemplateString) || self.peek_is(TokenType::TemplateStringStart) {
            Ok(self.peek()?)
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a template literal. The path (i.e. tag) must be passed in explicitly.
    ///
    /// Examples:
    /// ```
    /// `hello`
    /// `hello ${name}`
    /// `SELECT * FROM users`
    /// `${stmt}`
    /// `SELECT * FROM users WHERE name = ${name}` AND age > ${group.age()} LIMIT 10`
    /// ```
    pub fn eat_template_literal(&mut self) -> ParseResult<TemplateLiteral> {
        let _timing = self.timing_scope(tags::PARSE_LITERAL);
        let (strings, arguments) =
            self.eat_template_literal_parts(|parser| parser.eat_template_literal_argument())?;

        if arguments.is_empty() && strings.len() == 1 {
            Ok(TemplateLiteral::String { string: strings[0] })
        } else {
            Ok(TemplateLiteral::InterpolatedString { strings, arguments })
        }
    }

    /// Eat a type template literal.
    ///
    /// Examples:
    /// ```
    /// `${K}`
    /// `foo-${Bar}`
    /// ```
    pub fn eat_type_template_literal_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_LITERAL);
        let start = self.mark();
        let (strings, spans) = self.eat_template_literal_parts(|parser| {
            parser.with_options(parser.options.not_in_position().in_type(), |parser| {
                parser.eat_expression()
            })
        })?;

        let expression = Expression::TypeTemplateLiteral { strings, spans };
        Ok(self.tree.insert(expression, self.get_span_from(start)))
    }

    /// Eat the parts of a template literal.
    fn eat_template_literal_parts<T>(
        &mut self,
        mut parse_span: impl FnMut(&mut Parser) -> ParseResult<T>,
    ) -> ParseResult<(Vec<StringId>, Vec<T>)> {
        let next = *self.eat()?;
        let next_str = self.file.span_str(next.span);

        // template string without interpolation
        if next.token.ty == TokenType::TemplateString {
            let string = next_str.trim_start_matches('`').trim_end_matches('`');
            let string_id = self.strings.intern(string);
            return Ok((vec![string_id], Vec::new()));
        }
        // template string with interpolation
        if next.token.ty == TokenType::TemplateStringStart {
            let mut strings: Vec<StringId> = Vec::new();
            let mut spans: Vec<T> = Vec::new();

            // start: remove ` prefix and ${ suffix
            let string = next_str
                .strip_prefix('`')
                .and_then(|s| s.strip_suffix("${"))
                .unwrap_or("");
            let string_id = self.strings.intern(string);
            strings.push(string_id);

            // eat until the end
            while !self.peek_is(TokenType::TemplateStringEnd) {
                // string
                if self.peek_is(TokenType::TemplateStringMiddle) {
                    let token = *self.eat()?;
                    let token_str = self.file.span_str(token.span);
                    // remove } prefix and ${ suffix
                    let string = token_str
                        .strip_prefix('}')
                        .and_then(|s| s.strip_suffix("${"))
                        .unwrap_or("");
                    let string_id = self.strings.intern(string);
                    strings.push(string_id);
                }
                // span
                else {
                    self.eat_newlines_maybe()?;
                    let span = parse_span(self)?;
                    self.eat_newlines_maybe()?;
                    if !self.peek_is(TokenType::TemplateStringMiddle)
                        && !self.peek_is(TokenType::TemplateStringEnd)
                    {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }
                    spans.push(span);
                }
            }

            // end: remove } prefix and ` suffix
            let token = *self.eat_token(TokenType::TemplateStringEnd)?;
            let token_str = self.file.span_str(token.span);
            let string = token_str
                .strip_prefix('}')
                .and_then(|s| s.strip_suffix('`'))
                .unwrap_or("");
            let string_id = self.strings.intern(string);
            strings.push(string_id);

            return Ok((strings, spans));
        }

        Err(ParseError::unexpected(next.span))
    }

    /// Eat a template literal interpolation argument.
    ///
    /// Template literal interpolations parse as full expressions (no named args).
    pub fn eat_template_literal_argument(&mut self) -> ParseResult<LocalNodeId<Argument>> {
        let start = self.mark();

        let value = self.with_options(
            self.options
                .not_in_position()
                .not_in_tree_literal()
                .not_in_left_precedence(),
            |parser| parser.eat_expression(),
        )?;

        let argument_id = self.tree.insert(
            Argument::Positional {
                modifiers: None,
                value,
            },
            self.get_span_from(start),
        );

        Ok(argument_id)
    }

    /// Eat an array literal (including the surrounding brackets).
    pub fn eat_array_literal(&mut self) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let _timing = self.timing_scope(tags::PARSE_LITERAL);
        self.eat_token(TokenType::OpenBracket)?;
        self.eat_newlines_maybe()?;
        let elements = if self.peek_is(TokenType::CloseBracket) {
            vec![]
        } else {
            let element_options = self.options.not_in_position().not_in_left_precedence();
            self.with_options(element_options, |parser| {
                parser.eat_sequence_literal_body(None, TokenType::CloseBracket)
            })?
        };
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseBracket)?;
        Ok(elements)
    }

    /// Eat the body of a sequence literal (excluding the surrounding parenthesis).
    /// Only positional and spread elements are allowed (no named elements).
    pub fn eat_sequence_literal_body(
        &mut self,
        first_element: Option<LocalNodeId<Argument>>,
        close_token: TokenType,
    ) -> ParseResult<Vec<LocalNodeId<Argument>>> {
        let mut elements = Vec::new();
        if let Some(first) = first_element {
            elements.push(first);
        }
        // track whether we expect an element (at start or after comma)
        let mut expect_element = first_element.is_none();
        while self.has_more_tokens() {
            // allow trailing newlines before the closing token
            if self.peek_is(TokenType::Newline)
                && self
                    .peek_token_after_newlines(self.pos(), close_token)
                    .is_ok()
            {
                break;
            }
            // stop at closing parenthesis (trailing commas are allowed, no hole)
            if self.peek_token_type() == close_token {
                break;
            }
            // consume separator (comma)
            else if self.peek_comma().is_ok() {
                let start = self.mark();
                // leading hole: if we expected an element but got comma instead
                if expect_element {
                    let stub = self
                        .tree
                        .insert(Expression::Stub, self.get_span_from(start));
                    let hole = self.tree.insert(
                        Argument::Positional {
                            modifiers: None,
                            value: stub,
                        },
                        self.get_span_from(start),
                    );
                    elements.push(hole);
                }
                self.eat_item_stop_with_newlines()?;
                expect_element = true;
                continue;
            }
            // keep eating elements (positional/spread only)
            let element = self
                .eat_positional_argument()
                .for_node_type(NodeType::Argument)?;
            elements.push(element);
            expect_element = false;
        }
        Ok(elements)
    }

    /// Eat an object literal (including the surrounding braces).
    ///
    /// Examples:
    /// ```
    /// { }
    /// { a: 1, b }
    /// { a(x): void }
    pub fn eat_object_literal(&mut self) -> ParseResult<Vec<LocalNodeId<Property>>> {
        let _timing = self.timing_scope(tags::PARSE_LITERAL);
        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;
        let properties = self.eat_properties()?;
        self.eat_token(TokenType::CloseBrace)?;
        Ok(properties)
    }

    /// Peek whether `<...>` starts a generic arrow in tree literal positions.
    pub(super) fn peek_tree_generic_arrow(&self) -> bool {
        // only disambiguate when tree literals are enabled
        if !self.language.supports_jsx() {
            return false;
        }

        // type or static contexts do not use tree literal parsing
        if self.options.in_type || self.options.in_static {
            return false;
        }

        // require JSX disambiguators for JS, but allow lenient parsing in TS/DS
        let require_tree_disambiguator = self.language.is_javascript();

        self.peek_generic_arrow_after_type_parameters(require_tree_disambiguator)
    }

    /// Peek whether `<...>(...)` forms a generic arrow function signature.
    fn peek_generic_arrow_after_type_parameters(&self, require_tree_disambiguator: bool) -> bool {
        // require `<` at the current position
        if self.peek_token(TokenType::LessThan).is_err() {
            return false;
        }

        // require an identifier in the type parameter list
        let has_identifier = self.peek_next_token(TokenType::Identifier).is_ok();

        // allow multiline identifiers in type context
        let has_multiline_identifier =
            if self.options.in_type && self.peek_next_token(TokenType::Newline).is_ok() {
                let mut pos = self.pos() as usize;
                while let Some(token) = self.tokens.get(pos + 1)
                    && token.token.ty == TokenType::Newline
                {
                    pos += 1;
                }
                self.tokens
                    .get(pos + 1)
                    .is_some_and(|token| token.token.ty == TokenType::Identifier)
            } else {
                false
            };
        if !has_identifier && !has_multiline_identifier {
            return false;
        }

        // find the closing `>` for the type parameter list and track TSX disambiguators
        let mut angle_depth: usize = if self.has_split_token(TokenType::LessThan) {
            1
        } else {
            0
        };
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut has_tree_disambiguator = false;
        let mut pos = self.pos() as usize;
        let mut close_pos = None;
        while let Some(token) = self.tokens.get(pos) {
            match token.token.ty {
                TokenType::LessThan => angle_depth += 1,
                TokenType::ShiftLeft | TokenType::SaturatingShiftLeft => angle_depth += 2,
                TokenType::GreaterThan => {
                    angle_depth = angle_depth.saturating_sub(1);
                    if angle_depth == 0 {
                        close_pos = Some(pos as u32);
                        break;
                    }
                }
                TokenType::OpenParenthesis => paren_depth += 1,
                TokenType::CloseParenthesis => paren_depth = paren_depth.saturating_sub(1),
                TokenType::OpenBracket => bracket_depth += 1,
                TokenType::CloseBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenType::OpenBrace => brace_depth += 1,
                TokenType::CloseBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenType::Comma | TokenType::Assign => {
                    if angle_depth == 1
                        && paren_depth == 0
                        && bracket_depth == 0
                        && brace_depth == 0
                    {
                        has_tree_disambiguator = true;
                    }
                }
                _ => {}
            }
            if angle_depth == 1
                && paren_depth == 0
                && bracket_depth == 0
                && brace_depth == 0
                && self.keyword_for_index(pos) == Some(Keyword::Extends)
            {
                has_tree_disambiguator = true;
            }
            pos += 1;
        }
        let Some(close_pos) = close_pos else {
            return false;
        };
        if require_tree_disambiguator && !has_tree_disambiguator {
            return false;
        }

        // skip newlines after the type parameters
        let after_close_pos = if self.options.in_type {
            let mut pos = close_pos as usize;
            while let Some(token) = self.tokens.get(pos + 1)
                && token.token.ty == TokenType::Newline
            {
                pos += 1;
            }
            pos as u32
        } else {
            close_pos
        };

        // require `(` after the type parameters
        let Some(after_close) = self.tokens.get(after_close_pos as usize + 1) else {
            return false;
        };
        if after_close.token.ty != TokenType::OpenParenthesis {
            return false;
        }

        // find the closing `)` for the parameters
        let Ok(parenthesis_close) = self.find_matching_close(
            Some(after_close_pos + 1),
            TokenType::OpenParenthesis,
            TokenType::CloseParenthesis,
        ) else {
            return false;
        };

        // skip newlines after the parameter list
        let after_parenthesis_pos = if self.options.in_type {
            let mut pos = parenthesis_close as usize;
            while let Some(token) = self.tokens.get(pos + 1)
                && token.token.ty == TokenType::Newline
            {
                pos += 1;
            }
            pos as u32
        } else {
            parenthesis_close
        };

        // require `:` or `=>` after the parameters
        let Some(after_parenthesis) = self.tokens.get(after_parenthesis_pos as usize + 1) else {
            return false;
        };
        matches!(
            after_parenthesis.token.ty,
            TokenType::Colon | TokenType::Arrow | TokenType::ArrowWide
        )
    }

    /// Peek a tree literal (including the `<` and `>` tokens).
    #[inline]
    pub fn peek_tree_literal(&self) -> ParseResult<()> {
        if !self.peek_is(TokenType::LessThan) {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // skip newlines after `<`
        let mut pos = self.pos() as usize;
        while let Some(token) = self.tokens.get(pos + 1)
            && token.token.ty == TokenType::Newline
        {
            pos += 1;
        }

        // next token after `<` (and newlines)
        let next = self
            .tokens
            .get(pos + 1)
            .ok_or(ParseError::unexpected(self.peek()?.span))?;
        // closing tags should only appear inside tree content
        if next.token.ty == TokenType::Divide && !self.options.in_tree_literal {
            return Err(ParseError::unexpected(self.peek()?.span));
        }
        if !matches!(
            next.token.ty,
            TokenType::GreaterThan | TokenType::Divide | TokenType::Identifier
        ) {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // exclude generic arrow function disambiguation: <T,>(...)
        if next.token.ty == TokenType::Identifier {
            let mut comma_pos = pos + 1;
            while let Some(token) = self.tokens.get(comma_pos + 1)
                && token.token.ty == TokenType::Newline
            {
                comma_pos += 1;
            }
            if let Some(token) = self.tokens.get(comma_pos + 1)
                && token.token.ty == TokenType::Comma
            {
                return Err(ParseError::unexpected(self.peek()?.span));
            }
        }

        Ok(())
    }

    /// Skip whitespace-only tree string tokens (TSX content whitespace).
    /// JSX semantics ignore whitespace-only text between elements (Babel/TypeScript behavior).
    /// See: https://github.com/facebook/jsx/issues/19
    fn skip_tree_whitespace(&mut self) -> ParseResult<bool> {
        let mut skipped = false;
        loop {
            let token = self.peek()?;
            // skip newlines
            if token.token.ty == TokenType::Newline {
                self.bump();
                skipped = true;
                continue;
            }
            // skip whitespace-only tree strings (entire tokens, not content inside strings)
            if token.token.ty == TokenType::Literal
                && token.token.literal == Some(LiteralType::TreeString)
            {
                let content = self.get_span_str(token.span);
                if content.trim().is_empty() {
                    self.bump();
                    skipped = true;
                    continue;
                }
            }
            break;
        }
        Ok(skipped)
    }

    /// Eat a tree literal (including the `<` and `>` tokens).
    ///
    /// Examples:
    /// ```
    /// <Entity />
    /// <Entity a=1 test />
    /// <Level level=1>
    ///     <Entity name="Alfred" />
    ///     <Entity>2</Entity>
    ///     "some text"
    ///     ..someChildren.map(child => <Entity name={child.name} />)
    /// </Level>
    /// ```
    pub fn eat_tree_literal(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_LITERAL);
        let start = self.mark();
        self.eat_token(TokenType::LessThan)?;
        self.eat_newlines_maybe()?;

        // left
        let mut path_name_span = None;
        let path: Option<Path> = if self.peek_is(TokenType::Identifier) {
            let (path, name_span) = self.eat_tree_literal_path_with_last_span()?;
            path_name_span = Some(name_span);
            Some(path)
        } else {
            None
        };
        self.eat_newlines_maybe()?;
        let header_start = self.mark();

        // static arguments on the tag (TypeScript/TSX generic JSX components)
        let static_arguments = if path.is_some()
            && (self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft))
        {
            Some(
                self.with_options(self.options.not_in_tree_literal(), |parser| {
                    parser.eat_static_arguments()
                })?,
            )
        } else {
            None
        };
        // header (arguments separated by `=`)
        let arguments: Option<Vec<LocalNodeId<Argument>>> = {
            // fragment without arguments
            if self.peek_is(TokenType::Divide) || self.peek_is(TokenType::GreaterThan) {
                None
            }
            // fragment with arguments
            else {
                let mut arguments: Vec<LocalNodeId<Argument>> = vec![];
                while !self.peek_is(TokenType::Divide) && !self.peek_is(TokenType::GreaterThan) {
                    let argument = self.with_options(
                        self.options.not_in_position().in_tree_literal(),
                        |parser| parser.eat_tree_literal_argument(),
                    )?;
                    arguments.push(argument);
                    self.eat_newlines_maybe()?;
                }
                Some(arguments)
            }
        };
        self.eat_newlines_maybe()?;

        // body (either />, or > with child elements)
        let elements: Option<Vec<LocalNodeId<Argument>>> = {
            // fragment without children (/>)
            if self.peek_is(TokenType::Divide) {
                self.bump(); // eat /
                self.eat_token(TokenType::GreaterThan)?; // eat >
                None
            }
            // fragment with children (>)
            else {
                self.eat_token(TokenType::GreaterThan)?; // eat >
                self.skip_tree_whitespace()?; // skip whitespace-only tree content

                // eat children until closing fragment
                let mut elements: Vec<LocalNodeId<Argument>> = vec![];
                let mut found_closing = false;
                while self.has_more_tokens() {
                    // skip whitespace before checking for closing tag
                    self.skip_tree_whitespace()?;

                    // stop at closing fragment (</)
                    if self.peek_is(TokenType::LessThan) && self.peek_next_is(TokenType::Divide) {
                        // close fragment for fragment literals
                        if path.is_none()
                            && self.peek_next_next_token(TokenType::GreaterThan).is_ok()
                        {
                            self.bump(); // eat <
                            self.bump(); // eat /
                            self.bump(); // eat >
                            found_closing = true;
                            break;
                        }

                        // fragment close is invalid for non fragment tags
                        if path.is_some()
                            && self.peek_next_next_token(TokenType::GreaterThan).is_ok()
                        {
                            return Err(ParseError::unexpected(self.peek()?.span));
                        }

                        // named closing tag is invalid for fragment literals
                        if path.is_none() {
                            return Err(ParseError::unexpected(self.peek()?.span));
                        }

                        // check if closing fragment has same path
                        if let Some(path) = &path {
                            self.bump(); // eat <
                            self.bump(); // eat /
                            let closing_path = self.eat_tree_literal_path()?;
                            if closing_path == *path {
                                self.eat_token(TokenType::GreaterThan)?;
                                found_closing = true;
                                break;
                            }
                            return Err(ParseError::unexpected(self.peek()?.span));
                        }
                    }

                    // keep eating child elements
                    // NOTE: uses statement position so {expr} parses as block (expression container)
                    let element = self.with_options(
                        self.options
                            .not_in_position()
                            .in_tree_literal()
                            .in_statement_position(),
                        |parser| parser.eat_tree_argument(),
                    )?;
                    elements.push(element);
                    self.skip_tree_whitespace()?; // skip whitespace-only tree content
                }

                if !found_closing {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                Some(elements)
            }
        };

        // tree literal
        let left = path.as_ref().map(|path| {
            let expression_id = self.tree.insert(
                Expression::Path {
                    path: path.clone(),
                    static_arguments: static_arguments.clone(),
                },
                self.get_span_between(start, header_start),
            );
            if let Some(name_span) = path_name_span {
                self.tree.set_main_span(expression_id, name_span);
            }
            expression_id
        });
        let expression = Expression::TreeExpression {
            left,
            arguments,
            elements,
        };
        Ok(self.tree.insert(expression, self.get_span_from(start)))
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, Expression, FloatType, IfCondition, IfKind, IntType, Name, ScalarLiteral,
        TemplateLiteral, TokenType, TypeBinaryOperator, TypeLiteral,
    };
    use destack_source::LanguageType;

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    /// Parse integer literals in various formats.
    #[test]
    fn test_parse_integer_literal() {
        let mut test = TestParser::new("1 731 0x1234 2n");
        let mut parser = test.prepare();

        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Integer(1)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Integer(731)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Integer(0x1234)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Bigint(2)
        );
    }

    /// Parse large integer literals without overflow errors.
    #[test]
    fn test_parse_integer_literal_saturating() {
        let mut test = TestParser::new("9999999999999999999999999 9999999999999999999999999n");
        let mut parser = test.prepare();

        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Integer(i64::MAX)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Bigint(i64::MAX)
        );
    }

    /// Parse scientific notation and decimal floats.2
    #[test]
    fn test_parse_float_literal() {
        let mut test = TestParser::new("10e37 1.0");
        let mut parser = test.prepare();

        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Float(1.0e38)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Float(1.0)
        );
    }

    /// Parse true and false literals.
    #[test]
    fn test_parse_boolean_literal() {
        let mut test = TestParser::new("true false");
        let mut parser = test.prepare();

        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Boolean(true)
        );
        assert_eq!(
            parser.eat_scalar_literal().unwrap(),
            ScalarLiteral::Boolean(false)
        );
    }

    /// Parse string literal.
    #[test]
    fn test_parse_string_literal() {
        let mut test = TestParser::new(r#""hello" 'hi there'"#);
        let mut parser = test.prepare();

        let literal = parser.eat_scalar_literal().unwrap();
        assert_string!(
            parser,
            match literal {
                ScalarLiteral::String(id) => id,
                other => panic!("expected string literal, got {other:?}"),
            },
            "hello"
        );

        let literal = parser.eat_scalar_literal().unwrap();
        assert_string!(
            parser,
            match literal {
                ScalarLiteral::String(id) => id,
                other => panic!("expected string literal, got {other:?}"),
            },
            "hi there"
        );
    }

    /// Parse a regex string literal.
    #[test]
    fn test_parse_regex_string_literal() {
        let mut test = TestParser::new("/abc/\n/abc/g");
        let mut parser = test.prepare();

        // /abc/
        let literal = parser.eat_scalar_literal().unwrap();
        match literal {
            ScalarLiteral::RegexString { content, flags } => {
                assert_string!(parser, content, "abc");
                assert!(flags.is_none());
            }
            other => panic!("expected regex string literal, got {other:?}"),
        }
        parser.eat_newline().unwrap();

        // /abc/g
        let literal = parser.eat_scalar_literal().unwrap();
        match literal {
            ScalarLiteral::RegexString { content, flags } => {
                assert_string!(parser, content, "abc");
                assert_string!(parser, flags.unwrap(), "g");
            }
            other => panic!("expected regex string literal, got {other:?}"),
        }
    }

    /// Parse a template string literal.
    #[test]
    fn test_parse_template_literal() {
        let mut test = TestParser::new(
            r#"
`hello`
`hello ${name}`
`${stmt}`
`${start}${middle}${end}`
`SELECT * FROM users WHERE name = ${name} AND age > ${group.age()} LIMIT 10`
"#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        // `hello`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::String { string: template } => {
                // hello
                assert_string!(parser, template, "hello");
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `hello ${name}`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(arguments.len(), 1);
                assert_eq!(strings.len(), 2);
                // hello
                assert_string!(parser, strings[0], "hello ");
                // name
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "name");
                });
                //
                assert_string!(parser, strings[1], "");
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `${stmt}`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(strings.len(), 2);
                assert_eq!(arguments.len(), 1);
                // empty start & empty end
                assert_string!(parser, strings[0], "");
                assert_string!(parser, strings[1], "");
                // stmt
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "stmt");
                });
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `${start}${middle}${end}`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(strings.len(), 4);
                assert_eq!(arguments.len(), 3);
                // empty string before & after each argument
                assert_string!(parser, strings[0], "");
                assert_string!(parser, strings[1], "");
                assert_string!(parser, strings[2], "");
                assert_string!(parser, strings[3], "");
                // start
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "start");
                });
                // middle
                assert_node!(parser.tree, arguments[1], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "middle");
                });
                // end
                assert_node!(parser.tree, arguments[2], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "end");
                });
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `SELECT * FROM users WHERE name = ${name} AND age > ${group.age()} LIMIT 10`
        let literal = parser.eat_template_literal().unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(arguments.len(), 2);
                assert_eq!(strings.len(), 3);
                // SELECT * FROM users WHERE name =
                assert_string!(parser, strings[0], "SELECT * FROM users WHERE name = ");
                // name
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "name");
                });
                // AND age >
                assert_string!(parser, strings[1], " AND age > ");
                // group.age()
                assert_node!(parser.tree, arguments[1], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Call { left, .. } => {
                        assert_expression_path!(parser, parser.tree.get(*left), "group.age");
                    });
                });
                // LIMIT 10
                assert_string!(parser, strings[2], " LIMIT 10");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn test_parse_type_literal() {
        let mut test = TestParser::new("int32 uint8 float boolean symbol unique symbol");
        let mut parser = test.prepare();
        parser.options.in_type = true;

        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Int(IntType::Arbitrary {
                width: Some(32),
                is_signed: true
            })
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Int(IntType::Arbitrary {
                width: Some(8),
                is_signed: false
            })
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Float(FloatType { width: None })
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Boolean
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::Symbol
        ));
        assert!(matches!(
            parser.eat_type_literal(None).unwrap(),
            TypeLiteral::UniqueSymbol
        ));
    }

    #[test]
    fn test_parse_array_literal() {
        let mut test = TestParser::new("[1, 2]");
        let mut parser = test.prepare();

        let elements = parser.eat_array_literal().unwrap();
        assert_eq!(elements.len(), 2);
        // 1
        assert_node!(
            parser.tree,
            elements[0],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            }
        );
        // 2
        assert_node!(
            parser.tree,
            elements[1],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            }
        );
    }

    #[test]
    fn test_parse_sparse_array_middle_hole() {
        let mut test = TestParser::new("[1, , 3]");
        let mut parser = test.prepare();

        let elements = parser.eat_array_literal().unwrap();
        assert_eq!(elements.len(), 3);
        // 1
        assert_node!(
            parser.tree,
            elements[0],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            }
        );
        // hole (stub)
        assert_node!(
            parser.tree,
            elements[1],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Stub);
            }
        );
        // 3
        assert_node!(
            parser.tree,
            elements[2],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            }
        );
    }

    #[test]
    fn test_parse_sparse_array_leading_hole() {
        let mut test = TestParser::new("[, 1]");
        let mut parser = test.prepare();

        let elements = parser.eat_array_literal().unwrap();
        assert_eq!(elements.len(), 2);
        // hole (stub)
        assert_node!(
            parser.tree,
            elements[0],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Stub);
            }
        );
        // 1
        assert_node!(
            parser.tree,
            elements[1],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            }
        );
    }

    #[test]
    fn test_parse_sparse_array_trailing_hole() {
        let mut test = TestParser::new("[1, ]");
        let mut parser = test.prepare();

        let elements = parser.eat_array_literal().unwrap();
        // trailing comma without hole is allowed
        assert_eq!(elements.len(), 1);
        // 1
        assert_node!(
            parser.tree,
            elements[0],
            Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            }
        );
    }

    #[test]
    fn test_parse_tree_fragment() {
        let mut test = TestParser::new("<A/>");
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        // <A/>
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            // A
            assert_expression_path!(parser, parser.tree.get(*left), "A");
            assert!(arguments.is_none());
            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_fragment_with_kebab_tag() {
        let mut test =
            TestParser::new_with_options("<amp-something />", LanguageType::TypeScriptXml);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        // <amp-something />
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "ampSomething");
            assert!(arguments.is_none());
            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_fragment_with_arguments() {
        // pure TSX: numeric values need {}, boolean flags are implicit true
        let mut test = TestParser::new("<A a={1} annoying-bee={2} c={3} flag />");
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "A");
            assert_eq!(arguments.as_ref().unwrap().len(), 4);
            // a={1}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            // annoying-bee={2}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "annoyingBee");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
            // c={3}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "c");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
            // flag (implicit true)
            assert_node!(parser.tree, arguments.as_ref().unwrap()[3], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "flag");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });

            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_fragment_with_arguments_and_child() {
        // pure TSX syntax
        let mut test = TestParser::new(
            r"
<Tooltip
    title={true}
    flag
    something-else={false}
>
    {true}
</Tooltip>
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Tooltip");
            assert_eq!(arguments.as_ref().unwrap().len(), 3);
            // title={true}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "title");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
            // flag (implicit true)
            assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "flag");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
            // something-else={false}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "somethingElse");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
            });

            assert!(elements.is_some());
            // {true} child expression
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
        });
    }

    #[test]
    fn test_parse_tree_nested_deep() {
        // pure TSX: children must be elements or {expression}
        let mut test = TestParser::new(
            r"
<A>
    <B>
        <C>
            <D/>
            {2}
        </C>
    </B>
</A>
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "A");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            // <B>
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "B");
                    assert!(arguments.is_none());
                    assert!(elements.is_some());
                    // <C>
                    assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                            assert_expression_path!(parser, parser.tree.get(*left), "C");
                            assert!(arguments.is_none());
                            assert!(elements.is_some());
                            // <D/>
                            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                                    assert_expression_path!(parser, parser.tree.get(*left), "D");
                                    assert!(arguments.is_none());
                                    assert!(elements.is_none());
                                });
                            });
                            // {2}
                            assert_node!(parser.tree, elements.as_ref().unwrap()[1], Argument::Positional { modifiers: _, value } => {
                                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                            });
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_tree_in_parenthesis() {
        let mut test = TestParser::new(
            r#"
(
    <div className="font-semibold">
        <Link subtle to={1}>
            {2}
        </Link>
    </div>
)
        "#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression, Expression::Parenthesized { expression } => {
            // <div className="font-semibold">
            assert_node!(parser.tree, *expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                assert_expression_path!(parser, parser.tree.get(*left), "div");
                assert!(arguments.is_some());
                assert_eq!(arguments.as_ref().unwrap().len(), 1);
                // className="font-semibold"
                assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                    // className
                    assert_string!(parser, *name, "className");
                    // font-semibold
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, "font-semibold");
                    });
                });

                assert!(elements.is_some());
                assert_eq!(elements.as_ref().unwrap().len(), 1);
                // <Link subtle to={1}>
                assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(left), arguments, elements } => {
                        // Link
                        assert_expression_path!(parser, parser.tree.get(*left), "Link");
                        assert!(arguments.is_some());
                        assert_eq!(arguments.as_ref().unwrap().len(), 2);
                        // subtle
                        assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                            assert_string!(parser, *name, "subtle");
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
                        });
                        // to={1} - TSX: {} is expression container, value is just 1
                        assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                            assert_string!(parser, *name, "to");
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                        });

                        assert!(elements.is_some());
                        assert_eq!(elements.as_ref().unwrap().len(), 1);
                        // {2} - TSX: {} is expression container, value is just 2
                        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                        });
                    });
                });
            });
        });
    }

    /// Parse tree literal with static arguments on the tag.
    #[test]
    fn test_parse_tree_with_static_arguments() {
        let mut test = TestParser::new_with_options(
            r#"<Component<any>></Component>"#,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "Component");
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Any));
                });
            });
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 0);
        });
    }

    /// Parse tree literal with bit-shift-like static arguments on the tag.
    #[test]
    fn test_parse_tree_with_shift_left_static_arguments() {
        let mut static_test =
            TestParser::new_with_options(r#"<<T>(v: T) => void>"#, LanguageType::TypeScriptXml);
        let mut static_parser = static_test.prepare();
        let static_arguments = static_parser.eat_static_arguments().unwrap();
        let static_diagnostics = static_parser.diagnostics.drain();
        assert!(
            static_diagnostics.is_empty(),
            "unexpected static diagnostics: {static_diagnostics:?}"
        );
        assert_eq!(static_arguments.len(), 1);

        let mut test = TestParser::new_with_options(
            r#"<Component<<T>(v: T) => void> />"#,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let mut direct_test = TestParser::new_with_options(
            r#"<Component<<T>(v: T) => void> />"#,
            LanguageType::TypeScriptXml,
        );
        let mut direct_parser = direct_test.prepare();
        assert!(direct_parser.peek_tree_literal().is_ok());
        let direct_expression = direct_parser.eat_tree_literal().unwrap();
        let direct_diagnostics = direct_parser.diagnostics.drain();
        assert!(
            direct_diagnostics.is_empty(),
            "unexpected direct diagnostics: {direct_diagnostics:?}"
        );
        assert_node!(
            direct_parser.tree,
            direct_expression,
            Expression::TreeExpression { .. }
        );
        assert!(parser.peek_tree_literal().is_ok());
        let shift_token = parser
            .tokens
            .iter()
            .find(|token| token.span.start == 10)
            .expect("expected shift-left token");
        assert_eq!(shift_token.token.ty, TokenType::ShiftLeft);

        let expressions = parser.parse();
        let diagnostics = parser.diagnostics.drain();
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics: {diagnostics:?}"
        );
        assert!(!expressions.is_empty());
        let expression = match parser.tree.get(expressions[0]) {
            Expression::Statement(expression) => *expression,
            _ => expressions[0],
        };
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "Component");
                let static_arguments = static_arguments.as_ref().expect("expected static arguments");
                assert_eq!(static_arguments.len(), 1);
            });
            assert!(arguments.is_none());
            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_with_text_content() {
        let mut test = TestParser::new(r#"<h4>Tool: {part.toolName}</h4>"#);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "h4");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 2);
            // Tool:
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(_)));
            });
            // {part.toolName}
            assert!(matches!(
                parser.tree.get(elements.as_ref().unwrap()[1]),
                Argument::Positional { .. }
            ));
        });
    }

    #[test]
    fn test_parse_nested_tree_with_text_content() {
        // <div><h4>Tool: {x}</h4></div>
        let mut test = TestParser::new(r#"<div><h4>Tool: {x}</h4></div>"#);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 1);
            // nested <h4>...</h4>
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::TreeExpression { .. });
            });
        });
    }

    #[test]
    fn test_parse_tree_with_attribute_and_children() {
        // <div key={index}><h4>Tool: {x}</h4></div>
        let mut test = TestParser::new(r#"<div key={index}><h4>Tool: {x}</h4></div>"#);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            assert!(arguments.is_some());
            assert_eq!(arguments.as_ref().unwrap().len(), 1);
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 1);
        });
    }

    /// Tree fragment containing a callback that returns nested tree literals.
    #[test]
    fn test_parse_tree_fragment_with_nested_callback() {
        let mut test = TestParser::new(r#"<>{x.map(() => (<div><h4>T: {y}</h4></div>))}</>"#);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: None, arguments, elements } => {
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 1);
        });
    }

    /// Tree literal with JSX comment syntax {/* */}.
    /// The comment is filtered out, leaving an empty expression container.
    #[test]
    fn test_parse_tree_with_comment_container() {
        let mut test = TestParser::new(r#"<div>{/* comment */}</div>"#);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(_), arguments, elements } => {
            assert!(arguments.is_none());
            assert!(elements.is_some());
        });
    }

    /// Parse tree fragment with comments between the angle brackets.
    #[test]
    fn test_parse_tree_fragment_with_comments() {
        let mut test = TestParser::new_with_options(
            "<\n// comment\n/* comment */\n>\n</>",
            LanguageType::JavaScriptXml,
        );
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: None, arguments, elements } => {
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert!(elements.as_ref().unwrap().is_empty());
        });
    }

    /// Parse tree literal with namespace tag and attribute.
    #[test]
    fn test_parse_tree_with_namespace_tag() {
        let mut test =
            TestParser::new_with_options(r#"<Foo:Bar n:foo="bar" />"#, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Foo:Bar");
            let arguments = arguments.as_ref().expect("expected arguments");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "n:foo");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(_)));
            });
            assert!(elements.is_none());
        });
    }

    /// Ternary with tree literal containing && inside expression container.
    /// Regression test for fresh_expression() fix - ensures left_precedence is cleared.
    #[test]
    fn test_parse_ternary_with_and_in_tree() {
        // Just the ternary part, without leading condition
        let mut test = TestParser::new(r#"a ? <>{y && <E />}</> : null"#);
        let mut parser = test.prepare();
        let expr = parser.eat_expression().unwrap();
        // a ? ... : null -> If with IfKind::Ternary
        assert_node!(parser.tree, expr, Expression::If { kind, condition, then_expression, else_expression } => {
            assert_eq!(*kind, IfKind::Ternary);
            // condition: a
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::Path { .. });
            // consequence: <>{y && <E />}</>
            assert_node!(parser.tree, *then_expression, Expression::TreeExpression { left: None, arguments, elements } => {
                assert!(arguments.is_none());
                assert!(elements.is_some());
                assert_eq!(elements.as_ref().unwrap().len(), 1);
                // {y && <E />} - the && expression
                assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Binary { .. });
                });
            });
            // alternative: null
            assert!(else_expression.is_some());
            assert_node!(parser.tree, else_expression.unwrap(), Expression::TypeLiteral(TypeLiteral::Null));
        });
    }

    /// Nested tree literal in attribute expression container.
    /// Regression test for from_content fix in TreeExpressionEntry.
    #[test]
    fn test_parse_nested_tree_in_attribute() {
        let mut test = TestParser::new(r#"<Button icon={<Icon />} />"#);
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Button");
            assert!(arguments.is_some());
            assert_eq!(arguments.as_ref().unwrap().len(), 1);
            // icon={<Icon />}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "icon");
                // value is <Icon />
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(inner_left), arguments: inner_args, elements: inner_elems } => {
                    assert_expression_path!(parser, parser.tree.get(*inner_left), "Icon");
                    assert!(inner_args.is_none());
                    assert!(inner_elems.is_none());
                });
            });
            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_attribute_spread_with_cast() {
        let mut test = TestParser::new_with_options(
            r#"<WrappedComponent {...(this.props as P & DependentProps)} {...this.state} />"#,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "WrappedComponent");
            let arguments = arguments.as_ref().expect("expected arguments");
            assert_eq!(arguments.len(), 2);
            // {...(this.props as P & DependentProps)}
            assert_node!(parser.tree, arguments[0], Argument::Spread { modifiers: _, label: None, value } => {
                assert_node!(parser.tree, *value, Expression::Parenthesized { expression } => {
                    assert_node!(parser.tree, *expression, Expression::TypeBinary { operator, .. } => {
                        assert_eq!(*operator, TypeBinaryOperator::Cast);
                    });
                });
            });
            // {...this.state}
            assert_node!(parser.tree, arguments[1], Argument::Spread { modifiers: _, label: None, value } => {
                assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                    assert_node!(parser.tree, *left, Expression::This);
                    assert_string!(parser, *name, "state");
                });
            });
            assert!(elements.is_none());
        });
    }

    /// Deeply nested tree literals in attributes.
    /// Regression test for from_content fix with multiple nesting levels.
    #[test]
    fn test_parse_deeply_nested_tree_in_attr() {
        let mut test =
            TestParser::new(r#"<Outer title={<div><Button icon={<Icon />} /></div>} />"#);
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Outer");
            assert!(arguments.is_some());
            assert_eq!(arguments.as_ref().unwrap().len(), 1);
            // title={<div>...</div>}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "title");
                // <div><Button icon={<Icon />} /></div>
                assert_node!(parser.tree, *value, Expression::TreeExpression { elements: div_elems, .. } => {
                    assert!(div_elems.is_some());
                    assert_eq!(div_elems.as_ref().unwrap().len(), 1);
                });
            });
            assert!(elements.is_none());
        });
    }

    /// Object literal inside attribute expression container.
    #[test]
    fn test_parse_tree_attr_object_literal() {
        let mut test = TestParser::new(r#"<Rive style={{width: 400, height: 400}} />"#);
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Rive");
            assert!(arguments.is_some());
            assert_eq!(arguments.as_ref().unwrap().len(), 1);
            // style={{width: 400, height: 400}}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "style");
                // {width: 400, height: 400}
                assert_node!(parser.tree, *value, Expression::ObjectExpression { .. });
            });
            assert!(elements.is_none());
        });
    }

    /// Multiline tree literal with expression container and sibling elements.
    #[test]
    fn test_parse_multiline_tree_with_siblings() {
        let code = "<div>\n\t{x}\n\t<form onClick={() => {}}></form>\n</div>";
        let mut test = TestParser::new(code);
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 2);
            // {x}
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Path { .. });
            });
            // <form onClick={() => {}}></form>
            assert_node!(parser.tree, elements.as_ref().unwrap()[1], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::TreeExpression { left: Some(form_left), arguments: form_args, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*form_left), "form");
                    assert!(form_args.is_some());
                    assert_eq!(form_args.as_ref().unwrap().len(), 1);
                });
            });
        });
    }

    /// Logical && pattern inside tree content.
    #[test]
    fn test_parse_tree_with_logical_and() {
        let mut test = TestParser::new(r#"<div>{x && <span/>}</div>"#);
        let mut parser = test.prepare();
        let expr = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expr, Expression::TreeExpression { left: Some(left), arguments, elements } => {
            assert_expression_path!(parser, parser.tree.get(*left), "div");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            assert_eq!(elements.as_ref().unwrap().len(), 1);
            // {x && <span/>}
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Binary { left: bin_left, right: bin_right, .. } => {
                    // x
                    assert_node!(parser.tree, *bin_left, Expression::Path { .. });
                    // <span/>
                    assert_node!(parser.tree, *bin_right, Expression::TreeExpression { .. });
                });
            });
        });
    }

    /// Tree literal should parse after a closing class block on a new line.
    #[test]
    fn test_parse_tree_after_class_block_newline() {
        let mut test = TestParser::new_with_options(
            "class C extends D<T> {}\n<C/>",
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();

        // class declaration
        let class_expr = parser.try_eat_statement_expression().unwrap();
        assert_node!(parser.tree, class_expr, Expression::Declaration(_));

        // statement boundary
        parser.eat_statement_stop_with_newlines().unwrap();

        // tree literal expression
        let tree_expr = parser.try_eat_statement_expression().unwrap();
        assert_node!(parser.tree, tree_expr, Expression::TreeExpression { .. });
    }

    /// Valid template literal with interpolation should parse correctly.
    #[test]
    fn test_parse_template_literal_valid() {
        let mut test = TestParser::new("`hello ${name}!`");
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Valid template literal without interpolation.
    #[test]
    fn test_parse_template_literal_plain() {
        let mut test = TestParser::new("`hello world`");
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Empty template literal.
    #[test]
    fn test_parse_template_literal_empty() {
        let mut test = TestParser::new("``");
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Template literal with only interpolation `${foo}`.
    #[test]
    fn test_parse_template_literal_only_interpolation() {
        let mut test = TestParser::new("`${foo}`");
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Template literal with adjacent interpolations.
    #[test]
    fn test_parse_template_literal_adjacent_interpolations() {
        let mut test = TestParser::new("`${a}${b}${c}`");
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Template literal interpolation should allow optional chaining.
    #[test]
    fn test_parse_template_literal_optional_chain() {
        let mut test = TestParser::new(r#"`value ${theme?.activeColor}`"#);
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Template literal interpolation should allow ternary expressions.
    #[test]
    fn test_parse_template_literal_ternary() {
        let mut test = TestParser::new(r#"`value ${mode === "dark" ? "dark" : "light"}`"#);
        let mut parser = test.prepare();
        let result = parser.eat_template_literal();
        assert!(result.is_ok());
    }

    /// Parse JSX text that includes `=` and `>=` after opening tags.
    #[test]
    fn test_parse_jsx_text_with_equals() {
        let input = r#"
<div className={styles.foo}>=</div>;
<div className={styles.foo} >=</div>;
<div>=</div>;
<div >=</div>;
"#;
        let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let _ = parser.parse();
        assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    }

    /// Parse JSX fragments containing text and comparisons after closing tags.
    #[test]
    fn test_parse_jsx_fragment_with_equals() {
        let input = r#"
<>=x</>;
<>x</>>=1;
<span>=x</span>;
<span>x</span>>=1;
"#;
        let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let _ = parser.parse();
        assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    }

    /// Parse JSX fragments with text between child elements in arrays.
    #[test]
    fn test_parse_jsx_fragment_equals_in_array() {
        let input = r#"
function Test() {
    return (
        <Y
            elements={[
                <>
                    <span>x</span>=
                    <br />
                </>,
                true
            ]}
        />
    );
}
"#;
        let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let _ = parser.parse();
        assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    }

    /// Parse JSX elements after newline-terminated let and var declarations.
    #[test]
    fn test_parse_jsx_after_let_newline() {
        let input = r#"
let x
<Comp></Comp>

let x

<Comp></Comp>

let x;
<Comp></Comp>

var x
<Comp></Comp>

var x;
<Comp></Comp>

function x() {
    let x
    <div />
}

{ foo: 'test' }
<Comp></Comp>

function test1() {}
<Comp></Comp>

class Foo {}
<>
<Comp></Comp>
<Comp></Comp>
</>
"#;
        let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let _ = parser.parse();
        assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    }

    /// Parse JSX elements after return with newline termination.
    #[test]
    fn test_parse_jsx_after_return_newline() {
        let input = r#"
function test() {
    return
    <Comp />
}
"#;
        let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let _ = parser.parse();
        assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    }

    /// Reject JSX-looking input when it continues an expression across a newline.
    #[test]
    fn test_parse_jsx_after_expression_newline_is_error() {
        let input = r#"
x
<Comp />
"#;
        let mut test = TestParser::new_with_options(input, LanguageType::JavaScriptXml);
        let mut parser = test.prepare();
        let _ = parser.parse();
        assert!(!parser.errors.is_empty(), "expected parse errors");
    }
}
