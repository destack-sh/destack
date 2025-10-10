//! Parse expressions. Mostly defers to other parsers.

use crate::parse::prelude::*;
use crate::{
    Argument, AssignOperator, ParserError, AstResult, BinaryOperator, Expression, InfixOperator,
    Keyword, Mutability, NodeId, NodeType, Parser, ParserMark, Runtime, ScopedMutability,
    TokenSpan, TokenType, UnaryOperator, Visibility,
};

// can't use anything with `<` or `>` in static arguments
// (to avoid parsing ambiguity with `<>` brackets)
static NOT_IN_STATIC_BINARY_OPERATORS: [BinaryOperator; 7] = [
    // shift
    BinaryOperator::ShiftLeft,
    BinaryOperator::SaturatingShiftLeft,
    BinaryOperator::ShiftRight,
    // comparison
    BinaryOperator::LessThan,
    BinaryOperator::LessThanOrEqual,
    BinaryOperator::GreaterThan,
    BinaryOperator::GreaterThanOrEqual,
];

/// Make an infix operator.
#[inline]
fn to_infix_operator(
    token_str: &str,
    token: &TokenSpan,
    next_token: &TokenSpan,
    options: ParserOptions,
) -> AstResult<(InfixOperator, u8)> {
    // special case for shift right to avoid ungluing ambiguity
    if !options.in_static
        && token.token.ty == TokenType::GreaterThan
        && next_token.token.ty == TokenType::GreaterThan
    {
        Ok((InfixOperator::Binary(BinaryOperator::ShiftRight), 2))
    }
    // regular binary operator
    // (only a subset of binary operators are allowed in static types)
    else if let Some(binary_operator) = BinaryOperator::from_token(token_str, token.token.ty)
        && (!options.in_static || !NOT_IN_STATIC_BINARY_OPERATORS.contains(&binary_operator))
    {
        Ok((InfixOperator::Binary(binary_operator), 1))
    }
    // regular assign operator
    // (not allowed in static arguments)
    else if !options.in_static
        && !options.in_type
        && let Some(assign_operator) = AssignOperator::from_token(token.token.ty)
    {
        Ok((InfixOperator::Assign(assign_operator), 1))
    }
    // unexpected
    else {
        Err(ParserError::unexpected(token.span))
    }
}

impl<'a> Parser<'a> {
    /// Peek a unary operator.
    #[inline]
    pub fn peek_unary_operator(&self) -> AstResult<UnaryOperator> {
        let token = self.peek()?;
        UnaryOperator::from_token_type(token.token.ty).ok_or(ParserError::unexpected(token.span))
    }

    /// Peek a next unary operator.
    #[inline]
    pub fn peek_next_unary_operator(&self) -> AstResult<UnaryOperator> {
        let token = self.peek_next()?;
        UnaryOperator::from_token_type(token.token.ty).ok_or(ParserError::unexpected(token.span))
    }

    /// Peek an infix operator.
    #[inline]
    pub fn peek_infix_operator(&self) -> AstResult<(InfixOperator, u8)> {
        let token = self.peek()?;
        let token_str = self.get_span_str(token.span);
        let next_token = self.peek_next()?;
        to_infix_operator(token_str, token, next_token, self.options)
    }

    /// Peek a next infix operator.
    #[inline]
    pub fn peek_next_infix_operator(&self) -> AstResult<(InfixOperator, u8)> {
        let token = self.peek_next()?;
        let token_str = self.get_span_str(token.span);
        let next_token = self.peek_next_next()?;
        to_infix_operator(token_str, token, next_token, self.options)
    }

    /// Make an expression from an infix operator.
    #[inline]
    fn make_infix_expression(
        &self,
        left: NodeId<Expression>,
        operator: InfixOperator,
        right: NodeId<Expression>,
    ) -> Expression {
        match operator {
            InfixOperator::Binary(binary_operator) => Expression::Binary {
                left,
                operator: binary_operator,
                right,
            },
            InfixOperator::Assign(assign_operator) => Expression::Assign {
                left,
                operator: assign_operator,
                right,
            },
        }
    }

    /// Try to eat an expression as a statement (return Expression::Error if error and recovery is possible).
    #[inline]
    pub fn try_eat_expression_as_statement(&mut self) -> AstResult<NodeId<Expression>> {
        self.with_options(self.options.in_statement(), |parser| {
            parser.try_eat_expression(TokenType::Newline)
        })
    }

