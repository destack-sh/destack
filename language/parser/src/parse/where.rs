// parse use and where declarations
use crate::parse::timing::tags;
use crate::{ParseResult, Parser};

use destack_ast::{Keyword, LocalNodeId, NodeType, TokenType, WhereClause};
use destack_source::NodeSpanType;

impl Parser {
    /// Eat a where context declaration maybe.
    ///
    /// Examples:
    /// ```
    /// where T: int32
    /// where Foo: Bar
    /// where Foo, Bar
    /// where Foo.Bar: Baz
    /// ```
    pub fn eat_where_maybe(&mut self) -> ParseResult<Option<Vec<LocalNodeId<WhereClause>>>> {
        // look ahead to where without speculative rewinds
        let start_index = self.pos_index();
        let where_index = self.next_non_newline_index_from(start_index);
        if self.token_type_at(where_index) != TokenType::Identifier
            || self.keyword_for_index(where_index) != Some(Keyword::Where)
        {
            return Ok(None);
        }

        if where_index != start_index {
            self.eat_newlines_maybe()?;
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
    /// where T: Numeric, F: Numeric
    /// where (
    ///    T: Numeric
    ///    F: Numeric // optional comma
    /// )
    /// ```
    pub fn eat_where(&mut self) -> ParseResult<Vec<LocalNodeId<WhereClause>>> {
        let _timing = self.timing_scope(tags::PARSE_WHERE);
        self.eat_keyword(Keyword::Where)?;
        let body_options = self.options.in_before_block();
        let clauses = self.with_options(body_options, |parser| parser.eat_where_body())?;
        Ok(clauses)
    }

    /// Eat the clauses of a `where` declaration (without the `where` keyword).
    /// Separated by commas.
    fn eat_where_body(&mut self) -> ParseResult<Vec<LocalNodeId<WhereClause>>> {
        let mut clauses: Vec<LocalNodeId<WhereClause>> = Vec::new();

        // parenthesized list with newlines
        if self.peek_is(TokenType::OpenParenthesis) {
            self.eat_token(TokenType::OpenParenthesis)?;
            self.eat_newlines_maybe()?;
            while self.has_more_tokens() {
                self.eat_newlines_maybe()?;
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
        let start = self.mark_span();

        // left name
        let (left, left_span) = self.eat_identifier_with_span()?;

        // constraint type
        let type_start = self.mark_span();
        self.eat_token(TokenType::Colon)?;
        self.eat_newlines_maybe()?;
        let right = self.eat_type_expression_node_or_recover_missing(
            self.options.in_type(),
            NodeType::WhereClause,
        )?;
        let clause = self
            .tree
            .insert(WhereClause { left, right }, self.get_span_from(&start));

        // spans
        self.tree.set_main_span(clause, left_span);
        self.tree
            .set_side_span(clause, NodeSpanType::Type, self.get_span_from(&type_start));

        Ok(clause)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{CommentKind, IntType, NodeType, TypeExpression, TypeLiteral, WhereClause};
    use destack_source::NodeSpanType;

    use crate::{TestParser, assert_comment, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_where_type_assertion() {
        let mut test = TestParser::new("where T: int32");
        let mut parser = test.prepare();
        let clauses = parser.eat_where().unwrap();

        // where T: int32
        assert_eq!(clauses.len(), 1);
        assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
            assert_string!(parser, *left, "T");
            assert_node!(parser.tree, *right, TypeExpression::Literal { value } => {
                assert_eq!(
                    *value,
                    TypeLiteral::Int(IntType::Arbitrary {
                        width: Some(32),
                        is_signed: true,
                    })
                );
            });
        });
    }

    #[test]
    fn test_parse_where_multiple_clauses() {
        let input = "where T: Numeric, U: Copy, V: Comparable";
        let mut test = TestParser::new(input);
        let mut parser = test.prepare();
        let clauses = parser.eat_where().unwrap();

        assert_eq!(clauses.len(), 3);

        assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
            assert_string!(parser, *left, "T");
            assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Numeric");
            });
        });
        assert_node!(parser.tree, clauses[1], WhereClause { left, right } => {
            assert_string!(parser, *left, "U");
            assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Copy");
            });
        });
        assert_node!(parser.tree, clauses[2], WhereClause { left, right } => {
            assert_string!(parser, *left, "V");
            assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Comparable");
            });
        });
    }

    #[test]
    fn test_parse_where_parenthesized_multiline() {
        let input = r##"where (
  T: Numeric
  U: Copy,
  V: Comparable
)"##;
        let mut test = TestParser::new(input);
        let mut parser = test.prepare();
        let clauses = parser.eat_where().unwrap();

        assert_eq!(clauses.len(), 3);

        assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
            assert_string!(parser, *left, "T");
            assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Numeric");
            });
        });
        assert_node!(parser.tree, clauses[1], WhereClause { left, right } => {
            assert_string!(parser, *left, "U");
            assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Copy");
            });
        });
        assert_node!(parser.tree, clauses[2], WhereClause { left, right } => {
            assert_string!(parser, *left, "V");
            assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Comparable");
            });
        });
    }

    #[test]
    fn test_parse_parenthesized_where_with_missing_close_parenthesis() {
        let mut test = TestParser::new("where (T: Numeric, U: Copy");
        let mut parser = test.prepare();
        let clauses = parser.eat_where().unwrap();

        test.assert_error_leaves(&parser, &[(Some(NodeType::WhereClause), None, "")]);
        assert_eq!(clauses.len(), 2);

        // T: Numeric
        assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
            assert_string!(parser, *left, "T");
            assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Numeric");
            });
        });

        // U: Copy
        assert_node!(parser.tree, clauses[1], WhereClause { left, right } => {
            assert_string!(parser, *left, "U");
            assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Copy");
            });
        });
    }

    /// Ensure where clauses record main and type spans.
    #[test]
    fn test_where_clause_spans() {
        let mut test = TestParser::new("where T: Numeric");
        let mut parser = test.prepare();
        let clauses = parser.eat_where().unwrap();
        let clause_id = clauses[0];

        // main span
        let main_span = parser
            .tree
            .get_main_span(clause_id)
            .expect("expected main span");
        assert_eq!(parser.get_span_str(main_span), "T");

        // type span
        let type_span = parser
            .tree
            .get_side_span(clause_id, NodeSpanType::Type)
            .expect("expected type span");
        assert_eq!(parser.get_span_str(type_span), ": Numeric");
    }

    #[test]
    fn test_where_clause_constraint_with_boundary_comment() {
        let source = "where T: // bound-note\nNumeric";
        let mut test = TestParser::new(source);
        let mut parser = test.prepare();
        let clauses = parser.eat_where().unwrap();
        parser.attach_comments();

        assert_eq!(clauses.len(), 1);

        // T: Numeric
        assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
            assert_string!(parser, *left, "T");
            assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Numeric");
            });
        });

        // : // bound-note\nNumeric
        let type_span = parser
            .tree
            .get_side_span(clauses[0], NodeSpanType::Type)
            .expect("expected where type span");
        assert_eq!(parser.get_span_str(type_span), ": // bound-note\nNumeric");

        // // bound-note
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "bound-note");
    }
}
