use destack_language_token::TokenType;

use crate::{Keyword, Match, MatchCase, NodeId, ParseResult, Parser, Try};

impl<'a> Parser<'a> {
    /// Eat a match statement.
    ///
    /// Examples:
    /// ```
    /// match <expr> {
    ///     (x, y, ..) => {
    ///         ...
    ///     }
    ///     (x, y, z) => {
    ///         ...
    ///     }
    ///     _ = ohNoes()
    /// }
    /// ```
    pub fn eat_match(&mut self) -> ParseResult<NodeId<Match>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Match)?;
        let value_id = self.eat_expression(None)?;
        let cases_id = self.eat_match_body()?;
        let match_id = self.tree.allocate(
            Match {
                value: value_id,
                cases: cases_id,
            },
            self.get_span_from(start),
        );
        Ok(match_id)
    }

    /// Eat multiple match cases separated as statements.
    ///
    /// Examples:
    /// ```
    /// 2 => parse_int(2)
    /// (x, y) => {
    ///     ...
    /// }
    /// ```
    pub fn eat_match_body(&mut self) -> ParseResult<Vec<NodeId<MatchCase>>> {
        let mut cases: Vec<NodeId<MatchCase>> = Vec::new();
        self.eat_token(TokenType::OpenBrace)?;
        loop {
            // stop on closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            // allow statement separators between cases (newline/semicolon)
            else if self.peek_statement_stop().is_ok() {
                self.eat_statement_stop()?;
            }
            // case
            else {
                let case = self.eat_match_case()?;
                cases.push(case);
            }
        }
        self.eat_token(TokenType::CloseBrace)?;
        Ok(cases)
    }

    /// Eat a match case.
    ///
    /// Examples:
    /// ```
    /// 2 => parse_int(2)
    ///
    /// (x, y) if x > y => {
    ///     ...
    /// }
    /// ```
    pub fn eat_match_case(&mut self) -> ParseResult<NodeId<MatchCase>> {
        let start = self.mark();
        // pattern
        let pattern_id = self.eat_pattern(None)?;
        // guard
        let guard = if self.peek_keyword(Keyword::If).is_ok() {
            self.eat_keyword(Keyword::If)?;
            let guard = self.eat_expression(None)?;
            Some(guard)
        } else {
            None
        };
        // arrow
        self.eat_token(TokenType::Arrow)?;
        // body
        if self.peek_block().is_ok() {
            let block_id = self.eat_block()?;
            let match_case_id = self.tree.allocate(
                MatchCase::Block {
                    pattern: pattern_id,
                    body: block_id,
                    guard,
                },
                self.get_span_from(start),
            );
            Ok(match_case_id)
        }
        // statement
        else {
            let expression_id = self.eat_expression(None)?;
            let match_case_id = self.tree.allocate(
                MatchCase::Expression {
                    pattern: pattern_id,
                    body: expression_id,
                    guard,
                },
                self.get_span_from(start),
            );
            Ok(match_case_id)
        }
    }

    /// Eat a try statement.
    ///
    /// Examples:
    /// ```
    /// try fileOperation() // implicitly unwraps the Result, returns Error case
    ///
    /// try { // implicitly unwraps all Results inside
    ///     let a = riskyOperationA() // a is Result.Ok(_) from riskyOperationA
    ///     riskyOperationB(a)
    /// } // no catch needed if containing function has compatible Result type (Into suffices)
    ///
    /// try { // explicitly unwraps all Results inside
    ///     ...
    /// } catch e { // match all errors
    ///     NumericError(x) => Error(@format("bad number: {x}"))
    ///     FormatError => Error(@format("bad format {e}"))
    ///     // it's exhaustive! otherwise `_ =>` like in match (it is a match)
    /// }
    /// ```
    pub fn eat_try_catch(&mut self) -> ParseResult<NodeId<Try>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Try)?;

        // try block
        if self.peek_block().is_ok() {
            // try block
            let block_id = self.eat_block()?;

            // try block with catch
            if self.peek_keyword(Keyword::Catch).is_ok() {
                // catch match
                self.eat_keyword(Keyword::Catch)?;
                let catch_expression_id = self.eat_expression(None)?;
                let catch_match_cases_id = self.eat_match_body()?;
                let catch_match_id = self.tree.allocate(
                    Match {
                        value: catch_expression_id,
                        cases: catch_match_cases_id,
                    },
                    self.get_span_from(start),
                );
                let try_id = self.tree.allocate(
                    Try::BlockWithCatch {
                        try_block: block_id,
                        catch_match: catch_match_id,
                    },
                    self.get_span_from(start),
                );
                Ok(try_id)
            }
            // try block without catch
            else {
                let try_id = self.tree.allocate(
                    Try::Block {
                        try_block: block_id,
                    },
                    self.get_span_from(start),
                );
                Ok(try_id)
            }
        }
        // try expression
        else {
            let expression_id = self.eat_expression(None)?;
            let try_id = self.tree.allocate(
                Try::Expression {
                    try_expression: expression_id,
                },
                self.get_span_from(start),
            );
            Ok(try_id)
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_language_token::{SourceFile, tokenize_semantic};

    use crate::{Expression, MatchCase, Parser, Pattern, ScalarLiteral, Try};

    #[test]
    fn test_match_with_simple_guard_and_wildcard_arms() {
        let input = r###"
match x {
    1 => 10
    2 if true => 20
    _ => 0
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let match_id = parser.eat_match().unwrap();
        let match_ = parser.tree.get(match_id);
        assert_eq!(match_.cases.len(), 3);

        // value: path x
        match parser.tree.get(match_.value) {
            Expression::Path { path } => {
                let p = parser.paths.get(*path);
                assert_eq!(p.segments.len(), 1);
                assert_eq!(p.segments[0], parser.strings.intern("x"));
            }
            other => panic!("expected path x, got {other:?}"),
        }

        // case 0: 1 => 10
        match parser.tree.get(match_.cases[0]) {
            MatchCase::Expression {
                pattern,
                body,
                guard,
            } => {
                assert!(guard.is_none());
                match parser.tree.get(*pattern) {
                    Pattern::Literal(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Integer(n, _) => assert_eq!(*n, 1),
                        other => panic!("expected int 1, got {other:?}"),
                    },
                    other => panic!("expected literal pattern, got {other:?}"),
                }
                match parser.tree.get(*body) {
                    Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Integer(n, _) => assert_eq!(*n, 10),
                        other => panic!("expected int 10, got {other:?}"),
                    },
                    other => panic!("expected literal body, got {other:?}"),
                }
            }
            other => panic!("expected expression case, got {other:?}"),
        }

        // case 1: 2 if true => 20
        match parser.tree.get(match_.cases[1]) {
            MatchCase::Expression {
                pattern,
                body,
                guard,
            } => {
                // if true
                let guard_id = guard.expect("expected guard");
                match parser.tree.get(guard_id) {
                    Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Boolean(b) => assert!(*b),
                        other => panic!("expected boolean true, got {other:?}"),
                    },
                    other => panic!("expected scalar literal guard, got {other:?}"),
                }
                // pattern: 2
                match parser.tree.get(*pattern) {
                    Pattern::Literal(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Integer(n, _) => assert_eq!(*n, 2),
                        other => panic!("expected int 2, got {other:?}"),
                    },
                    other => panic!("expected literal pattern, got {other:?}"),
                }
                // body: 20
                match parser.tree.get(*body) {
                    Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Integer(n, _) => assert_eq!(*n, 20),
                        other => panic!("expected int 20, got {other:?}"),
                    },
                    other => panic!("expected literal body, got {other:?}"),
                }
            }
            other => panic!("expected expression case, got {other:?}"),
        }

        // case 2: _ => 0
        match parser.tree.get(match_.cases[2]) {
            MatchCase::Expression {
                pattern,
                body,
                guard,
            } => {
                assert!(guard.is_none());
                assert_eq!(parser.tree.get(*pattern), &Pattern::Wildcard);
                match parser.tree.get(*body) {
                    Expression::ScalarLiteral(lit_id) => match parser.tree.get(*lit_id) {
                        ScalarLiteral::Integer(n, _) => assert_eq!(*n, 0),
                        other => panic!("expected int 0, got {other:?}"),
                    },
                    other => panic!("expected literal body, got {other:?}"),
                }
            }
            other => panic!("expected expression case, got {other:?}"),
        }
    }

    #[test]
    fn test_try_expression() {
        let input = r###"
try foo()
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let try_id = parser.eat_try_catch().unwrap();
        match parser.tree.get(try_id) {
            Try::Expression { try_expression } => match parser.tree.get(*try_expression) {
                Expression::Call(call_id) => {
                    let call = parser.tree.get(*call_id);
                    // receiver is path foo
                    match parser.tree.get(call.receiver) {
                        Expression::Path { path } => {
                            let p = parser.paths.get(*path);
                            assert_eq!(p.segments.len(), 1);
                            assert_eq!(p.segments[0], parser.strings.intern("foo"));
                        }
                        other => panic!("expected path foo, got {other:?}"),
                    }
                }
                other => panic!("expected call expression, got {other:?}"),
            },
            other => panic!("expected try expression, got {other:?}"),
        }
    }

    #[test]
    fn test_try_block_with_catch_block_match() {
        let input = r###"
try {
} catch e {
    _ => {
    }
}
"###;
        let tokens = tokenize_semantic(input);
        let mut parser = Parser::new(SourceFile::new(0, input, input.len() as u32), &tokens);
        parser.eat_newline().unwrap();

        let try_id = parser.eat_try_catch().unwrap();
        match parser.tree.get(try_id) {
            Try::BlockWithCatch {
                try_block,
                catch_match,
            } => {
                // try block exists
                let _block = parser.tree.get(*try_block);

                // catch match: value is path e, one case with wildcard and block body
                let m = parser.tree.get(*catch_match);
                match parser.tree.get(m.value) {
                    Expression::Path { path } => {
                        let p = parser.paths.get(*path);
                        assert_eq!(p.segments.len(), 1);
                        assert_eq!(p.segments[0], parser.strings.intern("e"));
                    }
                    other => panic!("expected path e, got {other:?}"),
                }
                assert_eq!(m.cases.len(), 1);
                match parser.tree.get(m.cases[0]) {
                    MatchCase::Block {
                        pattern,
                        body: _,
                        guard,
                    } => {
                        assert!(guard.is_none());
                        assert_eq!(parser.tree.get(*pattern), &Pattern::Wildcard);
                    }
                    other => panic!("expected block case, got {other:?}"),
                }
            }
            other => panic!("expected try block with catch, got {other:?}"),
        }
    }
}
