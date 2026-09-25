use crate::parse::error::ParserResultExt;
use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::{Parser, ParserError, ParserResult};
use tspp_dir::{
    Block, BlockContext, BlockForm, Expression, Keyword, LocalNodeId, NodeType, Token, TokenType,
};
use tspp_source::{NodeSpanRegion, NodeSpanType};

/// The enclosing block rules used to classify one item.
#[derive(Debug, Copy, Clone)]
struct BlockFrame {
    /// The enclosing block source form.
    form: BlockForm,
    /// The enclosing block evaluation context.
    block_context: BlockContext,
}

/// The expressions collected from one block body.
struct BlockBody {
    /// The statement expressions before the possible value tail.
    leading_expressions: Vec<LocalNodeId<Expression>>,
    /// The possible value of an expression block.
    tail_expression: Option<LocalNodeId<Expression>>,
}

/// One block item classified by its source termination.
enum BlockItem {
    /// One statement expression.
    Statement(LocalNodeId<Expression>),
    /// One possible expression-block value tail.
    Tail(LocalNodeId<Expression>),
}

impl BlockItem {
    /// Return the expression carried by this block item.
    fn expression(self) -> LocalNodeId<Expression> {
        match self {
            Self::Statement(expression) | Self::Tail(expression) => expression,
        }
    }
}

impl Parser {
    /// Return whether the current colon can continue one consumed label.
    pub(crate) fn peek_label_body(&self, stop: ExpressionStop) -> bool {
        if stop.has(ExpressionStop::SWITCH_COLON)
            || stop.has(ExpressionStop::CONDITIONAL_COLON)
            || !self.peek_is(TokenType::Colon)
        {
            return false;
        }

        let target = self.peek_next_token();

        self.is_valid_label_target(target)
    }

    /// Return whether one token can begin a label body.
    fn is_valid_label_target(&self, target: Token) -> bool {
        matches!(
            self.token_keyword(target),
            Some(Keyword::While | Keyword::Do | Keyword::For | Keyword::Loop)
        )
    }

    /// Parse a label body after its identifier has been consumed.
    pub(crate) fn parse_label_body(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        self.eat_token(TokenType::Colon)?;

        // represent an empty labeled statement as an empty implicit block
        if self.peek_is(TokenType::Semicolon) {
            let start = self.mark_parse_start();
            self.bump();
            let block = self.insert_node(
                Block {
                    context: BlockContext::Statement,
                    form: BlockForm::Implicit,
                    leading_expressions: Vec::new(),
                    tail_expression: None,
                },
                self.range_since(&start),
            );

            return Ok(self.insert_node(Expression::Block(block), self.range_since(&start)));
        }

        self.parse_expression(ExpressionPosition::Block, ExpressionStop::default())
    }

    /// Return whether the current tokens start a block.
    #[inline]
    pub(crate) fn peek_block(&self) -> bool {
        self.peek_is(TokenType::OpenBrace)
            || self.peek_is_keyword(Keyword::Do)
                && self.peek_next_token_type() == TokenType::OpenBrace
    }

    /// Parse an explicit block.
    pub(crate) fn parse_block(
        &mut self,
        block_context: BlockContext,
    ) -> ParserResult<LocalNodeId<Block>> {
        let start = self.mark_parse_start();
        let form = if self.peek_is_keyword(Keyword::Do) {
            self.bump();
            BlockForm::Do
        } else {
            BlockForm::Explicit
        };

        // parse the delimited body
        self.eat_token_before(TokenType::OpenBrace, TokenType::CloseBrace)
            .in_node(NodeType::Block)?;
        let body = self.parse_block_items(form, block_context)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Block)?;

