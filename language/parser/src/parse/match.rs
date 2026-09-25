use crate::parse::error::ParserResultExt;
use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::{Parser, ParserResult};

use tspp_dir::{
    BlockContext, Condition, Expression, Keyword, LocalNodeId, MatchArm, NodeType, Pattern,
    TokenType,
};
use tspp_source::{ByteRange, NodeSpanRegion, NodeSpanType};

/// One match guard and its source range.
struct MatchGuard {
    /// The guard condition.
    condition: Condition,
    /// The complete guard clause range.
    range: ByteRange,
}

impl Parser {
    /// Parse one match expression.
    pub(crate) fn parse_match(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        let keyword_range = self.eat_keyword(Keyword::Match)?.range();
        let value = self.parse_parenthesized_expression(ExpressionPosition::Value)?;

        // parse the match arms
        self.eat_token_before(TokenType::OpenBrace, TokenType::CloseBrace)
            .in_node(NodeType::MatchArm)?;
        let arms = self.parse_match_arms()?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::MatchArm)?;

        // retain the complete match and its keyword
        let expression =
            self.insert_node(Expression::Match { value, arms }, self.range_since(&start));
        self.tree.set_main_range(expression, keyword_range);

        Ok(expression)
    }

    /// Parse match arms until the closing brace.
    fn parse_match_arms(&mut self) -> ParserResult<Vec<LocalNodeId<MatchArm>>> {
        let mut arms = Vec::new();
        while self.has_more_tokens() && !self.peek_is(TokenType::CloseBrace) {
            // retain one repeated Pattern placeholder as a complete arm
            if self.peek_repeated_pattern_marker() {
                arms.push(self.parse_match_arm_placeholder());
                continue;
            }

            // consume separators between arms
            if Self::is_statement_stop_token(self.peek_token_type()) {
                self.eat_statement_stop()?;
                continue;
            }

            let arm = self.parse_match_arm().in_node(NodeType::MatchArm)?;
            arms.push(arm);
        }

        Ok(arms)
    }

    /// Parse one repeated Pattern placeholder as a complete match arm.
    fn parse_match_arm_placeholder(&mut self) -> LocalNodeId<MatchArm> {
        let range = self.peek_token().range();
        let pattern = self.insert_node(Pattern::Wildcard, range);
        let body = self.insert_node(Expression::Error, range);
        let arm = MatchArm::Expression {
            pattern,
            guard: None,
            body,
        };
        self.bump();

        self.insert_node(arm, range)
    }

    /// Parse one match arm.
    fn parse_match_arm(&mut self) -> ParserResult<LocalNodeId<MatchArm>> {
        let documentation = self.parse_documentation();
        let decorators = if self.peek_is(TokenType::At) {
            self.parse_decorators()
        } else {
            smallvec::SmallVec::new()
        };
        let start = self.mark_parse_start();

        // parse the arm head
        let pattern = self.parse_pattern()?;
        let guard = self.parse_match_guard()?;
        self.eat_token(TokenType::ArrowWide)?;

        // retain the authored body form
        let (guard, guard_range) = match guard {
            Some(guard) => (Some(guard.condition), Some(guard.range)),
            None => (None, None),
        };
        let arm = if self.peek_block() {
            let body = self.parse_block(BlockContext::Expression)?;
            MatchArm::Block {
                pattern,
                guard,
                body,
            }
        } else {
            let body =
                self.parse_expression(ExpressionPosition::Value, ExpressionStop::MATCH_ARM_LINE)?;
            MatchArm::Expression {
                pattern,
                guard,
                body,
            }
        };
        let arm = self.insert_node(arm, self.range_since(&start));

        // retain the optional guard clause, documentation and decorators
        if let Some(guard_range) = guard_range {
            self.tree.set_side_range(
                arm,
                NodeSpanType::Region(NodeSpanRegion::Guard),
                guard_range,
            );
        }

        self.attach_documentation(arm, documentation);
        self.attach_decorators(arm.id, decorators);

        Ok(arm)
    }

    /// Parse a match guard when present.
    fn parse_match_guard(&mut self) -> ParserResult<Option<MatchGuard>> {
        if !self.peek_is_keyword(Keyword::If) {
            return Ok(None);
        }

        let start = self.mark_parse_start();
        self.eat_keyword(Keyword::If)?;
        let condition = self
            .parse_parenthesized_condition(ExpressionPosition::Value, ExpressionStop::default())?;
        let range = self.range_since(&start);

        Ok(Some(MatchGuard { condition, range }))
    }
}
