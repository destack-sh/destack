//! Parse expressions. Mostly defers to other parsers.

use dyst_ast::{ExportMode, IfStyle};

use crate::parse::prelude::*;
use crate::{
    Argument, AssignOperator, BinaryOperator, Expression, InfixOperator, Keyword, NodeId, NodeType,
    Parser, ParserError, ParserMark, ParserResult, Runtime, TokenSpan, TokenType, UnaryOperator,
    Visibility,
};

static DEFINITION_KEYWORDS: [Keyword; 15] = [
    Keyword::Namespace,
    Keyword::Module,
    Keyword::Struct,
    Keyword::Class,
    Keyword::Enum,
    Keyword::Union,
    Keyword::Function,
    Keyword::Interface,
    Keyword::Trait,
    Keyword::Type,
    Keyword::Const,
    Keyword::Readonly,
    Keyword::Let,
    Keyword::Var,
    Keyword::Implement,
];

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

// can't use anything with `<` or `>` in tree fragments
static NOT_IN_TREE_BINARY_OPERATORS: [BinaryOperator; 8] = [
    // shift
    BinaryOperator::ShiftLeft,
    BinaryOperator::SaturatingShiftLeft,
    BinaryOperator::ShiftRight,
    // comparison
    BinaryOperator::LessThan,
    BinaryOperator::LessThanOrEqual,
    BinaryOperator::GreaterThan,
    BinaryOperator::GreaterThanOrEqual,
    // multiply
    BinaryOperator::Divide,
];

