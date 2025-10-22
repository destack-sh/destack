use std::borrow::Cow;

use dyst_ast::{Definition, Keyword, Mutability, Name, Path, StringId, TemplateLiteral};
use std::str::FromStr;

use crate::parse::prelude::*;
use crate::parse::variant::VARIANT_FIELD_MODIFIERS;
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
    /// 1.0
    /// 0x1234
    /// /abc/
    /// /abc/g
    /// "hello"
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
            LiteralType::Int { base, is_empty } => {
                if is_empty {
                    return Err(ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    ));
                }

                // strip underscores for parsing
                let cleaned: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };

                // handle base-specific prefixes
                let parsed = match base {
                    NumberBase::Decimal => cleaned.parse::<i64>(),
                    NumberBase::Binary => i64::from_str_radix(cleaned.trim_start_matches("0b"), 2),
                    NumberBase::Octal => i64::from_str_radix(cleaned.trim_start_matches("0o"), 8),
                    NumberBase::Hexadecimal => {
                        i64::from_str_radix(cleaned.trim_start_matches("0x"), 16)
                    }
                };

                parsed.map(ScalarLiteral::Integer).map_err(|_| {
                    ParserError::expected_for(
                        literal_span.span,
                        TokenType::Literal,
                        NodeType::Expression,
                    )
                })
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

                let cleaned: Cow<'_, str> = if literal_str.contains('_') {
                    Cow::Owned(literal_str.replace('_', ""))
                } else {
                    Cow::Borrowed(literal_str)
                };

                cleaned
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
            LiteralType::Character { is_terminated } => {
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
                    let argument = self.eat_argument()?;
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

    /// Eat the body of a tuple literal (excluding the surrounding parenthesis).
    pub fn eat_tuple_literal_body(
        &mut self,
        first_element: Option<NodeId<Argument>>,
    ) -> ParserResult<Vec<NodeId<Argument>>> {
        let mut elements = Vec::new();
        if let Some(first) = first_element {
            elements.push(first);
        }
        loop {
            // stop at closing parenthesis
            if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
                continue;
            }

            // keep eating elements
            let element = self
                .eat_tuple_literal_element()
                .for_node_type(NodeType::Argument)?;
            elements.push(element);
        }
        Ok(elements)
    }

    /// Eat a single tuple literal element.
    pub fn eat_tuple_literal_element(&mut self) -> ParserResult<NodeId<Argument>> {
        let start = self.mark();
        // named argument
        if self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token(TokenType::Colon).is_ok()
        {
            let name = self.eat_identifier()?;
            self.eat_token(TokenType::Colon)?;
            let value = self.eat_expression()?;
            let argument_id = self.tree.insert(
                Argument::Named {
                    name: Name::Identifier(name),
                    value,
                },
                self.get_span_from(start),
            );
            Ok(argument_id)
        }
        // positional argument
        else {
            let value = self.eat_expression()?;
            let argument_id = self
                .tree
                .insert(Argument::Positional { value }, self.get_span_from(start));
            Ok(argument_id)
        }
    }

    /// Eat an array literal body and return its element expressions.
    pub fn eat_array_literal(&mut self) -> ParserResult<Vec<NodeId<Expression>>> {
        self.eat_token(TokenType::OpenBracket)
            .for_node_type(NodeType::Expression)?;
        self.eat_newlines_maybe()?;

        let mut elements = Vec::new();
        loop {
            // stop at closing bracket
            if self.peek_token(TokenType::CloseBracket).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
                continue;
            }
            // keep eating elements
            let element = self.eat_expression().for_node_type(NodeType::Expression)?;
            elements.push(element);
        }

        self.eat_token(TokenType::CloseBracket)
            .for_node_type(NodeType::Expression)?;
        Ok(elements)
    }

    /// Peek an anomymous non-empty struct literal (without prefix, like `{ x: 0, y }` or `{ ..a }`):
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
                    || Keyword::from_str(self.get_token_str(*token))
                        .map(|keyword| VARIANT_FIELD_MODIFIERS.contains(&keyword))
                        .unwrap_or(false))
            {
                pos += 1;
            }
            if pos + 3 >= self.tokens.len() {
                return Err(ParserError::unexpected(self.peek()?.span));
            }

            // struct literal field (name: type, name?: type, name = <expr>)
            let token_ty = self.tokens[pos].token.ty;
            let next_token_ty = self.tokens[pos + 1].token.ty;
            let next_next_token_ty = self.tokens[pos + 2].token.ty;
            match (token_ty, next_token_ty, next_next_token_ty) {
                    // name:
                    (TokenType::Identifier, TokenType::Colon, _)
                    // name?:
                    | (TokenType::Identifier, TokenType::Maybe, TokenType::Colon)
                    // name,
                    | (TokenType::Identifier, TokenType::Comma, _)
                    // ..T
                    | (TokenType::Range, TokenType::Identifier, _)
                    // ...T
                    | (TokenType::RangeWide, TokenType::Identifier, _) => {
                        return Ok(None);
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
                let Ok(function_id) = self.eat_function(None, None, is_maybe, expect_body) else {
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
                    Definition::Function { name, .. } => {
                        *name = None;
                    }
                    _ => panic!("expected function for"),
                };
                // maybe
                let value = if is_maybe {
                    self.tree
                        .insert(Expression::Maybe(value), self.tree.spans.get(value))
                } else {
                    value
                };
                return Ok(Some(self.tree.insert(
                    Argument::NamedFunction { name, value },
                    self.get_span_from(start),
                )));
            }
        }

        Err(ParserError::unexpected(self.peek()?.span))
    }

    /// Wrap an expression in a readonly maybe type (maybe).
    #[inline]
    fn make_readonly_maybe(
        &mut self,
        is_readonly: bool,
        expression_id: NodeId<Expression>,
    ) -> NodeId<Expression> {
        if is_readonly {
            self.tree.insert(
                Expression::Type {
                    mutability: Some(Mutability::Immutable),
                    value: expression_id,
                },
                self.tree.spans.get(expression_id),
            )
        } else {
            expression_id
        }
    }

    /// Eat the body of a struct literal (including the `{` and `}`, without a prefix).
    pub(crate) fn eat_struct_literal_body(
        &mut self,
        first_argument: Option<NodeId<Argument>>,
    ) -> ParserResult<Vec<NodeId<Argument>>> {
        // (skip the opening sequence if we're given first argument from a speculative parse)
        if first_argument.is_none() {
            self.eat_token(TokenType::OpenBrace)
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
            //  (so we re-implement some of the argument parsing logic here,
            //   because we need to account for readonly/maybe/functions/...)
            let start = self.mark();
            let argument_id = {
                // readonly
                let is_readonly = if self.peek_keyword(Keyword::Readonly).is_ok()
                    && self.peek_next_token(TokenType::Identifier).is_ok()
                {
                    self.bump(); // eat readonly
                    true
                } else {
                    false
                };

                // named argument
                if self.peek_name().is_ok() && self.peek_next_token(TokenType::Colon).is_ok() {
                    // name
                    let name = self.eat_name()?;
                    self.bump(); // eat colon
                    // value
                    let value =
                        self.with_options(self.options.nested(), |parser| parser.eat_expression())?;
                    let value = self.make_readonly_maybe(is_readonly, value);
                    self.tree
                        .insert(Argument::Named { name, value }, self.get_span_from(start))
                }
                // named maybe argument
                else if self.peek_name().is_ok()
                    && self.peek_next_token(TokenType::Maybe).is_ok()
                    && self.peek_next_next_token(TokenType::Colon).is_ok()
                {
                    // name
                    let name = self.eat_name()?;
                    self.bump(); // eat maybe
                    self.bump(); // eat colon
                    // value
                    let value =
                        self.with_options(self.options.nested(), |parser| parser.eat_expression())?;
                    let value = self
                        .tree
                        .insert(Expression::Maybe(value), self.tree.spans.get(value));
                    let value = self.make_readonly_maybe(is_readonly, value);
                    self.tree
                        .insert(Argument::Named { name, value }, self.get_span_from(start))
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
                    let value = self.eat_expression().for_node_type(NodeType::Argument)?;
                    let value = self.make_readonly_maybe(is_readonly, value);
                    self.tree.insert(
                        Argument::Dynamic { name, key, value },
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
                    // function
                    let is_maybe = self.peek_next_token(TokenType::Maybe).is_ok();
                    let function_id = self.eat_function(None, None, is_maybe, false)?;
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
                        Definition::Function { name, .. } => {
                            *name = None;
                        }
                        _ => panic!("expected function definition"),
                    };
                    // maybe
                    let value = if is_maybe {
                        self.tree
                            .insert(Expression::Maybe(value), self.tree.spans.get(value))
                    } else {
                        value
                    };
                    self.tree.insert(
                        Argument::NamedFunction { name, value },
                        self.get_span_from(start),
                    )
                }
                // spread argument
                else if self.peek_token(TokenType::Range).is_ok()
                    || self.peek_token(TokenType::RangeWide).is_ok()
                {
                    self.bump(); // eat range
                    let value =
                        self.with_options(self.options.nested(), |parser| parser.eat_expression())?;
                    self.tree
                        .insert(Argument::Spread { value }, self.get_span_from(start))
                }
                // shorthand argument
                else {
                    let name = self.eat_identifier()?;
                    self.tree
                        .insert(Argument::NamedShorthand { name }, self.get_span_from(start))
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
    use dyst_ast::{Definition, Mutability, Name, Parameter, TemplateLiteral};

    use crate::parse::tests::TestParser;
    use crate::{
        Argument, Block, Expression, ScalarLiteral, TokenType, TypeLiteral, assert_expr_path,
        assert_node, assert_path, assert_string,
    };

    /// Parse integer literals in various formats.
    #[test]
    fn test_parse_integer_literal() {
        let mut test = TestParser::new("1 731 0x1234");
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
    }

    /// Parse scientific notation and decimal floats.
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
                assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
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
                assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
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
                assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "start");
                });
                // middle
                assert_node!(parser.tree, arguments[1], Argument::Positional { value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "middle");
                });
                // end
                assert_node!(parser.tree, arguments[2], Argument::Positional { value } => {
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
                assert_node!(parser.tree, arguments[0], Argument::Positional { value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "name");
                });
                // AND age >
                assert_string!(parser, strings[1], " AND age > ");
                // group.age()
                assert_node!(parser.tree, arguments[1], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::Call { receiver, .. } => {
                        assert_expr_path!(parser, parser.tree.get(*receiver), "group.age");
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
        let mut test = TestParser::new("int32 uint8 float bool");
        let mut parser = test.prepare();
        parser.options.in_type = true;

        assert!(
            matches!(parser.eat_type_literal().unwrap(), TypeLiteral::Int(int_ty) if int_ty.width == Some(32))
        );
        assert!(
            matches!(parser.eat_type_literal().unwrap(), TypeLiteral::Int(int_ty) if !int_ty.is_signed && int_ty.width == Some(8))
        );
        assert!(
            matches!(parser.eat_type_literal().unwrap(), TypeLiteral::Float(float_ty) if float_ty.width.is_none())
        );
        assert!(matches!(
            parser.eat_type_literal().unwrap(),
            TypeLiteral::Boolean
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
    [var]: true
} ",
        );
        let mut parser = test.prepare();

        let arguments = parser.eat_struct_literal_body(None).unwrap();
        assert_eq!(arguments.len(), 4);

        // x: 1
        assert_node!(parser.tree, arguments[0], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
        // y
        assert_node!(parser.tree, arguments[1], Argument::NamedShorthand { name } => {
            assert_string!(parser, *name, "y");
        });
        // "Content-Type": "application/json"
        assert_node!(parser.tree, arguments[2], Argument::Named { name: Name::String(name), value } => {
            assert_string!(parser, *name, "Content-Type");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                assert_string!(parser, *string_id, "application/json");
            });
        });
        // [var]: true
        assert_node!(parser.tree, arguments[3], Argument::Dynamic { name: None, key, value } => {
            // var
            assert_expr_path!(parser, parser.tree.get(*key), "var");
            // true
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
        });
    }

    /// Parse a struct literal body with readonly/type-like fields and shorthands.
    #[test]
    fn test_parse_struct_literal_body_like_type() {
        let mut test = TestParser::new(
            "{ 
    readonly a?: T // readonly T?
    b
    c?: T // T?  
    readonly d: T // readonly T
    e<T>()
    f?(): T
} ",
        );
        let mut parser = test.prepare();

        let arguments = parser.eat_struct_literal_body(None).unwrap();
        assert_eq!(arguments.len(), 6);

        // readonly a?: T
        assert_node!(parser.tree, arguments[0], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "a");
            assert_node!(parser.tree, *value, Expression::Type { mutability: Some(Mutability::Immutable), value } => {
                assert_node!(parser.tree, *value, Expression::Maybe(inner) => {
                    assert_expr_path!(parser, parser.tree.get(*inner), "T");
                });
            });
        });

        // b (shorthand)
        assert_node!(parser.tree, arguments[1], Argument::NamedShorthand { name } => {
            assert_string!(parser, *name, "b");
        });

        // c?: T
        assert_node!(parser.tree, arguments[2], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "c");
            assert_node!(parser.tree, *value, Expression::Maybe(inner) => {
                assert_expr_path!(parser, parser.tree.get(*inner), "T");
            });
        });

        // readonly d: T
        assert_node!(parser.tree, arguments[3], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "d");
            assert_node!(parser.tree, *value, Expression::Type { mutability: Some(Mutability::Immutable), value } => {
                assert_expr_path!(parser, parser.tree.get(*value), "T");
            });
        });

        // e<T>()
        assert_node!(parser.tree, arguments[4], Argument::NamedFunction { name, value } => {
            assert_string!(parser, *name, "e");
            assert_node!(parser.tree, *value, Expression::Definition(function_id) => {
                assert_node!(parser.tree, *function_id, Definition::Function { name: None, static_parameters, .. } => {
                    // T
                    assert_node!(parser.tree, static_parameters.as_ref().unwrap()[0], Parameter::Named { name, .. } => {
                        assert_string!(parser, *name, "T");
                    });
                });
            });
        });

        // f?(): T
        assert_node!(parser.tree, arguments[5], Argument::NamedFunction { name, value } => {
            assert_string!(parser, *name, "f");
            assert_node!(parser.tree, *value, Expression::Maybe(inner) => {
                assert_node!(parser.tree, *inner, Expression::Definition(function_id) => {
                    assert_node!(parser.tree, *function_id, Definition::Function { name: None, .. });
                });
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
            assert_node!(parser.tree, fields[0], Argument::NamedFunction { name, value } => {
                assert_string!(parser, *name, "fetch");
                assert_node!(parser.tree, *value, Expression::Definition(function_id) => {
                    assert_node!(parser.tree, *function_id, Definition::Function { name: None, .. });
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
            assert_node!(parser.tree, fields[0], Argument::NamedFunction { name, value } => {
                assert_string!(parser, *name, "foo");
                assert_node!(parser.tree, *value, Expression::Definition(function_id) => {
                    assert_node!(parser.tree, *function_id, Definition::Function { name: None, .. });
                });
            });
            // foo?(): T
            assert_node!(parser.tree, fields[1], Argument::NamedFunction { name, value } => {
                assert_string!(parser, *name, "foo");
                assert_node!(parser.tree, *value, Expression::Maybe(inner) => {
                    assert_node!(parser.tree, *inner, Expression::Definition(function_id) => {
                        assert_node!(parser.tree, *function_id, Definition::Function { name: None, .. });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_tuple_literal_body() {
        let mut test = TestParser::new("(a: 1, 2)");
        let mut parser = test.prepare();

        parser.eat_token(TokenType::OpenParenthesis).unwrap();
        let first = parser.eat_tuple_literal_element().unwrap();
        let elements = parser.eat_tuple_literal_body(Some(first)).unwrap();
        assert_eq!(elements.len(), 2);

        // a: 1
        assert_node!(parser.tree, elements[0], Argument::Named { name: Name::Identifier(name), value } => {
            assert_string!(parser, *name, "a");
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
        // 2 (positional argument)
        assert_node!(parser.tree, elements[1], Argument::Positional { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
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
            Expression::ScalarLiteral(ScalarLiteral::Integer(1))
        );
        // 2
        assert_node!(
            parser.tree,
            elements[1],
            Expression::ScalarLiteral(ScalarLiteral::Integer(2))
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
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            // annoying-b=2
            assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "annoyingBee");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
            // c=3
            assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "c");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
            // flag
            assert_node!(parser.tree, arguments.as_ref().unwrap()[3], Argument::Named { name: Name::Identifier(name), value } => {
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
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "title");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
            // flag
            assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "flag");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
            // something-else=false
            assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { name: Name::Identifier(name), value } => {
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
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
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
            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
                assert_node!(parser.tree, *value, Expression::TreeLiteral { path, arguments, elements } => {
                    assert_path!(parser, path.as_ref().unwrap(), "B");
                    assert!(arguments.is_none());
                    assert!(elements.is_some());
                    // <C>
                    assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
                        assert_node!(parser.tree, *value, Expression::TreeLiteral { path, arguments, elements } => {
                            assert_path!(parser, path.as_ref().unwrap(), "C");
                            assert!(arguments.is_none());
                            assert!(elements.is_some());
                            // <D/>
                            assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
                                assert_node!(parser.tree, *value, Expression::TreeLiteral { path, arguments, elements } => {
                                    assert_path!(parser, path.as_ref().unwrap(), "D");
                                    assert!(arguments.is_none());
                                    assert!(elements.is_none());
                                });
                            });
                            // 2
                            assert_node!(parser.tree, elements.as_ref().unwrap()[1], Argument::Positional { value } => {
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
                assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { name: Name::Identifier(name), value } => {
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
                assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::TreeLiteral { path, arguments, elements } => {
                        // Link
                        assert_path!(parser, path.as_ref().unwrap(), "Link");
                        assert!(arguments.is_some());
                        assert_eq!(arguments.as_ref().unwrap().len(), 2);
                        // subtle
                        assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { name: Name::Identifier(name), value } => {
                            assert_string!(parser, *name, "subtle");
                            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
                        });
                        // to={1}
                        assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { name: Name::Identifier(name), value } => {
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
                        assert_node!(parser.tree, elements.as_ref().unwrap()[0], Argument::Positional { value } => {
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
                assert_node!(parser.tree, elements.as_ref().unwrap()[1], Argument::Named { name: Name::Identifier(name), value } => {
                    assert_string!(parser, *name, "header");
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
                        assert_string!(parser, *string_id, "Hello");
                    });
                });
            });
        });
    }
}