        Ok(self.insert_node(
            Block {
                context: block_context,
                form,
                leading_expressions: body.leading_expressions,
                tail_expression: body.tail_expression,
            },
            self.range_since(&start),
        ))
    }

    /// Parse a block or wrap one statement in an implicit block.
    pub(crate) fn parse_block_or_statement(&mut self) -> ParserResult<LocalNodeId<Block>> {
        if self.peek_block() {
            return self.parse_block(BlockContext::Statement);
        }

        let start = self.mark_parse_start();
        let mut leading_expressions = Vec::new();
        if self.peek_is(TokenType::Semicolon) {
            self.bump();
        } else {
            let item = self.parse_block_item(None);
            leading_expressions.push(item.expression());
        }

        Ok(self.insert_node(
            Block {
                context: BlockContext::Statement,
                form: BlockForm::Implicit,
                leading_expressions,
                tail_expression: None,
            },
            self.range_since(&start),
        ))
    }

    /// Parse one block body without its delimiters.
    pub(crate) fn parse_block_body(
        &mut self,
        form: BlockForm,
        block_context: BlockContext,
    ) -> ParserResult<Vec<LocalNodeId<Expression>>> {
        let BlockBody {
            mut leading_expressions,
            tail_expression,
        } = self.parse_block_items(form, block_context)?;
        if let Some(tail_expression) = tail_expression {
            leading_expressions.push(tail_expression);
        }

        Ok(leading_expressions)
    }

    /// Parse block items while preserving one possible value tail.
    fn parse_block_items(
        &mut self,
        form: BlockForm,
        block_context: BlockContext,
    ) -> ParserResult<BlockBody> {
        let mut leading_expressions = Vec::new();
        let mut tail_expression = None;

        loop {
            let token_type = self.peek_token_type();
            if token_type == TokenType::End
                || form != BlockForm::Implicit && token_type == TokenType::CloseBrace
            {
                break;
            }
            if token_type == TokenType::Semicolon {
                self.bump();
                continue;
            }

            // consume an unmatched root delimiter exactly once
            if form == BlockForm::Implicit && token_type == TokenType::CloseBrace {
                let token = self.peek_token_span();
                self.bump();
                let error = ParserError::unexpected(token);
                self.report_error(error);
                leading_expressions.push(self.insert_node(Expression::Error, token.span.range()));

                continue;
            }

            // a later item turns the previous tail candidate into a statement
            if let Some(expression) = tail_expression.take() {
                leading_expressions.push(expression);
            }

            let frame = BlockFrame {
                form,
                block_context,
            };
            match self.parse_block_item(Some(frame)) {
                BlockItem::Statement(expression) => leading_expressions.push(expression),
                BlockItem::Tail(expression) => tail_expression = Some(expression),
            }
        }

        // only brace-delimited expression blocks preserve a value tail
        if (!form.is_explicit() || block_context != BlockContext::Expression)
            && let Some(expression) = tail_expression.take()
        {
            leading_expressions.push(expression);
        }

        Ok(BlockBody {
            leading_expressions,
            tail_expression,
        })
    }

    /// Parse and classify one statement item with local recovery.
    fn parse_block_item(&mut self, block: Option<BlockFrame>) -> BlockItem {
        let expression =
            self.parse_expression(ExpressionPosition::Statement, ExpressionStop::default());
        let expression = match expression {
            Ok(expression) => expression,
            Err(error) => {
                let error = error.in_node(NodeType::Expression);
                let range = self.recover_statement(error.range(), error);

                return BlockItem::Statement(self.insert_node(Expression::Error, range));
            }
        };

        self.classify_block_item(expression, block)
    }

    /// Parse one standalone statement expression with local recovery.
    ///
    /// Examples:
    /// ```tspp
    /// return result;
    /// ```
    pub(crate) fn parse_statement(&mut self) -> LocalNodeId<Expression> {
        self.parse_block_item(None).expression()
    }

    /// Classify one expression as a statement or block value tail.
    fn classify_block_item(
        &mut self,
        expression: LocalNodeId<Expression>,
        block: Option<BlockFrame>,
    ) -> BlockItem {
        // expression;
        if self.peek_is(TokenType::Semicolon) {
            let semicolon = self.eat();
            self.set_node_trailing_range(expression, semicolon.span.end);

            return BlockItem::Statement(expression);
        }

        // classify the unseparated expression at its enclosing block position
        let node = self.tree.get(expression);
        let is_parenthesized = self
            .tree
            .get_side_range(
                expression,
                NodeSpanType::Region(NodeSpanRegion::Parentheses),
            )
            .is_some();
        let is_statement = !is_parenthesized && node.is_statement_boundary();
        let preserves_tail = node.preserves_value_tail_in_expression_block();
        let token_type = self.peek_token_type();
        let is_terminator = token_type == TokenType::End
            || block.is_some_and(|frame| {
                frame.form != BlockForm::Implicit && token_type == TokenType::CloseBrace
            });
        let has_separator = self.peek_is_on_new_line() || is_terminator;
        let keeps_tail = block.is_some_and(|frame| {
            frame.form.is_explicit()
                && frame.block_context == BlockContext::Expression
                && (!is_statement || preserves_tail)
        });

        // expression junk
        if !is_statement && !has_separator && !Self::can_start_recovered_statement_item(token_type)
        {
            let error = ParserError::unexpected(self.peek_token_span());
            let recovery_start = self.mark_parse_start();
            self.recover_statement(self.range_since(&recovery_start), error);

            return BlockItem::Statement(expression);
        }

        // { statements; tail } or statement
        if keeps_tail {
            BlockItem::Tail(expression)
        } else {
            BlockItem::Statement(expression)
        }
    }

    /// Return true when a token can start a recovered statement item.
    pub(crate) const fn can_start_recovered_statement_item(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::Identifier
                | TokenType::OpenBrace
                | TokenType::OpenParenthesis
                | TokenType::OpenBracket
                | TokenType::LessThan
                | TokenType::Literal
                | TokenType::At
                | TokenType::Spread
                | TokenType::Multiply
                | TokenType::ElementwiseAnd
                | TokenType::ElementwiseOr
                | TokenType::ElementwiseXor
                | TokenType::Not
                | TokenType::Hash
                | TokenType::Divide
                | TokenType::DivideAssign
        )
    }

    /// Parse one control body and normalize it to a block expression.
    pub(crate) fn parse_control_body(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        if self.peek_block() {
            let block = self.parse_block(BlockContext::Expression)?;

            return Ok(self.insert_node(Expression::Block(block), self.tree.get_range(block)));
        }

        let start = self.mark_parse_start();
        let (leading_expressions, tail_expression, block_context) = match self
            .parse_block_item(None)
        {
            BlockItem::Statement(expression) => (vec![expression], None, BlockContext::Statement),
            BlockItem::Tail(expression) => (Vec::new(), Some(expression), BlockContext::Expression),
        };
        let block = self.insert_node(
            Block {
                context: block_context,
                form: BlockForm::Implicit,
                leading_expressions,
                tail_expression,
            },
            self.range_since(&start),
        );

        Ok(self.insert_node(Expression::Block(block), self.tree.get_range(block)))
    }
}
