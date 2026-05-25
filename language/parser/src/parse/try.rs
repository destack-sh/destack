use crate::{Parser, ParserResult};
use destack_dir::{BlockContext, Catch, Expression, Keyword, LocalNodeId, NodeType, TokenType};

impl Parser {
    /// Eat a try expression.
    ///
    /// Examples:
    /// ```
    /// try {
    ///     fileOperation()?;
    /// } catch (e) {
    ///     handle(e);
    /// }
    ///
    /// try {
    ///     let a = riskyOperationA()?;
    ///     riskyOperationB(a)?;
    /// } catch (e) {
    ///     log("failed", e);
    /// } finally {
    ///     cleanup();
    /// }
    ///
    /// try {
    ///     riskyOperationA()?;
    /// } catch match (e) {
    ///     NumericError(x) => Error(`bad number: ${x}`)
    ///     FormatError => Error(`bad format ${e}`)
    ///     _ => Error(`unknown error: ${e}`)
    /// }
    /// ```
    ///
    /// The parser accepts `try <expr>` without catch/finally, but Analyze rejects it.
    pub fn eat_try(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();
        self.eat_keyword(Keyword::Try)?;

        // try block
        if self.is_block_start() {
            // try block
            let try_block = self.eat_block(BlockContext::Expression)?;
            let try_block_span = self.tree.get_span(try_block);
            let body = self.insert_node(Expression::Block(try_block), try_block_span);

            // catch
            let catch = if self.is_keyword(Keyword::Catch) {
                let catch_start = self.span_start();
                self.bump(); // eat keyword

                // no pattern or catch match
                let (pattern, ty, body) =
                    if self.is_block_start() || self.is_keyword(Keyword::Match) {
                        let body = self.eat_try_branch_body()?;
                        (None, None, body)
                    }
                    // catch pattern with expression content
                    else {
                        // parse catch binding pattern
                        self.eat_token(TokenType::OpenParenthesis)?;

                        let catch_pattern_flags = self
                            .flags
                            .not_in_position()
                            .in_before_type()
                            .in_before_block();
                        let catch_pattern =
                            self.with_flags(catch_pattern_flags, |parser| parser.eat_pattern())?;

                        let catch_ty = if self.peek_colon_is() {
                            self.bump(); // eat :
                            let catch_ty = self.eat_type_expression_or_recover_missing(
                                self.flags.not_in_position().in_type().in_before_block(),
                                NodeType::Pattern,
                            )?;
                            Some(catch_ty)
                        } else {
                            None
                        };

                        self.eat_close_token_or_recover_missing_with(
                            TokenType::CloseParenthesis,
                            NodeType::Pattern,
                            |parser, token_type| {
                                Self::is_close_delimiter_boundary_token(token_type)
                                    || parser.is_block_start()
                                    || parser.is_keyword(Keyword::Match)
                            },
                        )?;

                        let body = self.eat_try_branch_body()?;
                        (Some(catch_pattern), catch_ty, body)
                    };

                Some(self.insert_node(
                    Catch { pattern, ty, body },
                    self.get_span_from(&catch_start),
                ))
            } else {
                None
            };

            // finally
            let finally = if self.is_keyword(Keyword::Finally) {
                self.bump(); // eat keyword
                let finally = self.eat_try_branch_body()?;
                Some(finally)
            } else {
                None
            };

            // try
            let try_id = self.insert_node(
                Expression::Try {
                    body,
                    catch,
                    finally,
                },
                self.get_span_from(&start),
            );
            Ok(try_id)
        }
        // try expression
        else {
            let expression_flags = self.flags.not_in_position();
            let expression_id = self.with_flags(expression_flags, |parser| {
                parser.eat_expression(parser.flags)
            })?;
            let try_id = self.insert_node(
                Expression::Try {
                    body: expression_id,
                    catch: None,
                    finally: None,
                },
                self.get_span_from(&start),
            );
            Ok(try_id)
        }
    }

    /// Eat a catch or finally branch body.
    fn eat_try_branch_body(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        if self.is_block_start() {
            let block = self.eat_block(BlockContext::Expression)?;
            let span = self.tree.get_span(block);

            Ok(self.insert_node(Expression::Block(block), span))
        } else {
            self.with_flags(self.flags.not_in_position(), |parser| {
                parser.eat_statement_expression()
            })
        }
    }
}
