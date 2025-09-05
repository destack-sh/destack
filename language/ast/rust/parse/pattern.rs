//! Parse patterns.

use destack_language_token::TokenType;

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
                // eat parens, foward implicit tuple
                self.eat_token(TokenType::OpenParenthesis)?;
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
                        // named
                        else {
                            let pattern = self.eat_pattern(mode)?;
                            PatternField::Named {
                                name,
                                pattern: Some(pattern),
                            }
                        }
                    } else {
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
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Mutability, Parser, Pattern, PatternField, ScalarLiteral};

    #[test]
    fn test_parse_pattern_wildcard() {
        let input = "_";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let pattern_id = parser.eat_pattern(None).unwrap();
        assert_eq!(parser.tree.get(pattern_id), &Pattern::Wildcard);
    }

    #[test]
    fn test_parse_pattern_rest() {
        let input = "..";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let pattern_id = parser.eat_pattern(None).unwrap();
        assert_eq!(parser.tree.get(pattern_id), &Pattern::Rest);
    }

    #[test]
    fn test_parse_pattern_pointer() {
        // *var _
        let input = "*var _";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let pattern_id = parser.eat_pattern(None).unwrap();
        match parser.tree.get(pattern_id) {
            Pattern::Pointer { mutability, target } => {
                assert_eq!(*mutability, Mutability::Mutable);
                assert_eq!(parser.tree.get(*target), &Pattern::Wildcard);
            }
            other => panic!("expected pointer, got {other:?}"),
        }

        // *1
        let input = "*1";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let pattern_id = parser.eat_pattern(None).unwrap();
        match parser.tree.get(pattern_id) {
            Pattern::Pointer { mutability, target } => {
                assert_eq!(*mutability, Mutability::Immutable);
                match parser.tree.get(*target) {
                    Pattern::Literal(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Integer(n, _) => assert_eq!(*n, 1),
                        _ => panic!("expected integer literal"),
                    },
                    _ => panic!("expected literal pattern"),
                }
            }
            other => panic!("expected pointer, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_pattern_tuple() {
        let input = "(x: 1, 2, ..)";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let pattern_id = parser.eat_pattern(None).unwrap();
        match parser.tree.get(pattern_id) {
            Pattern::Tuple { fields } => {
                assert_eq!(fields.len(), 3);

                // x: 1
                match parser.tree.get(fields[0]) {
                    PatternField::Named {
                        name,
                        pattern: Some(pattern),
                    } => {
                        assert_eq!(*name, parser.strings.intern("x"));
                        match parser.tree.get(*pattern) {
                            Pattern::Literal(lit_id) => match parser.tree.get(*lit_id) {
                                ScalarLiteral::Integer(n, _) => assert_eq!(*n, 1),
                                _ => panic!("expected integer literal"),
                            },
                            _ => panic!("expected literal pattern"),
                        }
                    }
                    other => panic!("expected named field, got {other:?}"),
                }

                // 2
                match parser.tree.get(fields[1]) {
                    PatternField::Positional { pattern } => match parser.tree.get(*pattern) {
                        Pattern::Literal(lit_id) => match parser.tree.get(*lit_id) {
                            ScalarLiteral::Integer(n, _) => assert_eq!(*n, 2),
                            _ => panic!("expected integer literal"),
                        },
                        _ => panic!("expected literal pattern"),
                    },
                    other => panic!("expected positional field, got {other:?}"),
                }

                // ..
                match parser.tree.get(fields[2]) {
                    PatternField::Positional { pattern } => {
                        assert_eq!(parser.tree.get(*pattern), &Pattern::Rest)
                    }
                    other => panic!("expected positional field, got {other:?}"),
                }
            }
            other => panic!("expected tuple pattern, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_pattern_union() {
        let input = "1 | 2 | 3";
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        let pattern_id = parser.eat_pattern(None).unwrap();
        match parser.tree.get(pattern_id) {
            Pattern::Union { fields } => {
                assert_eq!(fields.len(), 3);
                let mut expect = 1;
                for field_id in fields.iter() {
                    match parser.tree.get(*field_id) {
                        Pattern::Literal(lit_id) => match parser.tree.get(*lit_id) {
                            ScalarLiteral::Integer(n, _) => {
                                assert_eq!(*n, expect);
                                expect += 1;
                            }
                            _ => panic!("expected integer literal"),
                        },
                        other => panic!("expected literal pattern, got {other:?}"),
                    }
                }
            }
            other => panic!("expected union pattern, got {other:?}"),
        }
    }
}
