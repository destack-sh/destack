use crate::parse::context::{ExpressionContext, FunctionContext};
use crate::parse::r#let::DeclaratorValue;
use crate::parse::lookahead::DelimiterDepth;
use crate::{ParseStart, Parser, ParserResult, TokenProbe};
use destack_dir::{
    Condition, ConditionOperand, Expression, IfForm, Keyword, LocalNodeId, NodeType,
    OperatorPrecedence, TokenType,
};
use destack_source::{ByteRange, NodeSpanRegion, NodeSpanType};

/// The head of one if expression.
struct IfHead {
    /// The source start for the if expression.
    start: ParseStart,
    /// The `if` keyword range.
    keyword_range: ByteRange,
    /// The if condition.
    condition: Condition,
}

/// One optional else clause.
struct ElseClause {
    /// The else branch expression.
    expression: LocalNodeId<Expression>,
    /// The `else` keyword range.
    range: ByteRange,
}

impl Parser {
    /// Parse one if expression.
    ///
    /// Examples:
    /// ```ds
    /// if (ready) run() else wait()
    /// ```
    pub(crate) fn parse_if(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let head = self.parse_if_head(function)?;
        let then_expression = self.parse_control_body(function)?;
        let else_clause = self.parse_else_clause(function)?;

        Ok(self.insert_if_expression(head, then_expression, else_clause))
    }

    /// Parse an optional else expression for an if expression.
    fn parse_else_clause(&mut self, function: FunctionContext) -> ParserResult<Option<ElseClause>> {
        if !self.peek_else_after_semicolons() {
            return Ok(None);
        }

        // consume branch terminators only when they lead to else
        while self.eat_token_if(TokenType::Semicolon) {}

        let else_range = self.peek_token_span().span.range();
        self.eat_keyword(Keyword::Else)?;
        let expression = if self.peek_is_keyword(Keyword::If) {
            self.parse_if(function)?
        } else {
            self.parse_control_body(function)?
        };

        Ok(Some(ElseClause {
            expression,
            range: else_range,
        }))
    }

    /// Return whether optional semicolons are followed by `else`.
    fn peek_else_after_semicolons(&self) -> bool {
        let mut probe = self.cursor.probe(&self.file);
        while probe.peek_token_type() == TokenType::Semicolon {
            probe.bump();
        }

        probe.peek_keyword() == Some(Keyword::Else)
    }

    /// Parse one if head.
    fn parse_if_head(&mut self, function: FunctionContext) -> ParserResult<IfHead> {
        let start = self.mark_parse_start();

        // keyword
        let keyword_range = self.eat_keyword(Keyword::If)?.range();

        // open parenthesis
        self.eat_token(TokenType::OpenParenthesis)?;

        let condition = self.parse_if_condition(function)?;

        // close the condition
        self.eat_close_token_or_recover_missing_with(
            TokenType::CloseParenthesis,
            NodeType::Expression,
            |parser, token_type| {
                Self::is_close_delimiter_boundary_token(token_type) || parser.peek_block()
            },
        )?;

        Ok(IfHead {
            start,
            keyword_range,
            condition,
        })
    }

    /// Parse one if condition.
    fn parse_if_condition(&mut self, function: FunctionContext) -> ParserResult<Condition> {
        if !self.peek_if_condition_binding_operand() {
            let condition = self.parse_expression(ExpressionContext {
                function,
                ..ExpressionContext::default()
            })?;

            return Ok(Condition::expression(condition));
        }

        let first = self.parse_if_condition_operand(function)?;
        let mut operands = vec![first];

        // collect top-level logical-and operands
        while self.peek_is(TokenType::LogicalAnd) {
            self.bump();

            let operand = self.parse_if_condition_operand(function)?;
            operands.push(operand);
        }

        Ok(Condition { operands })
    }

    /// Return whether the current if condition contains a top-level binding operand.
    fn peek_if_condition_binding_operand(&self) -> bool {
        let mut probe = self.cursor.probe(&self.file);
        let mut depth = DelimiterDepth::value();
        let mut is_operand_start = true;

        // scan the condition once without mutating parser state
        loop {
            let token_type = probe.peek_token_type();
            if depth.is_top_level() && token_type == TokenType::CloseParenthesis {
                break;
            }
            if token_type == TokenType::End {
                break;
            }

            // binding condition chains split only at top-level logical-and
            if depth.is_top_level() && token_type == TokenType::LogicalAnd {
                is_operand_start = true;
                probe.bump();
                continue;
            }

            // accept binding operands only at operand starts
            if is_operand_start
                && depth.is_top_level()
                && probe
                    .peek_keyword()
                    .and_then(super::r#let::LetHead::from_keyword)
                    .is_some()
            {
                // const binds only ahead of a top-level initializer
                if probe.peek_keyword() != Some(Keyword::Const)
                    || probe.scan_const_binding_operand()
                {
                    return true;
                }

                is_operand_start = false;
                continue;
            }

            // stop on malformed nested delimiters
            if !depth.advance(token_type) {
                break;
            }

            // nested delimiters cannot contribute top-level operands
            if depth.is_top_level() {
                is_operand_start = false;
            }

            probe.bump();
        }

        false
    }

    /// Parse one operand in an if condition chain.
    fn parse_if_condition_operand(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<ConditionOperand> {
        let keyword = self.peek_keyword();
        let is_binding = keyword
            .and_then(super::r#let::LetHead::from_keyword)
            .is_some()
            && (keyword != Some(Keyword::Const) || {
                let mut probe = self.cursor.probe(&self.file);
                probe.scan_const_binding_operand()
            });

        if is_binding {
            let head = self.parse_let_head()?;
            let declarator = self.parse_declarator(
                function,
                DeclaratorValue::Required(OperatorPrecedence::LogicalAnd),
            )?;

            Ok(ConditionOperand::Binding {
                kind: head.kind,
                mutability: head.mutability,
                declarator,
            })
        } else {
            let condition = self.parse_expression(ExpressionContext {
                function,
                minimum_precedence: OperatorPrecedence::LogicalAnd,
                ..ExpressionContext::default()
            })?;

            Ok(ConditionOperand::Expression { condition })
        }
    }

    /// Insert one if expression.
    fn insert_if_expression(
        &mut self,
        head: IfHead,
        then_expression: LocalNodeId<Expression>,
        else_clause: Option<ElseClause>,
    ) -> LocalNodeId<Expression> {
        let else_expression = else_clause.as_ref().map(|clause| clause.expression);

        // insert the expression node
        let if_id = self.insert_node(
            Expression::If {
                form: IfForm::If,
                condition: head.condition,
                then_expression,
                else_expression,
            },
            self.range_since(&head.start),
        );
        self.tree.set_main_range(if_id, head.keyword_range);

        // attach the else clause span
        if let Some(clause) = else_clause {
            self.tree.set_side_range(
                if_id,
                NodeSpanType::Region(NodeSpanRegion::Else),
                clause.range,
            );
        }

        if_id
    }
}

impl TokenProbe<'_> {
    /// Return whether a const keyword heads a binding condition operand.
    fn scan_const_binding_operand(&mut self) -> bool {
        self.bump();

        // a binding operand demands a top-level initializer before the operand ends
        let mut depth = DelimiterDepth::value();
        loop {
            let token_type = self.peek_token_type();
            if depth.is_top_level()
                && matches!(
                    token_type,
                    TokenType::Assign | TokenType::LogicalAnd | TokenType::CloseParenthesis
                )
            {
                return token_type == TokenType::Assign;
            }
            if token_type == TokenType::End || !depth.advance(token_type) {
                return false;
            }

            self.bump();
        }
    }
}
