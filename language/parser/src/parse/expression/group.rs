use crate::parse::DeclarationHeader;
use crate::parse::context::ExpressionContext;
use crate::{ParseStart, Parser, ParserResult};
use destack_dir::{Argument, Expression, LocalNodeId, NodeType, TokenType};

impl Parser {
    /// Parse one parenthesized expression, tuple, or arrow head.
    pub(in crate::parse::expression) fn parse_parenthesized_primary(
        &mut self,
        start: &ParseStart,
        context: ExpressionContext,
    ) -> ParserResult<(LocalNodeId<Expression>, bool)> {
        if self.peek_parenthesized_lambda(context) {
            let declaration = self.parse_function(start, DeclarationHeader::default(), context)?;
            let expression = self.insert_declaration_expression(start, declaration);

            return Ok((expression, false));
        }

        self.eat_token(TokenType::OpenParenthesis)?;
        if self.eat_token_if(TokenType::CloseParenthesis) {
            let expression = self.insert_node(
                Expression::TupleExpression {
                    elements: Vec::new(),
                },
                self.range_since(start),
            );

            return Ok((expression, false));
        }

        let expression = self.parse_expression(context.nested())?;
        if self.peek_is(TokenType::Comma) {
            return self.parse_parenthesized_tuple(start, expression, context);
        }

        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;
        self.record_parentheses(start, expression);

        Ok((expression, true))
    }

    /// Parse one expression delimited by parentheses.
    pub(crate) fn parse_parenthesized_expression(
        &mut self,
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let expression = self.parse_expression(context.nested())?;
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        Ok(expression)
    }

    /// Parse tuple elements after one parenthesized expression.
    fn parse_parenthesized_tuple(
        &mut self,
        start: &ParseStart,
        first: LocalNodeId<Expression>,
        context: ExpressionContext,
    ) -> ParserResult<(LocalNodeId<Expression>, bool)> {
        let first = self.insert_node(
            Argument::Positional { value: first },
            self.tree.get_range(first),
        );
        let mut elements = vec![first];
        while self.eat_token_if(TokenType::Comma) {
            if self.peek_is(TokenType::CloseParenthesis) {
                break;
            }

            let value = self.parse_expression(context.nested())?;
            elements
                .push(self.insert_node(Argument::Positional { value }, self.tree.get_range(value)));
        }
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        let expression = self.insert_node(
            Expression::TupleExpression { elements },
            self.range_since(start),
        );

        Ok((expression, false))
    }
}
