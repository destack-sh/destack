use std::borrow::Cow;

use dyst_ast::{
    BindingKind, BindingModifiers, Definition, DefinitionMeta, FunctionStyle, Keyword, Path,
    StringId, TemplateLiteral,
};
use std::str::FromStr;

use crate::lex::decode_html_entity;
use crate::parse::prelude::*;
use crate::parse::variant::BINDING_MODIFIERS;
use crate::{
    Argument, Expression, LiteralType, NodeId, NodeType, NumberBase, Parser, ParserError,
    ParserResult, ScalarLiteral, TokenSpan, TokenType,
};

impl<'a> Parser<'a> {
    /// Peek a scalar literal token.
    #[inline]
    pub fn peek_scalar_literal(&self) -> ParserResult<&TokenSpan> {
        if self.peek_token(TokenType::Literal).is_ok() {
            Ok(self.peek()?)
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
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
    pub fn eat_scalar_literal(&mut self) -> ParserResult<ScalarLiteral> {
        let literal_span = *self.eat()?;
        let Some(body) = literal_span.token.literal else {
            return Err(ParserError::unexpected(literal_span.span));
        };
        let literal_str = self.get_span_str(literal_span.span);

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
                    return Err(ParserError::expected_for(
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
                let parsed = match base {
                    NumberBase::Decimal => content.parse::<i64>(),
                    NumberBase::Binary => i64::from_str_radix(content.trim_start_matches("0b"), 2),
                    NumberBase::Octal => i64::from_str_radix(content.trim_start_matches("0o"), 8),
                    NumberBase::Hexadecimal => {
                        i64::from_str_radix(content.trim_start_matches("0x"), 16)
                    }
                };

                // int
                if is_bigint {
                    parsed.map(ScalarLiteral::Bigint).map_err(|_| {
                        ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
                } else {
                    parsed.map(ScalarLiteral::Integer).map_err(|_| {
                        ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
                }
            }

            // float literal
            LiteralType::Float {
                base: _,
                is_empty_exponent,
            } => {
                if is_empty_exponent {
                    return Err(ParserError::expected_for(
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
                        ParserError::expected_for(
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
                            ParserError::expected_for(
                                literal_span.span,
                                TokenType::Literal,
                                NodeType::Expression,
                            )
                        })
                } else {
                    if !is_terminated {
                        return Err(ParserError::expected_for(
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
                            ParserError::expected_for(
                                literal_span.span,
                                TokenType::Literal,
                                NodeType::Expression,
                            )
                        })
                }
            }

            // byte character literal (ignore quotes)
            LiteralType::Byte { is_terminated } => {
                if !is_terminated {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }
                let content = literal_str.trim_start_matches("b'").trim_end_matches('\'');
                content
                    .chars()
                    .next()
                    .map(|ch| ScalarLiteral::Byte(ch as u8))
                    .ok_or_else(|| {
                        ParserError::expected_for(
                            literal_span.span,
                            TokenType::Literal,
                            NodeType::Expression,
                        )
                    })
            }

            // string literal (ignore quotes)
            // supports both '...' and "..." delimited string literals
            LiteralType::String { is_terminated } => {
                if !is_terminated {
                    return Err(ParserError::expected_for(
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
                let string_id = self.intern_string(content);
                Ok(ScalarLiteral::String(string_id))
            }

            // regex string literal (ignore quotes)
            LiteralType::RegexString { has_flags } => {
                // regex without flags
                if !has_flags {
                    let content = literal_str.trim_start_matches("/").trim_end_matches("/");
                    let string_id = self.intern_string(content);
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
                    let string_id = self.intern_string(content);
                    let flags_id = self.intern_string(flags);
                    Ok(ScalarLiteral::RegexString {
                        content: string_id,
                        flags: Some(flags_id),
                    })
                }
            }

            // raw string literal (ignore quotes and hashes)
            LiteralType::RawString { hashes } => {
                let Some(hashes) = hashes else {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                };
                let num_hashes = hashes as usize;
                let prefix_len = 1 /* r */ + num_hashes + 1 /* opening " */;
                let suffix_len = 1 /* closing " */ + num_hashes;
                if literal_str.len() < prefix_len + suffix_len {
                    return Err(ParserError::expected(literal_span.span, TokenType::Literal));
                }
                let content = &literal_str[prefix_len..literal_str.len() - suffix_len];
                let string_id = self.intern_string(content);
                Ok(ScalarLiteral::String(string_id))
            }

            // byte string literal (ignore quotes)
            LiteralType::ByteString { is_terminated } => {
                if !is_terminated {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }
                let content = literal_str.trim_start_matches("b\"").trim_end_matches('"');
                Ok(ScalarLiteral::ByteString(content.as_bytes().to_vec()))
            }

            // raw byte string literal (ignore quotes and hashes)
            LiteralType::RawByteString { hashes } => {
                let Some(hashes) = hashes else {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                };
                let num_hashes = hashes as usize;
                let prefix_len = 2 /* br */ + num_hashes + 1 /* opening " */;
                let suffix_len = 1 /* closing " */ + num_hashes;
                if literal_str.len() < prefix_len + suffix_len {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }
                let content = &literal_str[prefix_len..literal_str.len() - suffix_len];
                Ok(ScalarLiteral::ByteString(content.as_bytes().to_vec()))
            }
        }
    }

    /// Peek a template literal.
    #[inline]
    pub fn peek_template_literal(&self) -> ParserResult<&TokenSpan> {
        if self.peek_token(TokenType::TemplateString).is_ok()
            || self.peek_token(TokenType::TemplateStringStart).is_ok()
        {
            Ok(self.peek()?)
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Eat a template literal. The path (i.e. tag) must be passed in explicitly.
    ///
    /// Examples:
    /// ```
    /// `hello`
    /// `hello ${name}`
    /// sql`SELECT * FROM users`
    /// sql`${stmt}`
    /// sql.expr`SELECT * FROM users WHERE name = ${name}` AND age > ${group.age()} LIMIT 10`
    /// ```
    pub fn eat_template_literal(&mut self, tag: Option<Path>) -> ParserResult<TemplateLiteral> {
        let next = *self.eat()?;
        let next_str = self.get_span_str(next.span);

        // template string without interpolation
        if next.token.ty == TokenType::TemplateString {
            let string = next_str.trim_start_matches('`').trim_end_matches('`');
            let string_id = self.intern_string(string);
            // tagged template string
            if let Some(tag) = tag {
                Ok(TemplateLiteral::TaggedString {
                    tag,
                    string: string_id,
                })
            }
            // plain template string
            else {
                Ok(TemplateLiteral::String { string: string_id })
            }
        }
        // template string with interpolation
        else if next.token.ty == TokenType::TemplateStringStart {
            let mut strings: Vec<StringId> = Vec::new();
            let mut arguments: Vec<NodeId<Argument>> = Vec::new();

            // start
            let string = &next_str[1..next_str.len() - 2]; // remove ` and ${
            let string_id = self.intern_string(string);
            strings.push(string_id);

            // eat until the end
            while self.peek_token(TokenType::TemplateStringEnd).is_err() {
                // string
                if self.peek_token(TokenType::TemplateStringMiddle).is_ok() {
                    let token = *self.eat()?;
                    let string = self.get_span_str(token.span);
                    let string = &string[1..string.len() - 2]; // remove } and ${
                    let string_id = self.intern_string(string);
                    strings.push(string_id);
                }
                // argument
                else {
                    self.eat_newlines_maybe()?;
                    let argument = self.eat_argument()?;
                    self.eat_newlines_maybe()?;
                    arguments.push(argument);
                }
            }

            // end
            let token = *self.eat_token(TokenType::TemplateStringEnd)?;
            let string = self.get_span_str(token.span);
            let string = &string[1..string.len() - 1]; // remove } and `
            let string_id = self.intern_string(string);
            strings.push(string_id);

            if let Some(tag) = tag {
                Ok(TemplateLiteral::TaggedInterpolatedString {
                    tag,
                    strings,
                    arguments,
                })
            } else {
                Ok(TemplateLiteral::InterpolatedString { strings, arguments })
            }
        }
        // error
        else {
            Err(ParserError::unexpected(next.span))
        }
    }

    /// Eat an array literal (including the surrounding brackets).
    pub fn eat_array_literal(&mut self) -> ParserResult<Vec<NodeId<Argument>>> {
        self.eat_token(TokenType::OpenBracket)?;
        self.eat_newlines_maybe()?;
        let elements = if self.peek_token(TokenType::CloseBracket).is_ok() {
            vec![]
        } else {
            self.with_options(self.options.not_in_parenthesis(), |parser| {
                parser.eat_sequence_literal_body(None, TokenType::CloseBracket)
            })?
        };
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseBracket)?;
        Ok(elements)
    }

    /// Eat the body of a sequence literal (excluding the surrounding parenthesis).
    pub fn eat_sequence_literal_body(
        &mut self,
        first_element: Option<NodeId<Argument>>,
        close_token: TokenType,
    ) -> ParserResult<Vec<NodeId<Argument>>> {
        let mut elements = Vec::new();
        if let Some(first) = first_element {
            elements.push(first);
        }
        loop {
            // stop at closing parenthesis
            if self.peek_token(close_token).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
                continue;
            }
            // keep eating elements
            let element = self.eat_argument().for_node_type(NodeType::Argument)?;
            elements.push(element);
        }
        Ok(elements)
    }

    /// Peek an anomymous struct literal (like `{}`, `{ x: 0, y }` or `{ ..a }`):
    ///  - Accepts `{ func() { .. } }` (function shorthand)
    ///  - Accepts `{ x, y }` or  `{ x: x }`
    ///  - Forbids `{ x }` (in favor of blocks expressions)
    ///  - Newlines after first `{` are skipped.
    pub fn peek_anonymous_struct_literal_body(&mut self) -> ParserResult<Option<NodeId<Argument>>> {
        if self.peek_token(TokenType::OpenBrace).is_ok() {
            let mut pos = (self.pos() + 1) as usize;

            // skip any newlines or modifiers
            while let Some(token) = self.tokens.get(pos)
                && (token.token.ty == TokenType::Newline
                    || token.token.ty == TokenType::Tag // (tag is a visibility modifier for #Compatibility)
                    || Keyword::from_str(self.get_token_str(*token))
                        .map(|keyword| BINDING_MODIFIERS.contains(&keyword))
                        .unwrap_or(false))
            {
                pos += 1;
            }

            // empty struct literal (if not expecting a block)
            if !self.options.in_before_block
                && !self.options.in_block_slot
                && self.tokens.get(pos).map(|token| token.token.ty) == Some(TokenType::CloseBrace)
            {
                return Ok(None);
            }

            // struct literal field
            if pos + 3 >= self.tokens.len() {
                return Err(ParserError::unexpected(self.peek()?.span));
            }
            let token_ty = self.tokens[pos].token.ty;
            let next_token_ty = self.tokens[pos + 1].token.ty;
            let next_next_token_ty = self.tokens[pos + 2].token.ty;
            match (token_ty, next_token_ty, next_next_token_ty) {
                    (TokenType::Identifier, TokenType::Colon, _)
                    // identifier?:
                    | (TokenType::Identifier, TokenType::Maybe, TokenType::Colon)
                    // identifier,
                    | (TokenType::Identifier, TokenType::Comma, _)
                    // string:
                    | (TokenType::Literal, TokenType::Colon, _)
                    // string?:
                    | (TokenType::Literal, TokenType::Maybe, TokenType::Colon)
                    // ..T
                    | (TokenType::Range, TokenType::Identifier, _)
                    // ...T
                    | (TokenType::Spread, TokenType::Identifier, _) => {
                        return Ok(None);
                    }
                    // [
                    | (TokenType::OpenBracket, _, _) => {
                        // only if the closing bracket is followed by a colon
                        if let Ok(closing_pos) = self.find_open_and_matching_close(TokenType::OpenBracket, TokenType::CloseBracket)
                            && let Some(token_after) = self.tokens.get(closing_pos as usize + 1) && token_after.token.ty == TokenType::Colon {
                                return Ok(None);
                            }
                    }
                    _ => {}
                }

            // function shorthand
            if token_ty == TokenType::Identifier
                && (next_token_ty == TokenType::OpenParenthesis
                    || next_token_ty == TokenType::Maybe
                        && next_next_token_ty == TokenType::OpenParenthesis
                    || next_token_ty == TokenType::LessThan
                    || next_token_ty == TokenType::Maybe
                        && next_next_token_ty == TokenType::LessThan)
            {
                // speculatively parse function definition
                // (since we can't just count bracket pairs here)
                let speculative_start = (self.mark(), self.tree.next_id());
                self.eat_token(TokenType::OpenBrace).expect("peeked");
                self.eat_newlines_maybe().expect("peeked");

                // function
                let start = self.mark();
                debug_assert!(self.peek_token(TokenType::Identifier).is_ok());
                let is_maybe = self.peek_next_token(TokenType::Maybe).is_ok();
                let expect_body = !self.options.in_type;
                let Ok(function_id) =
                    self.eat_function(DefinitionMeta::default(), is_maybe, expect_body)
                else {
                    self.restore(speculative_start.0, speculative_start.1);
                    return Err(ParserError::unexpected(self.peek()?.span));
                };
                // name
                let name = self
                    .tree
                    .get(function_id)
                    .name()
                    .ok_or(ParserError::unexpected(self.get_span_from(start)))?;
                // value
                let value = self.tree.insert(
                    Expression::Definition(function_id),
                    self.get_span_from(start),
                );
                // clear function name
                match self.tree.get_mut(function_id) {
                    Definition::Function {
                        meta: DefinitionMeta { name, .. },
                        style,
                        ..
                    } => {
                        *name = None;
                        *style = FunctionStyle::Lambda;
                    }
                    _ => panic!("expected function for"),
                };
                // maybe
                let modifiers = if is_maybe {
                    Some(BindingModifiers {
                        kind: Some(BindingKind::Maybe),
                        ..BindingModifiers::default()
                    })
                } else {
                    None
                };
                return Ok(Some(self.tree.insert(
                    Argument::Function {
                        modifiers,
                        name,
                        value,
                    },
                    self.get_span_from(start),
                )));
            }
        }

        Err(ParserError::unexpected(self.peek()?.span))
    }

    /// Eat the body of a struct literal (including the `{` and `}`, without a prefix).
    pub(crate) fn eat_struct_literal_body(
        &mut self,
        first_argument: Option<NodeId<Argument>>,
    ) -> ParserResult<Vec<NodeId<Argument>>> {
        // (skip the opening sequence if we're given first argument from a speculative parse)
        if first_argument.is_none() {
            self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
                .for_node_type(NodeType::Expression)?;
            self.eat_newlines_maybe()?;

            // empty struct
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                self.eat_token(TokenType::CloseBrace)
                    .for_node_type(NodeType::Expression)?;
                return Ok(vec![]);
            }
        } else {
            // eat any stop if first argument is already given
            if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
        }

        let mut arguments = Vec::new();
        if let Some(first) = first_argument {
            arguments.push(first);
        }
        loop {
            // stop at closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }

            // NOTE: struct literal arguments are different from regular arguments
            //  (so we re-extension some of the argument parsing logic here,
            //   because we need to account for readonly/maybe/functions/...)
            let start = self.mark();
            let argument_id = {
                // modifiers
                let mut modifiers = self.eat_binding_modifiers_prefix_maybe()?;

                // named argument
                if self.peek_name().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
                    // name
                    let name = self.eat_name()?;
                    self.bump(); // eat colon
                    self.eat_newlines_maybe()?;
                    // value
                    let value =
                        self.with_options(self.options.nested(), |parser| parser.eat_expression())?;
                    self.tree.insert(
                        Argument::Named {
                            modifiers,
                            name,
                            value,
                        },
                        self.get_span_from(start),
                    )
                }
                // named maybe argument
                else if self.peek_name().is_ok()
                    && self.peek_next_token(TokenType::Maybe).is_ok()
                    && self.peek_next_next_token(TokenType::Colon).is_ok()
                {
                    // name
                    let name = self.eat_name()?;
                    let modifiers = self.eat_binding_modifiers_postfix(modifiers)?; // eat maybe
                    self.bump(); // eat colon
                    self.eat_newlines_maybe()?;
                    // value
                    let value =
                        self.with_options(self.options.nested(), |parser| parser.eat_expression())?;
                    self.tree.insert(
                        Argument::Named {
                            modifiers,
                            name,
                            value,
                        },
                        self.get_span_from(start),
                    )
                }
                // dynamic argument
                else if self.peek_token(TokenType::OpenBracket).is_ok() {
                    self.bump(); // eat open bracket
                    // name
                    let name = if self.peek_token(TokenType::Identifier).is_ok()
                        && self.peek_next_token(TokenType::Colon).is_ok()
                    {
                        let name = self.eat_identifier()?;
                        self.bump(); // eat colon
                        Some(name)
                    } else {
                        None
                    };
                    // key
                    let key = self.eat_expression().for_node_type(NodeType::Argument)?;
                    self.eat_token(TokenType::CloseBracket)?;
                    // value
                    self.eat_token(TokenType::Colon)?;
                    self.eat_newlines_maybe()?;
                    let value = self.eat_expression().for_node_type(NodeType::Argument)?;
                    self.tree.insert(
                        Argument::Dynamic {
                            modifiers,
                            name,
                            key,
                            value,
                        },
                        self.get_span_from(start),
                    )
                }
                // function shorthand argument (might be maybe)
                else if self.peek_token(TokenType::Identifier).is_ok()
                    && (self
                        .peek_next_token_in(&[TokenType::LessThan, TokenType::OpenParenthesis])
                        .is_ok()
                        || self.peek_next_token(TokenType::Maybe).is_ok()
                            && self
                                .peek_next_next_token_in(&[
                                    TokenType::LessThan,
                                    TokenType::OpenParenthesis,
                                ])
                                .is_ok())
                {
                    // postfix modifiers
                    let is_maybe = self.peek_next_token(TokenType::Maybe).is_ok();
                    if is_maybe {
                        modifiers = match modifiers {
                            Some(modifiers) => Some(modifiers.with_kind(BindingKind::Maybe)),
                            None => Some(BindingModifiers::default().with_kind(BindingKind::Maybe)),
                        };
                    }
                    // function
                    let function_id = self
                        .eat_function(DefinitionMeta::default(), is_maybe, false)
                        .for_node_type(NodeType::Expression)?;
                    // name
                    let name = self
                        .tree
                        .get(function_id)
                        .name()
                        .ok_or(ParserError::unexpected(self.get_span_from(start)))?;
                    // value
                    let value = self.tree.insert(
                        Expression::Definition(function_id),
                        self.get_span_from(start),
                    );
                    // clear function name
                    match self.tree.get_mut(function_id) {
                        Definition::Function {
                            meta: DefinitionMeta { name, .. },
                            style,
                            ..
                        } => {
                            *name = None;
                            *style = FunctionStyle::Lambda;
                        }
                        _ => panic!("expected function definition"),
                    };
                    self.tree.insert(
                        Argument::Function {
                            modifiers,
                            name,
                            value,
                        },
                        self.get_span_from(start),
                    )
                }
                // spread argument
                else if self.peek_token(TokenType::Range).is_ok()
                    || self.peek_token(TokenType::Spread).is_ok()
                {
                    self.bump(); // eat range
                    let name = self.eat_argument_name_maybe()?;
                    let value =
                        self.with_options(self.options.nested(), |parser| parser.eat_expression())?;
                    self.tree.insert(
                        Argument::Spread {
                            modifiers,
                            name,
                            value,
                        },
                        self.get_span_from(start),
                    )
                }
                // shorthand argument
                else {
                    let name = self.eat_identifier()?;
                    self.tree.insert(
                        Argument::Shorthand { modifiers, name },
                        self.get_span_from(start),
                    )
                }
            };
            arguments.push(argument_id);

            // item stop
            if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
            }
        }

        self.eat_token(TokenType::CloseBrace)
            .for_node_type(NodeType::Expression)?;
        Ok(arguments)
    }

    /// Peek a tree literal (including the `<` and `>` tokens).
    #[inline]
    pub fn peek_tree_literal(&self) -> ParserResult<()> {
        if self.peek_token(TokenType::LessThan).is_ok()
            && (self.peek_next_token(TokenType::GreaterThan).is_ok()
                || self.peek_next_token(TokenType::Divide).is_ok()
                || self.peek_next_token(TokenType::Identifier).is_ok())
        {
            return Ok(());
        }
        Err(ParserError::unexpected(self.peek()?.span))
    }

    /// Eat a tree literal (including the `<` and `>` tokens).
    ///
    /// Examples:
    /// ```
    /// <Entity />
    /// <Entity a=1 test />
    /// <Level level=1>
    ///     player: <Entity name="Alfred" />
    ///     <Entity>2</Entity>
    ///     "some text"
    ///     ..someChildren.map(child => <Entity name={child.name} />)
    /// </Level>
    /// ```
    pub fn eat_tree_literal(&mut self) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();
        self.eat_token(TokenType::LessThan)?;
        self.eat_newlines_maybe()?;

        // path
        let path: Option<Path> = {
            if self.peek_token(TokenType::Identifier).is_ok() {
                Some(self.eat_path()?)
            } else {
                None
            }
        };
        self.eat_newlines_maybe()?;

        // header (arguments separated by `=`)
        let arguments: Option<Vec<NodeId<Argument>>> = {
            // fragment without arguments
            if self.peek_token(TokenType::Divide).is_ok()
                || self.peek_token(TokenType::GreaterThan).is_ok()
            {
                None
            }
            // fragment with arguments
            else {
                let mut arguments: Vec<NodeId<Argument>> = vec![];
                while self.peek_token(TokenType::Divide).is_err()
                    && self.peek_token(TokenType::GreaterThan).is_err()
                {
                    let argument = self.with_options(self.options.in_tree_literal(), |parser| {
                        parser.eat_tree_literal_argument()
                    })?;
                    arguments.push(argument);
                    if self.peek_any_stop().is_ok() {
                        self.eat_any_stop_with_newlines()?;
                    }
                }
                Some(arguments)
            }
        };
        self.eat_newlines_maybe()?;

        // body (either />, or > with child elements)
        let elements: Option<Vec<NodeId<Argument>>> = {
            // fragment without children (/>)
            if self.peek_token(TokenType::Divide).is_ok() {
                self.bump(); // eat /
                self.eat_token(TokenType::GreaterThan)?; // eat >
                None
            }
            // fragment with children (>)
            else {
                self.eat_token(TokenType::GreaterThan)?; // eat >
                self.eat_newlines_maybe()?;

                // eat children until closing fragment
                let mut elements: Vec<NodeId<Argument>> = vec![];
                loop {
                    // stop at closing fragment (</)
                    if self.peek_token(TokenType::LessThan).is_ok()
                        && self.peek_next_token(TokenType::Divide).is_ok()
                    {
                        // special case for empty fragment (/>)
                        if path.is_none()
                            && self.peek_next_next_token(TokenType::GreaterThan).is_ok()
                        {
                            self.bump(); // eat <
                            self.bump(); // eat /
                            self.bump(); // eat >
                            break;
                        }
                        // check if closing fragment has same path
                        else if let Some(path) = &path {
                            let speculative_start = (self.mark(), self.tree.next_id());
                            // speculatively eat </path
                            self.bump(); // eat <
                            self.bump(); // eat /
                            match self.eat_path() {
                                Ok(closing_path) => {
                                    // found our closing tag
                                    if closing_path == *path {
                                        self.eat_token(TokenType::GreaterThan)?;
                                        break;
                                    }
                                    // not our closing tag
                                    else {
                                        self.restore(speculative_start.0, speculative_start.1);
                                    }
                                }
                                Err(_) => {
                                    // something else
                                    self.restore(speculative_start.0, speculative_start.1);
                                }
                            };
                        }
                    }

                    // keep eating child elements
                    let element = self.with_options(self.options.in_tree_literal(), |parser| {
                        parser.eat_argument()
                    })?;
                    elements.push(element);
                    if self.peek_any_stop().is_ok() {
                        self.eat_any_stop_with_newlines()?;
                    }
                }

                Some(elements)
            }
        };

        // tree literal
        let expression = Expression::TreeLiteral {
            path,
            arguments,
            elements,
        };
        Ok(self.tree.insert(expression, self.get_span_from(start)))
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        BindingKind, Definition, DefinitionMeta, Mutability, Name, Parameter, TemplateLiteral,
    };

    use crate::parse::tests::TestParser;
    use crate::{
        Argument, Block, Expression, ScalarLiteral, TypeLiteral, assert_expr_path, assert_node,
        assert_path, assert_string,
    };

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
        let mut test = TestParser::new(r#""hello" 'hi there' b"abc""#);
        let mut parser = test.prepare();

        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(
            parser.strings.get(match literal {
                ScalarLiteral::String(id) => id,
                other => panic!("expected string literal, got {other:?}"),
            }),
            "hello"
        );

        let literal = parser.eat_scalar_literal().unwrap();
        assert_eq!(
            parser.strings.get(match literal {
                ScalarLiteral::String(id) => id,
                other => panic!("expected string literal, got {other:?}"),
            }),
            "hi there"
        );

        let literal = parser.eat_scalar_literal().unwrap();
        match literal {
            ScalarLiteral::ByteString(bytes) => assert_eq!(bytes, b"abc"),
            other => panic!("expected byte string literal, got {other:?}"),
        }
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
        let literal = parser.eat_template_literal(None).unwrap();
        match literal {
            TemplateLiteral::String { string: template } => {
                // hello
                assert_string!(parser, template, "hello");
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `hello ${name}`
        let literal = parser.eat_template_literal(None).unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(arguments.len(), 1);
                assert_eq!(strings.len(), 2);
                // hello
                assert_string!(parser, strings[0], "hello ");
                // name
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "name");
                });
                //
                assert_string!(parser, strings[1], "");
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `${stmt}`
        let literal = parser.eat_template_literal(None).unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(strings.len(), 2);
                assert_eq!(arguments.len(), 1);
                // empty start & empty end
                assert_string!(parser, strings[0], "");
                assert_string!(parser, strings[1], "");
                // stmt
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "stmt");
                });
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `${start}${middle}${end}`
        let literal = parser.eat_template_literal(None).unwrap();
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
                    assert_expr_path!(parser, parser.tree.get(*value), "start");
                });
                // middle
                assert_node!(parser.tree, arguments[1], Argument::Positional { modifiers: _, value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "middle");
                });
                // end
                assert_node!(parser.tree, arguments[2], Argument::Positional { modifiers: _, value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "end");
                });
            }
            other => panic!("unexpected {other:?}"),
        }
        parser.eat_newline().unwrap();

        // `SELECT * FROM users WHERE name = ${name} AND age > ${group.age()} LIMIT 10`
        let literal = parser.eat_template_literal(None).unwrap();
        match literal {
            TemplateLiteral::InterpolatedString { strings, arguments } => {
                assert_eq!(arguments.len(), 2);
                assert_eq!(strings.len(), 3);
                // SELECT * FROM users WHERE name =
                assert_string!(parser, strings[0], "SELECT * FROM users WHERE name = ");
                // name
                assert_node!(parser.tree, arguments[0], Argument::Positional { modifiers: _, value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "name");
                });
                // AND age >
                assert_string!(parser, strings[1], " AND age > ");
                // group.age()
                assert_node!(parser.tree, arguments[1], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Call { left, .. } => {
                        assert_expr_path!(parser, parser.tree.get(*left), "group.age");
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
        let mut test = TestParser::new("int32 uint8 float bool symbol unique symbol");
        let mut parser = test.prepare();
        parser.options.in_type = true;

        assert!(
            matches!(parser.eat_type_literal(None).unwrap(), TypeLiteral::Int(int_ty) if int_ty.width == Some(32))
        );
        assert!(
            matches!(parser.eat_type_literal(None).unwrap(), TypeLiteral::Int(int_ty) if !int_ty.is_signed && int_ty.width == Some(8))
        );
        assert!(
            matches!(parser.eat_type_literal(None).unwrap(), TypeLiteral::Float(float_ty) if float_ty.width.is_none())
        );
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

    /// Parse a struct literal body containing both named and positional arguments.
    #[test]
    fn test_parse_struct_literal_body() {
        let mut test = TestParser::new(
            "{ 
    x: 1, 
    y,
    'Content-Type': 'application/json'
    [x]: true
}",
        );
        let mut parser = test.prepare();

        let arguments = parser.eat_struct_literal_body(None).unwrap();
        assert_eq!(arguments.len(), 4);

        // x: 1
        assert_node!(parser.tree, arguments[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
        // y
        assert_node!(parser.tree, arguments[1], Argument::Shorthand { modifiers: _, name } => {
            assert_string!(parser, *name, "y");
        });
        // "Content-Type": "application/json"
        assert_node!(parser.tree, arguments[2], Argument::Named { modifiers: _, name: Name::String(name), value } => {
            assert_string!(parser, *name, "Content-Type");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                assert_string!(parser, *string_id, "application/json");
            });
        });
        // [x]: true
        assert_node!(parser.tree, arguments[3], Argument::Dynamic { modifiers: _, name: None, key, value } => {
            // x
            assert_expr_path!(parser, parser.tree.get(*key), "x");
            // true
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
        });
    }

    /// Parse a struct literal body with readonly/type-like fields and shorthands.
    #[test]
    fn test_parse_struct_literal_body_like_type() {
        let mut test = TestParser::new(
            "{ 
    readonly a?: T, // readonly T?
    b,
    c?: T, // T?  
    readonly d: T, // readonly T
    e<T>(),
    f?(): T,
}",
        );
        let mut parser = test.prepare();

        let arguments = parser.eat_struct_literal_body(None).unwrap();
        assert_eq!(arguments.len(), 6);

        // readonly a?: T
        assert_node!(parser.tree, arguments[0], Argument::Named { modifiers: Some(modifiers), name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "a");
            assert_eq!(modifiers.mutability, Some(Mutability::Immutable));
            assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
            assert_expr_path!(parser, parser.tree.get(*value), "T");
        });

        // b (shorthand)
        assert_node!(parser.tree, arguments[1], Argument::Shorthand { modifiers: _, name } => {
            assert_string!(parser, *name, "b");
        });

        // c?: T
        assert_node!(parser.tree, arguments[2], Argument::Named { modifiers: Some(modifiers), name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "c");
            assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
            assert_expr_path!(parser, parser.tree.get(*value), "T");
        });

        // readonly d: T
        assert_node!(parser.tree, arguments[3], Argument::Named { modifiers: Some(modifiers), name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "d");
            assert_eq!(modifiers.mutability, Some(Mutability::Immutable));
            assert_expr_path!(parser, parser.tree.get(*value), "T");
        });

        // e<T>()
        assert_node!(parser.tree, arguments[4], Argument::Function { modifiers: _, name, value } => {
            assert_string!(parser, name.string(), "e");
            assert_node!(parser.tree, *value, Expression::Definition(function_id) => {
                assert_node!(parser.tree, *function_id, Definition::Function { meta: DefinitionMeta { name: None, .. }, static_parameters, .. } => {
                    // T
                    assert_node!(parser.tree, static_parameters.as_ref().unwrap()[0], Parameter::Named { name, .. } => {
                        assert_string!(parser, *name, "T");
                    });
                });
            });
        });

        // f?(): T
        assert_node!(parser.tree, arguments[5], Argument::Function { modifiers: Some(modifiers), name, value } => {
            assert_string!(parser, name.string(), "f");
            assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
            assert_node!(parser.tree, *value, Expression::Definition(function_id) => {
                assert_node!(parser.tree, *function_id, Definition::Function { meta: DefinitionMeta { name: None, .. }, .. });
            });
        });
    }

    /// Parse a struct literal body with a single shorthand function argument.
    #[test]
    fn test_parse_struct_literal_single_shorthand_function() {
        let mut test = TestParser::new(
            r#"{
    fetch(req: Request) {
        return Response("Success!");
    },
}"#,
        );
        let mut parser = test.prepare();

        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::StructLiteral { ty: None, fields } => {
            assert_eq!(fields.len(), 1);
            // fetch
            assert_node!(parser.tree, fields[0], Argument::Function { modifiers: _, name, value } => {
                assert_string!(parser, name.string(), "fetch");
                assert_node!(parser.tree, *value, Expression::Definition(function_id) => {
                    assert_node!(parser.tree, *function_id, Definition::Function { meta: DefinitionMeta { name: None, .. }, .. });
                });
            });
        });
    }

    /// Parse a struct literal body with implicit function arguments in a type context.
    /// (Otherwise the function call would be interpreted as a call instead of a function argument)
    #[test]
    fn test_parse_struct_literal_body_with_implicit_function_in_type() {
        let mut test = TestParser::new(
            "{ 
    foo()
    foo?(): T
}",
        );
        let mut parser = test.prepare();
        parser.options.in_type = true;
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::StructLiteral { ty: None, fields } => {
            assert_eq!(fields.len(), 2);
            // foo
            assert_node!(parser.tree, fields[0], Argument::Function { modifiers: _, name, value } => {
                assert_string!(parser, name.string(), "foo");
                assert_node!(parser.tree, *value, Expression::Definition(function_id) => {
                    assert_node!(parser.tree, *function_id, Definition::Function { meta: DefinitionMeta { name: None, .. }, .. });
                });
            });
            // foo?(): T
            assert_node!(parser.tree, fields[1], Argument::Function { modifiers: Some(modifiers), name, value } => {
                assert_string!(parser, name.string(), "foo");
                assert_eq!(modifiers.kind, Some(BindingKind::Maybe));
                assert_node!(parser.tree, *value, Expression::Definition(function_id) => {
                    assert_node!(parser.tree, *function_id, Definition::Function { meta: DefinitionMeta { name: None, .. }, .. });
                });
            });
        });
    }

    #[test]
    fn test_parse_struct_literal_body_with_dynamic_field() {
        let mut test = TestParser::new(r#"{ [key: T]: (...args: Array<any>) => any }"#);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::StructLiteral { ty: None, fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], Argument::Dynamic { modifiers: _, name, key, value } => {
                // [key: a]
                assert_string!(parser, name.unwrap(), "key");
                assert_expr_path!(parser, parser.tree.get(*key), "T");
                // (...args: Array<any>) => any
                assert_node!(parser.tree, *value, Expression::Definition(function_id) => {
                    assert_node!(parser.tree, *function_id, Definition::Function { meta: DefinitionMeta { name: None, .. }, dynamic_parameters, .. } => {
                        // ...args: Array<any>
                        assert_eq!(dynamic_parameters.len(), 1);
                        assert_node!(parser.tree, dynamic_parameters[0], Parameter::Variadic { modifiers: _, name, ty, .. } => {
                            assert_string!(parser, *name, "args");
                            assert_expr_path!(parser, parser.tree.get(ty.unwrap()), "Array");
                        });
                    });
                });
            });
        });
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
    fn test_parse_tree_fragment() {
        let mut test = TestParser::new("<A/>");
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        // <A/>
        assert_node!(parser.tree, expression, Expression::TreeLiteral { path, arguments, elements } => {
            // A
            assert_path!(parser, path.as_ref().unwrap(), "A");
            assert!(arguments.is_none());
            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_fragment_with_arguments() {
        let mut test = TestParser::new("<A a=1 annoying-bee=2 c=3 flag />");
        let mut parser = test.prepare();
        let expression = parser.eat_tree_literal().unwrap();
        // <A a=1 annoying-b=2 c=3 />
        assert_node!(parser.tree, expression, Expression::TreeLiteral { path, arguments, elements } => {
            // A
            assert_path!(parser, path.as_ref().unwrap(), "A");
            assert_eq!(arguments.as_ref().unwrap().len(), 4);
            // a=1
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            // annoying-b=2
            assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "annoyingBee");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
            // c=3
            assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "c");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
            // flag
            assert_node!(parser.tree, arguments.as_ref().unwrap()[3], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "flag");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });

            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_fragment_with_arguments_and_child() {
        let mut test = TestParser::new(
            r"
<Tooltip
    title=true
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
        // <Tooltip>
        assert_node!(parser.tree, expression, Expression::TreeLiteral { path, arguments, elements } => {
            assert_path!(parser, path.as_ref().unwrap(), "Tooltip");
            assert_eq!(arguments.as_ref().unwrap().len(), 3);
            // title=true
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "title");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
            // flag
            assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "flag");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
            // something-else=false
            assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "somethingElse");
                assert_node!(parser.tree, *value, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                        assert_eq!(expressions.len(), 1);
                        assert_node!(parser.tree, expressions[0], Expression::ScalarLiteral(ScalarLiteral::Boolean(false)));
                    });
                })
            });

            assert!(elements.is_some());
            // {true}
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Block(block_id) => {
                    assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                        assert_eq!(expressions.len(), 1);
                        assert_node!(parser.tree, expressions[0], Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
                    });
                })
            });
        });
    }

    #[test]
    fn test_parse_tree_fragment_with_spread_argument() {
        let mut test = TestParser::new(
            r"
<A  
    a={..a} // not a rest because {..a} is just a struct literal
    {...b} // ...b
    ...c
/>",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression = parser.eat_tree_literal().unwrap();
        assert_node!(parser.tree, expression, Expression::TreeLiteral { path, arguments, elements: _ } => {
            assert_path!(parser, path.as_ref().unwrap(), "A");
            assert_eq!(arguments.as_ref().unwrap().len(), 3);
            // a={..a}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *value, Expression::StructLiteral { ty: None, fields } => {
                    assert_eq!(fields.len(), 1);
                    assert_node!(parser.tree, fields[0], Argument::Spread { modifiers: _, name: None, value } => {
                        assert_expr_path!(parser, parser.tree.get(*value), "a");
                    });
                });
            });
            // {...b}
            assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Spread { modifiers: _, name: None, value } => {
                assert_expr_path!(parser, parser.tree.get(*value), "b");
            });
            // ...c
            assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Spread { modifiers: _, name: None, value } => {
                assert_expr_path!(parser, parser.tree.get(*value), "c");
            });
        });
    }

    #[test]
    fn test_parse_tree_nested_deep() {
        let mut test = TestParser::new(
            r"
<A>
    <B>
        <C>
            <D/>
            2
        </C>
    </B>
</A>
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression = parser.eat_tree_literal().unwrap();
        // <A>
        assert_node!(parser.tree, expression, Expression::TreeLiteral { path, arguments, elements } => {
            assert_path!(parser, path.as_ref().unwrap(), "A");
            assert!(arguments.is_none());
            assert!(elements.is_some());
            // <B>
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::TreeLiteral { path, arguments, elements } => {
                    assert_path!(parser, path.as_ref().unwrap(), "B");
                    assert!(arguments.is_none());
                    assert!(elements.is_some());
                    // <C>
                    assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::TreeLiteral { path, arguments, elements } => {
                            assert_path!(parser, path.as_ref().unwrap(), "C");
                            assert!(arguments.is_none());
                            assert!(elements.is_some());
                            // <D/>
                            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                                assert_node!(parser.tree, *value, Expression::TreeLiteral { path, arguments, elements } => {
                                    assert_path!(parser, path.as_ref().unwrap(), "D");
                                    assert!(arguments.is_none());
                                    assert!(elements.is_none());
                                });
                            });
                            // 2
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
        header: "Hello"
    </div>
)
        "#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression, Expression::Parenthesized { expression } => {
            // <div className="font-semibold">
            assert_node!(parser.tree, *expression, Expression::TreeLiteral { path, arguments, elements } => {
                assert_path!(parser, path.as_ref().unwrap(), "div");
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
                assert_eq!(elements.as_ref().unwrap().len(), 2);
                // <Link subtle to={urls.annotation(annotation.id)}>
                assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::TreeLiteral { path, arguments, elements } => {
                        // Link
                        assert_path!(parser, path.as_ref().unwrap(), "Link");
                        assert!(arguments.is_some());
                        assert_eq!(arguments.as_ref().unwrap().len(), 2);
                        // subtle
                        assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                            assert_string!(parser, *name, "subtle");
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
                        });
                        // to={1}
                        assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                            assert_string!(parser, *name, "to");
                            assert_node!(parser.tree, *value, Expression::Block(block_id) => {
                                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                                    assert_eq!(expressions.len(), 1);
                                    assert_node!(parser.tree, expressions[0], Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                                });
                            });
                        });

                        assert!(elements.is_some());
                        assert_eq!(elements.as_ref().unwrap().len(), 1);
                        // {2}
                        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                            assert_node!(parser.tree, *value, Expression::Block(block_id) => {
                                assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                                    assert_eq!(expressions.len(), 1);
                                    assert_node!(parser.tree, expressions[0], Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                                });
                            });
                        });
                    });
                });
                // header="Hello"
                assert_node!(parser.tree, elements.as_ref().unwrap()[1], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                    assert_string!(parser, *name, "header");
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, "Hello");
                    });
                });
            });
        });
    }
}
