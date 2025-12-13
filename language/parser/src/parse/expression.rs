use std::str::FromStr;

use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Argument, AssignOperator, BinaryOperator, BindingAnchor, DeclarationAbstraction,
    DeclarationDescriptor, DeclarationKind, DependencyMode, EnumKind, Expression, IfKind,
    InfixOperator, Keyword, LocalNodeId, NodeType, PostfixPosition, TokenSpan, TokenType,
    TypeBinaryOperator, TypeUnaryOperator, UnaryOperator,
};

pub static DECLARATION_KEYWORDS: [Keyword; 21] = [
    Keyword::Declare,
    Keyword::Namespace,
    Keyword::Struct,
    Keyword::Class,
    Keyword::Enum,
    Keyword::Union,
    Keyword::Function,
    Keyword::Extension,
    Keyword::Interface,
    Keyword::Type,
    Keyword::Newtype,
    Keyword::Const,
    Keyword::Readonly,
    Keyword::Let,
    Keyword::Var,
    Keyword::Override,
    Keyword::Readonly,
    Keyword::Public,
    Keyword::Protected,
    Keyword::Private,
    Keyword::Async,
];

pub static DECLARATION_START_TOKENS: [TokenType; 6] = [
    TokenType::Literal,
    TokenType::Identifier,
    TokenType::OpenParenthesis,
    TokenType::OpenBrace,
    TokenType::OpenBracket,
    TokenType::LessThan,
];

pub static COMPOSITE_TYPE_KEYWORDS: [Keyword; 8] = [
    Keyword::Type,
    Keyword::Newtype,
    Keyword::Struct,
    Keyword::Class,
    Keyword::Enum,
    Keyword::Union,
    Keyword::Interface,
    Keyword::Function,
];

pub static PATTERN_START_TOKENS: [TokenType; 7] = [
    TokenType::Identifier,
    TokenType::Literal,
    TokenType::ElementwiseAnd,
    TokenType::OpenParenthesis,
    TokenType::OpenBrace,
    TokenType::OpenBracket,
    TokenType::Wildcard,
];

// can't use anything with `<` or `>` in static arguments
// (to avoid parsing ambiguity with `<>` brackets)
static NOT_IN_STATIC_BINARY_OPERATORS: [BinaryOperator; 8] = [
    // shift
    BinaryOperator::ShiftLeft,
    BinaryOperator::SaturatingShiftLeft,
    BinaryOperator::ShiftRight,
    BinaryOperator::UnsignedShiftRight,
    // comparison
    BinaryOperator::LessThan,
    BinaryOperator::LessThanOrEqual,
    BinaryOperator::GreaterThan,
    BinaryOperator::GreaterThanOrEqual,
];

// can't use anything with `<` or `>` in tree fragments
static NOT_IN_TREE_BINARY_OPERATORS: [BinaryOperator; 9] = [
    // shift
    BinaryOperator::ShiftLeft,
    BinaryOperator::SaturatingShiftLeft,
    BinaryOperator::ShiftRight,
    BinaryOperator::UnsignedShiftRight,
    // comparison
    BinaryOperator::LessThan,
    BinaryOperator::LessThanOrEqual,
    BinaryOperator::GreaterThan,
    BinaryOperator::GreaterThanOrEqual,
    // multiply
    BinaryOperator::Divide,
];

// can't use `in` in for each expressions
static NOT_IN_FOR_EACH_BINARY_OPERATORS: [BinaryOperator; 1] = [BinaryOperator::In];

/// Make an infix operator (in context).
#[inline]
fn to_infix_operator(
    token_str: &str,
    token: &TokenSpan,
    next_token: &TokenSpan,
    next_next_token: &TokenSpan,
    options: ParserOptions,
) -> ParseResult<(InfixOperator, u8)> {
    // special case for shift right (`>>`) and unsigned shift right (`>>>`) to avoid ungluing ambiguity
    if !options.in_static
        && !options.in_tree_literal
        && token.token.ty == TokenType::GreaterThan
        && next_token.token.ty == TokenType::GreaterThan
    {
        if next_next_token.token.ty == TokenType::GreaterThan {
            Ok((InfixOperator::Binary(BinaryOperator::UnsignedShiftRight), 3))
        } else {
            Ok((InfixOperator::Binary(BinaryOperator::ShiftRight), 2))
        }
    }
    // regular binary operator
    // (only a subset of binary operators are allowed in static and tree contexts)
    else if let Some(binary_operator) = BinaryOperator::from_token(token_str, token.token.ty)
        && (!options.in_static || !NOT_IN_STATIC_BINARY_OPERATORS.contains(&binary_operator))
        && (!options.in_tree_literal || !NOT_IN_TREE_BINARY_OPERATORS.contains(&binary_operator))
        && (!options.in_for_each || !NOT_IN_FOR_EACH_BINARY_OPERATORS.contains(&binary_operator))
    {
        Ok((InfixOperator::Binary(binary_operator), 1))
    }
    // regular type binary operator
    // (forbidden in super type clauses)
    else if !options.in_super_type
        && let Some(type_binary_operator) =
            TypeBinaryOperator::from_token(token_str, token.token.ty)
    {
        Ok((InfixOperator::TypeBinary(type_binary_operator), 1))
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
        Err(ParseError::unexpected(token.span))
    }
}

impl Parser {
    /// Peek a unary prefix operator.
    #[inline]
    pub fn peek_unary_prefix_operator(&self) -> ParseResult<UnaryOperator> {
        let token = self.peek()?;
        let operator = UnaryOperator::from_prefix_token(token.token.ty)
            .ok_or(ParseError::unexpected(token.span))?;

        // dereference (*x) is not valid in JS/TS compatibility mode
        if operator == UnaryOperator::Dereference && !self.language.is_destack() {
            return Err(ParseError::unexpected(token.span));
        }

        Ok(operator)
    }

