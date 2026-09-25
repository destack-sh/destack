use crate::parse::{ExpressionPosition, ExpressionStop, TypePosition, TypeStop};
use crate::{Parser, ParserResult};
use tspp_dir::{BlockContext, Catch, Expression, Keyword, LocalNodeId, NodeType, TokenType};

impl Parser {
    /// Parse one try expression.
    ///
    /// Examples:
    /// ```tspp
    /// try { work(); } catch (error) { handle(error); } finally { cleanup(); }
    /// ```
    pub(crate) fn parse_try(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        let keyword_range = self.eat_keyword(Keyword::Try)?.range();

        // try { body } catch ... finally ...
        if self.peek_block() {
            let try_block = self.parse_block(BlockContext::Expression)?;
            let try_block_range = self.tree.get_range(try_block);
            let body = self.insert_node(Expression::Block(try_block), try_block_range);

            // catch ...
            let catch = self.parse_catch()?;

            // finally ...
            let finally = if self.peek_is_keyword(Keyword::Finally) {
                self.bump();
                let finally = self.parse_try_branch()?;
                Some(finally)
            } else {
                None
            };

            // retain the complete branch sequence
            let try_id = self.insert_node(
                Expression::Try {
                    body,
                    catch,
                    finally,
                },
                self.range_since(&start),
            );
            self.tree.set_main_range(try_id, keyword_range);

            Ok(try_id)
        }
        // try expression
        else {
            let expression_id =
                self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?;
            let try_id = self.insert_node(
                Expression::Try {
                    body: expression_id,
                    catch: None,
                    finally: None,
                },
                self.range_since(&start),
            );
            self.tree.set_main_range(try_id, keyword_range);

            Ok(try_id)
        }
    }

    /// Parse one optional catch clause.
    fn parse_catch(&mut self) -> ParserResult<Option<LocalNodeId<Catch>>> {
        if !self.peek_is_keyword(Keyword::Catch) {
            return Ok(None);
        }

        let documentation = self.parse_documentation();
        let start = self.mark_parse_start();
        self.bump();

        // parse an unbound catch or catch-match body
        if self.peek_block() || self.peek_is_keyword(Keyword::Match) {
            let body = self.parse_try_branch()?;
            let catch = self.insert_node(
                Catch {
                    pattern: None,
                    ty: None,
                    body,
                },
                self.range_since(&start),
            );
            self.attach_documentation(catch, documentation);

            return Ok(Some(catch));
        }

        // parse the catch binding and optional type
        self.eat_token(TokenType::OpenParenthesis)?;
        let pattern = self.parse_pattern_before_type()?;
        let ty = if self.eat_token_if(TokenType::Colon) {
            Some(self.parse_type_or_recover_missing(
                TypePosition::Type,
                TypeStop::default(),
                NodeType::Pattern,
            )?)
        } else {
            None
        };

        // close the binding before entering the branch body
        self.eat_close_token_or_recover_missing_with(
            TokenType::CloseParenthesis,
            NodeType::Pattern,
            |parser, token_type| {
                Self::is_close_delimiter_boundary_token(token_type)
                    || parser.peek_block()
                    || parser.peek_is_keyword(Keyword::Match)
            },
        )?;
        let body = self.parse_try_branch()?;
        let catch = self.insert_node(
            Catch {
                pattern: Some(pattern),
                ty,
                body,
            },
            self.range_since(&start),
        );
        self.attach_documentation(catch, documentation);

        Ok(Some(catch))
    }

    /// Parse one catch or finally branch body.
    fn parse_try_branch(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        if self.peek_block() {
            let block = self.parse_block(BlockContext::Expression)?;

            return Ok(self.insert_node(Expression::Block(block), self.tree.get_range(block)));
        }

        Ok(self.parse_statement())
    }
}
