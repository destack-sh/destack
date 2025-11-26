use crate::parse::prelude::*;
use crate::{ParseResult, Parser};

use dyst_ast::{Expression, LocalNodeId, NodeType, Pattern, PatternField, TokenType};

impl Parser {
    /// Eat a pattern.
    ///
    /// Examples:
    /// ```
    /// _
    /// ..
    /// 1
    /// 2 | 3
    /// 4..6
    /// (x, 0, ..)
    /// { a: 2 }
    /// Success(_)
    /// Vector2 { x: 0, y, z: zed }
    /// geom.Mesh<2, float32> { vertices: [2, ..] }
    /// ```
    pub fn eat_pattern(&mut self) -> ParseResult<LocalNodeId<Pattern>> {
        let start = self.mark();

        // mutability
        let mutability = self.eat_mutability_maybe()?;

        // ------------------------------------------------------------
        // Primary patterns
        // ------------------------------------------------------------
        let pattern_id = {
            // wildcard
            if self.peek_token(TokenType::Wildcard).is_ok() {
                self.bump(); // eat wildcard
                self.tree
                    .insert(Pattern::Wildcard, self.get_span_from(start))
            }
            // rest
            else if self.peek_token(TokenType::Spread).is_ok() {
                self.bump(); // eat range
                let name = if self.peek_identifier().is_ok() {
                    Some(self.eat_identifier()?)
                } else {
                    None
                };
                self.tree
                    .insert(Pattern::Rest { name }, self.get_span_from(start))
            }
            // reference of
            else if self.peek_token(TokenType::Multiply).is_ok()
                || self.peek_token(TokenType::ElementwiseAnd).is_ok()
            {
                self.bump(); // eat & or *
                let mutability = self.eat_mutability_maybe()?;
                let right_id = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                self.tree.insert(
                    Pattern::ReferenceOf {
                        mutability,
                        right: right_id,
                    },
                    self.get_span_from(start),
                )
            }
            // value of
            else if self.peek_token(TokenType::ElementwiseXor).is_ok() {
                self.bump(); // eat ^
                let mutability = self.eat_mutability_maybe()?;
                let right_id = self.eat_pattern().for_node_type(NodeType::Pattern)?;
                self.tree.insert(
                    Pattern::ValueOf {
                        mutability,
                        right: right_id,
                    },
                    self.get_span_from(start),
                )
            }
            // tuple (without type, no struct tuples)
            else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.bump(); // eat open parenthesis
                self.eat_newlines_maybe()?;
                let fields = self
                    .with_options(self.options.nested(), |parser| {
                        parser.eat_pattern_field_list(TokenType::Comma, TokenType::CloseParenthesis)
                    })
                    .for_node_type(NodeType::Pattern)?;
                let pattern = Pattern::Tuple { ty: None, fields };
                self.eat_token(TokenType::CloseParenthesis)?;
                self.tree.insert(pattern, self.get_span_from(start))
            }
            // struct (without type)
            else if self.peek_token(TokenType::OpenBrace).is_ok() {
                self.bump(); // eat open brace
                self.eat_newlines_maybe()?;
                let fields = self
                    .with_options(self.options.nested(), |parser| {
                        parser.eat_pattern_field_list(TokenType::Comma, TokenType::CloseBrace)
                    })
                    .for_node_type(NodeType::Pattern)?;
                let pattern = Pattern::Struct { ty: None, fields };
                self.eat_token(TokenType::CloseBrace)?;
                self.tree.insert(pattern, self.get_span_from(start))
            }
            // array or slice
            else if self.peek_token(TokenType::OpenBracket).is_ok() {
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
                    .insert(Pattern::Slice { fields }, self.get_span_from(start))
            }
            // literal expression
            else if self.peek_scalar_literal().is_ok() {
                let scalar_literal_id =
                    self.eat_scalar_literal().for_node_type(NodeType::Pattern)?;
                let expression_id = self.tree.insert(
                    Expression::ScalarLiteral(scalar_literal_id),
                    self.get_span_from(start),
                );
                self.tree.insert(
                    Pattern::Expression {
                        value: expression_id,
                    },
                    self.get_span_from(start),
                )
            }
            // binding with expression or pattern
            else if !self.options.in_before_type
                && self.peek_identifier().is_ok()
                && self.peek_next_token(TokenType::Colon).is_ok()
            {
                let name = self.eat_identifier()?;
                self.bump(); // eat colon
                let inner_pattern_id = self
                    .with_options(self.options.in_static().in_before_block(), |parser| {
                        parser.eat_pattern()
                    })
                    .for_node_type(NodeType::Pattern)?;
                self.tree.insert(
                    Pattern::Binding {
                        mutability,
                        name,
                        pattern: Some(inner_pattern_id),
                    },
                    self.get_span_from(start),
                )
            }
            // path or identifier
            else {
                let path = self.eat_path().for_node_type(NodeType::Pattern)?;
                // tuple with path
                if self.peek_token(TokenType::OpenParenthesis).is_ok() {
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
                        self.get_span_from(start),
                    );
                    let pattern = Pattern::Tuple {
                        ty: Some(expression_id),
                        fields,
                    };
                    self.eat_token(TokenType::CloseParenthesis)?;
                    self.tree.insert(pattern, self.get_span_from(start))
                }
                // struct with path
                else if !self.options.in_before_block
                    && self.peek_token(TokenType::OpenBrace).is_ok()
                {
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
                        self.get_span_from(start),
                    );
                    let pattern = Pattern::Struct {
                        ty: Some(ty_id),
                        fields,
                    };
                    self.eat_token(TokenType::CloseBrace)?;
                    self.tree.insert(pattern, self.get_span_from(start))
                }
                // path
                else if path.segments.len() > 1 {
                    let expression_id = self.tree.insert(
                        Expression::Path {
                            path,
                            static_arguments: None,
                        },
                        self.get_span_from(start),
                    );
                    self.tree.insert(
                        Pattern::Expression {
                            value: expression_id,
                        },
                        self.get_span_from(start),
                    )
                }
                // identifier
                else {
                    self.tree.insert(
                        Pattern::Binding {
                            mutability,
                            name: path.segments[0],
                            pattern: None,
                        },
                        self.get_span_from(start),
                    )
                }
            }
        };

        // ------------------------------------------------------------
        // Postfix / infix patterns
        // ------------------------------------------------------------

        // unwrap
        if self.peek_token(TokenType::Maybe).is_ok() {
            self.bump(); // eat ?
            let pattern = Pattern::Maybe(pattern_id);
            let pattern_id = self.tree.insert(pattern, self.get_span_from(start));
            Ok(pattern_id)
        }
        // range
        else if self.peek_token(TokenType::Range).is_ok() {
            self.bump(); // eat range
            let end_id = self.eat_pattern().for_node_type(NodeType::Pattern)?;
            let pattern = Pattern::Range {
                start: Some(pattern_id),
                end: Some(end_id),
                is_inclusive: false,
            };
            let pattern_id = self.tree.insert(pattern, self.get_span_from(start));
            Ok(pattern_id)
        }
        // union
        else if self.peek_token(TokenType::ElementwiseOr).is_ok()
            && !self.options.in_union_pattern
        {
            // eat all union "fields" (just unnamed patterns)
            let mut patterns: Vec<LocalNodeId<Pattern>> = vec![pattern_id];
            while self.peek_token(TokenType::ElementwiseOr).is_ok() {
                self.bump(); // eat '|'
                let field_pattern_id = self
                    .with_options(self.options.in_union_pattern(), |parser| {
                        parser.eat_pattern()
                    })
                    .for_node_type(NodeType::Pattern)?;
                patterns.push(field_pattern_id);
            }
            let pattern = Pattern::Union { patterns };
            let pattern_id = self.tree.insert(pattern, self.get_span_from(start));
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
        while self.peek().is_ok() {
            if self.peek_token(terminator).is_ok() {
                break;
            }

            // field
            let field_start = self.mark();
            let pattern_field = {
                // named or named alias
                if self.peek_name().is_ok() || self.peek_mutability().is_ok() {
                    // mutability
                    let mutability = self.eat_mutability_maybe()?;

                    // name
                    let name = self.eat_name()?;

                    // alias or pattern
                    if self.peek_colon().is_ok() {
                        self.bump(); // eat colon
                        // named alias
                        if self.peek_identifier().is_ok() {
                            let alias = self.eat_identifier().for_node_type(NodeType::Pattern)?;
                            // default
                            let default = if self.peek_token(TokenType::Assign).is_ok() {
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
                            let default = if self.peek_token(TokenType::Assign).is_ok() {
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
                        // default
                        let default = if self.peek_token(TokenType::Assign).is_ok() {
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
                .insert(pattern_field, self.get_span_from(field_start));
            fields.push(pattern_field_id);

            // separator or newline
            if self.peek_token(seperator).is_ok() || self.peek_token(TokenType::Newline).is_ok() {
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
    use dyst_ast::{Expression, Mutability, Pattern, PatternField, ScalarLiteral};

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
    fn test_parse_pattern_rest() {
        // ..
        let mut test = TestParser::new("...");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Rest { name: None });
    }

    #[test]
    fn test_parse_pattern_rest_with_name() {
        // ...rest
        let mut test = TestParser::new("...rest");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Rest { name: Some(name) } => {
            assert_string!(parser, *name, "rest");
        });
    }

    #[test]
    fn test_parse_pattern_reference() {
        // &var _
        let mut test = TestParser::new("&var _");
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
        assert_node!(parser.tree, pattern_id, Pattern::ReferenceOf { mutability: None, right } => {
            // 1
            assert_node!(parser.tree, *right, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        })
    }

    #[test]
    fn test_parse_pattern_value() {
        // ^var _
        let mut test = TestParser::new("^mut x");
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

            // ..
            assert_node!(parser.tree, fields[4], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Rest { name: None });
            });
        });
    }

    #[test]
    fn test_parse_pattern_tuple_with_path() {
        let mut test = TestParser::new("Result.Success(_, ...)");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        // Result.Success(_, ..)
        assert_node!(parser.tree, pattern_id, Pattern::Tuple { ty, fields } => {
            // Result.Success
            assert!(ty.is_some());
            assert_eq!(fields.len(), 2);

            // _
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            });

            // ..
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Rest { name: None });
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
            assert_node!(parser.tree, fields[2], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Rest { name: None });
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

        assert_node!(parser.tree, pattern_id, Pattern::Struct { ty, fields } => {
            assert!(ty.is_none());
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
            assert_node!(parser.tree, fields[4], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Rest { name: None });
            });
        });
    }

    #[test]
    fn test_parse_pattern_struct_with_path() {
        let mut test = TestParser::new("Vector2 { x: 0, y }");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Struct { ty, fields } => {
            let ty = ty.expect("expected struct type");
            assert_node!(parser.tree, ty, Expression::Path { path, static_arguments: None } => {
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

        assert_node!(parser.tree, pattern_id, Pattern::Slice { fields } => {
            assert_eq!(fields.len(), 2);

            // 1
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
            });

            // ..
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Rest { name: None });
            });
        });
    }

    #[test]
    fn test_parse_pattern_maybe() {
        let mut test = TestParser::new("1?");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Maybe(inner) => {
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
}