    /// Peek a unary postfix operator.
    #[inline]
    pub fn peek_unary_postfix_operator(&self) -> ParseResult<UnaryOperator> {
        let token = self.peek()?;
        UnaryOperator::from_postfix_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Peek a type unary operator.
    #[inline]
    pub fn peek_type_unary_prefix_operator(&self) -> ParseResult<TypeUnaryOperator> {
        let token = self.peek()?;
        let token_str = self.get_span_str(token.span);
        TypeUnaryOperator::from_prefix_token(token_str, token.token.ty)
            .ok_or(ParseError::unexpected(token.span))
    }

    /// Peek a type unary postfix operator.
    #[inline]
    pub fn peek_type_unary_postfix_operator(&self) -> ParseResult<TypeUnaryOperator> {
        let token = self.peek()?;
        let next_token = self.peek_next()?;
        let token_str = self.get_span_str(token.span);
        let next_token_str = self.get_span_str(next_token.span);
        TypeUnaryOperator::from_postfix_token(token_str, next_token_str, token.token.ty)
            .ok_or(ParseError::unexpected(token.span))
    }

    /// Peek a next type unary operator.
    #[inline]
    pub fn peek_next_type_unary_operator(&self) -> ParseResult<TypeUnaryOperator> {
        let token = self.peek_next()?;
        let token_str = self.get_span_str(token.span);
        TypeUnaryOperator::from_prefix_token(token_str, token.token.ty)
            .ok_or(ParseError::unexpected(token.span))
    }

    /// Peek an assign operator.
    #[inline]
    pub fn peek_assign_operator(&self) -> ParseResult<AssignOperator> {
        let token = self.peek()?;
        AssignOperator::from_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Peek next assign operator.
    #[inline]
    pub fn peek_next_assign_operator(&self) -> ParseResult<AssignOperator> {
        let token = self.peek_next()?;
        AssignOperator::from_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Peek an infix operator.
    #[inline]
    pub fn peek_infix_operator(&self) -> ParseResult<(InfixOperator, u8)> {
        let token = self.peek()?;
        let token_str = self.get_span_str(token.span);
        let next_token = self.peek_next()?;
        let next_next_token = self.peek_next_next()?;
        to_infix_operator(token_str, token, next_token, next_next_token, self.options)
    }

    /// Peek a next infix operator.
    #[inline]
    pub fn peek_next_infix_operator(&self) -> ParseResult<(InfixOperator, u8)> {
        let token = self.peek_next()?;
        let token_str = self.get_span_str(token.span);
        let next_token = self.peek_next_next()?;
        let next_next_token = self.peek_next_next_next()?;
        to_infix_operator(token_str, token, next_token, next_next_token, self.options)
    }

    /// Make an expression from an infix operator.
    #[inline]
    fn make_infix_expression(
        &self,
        left: LocalNodeId<Expression>,
        operator: InfixOperator,
        right: LocalNodeId<Expression>,
    ) -> Expression {
        match operator {
            InfixOperator::Binary(binary_operator) => Expression::Binary {
                left,
                operator: binary_operator,
                right,
            },
            InfixOperator::TypeBinary(type_binary_operator) => Expression::TypeBinary {
                left,
                operator: type_binary_operator,
                right,
            },
            InfixOperator::Assign(assign_operator) => Expression::Assign {
                left,
                operator: assign_operator,
                right,
            },
        }
    }

    /// Try to eat an expression (return Expression::Error if error and recovery is possible).
    #[inline]
    pub fn try_eat_expression(
        &mut self,
        recover: TokenType,
    ) -> ParseResult<LocalNodeId<Expression>> {
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

    /// Eat an expression that might be paranthesized (skip the parenthesis if present).
    pub fn eat_expression_parenthesized_maybe(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        if self.peek_token(TokenType::OpenParenthesis).is_ok() {
            self.bump(); // eat open parenthesis
            self.eat_newlines_maybe()?;
            let expression_id = self.eat_expression()?;
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseParenthesis)?;
            self.tree.set_span(expression_id, self.get_span_from(start));
            Ok(expression_id)
        } else {
            self.eat_expression()
        }
    }

    destack_source::ensure_sufficient_stack! {
        /// Eat an expression.
        pub fn eat_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
            let start = self.mark();

        // labelled statement (like `label: while(...)` or `label: { }`)
        if self.options.in_statement_position
            && self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token(TokenType::Colon).is_ok()
        {
            let is_next_label_target = self.peek_next_next_token(TokenType::OpenBrace).is_ok()
                || self.peek_next_next_keyword(Keyword::While).is_ok()
                || self.peek_next_next_keyword(Keyword::Do).is_ok()
                || self.peek_next_next_keyword(Keyword::For).is_ok()
                || self.peek_next_next_keyword(Keyword::Loop).is_ok()
                || self.peek_next_next_keyword(Keyword::If).is_ok()
                || self.peek_next_next_keyword(Keyword::Switch).is_ok()
                || self.peek_next_next_keyword(Keyword::Try).is_ok()
                || self.peek_next_next_keyword(Keyword::With).is_ok();
            if is_next_label_target {
                let label = self.eat_identifier()?;
                self.eat_colon()?;
                let body = self.eat_expression()?;
                let labelled_id = self.tree.insert(
                    Expression::Labelled { label, body },
                    self.get_span_from(start),
                );
                return Ok(labelled_id);
            }
        }

        //
        // ------------------------------------------------------------
        // Modifiers
        // ------------------------------------------------------------
        //

        let mut descriptor: DeclarationDescriptor = DeclarationDescriptor::default();

        // export
        if self.peek_keyword(Keyword::Export).is_ok() {
            self.bump(); // eat export
            let mode = if self.peek_keyword(Keyword::Default).is_ok() {
                self.bump(); // eat default
                Some(DependencyMode::Default)
            } else if self.peek_token(TokenType::Assign).is_ok() {
                self.bump(); // eat assign
                Some(DependencyMode::Namespace)
            } else {
                Some(DependencyMode::Item)
            };

            // just parse the export if followed by dependency items or module export
            let keyword = self.peek_any_keyword().ok();
            if mode == Some(DependencyMode::Namespace)
                || (keyword.is_none() || !DECLARATION_KEYWORDS.contains(&keyword.unwrap()))
                    && self.peek_dependency_binding().is_ok()
            {
                self.rewind(start);
                let export = self.eat_export()?;
                return Ok(export);
            }

            descriptor.export = mode;
        }

        // kind
        descriptor.kind = if self.peek_keyword(Keyword::Declare).is_ok() {
            self.bump(); // eat declare
            DeclarationKind::Declaration
        } else {
            DeclarationKind::Definition
        };

        // abstraction
        descriptor.abstraction = if self.peek_keyword(Keyword::Abstract).is_ok()
            && !self.options.in_variant
            && self.peek_next_token(TokenType::Newline).is_err()
            && self.peek_next_any_keyword().is_ok_and(|kw| DECLARATION_KEYWORDS.contains(&kw))
        {
            self.bump(); // eat abstract
            DeclarationAbstraction::Abstract
        } else {
            DeclarationAbstraction::Concrete
        };

        // anchor
        descriptor.anchor = if self.peek_keyword(Keyword::Static).is_ok() {
            self.bump(); // eat static
            BindingAnchor::Static
        } else {
            BindingAnchor::Instance
        };

        //
        // ------------------------------------------------------------
        // Main expression
        // ------------------------------------------------------------
        //

        let mut left_expression_id: LocalNodeId<Expression> = {
            let token = *self.peek()?;
            let token_type = token.token.ty;
            let keyword = self.peek_any_keyword().ok();
            let next_token_type = self
                .peek_next()
                .ok()
                .map(|token| token.token.ty)
                .unwrap_or(TokenType::End);

            #[cfg(debug_assertions)]
            let _token_str = self.get_span_str(token.span);
            #[cfg(debug_assertions)]
            let _next_token_str = self
                .peek_next()
                .ok()
                .map(|token| self.get_span_str(token.span));
            #[cfg(debug_assertions)]
            let _next_next_token_str = self
                .peek_next_next()
                .ok()
                .map(|token| self.get_span_str(token.span));

            //
            // ------------------------------------------------------------
            // Grouping
            // ------------------------------------------------------------
            //

            // eat leading elementwise operator
            if token_type == TokenType::ElementwiseOr
                || token_type == TokenType::ElementwiseAnd && !self.language.is_destack()
            {
                self.bump(); // eat elementwise operator
                let leading_binary_operator = match token_type {
                    TokenType::ElementwiseOr => BinaryOperator::ElementwiseOr,
                    TokenType::ElementwiseAnd => BinaryOperator::ElementwiseAnd,
                    _ => unreachable!(),
                };

                // eat expression
                let expression_id = self.eat_expression()?;

                // check if it's the same elementwise operator
                let expression = self.tree.get(expression_id);
                match expression {
                    Expression::Binary { operator, .. } if *operator == leading_binary_operator => {
                        // all good, leading operator matches inner operator
                    }
                    _ => {
                        return Err(ParseError::unexpected(self.get_span_from(start)));
                    }
                }

                // expand span
                self.tree.set_span(expression_id, self.get_span_from(start));

                // forward the expression (no need to parse further here)
                return Ok(expression_id);
            }
            // shorthand lambda function value
            else if token_type == TokenType::Identifier
                && !self.options.in_type
                && !self.options.in_match_case
                && (self.peek_next_token(TokenType::Arrow).is_ok()
                    || self.peek_next_token(TokenType::ArrowWide).is_ok())
            {
                let lambda_id = self.eat_function(descriptor, false, false)?;
                self.tree.insert(
                    Expression::Declaration(lambda_id),
                    self.get_span_from(start),
                )
            }
            // parenthesis
            // (may be tuple, lambda or just a parenthesized expression)
            else if token_type == TokenType::OpenParenthesis {
                let closing_pos = self.find_matching_close(
                    None,
                    TokenType::OpenParenthesis,
                    TokenType::CloseParenthesis,
                )?;
                let closing_pos = self.skip_newlines(closing_pos)?;
                // function if the paranthesis are followed by an arrow (or colon)
                if self
                    .tokens
                    .get(closing_pos as usize + 1)
                    .map(|token| token.token.ty)
                    .map(|ty| {
                        ty == TokenType::Arrow
                            || ty == TokenType::ArrowWide
                            || !self.options.in_before_type
                                && !self.options.in_ternary_condition
                                && ty == TokenType::Colon
                    })
                    .unwrap_or(false)
                {
                    let lambda_id = self.eat_function(descriptor, false, false)?;
                    self.tree.insert(
                        Expression::Declaration(lambda_id),
                        self.get_span_from(start),
                    )
                }
                // tuple or parenthesized expression
                else {
                    self.bump(); // eat open paranthesis
                    self.eat_newlines_maybe()?;
                    // empty tuple/sequence if we immediately see a closing parenthesis
                    if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                        self.bump(); // eat closing parenthesis
                        // in Destack: empty tuple
                        if self.language.is_destack() {
                            self.tree.insert(
                                Expression::TupleExpression { elements: vec![] },
                                self.get_span_from(start),
                            )
                        }
                        // in JS/TS: empty sequence (unusual but valid)
                        else {
                            self.tree.insert(
                                Expression::SequenceExpression {
                                    expressions: vec![],
                                },
                                self.get_span_from(start),
                            )
                        }
                    }
                    // tuple if we see a named element
                    else if self.language.is_destack()
                        && self.peek_token(TokenType::Identifier).is_ok()
                        && self.peek_next_token(TokenType::Colon).is_ok()
                    {
                        let tuple_elements = self
                            .eat_sequence_literal_body(None, TokenType::CloseParenthesis)
                            .for_node_type(NodeType::Expression)?;
                        self.eat_newlines_maybe()?;
                        self.eat_token(TokenType::CloseParenthesis)?;
                        self.tree.insert(
                            Expression::TupleExpression {
                                elements: tuple_elements,
                            },
                            self.get_span_from(start),
                        )
                    }
                    // may be a tuple/sequence with anonymous elements or just a parenthesized expression (see below)
                    else {
                        let inner_start = self.pos();
                        let expression_id = self
                            .with_options(self.options.nested().in_parenthesis(), |parser| {
                                parser.eat_expression()
                            })?;
                        self.eat_newlines_maybe()?;
                        self.eat_token(TokenType::CloseParenthesis)?;
                        match self.tree.get(expression_id) {
                            // if it was a tuple starting here, expand it to cover the entire span
                            //  (except if that tuple has its own parenthesis already when nesting)
                            Expression::TupleExpression { .. }
                                if self.tokens[inner_start as usize].token.ty
                                    != TokenType::OpenParenthesis =>
                            {
                                self.tree.set_span(expression_id, self.get_span_from(start));
                                expression_id
                            }
                            // if it was a sequence expression starting here, expand it to cover the entire span
                            Expression::SequenceExpression { .. }
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
                }
            }
            //
            // ------------------------------------------------------------
            // Unary operations (prefix, right associative)
            // ------------------------------------------------------------
            //

            // unary prefix operations
            else if let Ok(operator) = self.peek_unary_prefix_operator() {
                self.bump(); // eat unary operator (always because right associative)
                let right = self.with_options(
                    self.options
                        .not_in_position()
                        .in_left_precedence(operator.precedence()),
                    |parser| parser.eat_expression(),
                )?;
                let expression = Expression::Unary { operator, right };
                self.tree.insert(expression, self.get_span_from(start))
            }
            // type unary operations
            else if let Ok(operator) = self.peek_type_unary_prefix_operator() {
                self.bump(); // eat type unary operator (always because right associative)
                let right = self.with_options(
                    self.options
                        .not_in_position()
                        .in_type()
                        .in_left_precedence(operator.precedence()),
                    |parser| parser.eat_expression(),
                )?;
                let expression = Expression::TypeUnary { operator, right };
                self.tree.insert(expression, self.get_span_from(start))
            }
            // value (`^` or `^var` or `^T`)
            else if self.peek_token(TokenType::ElementwiseXor).is_ok()
                && self.language.is_destack()
            {
                self.bump(); // eat ^
                let mutability = self.eat_mutability_maybe()?;
                let variance = self.eat_variance_bound_maybe()?;
                let right = self.with_options(self.options.not_in_position(), |parser| {
                    parser.eat_expression()
                })?;
                let expression = Expression::ValueOf {
                    mutability,
                    variance,
                    right,
                };
                self.tree.insert(expression, self.get_span_from(start))
            }
            // reference (`&` or `&var` or `&T`)
            else if self.peek_token(TokenType::ElementwiseAnd).is_ok()
                && self.language.is_destack()
            {
                self.bump(); // eat &
                let mutability = self.eat_mutability_maybe()?;
                let variance = self.eat_variance_bound_maybe()?;
                let right = self.with_options(self.options.not_in_position(), |parser| {
                    parser.eat_expression()
                })?;
                let expression = Expression::ReferenceOf {
                    mutability,
                    variance,
                    right,
                };
                self.tree.insert(expression, self.get_span_from(start))
            }
            //
            // ------------------------------------------------------------
            // Declarations
            // ------------------------------------------------------------
            //

            // composite type
            else if let Some(keyword) = keyword
                && COMPOSITE_TYPE_KEYWORDS.contains(&keyword)
                && next_token_type != TokenType::Dot
                && next_token_type != TokenType::OpenBracket
                // function* is a generator declaration, not a type literal
                && !(keyword == Keyword::Function && next_token_type == TokenType::Multiply)
                // composite type is eagerly closed before a block
                // (to allow stuff like `if x instanceof type { ... }` where type excludes the block)
                && (!DECLARATION_START_TOKENS.contains(&next_token_type) || self.options.in_before_block && next_token_type == TokenType::OpenBrace)
                // type is only allowed in `(type)` parenthesis to disambiguate from expression form
                // (other composites don't need this since they're always followed by `<`, `(`, or `{`))
                && (keyword != Keyword::Type && keyword != Keyword::Newtype || self.prev_token_type() == TokenType::OpenParenthesis && next_token_type == TokenType::CloseParenthesis)
            {
                let type_literal = self.eat_composite_type_literal()?;
                self.tree.insert(
                    Expression::TypeLiteral(type_literal),
                    self.get_span_from(start),
                )
            }
            // namespace
            else if keyword == Some(Keyword::Namespace)
                && DECLARATION_START_TOKENS.contains(&next_token_type)
            {
                let namespace_id = self.eat_namespace(descriptor)?;
                self.tree.insert(
                    Expression::Declaration(namespace_id),
                    self.get_span_from(start),
                )
            }
            // struct / class
            else if (keyword == Some(Keyword::Struct) || keyword == Some(Keyword::Class))
                && DECLARATION_START_TOKENS.contains(&next_token_type)
            {
                let struct_id = self.eat_struct_or_class(descriptor)?;
                self.tree.insert(
                    Expression::Declaration(struct_id),
                    self.get_span_from(start),
                )
            }
            // enum
            else if keyword == Some(Keyword::Enum)
                && DECLARATION_START_TOKENS.contains(&next_token_type)
            {
                let enum_id = self.eat_enum(EnumKind::Enum, descriptor)?;
                self.tree
                    .insert(Expression::Declaration(enum_id), self.get_span_from(start))
            }
            // const enum
            else if keyword == Some(Keyword::Const)
                && self.peek_next_keyword(Keyword::Enum).is_ok()
            {
                self.eat_keyword(Keyword::Const)?;
                let enum_id = self.eat_enum(EnumKind::Const, descriptor)?;
                self.tree
                    .insert(Expression::Declaration(enum_id), self.get_span_from(start))
            }
            // interface
            else if keyword == Some(Keyword::Interface)
                && DECLARATION_START_TOKENS.contains(&next_token_type)
            {
                let interface_id = self.eat_interface(descriptor)?;
                self.tree.insert(
                    Expression::Declaration(interface_id),
                    self.get_span_from(start),
                )
            }
            // extension
            else if keyword == Some(Keyword::Extension)
                && DECLARATION_START_TOKENS.contains(&next_token_type)
            {
                let extension_id = self.eat_extension(descriptor)?;
                self.tree.insert(
                    Expression::Declaration(extension_id),
                    self.get_span_from(start),
                )
            }
            // function
            else if (keyword == Some(Keyword::Function)
                || keyword == Some(Keyword::Async)
                || keyword == Some(Keyword::Abstract)
                || keyword == Some(Keyword::Override)
                || (self.options.in_type
                    && keyword == Some(Keyword::New)
                    && (next_token_type == TokenType::LessThan
                        || next_token_type == TokenType::OpenParenthesis))
                || (self.options.in_variant
                    && (keyword == Some(Keyword::Get)
                        || keyword == Some(Keyword::Set)
                        || keyword == Some(Keyword::Constructor))))
                && [
                    TokenType::Identifier,
                    TokenType::OpenParenthesis,
                    TokenType::LessThan,
                    TokenType::At,
                    TokenType::Multiply, // function* generator
                ]
                .contains(&next_token_type)
            {
                let function_id = self.eat_function(descriptor, false, false)?;
                self.tree.insert(
                    Expression::Declaration(function_id),
                    self.get_span_from(start),
                )
            }
            //
            // ------------------------------------------------------------
            // Control flow(ish)
            // ------------------------------------------------------------
            //
            // new
            else if keyword == Some(Keyword::New) && next_token_type == TokenType::Identifier {
                self.eat_new()?
            }
            // delete
            else if keyword == Some(Keyword::Delete) && next_token_type == TokenType::Identifier {
                self.eat_delete()?
            }
            // import
            else if keyword == Some(Keyword::Import)
                && [
                    TokenType::Multiply,
                    TokenType::Identifier,
                    TokenType::OpenBrace,
                    TokenType::Literal,
                ]
                .contains(&next_token_type)
            {
                self.eat_import()?
            }
            // let
            else if keyword == Some(Keyword::Let)
                || keyword == Some(Keyword::Var)
                || keyword == Some(Keyword::Const)
            {
                self.eat_let(descriptor)?
            }
            // type
            else if (keyword == Some(Keyword::Type)
                || keyword == Some(Keyword::Readonly)
                || keyword == Some(Keyword::Newtype))
                && ([
                    TokenType::Identifier,
                    TokenType::OpenBrace,
                    TokenType::OpenParenthesis,
                    TokenType::OpenBracket,
                    TokenType::Literal,
                ]
                .contains(&next_token_type))
            {
                self.eat_type(descriptor)?
            }
            // if
            else if keyword == Some(Keyword::If) {
                self.eat_if()?
            }
            // while
            else if keyword == Some(Keyword::While)
                || keyword == Some(Keyword::Do)
                    // do must be followed by a while after open/close brace
                    && self
                        .find_open_and_matching_close(TokenType::OpenBrace, TokenType::CloseBrace)
                        .ok()
                        .map(|pos| {
                            self.tokens
                                .get(pos as usize + 1)
                                .map(|token| Keyword::from_str(self.get_token_str(*token)) == Ok(Keyword::While))
                                .unwrap_or(false)
                        })
                        .unwrap_or(false)
            {
                self.eat_while()?
            }
            // for
            else if keyword == Some(Keyword::For) {
                self.eat_for()?
            }
            // loop (Destack-only, for #Compatibility with TS)
            else if keyword == Some(Keyword::Loop)
                && self.language.is_destack()
                && self.peek_next_block().is_ok()
            {
                self.eat_loop()?
            }
            // try
            else if keyword == Some(Keyword::Try) {
                self.eat_try()?
            }
            // match / switch
            else if keyword == Some(Keyword::Switch)
                || (keyword == Some(Keyword::Match) && self.language.is_destack())
            {
                self.eat_match()?
            }
            // break
            else if keyword == Some(Keyword::Break) {
                self.eat_break()?
            }
            // continue
            else if keyword == Some(Keyword::Continue) {
                self.eat_continue()?
            }
            // await
            else if keyword == Some(Keyword::Await)
                && self.peek_next_keyword(Keyword::Import).is_err()
            {
                self.eat_await()?
            }
            // yield (only valid inside generator functions)
            else if keyword == Some(Keyword::Yield) && self.options.in_generator {
                self.eat_yield()?
            }
            // throw
            else if keyword == Some(Keyword::Throw) {
                self.eat_throw()?
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
            // array literal
            else if token_type == TokenType::OpenBracket {
                let elements = self.with_options(self.options.not_in_position(), |parser| {
                    parser.eat_array_literal()
                })?;
                self.tree.insert(
                    Expression::ArrayExpression { elements },
                    self.get_span_from(start),
                )
            }
            // anonymous struct literal
            else if token_type == TokenType::OpenBrace && !self.options.in_statement_position {
                let properties = self.with_options(self.options.not_in_position(), |parser| {
                    parser.eat_object_literal()
                })?;
                self.tree.insert(
                    Expression::ObjectExpression {
                        ty: None,
                        properties,
                    },
                    self.get_span_from(start),
                )
            }
            // block
            else if self.peek_block().is_ok() {
                let block_id = self.eat_block()?;
                self.tree
                    .insert(Expression::Block(block_id), self.get_span_from(start))
            }
            // tree literal
            else if self.language.supports_jsx()
                && token_type == TokenType::LessThan
                && self.peek_tree_literal().is_ok()
            {
                self.with_options(self.options.not_in_position(), |parser| {
                    parser.eat_tree_literal()
                })?
            }
            // disambiguated statically parameterized lambda <T,>(...)
            else if token_type == TokenType::LessThan
                && self.peek_next_token(TokenType::Identifier).is_ok()
                && self.peek_next_next_token(TokenType::Comma).is_ok()
            {
                let function_id = self.eat_function(descriptor, false, false)?;
                self.tree.insert(
                    Expression::Declaration(function_id),
                    self.get_span_from(start),
                )
            }
            // template literal
            else if self.peek_template_literal().is_ok() {
                let template_literal = self.eat_template_literal()?;
                self.tree.insert(
                    Expression::TemplateExpression {
                        value: template_literal,
                    },
                    self.get_span_from(start),
                )
            }
            // scalar literal
            else if self.peek_scalar_literal().is_ok() {
                let scalar_literal = self.eat_scalar_literal()?;
                self.tree.insert(
                    Expression::ScalarLiteral(scalar_literal),
                    self.get_span_from(start),
                )
            }
            // type literal
            // (type literals are contextual, most are only parsed inside type context to avoid shadowing)
            else if let Ok(type_literal) = self.peek_type_literal() {
                let type_literal = self.eat_type_literal(Some(type_literal))?;
                self.tree.insert(
                    Expression::TypeLiteral(type_literal),
                    self.get_span_from(start),
                )
            }
            // alias / path / statically parameterized call
            else if token_type == TokenType::Identifier {
                let path = self.eat_path().for_node_type(NodeType::Expression)?;

                // speculatively unwrap postfix static parameterisation with `<`
                //  (might also be just a comparison operator)
                let static_arguments = if !self.options.in_new_receiver
                    && self.peek_token(TokenType::LessThan).is_ok()
                {
                    let speculative_start = self.mark();
                    let speculative_start_idx = self.tree.next_id();
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

                // immediately parse call if we have static arguments
                // (so we can stuff the arguments into the call expression)
                if static_arguments.is_some() && self.peek_token(TokenType::OpenParenthesis).is_ok()
                {
                    let receiver = Expression::Path {
                        path,
                        static_arguments: None,
                    };
                    let receiver_id = self.tree.insert(receiver, self.get_span_from(start));
                    self.eat_call(receiver_id, static_arguments, PostfixPosition::Direct)?
                } else {
                    let expression = Expression::Path {
                        path,
                        static_arguments,
                    };
                    self.tree.insert(expression, self.get_span_from(start))
                }
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

        // struct literal postfix with `{` (like `Vector2 { x: 0, y }`)
        if let Expression::Path { .. } = self.tree.get(left_expression_id)
            && self.peek_token(TokenType::OpenBrace).is_ok()
            && !self.options.in_before_block
        {
            let properties = self.eat_object_literal()?;
            left_expression_id = self.tree.insert(
                Expression::ObjectExpression {
                    ty: Some(left_expression_id),
                    properties,
                },
                self.get_span_from(start),
            );
        }
        // template literal postfix with `sql` (like `sql`SELECT * FROM users`)
        else if self.peek_template_literal().is_ok() {
            let template_literal = self.eat_template_literal()?;
            left_expression_id = self.tree.insert(
                Expression::TaggedTemplateExpression {
                    tag: left_expression_id,
                    value: template_literal,
                },
                self.get_span_from(start),
            );
        }

        // eat all regular postfix operators
        while self.peek().is_ok() {
            // unary postfix operations
            if let Ok(operator) = self.peek_unary_postfix_operator() {
                self.bump(); // eat unary operator
                left_expression_id = self.tree.insert(
                    Expression::Unary {
                        operator,
                        right: left_expression_id,
                    },
                    self.get_span_from(start),
                );
            }
            // type unary postfix operations
            else if let Ok(operator) = self.peek_type_unary_postfix_operator() {
                self.bump(); // eat type unary operator
                if operator == TypeUnaryOperator::AsConst {
                    self.bump(); // eat second token
                }
                left_expression_id = self.tree.insert(
                    Expression::TypeUnary {
                        operator,
                        right: left_expression_id,
                    },
                    self.get_span_from(start),
                );
            }
            // range (`..`, `..=`)
            else if self.peek_token(TokenType::Range).is_ok() {
                self.bump(); // eat .. or ...
                let is_inclusive = if self.peek_token(TokenType::Equal).is_ok() {
                    self.bump(); // eat =
                    true
                } else {
                    false
                };
                let right_expression_id = self
                    .with_options(self.options.not_in_position(), |parser| {
                        parser.eat_expression()
                    })?;
                left_expression_id = self.tree.insert(
                    Expression::RangeExpression {
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
                let name = self.eat_identifier()?;
                // speculatively unwrap postfix static parameterisation with `<`
                //  (might also be just a comparison operator)
                let static_arguments = if self.peek_token(TokenType::LessThan).is_ok() {
                    let speculative_start = self.mark();
                    let speculative_start_idx = self.tree.next_id();
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
                left_expression_id = self.tree.insert(
                    Expression::Member {
                        left: left_expression_id,
                        name,
                        static_arguments,
                    },
                    self.get_span_from(start),
                );
            }
            // index (like `[]`)
            else if self.peek_token(TokenType::OpenBracket).is_ok()
                && !matches!(self.tree.get(left_expression_id), Expression::Maybe { .. })
                || self.peek_token(TokenType::Dot).is_ok()
                    && self.peek_next_token(TokenType::OpenBracket).is_ok()
            {
                let position = if self.peek_token(TokenType::Dot).is_ok() {
                    self.bump(); // eat .
                    PostfixPosition::Indirect
                } else {
                    PostfixPosition::Direct
                };
                left_expression_id = self.eat_index(left_expression_id, position)?;
            }
            // call (like `()`)
            else if self.peek_token(TokenType::OpenParenthesis).is_ok()
                && !matches!(self.tree.get(left_expression_id), Expression::Maybe { .. })
                && !self.options.in_new_receiver
                || self.peek_token(TokenType::Dot).is_ok()
                    && self.peek_next_token(TokenType::OpenParenthesis).is_ok()
            {
                let position = if self.peek_token(TokenType::Dot).is_ok() {
                    self.bump(); // eat .
                    PostfixPosition::Indirect
                } else {
                    PostfixPosition::Direct
                };
                left_expression_id = self.eat_call(left_expression_id, None, position)?;
            }
            // maybe or ternary if
            // (like `x?`, `x.?`, `x?.` or `cond ? then : else`)
            else if self.peek_token(TokenType::Maybe).is_ok()
                || self.peek_token(TokenType::Dot).is_ok()
                    && self.peek_next_token(TokenType::Maybe).is_ok()
                || self.peek_newline().is_ok() && self.peek_next_token(TokenType::Maybe).is_ok()
                || self.peek_newline().is_ok()
                    && self.peek_next_token(TokenType::Dot).is_ok()
                    && self.peek_next_next_token(TokenType::Maybe).is_ok()
            {
                self.eat_newlines_maybe()?; // eat newlines
                // maybe or maybe dot (followed by a delimiter/stop, but not preceded by a newline)
                if self.peek_token(TokenType::Maybe).is_ok()
                    && (self.peek_next_any_stop().is_ok()
                        && self.language.is_destack()
                        && self.prev_token_type() != TokenType::Newline
                        || self.peek_next_any_close_parenthesis().is_ok()
                        || self.peek_next_token(TokenType::Dot).is_ok()
                        || self.peek_next_assign_operator().is_ok())
                {
                    self.bump(); // eat ?
                    // (don't consume delimiter/stop)
                    left_expression_id = self.tree.insert(
                        Expression::Maybe {
                            left: left_expression_id,
                            position: PostfixPosition::Direct,
                        },
                        self.get_span_from(start),
                    );
                }
                // dot maybe
                else if self.peek_token(TokenType::Dot).is_ok()
                    && self.peek_next_token(TokenType::Maybe).is_ok()
                {
                    self.bump(); // eat .
                    self.bump(); // eat ?
                    left_expression_id = self.tree.insert(
                        Expression::Maybe {
                            left: left_expression_id,
                            position: PostfixPosition::Indirect,
                        },
                        self.get_span_from(start),
                    );
                }
                // ternary if (we already have the condition)
                else {
                    self.bump(); // eat ?
                    self.eat_newlines_maybe()?;
                    // then expression
                    let then_expression_id = self.with_options(
                        self.options.not_in_position().in_ternary_condition(),
                        |parser| parser.eat_expression(),
                    )?;
                    // :
                    self.eat_newlines_maybe()?;
                    self.eat_colon()?;
                    self.eat_newlines_maybe()?;
                    // else expression
                    let else_expression_id = self
                        .with_options(self.options.not_in_position(), |parser| {
                            parser.eat_expression()
                        })?;
                    // ternary if
                    let expression = Expression::If {
                        kind: IfKind::Ternary,
                        condition: left_expression_id,
                        then_expression: then_expression_id,
                        else_expression: Some(else_expression_id),
                    };
                    left_expression_id = self.tree.insert(expression, self.get_span_from(start));
                }
            }
            // must
            else if self.peek_token(TokenType::Not).is_ok()
                || self.peek_token(TokenType::Dot).is_ok()
                    && self.peek_next_token(TokenType::Not).is_ok()
            {
                let position = if self.peek_token(TokenType::Dot).is_ok() {
                    self.bump(); // eat .
                    PostfixPosition::Indirect
                } else {
                    PostfixPosition::Direct
                };
                self.bump(); // eat !
                left_expression_id = self.tree.insert(
                    Expression::Must {
                        position,
                        left: left_expression_id,
                    },
                    self.get_span_from(start),
                );
            }
            // tuple (Destack) or sequence expression (JS/TS)
            // (if we have a delimiter following an expression inside parentheses)
            else if self.options.in_parenthesis && self.peek_token(TokenType::Comma).is_ok() {
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;
                if self.language.is_destack() {
                    // build a tuple
                    let first_element_id = self.tree.insert(
                        Argument::Positional {
                            value: left_expression_id,
                        },
                        self.get_span_from(start),
                    );
                    // parse remaining elements
                    let tuple_elements =
                        self.with_options(self.options.not_in_position(), |parser| {
                            parser.eat_sequence_literal_body(
                                Some(first_element_id),
                                TokenType::CloseParenthesis,
                            )
                        })?;
                    // build tuple literal
                    left_expression_id = self.tree.insert(
                        Expression::TupleExpression {
                            elements: tuple_elements,
                        },
                        self.get_span_from(start),
                    );
                } else {
                    // build a sequence expression (comma operator)
                    let mut expressions = vec![left_expression_id];
                    // parse remaining expressions until we see the close parenthesis
                    // (mirrors eat_sequence_literal_body behavior for consistency)
                    while self.peek_token(TokenType::CloseParenthesis).is_err() {
                        // consume any comma delimiter
                        if self.peek_token(TokenType::Comma).is_ok() {
                            self.bump(); // eat comma
                            self.eat_newlines_maybe()?;
                            continue;
                        }
                        // parse next expression
                        let expr_id = self
                            .with_options(self.options.not_in_position(), |parser| {
                                parser.eat_expression()
                            })?;
                        expressions.push(expr_id);
                        self.eat_newlines_maybe()?;
                    }
                    // build sequence expression
                    left_expression_id = self.tree.insert(
                        Expression::SequenceExpression { expressions },
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
        while self.peek().is_ok() {
            let (right_operator, operator_offset) = {
                // infix operator on same line with higher precedence
                if let Ok((operator, operator_offset)) = self.peek_infix_operator()
                    && (self.options.left_precedence.is_none()
                        || self.options.left_precedence.unwrap() < operator.precedence())
                {
                    (operator, operator_offset)
                }
                // infix operator on next line with higher precedence
                else if self.peek_token(TokenType::Newline).is_ok()
                    && let Ok((operator, operator_offset)) = self.peek_next_infix_operator()
                    && (self.options.left_precedence.is_none()
                        || self.options.left_precedence.unwrap() < operator.precedence())
                {
                    (operator, operator_offset)
                }
                // no infix operator with higher precedence
                else {
                    break;
                }
            };
            if self.peek_token(TokenType::Newline).is_ok() {
                self.eat_newlines_maybe()?; // eat newlines
            }
            self.bump_by(operator_offset); // eat infix operator
            self.eat_newline_maybe()?; // allow newlines after infix operator

            // eat right expression
            let right_expression_id = self.with_options(
                self.options
                    .not_in_position()
                    .in_left_precedence(right_operator.precedence()),
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
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, AssignOperator, BinaryOperator, Block, Declaration, DeclarationDescriptor,
        DeclarationType, Declarator, DependencyItem, DependencyKind, DependencyMode, EnumField,
        EnumKind, Expression, FunctionKind, IntType, Key, Mutability, Name, Parameter, Pattern,
        PatternField, PostfixPosition, Property, ScalarLiteral, TypeBinaryOperator, TypeLiteral,
        TypeUnaryOperator, UnaryOperator, VarianceBound,
    };
    use destack_source::{LanguageOptions, LanguageType};

    use crate::{
        TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
    };

    /// Disambiguate using import as a path.
    #[test]
    fn test_parse_import_as_path() {
        let mut test = TestParser::new("import.descriptor.env");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        assert_expression_path!(
            parser,
            parser.tree.get(expression_id),
            "import.descriptor.env"
        );
    }

    /// Disambiguate using `type` as a variable.
    #[test]
    fn test_parse_type_as_variable() {
        let mut test = TestParser::new(
            r"
let type = 1
type = type * 2
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        // let type = 1
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, value: Some(value), .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "type");
                });
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
        parser.eat_newline().unwrap();

        // type = type * 2
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "type");
            assert_eq!(*operator, AssignOperator::Assign);
            // type * 2
            assert_node!(parser.tree, *right, Expression::Binary { left, operator, right, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "type");
                assert_eq!(*operator, BinaryOperator::Multiply);
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
        });
        parser.eat_newline().unwrap();
    }

    /// Parse keywords as fields and identifiers.
    #[test]
    fn test_parse_keywords_as_fields_and_identifiers() {
        let mut test = TestParser::new(
            "{ 
    // can be used as both fields and bindings
    namespace: namespace,
    module: module,
    struct: struct,
    class: class,
    enum: enum,
    union: union,
    interface: interface,
    type: type,
    implement: implement,
    function: function,
    constructor: constructor,
    // can only be used as fields
    let: 0,
    var: 0,
    new: 0,
    delete: 0,
    switch: 0,
    case: 0,
    default: 0,
    do: 0,
    while: 0,
    for: 0,
    loop: 0,
    break: 0,
    continue: 0,
    match: 0,
}",
        );
        let mut parser = test.prepare();
        let _ = parser.eat_expression().unwrap();
    }

    /// Parse an if extends struct condition without consuming the block.
    #[test]
    fn test_parse_if_extends_struct_type_literal() {
        let mut test = TestParser::new("if x extends struct {\n    body\n}");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::If { condition, then_expression, else_expression, .. } => {
            assert!(else_expression.is_none());
            // x extends struct
            assert_node!(parser.tree, *condition, Expression::TypeBinary { left, operator, right } => {
                assert_expression_path!(parser, parser.tree.get(*left), "x");
                assert_eq!(*operator, TypeBinaryOperator::Extends);
                assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Composite(DeclarationType::Struct)));
            });
            // { body }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions } => {
                    assert_eq!(expressions.len(), 1);
                    assert_expression_path!(parser, parser.tree.get(expressions[0]), "body");
                });
            });
        });
    }

    /// Parse an if instanceof class condition inside parentheses.
    #[test]
    fn test_parse_if_instanceof_class_type_literal() {
        let mut test = TestParser::new("if (T instanceof class) {\n    value\n}");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::If { condition, then_expression, else_expression, .. } => {
            assert!(else_expression.is_none());
            // T instanceof class
            assert_node!(parser.tree, *condition, Expression::TypeBinary { left, operator, right } => {
                assert_expression_path!(parser, parser.tree.get(*left), "T");
                assert_eq!(*operator, TypeBinaryOperator::InstanceOf);
                assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Composite(DeclarationType::Class)));
            });
            // { value }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions } => {
                    assert_eq!(expressions.len(), 1);
                    assert_expression_path!(parser, parser.tree.get(expressions[0]), "value");
                });
            });
        });
    }

    /// Parse `export { bar, baz } from foo`.
    #[test]
    fn test_parse_export_expression_with_items_block() {
        let mut test = TestParser::new("export { bar, baz } from \"foo\"");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // export { bar, baz } from foo
        assert_node!(parser.tree, expression_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert_eq!(items.len(), 2);
            // bar
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, *name, "bar");
                assert!(alias.is_none());
            });
            // baz
            assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, *name, "baz");
                assert!(alias.is_none());
            });
        });
    }

    /// Parse `export { bar, baz }`.
    #[test]
    fn test_parse_export_expression_items_without_target() {
        let mut test = TestParser::new("export { bar, baz }");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Export { kind, target: None, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 2);
            // bar
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, *name, "bar");
                assert!(alias.is_none());
            });
            // baz
            assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, *name, "baz");
                assert!(alias.is_none());
            });
        });
    }

    /// Parse `export * as baz from "foo"`.
    #[test]
    fn test_parse_export_expression_namespace_alias() {
        let mut test = TestParser::new("export * as baz from \"foo\"");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // export * as baz from foo
        assert_node!(parser.tree, expression_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert_eq!(items.len(), 1);
            // * as baz
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_string!(parser, *alias, "baz");
            });
        });
    }

    /// Parse `export = foo`.
    #[test]
    fn test_parse_export_expression_module_export() {
        let mut test = TestParser::new("export = foo");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::Export { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(target.is_none());
            assert_eq!(items.len(), 1);
            // = foo
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: None, value: Some(value), .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_expression_path!(parser, parser.tree.get(*value), "foo");
            });
        });
    }

    /// Parse an export declaration of a type declaration.
    #[test]
    fn test_parse_export_expression_type_declaration() {
        let mut test = TestParser::new("export type NonNullValue = Something");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor: DeclarationDescriptor { name, export, .. }, .. } => {
                assert_string!(parser, name.unwrap().string(), "NonNullValue");
                assert!(export.is_some());
            });
        });
    }

    /// Parse `import { bar, baz } from foo`.
    #[test]
    fn test_parse_import_expression_with_items_block() {
        let mut test = TestParser::new("import { bar, baz } from \"foo\"");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // import { bar, baz } from foo
        assert_node!(parser.tree, expression_id, Expression::Import { kind, target, items, arguments: None, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert_eq!(items.len(), 2);
            // bar
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, *name, "bar");
                assert!(alias.is_none());
            });
            // baz
            assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, *name, "baz");
                assert!(alias.is_none());
            });
        });
    }

    /// Parse `import * as baz from "foo" with { bar: true }`.
    #[test]
    fn test_parse_import_expression_namespace_alias_with_arguments() {
        let mut test = TestParser::new("import * as baz from \"foo\" with { bar: true }");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // import * as baz from foo with { bar: true }
        assert_node!(parser.tree, expression_id, Expression::Import { kind, target, items, arguments: Some(arguments), .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert_eq!(items.len(), 1);
            // * as baz
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_string!(parser, *alias, "baz");
            });
            // with { bar: true }
            assert_eq!(arguments.len(), 1);
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
                    assert_node!(parser.tree, *left, Expression::Unary { operator, right } => {
                        assert_eq!(*operator, UnaryOperator::PostIncrement);
                        assert_expression_path!(parser, parser.tree.get(*right), "a");
                    });
                    // +
                    assert_eq!(*operator, BinaryOperator::Add);
                    // ++a
                    assert_node!(parser.tree, *right, Expression::Unary { operator, right } => {
                        assert_eq!(*operator, UnaryOperator::PreIncrement);
                        assert_expression_path!(parser, parser.tree.get(*right), "a");
                    });
                });
            });

            // *
            assert_eq!(*operator, BinaryOperator::Multiply);

            // (b-- - --b)
            assert_node!(parser.tree, *right, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Binary { left, operator, right, ..} => {
                    // b--
                    assert_node!(parser.tree, *left, Expression::Unary { operator, right } => {
                        assert_eq!(*operator, UnaryOperator::PostDecrement);
                        assert_expression_path!(parser, parser.tree.get(*right), "b");
                    });
                    // -
                    assert_eq!(*operator, BinaryOperator::Subtract);
                    // --b
                    assert_node!(parser.tree, *right, Expression::Unary { operator, right } => {
                        assert_eq!(*operator, UnaryOperator::PreDecrement);
                        assert_expression_path!(parser, parser.tree.get(*right), "b");
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
        assert_node!(parser.tree, expr_id, Expression::TupleExpression { elements, .. } => {
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
            Expression::TupleExpression { elements, .. } => {
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

    /// Parse a tuple literal over multiple lines.
    #[test]
    fn test_parse_tuple_literal_multiline() {
        let mut test = TestParser::new(
            r"
const shapes = (
    TetrisPieceShape.I,
    TetrisPieceShape.J,
    TetrisPieceShape.L,
    TetrisPieceShape.O,
    TetrisPieceShape.S,
)",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                // shapes
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "shapes");
                });
                // (...)
                assert_node!(parser.tree, value.unwrap(), Expression::TupleExpression { elements, .. } => {
                    assert_eq!(elements.len(), 5);
                    // TetrisPieceShape.I
                    assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "TetrisPieceShape.I");
                    });
                });
            });
        });
    }

    /// Parse a range literal.
    #[test]
    fn test_parse_range_literal() {
        let mut test = TestParser::new("1..3");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::RangeExpression { start, end, .. } => {
            assert_node!(parser.tree, *start, Expression::ScalarLiteral(ScalarLiteral::Integer(val)) => {
                assert_eq!(*val, 1);
            });
            assert_node!(parser.tree, *end, Expression::ScalarLiteral(ScalarLiteral::Integer(val)) => {
                assert_eq!(*val, 3);
            });
        });
    }

    /// Parse an anonymous block.
    #[test]
    fn test_parse_anonymous_struct_literal() {
        let mut test = TestParser::new("{ }");
        let mut parser = test.prepare();
        parser.options.in_statement_position = true;
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Block { .. });
    }

    /// Parse an anonymous block with a do disambiguation.
    #[test]
    fn test_parse_anonymous_block_with_do_disambiguation() {
        let mut test = TestParser::new("let x = do { }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "x");
                });
                assert_node!(parser.tree, value.unwrap(), Expression::Block { .. });
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
            assert_node!(parser.tree, *expression, Expression::ObjectExpression { ty: None, properties, .. } => {
                assert_eq!(properties.len(), 2);
                assert_node!(parser.tree, properties[0], Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
                    assert_string!(parser, *name, "x");
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
                assert_node!(parser.tree, properties[1], Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: None, default: None, .. } => {
                    assert_string!(parser, *name, "y");
                });
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

    /// Parse a ternary if expression over multiple lines.
    #[test]
    fn test_parse_if_ternary_multiline() {
        let mut test = TestParser::new("true\n\t? 1\n\t: 2");
        let mut parser = test.prepare();
        let if_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            assert_node!(parser.tree, *condition, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            assert_node!(parser.tree, *then_expression, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            assert_node!(parser.tree, else_expression.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });
    }

    /// Parse a ternary if expression over multiple lines with comments.
    #[test]
    fn test_parse_if_ternary_multiline_with_comments() {
        let mut test = TestParser::new(
            r#"
 cond
    ? // comment
      a
    : // comment
      b"#,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let if_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            // cond
            assert_expression_path!(parser, parser.tree.get(*condition), "cond");
            // a
            assert_expression_path!(parser, parser.tree.get(*then_expression), "a");
            // b
            assert_expression_path!(parser, parser.tree.get(else_expression.unwrap()), "b");
        });
    }

    /// Parse a ternary if expression with parenthesis (disambiguate from call expression).
    #[test]
    fn test_parse_if_ternary_with_parenthesis() {
        let mut test = TestParser::new("x ? () : ()");
        let mut parser = test.prepare();
        let if_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            assert_node!(parser.tree, *condition, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
            assert_node!(parser.tree, *then_expression, Expression::TupleExpression { elements, .. } => {
                assert_eq!(elements.len(), 0);
            });
            assert_node!(parser.tree, else_expression.unwrap(), Expression::TupleExpression { elements, .. } => {
                assert_eq!(elements.len(), 0);
            });
        });
    }

    /// Parse a ternary if expression with brackets (disambiguate from index).
    #[test]
    fn test_parse_if_ternary_with_brackets() {
        let mut test = TestParser::new("x ? [] : []");
        let mut parser = test.prepare();
        let if_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            assert_node!(parser.tree, *condition, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
            assert_node!(parser.tree, *then_expression, Expression::ArrayExpression { elements } => {
                assert_eq!(elements.len(), 0);
            });
            assert_node!(parser.tree, else_expression.unwrap(), Expression::ArrayExpression { elements } => {
                assert_eq!(elements.len(), 0);
            });
        });
    }

    /// Parse a ternary if with braces (disambiguate from block).
    #[test]
    fn test_parse_if_ternary_with_braces() {
        let mut test = TestParser::new("x ? {} : {}");
        let mut parser = test.prepare();
        let if_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            assert_node!(parser.tree, *condition, Expression::Path { path, .. } => {
                assert_path!(parser, *path, "x");
            });
            assert_node!(parser.tree, *then_expression, Expression::ObjectExpression { ty: None, properties, .. } => {
                assert_eq!(properties.len(), 0);
            });
            assert_node!(parser.tree, else_expression.unwrap(), Expression::ObjectExpression { ty: None, properties, .. } => {
                assert_eq!(properties.len(), 0);
            });
        });
    }

    /// Parse a mixed index postfix expression (should disambiguate ternary and index/call).
    #[test]
    fn test_parse_mixed_index_call_postfix() {
        let mut test = TestParser::new("x?.[f]?.y<T>?.().?");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // x?.[f]?.y<T>?.().?
        // .?
        assert_node!(parser.tree, expr_id, Expression::Maybe { left, position: PostfixPosition::Indirect } => {
            // ()
            assert_node!(parser.tree, *left, Expression::Call { left, dynamic_arguments, .. } => {
                assert_eq!(dynamic_arguments.len(), 0);
                // ?
                assert_node!(parser.tree, *left, Expression::Maybe { left, position: PostfixPosition::Direct } => {
                    // .y
                    assert_node!(parser.tree, *left, Expression::Member { left, name, static_arguments: Some(static_arguments) } => {
                        // y
                        assert_string!(parser, *name, "y");
                        // <T>
                        assert_eq!(static_arguments.len(), 1);
                        // ?
                        assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                            // .[f]
                            assert_node!(parser.tree, *left, Expression::Index { left, index, position: PostfixPosition::Indirect } => {
                                // f
                                assert_node!(parser.tree, index.unwrap(), Expression::Path { path, static_arguments } => {
                                    assert!(static_arguments.is_none());
                                    assert_path!(parser, *path, "f");
                                });
                                // ?
                                assert_node!(parser.tree, *left, Expression::Maybe { left, .. } => {
                                    // x
                                    assert_expression_path!(parser, parser.tree.get(*left), "x");
                                });
                            });
                        });
                    });
                });
            });
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
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert_eq!(signature.dynamic_parameters.len(), 0);
                assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
            });
        });
    }

    /// Parse a lambda function type with parameters and return type.
    #[test]
    fn test_parse_lambda_function_type() {
        let mut test = TestParser::new("(a: int32) => int32");
        let mut parser = test.prepare();
        let expr_id = parser
            .with_options(parser.options.in_type(), |parser| parser.eat_expression())
            .unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                // (a: int32)
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "a");
                    assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
                });
                // int32
                assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Int(IntType::Arbitrary { width: Some(32), is_signed: true })));
            });
        });
    }

    /// Parse a lambda function value with a body.
    #[test]
    fn test_parse_lambda_function_value() {
        let mut test = TestParser::new("(a) => a > 2");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert!(signature.return_type.is_none());
                assert!(body.is_some());
                assert_eq!(signature.dynamic_parameters.len(), 1);
                // (a)
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
                    assert_string!(parser, *name, "a");
                });
                // a > 2
                assert_node!(parser.tree, body.unwrap(), Expression::Binary { left, operator, right } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                    assert_expression_path!(parser, parser.tree.get(*left), "a");
                    assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                });
            });
        });
    }

    /// Parse a lambda function value with a body and pattern parameters.
    #[test]
    fn test_parse_lambda_function_value_with_pattern_parameters() {
        let mut test = TestParser::new("(_, { x, y }: T) => a");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(_), .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert!(signature.return_type.is_none());
                assert_eq!(signature.dynamic_parameters.len(), 2);
                // _
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Pattern { pattern, ty: None, .. } => {
                    assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                });
                // { x, y }: T
                assert_node!(parser.tree, signature.dynamic_parameters[1], Parameter::Pattern { pattern, ty, .. } => {
                    // { x, y }
                    assert_node!(parser.tree, *pattern, Pattern::Object { fields, .. } => {
                        assert_eq!(fields.len(), 2);
                        // x
                        assert_node!(parser.tree, fields[0], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                            assert_name!(parser, *name, "x");
                        });
                        // y
                        assert_node!(parser.tree, fields[1], PatternField::Named { name, mutability: None, pattern: None, default: None } => {
                            assert_name!(parser, *name, "y");
                        });
                    });
                    // T
                    assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "T");
                    });
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
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert!(signature.return_type.is_none());
                assert_eq!(signature.dynamic_parameters.len(), 1);
                // x
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
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
            Expression::ObjectExpression { ty: Some(ty), properties, .. } => {
                // geom.Vector2
                assert_node!(
                    parser.tree,
                    *ty,
                    Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "geom.Vector2");
                    }
                );
                assert_eq!(properties.len(), 2);
                // x: 1
                assert_node!(
                    parser.tree,
                    properties[0],
                    Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
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
                    properties[1],
                    Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: None, default: None, .. } => {
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
geom.Mesh<2, 4> { 
    vertices: [1, 2],
    y,
}"##,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(
            parser.tree,
            expr_id,
            Expression::ObjectExpression { ty: Some(ty), properties, .. } => {
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
                assert_eq!(properties.len(), 2);
                // vertices: [1, 2]
                assert_node!(
                    parser.tree,
                    properties[0],
                    Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
                        assert_string!(parser, *name, "vertices");
                        assert_node!(
                            parser.tree,
                            *value,
                            Expression::ArrayExpression { .. }
                        );
                    }
                );
                // y
                assert_node!(
                    parser.tree,
                    properties[1],
                    Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: None, default: None, .. } => {
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
        assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
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
                                assert_expression_path!(parser, parser.tree.get(*value), "C");
                            });
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
            assert_expression_path!(parser, parser.tree.get(*right), "x");
        });
    }

    /// Dereference should fail in JavaScript compatibility mode.
    #[test]
    fn test_dereference_fails_in_js_mode() {
        let options = LanguageOptions::default().with_type(LanguageType::JavaScript);
        let mut test = TestParser::new_with_options("*x", options);
        let mut parser = test.prepare();
        // Should fail to parse *x as dereference in JS mode
        let result = parser.eat_expression();
        assert!(result.is_err() || !parser.diagnostics.is_empty());
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
            Expression::ReferenceOf { mutability: None, right, .. } => {
                // x
                assert_expression_path!(parser, parser.tree.get(*right), "x");
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
            Expression::ReferenceOf { mutability: Some(Mutability::Mutable), right, .. } => {
                // self.foo()
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Call { left, .. } => {
                        // self.foo
                        assert_node!(
                            parser.tree,
                            *left,
                            Expression::Path { path, .. } => {
                                assert_path!(parser, *path, "self.foo");
                            }
                        );
                    }
                );
            }
        );
    }

    /// Parse a bound reference expression.
    #[test]
    fn test_parse_bound_reference_expression() {
        let mut test = TestParser::new("&const super T");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::ReferenceOf { mutability: Some(mutability), variance, right, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(*variance, Some(VarianceBound::Super));
            assert_expression_path!(parser, parser.tree.get(*right), "T");
        });
    }

    /// Parse a value expression.
    #[test]
    fn test_parse_value_expression() {
        let mut test = TestParser::new("^mut super T");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::ValueOf { mutability, variance, right, .. } => {
            assert_eq!(*mutability, Some(Mutability::Mutable));
            assert_eq!(*variance, Some(VarianceBound::Super));
            assert_expression_path!(parser, parser.tree.get(*right), "T");
        });
    }

    /// Parse a new constructor call.
    #[test]
    fn test_parse_new_constructor_call() {
        let mut test = TestParser::new("new Foo()");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::New { left, static_arguments, dynamic_arguments } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Foo");
            assert!(static_arguments.is_none());
            assert!(dynamic_arguments.is_empty());
        });
    }

    /// Parse a delete expression.
    #[test]
    fn test_parse_delete_expression() {
        let mut test = TestParser::new("delete foo.bar");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Delete { value } => {
            assert_expression_path!(parser, parser.tree.get(*value), "foo.bar");
        });
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
            Expression::Let { mutability, declarators, .. } => {
                assert_eq!(*mutability, Mutability::Immutable);
                assert_eq!(declarators.len(), 1);
                assert_node!(
                    parser.tree,
                    declarators[0],
                    Declarator { pattern, value, .. } => {
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
                                            Expression::Call { left, .. } => {
                                                // foo.parse
                                                assert_node!(
                                                    parser.tree,
                                                    *left,
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
            Expression::Call { left: baz_recv, .. } => {
                // self.foo()
                assert_node!(
                    parser.tree,
                    *baz_recv,
                    Expression::Member { left, name, .. } => {
                        assert_string!(parser, *name, "baz");
                        assert_node!(parser.tree, *left, Expression::Call { left: foo_recv, .. } => {
                            // self.foo
                            assert_expression_path!(parser, parser.tree.get(*foo_recv), "self.foo");
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
                assert_expression_path!(parser, parser.tree.get(*left), "x");
                // y
                assert_expression_path!(parser, parser.tree.get(*right), "y");
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
                        assert_expression_path!(parser, parser.tree.get(*left), "a");
                        assert_eq!(*operator, BinaryOperator::Add);
                        // b
                        assert_expression_path!(parser, parser.tree.get(*right), "b");
                    }
                );
                // c
                assert_expression_path!(parser, parser.tree.get(*right), "c");
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
                        assert_expression_path!(parser, parser.tree.get(*left), "a");
                        assert_eq!(*operator, BinaryOperator::Add);
                        // b
                        assert_expression_path!(parser, parser.tree.get(*right), "b");
                    }
                );
                // c
                assert_expression_path!(parser, parser.tree.get(*right), "c");
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
                assert_expression_path!(parser, parser.tree.get(*left), "a");
                // b * c
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Multiply);
                        // b
                        assert_expression_path!(parser, parser.tree.get(*left), "b");
                        // c
                        assert_expression_path!(parser, parser.tree.get(*right), "c");
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
                        assert_expression_path!(parser, parser.tree.get(*left), "a");
                        // b * c
                        assert_node!(
                            parser.tree,
                            *right,
                            Expression::Binary { left, operator, right, .. } => {
                                assert_eq!(*operator, BinaryOperator::Multiply);
                                // b
                                assert_expression_path!(parser, parser.tree.get(*left), "b");
                                // c
                                assert_expression_path!(parser, parser.tree.get(*right), "c");
                            }
                        );
                    }
                );
                // d
                assert_expression_path!(parser, parser.tree.get(*right), "d");
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
                        assert_expression_path!(parser, parser.tree.get(*left), "a");
                        // b
                        assert_expression_path!(parser, parser.tree.get(*right), "b");
                    }
                );
                // c + d
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Add);
                        // c
                        assert_expression_path!(parser, parser.tree.get(*left), "c");
                        // d
                        assert_expression_path!(parser, parser.tree.get(*right), "d");
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
                        assert_expression_path!(parser, parser.tree.get(*left), "a");
                        // b
                        assert_expression_path!(parser, parser.tree.get(*right), "b");
                    }
                );
                // c == d
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Equal);
                        // c
                        assert_expression_path!(parser, parser.tree.get(*left), "c");
                        // d
                        assert_expression_path!(parser, parser.tree.get(*right), "d");
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
                        assert_expression_path!(parser, parser.tree.get(*right), "a");
                    }
                );
                // b
                assert_expression_path!(parser, parser.tree.get(*right), "b");
            }
        );
    }

    /// Postfix call has higher precedence than addition.
    #[test]
    fn test_parse_precedence_postfix_call_before_add() {
        let mut test = TestParser::new("a() + b() / c");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a() + b() / c
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::Add);
                // a()
                assert_node!(
                    parser.tree,
                    *left,
                    Expression::Call { left, .. } => {
                        // a
                        assert_expression_path!(parser, parser.tree.get(*left), "a");
                    }
                );
                // b() / c
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Divide);
                        // b()
                        assert_node!(
                            parser.tree,
                            *left,
                            Expression::Call { left, .. } => {
                                // b
                                assert_expression_path!(parser, parser.tree.get(*left), "b");
                            }
                        );
                        // c
                        assert_expression_path!(parser, parser.tree.get(*right), "c");
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
                assert_node!(parser.tree, *left, Expression::Call { left, .. } => {
                    // y.sqrt
                    assert_expression_path!(parser, parser.tree.get(*left), "y.sqrt");
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

    /// Parse type unary prefix keyof, typeof, and infer operations.
    #[test]
    fn test_parse_type_unary_prefix_expression() {
        let mut test = TestParser::new("keyof typeof infer Value");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // keyof typeof infer Value
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
            // keyof
            assert_eq!(*operator, TypeUnaryOperator::Keyof);
            assert_node!(parser.tree, *right, Expression::TypeUnary { operator, right } => {
                // typeof
                assert_eq!(*operator, TypeUnaryOperator::Typeof);
                assert_node!(parser.tree, *right, Expression::TypeUnary { operator, right } => {
                    // infer
                    assert_eq!(*operator, TypeUnaryOperator::Infer);
                    assert_expression_path!(parser, parser.tree.get(*right), "Value");
                });
            });
        });
    }

    /// Parse type unary postfix as const operation.
    #[test]
    fn test_parse_type_unary_postfix_as_const_expression() {
        let mut test = TestParser::new("Value as const");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // Value as const
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
            assert_eq!(*operator, TypeUnaryOperator::AsConst);
            assert_expression_path!(parser, parser.tree.get(*right), "Value");
        });
    }

    /// Parse a type asserts expression.
    #[test]
    fn test_parse_type_unary_postfix_asserts_expression() {
        let mut test = TestParser::new(
            r"
function isStringy(value: any): asserts value is string {
    // ...
}
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let expr_id = parser.eat_expression().unwrap();
        // function isStringy(value: any): asserts value is string { .. }
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { descriptor, signature, .. } => {
                // isStringy
                assert_string!(parser, descriptor.name.unwrap().string(), "isStringy");
                assert_eq!(signature.dynamic_parameters.len(), 1);
                // value: any
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "value");
                    assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Any));
                });
                // asserts value is string
                assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeUnary { operator, right } => {
                    assert_eq!(*operator, TypeUnaryOperator::Asserts);
                    assert_node!(parser.tree, *right, Expression::TypeBinary { left, operator, right, .. } => {
                        // value is string
                        assert_expression_path!(parser, parser.tree.get(*left), "value");
                        // is
                        assert_eq!(*operator, TypeBinaryOperator::Is);
                        // string
                        assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::String));
                    });
                });
            });
        });
    }

    /// Parse a leading elementwise operator in a type expression.
    #[test]
    fn test_parse_elementwise_leading_type_expression() {
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
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor: DeclarationDescriptor { name, .. }, value, .. } => {
                // value
                assert_string!(parser, name.unwrap().string(), "Value");
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
        });
    }

    /// Parse a leading elementwise operator in a value expression.
    #[test]
    fn test_parse_elementwise_leading_value_expression() {
        let mut test = TestParser::new(
            "
const value =
  | 1
  | 2
  | 3",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expr_id = parser.eat_expression().unwrap();
        // const value = | 1 | 2 | 3
        assert_node!(parser.tree, expr_id, Expression::Let { mutability, declarators, .. } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
                // | 1 | 2 | 3
                assert_node!(parser.tree, value.unwrap(), Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    // 1 | 2
                    assert_node!(parser.tree, *left, Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                        // 1
                        assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                        // 2
                        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
                    });
                    // 3
                    assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
                });
            });
        });
    }

    /// Parse a statement expression.
    #[test]
    fn test_parse_statement_expression() {
        let mut test = TestParser::new("a;");
        let mut parser = test.prepare();
        let expr_id = parser.try_eat_statement_expression().unwrap();
        // a;
        assert_node!(parser.tree, expr_id, Expression::Statement(expression_id) => {
            assert_expression_path!(parser, parser.tree.get(*expression_id), "a");
        });
    }

    /// Comma in parentheses parses as sequence expression.
    #[test]
    fn test_parse_sequence_expression() {
        let options = LanguageOptions::default().with_type(LanguageType::JavaScript);
        let mut test = TestParser::new_with_options("(a, b, c)", options);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // (a, b, c)
        assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
            assert_eq!(expressions.len(), 3);
            // a
            assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");
            // b
            assert_expression_path!(parser, parser.tree.get(expressions[1]), "b");
            // c
            assert_expression_path!(parser, parser.tree.get(expressions[2]), "c");
        });
    }

    /// Comma in parentheses parses as tuple expression.
    #[test]
    fn test_parse_tuple_expression() {
        let options = LanguageOptions::default().with_type(LanguageType::Destack);
        let mut test = TestParser::new_with_options("(a, b, c)", options);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // (a, b, c)
        assert_node!(parser.tree, expr_id, Expression::TupleExpression { elements } => {
            assert_eq!(elements.len(), 3);
            // a
            assert_node!(parser.tree, elements[0], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "a");
            });
            // b
            assert_node!(parser.tree, elements[1], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "b");
            });
            // c
            assert_node!(parser.tree, elements[2], Argument::Positional { value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "c");
            });
        });
    }

    /// Sequence expression with type literal elements.
    #[test]
    fn test_parse_sequence_expression_with_type_literal() {
        let options = LanguageOptions::default().with_type(LanguageType::JavaScript);
        let mut test = TestParser::new_with_options("(a, void, 1)", options);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // (a, void, 1)
        assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
            assert_eq!(expressions.len(), 3);
            // a
            assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");
            // void (type literal)
            assert_node!(parser.tree, expressions[1], Expression::TypeLiteral(TypeLiteral::Void));
            // 1
            assert_node!(parser.tree, expressions[2], Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    }

    /// Test const enum declaration.
    #[test]
    fn test_parse_const_enum() {
        let mut test = TestParser::new("const enum Foo { A, B }");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // const enum Foo { A, B }
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Enum { descriptor, kind, fields, .. } => {
                assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
                assert_eq!(*kind, EnumKind::Const);
                assert_eq!(fields.len(), 2);
                assert_node!(parser.tree, fields[0], EnumField { name, value } => {
                    assert_string!(parser, name.string(), "A");
                    assert!(value.is_none());
                });
                assert_node!(parser.tree, fields[1], EnumField { name, value } => {
                    assert_string!(parser, name.string(), "B");
                    assert!(value.is_none());
                });
            });
        });
    }
}
