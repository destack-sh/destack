use crate::parse::flags::ParserFlags;
use crate::{Parser, ParserResult, ParserSpanStart};
use destack_dir::{
    Condition, ConditionOperand, Expression, IfForm, Keyword, LocalNodeId, NodeType,
    OperatorPrecedence, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

/// The parsed head of one if expression.
pub(crate) struct IfHead {
    /// The source start for the if expression.
    pub(crate) start: ParserSpanStart,
    /// The parsed if condition.
    pub(crate) condition: Condition,
}

impl Parser {
    /// Parse an if / else expression.
    ///
    /// Examples:
    /// ```
    /// // ternary
    /// cond ? a : b
    ///
    /// // if
    /// if (x > 0) {
    ///     print("positive")
    /// }
    ///
    /// // if else
    /// if (x > 0) {
    ///     print("positive")
    /// } else {
    ///     print("not positive")
    /// }
    ///
    /// // if else if
    /// if (x > 0) {
    ///     print("positive")
    /// } else if (x == 0) {
    ///     print("zero")
    /// } else {
    ///     print("negative")
    /// }
    /// ```
    pub fn eat_if(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let head = self.eat_if_head()?;

        self.eat_if_after_head(head)
    }

    /// Return parser flags for `if` conditions.
    #[inline]
    fn if_condition_flags(&self) -> ParserFlags {
        let ambient_context = self.flags.nested().with_before_block(true);
        let expression_context = self.flags.nested();

        self.flags
            .with_ambient_context(ambient_context)
            .with_expression_context(expression_context)
    }

    /// Eat an optional else expression for an if expression.
    pub(crate) fn eat_if_else_expression_maybe(
        &mut self,
    ) -> ParserResult<Option<(LocalNodeId<Expression>, Span)>> {
        // save state so missing else can rewind cleanly
        let else_mark = self.checkpoint();
        let else_tree_mark = self.tree.next_id();

        // semicolon statement forms consume optional separators before else
        if !self.language.is_destack() {
            while self.peek_is(TokenType::Semicolon) {
                self.bump();
            }
        }

        // no else: restore speculative state
        if !self.is_keyword(Keyword::Else) {
            self.restore(else_mark, else_tree_mark);
            return Ok(None);
        }

        // else keyword
        let else_span = self.peek()?.span;
        self.eat_keyword(Keyword::Else)?;

        // else body
        let else_expression_id = self.eat_control_body_expression()?;

        Ok(Some((else_expression_id, else_span)))
    }

    /// Eat one if head.
    pub(crate) fn eat_if_head(&mut self) -> ParserResult<IfHead> {
        let start = self.span_start();

        // NOTE: ternary if is parsed in expression loop, not in eat_if

        // keyword
        self.eat_keyword(Keyword::If)?;

        // open parenthesis
        self.eat_token(TokenType::OpenParenthesis)?;

        // condition
        let condition: Condition = self.with_flags(self.if_condition_flags(), |parser| {
            parser.eat_if_condition()
        })?;

        // close the condition
        self.eat_close_token_or_recover_missing_with(
            TokenType::CloseParenthesis,
            NodeType::Expression,
            |parser, token_type| {
                Self::is_close_delimiter_boundary_token(token_type) || parser.is_block_start()
            },
        )?;

        Ok(IfHead { start, condition })
    }

    /// Eat one if condition.
    fn eat_if_condition(&mut self) -> ParserResult<Condition> {
        let checkpoint = self.checkpoint();
        let tree_mark = self.tree.next_id();
        let first = self.eat_if_condition_operand()?;
        let mut operands = Vec::new();

        // collect top-level logical-and operands
        while self.peek_is(TokenType::LogicalAnd) {
            self.bump();

            if operands.is_empty() {
                operands.push(first.clone());
            }

            let operand = self.eat_if_condition_operand()?;
            operands.push(operand);
        }

        // return a binding condition chain
        if !operands.is_empty() {
            let condition = Condition { operands };
            if condition.has_binding() {
                return Ok(condition);
            }
        } else if matches!(first, ConditionOperand::Binding { .. }) {
            return Ok(Condition {
                operands: vec![first],
            });
        }

        // otherwise parse the whole condition as a regular expression
        self.restore(checkpoint, tree_mark);
        let condition = self.eat_expression(self.flags)?;

        Ok(Condition::expression(condition))
    }

    /// Eat one operand in an if condition chain.
    fn eat_if_condition_operand(&mut self) -> ParserResult<ConditionOperand> {
        let keyword = self.peek_any_keyword().ok();
        let is_binding = keyword
            .and_then(Self::let_kind_and_mutability_for_keyword)
            .is_some();

        if is_binding {
            let (kind, mutability) = self.eat_let_kind()?;
            let minimum_precedence = OperatorPrecedence::LogicalAnd as u16 + 1;
            let declarator = self.eat_declarator(true, true, Some(minimum_precedence))?;

            Ok(ConditionOperand::Binding {
                kind,
                mutability,
                declarator,
            })
        } else {
            let minimum_precedence = OperatorPrecedence::LogicalAnd as u16 + 1;
            let condition = self.eat_expression_at_precedence(self.flags, minimum_precedence)?;

            Ok(ConditionOperand::Expression { condition })
        }
    }

    /// Insert one if expression.
    pub(crate) fn insert_if_expression(
        &mut self,
        head: IfHead,
        then_expression: LocalNodeId<Expression>,
        else_expression: Option<(LocalNodeId<Expression>, Span)>,
    ) -> LocalNodeId<Expression> {
        let else_expression_id = else_expression
            .as_ref()
            .map(|(else_expression_id, _)| *else_expression_id);

        // build the expression node
        let if_id = self.insert_node(
            Expression::If {
                form: IfForm::If,
                condition: head.condition,
                then_expression,
                else_expression: else_expression_id,
            },
            self.get_span_from(&head.start),
        );

        // attach the else clause span
        if let Some((_, else_span)) = else_expression {
            self.tree.set_side_span(
                if_id,
                NodeSpanType::Region(NodeSpanRegion::Clause),
                else_span,
            );
        }

        if_id
    }

    /// Finish one if after its then block has been parsed.
    pub(crate) fn finish_if_after_then_block(
        &mut self,
        head: IfHead,
        then_expression: LocalNodeId<Expression>,
    ) -> ParserResult<LocalNodeId<Expression>> {
        // semicolon statement forms allow a trailing then semicolon
        if !self.language.is_destack() && self.peek_is(TokenType::Semicolon) {
            self.bump();
        }

        // optional else branch
        let else_expression = self.eat_if_else_expression_maybe()?;

        Ok(self.insert_if_expression(head, then_expression, else_expression))
    }
}
