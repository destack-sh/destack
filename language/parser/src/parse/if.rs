use crate::parse::context::{ExpressionContext, FunctionContext};
use crate::{ParseStart, Parser, ParserResult};
use destack_dir::{Condition, Expression, IfForm, Keyword, LocalNodeId, TokenType};
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

        // condition
        let condition = self.parse_parenthesized_condition(ExpressionContext {
            function,
            ..ExpressionContext::default()
        })?;

        Ok(IfHead {
            start,
            keyword_range,
            condition,
        })
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
