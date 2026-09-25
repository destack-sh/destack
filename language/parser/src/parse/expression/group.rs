use crate::parse::decorator::Decorators;
use crate::parse::{DeclarationHeader, ExpressionPosition, ExpressionStop};
use crate::{ParseStart, Parser, ParserResult};
use tspp_dir::{Argument, Expression, LocalNodeId, NodeType, TokenType};

impl Parser {
    /// Parse one parenthesized expression, tuple, or arrow head.
    pub(in crate::parse::expression) fn parse_parenthesized_primary(
        &mut self,
        start: &ParseStart,
        position: ExpressionPosition,
        stop: ExpressionStop,
    ) -> ParserResult<(LocalNodeId<Expression>, bool)> {
        if self.peek_parenthesized_lambda(stop) {
            let declaration = self.parse_function(start, DeclarationHeader::default(), position)?;
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

        // parse the first element's decorators here since tuple elements own them
        let element_start = self.mark_parse_start();
        let decorators = self.parse_element_decorators(position);

        let expression = self.parse_expression(position.nested(), ExpressionStop::default())?;
        if self.peek_is(TokenType::Comma) {
            return self.parse_parenthesized_tuple(
                start,
                &element_start,
                expression,
                decorators,
                position,
            );
        }

        // a sole parenthesized value keeps its decorators on its expression owner
        self.attach_expression_decorators(expression, decorators);

        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;
        self.record_parentheses(start, expression);

        Ok((expression, true))
    }

    /// Parse one expression delimited by parentheses.
    pub(crate) fn parse_parenthesized_expression(
        &mut self,
        position: ExpressionPosition,
    ) -> ParserResult<LocalNodeId<Expression>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let expression = self.parse_expression(position.nested(), ExpressionStop::default())?;
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        Ok(expression)
    }

    /// Parse tuple elements after one parenthesized expression.
    fn parse_parenthesized_tuple(
        &mut self,
        start: &ParseStart,
        first_start: &ParseStart,
        first: LocalNodeId<Expression>,
        first_decorators: Decorators,
        position: ExpressionPosition,
    ) -> ParserResult<(LocalNodeId<Expression>, bool)> {
        let first = self.insert_tuple_element(first_start, first, first_decorators);
        let mut elements = vec![first];

        // parse the remaining elements with the decorators they own
        while self.eat_token_if(TokenType::Comma) {
            if self.peek_is(TokenType::CloseParenthesis) {
                break;
            }

            let element_start = self.mark_parse_start();
            let decorators = self.parse_element_decorators(position);
            let value = self.parse_expression(position.nested(), ExpressionStop::default())?;
            elements.push(self.insert_tuple_element(&element_start, value, decorators));
        }
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        let expression = self.insert_node(
            Expression::TupleExpression { elements },
            self.range_since(start),
        );

        Ok((expression, false))
    }

    /// Parse the decorators owned by one parenthesized element.
    fn parse_element_decorators(&mut self, position: ExpressionPosition) -> Decorators {
        // a surrounding decorator owns the `@` tokens within its own value
        if position.is_decorator() {
            return Decorators::new();
        }

        self.parse_decorators()
    }

    /// Insert one tuple element around its parsed value.
    fn insert_tuple_element(
        &mut self,
        start: &ParseStart,
        value: LocalNodeId<Expression>,
        decorators: Decorators,
    ) -> LocalNodeId<Argument> {
        // decorated elements span their decorators, plain elements span their value
        let range = if decorators.is_empty() {
            self.tree.get_range(value)
        } else {
            self.range_since(start)
        };

        let element = self.insert_node(Argument::Positional { value }, range);
        self.attach_decorators(element.id, decorators);

        element
    }
}