    /// Try to eat an expression (return Expression::Error if error and recovery is possible).
    #[inline]
    pub fn try_eat_expression(&mut self, recover: TokenType) -> AstResult<NodeId<Expression>> {
        match self.eat_expression() {
            Ok(expression_id) => Ok(expression_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let start = ParserMark::new(span.start as usize);
                self.try_recover(start, recover, Some(err))?;
                let error_id = self
                    .tree
                    .allocate(Expression::Error, self.get_span_from(start));
                Ok(error_id)
            }
        }
    }

    /// Peek a member access of the given token type.
    /// Returns the total distance to eat (including the newlines, dot, and token).
    #[inline]
    fn peek_member(&self, token_type: TokenType) -> AstResult<u8> {
        // immediate member access
        if self.peek_token(TokenType::Dot).is_ok() && self.peek_next_token(token_type).is_ok() {
            Ok(2)
        }
        // member access across newline
        else if self.peek_token(TokenType::Newline).is_ok()
            && self.peek_next_token(TokenType::Dot).is_ok()
            && self.peek_next_next_token(token_type).is_ok()
        {
            Ok(3)
        }
        // nothing
        else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Eat an expression.
    pub fn eat_expression(&mut self) -> AstResult<NodeId<Expression>> {
        let start = self.mark();

        // visibility
        let visibility: Option<Visibility> = match self.peek_visibility() {
            Ok(Some(visibility)) => {
                self.bump(); // eat visibility
                Some(visibility)
            }
            _ => None,
        };

        // runtime
        let mut runtime = if self.peek_token(TokenType::At).is_ok() {
            self.bump(); // eat @
            Some(Runtime::Static)
        } else {
            None
        };

        let mut left_expression_id: NodeId<Expression> = {
            let token = self.peek()?;
            let keyword = self.peek_any_keyword().ok();
            #[cfg(debug_assertions)]
            let _token_str = self.get_span_str(token.span);

            //
            // ------------------------------------------------------------
            // Grouping
            // ------------------------------------------------------------
            //

            // parenthesis (may be tuple or just a parenthesized expression)
            if token.token.ty == TokenType::OpenParenthesis {
                self.bump(); // eat open paranthesis
                self.eat_newlines_maybe()?;

                // if we immediately see a closing parenthesis, it's an empty tuple
                if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                    self.bump(); // eat closing parenthesis
                    self.tree.allocate(
                        Expression::TupleLiteral { elements: vec![] },
                        self.get_span_from(start),
                    )
                }
                // named tuple element, must be some tuple
                else if self.peek_token(TokenType::Identifier).is_ok()
                    && self.peek_next_token(TokenType::Colon).is_ok()
                {
                    let tuple_elements = self
                        .eat_tuple_literal_body(None)
                        .for_node_type(NodeType::Expression)?;
                    self.eat_token(TokenType::CloseParenthesis)?;
                    self.tree.allocate(
                        Expression::TupleLiteral {
                            elements: tuple_elements,
                        },
                        self.get_span_from(start),
                    )
                }
                // may be a tuple with anonymous elements or just a parenthesized expression (see below)
                else {
                    let inner_start = self.pos();
                    let expression_id = self
                        .with_options(self.options.in_parenthesis(), |parser| {
                            parser.eat_expression()
                        })?;
                    self.eat_token(TokenType::CloseParenthesis)?;
                    match self.tree.get(expression_id) {
                        // if it was a tuple starting here, expand it to cover the entire span
                        //  (except if that tuple has its own parenthesis already when nesting)
                        Expression::TupleLiteral { .. }
                            if self.tokens[inner_start as usize].token.ty
                                != TokenType::OpenParenthesis =>
                        {
                            self.tree.set_span(expression_id, self.get_span_from(start));
                            expression_id
                        }
                        // otherwise it was a manually parenthesized expression, wrap it
                        _ => self.tree.allocate(
                            Expression::Parenthesized {
                                expression: expression_id,
                            },
                            self.get_span_from(start),
                        ),
                    }
                }
            }
            //
            // ------------------------------------------------------------
            // Unary operations (prefix, right associative)
            // ------------------------------------------------------------
            //

            // unary operations
            else if let Ok(unary_operator) = self.peek_unary_operator() {
                let right_precedence = unary_operator.precedence();
                self.bump(); // eat unary operator (always because right associative)
                let right = self.with_options(
                    self.options.in_left_precedence(right_precedence),
                    |parser| parser.eat_expression(),
                )?;
                let expression = Expression::Unary {
                    operator: unary_operator,
                    right,
                };
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // reference (`&` or `&var` or `&const`)
            else if self.peek_token(TokenType::ElementwiseAnd).is_ok() {
                self.bump(); // eat &
                let mutability = if self.peek_keyword(Keyword::Var).is_ok()
                    || self.peek_keyword(Keyword::Const).is_ok()
                    || self.peek_keyword(Keyword::Mut).is_ok()
                {
                    self.eat_scoped_mutability()
                        .for_node_type(NodeType::Expression)?
                } else {
                    ScopedMutability::Unscoped {
                        mutability: Mutability::Immutable,
                    }
                };
                let right = self.eat_expression()?;
                let expression = Expression::Reference { mutability, right };
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Declarations
            // ------------------------------------------------------------
            //

            // module
            else if keyword == Some(Keyword::Module) {
                let module_id = self.eat_module(visibility)?;
                self.tree
                    .allocate(Expression::Definition(module_id), self.get_span_from(start))
            }
            // struct
            else if keyword == Some(Keyword::Struct) {
                let struct_id = self.eat_struct(visibility)?;
                self.tree
                    .allocate(Expression::Definition(struct_id), self.get_span_from(start))
            }
            // enum
            else if keyword == Some(Keyword::Enum) {
                let enum_id = self.eat_enum(visibility)?;
                self.tree
                    .allocate(Expression::Definition(enum_id), self.get_span_from(start))
            }
            // union
            else if keyword == Some(Keyword::Union) {
                let union_id = self.eat_union(visibility)?;
                self.tree
                    .allocate(Expression::Definition(union_id), self.get_span_from(start))
            }
            // trait
            else if keyword == Some(Keyword::Trait) {
                let trait_id = self.eat_trait(visibility)?;
                self.tree
                    .allocate(Expression::Definition(trait_id), self.get_span_from(start))
            }
            // implement
            else if keyword == Some(Keyword::Implement) {
                let implement_id = self.eat_implement()?;
                self.tree.allocate(
                    Expression::Definition(implement_id),
                    self.get_span_from(start),
                )
            }
            // function
            else if keyword == Some(Keyword::Function) {
                let function_id = self.eat_function(visibility)?;
                self.tree.allocate(
                    Expression::Definition(function_id),
                    self.get_span_from(start),
                )
            }
            // block
            else if self.peek_block().is_ok() {
                let block_id = self.eat_block()?;
                self.tree
                    .allocate(Expression::Block(block_id), self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Control flow
            // ------------------------------------------------------------
            //
            // with
            else if keyword == Some(Keyword::With) {
                self.eat_with()?
            }
            // use
            else if keyword == Some(Keyword::Use) {
                self.eat_use(visibility)?
            }
            // let
            else if keyword == Some(Keyword::Let)
                || keyword == Some(Keyword::Var)
                || keyword == Some(Keyword::Const)
            {
                self.eat_let(visibility)?
            }
            // if
            else if keyword == Some(Keyword::If) {
                self.eat_if(runtime)?
            }
            // while
            else if keyword == Some(Keyword::While) {
                self.eat_while(runtime)?
            }
            // for
            else if keyword == Some(Keyword::For) {
                self.eat_for(runtime)?
            }
            // loop
            else if keyword == Some(Keyword::Loop) {
                self.eat_loop(runtime)?
            }
            // try
            else if keyword == Some(Keyword::Try) {
                self.eat_try(runtime)?
            }
            // match
            else if keyword == Some(Keyword::Match) {
                self.eat_match(runtime)?
            }
            // break
            else if keyword == Some(Keyword::Break) {
                self.eat_break()?
            }
            // continue
            else if keyword == Some(Keyword::Continue) {
                self.eat_continue()?
            }
            // defer
            else if keyword == Some(Keyword::Defer) {
                self.eat_defer()?
            }
            // return
            else if keyword == Some(Keyword::Return) {
                self.eat_return()?
            }
            //
            // ------------------------------------------------------------
            // Literals / Aliases
            // ------------------------------------------------------------
            //
            // array
            else if token.token.ty == TokenType::OpenBracket {
                let array_literal = self.eat_array_literal()?;
                self.tree.allocate(
                    Expression::ArrayLiteral {
                        elements: array_literal,
                    },
                    self.get_span_from(start),
                )
            }
            // scalar
            else if self.peek_scalar_literal().is_ok() {
                let scalar_literal = self.eat_scalar_literal()?;
                self.tree.allocate(
                    Expression::ScalarLiteral(scalar_literal),
                    self.get_span_from(start),
                )
            }
            // type
            // nocheckin TODO #Broken: only consider type literals in type parser context?
            //  (they might be shadowed, so need to resolve the others at DIR-level?)
            else if self.peek_type_literal().is_ok() {
                let type_literal = self.eat_type_literal()?;
                self.tree.allocate(
                    Expression::TypeLiteral(type_literal),
                    self.get_span_from(start),
                )
            }
            // alias / path
            else if token.token.ty == TokenType::Identifier {
                let path_id = self.eat_path().for_node_type(NodeType::Expression)?;

                // speculatively unwrap postfix static parameterisation with `<`
                //  (might also be just a comparison operator)
                let speculative_start = self.mark();
                let speculative_start_idx = self.tree.next_id();
                let static_arguments = if self.peek_token(TokenType::LessThan).is_ok() {
                    match self.eat_static_arguments() {
                        Ok(static_arguments) => Some(static_arguments),
                        Err(_) => {
                            self.restore(speculative_start, speculative_start_idx);
                            None
                        }
                    }
                } else {
                    None
                };
                let expression = Expression::Path {
                    path: path_id,
                    static_arguments,
                };
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Error
            // ------------------------------------------------------------
            //
            else {
                return Err(ParserError::unexpected(self.peek()?.span));
            }
        };

        //
        // ------------------------------------------------------------
        // Postfix operations
        // ------------------------------------------------------------
        //

        // implicitly call static functions without arguments (e.g., `@entity`)
        if runtime.is_some()
            && let Expression::Path { .. } = self.tree.get(left_expression_id)
            && self.peek_token(TokenType::OpenParenthesis).is_err()
        {
            left_expression_id = self.tree.allocate(
                Expression::Call {
                    runtime,
                    receiver: left_expression_id,
                    dynamic_arguments: vec![],
                },
                self.get_span_from(start),
            );
            runtime = None;
        }
        // struct literal postfix with `{`
        else if let Expression::Path { .. } = self.tree.get(left_expression_id)
            && self.peek_token(TokenType::OpenBrace).is_ok()
            && !self.options.in_before_block
        {
            let fields = self.eat_struct_literal_body()?;
            left_expression_id = self.tree.allocate(
                Expression::StructLiteral {
                    ty: left_expression_id,
                    fields,
                },
                self.get_span_from(start),
            );
        }

        // eat all regular postfix operators
        loop {
            // range (`..`, `..=`)
            if self.peek_token(TokenType::Range).is_ok()
                || self.peek_token(TokenType::RangeWide).is_ok()
            {
                self.bump(); // eat ..
                let is_inclusive = if self.peek_token(TokenType::Equal).is_ok() {
                    self.bump(); // eat =
                    true
                } else {
                    false
                };
                let right_expression_id = self.eat_expression()?;
                left_expression_id = self.tree.allocate(
                    Expression::RangeLiteral {
                        start: left_expression_id,
                        end: right_expression_id,
                        is_inclusive,
                    },
                    self.get_span_from(start),
                );
            }
            // member (also works across newline)
            else if let Ok(distance) = self.peek_member(TokenType::Identifier) {
                self.bump_by(distance - 1); // keep the identifier
                let path_id = self.eat_path()?;
                left_expression_id = self.tree.allocate(
                    Expression::Member {
                        receiver: left_expression_id,
                        path: path_id,
                    },
                    self.get_span_from(start),
                );
            }
            // index (implicit with `.0`)
            else if let Ok(distance) = self.peek_member(TokenType::Literal) {
                self.bump_by(distance - 2); // eat only newlines
                left_expression_id = self.eat_index_postfix_implicit(left_expression_id)?;
            }
            // index (explicit with `[]`)
            else if self.peek_token(TokenType::OpenBracket).is_ok() {
                left_expression_id = self.eat_index_postfix_explicit(left_expression_id)?;
            }
            // call
            else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                left_expression_id = self.eat_call_postfix(left_expression_id, runtime)?;
            }
            // unwrap
            else if self.peek_token(TokenType::Maybe).is_ok() {
                self.bump(); // eat ?
                left_expression_id = self.tree.allocate(
                    Expression::Maybe(left_expression_id),
                    self.get_span_from(start),
                );
            }
            // force unwrap
            else if self.peek_token(TokenType::Not).is_ok() {
                self.bump(); // eat !
                left_expression_id = self.tree.allocate(
                    Expression::Must(left_expression_id),
                    self.get_span_from(start),
                );
            }
            // tuple (if we have a comma / newline following an expression inside parentheses)
            else if self.options.in_parenthesis
                && (self.peek_token(TokenType::Comma).is_ok()
                    || self.peek_token(TokenType::Newline).is_ok())
            {
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;
                // we already have the first element (the expression itself)
                let first_element_id = self.tree.allocate(
                    Argument::Positional {
                        value: left_expression_id,
                    },
                    self.get_span_from(start),
                );
                // parse remaining elements
                let tuple_elements = self.eat_tuple_literal_body(Some(first_element_id))?;
                // build tuple literal
                left_expression_id = self.tree.allocate(
                    Expression::TupleLiteral {
                        elements: tuple_elements,
                    },
                    self.get_span_from(start),
                );
            }
            // done
            else {
                break;
            }
        }

        //
        // ------------------------------------------------------------
        // Infix operations (binary and assign, left associative)
        // ------------------------------------------------------------
        //

        // eat infix expressions while left precedence is weaker than right precedence
        loop {
            let (right_operator, operator_len) = {
                // infix operator on same line
                if let Ok((right_operator, operator_len)) = self.peek_infix_operator()
                    && (self.options.left_precedence.is_none()
                        || self.options.left_precedence.unwrap() < right_operator.precedence())
                {
                    (right_operator, operator_len)
                }
                // infix operator on next line
                else if self.peek_token(TokenType::Newline).is_ok()
                    && let Ok((right_operator, operator_len)) = self.peek_next_infix_operator()
                    && (self.options.left_precedence.is_none()
                        || self.options.left_precedence.unwrap() < right_operator.precedence())
                {
                    (right_operator, operator_len)
                }
                // no infix operator, break
                else {
                    break;
                }
            };
            if self.peek_token(TokenType::Newline).is_ok() {
                self.bump(); // eat newline
            }
            self.bump_by(operator_len); // eat infix operator
            self.eat_newline_maybe()?; // allow one newline

            // eat right expression
            let right_expression_id = self.with_options(
                self.options.in_left_precedence(right_operator.precedence()),
                |parser| parser.eat_expression(),
            )?;

            // combine into new left expression
            let left_expression =
                self.make_infix_expression(left_expression_id, right_operator, right_expression_id);
            left_expression_id = self
                .tree
                .allocate(left_expression, self.get_span_from(start))
        }

        Ok(left_expression_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::parse::tests::TestParser;
    use crate::{
        Argument, BinaryOperator, Expression, Mutability, Pattern, Runtime, ScalarLiteral,
        ScopedMutability, UnaryOperator, assert_expr_path, assert_node, assert_path, assert_string,
    };

    /// Parse an empty parenthesis as a tuple literal.
    #[test]
    fn test_parse_empty_parenthesis_tuple() {
        let mut test = TestParser::new("()");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::TupleLiteral { elements, .. } => {
            assert_eq!(elements.len(), 0);
        });
    }

    /// Parse a tuple literal with two elements.
    #[test]
    fn test_parse_tuple_literal() {
        let mut test = TestParser::new("(1, 2)");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(
            parser.tree,
            expr_id,
            Expression::TupleLiteral { elements, .. } => {
                assert_eq!(elements.len(), 2);
                // 1
                assert_node!(
                    parser.tree,
                    elements[0],
                    Argument::Positional { value } => {
                        assert_node!(
                            parser.tree,
                            *value,
                            Expression::ScalarLiteral(ScalarLiteral::Integer(1))
                        );
                    }
                );
                // 2
                assert_node!(
                    parser.tree,
                    elements[1],
                    Argument::Positional { value } => {
                        assert_node!(
                            parser.tree,
                            *value,
                            Expression::ScalarLiteral(ScalarLiteral::Integer(2))
                        );
                    }
                );
            }
        );
    }

    /// Parse a range literal.
    #[test]
    fn test_parse_range_literal() {
        let mut test = TestParser::new("1..3");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::RangeLiteral { start, end, .. } => {
            assert_node!(parser.tree, *start, Expression::ScalarLiteral(ScalarLiteral::Integer(val)) => {
                assert_eq!(*val, 1);
            });
            assert_node!(parser.tree, *end, Expression::ScalarLiteral(ScalarLiteral::Integer(val)) => {
                assert_eq!(*val, 3);
            });
        });
    }

    /// Parse a struct literal with a path type and two fields.
    #[test]
    fn test_parse_struct_literal_path() {
        let mut test = TestParser::new("geom.Vector2 { x: 1, y }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(
            parser.tree,
            expr_id,
            Expression::StructLiteral { ty, fields, .. } => {
                // geom.Vector2
                assert_node!(
                    parser.tree,
                    *ty,
                    Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "geom.Vector2");
                    }
                );
                assert_eq!(fields.len(), 2);
                // x: 1
                assert_node!(
                    parser.tree,
                    fields[0],
                    Argument::Named { name, value } => {
                        assert_string!(parser, *name, "x");
                        assert_node!(
                            parser.tree,
                            *value,
                            Expression::ScalarLiteral(ScalarLiteral::Integer(val)) => {
                                assert_eq!(*val, 1);
                            }
                        );
                    }
                );
                // y
                assert_node!(
                    parser.tree,
                    fields[1],
                    Argument::NamedShorthand { name } => {
                        assert_string!(parser, *name, "y");
                    }
                );
            }
        );
    }

    /// Parse a struct literal with static parameters and two fields.
    #[test]
    fn test_parse_struct_literal_path_with_static_parameters() {
        let mut test = TestParser::new(
            r##"
geom.Mesh<2, Dims: 4> { 
    vertices: [1, 2]
    y
}"##,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(
            parser.tree,
            expr_id,
            Expression::StructLiteral { ty, fields, .. } => {
                assert_node!(
                    parser.tree,
                    *ty,
                    Expression::Path { path, static_arguments } => {
                        assert_path!(parser, *path, "geom.Mesh");
                        assert!(static_arguments.is_some());
                        let params = static_arguments.as_ref().unwrap();
                        assert_eq!(params.len(), 2);
                    }
                );
                assert_eq!(fields.len(), 2);
                // vertices: [1, 2]
                assert_node!(
                    parser.tree,
                    fields[0],
                    Argument::Named { name, value } => {
                        assert_string!(parser, *name, "vertices");
                        assert_node!(
                            parser.tree,
                            *value,
                            Expression::ArrayLiteral { .. }
                        );
                    }
                );
                // y
                assert_node!(
                    parser.tree,
                    fields[1],
                    Argument::NamedShorthand { name } => {
                        assert_string!(parser, *name, "y");
                    }
                );
            }
        );
    }

    /// Parse a let binding with a type with static parameters as value.
    #[test]
    fn test_parse_type_with_static_parameters() {
        let mut test = TestParser::new("let Alias = A<B<C>>");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // let Alias = A<B<C>>
        assert_node!(parser.tree, expr_id, Expression::Let { pattern, value, .. } => {
            // Alias
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "Alias");
            });
            // A<B<C>>
            assert_node!(parser.tree, value.unwrap(), Expression::Path { path, static_arguments } => {
                assert_path!(parser, *path, "A");
                assert!(static_arguments.is_some());
                // B<C>
                assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { value } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                        assert_path!(parser, *path, "B");
                        assert!(static_arguments.is_some());
                        // C
                        assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { value } => {
                            assert_expr_path!(parser, parser.tree.get(*value), "C");
                        });
                    });
                });
            });
        });
    }

    /// Parse a dereference expression.
    #[test]
    fn test_parse_dereference_variable() {
        let mut test = TestParser::new("*x");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // *x
        assert_node!(parser.tree, expr_id, Expression::Unary { operator, right, .. } => {
            assert_eq!(*operator, UnaryOperator::Dereference);
            // x
            assert_expr_path!(parser, parser.tree.get(*right), "x");
        });
    }

    /// Parse a reference expression.
    #[test]
    fn test_parse_reference_variable() {
        let mut test = TestParser::new("&x");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // &x
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Reference { mutability, right, .. } => {
                assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
                // x
                assert_expr_path!(parser, parser.tree.get(*right), "x");
            }
        );
    }

    /// Parse a reference to a member call.
    #[test]
    fn test_parse_reference_member_call() {
        let mut test = TestParser::new("&var self.foo()");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // &var self.foo()
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Reference { mutability, right, .. } => {
                assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Mutable });
                // self.foo()
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Call { runtime, receiver, .. } => {
                        assert_eq!(*runtime, None);
                        // self.foo
                        assert_node!(
                            parser.tree,
                            *receiver,
                            Expression::Path { path, .. } => {
                                assert_path!(parser, *path, "self.foo");
                            }
                        );
                    }
                );
            }
        );
    }

    /// Parse a multi-line let with multi-line infix.
    #[test]
    fn test_parse_let_multiline_infix() {
        let mut test = TestParser::new(
            r"
let x = 
    foo.parse()
        + 2 
        + x
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expr_id = parser.eat_expression().unwrap();
        // let x = foo.parse() + 2 + x
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Let { mutability, pattern, value, .. } => {
                assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
                // x
                assert_node!(
                    parser.tree,
                    *pattern,
                    Pattern::Binding { name, .. } => {
                        assert_string!(parser, *name, "x");
                    }
                );
                // foo.parse() + 2 + x
                assert_node!(
                    parser.tree,
                    value.unwrap(),
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Add);
                        // foo.parse() + 2
                        assert_node!(
                            parser.tree,
                            *left,
                            Expression::Binary { left, operator, right, .. } => {
                                assert_eq!(*operator, BinaryOperator::Add);
                                // foo.parse()
                                assert_node!(
                                    parser.tree,
                                    *left,
                                    Expression::Call { runtime, receiver, .. } => {
                                        assert_eq!(*runtime, None);
                                        // foo.parse
                                        assert_node!(
                                            parser.tree,
                                            *receiver,
                                            Expression::Path { path, .. } => {
                                                assert_path!(parser, *path, "foo.parse");
                                            }
                                        );
                                    }
                                );
                                // 2
                                assert_node!(
                                    parser.tree,
                                    *right,
                                    Expression::ScalarLiteral(ScalarLiteral::Integer(2))
                                );
                            }
                        );
                        // x
                        assert_node!(
                            parser.tree,
                            *right,
                            Expression::Path { path, .. } => {
                                assert_path!(parser, *path, "x");
                            }
                        );
                    }
                );
            }
        );
    }

    /// Parse multi-line member and calls.
    #[test]
    fn test_parse_member_multiline() {
        let mut test = TestParser::new(
            r"
self
    .foo()
    .baz()
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expr_id = parser.eat_expression().unwrap();
        // self.foo().baz()
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Call { receiver: baz_recv, .. } => {
                // self.foo()
                assert_node!(
                    parser.tree,
                    *baz_recv,
                    Expression::Member { receiver, path, .. } => {
                        assert_path!(parser, *path, "baz");
                        assert_node!(parser.tree, *receiver, Expression::Call { receiver: foo_recv, .. } => {
                            // self.foo
                            assert_node!(
                                parser.tree,
                                *foo_recv,
                                Expression::Member { receiver: self_recv, path: foo_path, .. } => {
                                    // self
                                    assert_expr_path!(parser, parser.tree.get(*self_recv), "self");
                                    // foo
                                    assert_path!(parser, *foo_path, "foo");
                                }
                            );
                        })
                    }
                );
            }
        );
    }

    /// Parse a less-than comparison.
    #[test]
    fn test_parse_comparison_less_than() {
        let mut test = TestParser::new("x < y");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // x < y
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
                // x
                assert_expr_path!(parser, parser.tree.get(*left), "x");
                // y
                assert_expr_path!(parser, parser.tree.get(*right), "y");
            }
        );
    }

    /// Addition is left associative.
    #[test]
    fn test_parse_precedence_addition_left_associative() {
        let mut test = TestParser::new("a + b + c");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a + b + c
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::Add);
                // a + b
                assert_node!(
                    parser.tree,
                    *left,
                    Expression::Binary { left, operator, right, .. } => {
                        // a
                        assert_expr_path!(parser, parser.tree.get(*left), "a");
                        assert_eq!(*operator, BinaryOperator::Add);
                        // b
                        assert_expr_path!(parser, parser.tree.get(*right), "b");
                    }
                );
                // c
                assert_expr_path!(parser, parser.tree.get(*right), "c");
            }
        );
    }

    /// Infix operators work across lines.
    #[test]
    fn test_parse_precedence_addition_across_lines() {
        let mut test = TestParser::new("a +\n b +\n c");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a + b + c (across lines)
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::Add);
                // a + b
                assert_node!(
                    parser.tree,
                    *left,
                    Expression::Binary { left, operator, right, .. } => {
                        // a
                        assert_expr_path!(parser, parser.tree.get(*left), "a");
                        assert_eq!(*operator, BinaryOperator::Add);
                        // b
                        assert_expr_path!(parser, parser.tree.get(*right), "b");
                    }
                );
                // c
                assert_expr_path!(parser, parser.tree.get(*right), "c");
            }
        );
    }

    /// Multiplication has higher precedence than addition.
    #[test]
    fn test_parse_precedence_multiply_before_addition() {
        let mut test = TestParser::new("a + b * c");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a + b * c
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::Add);
                // a
                assert_expr_path!(parser, parser.tree.get(*left), "a");
                // b * c
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Multiply);
                        // b
                        assert_expr_path!(parser, parser.tree.get(*left), "b");
                        // c
                        assert_expr_path!(parser, parser.tree.get(*right), "c");
                    }
                );
            }
        );
    }

    /// Mixed precedence chain with addition and multiplication.
    #[test]
    fn test_parse_precedence_chain_mixed() {
        let mut test = TestParser::new("a + b * c + d");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a + b * c + d
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::Add);
                // a + b * c
                assert_node!(
                    parser.tree,
                    *left,
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Add);
                        // a
                        assert_expr_path!(parser, parser.tree.get(*left), "a");
                        // b * c
                        assert_node!(
                            parser.tree,
                            *right,
                            Expression::Binary { left, operator, right, .. } => {
                                assert_eq!(*operator, BinaryOperator::Multiply);
                                // b
                                assert_expr_path!(parser, parser.tree.get(*left), "b");
                                // c
                                assert_expr_path!(parser, parser.tree.get(*right), "c");
                            }
                        );
                    }
                );
                // d
                assert_expr_path!(parser, parser.tree.get(*right), "d");
            }
        );
    }

    /// Addition has higher precedence than elementwise or.
    #[test]
    fn test_parse_precedence_elementwise_vs_addition() {
        let mut test = TestParser::new("a + b | c + d");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a + b | c + d
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                // a + b
                assert_node!(
                    parser.tree,
                    *left,
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Add);
                        // a
                        assert_expr_path!(parser, parser.tree.get(*left), "a");
                        // b
                        assert_expr_path!(parser, parser.tree.get(*right), "b");
                    }
                );
                // c + d
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Add);
                        // c
                        assert_expr_path!(parser, parser.tree.get(*left), "c");
                        // d
                        assert_expr_path!(parser, parser.tree.get(*right), "d");
                    }
                );
            }
        );
    }

    /// Comparison has higher precedence than logical and.
    #[test]
    fn test_parse_precedence_comparison_vs_logical() {
        let mut test = TestParser::new("a == b && c == d");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a == b && c == d
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::And);
                // a == b
                assert_node!(
                    parser.tree,
                    *left,
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Equal);
                        // a
                        assert_expr_path!(parser, parser.tree.get(*left), "a");
                        // b
                        assert_expr_path!(parser, parser.tree.get(*right), "b");
                    }
                );
                // c == d
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Equal);
                        // c
                        assert_expr_path!(parser, parser.tree.get(*left), "c");
                        // d
                        assert_expr_path!(parser, parser.tree.get(*right), "d");
                    }
                );
            }
        );
    }

    /// Unary prefix has higher precedence than multiplication.
    #[test]
    fn test_parse_precedence_unary_before_multiply() {
        let mut test = TestParser::new("-a * b");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // -a * b
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::Multiply);
                // -a
                assert_node!(
                    parser.tree,
                    *left,
                    Expression::Unary { right, .. } => {
                        // a
                        assert_expr_path!(parser, parser.tree.get(*right), "a");
                    }
                );
                // b
                assert_expr_path!(parser, parser.tree.get(*right), "b");
            }
        );
    }

    /// Postfix call has higher precedence than addition.
    #[test]
    fn test_parse_precedence_postfix_call_before_add() {
        let mut test = TestParser::new("a() + @b() / c");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a() + @b() / c
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::Add);
                // a()
                assert_node!(
                    parser.tree,
                    *left,
                    Expression::Call { runtime, receiver, .. } => {
                        assert_eq!(*runtime, None);
                        // a
                        assert_expr_path!(parser, parser.tree.get(*receiver), "a");
                    }
                );
                // @b() / c
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Divide);
                        // @b()
                        assert_node!(
                            parser.tree,
                            *left,
                            Expression::Call { runtime, receiver, .. } => {
                                assert_eq!(*runtime, Some(Runtime::Static));
                                // b
                                assert_expr_path!(parser, parser.tree.get(*receiver), "b");
                            }
                        );
                        // c
                        assert_expr_path!(parser, parser.tree.get(*right), "c");
                    }
                );
            }
        );
    }

    /// Combine postfix member access and call with coalesce.
    #[test]
    fn test_parse_precedence_postfix_call_before_coalesce() {
        let mut test = TestParser::new("y.sqrt() ?? 0");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // y.sqrt() ?? 0
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::Coalesce);
                // y.sqrt()
                assert_node!(parser.tree, *left, Expression::Call { receiver, .. } => {
                    // y.sqrt
                    assert_expr_path!(parser, parser.tree.get(*receiver), "y.sqrt");
                });
                // 0
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::ScalarLiteral(ScalarLiteral::Integer(0))
                );
            }
        );
    }
}
