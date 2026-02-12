use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

use destack_ast::{
    Expression, LiteralType, LocalNodeId, Name, NodeType, Pattern, PatternField, ScalarLiteral,
    TokenType, TypeLiteral,
};

impl Parser {
    /// Eat a pattern that might be paranthesized (skip the parenthesis if present).
    pub fn eat_pattern_parenthesized_maybe(&mut self) -> ParseResult<LocalNodeId<Pattern>> {
        let start = self.mark();
        if self.peek_is(TokenType::OpenParenthesis) {
            self.bump(); // eat open parenthesis
            self.eat_newlines_maybe()?;
            let pattern_id = self.eat_pattern()?;
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseParenthesis)?;
            self.tree.set_span(pattern_id, self.get_span_from(&start));
            Ok(pattern_id)
        } else {
            self.eat_pattern()
        }
    }

    /// Eat a pattern.
    ///
    /// Examples:
    /// ```
    /// _
    /// ..
    /// 1
    /// 2 | 3
    /// 4..6
    /// (x, 0, ...)
    /// { a: 2 }
    /// Success(_)
    /// Vector2 { x: 0, y, z: zed }
    /// geom.Mesh<2, float32> { vertices: [2, ...] }
    /// ```
    pub fn eat_pattern(&mut self) -> ParseResult<LocalNodeId<Pattern>> {
        let _timing = self.timing_scope(tags::PARSE_PATTERN);
        let start = self.mark();

        // mutability
        let mutability = self.eat_mutability_maybe()?;

        // ------------------------------------------------------------
        // Primary patterns
        // ------------------------------------------------------------
        let pattern_id = {
            // wildcard
            if self.peek_identifier_str_is("_") {
                self.bump(); // eat wildcard
                self.tree
                    .insert(Pattern::Wildcard, self.get_span_from(&start))
            }
            // reference of
            else if self.peek_is(TokenType::ElementwiseAnd) {
                self.bump(); // eat &
                let mutability = self.eat_reference_mutability_maybe()?;
                let right_id = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                self.tree.insert(
                    Pattern::ReferenceOf {
                        mutability,
                        right: right_id,
                    },
                    self.get_span_from(&start),
                )
            }
            // value of
            else if self.peek_is(TokenType::ElementwiseXor) {
                self.bump(); // eat ^
                let mutability = self.eat_reference_mutability_maybe()?;
                let right_id = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                self.tree.insert(
                    Pattern::ValueOf {
                        mutability,
                        right: right_id,
                    },
                    self.get_span_from(&start),
                )
            }
            // tuple (without type, no struct tuples)
            else if self.peek_is(TokenType::OpenParenthesis) {
                self.bump(); // eat open parenthesis
                self.eat_newlines_maybe()?;
                let fields = self
                    .with_options(self.options.nested(), |parser| {
                        parser.eat_pattern_field_list(TokenType::Comma, TokenType::CloseParenthesis)
                    })
                    .for_node_type(NodeType::Pattern)?;
                let pattern = Pattern::Tuple { fields };
                self.eat_token(TokenType::CloseParenthesis)?;
                self.tree.insert(pattern, self.get_span_from(&start))
            }
            // struct (without type)
            else if self.peek_is(TokenType::OpenBrace) {
                self.bump(); // eat open brace
                self.eat_newlines_maybe()?;
                let fields = self
                    .with_options(self.options.nested(), |parser| {
                        parser.eat_pattern_field_list(TokenType::Comma, TokenType::CloseBrace)
                    })
                    .for_node_type(NodeType::Pattern)?;
                let pattern = Pattern::Object { fields };
                self.eat_token(TokenType::CloseBrace)?;
                self.tree.insert(pattern, self.get_span_from(&start))
            }
            // array or slice
            else if self.peek_is(TokenType::OpenBracket) {
                self.bump(); // eat open bracket
                self.eat_newlines_maybe()?;
                let fields = self
                    .with_options(self.options.nested(), |parser| {
                        parser.eat_pattern_field_list(TokenType::Comma, TokenType::CloseBracket)
                    })
                    .for_node_type(NodeType::Pattern)?;
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::CloseBracket)?;
                self.tree
                    .insert(Pattern::Array { fields }, self.get_span_from(&start))
            }
            // literal expression
            else if self.is_scalar_literal_start() {
                let scalar_literal_id =
                    self.eat_scalar_literal().for_node_type(NodeType::Pattern)?;
                let expression_id = self.tree.insert(
                    Expression::ScalarLiteral(scalar_literal_id),
                    self.get_span_from(&start),
                );
                self.tree.insert(
                    Pattern::Expression {
                        value: expression_id,
                    },
                    self.get_span_from(&start),
                )
            }
            // null and undefined literals
            else if self.peek_identifier_str_is("null")
                || self.peek_identifier_str_is("undefined")
            {
                let literal = if self.peek_identifier_str_is("null") {
                    TypeLiteral::Null
                } else {
                    TypeLiteral::Undefined
                };
                self.bump(); // eat literal identifier

                let expression_id = self
                    .tree
                    .insert(Expression::TypeLiteral(literal), self.get_span_from(&start));
                self.tree.insert(
                    Pattern::Expression {
                        value: expression_id,
                    },
                    self.get_span_from(&start),
                )
            }
            // binding with expression or pattern
            else if !self.options.in_before_type
                && self.peek_identifier_is()
                && self.peek_next_is(TokenType::Colon)
            {
                let (name, name_span) = self.eat_binding_identifier_with_span()?;
                self.bump(); // eat colon
                let inner_pattern_id = self
                    .with_options(self.options.in_static().in_before_block(), |parser| {
                        parser.eat_pattern()
                    })
                    .for_node_type(NodeType::Pattern)?;
                let pattern_id = self.tree.insert(
                    Pattern::Binding {
                        mutability,
                        name,
                        pattern: Some(inner_pattern_id),
                    },
                    self.get_span_from(&start),
                );
                self.tree.set_main_span(pattern_id, name_span);
                pattern_id
            }
            // path or identifier
            else {
                let (path, name_span) = self
                    .eat_path_with_last_span()
                    .for_node_type(NodeType::Pattern)?;
                // tuple with path
                if self.peek_is(TokenType::OpenParenthesis) {
                    self.bump(); // eat open parenthesis
                    self.eat_newlines_maybe()?;
                    let fields = self
                        .eat_pattern_field_list(TokenType::Comma, TokenType::CloseParenthesis)
                        .for_node_type(NodeType::Pattern)?;
                    let expression_id = self.tree.insert(
                        Expression::Path {
                            path,
                            static_arguments: None,
                        },
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(expression_id, name_span);
                    let pattern = Pattern::TaggedTuple {
                        ty: expression_id,
                        fields,
                    };
                    self.eat_token(TokenType::CloseParenthesis)?;
                    self.tree.insert(pattern, self.get_span_from(&start))
                }
                // struct with path
                else if !self.options.in_before_block && self.peek_is(TokenType::OpenBrace) {
                    self.bump(); // eat open brace
                    self.eat_newlines_maybe()?;
                    let fields = self
                        .eat_pattern_field_list(TokenType::Comma, TokenType::CloseBrace)
                        .for_node_type(NodeType::Pattern)?;
                    let ty_id = self.tree.insert(
                        Expression::Path {
                            path,
                            static_arguments: None,
                        },
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(ty_id, name_span);
                    let pattern = Pattern::TaggedObject { ty: ty_id, fields };
                    self.eat_token(TokenType::CloseBrace)?;
                    self.tree.insert(pattern, self.get_span_from(&start))
                }
                // path
                else if path.segments.len() > 1 {
                    let expression_id = self.tree.insert(
                        Expression::Path {
                            path,
                            static_arguments: None,
                        },
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(expression_id, name_span);
                    self.tree.insert(
                        Pattern::Expression {
                            value: expression_id,
                        },
                        self.get_span_from(&start),
                    )
                }
                // nullish literals in patterns
                else if path.segments[0] == self.type_literal_identifiers.null_
                    || path.segments[0] == self.type_literal_identifiers.undefined
                {
                    let type_literal = if path.segments[0] == self.type_literal_identifiers.null_ {
                        TypeLiteral::Null
                    } else {
                        TypeLiteral::Undefined
                    };
                    let expression_id = self.tree.insert(
                        Expression::TypeLiteral(type_literal),
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(expression_id, name_span);
                    self.tree.insert(
                        Pattern::Expression {
                            value: expression_id,
                        },
                        self.get_span_from(&start),
                    )
                }
                // identifier
                else {
                    let pattern_id = self.tree.insert(
                        Pattern::Binding {
                            mutability,
                            name: path.segments[0],
                            pattern: None,
                        },
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(pattern_id, name_span);
                    pattern_id
                }
            }
        };

        // ------------------------------------------------------------
        // Postfix / infix patterns
        // ------------------------------------------------------------

        // must
        if self.peek_is(TokenType::Not) {
            self.bump(); // eat !
            let pattern = Pattern::Must(pattern_id);
            let pattern_id = self.tree.insert(pattern, self.get_span_from(&start));
            Ok(pattern_id)
        }
        // range
        else if self.peek_is(TokenType::Range) {
            self.bump(); // eat range
            let is_inclusive = if self.peek_is(TokenType::Assign) {
                self.bump(); // eat =
                true
            } else {
                false
            };
            let end_id = self.eat_pattern().for_node_type(NodeType::Pattern)?;
            let pattern = Pattern::Range {
                start: Some(pattern_id),
                end: Some(end_id),
                is_inclusive,
            };
            let pattern_id = self.tree.insert(pattern, self.get_span_from(&start));
            Ok(pattern_id)
        }
        // union
        else if self.peek_is(TokenType::ElementwiseOr) && !self.options.in_union_pattern {
            // eat all union "fields" (just unnamed patterns)
            let mut patterns: Vec<LocalNodeId<Pattern>> = vec![pattern_id];
            while self.peek_is(TokenType::ElementwiseOr) {
                self.bump(); // eat '|'
                let field_pattern_id = self
                    .with_options(self.options.in_union_pattern(), |parser| {
                        parser.eat_pattern()
                    })
                    .for_node_type(NodeType::Pattern)?;
                patterns.push(field_pattern_id);
            }
            let pattern = Pattern::Union { patterns };
            let pattern_id = self.tree.insert(pattern, self.get_span_from(&start));
            Ok(pattern_id)
        }
        // no infix
        else {
            Ok(pattern_id)
        }
    }

    /// Eat a pattern field list (like `x, y, z` or `1 | 2`).
    fn eat_pattern_field_list(
        &mut self,
        seperator: TokenType,
        terminator: TokenType,
    ) -> ParseResult<Vec<LocalNodeId<PatternField>>> {
        let mut fields: Vec<LocalNodeId<PatternField>> = Vec::new();
        let is_object_pattern = terminator == TokenType::CloseBrace;
        let enforce_terminal_spread =
            self.language.is_javascript() || self.language.is_typescript();
        let mut has_spread_field = false;

        while self.has_more_tokens() {
            // stop at the pattern terminator
            if self.peek_token_type() == terminator {
                break;
            }

            // in JS/TS: spread fields must be terminal
            if enforce_terminal_spread && has_spread_field {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // parse one field
            let field_start = self.mark();
            let mut name_span = None;
            let has_object_literal_alias_head =
                is_object_pattern && self.peek_object_pattern_alias_head();
            let can_start_named_or_spread_field = self.peek_name_is()
                || has_object_literal_alias_head
                || self.peek_mutability_is()
                || self.peek_is(TokenType::Spread);
            let pattern_field = {
                // elision: empty slot before separator (like `[,a]` or `[,,b]`)
                if self.peek_token_type() == seperator {
                    PatternField::Elision
                }
                // positional wildcard for tuples/arrays
                else if !is_object_pattern
                    && self.peek_identifier_str_is("_")
                    && !self.peek_next_is(TokenType::Colon)
                {
                    let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                    PatternField::Positional { pattern }
                }
                // computed property (object patterns only)
                else if is_object_pattern
                    && (self.peek_is(TokenType::OpenBracket)
                        || (self.peek_mutability_is() && self.peek_next_is(TokenType::OpenBracket)))
                {
                    let mutability = self.eat_mutability_maybe()?;
                    self.eat_token(TokenType::OpenBracket)?;
                    let key = self.with_options(
                        self.options.not_in_position().not_in_sequence_expression(),
                        |parser| parser.eat_expression(parser.options),
                    )?;
                    self.eat_token(TokenType::CloseBracket)?;
                    self.eat_newlines_maybe()?;
                    self.eat_token(TokenType::Colon)?;
                    let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                    let default = self.eat_pattern_field_default_maybe()?;
                    PatternField::Computed {
                        mutability,
                        key,
                        pattern: Some(pattern),
                        default,
                    }
                }
                // named field variants and spread fields
                else if can_start_named_or_spread_field {
                    // field modifiers
                    let mutability = if is_object_pattern
                        && self.peek_mutability_is()
                        && (self.peek_next_is(TokenType::Colon)
                            || self.peek_next_is(seperator)
                            || self.peek_next_is(terminator)
                            || self.peek_next_is(TokenType::Assign)
                            || self.peek_next_is(TokenType::Maybe))
                    {
                        None
                    } else {
                        self.eat_mutability_maybe()?
                    };

                    // spread fields
                    if self.peek_is(TokenType::Spread) {
                        self.bump(); // eat spread

                        // spread with omitted target is allowed before separators and terminators
                        let pattern = if self.peek_is(TokenType::Newline)
                            && (self.is_token_after_newlines(self.pos(), seperator)
                                || self.is_token_after_newlines(self.pos(), terminator))
                        {
                            self.eat_newlines_maybe()?;
                            None
                        } else if self.peek_token_type() == seperator || self.peek_is(terminator) {
                            None
                        } else {
                            let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                            Some(pattern)
                        };
                        PatternField::Spread {
                            mutability,
                            pattern,
                        }
                    }
                    // named alias and named pattern fields
                    else {
                        let has_named_colon_field = self.peek_name_is()
                            && self.peek_next_is(TokenType::Colon)
                            || is_object_pattern && self.peek_object_pattern_alias_head();
                        if has_named_colon_field {
                            let (name, _name_span) =
                                self.eat_pattern_field_name_with_span(terminator)?;
                            self.bump(); // eat colon

                            // named alias field
                            if self.peek_identifier_is() {
                                let (alias, alias_span) = self
                                    .eat_binding_identifier_with_span()
                                    .for_node_type(NodeType::Pattern)?;
                                name_span = Some(alias_span);
                                let default = self.eat_pattern_field_default_maybe()?;
                                PatternField::Alias {
                                    mutability,
                                    name,
                                    alias,
                                    default,
                                }
                            }
                            // named field with nested pattern
                            else {
                                let pattern =
                                    self.eat_pattern().for_node_type(NodeType::Pattern)?;
                                let default = self.eat_pattern_field_default_maybe()?;
                                PatternField::Named {
                                    mutability,
                                    name,
                                    pattern: Some(pattern),
                                    default,
                                }
                            }
                        }
                        // named shorthand field
                        else {
                            let (name, span) = self.eat_pattern_field_name_with_span(terminator)?;
                            name_span = Some(span);
                            let default = self.eat_pattern_field_default_maybe()?;
                            PatternField::Named {
                                mutability,
                                name,
                                pattern: None,
                                default,
                            }
                        }
                    }
                }
                // positional field
                else {
                    let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                    PatternField::Positional { pattern }
                }
            };
            let pattern_field_id = self
                .tree
                .insert(pattern_field, self.get_span_from(&field_start));
            if let Some(name_span) = name_span {
                self.tree.set_main_span(pattern_field_id, name_span);
            }
            fields.push(pattern_field_id);

            // in JS/TS: no separator after spread fields
            if enforce_terminal_spread
                && matches!(self.tree.get(pattern_field_id), PatternField::Spread { .. })
            {
                has_spread_field = true;
                let has_separator_after_spread = self.peek_token_type() == seperator;
                let has_non_terminal_newline_after_spread = self.peek_is(TokenType::Newline)
                    && !self.is_token_after_newlines(self.pos(), terminator);
                if has_separator_after_spread || has_non_terminal_newline_after_spread {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
            }

            // consume separators and newline separators
            if self.peek_token_type() == seperator || self.peek_is(TokenType::Newline) {
                self.bump(); // eat separator
                self.eat_newlines_maybe()?;
            } else {
                break;
            }
        }

        Ok(fields)
    }

    // eat a pattern field default assignment if present
    fn eat_pattern_field_default_maybe(&mut self) -> ParseResult<Option<LocalNodeId<Expression>>> {
        if !self.peek_is(TokenType::Assign) {
            return Ok(None);
        }

        self.bump(); // eat assign
        let default = self.with_options(
            self.options.not_in_position().not_in_sequence_expression(),
            |parser| parser.eat_expression(parser.options),
        )?;
        Ok(Some(default))
    }

    // check whether object pattern field head is a literal alias key before `:`
    fn peek_object_pattern_alias_head(&mut self) -> bool {
        let has_numeric_alias_head =
            self.peek_numeric_literal_is() && self.peek_next_is(TokenType::Colon);
        let has_boolean_alias_head =
            self.peek_boolean_pattern_name_head() && self.peek_next_is(TokenType::Colon);
        has_numeric_alias_head || has_boolean_alias_head
    }

    // check whether next token is a boolean literal key in object patterns
    fn peek_boolean_pattern_name_head(&mut self) -> bool {
        self.peek().is_ok_and(|token| {
            token.token.ty == TokenType::Literal
                && matches!(token.token.literal, Some(LiteralType::Boolean { .. }))
        })
    }

    // eat a name for object pattern fields
    fn eat_pattern_field_name_with_span(
        &mut self,
        terminator: TokenType,
    ) -> ParseResult<(Name, destack_source::Span)> {
        let is_numeric_object_key =
            terminator == TokenType::CloseBrace && self.peek_numeric_literal_is();
        if is_numeric_object_key {
            return self.eat_numeric_pattern_name_with_span();
        }
        let is_boolean_object_key =
            terminator == TokenType::CloseBrace && self.peek_boolean_pattern_name_head();
        if is_boolean_object_key {
            return self.eat_boolean_pattern_name_with_span();
        }

        self.eat_name_with_span()
    }

    // eat a numeric pattern field name as Name::Number
    fn eat_numeric_pattern_name_with_span(&mut self) -> ParseResult<(Name, destack_source::Span)> {
        let token = *self.peek_numeric_literal()?;
        let key_string = self.file.span_str(token.span).to_string();

        let numeric_literal = self.eat_scalar_literal()?;
        if !matches!(
            numeric_literal,
            ScalarLiteral::Integer(_) | ScalarLiteral::Float(_) | ScalarLiteral::Bigint(_)
        ) {
            return Err(ParseError::unexpected(token.span));
        }

        let key_name = self.strings.intern(key_string);
        Ok((Name::Number(key_name), token.span))
    }

    // eat a boolean pattern field name as Name::Identifier
    fn eat_boolean_pattern_name_with_span(&mut self) -> ParseResult<(Name, destack_source::Span)> {
        let token = *self.peek()?;
        if token.token.ty != TokenType::Literal
            || !matches!(token.token.literal, Some(LiteralType::Boolean { .. }))
        {
            return Err(ParseError::unexpected(token.span));
        }

        self.bump();
        let key_name = self.get_span_str(token.span).to_owned();
        let key_name = self.strings.intern(&key_name);
        Ok((Name::Identifier(key_name), token.span))
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{Expression, Mutability, Name, Pattern, PatternField, ScalarLiteral};
    use destack_source::LanguageType;

    use crate::{
        TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_pattern_wildcard() {
        // _
        let mut test = TestParser::new("_");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Wildcard);
    }

    #[test]
    fn test_parse_pattern_reference() {
        // &_
        let mut test = TestParser::new("&_");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        // &
        assert_node!(parser.tree, pattern_id,
            Pattern::ReferenceOf { mutability: Some(mutability), right } => {
                // var
                assert_eq!(*mutability, Mutability::Mutable);
                // _
                assert_node!(parser.tree, *right, Pattern::Wildcard)
            }
        );

        // &1
        let mut test = TestParser::new("&1");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        // &
        assert_node!(parser.tree, pattern_id, Pattern::ReferenceOf { mutability: Some(mutability), right } => {
            assert_eq!(*mutability, Mutability::Mutable);
            // 1
            assert_node!(parser.tree, *right, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        })
    }

    #[test]
    fn test_parse_pattern_value() {
        // ^x
        let mut test = TestParser::new("^x");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::ValueOf { mutability: Some(mutability), right } => {
            assert_eq!(*mutability, Mutability::Mutable);
            assert_node!(parser.tree, *right, Pattern::Binding { mutability: None, name, pattern: None } => {
                assert_string!(parser, *name, "x");
            });
        });
    }

    #[test]
    fn test_parse_pattern_identifier() {
        let mut test = TestParser::new("x");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Binding { mutability: None, name, pattern: None } => {
            assert_string!(parser, *name, "x");
        });
    }

    #[test]
    fn test_parse_pattern_path() {
        let mut test = TestParser::new("MyEnum.A");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Expression { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "MyEnum.A");
        });
    }

    #[test]
    fn test_parse_pattern_tuple() {
        let mut test = TestParser::new("(x: 1, 2, var y, const z, ...)");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        // (x: 1, 2, var y, const z, ...)
        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
            assert_eq!(fields.len(), 5);

            // x: 1
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), mutability: None, default: None } => {
                assert_name!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });

            // 2
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });

            // var y
            assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, mutability: Some(mutability), default: None } => {
                assert_name!(parser, *name, "y");
                assert_eq!(*mutability, Mutability::Mutable);
            });

            // const z
            assert_node!(parser.tree, fields[3], PatternField::Named { name, pattern: None, mutability: Some(mutability), default: None } => {
                assert_name!(parser, *name, "z");
                assert_eq!(*mutability, Mutability::Immutable);
            });

            // ...
            assert_node!(parser.tree, fields[4], PatternField::Spread { mutability: None, pattern: None } => {
            });
        });
    }

    #[test]
    fn test_parse_pattern_tuple_with_path() {
        let mut test = TestParser::new("Result.Success(_, ...)");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        // Result.Success(_, ..)
        assert_node!(parser.tree, pattern_id, Pattern::TaggedTuple { ty, fields } => {
            // Result.Success
            let _ = ty; // ty is required for TaggedTuple
            assert_eq!(fields.len(), 2);

            // _
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            });

            // ..
            assert_node!(parser.tree, fields[1], PatternField::Spread { mutability: None, pattern: None } => {
            });
        });
    }

    #[test]
    fn test_parse_pattern_object_field_const_alias_in_typescript() {
        let mut test =
            TestParser::new_with_options("{ const: value, title }", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        // { const: value, title }
        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 2);

            // const: value
            assert_node!(parser.tree, fields[0], PatternField::Alias { mutability: None, name, alias, default: None } => {
                assert_name!(parser, *name, "const");
                assert_string!(parser, *alias, "value");
            });

            // title
            assert_node!(parser.tree, fields[1], PatternField::Named { mutability: None, name, pattern: None, default: None } => {
                assert_name!(parser, *name, "title");
            });
        });
    }

    #[test]
    fn test_parse_pattern_tuple_spread_non_terminal_destack() {
        let mut test = TestParser::new("(x, ...rest, z)");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
            assert_eq!(fields.len(), 3);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, mutability: None, default: None } => {
                assert_name!(parser, *name, "x");
            });
            assert_node!(parser.tree, fields[1], PatternField::Spread { mutability: None, pattern: Some(pattern) } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "rest");
                });
            });
            assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, mutability: None, default: None } => {
                assert_name!(parser, *name, "z");
            });
        });
    }

    #[test]
    fn test_parse_pattern_tuple_newline_separated() {
        let mut test = TestParser::new(
            "
(
    x: 1
    2,
    ...
)",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let pattern_id = parser.eat_pattern().unwrap();

        // (x: 1, 2, ..)
        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
            assert_eq!(fields.len(), 3);

            // x: 1
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), mutability: None, default: None } => {
                assert_name!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });

            // 2
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });

            // ..
            assert_node!(parser.tree, fields[2], PatternField::Spread { mutability: None, pattern: None } => {
            });
        });
    }

    #[test]
    fn test_parse_pattern_union() {
        // 1 | 2 | 3
        let mut test = TestParser::new("1 | 2 | 3");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
            assert_eq!(patterns.len(), 3);

            // 1
            assert_node!(parser.tree, patterns[0], Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });

            // 2
            assert_node!(parser.tree, patterns[1], Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });

            // 3
            assert_node!(parser.tree, patterns[2], Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_anonymous() {
        let mut test = TestParser::new("{ x: 1, y, var z, const w: 4, ... }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 5);

            // x: 1
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), mutability: None, default: None } => {
                assert_name!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });

            // y
            assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, mutability: None, default: None } => {
                assert_name!(parser, *name, "y");
            });

            // var z
            assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, mutability: Some(mutability), default: None } => {
                assert_name!(parser, *name, "z");
                assert_eq!(*mutability, Mutability::Mutable);
            });

            // const w: 4
            assert_node!(parser.tree, fields[3], PatternField::Named { name, pattern: Some(pattern), mutability: Some(mutability), default: None } => {
                assert_name!(parser, *name, "w");
                assert_eq!(*mutability, Mutability::Immutable);
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
                });
            });

            // ..
            assert_node!(parser.tree, fields[4], PatternField::Spread { mutability: None, pattern: None } => {
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_numeric_name_aliases() {
        let mut test = TestParser::new("{ 0: fieldNameOrOptions, 1: from, length: argc }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 3);
            assert_node!(parser.tree, fields[0], PatternField::Alias { name, alias, .. } => {
                assert_node!(name, Name::Number(name) => {
                    assert_string!(parser, *name, "0");
                });
                assert_string!(parser, *alias, "fieldNameOrOptions");
            });
            assert_node!(parser.tree, fields[1], PatternField::Alias { name, alias, .. } => {
                assert_node!(name, Name::Number(name) => {
                    assert_string!(parser, *name, "1");
                });
                assert_string!(parser, *alias, "from");
            });
            assert_node!(parser.tree, fields[2], PatternField::Alias { name, alias, .. } => {
                assert_name!(parser, *name, "length");
                assert_string!(parser, *alias, "argc");
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_boolean_name_aliases() {
        let mut test = TestParser::new_with_options(
            "{ false: decorators, true: metadata }",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Alias { name, alias, .. } => {
                assert_name!(parser, *name, "false");
                assert_string!(parser, *alias, "decorators");
            });
            assert_node!(parser.tree, fields[1], PatternField::Alias { name, alias, .. } => {
                assert_name!(parser, *name, "true");
                assert_string!(parser, *alias, "metadata");
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_numeric_literal_field() {
        let mut test = TestParser::new_with_options("{ 5 }", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(5)));
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_computed_field() {
        let mut test = TestParser::new("{ [key]: value }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Computed { mutability: None, key, pattern, default } => {
                assert!(default.is_none());
                assert_node!(parser.tree, *key, Expression::Path { path, static_arguments: None } => {
                    assert_path!(parser, *path, "key");
                });
                assert_node!(parser.tree, pattern.unwrap(), Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "value");
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_with_path() {
        let mut test = TestParser::new("Vector2 { x: 0, y }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::TaggedObject { ty, fields } => {
            assert_node!(parser.tree, *ty, Expression::Path { path, static_arguments: None } => {
                assert_path!(parser, *path, "Vector2");
            });
            assert_eq!(fields.len(), 2);

            // x: 0
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), mutability: None, default: None } => {
                assert_name!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
                });
            });

            // y
            assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, mutability: None, default: None } => {
                assert_name!(parser, *name, "y");
            });
        });
    }

    #[test]
    fn test_parse_pattern_slice() {
        let mut test = TestParser::new("[1, ...]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Array { fields } => {
            assert_eq!(fields.len(), 2);
            // 1
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });
            // ...
            assert_node!(parser.tree, fields[1], PatternField::Spread { mutability: None, pattern: None } => {
            });
        });
    }

    /// Parse a spread field with an array pattern.
    #[test]
    fn test_parse_pattern_spread_array_pattern() {
        let mut test = TestParser::new("[...[x, y]]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Array { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Spread { mutability: None, pattern: Some(pattern) } => {
                assert_node!(parser.tree, *pattern, Pattern::Array { fields } => {
                    assert_eq!(fields.len(), 2);
                    assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, .. } => {
                        assert_name!(parser, *name, "x");
                    });
                    assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, .. } => {
                        assert_name!(parser, *name, "y");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_object_readonly_shorthand_typescript() {
        let mut test = TestParser::new_with_options("{ readonly }", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                assert_name!(parser, *name, "readonly");
            });
        });
    }

    #[test]
    fn test_parse_pattern_object_readonly_shorthand_destack() {
        let mut test = TestParser::new("{ readonly }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                assert_name!(parser, *name, "readonly");
            });
        });
    }

    #[test]
    fn test_parse_pattern_object_readonly_modifier_with_name_destack() {
        let mut test = TestParser::new("{ readonly value }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, mutability: Some(mutability), pattern: None, default: None } => {
                assert_name!(parser, *name, "value");
                assert_eq!(*mutability, Mutability::Immutable);
            });
        });
    }

    #[test]
    fn test_parse_pattern_object_spread_newline_before_terminator_typescript() {
        let mut test =
            TestParser::new_with_options("{\n  onSuccess,\n  ...rest\n}", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, mutability: None, default: None } => {
                assert_name!(parser, *name, "onSuccess");
            });
            assert_node!(parser.tree, fields[1], PatternField::Spread { mutability: None, pattern: Some(pattern) } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "rest");
                });
            });
        });
    }

    #[test]
    fn test_parse_pattern_must() {
        let mut test = TestParser::new("1!");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Must(inner) => {
            assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
    }

    #[test]
    fn test_parse_pattern_range() {
        let mut test = TestParser::new("1..4");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Range { start, end, is_inclusive } => {
            let start = start.expect("expected range start");
            let end = end.expect("expected range end");
            assert!(!is_inclusive);

            // 1
            assert_node!(parser.tree, start, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });

            // 4
            assert_node!(parser.tree, end, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
            });
        });
    }

    #[test]
    fn test_parse_pattern_range_inclusive() {
        let mut test = TestParser::new("1..=4");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Range { start, end, is_inclusive } => {
            let start = start.expect("expected range start");
            let end = end.expect("expected range end");
            assert!(*is_inclusive);
            // 1
            assert_node!(parser.tree, start, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            // 4
            assert_node!(parser.tree, end, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
            });
        });
    }

    #[test]
    fn test_parse_pattern_binding_with_pattern() {
        let mut test = TestParser::new("const value: 5");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Binding { mutability: Some(mutability), name, pattern: Some(pattern) } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_string!(parser, *name, "value");
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(5)));
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_elision() {
        // [,a] - elision before 'a'
        let mut test = TestParser::new("[,a]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Array { fields } => {
            assert_eq!(fields.len(), 2);

            // elision (empty slot)
            assert_node!(parser.tree, fields[0], PatternField::Elision);

            // a (identifiers are parsed as Named shorthand)
            assert_node!(parser.tree, fields[1], PatternField::Named { mutability: None, name, pattern: None, default: None } => {
                assert_name!(parser, *name, "a");
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_multiple_elisions() {
        // [,,a] - two elisions before 'a'
        let mut test = TestParser::new("[,,a]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Array { fields } => {
            assert_eq!(fields.len(), 3);

            // first elision
            assert_node!(parser.tree, fields[0], PatternField::Elision);

            // second elision
            assert_node!(parser.tree, fields[1], PatternField::Elision);

            // a (identifiers are parsed as Named shorthand)
            assert_node!(parser.tree, fields[2], PatternField::Named { mutability: None, name, pattern: None, default: None } => {
                assert_name!(parser, *name, "a");
            });
        });
    }

    #[test]
    fn test_parse_pattern_array_trailing_elision() {
        // [a,] - element followed by trailing comma (not elision)
        let mut test = TestParser::new("[a,]");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Array { fields } => {
            // trailing comma doesn't create elision, just 'a'
            assert_eq!(fields.len(), 1);

            // a (identifiers are parsed as Named shorthand)
            assert_node!(parser.tree, fields[0], PatternField::Named { mutability: None, name, pattern: None, default: None } => {
                assert_name!(parser, *name, "a");
            });
        });
    }

    #[test]
    fn test_parse_object_pattern_defaults_do_not_consume_following_fields_javascript() {
        // {a,b=1,c:d,e:f=2,[g]:[h]}
        let mut test =
            TestParser::new_with_options("{a,b=1,c:d,e:f=2,[g]:[h]}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 5);

            // b=1
            assert_node!(parser.tree, fields[1], PatternField::Named { default, .. } => {
                assert!(default.is_some());
            });

            // c:d
            assert_node!(parser.tree, fields[2], PatternField::Alias { default, .. } => {
                assert!(default.is_none());
            });

            // e:f=2
            assert_node!(parser.tree, fields[3], PatternField::Alias { default, .. } => {
                assert!(default.is_some());
            });

            // [g]:[h]
            assert_node!(parser.tree, fields[4], PatternField::Computed { pattern, .. } => {
                assert!(pattern.is_some());
            });
        });
    }

    #[test]
    fn test_parse_object_pattern_alias_and_computed_defaults_javascript() {
        // {c, d:e=1, [f]:g=2, h=i}
        let mut test =
            TestParser::new_with_options("{c, d:e=1, [f]:g=2, h=i}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 4);

            // d:e=1
            assert_node!(parser.tree, fields[1], PatternField::Alias { default, .. } => {
                assert!(default.is_some());
            });

            // [f]:g=2
            assert_node!(parser.tree, fields[2], PatternField::Computed { default, .. } => {
                assert!(default.is_some());
            });

            // h=i
            assert_node!(parser.tree, fields[3], PatternField::Named { default, .. } => {
                assert!(default.is_some());
            });
        });
    }
}