/// Make an infix operator.
#[inline]
fn to_infix_operator(
    token_str: &str,
    token: &TokenSpan,
    next_token: &TokenSpan,
    options: ParserOptions,
) -> ParserResult<(InfixOperator, u8)> {
    // special case for shift right to avoid ungluing ambiguity
    if !options.in_static
        && token.token.ty == TokenType::GreaterThan
        && next_token.token.ty == TokenType::GreaterThan
    {
        Ok((InfixOperator::Binary(BinaryOperator::ShiftRight), 2))
    }
    // regular binary operator
    // (only a subset of binary operators are allowed in static and tree contexts)
    else if let Some(binary_operator) = BinaryOperator::from_token(token_str, token.token.ty)
        && (!options.in_static || !NOT_IN_STATIC_BINARY_OPERATORS.contains(&binary_operator))
        && (!options.in_tree_literal || !NOT_IN_TREE_BINARY_OPERATORS.contains(&binary_operator))
    {
        Ok((InfixOperator::Binary(binary_operator), 1))
    }
    // regular assign operator
    // (not allowed in static, type, and tree contexts)
    else if !options.in_static
        && !options.in_type
        && !options.in_tree_literal
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
    /// Peek a unary prefix operator.
    #[inline]
    pub fn peek_unary_prefix_operator(&self) -> ParserResult<UnaryOperator> {
        let token = self.peek()?;
        UnaryOperator::from_prefix_token(token.token.ty).ok_or(ParserError::unexpected(token.span))
    }

    /// Peek a next unary prefix operator.
    #[inline]
    pub fn peek_next_unary_prefix_operator(&self) -> ParserResult<UnaryOperator> {
        let token = self.peek_next()?;
        UnaryOperator::from_prefix_token(token.token.ty).ok_or(ParserError::unexpected(token.span))
    }

    /// Peek a unary postfix operator.
    #[inline]
    pub fn peek_unary_postfix_operator(&self) -> ParserResult<UnaryOperator> {
        let token = self.peek()?;
        UnaryOperator::from_postfix_token(token.token.ty).ok_or(ParserError::unexpected(token.span))
    }

    /// Peek an assign operator.
    #[inline]
    pub fn peek_assign_operator(&self) -> ParserResult<AssignOperator> {
        let token = self.peek()?;
        AssignOperator::from_token(token.token.ty).ok_or(ParserError::unexpected(token.span))
    }

    /// Peek an infix operator.
    #[inline]
    pub fn peek_infix_operator(&self) -> ParserResult<(InfixOperator, u8)> {
        let token = self.peek()?;
        let token_str = self.get_span_str(token.span);
        let next_token = self.peek_next()?;
        to_infix_operator(token_str, token, next_token, self.options)
    }

    /// Peek a next infix operator.
    #[inline]
    pub fn peek_next_infix_operator(&self) -> ParserResult<(InfixOperator, u8)> {
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
    pub fn try_eat_expression_as_statement(&mut self) -> ParserResult<NodeId<Expression>> {
        self.with_options(self.options.in_statement(), |parser| {
            parser.try_eat_expression(TokenType::Newline)
        })
    }

    /// Try to eat an expression (return Expression::Error if error and recovery is possible).
    #[inline]
    pub fn try_eat_expression(&mut self, recover: TokenType) -> ParserResult<NodeId<Expression>> {
        match self.eat_expression() {
            Ok(expression_id) => Ok(expression_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let start = ParserMark::new(span.start as usize);
                self.try_recover(start, recover, Some(err))?;
                let error_id = self
                    .tree
                    .insert(Expression::Error, self.get_span_from(start));
                Ok(error_id)
            }
        }
    }

    /// Peek a member access of the given token type.
    /// Returns the total distance to eat (including the newlines, dot, and token).
    #[inline]
    fn peek_member(&self, token_type: TokenType) -> ParserResult<u8> {
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
    pub fn eat_expression(&mut self) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();

        // export
        let export = if self.peek_keyword(Keyword::Export).is_ok() {
            self.bump(); // eat export
            let mode = if self.peek_keyword(Keyword::Default).is_ok() {
                self.bump(); // eat default
                Some(ExportMode::Default)
            } else {
                Some(ExportMode::Item)
            };

            // just parse the export if followed by import items
            let keyword = self.peek_any_keyword().ok();
            if (keyword.is_none() || !DEFINITION_KEYWORDS.contains(&keyword.unwrap()))
                && self.peek_import_clause().is_ok()
            {
                return self.eat_export(mode);
            }

            mode
        } else {
            None
        };

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

            // shorthand lambda function value
            if token.token.ty == TokenType::Identifier
                && !self.options.in_type
                && !self.options.in_match_case
                && (self.peek_next_token(TokenType::Arrow).is_ok()
                    || self.peek_next_token(TokenType::ArrowWide).is_ok())
            {
                let lambda_id = self.eat_function(visibility, export)?;
                self.tree
                    .insert(Expression::Definition(lambda_id), self.get_span_from(start))
            }
            // parenthesis
            // (may be tuple, lambda or just a parenthesized expression)
            else if token.token.ty == TokenType::OpenParenthesis {
                // speculative start (need to backtrack for lambda)
                let speculative_start = (self.mark(), self.tree.next_id());

                self.bump(); // eat open paranthesis
                self.eat_newlines_maybe()?;

                let expression_id: NodeId<Expression> = {
                    // if we immediately see a closing parenthesis, it's an empty tuple
                    if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                        self.bump(); // eat closing parenthesis
                        self.tree.insert(
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
                        self.eat_newlines_maybe()?;
                        self.eat_token(TokenType::CloseParenthesis)?;
                        self.tree.insert(
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
                            .with_options(self.options.nested_in_parenthesis(), |parser| {
                                parser.eat_expression()
                            })?;
                        self.eat_newlines_maybe()?;
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
                            _ => self.tree.insert(
                                Expression::Parenthesized {
                                    expression: expression_id,
                                },
                                self.get_span_from(start),
                            ),
                        }
                    }
                };

                // if followed by an arrow, backtrack and parse as a lambda
                //  (also support colon for #Leniency)
                if !self.options.in_match_case
                    && (self.options.in_type || !self.options.in_before_block)
                    && (self.peek_arrow().is_ok() || self.peek_colon().is_ok())
                {
                    self.restore(speculative_start.0, speculative_start.1);
                    let lambda_id = self.eat_function(visibility, export)?;
                    self.tree
                        .insert(Expression::Definition(lambda_id), self.get_span_from(start))
                } else {
                    expression_id
                }
            }
            //
            // ------------------------------------------------------------
            // Unary operations (prefix, right associative)
            // ------------------------------------------------------------
            //

            // unary prefix operations
            else if let Ok(unary_operator) = self.peek_unary_prefix_operator() {
                let right_precedence = unary_operator.precedence();
                self.bump(); // eat unary operator (always because right associative)
                let right = self.with_options(
                    self.options.in_left_precedence(right_precedence),
                    |parser| parser.eat_expression(),
                )?;
                let expression = Expression::Unary {
                    operator: unary_operator,
                    expression: right,
                };
                self.tree.insert(expression, self.get_span_from(start))
            }
            // reference (`&` or `&var` or `&const`)
            else if self.peek_token(TokenType::ElementwiseAnd).is_ok() {
                self.bump(); // eat &
                let mutability = self.eat_scoped_mutability_maybe()?;
                let right = self.eat_expression()?;
                let expression = Expression::Reference { mutability, right };
                self.tree.insert(expression, self.get_span_from(start))
            }
            // dynamic (`$` or `$var` or `$const`)
            else if self.peek_token(TokenType::Dynamic).is_ok() {
                self.bump(); // eat $
                let mutability = self.eat_scoped_mutability_maybe()?;
                let right = self.eat_expression()?;
                let expression = Expression::Dynamic { mutability, right };
                self.tree.insert(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Declarations
            // ------------------------------------------------------------
            //

            // module
            else if keyword == Some(Keyword::Module) || keyword == Some(Keyword::Namespace) {
                let module_id = self.eat_module(visibility, export)?;
                self.tree
                    .insert(Expression::Definition(module_id), self.get_span_from(start))
            }
            // struct
            else if keyword == Some(Keyword::Struct) || keyword == Some(Keyword::Class) {
                let struct_id = self.eat_struct(visibility, export)?;
                self.tree
                    .insert(Expression::Definition(struct_id), self.get_span_from(start))
            }
            // enum
            else if keyword == Some(Keyword::Enum) {
                let enum_id = self.eat_enum(visibility, export)?;
                self.tree
                    .insert(Expression::Definition(enum_id), self.get_span_from(start))
            }
            // union
            else if keyword == Some(Keyword::Union) {
                let union_id = self.eat_union(visibility, export)?;
                self.tree
                    .insert(Expression::Definition(union_id), self.get_span_from(start))
            }
            // interface
            else if keyword == Some(Keyword::Interface) {
                let interface_id = self.eat_interface(visibility, export)?;
                self.tree.insert(
                    Expression::Definition(interface_id),
                    self.get_span_from(start),
                )
            }
            // implement
            else if keyword == Some(Keyword::Implement) {
                let implement_id = self.eat_implement(visibility, export)?;
                self.tree.insert(
                    Expression::Definition(implement_id),
                    self.get_span_from(start),
                )
            }
            // function
            else if keyword == Some(Keyword::Function) {
                let function_id = self.eat_function(visibility, export)?;
                self.tree.insert(
                    Expression::Definition(function_id),
                    self.get_span_from(start),
                )
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
            // import
            else if keyword == Some(Keyword::Import) {
                self.eat_import()?
            }
            // let
            else if keyword == Some(Keyword::Let)
                || keyword == Some(Keyword::Var)
                || keyword == Some(Keyword::Const)
            {
                self.eat_let(visibility, export)?
            }
            // type
            else if keyword == Some(Keyword::Type) || keyword == Some(Keyword::Readonly) {
                self.eat_type(visibility, export)?
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
            else if keyword == Some(Keyword::Match) || keyword == Some(Keyword::Switch) {
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
            // await
            else if keyword == Some(Keyword::Await) {
                self.eat_await()?
            }
            // yield
            else if keyword == Some(Keyword::Yield) {
                self.eat_yield()?
            }
            // return
            else if keyword == Some(Keyword::Return) {
                self.eat_return()?
            }
            //
            // ------------------------------------------------------------
            // Literals / Aliases / Values
            // ------------------------------------------------------------
            //
            // array
            else if token.token.ty == TokenType::OpenBracket {
                let array_literal = self.eat_array_literal()?;
                self.tree.insert(
                    Expression::ArrayLiteral {
                        elements: array_literal,
                    },
                    self.get_span_from(start),
                )
            }
            // anonymous struct literal
            else if !self.options.in_before_block
                && self.peek_anonymous_struct_literal_body().is_ok()
            {
                let fields = self.eat_struct_literal_body()?;
                self.tree.insert(
                    Expression::StructLiteral { ty: None, fields },
                    self.get_span_from(start),
                )
            }
            // block
            else if self.peek_block().is_ok() {
                let block_id = self.eat_block()?;
                self.tree
                    .insert(Expression::Block(block_id), self.get_span_from(start))
            }
            // tree
            else if self.peek_tree_literal().is_ok() {
                self.eat_tree_literal()?
            }
            // template
            else if self.peek_template_literal().is_ok() {
                let template_literal = self.eat_template_literal(None)?;
                self.tree.insert(
                    Expression::TemplateLiteral(template_literal),
                    self.get_span_from(start),
                )
            }
            // scalar
            else if self.peek_scalar_literal().is_ok() {
                let scalar_literal = self.eat_scalar_literal()?;
                self.tree.insert(
                    Expression::ScalarLiteral(scalar_literal),
                    self.get_span_from(start),
                )
            }
            // type
            // (type literals are contextual, most are only parsed inside type context to avoid shadowing)
            else if self.peek_type_literal().is_ok() {
                let type_literal = self.eat_type_literal()?;
                self.tree.insert(
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
                self.tree.insert(expression, self.get_span_from(start))
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
            left_expression_id = self.tree.insert(
                Expression::Call {
                    runtime,
                    receiver: left_expression_id,
                    dynamic_arguments: vec![],
                },
                self.get_span_from(start),
            );
            runtime = None;
        }
        // struct literal postfix with `{` (like `Vector2 { x: 0, y }`)
        else if let Expression::Path { .. } = self.tree.get(left_expression_id)
            && self.peek_token(TokenType::OpenBrace).is_ok()
            && !self.options.in_before_block
        {
            let fields = self.eat_struct_literal_body()?;
            left_expression_id = self.tree.insert(
                Expression::StructLiteral {
                    ty: Some(left_expression_id),
                    fields,
                },
                self.get_span_from(start),
            );
        }
        // template literal postfix with `sql` (like `sql`SELECT * FROM users`)
        else if let Expression::Path {
            path,
            static_arguments: None, // no static arguments allowed in template literals
        } = self.tree.get(left_expression_id)
            && self.peek_template_literal().is_ok()
        {
            // remove path expression
            let path = path.clone();
            debug_assert!(self.tree.next_id() == left_expression_id.id + 1);
            self.tree.reset_to(left_expression_id.id);

            // replace with template literal expression
            let template_literal = self.eat_template_literal(Some(path))?;
            left_expression_id = self.tree.insert(
                Expression::TemplateLiteral(template_literal),
                self.get_span_from(start),
            );
        }

        // eat all regular postfix operators
        loop {
            // unary postfix operations
            if let Ok(unary_operator) = self.peek_unary_postfix_operator() {
                self.bump(); // eat unary operator
                left_expression_id = self.tree.insert(
                    Expression::Unary {
                        operator: unary_operator,
                        expression: left_expression_id,
                    },
                    self.get_span_from(start),
                );
            }
            // range (`..`, `..=`)
            else if self.peek_token(TokenType::Range).is_ok()
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
                left_expression_id = self.tree.insert(
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
                left_expression_id = self.tree.insert(
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
            // unwrap or ternary if
            else if self.peek_token(TokenType::Maybe).is_ok() {
                self.bump(); // eat ?
                // maybe
                if self.peek_any_stop().is_ok()
                    || self.peek_any_close_parenthesis().is_ok()
                    || self.peek_token(TokenType::Dot).is_ok()
                    || self.peek_assign_operator().is_ok()
                {
                    left_expression_id = self.tree.insert(
                        Expression::Maybe(left_expression_id),
                        self.get_span_from(start),
                    );
                }
                // ternary if (we already have the condition)
                else {
                    // then expression
                    let then_expression_id = self.eat_expression()?;
                    // :
                    self.eat_colon()?;
                    // else expression
                    let else_expression_id = self.eat_expression()?;
                    // ternary if
                    let expression = Expression::If {
                        runtime,
                        style: IfStyle::Ternary,
                        condition: left_expression_id,
                        then_expression: then_expression_id,
                        else_expression: Some(else_expression_id),
                    };
                    left_expression_id = self.tree.insert(expression, self.get_span_from(start));
                }
            }
            // force unwrap
            else if self.peek_token(TokenType::Not).is_ok() {
                self.bump(); // eat !
                left_expression_id = self.tree.insert(
                    Expression::Must(left_expression_id),
                    self.get_span_from(start),
                );
            }
            // tuple
            // (if we have a delimiter following an expression inside parentheses)
            else if self.options.in_parenthesis
                && (self.peek_token(TokenType::Comma).is_ok()
                    || self.peek_token(TokenType::Newline).is_ok())
            {
                let is_comma = self.peek_token(TokenType::Comma).is_ok();
                self.bump(); // eat comma or newline
                self.eat_newlines_maybe()?;

                // only consider as tuple if there is more to come or there is a comma
                if is_comma || self.peek_token(TokenType::CloseParenthesis).is_err() {
                    // we already have the first element (the expression itself)
                    let first_element_id = self.tree.insert(
                        Argument::Positional {
                            value: left_expression_id,
                        },
                        self.get_span_from(start),
                    );
                    // parse remaining elements
                    let tuple_elements = self.eat_tuple_literal_body(Some(first_element_id))?;
                    // build tuple literal
                    left_expression_id = self.tree.insert(
                        Expression::TupleLiteral {
                            elements: tuple_elements,
                        },
                        self.get_span_from(start),
                    );
                }
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
            left_expression_id = self.tree.insert(left_expression, self.get_span_from(start))
        }

        Ok(left_expression_id)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{
        Definition, ExportMode, FunctionStyle, ImportTarget, IntType, Parameter, TypeLiteral,
        WithClause,
    };

    use crate::parse::tests::TestParser;
    use crate::{
        Argument, BinaryOperator, Expression, ImportItem, Mutability, Pattern, Runtime,
        ScalarLiteral, ScopedMutability, UnaryOperator, assert_expr_path, assert_node, assert_path,
        assert_string,
    };

    /// Parse `export { bar, baz } from foo` through the expression parser.
    #[test]
    fn test_parse_export_expression_with_items_block() {
        let mut test = TestParser::new("export { bar, baz } from foo");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // export { bar, baz } from foo
        assert_node!(parser.tree, expression_id, Expression::Export { mode, target: Some(ImportTarget::Virtual(target)), alias, items } => {
            assert_eq!(*mode, ExportMode::Item);
            assert!(alias.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], ImportItem { name, alias } => {
                assert_string!(parser, *name, "bar");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], ImportItem { name, alias } => {
                assert_string!(parser, *name, "baz");
                assert!(alias.is_none());
            });
            assert_path!(parser, *target, "foo");
        });
    }

    /// Parse `export { bar, baz }` through the expression parser.
    #[test]
    fn test_parse_export_expression_items_without_target() {
        let mut test = TestParser::new("export { bar, baz }");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Export { mode, target, alias, items } => {
            assert_eq!(*mode, ExportMode::Item);
            assert!(alias.is_none());
            assert!(target.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], ImportItem { name, alias } => {
                assert_string!(parser, *name, "bar");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], ImportItem { name, alias } => {
                assert_string!(parser, *name, "baz");
                assert!(alias.is_none());
            });
        });
    }

    /// Parse `export * as baz from foo` through the expression parser.
    #[test]
    fn test_parse_export_expression_star_alias() {
        let mut test = TestParser::new("export * as baz from foo");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // export * as baz from foo
        assert_node!(parser.tree, expression_id, Expression::Export { mode, target: Some(ImportTarget::Virtual(target)), alias, items } => {
            assert_eq!(*mode, ExportMode::Item);
            assert_string!(parser, alias.unwrap(), "baz");
            assert!(items.is_none());
            assert_path!(parser, *target, "foo");
        });
    }

    /// Parse an export declaration of a type definition.
    #[test]
    fn test_parse_export_expression_type_definition() {
        let mut test = TestParser::new("export type NonNullValue = Something");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::LetType { name, visibility, export, .. } => {
            assert_string!(parser, name.unwrap(), "NonNullValue");
            assert!(visibility.is_none());
            assert!(export.is_some());
        });
    }

    /// Parse `import { bar, baz } from foo` through the expression parser.
    #[test]
    fn test_parse_import_expression_with_items_block() {
        let mut test = TestParser::new("import { bar, baz } from foo");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // import { bar, baz } from foo
        assert_node!(parser.tree, expression_id, Expression::Import { target: ImportTarget::Virtual(target), alias, items } => {
            assert!(alias.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], ImportItem { name, alias } => {
                assert_string!(parser, *name, "bar");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], ImportItem { name, alias } => {
                assert_string!(parser, *name, "baz");
                assert!(alias.is_none());
            });
            assert_path!(parser, *target, "foo");
        });
    }

    /// Parse `import * as baz from foo` through the expression parser.
    #[test]
    fn test_parse_import_expression_star_alias() {
        let mut test = TestParser::new("import * as baz from foo");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // import * as baz from foo
        assert_node!(parser.tree, expression_id, Expression::Import { target: ImportTarget::Virtual(target), alias, items } => {
            assert_string!(parser, alias.unwrap(), "baz");
            assert!(items.is_none());
            assert_path!(parser, *target, "foo");
        });
    }

    /// Reject `import { foo }` without a target.
    #[test]
    fn test_parse_import_expression_items_without_target_error() {
        let mut test = TestParser::new("import { foo }");
        let mut parser = test.prepare();
        assert!(parser.eat_expression().is_err());
    }

    /// Parse mixed prefix and postfix increment/decrement operations.
    #[test]
    fn test_parse_mixed_prefix_and_postfix_increment_decrement() {
        let mut test = TestParser::new("(a++ + ++a) * (b-- - --b)");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // (a++ + ++a) * (b-- - --b)
        assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right, .. } => {

            // (a++ + ++a)
            assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Binary { left, operator, right, ..} => {
                    // a++
                    assert_node!(parser.tree, *left, Expression::Unary { operator, expression } => {
                        assert_eq!(*operator, UnaryOperator::PostIncrement);
                        assert_expr_path!(parser, parser.tree.get(*expression), "a");
                    });
                    // +
                    assert_eq!(*operator, BinaryOperator::Add);
                    // ++a
                    assert_node!(parser.tree, *right, Expression::Unary { operator, expression } => {
                        assert_eq!(*operator, UnaryOperator::PreIncrement);
                        assert_expr_path!(parser, parser.tree.get(*expression), "a");
                    });
                });
            });

            // *
            assert_eq!(*operator, BinaryOperator::Multiply);

            // (b-- - --b)
            assert_node!(parser.tree, *right, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Binary { left, operator, right, ..} => {
                    // b--
                    assert_node!(parser.tree, *left, Expression::Unary { operator, expression } => {
                        assert_eq!(*operator, UnaryOperator::PostDecrement);
                        assert_expr_path!(parser, parser.tree.get(*expression), "b");
                    });
                    // -
                    assert_eq!(*operator, BinaryOperator::Subtract);
                    // --b
                    assert_node!(parser.tree, *right, Expression::Unary { operator, expression } => {
                        assert_eq!(*operator, UnaryOperator::PreDecrement);
                        assert_expr_path!(parser, parser.tree.get(*expression), "b");
                    });
                });
            });
        });
    }

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

    /// Parse an anonymous struct literal.
    #[test]
    fn test_parse_anonymous_struct_literal() {
        let mut test = TestParser::new("{ x: 1, y }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::StructLiteral { ty: None, fields, .. } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], Argument::Named { name, value } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            assert_node!(parser.tree, fields[1], Argument::NamedShorthand { name } => {
                assert_string!(parser, *name, "y");
            });
        });
    }

    /// Parse an anonymous struct literal in parenthesis.
    #[test]
    fn test_parse_anonymous_struct_literal_in_parenthesis() {
        let mut test = TestParser::new("({ x: 1, y })");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Parenthesized { expression } => {
            assert_node!(parser.tree, *expression, Expression::StructLiteral { ty: None, fields, .. } => {
                assert_eq!(fields.len(), 2);
                assert_node!(parser.tree, fields[0], Argument::Named { name, value } => {
                    assert_string!(parser, *name, "x");
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
                assert_node!(parser.tree, fields[1], Argument::NamedShorthand { name } => {
                    assert_string!(parser, *name, "y");
                });
            });
        });
    }

    /// Parse an anonymous struct literal with newlines.
    #[test]
    fn test_parse_anonymous_struct_literal_with_newlines() {
        let mut test = TestParser::new("{\n\n x: 1,\n\n y\n}");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::StructLiteral { ty: None, fields, .. } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], Argument::Named { name, value } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            assert_node!(parser.tree, fields[1], Argument::NamedShorthand { name } => {
                assert_string!(parser, *name, "y");
            });
        });
    }

    /// Parse a ternary if expression.
    #[test]
    fn test_parse_if_ternary() {
        let mut test = TestParser::new("true ? 1 : 2");
        let mut parser = test.prepare();
        let if_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            assert_node!(parser.tree, *then_expression, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            assert_node!(parser.tree, else_expression.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });
    }

    /// Parse a lambda function type with empty parameters.
    #[test]
    fn test_parse_lambda_function_empty_type() {
        let mut test = TestParser::new("() => void");
        let mut parser = test.prepare();
        let expr_id = parser
            .with_options(parser.options.in_type(), |parser| parser.eat_expression())
            .unwrap();
        assert_node!(parser.tree, expr_id, Expression::Definition(definition_id) => {
            assert_node!(parser.tree, *definition_id, Definition::Function {
                style: FunctionStyle::Lambda,
                dynamic_parameters,
                return_type,
                ..
            } => {
                assert_eq!(dynamic_parameters.len(), 0);
                assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
            });
        });
    }

    /// Parse a lambda function type with parameters and return type.
    #[test]
    fn test_parse_lambda_function_type() {
        let mut test = TestParser::new("(a: int32) => int32 with Time");
        let mut parser = test.prepare();
        let expr_id = parser
            .with_options(parser.options.in_type(), |parser| parser.eat_expression())
            .unwrap();
        assert_node!(parser.tree, expr_id, Expression::Definition(definition_id) => {
            assert_node!(parser.tree, *definition_id, Definition::Function {
                style: FunctionStyle::Lambda,
                dynamic_parameters,
                return_type,
                with_clauses,
                ..
            } => {
                // (a: int32)
                assert_node!(parser.tree, dynamic_parameters[0], Parameter::Scalar { name, ty, .. } => {
                    assert_string!(parser, *name, "a");
                    assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType { width: Some(32), is_signed: true })));
                });
                // int32
                assert_node!(parser.tree, return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType { width: Some(32), is_signed: true })));
                // with Time
                assert_node!(parser.tree, with_clauses.as_ref().unwrap()[0], WithClause { right, .. } => {
                    assert_expr_path!(parser, parser.tree.get(*right), "Time");
                });
            });
        });
    }

    /// Parse a lambda function value with a body.
    #[test]
    fn test_parse_lambda_function_value() {
        let mut test = TestParser::new("(a) => a > 2");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Definition(definition_id) => {
            assert_node!(parser.tree, *definition_id, Definition::Function {
                style: FunctionStyle::Lambda,
                dynamic_parameters,
                return_type: None,
                body,
                ..
            } => {
                assert!(body.is_some());
                // (a)
                assert_node!(parser.tree, dynamic_parameters[0], Parameter::Scalar { name, ty: None, .. } => {
                    assert_string!(parser, *name, "a");
                });
                // a > 2
                assert_node!(parser.tree, body.unwrap(), Expression::Binary { left, operator, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    assert_expr_path!(parser, parser.tree.get(*left), "a");
                    assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });
        });
    }

    /// Parse a lambda function value with a shorthand argument.
    #[test]
    fn test_parse_lambda_function_value_shorthand() {
        let mut test = TestParser::new("x => x");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Definition(definition_id) => {
            assert_node!(parser.tree, *definition_id, Definition::Function {
                style: FunctionStyle::Lambda,
                dynamic_parameters,
                return_type: None,
                body,
                ..
            } => {
                assert_eq!(dynamic_parameters.len(), 1);
                // x
                assert_node!(parser.tree, dynamic_parameters[0], Parameter::Scalar { name, ty: None, .. } => {
                    assert_string!(parser, *name, "x");
                });
                // x
                assert_node!(parser.tree, body.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "x");
                });
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
            Expression::StructLiteral { ty: Some(ty), fields, .. } => {
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
            Expression::StructLiteral { ty: Some(ty), fields, .. } => {
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
        assert_node!(parser.tree, expr_id, Expression::Unary { operator, expression, .. } => {
            assert_eq!(*operator, UnaryOperator::Dereference);
            // x
            assert_expr_path!(parser, parser.tree.get(*expression), "x");
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
            Expression::Reference { mutability: None, right, .. } => {
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
            Expression::Reference { mutability: Some(ScopedMutability::Unscoped { mutability: Mutability::Mutable }), right, .. } => {
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
const x = 
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
                    Expression::Unary { expression, .. } => {
                        // a
                        assert_expr_path!(parser, parser.tree.get(*expression), "a");
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

    /// Parse a leading elementwise operator.
    #[test]
    fn test_parse_elementwise_leading_expression() {
        let mut test = TestParser::new(
            "
type Value =
  | string
  | number
  | boolean
        ",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expr_id = parser.eat_expression().unwrap();
        // type Value = | string | number | boolean
        assert_node!(parser.tree, expr_id, Expression::LetType { name, value, .. } => {
            // value
            assert_string!(parser, name.unwrap(), "Value");
            // | string | number | boolean
            assert_node!(parser.tree, *value, Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                // string | number
                assert_node!(parser.tree, *left, Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    // string
                    assert_node!(parser.tree, *left, Expression::TypeLiteral(TypeLiteral::String));
                    // number
                    assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Number));
                });
                // boolean
                assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Boolean));
            });
        });
    }
}
