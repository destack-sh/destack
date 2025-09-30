//! Parse patterns.

use crate::parse::prelude::*;
use dyst_token::TokenType;

use crate::parse::ParserOptions;
use crate::{
    ExpressionParserOptions, Keyword, Mutability, NodeId, NodeType, ParseError, ParseResult,
    Parser, Pattern, PatternField,
};

impl<'a> Parser<'a> {
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
    /// Success(_)
    /// Vector2 { x: 0, y, z: zed }
    /// geom.Mesh<2, float32> { vertices: [2, ..] }
    /// ```
    pub fn eat_pattern(
        &mut self,
        options: ExpressionParserOptions,
    ) -> ParseResult<NodeId<Pattern>> {
        let start = self.mark();

        // ------------------------------------------------------------
        // Primary patterns
        // ------------------------------------------------------------
        let pattern_id = {
            // wildcard
            if self.peek_token(TokenType::Wildcard).is_ok() {
                self.bump(); // eat wildcard
                self.tree
                    .allocate(Pattern::Wildcard, self.get_span_from(start))
            }
            // rest
            else if self.peek_token(TokenType::Range).is_ok()
                || self.peek_token(TokenType::RangeWide).is_ok()
            {
                self.bump(); // eat range
                self.tree.allocate(Pattern::Rest, self.get_span_from(start))
            }
            // pointer
            else if self.peek_token(TokenType::Multiply).is_ok()
                || self.peek_token(TokenType::ElementwiseAnd).is_ok()
            {
                self.bump(); // eat pointer
                let mutability = if self.peek_keyword(Keyword::Var).is_ok() {
                    self.bump(); // eat var
                    Mutability::Mutable
                } else {
                    Mutability::Immutable
                };
                let target_id = self.eat_pattern(options).for_node_type(NodeType::Pattern)?;
                self.tree.allocate(
                    Pattern::Reference {
                        mutability,
                        target: target_id,
                    },
                    self.get_span_from(start),
                )
            }
            // literal
            else if self.peek_scalar_literal().is_ok() {
                let scalar_literal_id =
                    self.eat_scalar_literal().for_node_type(NodeType::Pattern)?;
                self.tree.allocate(
                    Pattern::Literal(scalar_literal_id),
                    self.get_span_from(start),
                )
            }
            // tuple (without path prefix, no struct tuples)
            else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.bump(); // eat open parenthesis
                self.eat_newlines_maybe()?;
                let fields = self
                    .eat_pattern_field_list(
                        TokenType::Comma,
                        TokenType::CloseParenthesis,
                        ExpressionParserOptions::default(),
                    )
                    .for_node_type(NodeType::Pattern)?;
                let pattern = Pattern::Tuple { path: None, fields };
                self.eat_token(TokenType::CloseParenthesis)?;
                self.tree.allocate(pattern, self.get_span_from(start))
            }
            // array or slice
            else if self.peek_token(TokenType::OpenBracket).is_ok() {
                self.bump(); // eat open bracket
                self.eat_newlines_maybe()?;
                let fields = self
                    .eat_pattern_field_list(
                        TokenType::Comma,
                        TokenType::CloseBracket,
                        ExpressionParserOptions::default(),
                    )
                    .for_node_type(NodeType::Pattern)?;
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::CloseBracket)?;
                self.tree
                    .allocate(Pattern::Slice { fields }, self.get_span_from(start))
            }
            // path or identifier
            else {
                let path_id = self.eat_path().for_node_type(NodeType::Pattern)?;
                let path = self.session.paths.get(path_id);
                // tuple with path
                if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                    self.bump(); // eat open parenthesis
                    self.eat_newlines_maybe()?;
                    let fields = self
                        .eat_pattern_field_list(
                            TokenType::Comma,
                            TokenType::CloseParenthesis,
                            ExpressionParserOptions::default(),
                        )
                        .for_node_type(NodeType::Pattern)?;
                    let pattern = Pattern::Tuple {
                        path: Some(path_id),
                        fields,
                    };
                    self.eat_token(TokenType::CloseParenthesis)?;
                    self.tree.allocate(pattern, self.get_span_from(start))
                }
                // path
                else if path.segments.len() > 1 {
                    self.tree
                        .allocate(Pattern::Path(path_id), self.get_span_from(start))
                }
                // identifier (without mutability)
                else {
                    self.tree.allocate(
                        Pattern::Binding {
                            name: path.segments[0],
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
            let pattern_id = self.tree.allocate(pattern, self.get_span_from(start));
            Ok(pattern_id)
        }
        // range
        else if self.peek_token(TokenType::Range).is_ok()
            || self.peek_token(TokenType::RangeWide).is_ok()
        {
            self.bump(); // eat range
            let end_id = self.eat_pattern(options).for_node_type(NodeType::Pattern)?;
            let pattern = Pattern::Range {
                start: Some(pattern_id),
                end: Some(end_id),
                is_inclusive: false,
            };
            let pattern_id = self.tree.allocate(pattern, self.get_span_from(start));
            Ok(pattern_id)
        }
        // union
        else if self.peek_token(TokenType::ElementwiseOr).is_ok()
            && !self.options.in_implicit_union
        {
            // eat all union "fields" (just unnamed patterns)
            let mut fields: Vec<NodeId<Pattern>> = vec![pattern_id];
            while self.peek_token(TokenType::ElementwiseOr).is_ok() {
                self.bump(); // eat '|'
                let field_pattern_id = self
                    .with_options(
                        ParserOptions {
                            in_implicit_union: true,
                            ..self.options
                        },
                        |parser| parser.eat_pattern(options),
                    )
                    .for_node_type(NodeType::Pattern)?;
                fields.push(field_pattern_id);
            }
            let pattern = Pattern::Union { fields };
            let pattern_id = self.tree.allocate(pattern, self.get_span_from(start));
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
        options: ExpressionParserOptions,
    ) -> ParseResult<Vec<NodeId<PatternField>>> {
        let mut fields: Vec<NodeId<PatternField>> = Vec::new();
        loop {
            if self.peek_token(terminator).is_ok() {
                break;
            }

            // field
            let field_start = self.mark();
            let pattern_field = {
                // named or named alias
                if self.peek_identifier().is_ok()
                    || self.peek_keyword(Keyword::Var).is_ok()
                    || self.peek_keyword(Keyword::Const).is_ok()
                {
                    // mutability
                    let mutability = if self.peek_keyword(Keyword::Var).is_ok() {
                        self.bump(); // eat var
                        Some(Mutability::Mutable)
                    } else if self.peek_keyword(Keyword::Const).is_ok() {
                        self.bump(); // eat const
                        Some(Mutability::Immutable)
                    } else {
                        None
                    };

                    // name
                    let name = self.eat_identifier()?;

                    // alias or pattern
                    if self.peek_colon().is_ok() {
                        self.bump(); // eat colon
                        // named alias
                        if self.peek_identifier().is_ok() {
                            let alias = self.eat_identifier().for_node_type(NodeType::Pattern)?;
                            PatternField::NamedAlias {
                                name,
                                alias,
                                mutability,
                            }
                        }
                        // named with pattern
                        else {
                            let pattern =
                                self.eat_pattern(options).for_node_type(NodeType::Pattern)?;
                            PatternField::Named {
                                name,
                                pattern: Some(pattern),
                                mutability,
                            }
                        }
                    }
                    // named without pattern
                    else {
                        PatternField::Named {
                            name,
                            pattern: None,
                            mutability,
                        }
                    }
                }
                // positional
                else {
                    let pattern = self.eat_pattern(options).for_node_type(NodeType::Pattern)?;
                    PatternField::Positional { pattern }
                }
            };
            let pattern_field_id = self
                .tree
                .allocate(pattern_field, self.get_span_from(field_start));
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
    use crate::parse::tests::TestParser;
    use crate::{
        IntType, Mutability, Pattern, PatternField, ScalarLiteral, assert_node, assert_path,
        assert_string,
    };

    #[test]
    fn test_parse_pattern_wildcard() {
        // _
        let mut test = TestParser::new("_");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern(Default::default()).unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Wildcard);
    }

    #[test]
    fn test_parse_pattern_rest() {
        // ..
        let mut test = TestParser::new("..");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern(Default::default()).unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Rest);
    }

    #[test]
    fn test_parse_pattern_reference() {
        // &var _
        let mut test = TestParser::new("&var _");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern(Default::default()).unwrap();
        // &
        assert_node!(parser.tree, pattern_id,
            Pattern::Reference { mutability, target } => {
                // var
                assert_eq!(*mutability, Mutability::Mutable);
                // _
                assert_node!(parser.tree, *target, Pattern::Wildcard)
            }
        );

        // &1
        let mut test = TestParser::new("&1");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern(Default::default()).unwrap();
        // &
        assert_node!(parser.tree, pattern_id, Pattern::Reference { mutability, target } => {
            assert_eq!(*mutability, Mutability::Immutable);
            // 1
            assert_node!(parser.tree, *target, Pattern::Literal(literal) => {
                assert_node!(parser.tree, *literal, ScalarLiteral::Integer(1, IntType { width: 32, is_signed: true }))
            });
        })
    }

    #[test]
    fn test_parse_pattern_identifier() {
        let mut test = TestParser::new("x");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern(Default::default()).unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Binding { name } => {
            assert_string!(parser.session, *name, "x");
        });
    }

    #[test]
    fn test_parse_pattern_path() {
        let mut test = TestParser::new("MyEnum.A");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern(Default::default()).unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Path(path) => {
            assert_path!(parser.session, *path, "MyEnum.A");
        });
    }

