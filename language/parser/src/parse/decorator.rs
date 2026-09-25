use crate::parse::lookahead::DelimiterDepth;
use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::{Parser, ParserResult, TokenProbe};
use smallvec::SmallVec;
use tspp_dir::{
    Decorator, DecoratorPosition, Expression, LocalNodeId, OperatorPrecedence, TokenType,
};

/// Decorators awaiting attachment to the next owner at one parse site.
pub(crate) type Decorators = SmallVec<[LocalNodeId<Decorator>; 2]>;

impl Parser {
    /// Parse a decorator prefix sequence if present at the current token.
    ///
    /// Examples:
    /// ```tspp
    /// @sealed
    /// @route("/users")
    /// ```
    pub(crate) fn parse_decorators(&mut self) -> Decorators {
        let mut decorators = Decorators::new();

        while self.peek_is(TokenType::At) {
            let start = self.mark_parse_start();
            match self.parse_decorator() {
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
    fn parse_decorator(&mut self) -> ParserResult<LocalNodeId<Decorator>> {
        let start = self.mark_parse_start();

        // eat @ marker
        self.eat_token(TokenType::At)?;

        let expression = self.parse_expression_at(
            ExpressionPosition::DecoratorHead,
            ExpressionStop::default(),
            OperatorPrecedence::Primary,
        )?;

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

impl TokenProbe<'_> {
    /// Advance past decorator targets and their applications.
    pub(in crate::parse) fn scan_decorators(&mut self) -> bool {
        while self.peek_token_type() == TokenType::At {
            self.bump();

            // scan the decorator target
            if self.peek_token_type() == TokenType::Identifier {
                self.bump();
            } else if !self
                .scan_delimiter_group(TokenType::OpenParenthesis, TokenType::CloseParenthesis)
            {
                return false;
            }

            loop {
                let token = self.peek_token();
                let token_type = token.ty();

                // stop before the decorated owner
                if token.is_on_new_line() && token_type != TokenType::Dot {
                    break;
                }

                // scan named member access
                if token_type == TokenType::Dot {
                    self.bump();
                    if self.peek_token_type() != TokenType::Identifier {
                        return false;
                    }
                    self.bump();

                    continue;
                }

                // scan one decorator application
                if token_type == TokenType::OpenParenthesis {
                    if !self.scan_delimiter_group(
                        TokenType::OpenParenthesis,
                        TokenType::CloseParenthesis,
                    ) {
                        return false;
                    }

                    continue;
                }

                // scan a balanced generic postfix
                if matches!(token_type, TokenType::LessThan | TokenType::ShiftLeft) {
                    let mut delimiters = DelimiterDepth::type_expression();
                    loop {
                        let token_type = self.peek_token_type();
                        if token_type == TokenType::End || !delimiters.advance(token_type) {
                            return false;
                        }

                        self.bump();
                        if delimiters.is_top_level() {
                            break;
                        }
                    }

                    continue;
                }

                break;
            }
        }

        true
    }
}
