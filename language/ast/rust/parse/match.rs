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
    /// }
    ///
    /// catch <expr> {
    ///     NetworkError => @panic("network error")
    ///     FormatError => @panic("format error")
    ///     _ => return false
    /// }
    /// ```
    pub fn eat_match(&mut self) -> ParseResult<NodeId<Match>> {
        let start = self.mark();
        self.eat_keyword(Keyword::Match)?;
        let value_id = self.eat_expression()?;
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
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            let case = self.eat_match_case()?;
            cases.push(case);
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
        let pattern_id = self.eat_pattern()?;
        // guard
        let guard = if self.peek_keyword(Keyword::If).is_ok() {
            self.eat_keyword(Keyword::If)?;
            let guard = self.eat_expression()?;
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
            let expression_id = self.eat_expression()?;
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
            let block_id = self.eat_block()?;

            // try block with catch
            if self.peek_keyword(Keyword::Catch).is_ok() {
                self.eat_keyword(Keyword::Catch)?;
                let catch_expression_id = self.eat_expression()?;
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
            let expression_id = self.eat_expression()?;
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
