use crate::parse::context::{DecoratorContext, ExpressionContext, FunctionContext};
use crate::{Parser, ParserResult};
use destack_dir::{
    Decorator, DecoratorPosition, Expression, LocalNodeId, OperatorPrecedence, TokenType,
};
use smallvec::SmallVec;

/// Decorators awaiting attachment to the next owner at one parse site.
pub(crate) type Decorators = SmallVec<[LocalNodeId<Decorator>; 2]>;

impl Parser {
    /// Parse a decorator prefix sequence if present at the current token.
    ///
    /// Examples:
    /// ```ds
    /// @sealed
    /// @route("/users")
    /// ```
    pub(crate) fn parse_decorators(&mut self, function: FunctionContext) -> Decorators {
        let mut decorators = Decorators::new();

        while self.peek_is(TokenType::At) {
            let start = self.mark_parse_start();
            match self.parse_decorator(function) {
                Ok(decorator) => decorators.push(decorator),
                Err(error) => {
                    let range = self.range_since(&start);
                    self.recover_statement(range, error);
                }
            }
        }

        decorators
    }

    /// Attach decorators to the node that owns one complete expression.
    pub(crate) fn attach_expression_decorators(
        &mut self,
        expression: LocalNodeId<Expression>,
        decorators: Decorators,
    ) {
        match self.tree.get(expression) {
            Expression::Declaration(declaration) => {
                let declaration = *declaration;
                self.attach_decorators(declaration.id, decorators);
            }
            Expression::Missing | Expression::Error => {}
            _ => self.attach_decorators(expression.id, decorators),
        }
    }

    /// Attach decorators to an owner node in source order.
    pub(crate) fn attach_decorators(&mut self, target_node_id: u32, decorators: Decorators) {
        if decorators.is_empty() {
            return;
        }

        for decorator in decorators {
            self.tree.attach_decorator(target_node_id, decorator);
        }
    }

    /// Parse one decorator expression.
    fn parse_decorator(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Decorator>> {
        let start = self.mark_parse_start();

        // eat @ marker
        self.eat_token(TokenType::At)?;

        let expression = self.parse_expression(ExpressionContext {
            function,
            decorator: DecoratorContext::Head,
            minimum_precedence: OperatorPrecedence::Primary,
            ..ExpressionContext::default()
        })?;

        // store decorator side node
        let decorator = self.insert_node(
            Decorator {
                expression,
                position: DecoratorPosition::BlockPrefix,
            },
            self.range_since(&start),
        );
        let main_span = self
            .tree
            .get_main_range(expression)
            .unwrap_or_else(|| self.tree.get_range(expression));
        self.tree.set_main_range(decorator, main_span);
        Ok(decorator)
    }
}
