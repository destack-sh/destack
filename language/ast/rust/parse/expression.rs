//! Parse expressions. Mostly defers to other parsers.

use crate::parse::prelude::*;
use crate::{RangeLiteral, ScopedMutability, StructLiteral, TupleLiteral, TupleLiteralField};
use dyst_token::{TokenSpan, TokenType};

use crate::{
    AssignOperator, BinaryOperator, Call, Expression, InfixOperator, Keyword, Mutability, NodeId,
    NodeType, ParseError, ParseResult, Parser, ParserMark, Runtime, UnaryOperator, Visibility,
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
) -> ParseResult<(InfixOperator, u8)> {
    // special case for shift right to avoid ungluing ambiguity
    if !options.in_static
        && token.token.r#type == TokenType::GreaterThan
        && next_token.token.r#type == TokenType::GreaterThan
    {
        Ok((InfixOperator::Binary(BinaryOperator::ShiftRight), 2))
    }
    // regular binary operator
    // (only a subset of binary operators are allowed in static types)
    else if let Some(binary_operator) = BinaryOperator::from_token(token_str, token.token.r#type)
        && (!options.in_static || !NOT_IN_STATIC_BINARY_OPERATORS.contains(&binary_operator))
    {
        Ok((InfixOperator::Binary(binary_operator), 1))
    }
    // regular assign operator
    // (not allowed in static arguments)
    else if !options.in_static
        && !options.in_type
        && let Some(assign_operator) = AssignOperator::from_token(token.token.r#type)
    {
        Ok((InfixOperator::Assign(assign_operator), 1))
    }
    // unexpected
    else {
        Err(ParseError::unexpected(token.span))
    }
}

impl<'a> Parser<'a> {
    /// Peek a unary operator.
    #[inline]
    pub fn peek_unary_operator(&self) -> ParseResult<UnaryOperator> {
        let token = self.peek()?;
        UnaryOperator::from_token_type(token.token.r#type).ok_or(ParseError::unexpected(token.span))
    }

    /// Peek a next unary operator.
    #[inline]
    pub fn peek_next_unary_operator(&self) -> ParseResult<UnaryOperator> {
        let token = self.peek_next()?;
        UnaryOperator::from_token_type(token.token.r#type).ok_or(ParseError::unexpected(token.span))
    }

    /// Peek an infix operator.
    #[inline]
    pub fn peek_infix_operator(&self) -> ParseResult<(InfixOperator, u8)> {
        let token = self.peek()?;
        let token_str = self.get_span_str(token.span);
        let next_token = self.peek_next()?;
        to_infix_operator(token_str, token, next_token, self.options)
    }

    /// Peek a next infix operator.
    #[inline]
    pub fn peek_next_infix_operator(&self) -> ParseResult<(InfixOperator, u8)> {
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
    pub fn try_eat_expression_as_statement(&mut self) -> ParseResult<NodeId<Expression>> {
        self.with_options(self.options.in_statement(), |parser| {
            parser.try_eat_expression(TokenType::Newline)
        })
    }

    /// Try to eat an expression (return Expression::Error if error and recovery is possible).
    #[inline]
    pub fn try_eat_expression(&mut self, recover: TokenType) -> ParseResult<NodeId<Expression>> {
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
    fn peek_member(&self, token_type: TokenType) -> ParseResult<u8> {
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
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat an expression.
    pub fn eat_expression(&mut self) -> ParseResult<NodeId<Expression>> {
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
            if token.token.r#type == TokenType::OpenParenthesis {
                self.bump(); // eat open paranthesis
                self.eat_newlines_maybe()?;

                // if we immediately see a closing parenthesis, it's an empty tuple
                if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                    self.bump(); // eat closing parenthesis
                    let tuple_literal_id = self
                        .tree
                        .allocate(TupleLiteral { elements: vec![] }, self.get_span_from(start));
                    self.tree.allocate(
                        Expression::TupleLiteral(tuple_literal_id),
                        self.get_span_from(start),
                    )
                }
                // named tuple element, must be some tuple
                else if self.peek_token(TokenType::Identifier).is_ok()
                    && self.peek_next_token(TokenType::Colon).is_ok()
                {
                    let tuple_elements = self
                        .eat_tuple_literal_body(None)
                        .for_node_type(NodeType::TupleLiteral)?;
                    self.eat_token(TokenType::CloseParenthesis)?;
                    let tuple_literal_id = self.tree.allocate(
                        TupleLiteral {
                            elements: tuple_elements,
                        },
                        self.get_span_from(start),
                    );
                    self.tree.allocate(
                        Expression::TupleLiteral(tuple_literal_id),
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
                        Expression::TupleLiteral(_)
                            if self.tokens[inner_start as usize].token.r#type
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
                let module_id = self
                    .eat_module(visibility)
                    .for_node_type(NodeType::Module)?;
                let expression = Expression::Module(module_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // struct
            else if keyword == Some(Keyword::Struct) {
                let struct_id = self
                    .eat_struct(visibility)
                    .for_node_type(NodeType::Struct)?;
                let expression = Expression::Struct(struct_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // enum
            else if keyword == Some(Keyword::Enum) {
                let enum_id = self.eat_enum(visibility).for_node_type(NodeType::Enum)?;
                let expression = Expression::Enum(enum_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // union
            else if keyword == Some(Keyword::Union) {
                let union_id = self.eat_union(visibility).for_node_type(NodeType::Union)?;
                let expression = Expression::Union(union_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // trait
            else if keyword == Some(Keyword::Trait) {
                let trait_id = self.eat_trait(visibility).for_node_type(NodeType::Trait)?;
                let expression = Expression::Trait(trait_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // implement
            else if keyword == Some(Keyword::Implement) {
                let implement_id = self.eat_implement().for_node_type(NodeType::Implement)?;
                let expression = Expression::Implement(implement_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // function
            else if keyword == Some(Keyword::Function) {
                let function_id = self
                    .eat_function(visibility)
                    .for_node_type(NodeType::Function)?;
                let expression = Expression::Function(function_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // block
            else if self.peek_block().is_ok() {
                let block_id = self.eat_block().for_node_type(NodeType::Block)?;
                let expression = Expression::Block(block_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Control flow
            // ------------------------------------------------------------
            //
            // with
            else if keyword == Some(Keyword::With) {
                let with_id = self.eat_with().for_node_type(NodeType::With)?;
                let expression = Expression::With(with_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // use
            else if keyword == Some(Keyword::Use) {
                let use_id = self.eat_use(visibility).for_node_type(NodeType::Use)?;
                let expression = Expression::Use(use_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // let
            else if keyword == Some(Keyword::Let)
                || keyword == Some(Keyword::Var)
                || keyword == Some(Keyword::Const)
            {
                let let_id = self.eat_let(visibility).for_node_type(NodeType::Let)?;
                let expression = Expression::Let(let_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // if
            else if keyword == Some(Keyword::If) {
                let if_id = self.eat_if(runtime).for_node_type(NodeType::If)?;
                let expression = Expression::If(if_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // while
            else if keyword == Some(Keyword::While) {
                let while_id = self.eat_while(runtime).for_node_type(NodeType::While)?;
                let expression = Expression::While(while_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // for
            else if keyword == Some(Keyword::For) {
                let for_id = self.eat_for(runtime).for_node_type(NodeType::For)?;
                let expression = Expression::For(for_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // loop
            else if keyword == Some(Keyword::Loop) {
                let loop_id = self.eat_loop(runtime).for_node_type(NodeType::Loop)?;
                let expression = Expression::Loop(loop_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // try
            else if keyword == Some(Keyword::Try) {
                let try_id = self.eat_try_catch().for_node_type(NodeType::Try)?;
                let expression = Expression::Try(try_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // match
            else if keyword == Some(Keyword::Match) {
                let match_id = self.eat_match().for_node_type(NodeType::Match)?;
                let expression = Expression::Match(match_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // break
            else if keyword == Some(Keyword::Break) {
                let break_id = self.eat_break().for_node_type(NodeType::Break)?;
                let expression = Expression::Break(break_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // continue
            else if keyword == Some(Keyword::Continue) {
                let continue_id = self.eat_continue().for_node_type(NodeType::Continue)?;
                let expression = Expression::Continue(continue_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // defer
            else if keyword == Some(Keyword::Defer) {
                let defer_id = self.eat_defer().for_node_type(NodeType::Defer)?;
                let expression = Expression::Defer(defer_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // return
            else if keyword == Some(Keyword::Return) {
                let return_id = self.eat_return().for_node_type(NodeType::Return)?;
                let expression = Expression::Return(return_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Bindings / Literals / Aliases
            // ------------------------------------------------------------
            //
            // let
            else if keyword == Some(Keyword::Let)
                || keyword == Some(Keyword::Var)
                || keyword == Some(Keyword::Const)
            {
                let let_id = self.eat_let(visibility).for_node_type(NodeType::Let)?;
                let expression = Expression::Let(let_id);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // array
            else if token.token.r#type == TokenType::OpenBracket {
                let array_literal = self
                    .eat_array_literal()
                    .for_node_type(NodeType::ArrayLiteral)?;
                let expression = Expression::ArrayLiteral(array_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // scalar
            else if self.peek_scalar_literal().is_ok() {
                let scalar_literal = self
                    .eat_scalar_literal()
                    .for_node_type(NodeType::ScalarLiteral)?;
                let expression = Expression::ScalarLiteral(scalar_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // type
            else if self.peek_type_literal().is_ok() {
                let type_literal = self
                    .eat_type_literal()
                    .for_node_type(NodeType::TypeLiteral)?;
                let expression = Expression::TypeLiteral(type_literal);
                self.tree.allocate(expression, self.get_span_from(start))
            }
            // alias / path
            else if token.token.r#type == TokenType::Identifier {
                let path_id = self.eat_path().for_node_type(NodeType::Expression)?;

                // speculatively unwrap postfix static parameterisation with `<`
                //  (might also be just a comparison operator)
                let speculative_start = self.mark();
                let speculative_start_idx = self.tree.next_id;
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
                return Err(ParseError::unexpected(self.peek()?.span));
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
            let call_id = self.tree.allocate(
                Call {
                    runtime,
                    receiver: left_expression_id,
                    dynamic_arguments: vec![],
                },
                self.get_span_from(start),
            );
            left_expression_id = self
                .tree
                .allocate(Expression::Call(call_id), self.get_span_from(start));
            runtime = None;
        }
        // struct literal postfix with `{`
        else if let Expression::Path { .. } = self.tree.get(left_expression_id)
            && self.peek_token(TokenType::OpenBrace).is_ok()
            && !self.options.in_before_block
        {
            let fields = self
                .eat_struct_literal_body()
                .for_node_type(NodeType::StructLiteral)?;
            let struct_literal_id = self.tree.allocate(
                StructLiteral {
                    r#type: left_expression_id,
                    fields,
                },
                self.get_span_from(start),
            );
            left_expression_id = self.tree.allocate(
                Expression::StructLiteral(struct_literal_id),
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
                let right_expression_id = self
                    .eat_expression()
                    .for_node_type(NodeType::RangeLiteral)?;
                let literal_id = self.tree.allocate(
                    RangeLiteral {
                        start: left_expression_id,
                        end: right_expression_id,
                        is_inclusive,
                    },
                    self.get_span_from(start),
                );
                let expression = Expression::RangeLiteral(literal_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // member (also works across newline)
            else if let Ok(distance) = self.peek_member(TokenType::Identifier) {
                self.bump_by(distance - 1); // keep the identifier
                let path_id = self.eat_path().for_node_type(NodeType::Expression)?;
                let expression = Expression::Member {
                    receiver: left_expression_id,
                    path: path_id,
                };
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // index (implicit with `.0`)
            else if let Ok(distance) = self.peek_member(TokenType::Literal) {
                self.bump_by(distance - 2); // eat only newlines
                let index_id = self
                    .eat_index_postfix_implicit(left_expression_id)
                    .for_node_type(NodeType::Index)?;
                let expression = Expression::Index(index_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // index (explicit with `[]`)
            else if self.peek_token(TokenType::OpenBracket).is_ok() {
                let index_id = self
                    .eat_index_postfix_explicit(left_expression_id)
                    .for_node_type(NodeType::Index)?;
                let expression = Expression::Index(index_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // call
            else if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                let call_id = self
                    .eat_call_postfix(left_expression_id, runtime)
                    .for_node_type(NodeType::Call)?;
                let expression = Expression::Call(call_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // unwrap
            else if self.peek_token(TokenType::Maybe).is_ok() {
                self.bump(); // eat ?
                let expression = Expression::Maybe(left_expression_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
            }
            // force unwrap
            else if self.peek_token(TokenType::Not).is_ok() {
                self.bump(); // eat !
                let expression = Expression::Must(left_expression_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
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
                    TupleLiteralField::Positional {
                        value: left_expression_id,
                    },
                    self.get_span_from(start),
                );
                // parse remaining elements
                let tuple_elements = self
                    .eat_tuple_literal_body(Some(first_element_id))
                    .for_node_type(NodeType::TupleLiteral)?;
                // build tuple literal
                let tuple_literal_id = self.tree.allocate(
                    TupleLiteral {
                        elements: tuple_elements,
                    },
                    self.get_span_from(start),
                );
                let expression = Expression::TupleLiteral(tuple_literal_id);
                left_expression_id = self.tree.allocate(expression, self.get_span_from(start));
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
        Argument, BinaryOperator, Call, Expression, FieldLiteral, Let, Mutability, Pattern,
        RangeLiteral, Runtime, ScalarLiteral, ScopedMutability, StructLiteral, TupleLiteral,
        TupleLiteralField, UnaryOperator, assert_expr_path, assert_int, assert_lit_int,
        assert_node, assert_path, assert_string,
    };

    /// Empty parenthesis are tuples.
    #[test]
    fn test_parse_empty_parenthesis_tuple() {
        let mut test = TestParser::new("()");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::TupleLiteral(tuple_literal_id) => {
            assert_node!(parser.tree, *tuple_literal_id, TupleLiteral { elements } => {
                assert_eq!(elements.len(), 0);
            });
        });
    }

    /// Tuple literals are disambiguated.
    /// (1, 2)
    #[test]
    fn test_parse_tuple_literal() {
        let mut test = TestParser::new("(1, 2)");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // (1, 2)
        assert_node!(
            parser.tree,
            expr_id,
            Expression::TupleLiteral(tuple_literal_id) => {
                assert_node!(
                    parser.tree,
                    *tuple_literal_id,
                    TupleLiteral { elements } => {
                        assert_eq!(elements.len(), 2);
                        // 1
                        assert_node!(
                            parser.tree,
                            elements[0],
                            TupleLiteralField::Positional { value } => {
                                assert_node!(
                                    parser.tree,
                                    *value,
                                    Expression::ScalarLiteral(scalar_literal_id) => {
                                        assert_int!(parser.tree, *scalar_literal_id, 1);
                                    }
                                );
                            }
                        );
                        // 2
                        assert_node!(
                            parser.tree,
                            elements[1],
                            TupleLiteralField::Positional { value } => {
                                assert_node!(
                                    parser.tree,
                                    *value,
                                    Expression::ScalarLiteral(scalar_literal_id) => {
                                        assert_int!(parser.tree, *scalar_literal_id, 2);
                                    }
                                );
                            }
                        );
                    }
                );
            }
        );
    }

    /// Range literals are disambiguated.
    /// 1..3
    #[test]
    fn test_parse_range_literal() {
        let mut test = TestParser::new("1..3");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // 1..3
        assert_node!(parser.tree, expr_id, Expression::RangeLiteral(range_literal_id) => {
            assert_node!(parser.tree, *range_literal_id, RangeLiteral { start, end, .. } => {
                assert_node!(parser.tree, *start, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_lit_int!(parser.session, parser.tree.get(*scalar_literal_id), 1);
                });
                assert_node!(parser.tree, *end, Expression::ScalarLiteral(scalar_literal_id) => {
                    assert_lit_int!(parser.session, parser.tree.get(*scalar_literal_id), 3);
                });
            });
        });
    }

    /// Struct literals are disambiguated.
    /// geom.Vector2 { x: 1, y }
    #[test]
    fn test_parse_struct_literal_path() {
        let mut test = TestParser::new("geom.Vector2 { x: 1, y }");
        let mut parser = test.prepare();

        let expr_id = parser.eat_expression().unwrap();

        // geom.Vector2 { x: 1, y }
        assert_node!(
            parser.tree,
            expr_id,
            Expression::StructLiteral(struct_literal_id) => {
                assert_node!(
                    parser.tree,
                    *struct_literal_id,
                    StructLiteral { r#type, fields } => {
                        // geom.Vector2
                        assert_node!(
                            parser.tree,
                            *r#type,
                            Expression::Path { path, static_arguments: _ } => {
                                assert_path!(parser.session, *path, "geom.Vector2");
                            }
                        );
                        // fields
                        assert_eq!(fields.len(), 2);
                        // x: 1
                        assert_node!(
                            parser.tree,
                            fields[0],
                            FieldLiteral::Named { name, value } => {
                                // x
                                assert_string!(parser.session, *name, "x");
                                // 1
                                assert_node!(
                                    parser.tree,
                                    *value,
                                    Expression::ScalarLiteral(scalar_id) => {
                                        assert_node!(
                                            parser.tree,
                                            *scalar_id,
                                            ScalarLiteral::Integer(1)
                                        );
                                    }
                                );
                            }
                        );
                        // y
                        assert_node!(
                            parser.tree,
                            fields[1],
                            FieldLiteral::NamedShorthand { name } => {
                                // y
                                assert_string!(parser.session, *name, "y");
                            }
                        );
                    }
                );
            }
        );
    }

    /// Struct literals with static parameters are disambiguated.
    /// geom.Mesh<2, Dims: 4> {
    ///     vertices: [1, 2]
    ///     y  
    /// }
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
            Expression::StructLiteral(struct_literal_id) => {
                let struct_literal = parser.tree.get(*struct_literal_id);
                // geom.Mesh<2, Dims: 4>
                assert_node!(
                    parser.tree,
                    struct_literal.r#type,
                    Expression::Path { path, static_arguments } => {
                        // geom.Mesh
                        assert_path!(parser.session, *path, "geom.Mesh");
                        // <2, Dims: 4>
                        assert!(static_arguments.is_some());
                        let params = static_arguments.as_ref().unwrap();
                        assert_eq!(params.len(), 2);
                    }
                );
                let fields = &struct_literal.fields;
                assert_eq!(fields.len(), 2);
                // vertices: [1, 2]
                assert_node!(
                    parser.tree,
                    fields[0],
                    FieldLiteral::Named { name, value } => {
                        assert_string!(parser.session, *name, "vertices");
                        assert_node!(
                            parser.tree,
                            *value,
                            Expression::ArrayLiteral(_)
                        );
                    }
                );
                // y
                assert_node!(
                    parser.tree,
                    fields[1],
                    FieldLiteral::NamedShorthand { name } => {
                        assert_string!(parser.session, *name, "y");
                    }
                );
            }
        );
    }

    /// Types with static parameters are disambiguated as values.
    /// let Alias = A<B<C>>
    #[test]
    fn test_parse_type_with_static_parameters() {
        let mut test = TestParser::new("let Alias = A<B<C>>");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // let Alias = A<B<C>>
        assert_node!(parser.tree, expr_id, Expression::Let(let_id) => {
            assert_node!(parser.tree, *let_id, Let { mutability: _, pattern, r#type: _, value, visibility: _, .. } => {
                // let Alias
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser.session, *name, "Alias");
                });
                // A<B<C>>
                assert_node!(parser.tree, value.unwrap(), Expression::Path { path, static_arguments } => {
                    assert_path!(parser.session, *path, "A");
                    assert!(static_arguments.is_some());
                    assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { value } => {
                        // B<C>
                        assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                            assert_path!(parser.session, *path, "B");
                            assert!(static_arguments.is_some());
                            // C
                            assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { value } => {
                                assert_expr_path!(parser.session, parser.tree.get(*value), "C");
                            });
                        });
                    });
                });
            });
        });
    }

    /// Dereference variable.
    /// *x
    #[test]
    fn test_parse_dereference_variable() {
        let mut test = TestParser::new("*x");
        let mut parser = test.prepare();

        let expr_id = parser.eat_expression().unwrap();

        // *x
        assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
            assert_eq!(*operator, UnaryOperator::Dereference);
            assert_expr_path!(parser.session, parser.tree.get(*right), "x");
        });
    }

    /// Reference operator on variable.
    /// &x
    #[test]
    fn test_parse_reference_variable() {
        let mut test = TestParser::new("&x");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // &x
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Reference { mutability, right } => {
                // &
                assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
                // x
                assert_expr_path!(parser.session, parser.tree.get(*right), "x");
            }
        );
    }

    /// Reference operator on member access with method call.
    /// &var self.foo()
    #[test]
    fn test_parse_reference_member_call() {
        let mut test = TestParser::new("&var self.foo()");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // &var self.foo()
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Reference { mutability, right } => {
                // &var
                assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Mutable });
                // self.foo()
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Call(call_id) => {
                        assert_node!(
                            parser.tree,
                            *call_id,
                            Call { runtime, receiver, dynamic_arguments: _ } => {
                                assert_eq!(*runtime, None);
                                // self.foo
                                assert_node!(
                                    parser.tree,
                                    *receiver,
                                    Expression::Path { path, static_arguments: _ } => {
                                        assert_path!(parser.session, *path, "self.foo");
                                    }
                                );
                            }
                        );
                    }
                );
            }
        );
    }

    /// Test parse mult-line let with multi-linx infix.
    /// let x =
    ///     foo.parse()
    ///         + 2
    ///         + x
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
            Expression::Let(let_id) => {
                assert_node!(
                    parser.tree,
                    *let_id,
                    Let { mutability, pattern, r#type: _, value, visibility: _, .. } => {
                        assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
                        // x
                        assert_node!(
                            parser.tree,
                            *pattern,
                            Pattern::Binding { name, .. } => {
                                assert_eq!(parser.session.get_string(*name), "x");
                            }
                        );

                        // foo.parse() + 2 + x
                        assert_node!(
                            parser.tree,
                            value.unwrap(),
                            Expression::Binary { left, operator, right } => {
                                assert_eq!(*operator, BinaryOperator::Add);
                                // foo.parse() + 2
                                assert_node!(
                                    parser.tree,
                                    *left,
                                    Expression::Binary { left, operator, right } => {
                                        assert_eq!(*operator, BinaryOperator::Add);
                                        // foo.parse()
                                        assert_node!(
                                            parser.tree,
                                            *left,
                                            Expression::Call(call_id) => {
                                                assert_node!(
                                                    parser.tree,
                                                    *call_id,
                                                    Call { runtime, receiver, dynamic_arguments: _ } => {
                                                        assert_eq!(*runtime, None);
                                                        // foo.parse
                                                        assert_node!(
                                                            parser.tree,
                                                            *receiver,
                                                            Expression::Path { path, static_arguments: _ } => {
                                                                assert_path!(parser.session, *path, "foo.parse");
                                                            }
                                                        );
                                                    }
                                                );
                                            }
                                        );
                                        // 2
                                        assert_node!(
                                            parser.tree,
                                            *right,
                                            Expression::ScalarLiteral(scalar_id) => {
                                                assert_node!(
                                                    parser.tree,
                                                    *scalar_id,
                                                    ScalarLiteral::Integer(value) => {
                                                        assert_eq!(*value, 2);
                                                    }
                                                );
                                            }
                                        );
                                    }
                                );
                                // x
                                assert_node!(
                                    parser.tree,
                                    *right,
                                    Expression::Path { path, static_arguments: _ } => {
                                        assert_path!(parser.session, *path, "x");
                                    }
                                );
                            }
                        );
                    }
                );
            }
        );
    }

    /// Reference operator on member access with method call.
    /// self
    ///    .foo()
    ///    .baz()
    #[test]
    fn test_parse_member_access_multiline() {
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
            Expression::Call(call_id) => {
                assert_node!(
                    parser.tree,
                    *call_id,
                    Call { runtime, receiver, dynamic_arguments: _ } => {
                        assert_eq!(*runtime, None);
                        // self.foo().baz
                        assert_node!(
                            parser.tree,
                            *receiver,
                            Expression::Member { receiver, path } => {
                                // self.foo()
                                assert_node!(
                                    parser.tree,
                                    *receiver,
                                    Expression::Call(call_id) => {
                                        assert_node!(
                                            parser.tree,
                                            *call_id,
                                            Call { runtime, receiver, dynamic_arguments: _ } => {
                                                assert_eq!(*runtime, None);
                                                // self.foo
                                                assert_node!(
                                                    parser.tree,
                                                    *receiver,
                                                    Expression::Member { receiver, path } => {
                                                        // self
                                                        assert_expr_path!(parser.session, parser.tree.get(*receiver), "self");
                                                        // foo
                                                        assert_path!(parser.session, *path, "foo");
                                                    }
                                                );
                                            }
                                        );
                                    }
                                );
                                // baz
                                assert_path!(parser.session, *path, "baz");
                            }
                        );
                    }
                );
            }
        );
    }

    /// Comparison operator should be disambiguated from static parameterisation.
    /// x < y
    #[test]
    fn test_parse_comparison_less_than() {
        let mut test = TestParser::new("x < y");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // x < y
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::LessThan);
                // x
                assert_expr_path!(parser.session, parser.tree.get(*left), "x");
                // y
                assert_expr_path!(parser.session, parser.tree.get(*right), "y");
            }
        );
    }

    /// Addition is left associative.
    /// a + b + c
    /// => ((a + b) + c)
    #[test]
    fn test_parse_precedence_addition_left_associative() {
        let mut test = TestParser::new("a + b + c");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + b) + c)
            Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::Add);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + b)
                    Expression::Binary { left, operator, right } => {
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // b
                        assert_expr_path!(parser.session, parser.tree.get(*right), "b");
                    }
                );
                // c
                assert_expr_path!(parser.session, parser.tree.get(*right), "c");
            }
        );
    }

    /// Infix operators work across lines.
    /// a +
    /// b +
    /// c
    /// => ((a + b) + c)
    #[test]
    fn test_parse_precedence_addition_across_lines() {
        let mut test = TestParser::new("a +\n b +\n c");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + b) + c)
            Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::Add);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + b)
                    Expression::Binary { left, operator, right } => {
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // b
                        assert_expr_path!(parser.session, parser.tree.get(*right), "b");
                    }
                );
                // c
                assert_expr_path!(parser.session, parser.tree.get(*right), "c");
            }
        );
    }

    /// Multiplication has higher precedence than addition.
    /// a + b * c
    /// => (a + (b * c))
    #[test]
    fn test_parse_precedence_multiply_before_addition() {
        let mut test = TestParser::new("a + b * c");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // (a + (b * c))
            Expression::Binary { left, operator, right } => {
                // +
                assert_eq!(*operator, BinaryOperator::Add);
                // a
                assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                assert_node!(
                    parser.tree,
                    *right,
                    // (b * c)
                    Expression::Binary { left, operator, right } => {
                        // *
                        assert_eq!(*operator, BinaryOperator::Multiply);
                        // b
                        assert_expr_path!(parser.session, parser.tree.get(*left), "b");
                        // c
                        assert_expr_path!(parser.session, parser.tree.get(*right), "c");
                    }
                );
            }
        );
    }

    /// Mixed precedence chain with addition and multiplication.
    /// a + b * c + d
    /// => ((a + (b * c)) + d)
    #[test]
    fn test_parse_precedence_chain_mixed() {
        let mut test = TestParser::new("a + b * c + d");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + (b * c)) + d)
            Expression::Binary { left, operator, right } => {
                // +
                assert_eq!(*operator, BinaryOperator::Add);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + (b * c))
                    Expression::Binary { left, operator, right } => {
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                        assert_node!(
                            parser.tree,
                            *right,
                            // (b * c)
                            Expression::Binary { left, operator, right } => {
                                // *
                                assert_eq!(*operator, BinaryOperator::Multiply);
                                // b
                                assert_expr_path!(parser.session, parser.tree.get(*left), "b");
                                // c
                                assert_expr_path!(parser.session, parser.tree.get(*right), "c");
                            }
                        );
                    }
                );
                // d
                assert_expr_path!(parser.session, parser.tree.get(*right), "d");
            }
        );
    }

    /// Addition has higher precedence than elementwise or.
    /// a + b | c + d
    /// => ((a + b) | (c + d))
    #[test]
    fn test_parse_precedence_elementwise_vs_addition() {
        let mut test = TestParser::new("a + b | c + d");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a + b) | (c + d))
            Expression::Binary { left, operator, right } => {
                // |
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a + b)
                    Expression::Binary { left, operator, right } => {
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                        // b
                        assert_expr_path!(parser.session, parser.tree.get(*right), "b");
                    }
                );
                assert_node!(
                    parser.tree,
                    *right,
                    // (c + d)
                    Expression::Binary { left, operator, right } => {
                        // +
                        assert_eq!(*operator, BinaryOperator::Add);
                        // c
                        assert_expr_path!(parser.session, parser.tree.get(*left), "c");
                        // d
                        assert_expr_path!(parser.session, parser.tree.get(*right), "d");
                    }
                );
            }
        );
    }

    /// Comparison has higher precedence than logical and.
    /// a == b && c == d
    /// => ((a == b) && (c == d))
    #[test]
    fn test_parse_precedence_comparison_vs_logical() {
        let mut test = TestParser::new("a == b && c == d");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a == b) && (c == d))
            Expression::Binary { left, operator, right } => {
                // &&
                assert_eq!(*operator, BinaryOperator::And);
                assert_node!(
                    parser.tree,
                    *left,
                    // (a == b)
                    Expression::Binary { left, operator, right } => {
                        // ==
                        assert_eq!(*operator, BinaryOperator::Equal);
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*left), "a");
                        // b
                        assert_expr_path!(parser.session, parser.tree.get(*right), "b");
                    }
                );
                assert_node!(
                    parser.tree,
                    *right,
                    // (c == d)
                    Expression::Binary { left, operator, right } => {
                        // ==
                        assert_eq!(*operator, BinaryOperator::Equal);
                        // c
                        assert_expr_path!(parser.session, parser.tree.get(*left), "c");
                        // d
                        assert_expr_path!(parser.session, parser.tree.get(*right), "d");
                    }
                );
            }
        );
    }

    /// Unary prefix has higher precedence than multiplication.
    /// -a * b
    /// => ((-a) * b)
    #[test]
    fn test_parse_precedence_unary_before_multiply() {
        let mut test = TestParser::new("-a * b");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((-a) * b)
            Expression::Binary { left, operator, right } => {
                // *
                assert_eq!(*operator, BinaryOperator::Multiply);
                assert_node!(
                    parser.tree,
                    *left,
                    // (-a)
                    Expression::Unary { operator: _, right } => {
                        // a
                        assert_expr_path!(parser.session, parser.tree.get(*right), "a");
                    }
                );
                // b
                assert_expr_path!(parser.session, parser.tree.get(*right), "b");
            }
        );
    }

    /// Postfix call has higher precedence than addition.
    /// Static calls are right associative.
    /// a() + @b() / c
    /// => ((a()) + ((@b()) / c))
    #[test]
    fn test_parse_precedence_postfix_call_before_add() {
        let mut test = TestParser::new("a() + @b() / c");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((a()) + ((@b()) / b))
            Expression::Binary { left, operator, right } => {
                // +
                assert_eq!(*operator, BinaryOperator::Add);
                // (a())
                assert_node!(
                    parser.tree,
                    *left,
                    // a()
                    Expression::Call(call_id) => {
                        assert_node!(
                            parser.tree,
                            *call_id,
                            Call { runtime, receiver, dynamic_arguments: _ } => {
                                assert_eq!(*runtime, None);
                                // a
                                assert_expr_path!(parser.session, parser.tree.get(*receiver), "a");
                            }
                        );
                    }
                );
                // ((@b()) / c)
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Binary { left, operator, right } => {
                        // /
                        assert_eq!(*operator, BinaryOperator::Divide);
                        // (@b())
                        assert_node!(
                            parser.tree,
                            *left,
                            Expression::Call(call_id) => {
                                assert_node!(
                                    parser.tree,
                                    *call_id,
                                    Call { runtime, receiver, dynamic_arguments: _ } => {
                                        assert_eq!(*runtime, Some(Runtime::Static));
                                        // b
                                        assert_expr_path!(parser.session, parser.tree.get(*receiver), "b");
                                    }
                                );
                            }
                        );
                        // c
                        assert_expr_path!(parser.session, parser.tree.get(*right), "c");
                    }
                );
            }
        );
    }

    /// Combine postfix member access and call with coalesce.
    /// y.sqrt() ?? 0
    /// => ((   y.sqrt()) ?? 0)
    #[test]
    fn test_parse_precedence_postfix_call_before_coalesce() {
        let mut test = TestParser::new("y.sqrt() ?? 0");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            // ((y.sqrt()) ?? 0)
            Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::Coalesce);

                // (y.sqrt())
                assert_node!(parser.tree, *left, Expression::Call(call_id) => {
                    assert_node!(parser.tree, *call_id, Call { receiver, .. } => {
                        // y.sqrt
                        assert_expr_path!(parser.session, parser.tree.get(*receiver), "y.sqrt");
                    });
                });

                // 0
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::ScalarLiteral(scalar_id) => {
                        assert_node!(
                            parser.tree,
                            *scalar_id,
                            ScalarLiteral::Integer(0)
                        );
                    }
                );
            }
        );
    }
}
