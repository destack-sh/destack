use crate::parse::{TypePosition, TypeStop};
use crate::{Parser, ParserError, ParserResult};

use tspp_dir::{
    Keyword, LocalNodeId, NodeType, TokenType, TypeExpression, WhereClause, WhereRelation,
};
use tspp_source::{NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Parse zero or more where clauses.
    pub(crate) fn parse_where_clauses(&mut self) -> ParserResult<Vec<LocalNodeId<WhereClause>>> {
        if !self.peek_is_keyword(Keyword::Where) {
            return Ok(Vec::new());
        }

        // keep property names distinct from constraint clauses
        let peek_next_token = self.peek_token_at(1);
        let is_property = matches!(peek_next_token.ty(), TokenType::Maybe | TokenType::Colon)
            || peek_next_token.is(TokenType::LessThan)
            || peek_next_token.is(TokenType::OpenParenthesis)
                && self.peek_token().range().end == peek_next_token.start();
        if is_property {
            return Ok(Vec::new());
        }

        self.parse_where()
    }

    /// Parse one required where clause sequence.
    ///
    /// Examples:
    /// ```tspp
    /// where T: Serializable, T.Output == U
    /// ```
    pub(crate) fn parse_where(&mut self) -> ParserResult<Vec<LocalNodeId<WhereClause>>> {
        self.eat_keyword(Keyword::Where)?;

        self.parse_where_body()
    }

    /// Parse the clauses after a `where` keyword.
    /// Separated by commas.
    fn parse_where_body(&mut self) -> ParserResult<Vec<LocalNodeId<WhereClause>>> {
        let mut clauses: Vec<LocalNodeId<WhereClause>> = Vec::new();

        // parenthesized list with newlines
        if self.peek_is(TokenType::OpenParenthesis) {
            self.eat_token(TokenType::OpenParenthesis)?;
            while self.has_more_tokens() {
                if self.peek_is(TokenType::CloseParenthesis) {
                    break;
                }
                let next_clause = self.parse_where_clause()?;
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
                let clause = self.parse_where_clause()?;
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

    /// Parse one where clause.
    fn parse_where_clause(&mut self) -> ParserResult<LocalNodeId<WhereClause>> {
        let documentation = self.parse_documentation();

        // retain one repeated Pattern placeholder as a complete clause
        if self.peek_repeated_pattern_marker() {
            let range = self.peek_token().range();
            let left = self.insert_node(TypeExpression::Error, range);
            let right = self.insert_node(TypeExpression::Error, range);
            let clause = self.insert_node(
                WhereClause {
                    relation: WhereRelation::Satisfies,
                    left,
                    right,
                },
                range,
            );
            self.bump();
            self.attach_documentation(clause, documentation);

            return Ok(clause);
        }

        let start = self.mark_parse_start();

        // parse left operand
        let left = self.parse_type_or_recover_missing(
            TypePosition::Type,
            TypeStop::RELATION,
            NodeType::WhereClause,
        )?;
        let left_range = self.tree.get_range(left);

        // start relation span
        let type_start = self.mark_parse_start();

        // accept constraint relation
        let relation = if self.peek_is(TokenType::Colon) {
            self.bump();
            WhereRelation::Satisfies
        }
        // accept equality relation
        else if self.peek_is(TokenType::Equal) {
            self.bump();
            WhereRelation::Equal
        }
        // recover constraint keywords
        else if matches!(
            self.peek_keyword(),
            Some(Keyword::Extends | Keyword::Implements)
        ) {
            let error = ParserError::expected(self.peek_token_span(), TokenType::Colon);
            self.report_error(error);
            self.bump();
            WhereRelation::Satisfies
        }
        // require canonical relation syntax
        else {
            self.eat_token(TokenType::Colon)?;
            WhereRelation::Satisfies
        };

        // parse right operand
        let right = self.parse_type_or_recover_missing(
            TypePosition::Type,
            TypeStop::default(),
            NodeType::WhereClause,
        )?;
        let clause = self.insert_node(
            WhereClause {
                relation,
                left,
                right,
            },
            self.range_since(&start),
        );

        // record spans
        self.tree.set_main_range(clause, left_range);
        self.tree.set_side_range(
            clause,
            NodeSpanType::Region(NodeSpanRegion::Type),
            self.range_since(&type_start),
        );
        self.attach_documentation(clause, documentation);

        Ok(clause)
    }
}
