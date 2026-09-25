use crate::parse::r#let::DeclaratorValue;
use crate::parse::lookahead::DelimiterDepth;
use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::{Parser, ParserResult, TokenProbe};
use tspp_dir::{Condition, ConditionOperand, Keyword, NodeType, OperatorPrecedence, TokenType};

impl Parser {
    /// Parse one parenthesized control condition.
    pub(crate) fn parse_parenthesized_condition(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<Condition> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let condition = self.parse_condition(position, stop)?;
        self.eat_close_token_or_recover_missing_with(
            TokenType::CloseParenthesis,
            NodeType::Expression,
            |parser, token_type| {
                Self::is_close_delimiter_boundary_token(token_type) || parser.peek_block()
            },
        )?;

        Ok(condition)
    }

    /// Parse one control condition after its opening delimiter.
    fn parse_condition(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<Condition> {
        // preserve ordinary expressions as one operand
        if !self.peek_condition_binding_operand() {
            let condition = self.parse_expression(position, stop)?;

            return Ok(Condition::expression(condition));
        }

        let first = self.parse_condition_operand(position, stop)?;
        let mut operands = vec![first];

        // collect top-level logical-and operands
        while self.peek_is(TokenType::LogicalAnd) {
            self.bump();

            let operand = self.parse_condition_operand(position, stop)?;
            operands.push(operand);
        }

        Ok(Condition { operands })
    }

    /// Return whether the current condition contains a top-level binding operand.
    fn peek_condition_binding_operand(&self) -> bool {
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

    /// Parse one operand in a condition chain.
    fn parse_condition_operand(
        &mut self,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<ConditionOperand> {
        let keyword = self.peek_keyword();
        let is_binding = keyword
            .and_then(super::r#let::LetHead::from_keyword)
            .is_some()
            && (keyword != Some(Keyword::Const) || {
                let mut probe = self.cursor.probe(&self.file);
                probe.scan_const_binding_operand()
            });

        // parse a declaration operand
        if is_binding {
            let head = self.parse_let_head()?;
            let declarator =
                self.parse_declarator(DeclaratorValue::Required(OperatorPrecedence::LogicalAnd))?;

            Ok(ConditionOperand::Binding {
                kind: head.kind,
                mutability: head.mutability,
                declarator,
            })
        }
        // parse one boolean operand without consuming the next conjunction
        else {
            let condition =
                self.parse_expression_at(position, stop, OperatorPrecedence::LogicalAnd)?;

            Ok(ConditionOperand::Expression { condition })
        }
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