    #[test]
    fn test_parse_pattern_tuple() {
        let mut test = TestParser::new("(x: 1, 2, var y, const z, ..)");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern(Default::default()).unwrap();

        // (x: 1, 2, var y, const z, ..)
        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
            assert_eq!(fields.len(), 5);

            // x: 1
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), mutability: None } => {
                assert_string!(parser.session, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Literal(literal) => {
                    assert_node!(parser.tree, *literal, ScalarLiteral::Integer(1, IntType { width: 32, is_signed: true }));
                });
            });

            // 2
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Literal(literal) => {
                    assert_node!(parser.tree, *literal, ScalarLiteral::Integer(2, IntType { width: 32, is_signed: true }));
                });
            });

            // var y
            assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, mutability: Some(mutability) } => {
                assert_string!(parser.session, *name, "y");
                assert_eq!(*mutability, Mutability::Mutable);
            });

            // const z
            assert_node!(parser.tree, fields[3], PatternField::Named { name, pattern: None, mutability: Some(mutability) } => {
                assert_string!(parser.session, *name, "z");
                assert_eq!(*mutability, Mutability::Immutable);
            });

            // ..
            assert_node!(parser.tree, fields[4], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Rest);
            });
        });
    }

    #[test]
    fn test_parse_pattern_tuple_with_path() {
        let mut test = TestParser::new("Result.Success(_, ..)");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern(Default::default()).unwrap();

        // Result.Success(_, ..)
        assert_node!(parser.tree, pattern_id, Pattern::Tuple { path, fields } => {
            // Result.Success
            assert!(path.is_some());
            assert_eq!(fields.len(), 2);

            // _
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Wildcard);
            });

            // ..
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Rest);
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
    ..
)",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let pattern_id = parser.eat_pattern(Default::default()).unwrap();

        // (x: 1, 2, ..)
        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
            assert_eq!(fields.len(), 3);

            // x: 1
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), mutability: None } => {
                assert_string!(parser.session, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Literal(literal) => {
                    assert_node!(parser.tree, *literal, ScalarLiteral::Integer(1, IntType { width: 32, is_signed: true }));
                });
            });

            // 2
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Literal(literal) => {
                    assert_node!(parser.tree, *literal, ScalarLiteral::Integer(2, IntType { width: 32, is_signed: true }));
                });
            });

            // ..
            assert_node!(parser.tree, fields[2], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Rest);
            });
        });
    }

    #[test]
    fn test_parse_pattern_union() {
        // 1 | 2 | 3
        let mut test = TestParser::new("1 | 2 | 3");
        let mut parser = test.prepare();
        let pattern_id = parser.eat_pattern(Default::default()).unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Union { fields } => {
            assert_eq!(fields.len(), 3);

            // 1
            assert_node!(parser.tree, fields[0], Pattern::Literal(literal) => {
                assert_node!(parser.tree, *literal, ScalarLiteral::Integer(1, IntType { width: 32, is_signed: true }));
            });

            // 2
            assert_node!(parser.tree, fields[1], Pattern::Literal(literal) => {
                assert_node!(parser.tree, *literal, ScalarLiteral::Integer(2, IntType { width: 32, is_signed: true }));
            });

            // 3
            assert_node!(parser.tree, fields[2], Pattern::Literal(literal) => {
                assert_node!(parser.tree, *literal, ScalarLiteral::Integer(3, IntType { width: 32, is_signed: true }));
            });
        });
    }
}
