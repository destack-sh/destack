use crate::parse::error::ParserResultExt;
use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::{Parser, ParserError, ParserResult};

use tspp_dir::{
    Block, BlockContext, BlockForm, Expression, Keyword, LocalNodeId, NodeType, SwitchCase,
    SwitchSelector, TokenType,
};

impl Parser {
    /// Parse one switch statement.
    pub(crate) fn parse_switch(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();
        let keyword_range = self.eat_keyword(Keyword::Switch)?.range();
        let value = self.parse_parenthesized_expression(ExpressionPosition::Value)?;

        // parse the switch cases
        self.eat_token_before(TokenType::OpenBrace, TokenType::CloseBrace)
            .in_node(NodeType::SwitchCase)?;
        let cases = self.parse_switch_cases()?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::SwitchCase)?;

        // retain the complete switch and its keyword
        let expression = self.insert_node(
            Expression::Switch { value, cases },
            self.range_since(&start),
        );
        self.tree.set_main_range(expression, keyword_range);

        Ok(expression)
    }

    /// Parse switch cases until the closing brace.
    fn parse_switch_cases(&mut self) -> ParserResult<Vec<LocalNodeId<SwitchCase>>> {
        let mut cases = Vec::new();
        let mut has_default = false;
        while self.has_more_tokens() && !self.peek_is(TokenType::CloseBrace) {
            // retain one repeated Pattern placeholder as a complete case
            if self.peek_repeated_pattern_marker() {
                cases.push(self.parse_switch_case_placeholder());
                continue;
            }

            // consume separators between cases
            if Self::is_statement_stop_token(self.peek_token_type()) {
                self.eat_statement_stop()?;
                continue;
            }

            // reject a second default at its selector
            if has_default && self.peek_is_keyword(Keyword::Default) {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            // parse and retain the next case
            let case = self.parse_switch_case().in_node(NodeType::SwitchCase)?;
            if self.tree.get(case).selector == SwitchSelector::Default {
                has_default = true;
            }
            cases.push(case);
        }

        Ok(cases)
    }

    /// Parse one repeated Pattern placeholder as a complete switch case.
    fn parse_switch_case_placeholder(&mut self) -> LocalNodeId<SwitchCase> {
        let range = self.peek_token().range();
        let body = self.insert_node(
            Block {
                context: BlockContext::Statement,
                form: BlockForm::Implicit,
                leading_expressions: Vec::new(),
                tail_expression: None,
            },
            range,
        );
        let case = SwitchCase {
            selector: SwitchSelector::Default,
            body,
        };
        self.bump();

        self.insert_node(case, range)
    }

    /// Parse one switch case.
    fn parse_switch_case(&mut self) -> ParserResult<LocalNodeId<SwitchCase>> {
        let documentation = self.parse_documentation();
        let decorators = if self.peek_is(TokenType::At) {
            self.parse_decorators()
        } else {
            smallvec::SmallVec::new()
        };
        let start = self.mark_parse_start();

        // parse the selector
        let selector = if self.peek_is_keyword(Keyword::Default) {
            self.bump();
            SwitchSelector::Default
        } else {
            self.eat_keyword(Keyword::Case)?;
            let value =
                self.parse_expression(ExpressionPosition::Value, ExpressionStop::SWITCH_COLON)?;
            SwitchSelector::Case(value)
        };
        let selector_range = self.range_since(&start);
        self.eat_token(TokenType::Colon)?;
        let body_start = self.mark_parse_start();

        // collect the statement sequence before the next selector
        let mut expressions = Vec::new();
        while self.has_more_tokens()
            && !self.peek_is_keyword(Keyword::Case)
            && !self.peek_is_keyword(Keyword::Default)
            && !self.peek_is(TokenType::CloseBrace)
        {
            if self.peek_is(TokenType::Semicolon) {
                self.eat_statement_stop()?;
                continue;
            }

            expressions.push(self.parse_statement());
            if matches!(
                self.peek_token_type(),
                TokenType::Comma | TokenType::Semicolon
            ) {
                self.eat_any_stop()?;
            }
        }

        // retain the statements in one implicit block
        let body = self.insert_node(
            Block {
                context: BlockContext::Statement,
                form: BlockForm::Implicit,
                leading_expressions: expressions,
                tail_expression: None,
            },
            self.range_since(&body_start),
        );

        // retain the complete case and its selector
        let case = self.insert_node(SwitchCase { selector, body }, self.range_since(&start));
        self.tree.set_main_range(case, selector_range);

        // attach case documentation and decorators
        self.attach_documentation(case, documentation);
        self.attach_decorators(case.id, decorators);

        Ok(case)
    }
}
