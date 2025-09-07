//! Parse patterns.

use dyst_language_token::TokenType;

use crate::{Keyword, Mutability, NodeId, ParseError, ParseResult, Parser, Pattern, PatternField};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatternParseMode {
    IgnoreImplicit,
}

impl<'a> Parser<'a> {
    /// Eat a pattern.
    ///
    /// Examples:
    /// ```
    /// _
    /// 1
    /// 2 | 3
    /// 4..6
    /// (x, 0, ..)
    /// Vector2 { x: 0, y, z: zed }
    /// ```
    pub fn eat_pattern(&mut self, mode: Option<PatternParseMode>) -> ParseResult<NodeId<Pattern>> {
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
            else if self.peek_token(TokenType::Range).is_ok() {
                self.bump(); // eat range
                self.tree.allocate(Pattern::Rest, self.get_span_from(start))
            }
            // pointer
            else if self.peek_token(TokenType::Multiply).is_ok() {
                self.bump(); // eat pointer
                let mutability = if self.peek_keyword(Keyword::Var).is_ok() {
                    self.bump(); // eat var
                    Mutability::Mutable
                } else {
                    Mutability::Immutable
                };
                let target_id = self.eat_pattern(None)?;
                self.tree.allocate(
                    Pattern::Pointer {
                        mutability,
                        target: target_id,
                    },
                    self.get_span_from(start),
                )
            }
            // literal
            else if self.peek_scalar_literal().is_ok() {
                let scalar_literal_id = self.eat_scalar_literal()?;
                self.tree.allocate(
                    Pattern::Literal(scalar_literal_id),
                    self.get_span_from(start),
                )
            }
            // identifier
            else if self.peek_identifier().is_ok() {
                let identifier_id = self.eat_identifier()?;
                self.tree.allocate(
                    Pattern::Identifier(identifier_id),
                    self.get_span_from(start),
                )
            }
            // tuple
            else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                self.bump(); // eat open parenthesis
                let fields = self.eat_pattern_field_body(TokenType::Comma, None)?;
                let pattern = Pattern::Tuple { fields };
                self.eat_token(TokenType::CloseParenthesis)?;
                self.tree.allocate(pattern, self.get_span_from(start))
            }
            // array or slice
            else if self.peek_token(TokenType::OpenBracket).is_ok() {
                self.bump(); // eat open bracket
                let fields = self.eat_pattern_field_body(TokenType::Comma, None)?;
                self.eat_token(TokenType::CloseBracket)?;
                self.tree
                    .allocate(Pattern::Slice { fields }, self.get_span_from(start))
            }
            // error
            else {
                return Err(ParseError::UnexpectedToken(self.peek()?.span));
            }
        };

        // ------------------------------------------------------------
        // Postfix->Infix patterns
        // ------------------------------------------------------------

        let ignore_implicit: bool = match mode {
            Some(PatternParseMode::IgnoreImplicit) => true,
            None => false,
        };
        // range
        if self.peek_token(TokenType::Range).is_ok() {
            self.bump(); // eat range
            let end_id = self.eat_pattern(Some(PatternParseMode::IgnoreImplicit))?;
            let pattern = Pattern::Range {
                start: Some(pattern_id),
                end: Some(end_id),
                is_inclusive: false,
            };
            let pattern_id = self.tree.allocate(pattern, self.get_span_from(start));
            Ok(pattern_id)
        }
        // union
        else if self.peek_token(TokenType::BitwiseOr).is_ok() && !ignore_implicit {
            // eat all union "fields" (just unnamed patterns)
            let mut fields: Vec<NodeId<Pattern>> = vec![pattern_id];
            while self.peek_token(TokenType::BitwiseOr).is_ok() {
                self.bump(); // eat '|'
                let field_pattern_id = self.eat_pattern(Some(PatternParseMode::IgnoreImplicit))?;
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
    fn eat_pattern_field_body(
        &mut self,
        seperator: TokenType,
        mode: Option<PatternParseMode>,
    ) -> ParseResult<Vec<NodeId<PatternField>>> {
        let mut fields: Vec<NodeId<PatternField>> = Vec::new();
        loop {
            // field
            let field_start = self.mark();
            let pattern_field = {
                // named or named alias
                if self.peek_identifier().is_ok() {
                    let name = self.eat_identifier()?;
                    if self.peek_colon().is_ok() {
                        self.eat_colon()?;
                        // named alias
                        if self.peek_identifier().is_ok() {
                            let alias = self.eat_identifier()?;
                            PatternField::NamedAlias { name, alias }
                        }
                        // named with pattern
                        else {
                            let pattern = self.eat_pattern(mode)?;
                            PatternField::Named {
                                name,
                                pattern: Some(pattern),
                            }
                        }
                    }
                    // named without pattern
                    else {
                        PatternField::Named {
                            name,
                            pattern: None,
                        }
                    }
                }
                // positional
                else {
                    let pattern = self.eat_pattern(mode)?;
                    PatternField::Positional { pattern }
                }
            };
            let pattern_field_id = self
                .tree
                .allocate(pattern_field, self.get_span_from(field_start));
            fields.push(pattern_field_id);

            // separator
            if self.peek_token(seperator).is_ok() {
                self.eat_token(seperator)?;
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
    use crate::{IntType, Mutability, Pattern, PatternField, ScalarLiteral, assert_node};

    #[test]
    fn test_parse_pattern_wildcard() {
        // _
        let test = TestParser::new("_");
        let mut parser = test.parser();
        let pattern_id = parser.eat_pattern(None).unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Wildcard);
    }

    #[test]
    fn test_parse_pattern_rest() {
        // ..
        let test = TestParser::new("..");
        let mut parser = test.parser();
        let pattern_id = parser.eat_pattern(None).unwrap();
        assert_node!(parser.tree, pattern_id, Pattern::Rest);
    }

    #[test]
    fn test_parse_pattern_pointer() {
        // *var _
        let test = TestParser::new("*var _");
        let mut parser = test.parser();
        let pattern_id = parser.eat_pattern(None).unwrap();
        // *
        assert_node!(parser.tree, pattern_id,
            Pattern::Pointer { mutability, target } => {
                // var
                assert_eq!(*mutability, Mutability::Mutable);
                // _
                assert_node!(parser.tree, *target, Pattern::Wildcard)
            }
        );

        // *1
        let test = TestParser::new("*1");
        let mut parser = test.parser();
        let pattern_id = parser.eat_pattern(None).unwrap();
        // *
        assert_node!(parser.tree, pattern_id, Pattern::Pointer { mutability, target } => {
            assert_eq!(*mutability, Mutability::Immutable);
            // 1
            assert_node!(parser.tree, *target, Pattern::Literal(literal) => {
                assert_node!(parser.tree, *literal, ScalarLiteral::Integer(1, IntType { width: 32, is_signed: true }))
            });
        })
    }

    #[test]
    fn test_parse_pattern_tuple() {
        // (x: 1, 2, ..)
        let test = TestParser::new("(x: 1, 2, ..)");
        let mut parser = test.parser();
        let pattern_id = parser.eat_pattern(None).unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields } => {
            assert_eq!(fields.len(), 3);

            // x: 1
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern) } => {
                assert_eq!(*name, parser.strings.intern("x"));
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
        let test = TestParser::new("1 | 2 | 3");
        let mut parser = test.parser();
        let pattern_id = parser.eat_pattern(None).unwrap();

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
