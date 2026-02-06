use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use destack_ast::{Expression, LocalNodeId, NodeType, Pattern, PatternField, TokenType};

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
            if self.peek_identifier_str("_").is_ok() {
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
            // binding with expression or pattern
            else if !self.options.in_before_type
                && self.peek_identifier().is_ok()
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
        let in_js_or_ts = self.language.is_javascript() || self.language.is_typescript();
        let mut has_spread_field = false;
        while self.has_more_tokens() {
            if self.peek_token_type() == terminator {
                break;
            }

            // in js and ts: spread fields must be terminal
            if in_js_or_ts && has_spread_field {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            // field
            let field_start = self.mark();
            let mut name_span = None;
            let pattern_field = {
                // elision: empty slot before separator (like `[,a]` or `[,,b]`)
                if self.peek_token_type() == seperator {
                    PatternField::Elision
                }
                // positional wildcard for tuples/arrays
                else if terminator != TokenType::CloseBrace
                    && self.peek_identifier_str("_").is_ok()
                    && !self.peek_next_is(TokenType::Colon)
                {
                    let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                    PatternField::Positional { pattern }
                }
                // computed property (object patterns only)
                else if terminator == TokenType::CloseBrace
                    && (self.peek_is(TokenType::OpenBracket)
                        || (self.peek_mutability().is_ok()
                            && self.peek_next_is(TokenType::OpenBracket)))
                {
                    let mutability = self.eat_mutability_maybe()?;
                    self.eat_token(TokenType::OpenBracket)?;
                    let key = self.with_options(self.options.not_in_position(), |parser| {
                        parser.eat_expression()
                    })?;
                    self.eat_token(TokenType::CloseBracket)?;
                    self.eat_newlines_maybe()?;
                    self.eat_token(TokenType::Colon)?;
                    let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                    let default = if self.peek_is(TokenType::Assign) {
                        self.bump(); // eat assign
                        let default = self
                            .with_options(self.options.not_in_position(), |parser| {
                                parser.eat_expression()
                            })?;
                        Some(default)
                    } else {
                        None
                    };
                    PatternField::Computed {
                        mutability,
                        key,
                        pattern: Some(pattern),
                        default,
                    }
                }
                // named or named alias or spread
                else if self.peek_name().is_ok()
                    || self.peek_mutability().is_ok()
                    || self.peek_is(TokenType::Spread)
                {
                    // mutability
                    let mutability = self.eat_mutability_maybe()?;

                    // spread
                    if self.peek_is(TokenType::Spread) {
                        self.bump(); // eat spread
                        let pattern = if self.peek_is(TokenType::Newline)
                            && (self
                                .peek_token_after_newlines(self.pos(), seperator)
                                .is_ok()
                                || self
                                    .peek_token_after_newlines(self.pos(), terminator)
                                    .is_ok())
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
                    // alias or pattern
                    else if self.peek_name().is_ok() && self.peek_next_is(TokenType::Colon) {
                        let (name, _name_span) = self.eat_name_with_span()?;
                        self.bump(); // eat colon
                        // named alias
                        if self.peek_identifier().is_ok() {
                            let (alias, alias_span) = self
                                .eat_binding_identifier_with_span()
                                .for_node_type(NodeType::Pattern)?;
                            name_span = Some(alias_span);
                            // default
                            let default = if self.peek_is(TokenType::Assign) {
                                self.bump(); // eat assign
                                let default = self
                                    .with_options(self.options.not_in_position(), |parser| {
                                        parser.eat_expression()
                                    })?;
                                Some(default)
                            } else {
                                None
                            };
                            PatternField::Alias {
                                mutability,
                                name,
                                alias,
                                default,
                            }
                        }
                        // named with pattern
                        else {
                            let pattern = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                            // default
                            let default = if self.peek_is(TokenType::Assign) {
                                self.bump(); // eat assign
                                let default = self
                                    .with_options(self.options.not_in_position(), |parser| {
                                        parser.eat_expression()
                                    })?;
                                Some(default)
                            } else {
                                None
                            };
                            PatternField::Named {
                                mutability,
                                name,
                                pattern: Some(pattern),
                                default,
                            }
                        }
                    }
                    // named without pattern
                    else {
                        let (name, span) = self.eat_name_with_span()?;
                        name_span = Some(span);
                        // default
                        let default = if self.peek_is(TokenType::Assign) {
                            self.bump(); // eat assign
                            let default = self
                                .with_options(self.options.not_in_position(), |parser| {
                                    parser.eat_expression()
                                })?;
                            Some(default)
                        } else {
                            None
                        };
                        PatternField::Named {
                            mutability,
                            name,
                            pattern: None,
                            default,
                        }
                    }
                }
                // positional
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

            // in js and ts: no separator after spread fields
            if in_js_or_ts && matches!(self.tree.get(pattern_field_id), PatternField::Spread { .. })
            {
                has_spread_field = true;
                if self.peek_token_type() == seperator || self.peek_is(TokenType::Newline) {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
            }

            // separator or newline
            if self.peek_token_type() == seperator || self.peek_is(TokenType::Newline) {
                self.bump(); // eat separator
                self.eat_newlines_maybe()?;
            } else {
                break;
            }
        }

        Ok(fields)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{Expression, Mutability, Pattern, PatternField, ScalarLiteral};

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
}
