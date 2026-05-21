// parse use and where declarations
use crate::{ParseError, ParseResult, Parser};

use destack_dir::{Keyword, LocalNodeId, NodeType, TokenType, WhereClause};
use destack_source::{NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Eat a where context declaration maybe.
    ///
    /// Examples:
    /// ```
    /// where T: int32
    /// where Foo: Bar
    /// where Foo.Bar: Baz
    /// where BaseOf<Foo>: Copy
    /// ```
    pub fn eat_where_maybe(&mut self) -> ParseResult<Option<Vec<LocalNodeId<WhereClause>>>> {
        // where clauses start at the current token
        if !self.is_keyword(Keyword::Where) {
            return Ok(None);
        }

        Ok(Some(self.eat_where()?))
    }

    /// Eat a where context declaration.
    ///
    /// Examples:
    /// ```
    /// where T: int32
    /// where Foo: Bar
    /// where Foo.Bar: Baz
    /// where BaseOf<Foo>: Copy
    /// where T: Numeric, F: Numeric
    /// where (
    ///    T: Numeric
    ///    F: Numeric // optional comma
    /// )
    /// ```
    pub fn eat_where(&mut self) -> ParseResult<Vec<LocalNodeId<WhereClause>>> {
        self.eat_keyword(Keyword::Where)?;
        let body_flags = self.flags.in_before_block();
        let clauses = self.with_flags(body_flags, |parser| parser.eat_where_body())?;
        Ok(clauses)
    }

    /// Eat the clauses of a `where` declaration (without the `where` keyword).
    /// Separated by commas.
    fn eat_where_body(&mut self) -> ParseResult<Vec<LocalNodeId<WhereClause>>> {
        let mut clauses: Vec<LocalNodeId<WhereClause>> = Vec::new();

        // parenthesized list with newlines
        if self.peek_is(TokenType::OpenParenthesis) {
            self.eat_token(TokenType::OpenParenthesis)?;
            while self.has_more_tokens() {
                if self.peek_is(TokenType::CloseParenthesis) {
                    break;
                }
                let next_clause = self.eat_where_clause()?;
                clauses.push(next_clause);
                // optional comma with newlines
                if self.peek_is(TokenType::Comma) {
                    self.eat_token(TokenType::Comma)?;
                }
            }
            self.eat_close_token_or_recover_missing(
                TokenType::CloseParenthesis,
                NodeType::WhereClause,
            )?;
        }
        // plain list separated by commas
        else {
            while self.has_more_tokens() {
                let clause = self.eat_where_clause()?;
                clauses.push(clause);
                // required comma
                if self.peek_is(TokenType::Comma) {
                    self.eat_token(TokenType::Comma)?;
                } else {
                    break;
                }
            }
        }

        Ok(clauses)
    }

    /// Eat a single where clause.
    fn eat_where_clause(&mut self) -> ParseResult<LocalNodeId<WhereClause>> {
        // span start
        let start = self.span_start();

        // left type
        let left_flags = self
            .flags
            .in_type()
            .in_before_block()
            .in_ternary_condition()
            .disallow_type_conditional();
        let left =
            self.eat_type_expression_or_recover_missing(left_flags, NodeType::WhereClause)?;
        let left_span = self.tree.get_span(left);

        // constraint marker
        let type_start = self.span_start();
        if self.peek_colon_is() {
            self.bump(); // eat colon
        } else if matches!(
            self.current_keyword(),
            Some(Keyword::Extends | Keyword::Implements)
        ) {
            let error = ParseError::expected(self.peek()?.span, TokenType::Colon);
            self.error(&error);
            self.bump(); // eat stale relation separator
        } else {
            self.eat_token(TokenType::Colon)?;
        }

        // constraint type
        let right = self
            .eat_type_expression_or_recover_missing(self.flags.in_type(), NodeType::WhereClause)?;
        let clause = self
            .tree
            .insert(WhereClause { left, right }, self.get_span_from(&start));

        // spans
        self.tree.set_main_span(clause, left_span);
        self.tree.set_side_span(
            clause,
            NodeSpanType::Region(NodeSpanRegion::Type),
            self.get_span_from(&type_start),
        );

        Ok(clause)
    }
}
