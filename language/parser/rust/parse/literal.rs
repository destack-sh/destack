use std::borrow::Cow;

use dyst_ast::Path;

use crate::parse::prelude::*;
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
            let argument_id = self
                .tree
                .insert(Argument::Named { name, value }, self.get_span_from(start));
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

    /// Peek an anomymous non-empty struct literal (without prefix, like `{ x: 0, y }` or `{ ..a }`).
    /// The first element may also be after some newlines.
    pub fn peek_anonymous_struct_literal_body(&self) -> ParserResult<()> {
        if self.peek_token(TokenType::OpenBrace).is_ok() {
            let mut current_pos = self.pos() + 1;
            // skip any newlines
            while let Some(token) = self.tokens.get(current_pos as usize)
                && token.token.ty == TokenType::Newline
            {
                current_pos += 1;
            }
            // we're looking for (identifier, colon) | (range, identifier) | (range wide, identifier)
            let token_ty = self.tokens[current_pos as usize].token.ty;
            let next_token_ty = self.tokens[current_pos as usize + 1].token.ty;
            match (token_ty, next_token_ty) {
                (TokenType::Identifier, TokenType::Colon)
                | (TokenType::Range, TokenType::Identifier)
                | (TokenType::RangeWide, TokenType::Identifier) => {
                    return Ok(());
                }
                _ => {}
            }
        }

        Err(ParserError::unexpected(self.peek()?.span))
    }

    /// Eat the body of a struct literal (including the `{` and `}`, without a prefix).
    pub(crate) fn eat_struct_literal_body(&mut self) -> ParserResult<Vec<NodeId<Argument>>> {
        self.eat_token(TokenType::OpenBrace)
            .for_node_type(NodeType::Expression)?;
        self.eat_newlines_maybe()?;

        // empty struct
        if self.peek_token(TokenType::CloseBrace).is_ok() {
            self.eat_token(TokenType::CloseBrace)
                .for_node_type(NodeType::Expression)?;
            return Ok(vec![]);
        }

        let mut arguments = Vec::new();
        loop {
            // stop at closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }

            let start = self.mark();
            // named argument
            let argument_id = {
                if self.peek_token(TokenType::Identifier).is_ok()
                    && (self.peek_next_token(TokenType::Colon).is_ok()
                        || self.peek_next_token(TokenType::Assign).is_ok())
                {
                    let name = self.eat_identifier()?;
                    self.bump(); // eat colon or assign
                    let value = self.eat_expression()?;
                    self.tree
                        .insert(Argument::Named { name, value }, self.get_span_from(start))
                }
                // spread argument
                else if self.peek_token(TokenType::Range).is_ok()
                    || self.peek_token(TokenType::RangeWide).is_ok()
                {
                    self.bump(); // eat range
                    let value = self.eat_expression()?;
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
            if self.peek_item_stop().is_ok() {
                self.eat_item_stop_with_newlines()?;
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
    use crate::parse::tests::TestParser;
    use crate::{
        Argument, Expression, ScalarLiteral, TokenType, TypeLiteral, assert_node, assert_path,
        assert_string,
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

    #[test]
    fn test_parse_struct_literal_body() {
        let mut test = TestParser::new("{ x: 1, y } ");
        let mut parser = test.prepare();

        let arguments = parser.eat_struct_literal_body().unwrap();
        assert_eq!(arguments.len(), 2);
        assert!(matches!(
            parser.tree.get(arguments[0]),
            Argument::Named { .. }
        ));
        assert!(matches!(
            parser.tree.get(arguments[1]),
            Argument::NamedShorthand { .. }
        ));
    }

    #[test]
    fn test_parse_tuple_literal_body() {
        let mut test = TestParser::new("(a: 1, 2)");
        let mut parser = test.prepare();

        parser.eat_token(TokenType::OpenParenthesis).unwrap();
        let first = parser.eat_tuple_literal_element().unwrap();
        let elements = parser.eat_tuple_literal_body(Some(first)).unwrap();
        assert_eq!(elements.len(), 2);
        assert!(matches!(
            parser.tree.get(elements[0]),
            Argument::Named { .. }
        ));
        assert!(matches!(
            parser.tree.get(elements[1]),
            Argument::Positional { .. }
        ));
    }

    #[test]
    fn test_parse_array_literal() {
        let mut test = TestParser::new("[1, 2]");
        let mut parser = test.prepare();

        let elements = parser.eat_array_literal().unwrap();
        assert_eq!(elements.len(), 2);
        assert!(matches!(
            parser.tree.get(elements[0]),
            Expression::ScalarLiteral(ScalarLiteral::Integer(1))
        ));
        assert!(matches!(
            parser.tree.get(elements[1]),
            Expression::ScalarLiteral(ScalarLiteral::Integer(2))
        ));
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
            assert_node!(parser.tree, arguments.as_ref().unwrap()[0], Argument::Named { name, value } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            // annoying-b=2
            assert_node!(parser.tree, arguments.as_ref().unwrap()[1], Argument::Named { name, value } => {
                assert_string!(parser, *name, "annoyingBee");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
            // c=3
            assert_node!(parser.tree, arguments.as_ref().unwrap()[2], Argument::Named { name, value } => {
                assert_string!(parser, *name, "c");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
            // flag
            assert_node!(parser.tree, arguments.as_ref().unwrap()[3], Argument::Named { name, value } => {
                assert_string!(parser, *name, "flag");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });

            assert!(elements.is_none());
        });
    }

    #[test]
    fn test_parse_tree_nested() {
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
}
