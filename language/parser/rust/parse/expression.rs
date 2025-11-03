//! Parse expressions. Mostly defers to other parsers.

use dyst_ast::{
    DeclarationKind, DeclarationScope, DefinitionMeta, ExportType, IfStyle, PostfixPosition,
    TypeBinaryOperator, TypeUnaryOperator,
};

use crate::parse::prelude::*;
use crate::{
    Argument, AssignOperator, BinaryOperator, Expression, InfixOperator, Keyword, NodeId, NodeType,
    Parser, ParserError, ParserMark, ParserResult, Runtime, TokenSpan, TokenType, UnaryOperator,
};

pub static DEFINITION_KEYWORDS: [Keyword; 21] = [
    Keyword::Declare,
    Keyword::Namespace,
    Keyword::Module,
    Keyword::Struct,
    Keyword::Class,
    Keyword::Enum,
    Keyword::Union,
    Keyword::Function,
    Keyword::Extension,
    Keyword::Interface,
    Keyword::Type,
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

pub static DEFINITION_START_TOKENS: [TokenType; 6] = [
    TokenType::Literal,
    TokenType::Identifier,
    TokenType::OpenParenthesis,
    TokenType::OpenBrace,
    TokenType::OpenBracket,
    TokenType::LessThan,
];

pub static COMPOSITE_TYPE_KEYWORDS: [Keyword; 9] = [
    Keyword::Type,
    Keyword::Module,
    Keyword::Struct,
    Keyword::Class,
    Keyword::Enum,
    Keyword::Union,
    Keyword::Tuple,
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

    /// Peek a unary postfix operator.
    #[inline]
    pub fn peek_unary_postfix_operator(&self) -> ParserResult<UnaryOperator> {
        let token = self.peek()?;
        UnaryOperator::from_postfix_token(token.token.ty).ok_or(ParserError::unexpected(token.span))
    }

    /// Peek a type unary operator.
    #[inline]
    pub fn peek_type_unary_prefix_operator(&self) -> ParserResult<TypeUnaryOperator> {
        let token = self.peek()?;
        let token_str = self.get_span_str(token.span);
        TypeUnaryOperator::from_prefix_token(token_str, token.token.ty)
            .ok_or(ParserError::unexpected(token.span))
    }

    /// Peek a type unary postfix operator.
    #[inline]
    pub fn peek_type_unary_postfix_operator(&self) -> ParserResult<TypeUnaryOperator> {
        let token = self.peek()?;
        let next_token = self.peek_next()?;
        let token_str = self.get_span_str(token.span);
        let next_token_str = self.get_span_str(next_token.span);
        TypeUnaryOperator::from_postfix_token(token_str, next_token_str, token.token.ty)
            .ok_or(ParserError::unexpected(token.span))
    }

    /// Peek a next type unary operator.
    #[inline]
    pub fn peek_next_type_unary_operator(&self) -> ParserResult<TypeUnaryOperator> {
        let token = self.peek_next()?;
        let token_str = self.get_span_str(token.span);
        TypeUnaryOperator::from_prefix_token(token_str, token.token.ty)
            .ok_or(ParserError::unexpected(token.span))
    }

    /// Peek an assign operator.
    #[inline]
    pub fn peek_assign_operator(&self) -> ParserResult<AssignOperator> {
        let token = self.peek()?;
        AssignOperator::from_token(token.token.ty).ok_or(ParserError::unexpected(token.span))
    }

    /// Peek next assign operator.
    #[inline]
    pub fn peek_next_assign_operator(&self) -> ParserResult<AssignOperator> {
        let token = self.peek_next()?;
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

    // TODO #Broken: handle semicolon properly? (empty statements, parse, format, ..)
    // (to disambiguate expressions as values to expressions as statements)
    // just add Expression::Statement and use that as the root node in blocks/definitions?

    /// Eat an expression.
    pub fn eat_expression(&mut self) -> ParserResult<NodeId<Expression>> {
        let start = self.mark();

        //
        // ------------------------------------------------------------
        // Modifiers
        // ------------------------------------------------------------
        //

        let mut meta: DefinitionMeta = DefinitionMeta::default();

        // export
        if self.peek_keyword(Keyword::Export).is_ok() {
            self.bump(); // eat export
            let mode = if self.peek_keyword(Keyword::Default).is_ok() {
                self.bump(); // eat default
                Some(ExportType::Default)
            } else if self.peek_token(TokenType::Assign).is_ok() {
                self.bump(); // eat assign
                Some(ExportType::Module)
            } else {
                Some(ExportType::Item)
            };

            // just parse the export if followed by dependency items or module export
            let keyword = self.peek_any_keyword().ok();
            if mode == Some(ExportType::Module)
                || (keyword.is_none() || !DEFINITION_KEYWORDS.contains(&keyword.unwrap()))
                    && self.peek_import_clause().is_ok()
            {
                return self.eat_export(mode);
            }

            meta.export = mode;
        }

        // kind
        meta.kind = if self.peek_keyword(Keyword::Declare).is_ok() {
            self.bump(); // eat declare
            DeclarationKind::Declaration
        } else {
            DeclarationKind::Definition
        };

        // visibility
        meta.visibility = match self.peek_visibility() {
            Ok(Some(visibility)) => {
                self.bump(); // eat visibility
                Some(visibility)
            }
            _ => None,
        };

        // scope
        meta.scope = if self.peek_keyword(Keyword::Static).is_ok() {
            self.bump(); // eat static
            DeclarationScope::Static
        } else {
            DeclarationScope::Container
        };

        // runtime
        let mut runtime = if self.peek_token(TokenType::At).is_ok() {
            self.bump(); // eat @
            Some(Runtime::Static)
        } else {
            None
        };

        //
        // ------------------------------------------------------------
        // Main expression
        // ------------------------------------------------------------
        //

        let mut left_expression_id: NodeId<Expression> = {
            let token = *self.peek()?;
            let token_type = token.token.ty;
            let keyword = self.peek_any_keyword().ok();
            let next_token_type = self
                .peek_next()
                .ok()
                .map(|token| token.token.ty)
                .unwrap_or(TokenType::End);
            let next_next_token_type = self
                .peek_next_next()
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
                || token_type == TokenType::ElementwiseAnd
                    && self.language.is_compatible_with_typescript()
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
                        return Err(ParserError::unexpected(self.get_span_from(start)));
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
                let lambda_id = self.eat_function(meta, false, false)?;
                self.tree
                    .insert(Expression::Definition(lambda_id), self.get_span_from(start))
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
                    let lambda_id = self.eat_function(meta, false, false)?;
                    self.tree
                        .insert(Expression::Definition(lambda_id), self.get_span_from(start))
                }
                // tuple or parenthesized expression
                else {
                    self.bump(); // eat open paranthesis
                    self.eat_newlines_maybe()?;
                    // empty tuple if we immediately see a closing parenthesis
                    if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                        self.bump(); // eat closing parenthesis
                        self.tree.insert(
                            Expression::TupleLiteral { elements: vec![] },
                            self.get_span_from(start),
                        )
                    }
                    // tuple if we see a named element
                    else if self.peek_token(TokenType::Identifier).is_ok()
                        && self.peek_next_token(TokenType::Colon).is_ok()
                    {
                        let tuple_elements = self
                            .eat_sequence_literal_body(None, TokenType::CloseParenthesis)
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
                }
            }
            //
            // ------------------------------------------------------------
            // Unary operations (prefix, right associative)
            // ------------------------------------------------------------
            //

            // unary prefix operations
            else if let Ok(unary_operator) = self.peek_unary_prefix_operator() {
                self.bump(); // eat unary operator (always because right associative)
                let right = self.with_options(
                    self.options.in_left_precedence(unary_operator.precedence()),
                    |parser| parser.eat_expression(),
                )?;
                let expression = Expression::Unary {
                    operator: unary_operator,
                    expression: right,
                };
                self.tree.insert(expression, self.get_span_from(start))
            }
            // type unary operations
            else if let Ok(type_unary_operator) = self.peek_type_unary_prefix_operator() {
                self.bump(); // eat type unary operator (always because right associative)
                let right = self.with_options(
                    self.options
                        .type_in_left_precedence(type_unary_operator.precedence()),
                    |parser| parser.eat_expression(),
                )?;
                let expression = Expression::TypeUnary {
                    operator: type_unary_operator,
                    expression: right,
                };
                self.tree.insert(expression, self.get_span_from(start))
            }
            // value (`^` or `^var` or `^T`)
            else if self.peek_token(TokenType::ElementwiseXor).is_ok() {
                self.bump(); // eat ^
                let mutability = self.eat_scoped_mutability_maybe()?;
                let variance = self.eat_variance_modifier_maybe()?;
                let right = self.eat_expression()?;
                let expression = Expression::Value {
                    mutability,
                    variance,
                    right,
                };
                self.tree.insert(expression, self.get_span_from(start))
            }
            // reference (`&` or `&var` or `&T`)
            else if self.peek_token(TokenType::ElementwiseAnd).is_ok() {
                self.bump(); // eat &
                let mutability = self.eat_scoped_mutability_maybe()?;
                let variance = self.eat_variance_modifier_maybe()?;
                let right = self.eat_expression()?;
                let expression = Expression::Reference {
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
                // composite type is eagerly closed before a block 
                // (to allow stuff like `if x instanceof type { ... }` where type excludes the block)
                && (!DEFINITION_START_TOKENS.contains(&next_token_type) || self.options.in_before_block && next_token_type == TokenType::OpenBrace)
                // type is only allowed in `(type)` parenthesis to disambiguate from expression form
                // (other composites don't need this since they're always followed by `<`, `(`, or `{`))
                && (keyword != Keyword::Type || self.prev_token_type() == TokenType::OpenParenthesis && next_token_type == TokenType::CloseParenthesis)
            {
                let type_literal = self.eat_composite_type_literal()?;
                self.tree.insert(
                    Expression::TypeLiteral(type_literal),
                    self.get_span_from(start),
                )
            }
            // module
            else if (keyword == Some(Keyword::Module) || keyword == Some(Keyword::Namespace))
                && DEFINITION_START_TOKENS.contains(&next_token_type)
            {
                let module_id = self.eat_module(meta)?;
                self.tree
                    .insert(Expression::Definition(module_id), self.get_span_from(start))
            }
            // struct / class
            else if (keyword == Some(Keyword::Struct) || keyword == Some(Keyword::Class))
                && DEFINITION_START_TOKENS.contains(&next_token_type)
            {
                let struct_id = self.eat_struct(meta)?;
                self.tree
                    .insert(Expression::Definition(struct_id), self.get_span_from(start))
            }
            // enum
            else if keyword == Some(Keyword::Enum)
                && DEFINITION_START_TOKENS.contains(&next_token_type)
            {
                let enum_id = self.eat_enum(meta)?;
                self.tree
                    .insert(Expression::Definition(enum_id), self.get_span_from(start))
            }
            // union
            else if keyword == Some(Keyword::Union)
                && DEFINITION_START_TOKENS.contains(&next_token_type)
            {
                let union_id = self.eat_union(meta)?;
                self.tree
                    .insert(Expression::Definition(union_id), self.get_span_from(start))
            }
            // interface
            else if keyword == Some(Keyword::Interface)
                && DEFINITION_START_TOKENS.contains(&next_token_type)
            {
                let interface_id = self.eat_interface(meta)?;
                self.tree.insert(
                    Expression::Definition(interface_id),
                    self.get_span_from(start),
                )
            }
            // extension
            else if keyword == Some(Keyword::Extension)
                && DEFINITION_START_TOKENS.contains(&next_token_type)
            {
                let extension_id = self.eat_extension(meta)?;
                self.tree.insert(
                    Expression::Definition(extension_id),
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
                ]
                .contains(&next_token_type)
            {
                let function_id = self.eat_function(meta, false, false)?;
                self.tree.insert(
                    Expression::Definition(function_id),
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
            // with
            else if keyword == Some(Keyword::With) {
                self.eat_with()?
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
                || (keyword == Some(Keyword::Await)
                    && self.peek_next_keyword(Keyword::Import).is_ok()
                    && next_next_token_type == TokenType::OpenParenthesis)
            {
                self.eat_import()?
            }
            // let
            else if keyword == Some(Keyword::Let)
                || keyword == Some(Keyword::Var)
                || keyword == Some(Keyword::Const)
            {
                self.eat_let(meta)?
            }
            // type
            else if (keyword == Some(Keyword::Type) || keyword == Some(Keyword::Readonly))
                && ([
                    TokenType::Identifier,
                    TokenType::OpenBrace,
                    TokenType::OpenParenthesis,
                    TokenType::OpenBracket,
                    TokenType::Literal,
                ]
                .contains(&next_token_type))
            {
                self.eat_type(meta)?
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
            else if keyword == Some(Keyword::Loop) && self.peek_next_block().is_ok() {
                self.eat_loop(runtime)?
            }
            // try
            else if keyword == Some(Keyword::Try) {
                self.eat_try(runtime)?
            }
            // match / switch
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
            else if keyword == Some(Keyword::Await)
                && self.peek_next_keyword(Keyword::Import).is_err()
            {
                self.eat_await()?
            }
            // yield
            else if keyword == Some(Keyword::Yield) {
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
                let elements = self.with_options(self.options.not_in_parenthesis(), |parser| {
                    parser.eat_array_literal()
                })?;
                self.tree.insert(
                    Expression::ArrayLiteral { elements },
                    self.get_span_from(start),
                )
            }
            // anonymous struct literal
            else if token_type == TokenType::OpenBrace
                && let Ok(first_argument) = self.peek_anonymous_struct_literal_body()
            {
                let fields = self.with_options(self.options.not_in_parenthesis(), |parser| {
                    parser.eat_struct_literal_body(first_argument)
                })?;
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
            // tree literal
            else if self.language.supports_tree_literal()
                && token_type == TokenType::LessThan
                && self.peek_tree_literal().is_ok()
            {
                self.with_options(self.options.not_in_parenthesis(), |parser| {
                    parser.eat_tree_literal()
                })?
            }
            // template literal
            else if self.peek_template_literal().is_ok() {
                let template_literal = self.eat_template_literal(None)?;
                self.tree.insert(
                    Expression::TemplateLiteral(template_literal),
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
            // alias / path
            else if token_type == TokenType::Identifier {
                let path = self.eat_path().for_node_type(NodeType::Expression)?;
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
                let expression = Expression::Path {
                    path,
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
                    position: PostfixPosition::Direct,
                    runtime,
                    left: left_expression_id,
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
            let fields = self.eat_struct_literal_body(None)?;
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
            // type unary postfix operations
            else if let Ok(type_unary_operator) = self.peek_type_unary_postfix_operator() {
                self.bump(); // eat type unary operator
                if type_unary_operator == TypeUnaryOperator::AsConst {
                    self.bump(); // eat second token
                }
                left_expression_id = self.tree.insert(
                    Expression::TypeUnary {
                        operator: type_unary_operator,
                        expression: left_expression_id,
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
                    .with_options(self.options.not_in_parenthesis(), |parser| {
                        parser.eat_expression()
                    })?;
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
                let path = self.eat_path()?;
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
                        path,
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
                || self.peek_token(TokenType::Dot).is_ok()
                    && self.peek_next_token(TokenType::OpenParenthesis).is_ok()
            {
                let position = if self.peek_token(TokenType::Dot).is_ok() {
                    self.bump(); // eat .
                    PostfixPosition::Indirect
                } else {
                    PostfixPosition::Direct
                };
                left_expression_id = self.eat_call(left_expression_id, position, runtime)?;
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
                        && self.language.supports_standalone_maybe()
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
                    let then_expression_id = self
                        .with_options(self.options.in_ternary_condition(), |parser| {
                            parser.eat_expression()
                        })?;
                    // :
                    self.eat_newlines_maybe()?;
                    self.eat_colon()?;
                    self.eat_newlines_maybe()?;
                    // else expression
                    let else_expression_id = self
                        .with_options(self.options.not_in_parenthesis(), |parser| {
                            parser.eat_expression()
                        })?;
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
            // tuple
            // (if we have a delimiter following an expression inside parentheses)
            else if self.options.in_parenthesis && self.peek_token(TokenType::Comma).is_ok() {
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;
                // we already have the first element (the expression itself)
                let first_element_id = self.tree.insert(
                    Argument::Positional {
                        modifiers: None,
                        value: left_expression_id,
                    },
                    self.get_span_from(start),
                );
                // parse remaining elements
                let tuple_elements =
                    self.with_options(self.options.not_in_parenthesis(), |parser| {
                        parser.eat_sequence_literal_body(
                            Some(first_element_id),
                            TokenType::CloseParenthesis,
                        )
                    })?;
                // build tuple literal
                left_expression_id = self.tree.insert(
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
            let (right_operator, operator_offset) = {
                // infix operator on same line with higher precedence
                if let Ok((right_operator, operator_offset)) = self.peek_infix_operator()
                    && (self.options.left_precedence.is_none()
                        || self.options.left_precedence.unwrap() < right_operator.precedence())
                {
                    (right_operator, operator_offset)
                }
                // infix operator on next line with higher precedence
                else if self.peek_token(TokenType::Newline).is_ok()
                    && let Ok((right_operator, operator_offset)) = self.peek_next_infix_operator()
                    && (self.options.left_precedence.is_none()
                        || self.options.left_precedence.unwrap() < right_operator.precedence())
                {
                    (right_operator, operator_offset)
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
        AssignOperator, Block, Definition, DefinitionMeta, DefinitionType, DependencyKind,
        DependencyTarget, ExportType, FunctionStyle, IntType, Name, Parameter, PatternField,
        PostfixPosition, TypeBinaryOperator, TypeLiteral, TypeUnaryOperator, VarianceBound,
        WithClause,
    };

    use crate::parse::tests::TestParser;
    use crate::{
        Argument, BinaryOperator, DependencyItem, Expression, Mutability, Pattern, Runtime,
        ScalarLiteral, ScopedMutability, UnaryOperator, assert_expr_path, assert_name, assert_node,
        assert_path, assert_string,
    };

    /// Disambiguate using import as a path.
    #[test]
    fn test_parse_import_as_path() {
        let mut test = TestParser::new("import.meta.env");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        assert_expr_path!(parser, parser.tree.get(expression_id), "import.meta.env");
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
        assert_node!(parser.tree, expression_id, Expression::Let { pattern, value: Some(value), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "type");
            });
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
        parser.eat_newline().unwrap();

        // type = type * 2
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::Assign { left, operator, right, .. } => {
            assert_expr_path!(parser, parser.tree.get(*left), "type");
            assert_eq!(*operator, AssignOperator::Assign);
            // type * 2
            assert_node!(parser.tree, *right, Expression::Binary { left, operator, right, .. } => {
                assert_expr_path!(parser, parser.tree.get(*left), "type");
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
                assert_expr_path!(parser, parser.tree.get(*left), "x");
                assert_eq!(*operator, TypeBinaryOperator::Extends);
                assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Composite(DefinitionType::Struct)));
            });
            // { body }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert_eq!(expressions.len(), 1);
                    assert_expr_path!(parser, parser.tree.get(expressions[0]), "body");
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
            // (T instanceof class)
            assert_node!(parser.tree, *condition, Expression::Parenthesized { expression } => {
                // T instanceof class
                assert_node!(parser.tree, *expression, Expression::TypeBinary { left, operator, right } => {
                    assert_expr_path!(parser, parser.tree.get(*left), "T");
                    assert_eq!(*operator, TypeBinaryOperator::Instanceof);
                    assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Composite(DefinitionType::Class)));
                });
            });
            // { value }
            assert_node!(parser.tree, *then_expression, Expression::Block(block_id) => {
                assert_node!(parser.tree, *block_id, Block { format: _, expressions, label } => {
                    assert!(label.is_none());
                    assert_eq!(expressions.len(), 1);
                    assert_expr_path!(parser, parser.tree.get(expressions[0]), "value");
                });
            });
        });
    }

    /// Parse `export { bar, baz } from foo`.
    #[test]
    fn test_parse_export_expression_with_items_block() {
        let mut test = TestParser::new("export { bar, baz } from foo");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // export { bar, baz } from foo
        assert_node!(parser.tree, expression_id, Expression::Export { mode, kind: DependencyKind::Value, target: Some(DependencyTarget::Path(target)), alias, items, value: None } => {
            assert_eq!(*mode, ExportType::Item);
            assert!(alias.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { name, alias, .. } => {
                assert_string!(parser, *name, "bar");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem { name, alias, .. } => {
                assert_string!(parser, *name, "baz");
                assert!(alias.is_none());
            });
            assert_path!(parser, *target, "foo");
        });
    }

    /// Parse `export { bar, baz }`.
    #[test]
    fn test_parse_export_expression_items_without_target() {
        let mut test = TestParser::new("export { bar, baz }");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Export { mode, kind, target, alias, items, value: None } => {
            assert_eq!(*mode, ExportType::Item);
            assert_eq!(*kind, DependencyKind::Value);
            assert!(alias.is_none());
            assert!(target.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { name, alias, .. } => {
                assert_string!(parser, *name, "bar");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem { name, alias, .. } => {
                assert_string!(parser, *name, "baz");
                assert!(alias.is_none());
            });
        });
    }

    /// Parse `export * as baz from foo`.
    #[test]
    fn test_parse_export_expression_star_alias() {
        let mut test = TestParser::new("export * as baz from foo");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // export * as baz from foo
        assert_node!(parser.tree, expression_id, Expression::Export { mode, kind: DependencyKind::Value, target: Some(DependencyTarget::Path(target)), alias, items, value: None } => {
            assert_eq!(*mode, ExportType::Item);
            assert_string!(parser, alias.unwrap(), "baz");
            assert!(items.is_none());
            assert_path!(parser, *target, "foo");
        });
    }

    /// Parse `export = foo`.
    #[test]
    fn test_parse_export_expression_module_export() {
        let mut test = TestParser::new("export = foo");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::Export { mode, kind: DependencyKind::Value, target: None, value: Some(value), .. } => {
            assert_eq!(*mode, ExportType::Module);
            assert_expr_path!(parser, parser.tree.get(*value), "foo");
        });
    }

    /// Parse an export declaration of a type definition.
    #[test]
    fn test_parse_export_expression_type_definition() {
        let mut test = TestParser::new("export type NonNullValue = Something");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::LetType { meta: DefinitionMeta { name, visibility, export, .. }, .. } => {
            assert_string!(parser, name.unwrap().string(), "NonNullValue");
            assert!(visibility.is_none());
            assert!(export.is_some());
        });
    }

    /// Parse `import { bar, baz } from foo`.
    #[test]
    fn test_parse_import_expression_with_items_block() {
        let mut test = TestParser::new("import { bar, baz } from foo");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // import { bar, baz } from foo
        assert_node!(parser.tree, expression_id, Expression::Import { kind: DependencyKind::Value, target: DependencyTarget::Path(target), alias, items, arguments: None, .. } => {
            assert!(alias.is_none());
            let items = items.as_ref().expect("expected items");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { name, alias, .. } => {
                assert_string!(parser, *name, "bar");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem { name, alias, .. } => {
                assert_string!(parser, *name, "baz");
                assert!(alias.is_none());
            });
            assert_path!(parser, *target, "foo");
        });
    }

    /// Parse `import * as baz from foo`.
    #[test]
    fn test_parse_import_expression_star_alias_with_arguments() {
        let mut test = TestParser::new("import * as baz from foo with { bar: true }");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // import * as baz from foo with { bar: true }
        assert_node!(parser.tree, expression_id, Expression::Import { kind: DependencyKind::Value, target: DependencyTarget::Path(target), alias, items, arguments, .. } => {
            // * as baz
            assert_string!(parser, alias.unwrap(), "baz");
            assert!(items.is_none());
            assert_path!(parser, *target, "foo");
            // with { bar: true }
            let arguments = arguments.as_ref().expect("expected arguments");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Named { modifiers: _, name, value } => {
                assert_string!(parser, name.string(), "bar");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
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
                    Argument::Positional { modifiers: _, value } => {
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
                    Argument::Positional { modifiers: _, value } => {
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
        assert_node!(parser.tree, expr_id, Expression::Let { pattern, value, .. } => {
            // shapes
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "shapes");
            });
            // (...)
            assert_node!(parser.tree, value.unwrap(), Expression::TupleLiteral { elements, .. } => {
                assert_eq!(elements.len(), 5);
                // TetrisPieceShape.I
                assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                    assert_expr_path!(parser, parser.tree.get(*value), "TetrisPieceShape.I");
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
            assert_node!(parser.tree, fields[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            assert_node!(parser.tree, fields[1], Argument::Shorthand { modifiers: _, name } => {
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
                assert_node!(parser.tree, fields[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                    assert_string!(parser, *name, "x");
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
                });
                assert_node!(parser.tree, fields[1], Argument::Shorthand { modifiers: _, name } => {
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
            assert_node!(parser.tree, fields[0], Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
            assert_node!(parser.tree, fields[1], Argument::Shorthand { modifiers: _, name } => {
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
            assert_expr_path!(parser, parser.tree.get(*condition), "cond");
            // a
            assert_expr_path!(parser, parser.tree.get(*then_expression), "a");
            // b
            assert_expr_path!(parser, parser.tree.get(else_expression.unwrap()), "b");
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
            assert_node!(parser.tree, *then_expression, Expression::TupleLiteral { elements, .. } => {
                assert_eq!(elements.len(), 0);
            });
            assert_node!(parser.tree, else_expression.unwrap(), Expression::TupleLiteral { elements, .. } => {
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
            assert_node!(parser.tree, *then_expression, Expression::ArrayLiteral { elements } => {
                assert_eq!(elements.len(), 0);
            });
            assert_node!(parser.tree, else_expression.unwrap(), Expression::ArrayLiteral { elements } => {
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
            assert_node!(parser.tree, *then_expression, Expression::StructLiteral { ty: None, fields, .. } => {
                assert_eq!(fields.len(), 0);
            });
            assert_node!(parser.tree, else_expression.unwrap(), Expression::StructLiteral { ty: None, fields, .. } => {
                assert_eq!(fields.len(), 0);
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
                    assert_node!(parser.tree, *left, Expression::Member { left, path, static_arguments: Some(static_arguments) } => {
                        // y
                        assert_path!(parser, *path, "y");
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
                                    assert_expr_path!(parser, parser.tree.get(*left), "x");
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
                assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
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
                assert_eq!(dynamic_parameters.len(), 1);
                // (a)
                assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
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

    /// Parse a lambda function value with a body and pattern parameters.
    #[test]
    fn test_parse_lambda_function_value_with_pattern_parameters() {
        let mut test = TestParser::new("(_, { x, y }: T) => a");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Definition(definition_id) => {
            assert_node!(parser.tree, *definition_id, Definition::Function {
                style: FunctionStyle::Lambda,
                dynamic_parameters,
                return_type: None,
                body: Some(_),
                ..
            } => {
                assert_eq!(dynamic_parameters.len(), 2);
                // _
                assert_node!(parser.tree, dynamic_parameters[0], Parameter::Pattern { pattern, ty: None, .. } => {
                    assert_node!(parser.tree, *pattern, Pattern::Wildcard);
                });
                // { x, y }: T
                assert_node!(parser.tree, dynamic_parameters[1], Parameter::Pattern { pattern, ty, .. } => {
                    // { x, y }
                    assert_node!(parser.tree, *pattern, Pattern::Struct { fields, .. } => {
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
                assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty: None, .. } => {
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
                    Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
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
                    Argument::Shorthand { modifiers: _, name } => {
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
                    Argument::Named { modifiers: _, name: Name::Identifier(name), value } => {
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
                    Argument::Shorthand { modifiers: _, name } => {
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
                assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                        assert_path!(parser, *path, "B");
                        assert!(static_arguments.is_some());
                        // C
                        assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
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
                    Expression::Call { runtime, left, .. } => {
                        assert_eq!(*runtime, None);
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
        assert_node!(parser.tree, expr_id, Expression::Reference { mutability: Some(mutability), variance, right, .. } => {
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
            assert_eq!(*variance, Some(VarianceBound::Super));
            assert_expr_path!(parser, parser.tree.get(*right), "T");
        });
    }

    /// Parse a value expression.
    #[test]
    fn test_parse_value_expression() {
        let mut test = TestParser::new("^mut super T");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Value { mutability, variance, right, .. } => {
            assert_eq!(*mutability, Some(ScopedMutability::Unscoped { mutability: Mutability::Mutable }));
            assert_eq!(*variance, Some(VarianceBound::Super));
            assert_expr_path!(parser, parser.tree.get(*right), "T");
        });
    }

    /// Parse a new constructor call.
    #[test]
    fn test_parse_new_constructor_call() {
        let mut test = TestParser::new("new Foo()");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::New { left, static_arguments, dynamic_arguments } => {
            assert_path!(parser, *left, "Foo");
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
            assert_expr_path!(parser, parser.tree.get(*value), "foo.bar");
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
                                    Expression::Call { runtime, left, .. } => {
                                        assert_eq!(*runtime, None);
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
                    Expression::Member { left, path, .. } => {
                        assert_path!(parser, *path, "baz");
                        assert_node!(parser.tree, *left, Expression::Call { left: foo_recv, .. } => {
                            // self.foo
                            assert_expr_path!(parser, parser.tree.get(*foo_recv), "self.foo");
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
                    Expression::Call { runtime, left, .. } => {
                        assert_eq!(*runtime, None);
                        // a
                        assert_expr_path!(parser, parser.tree.get(*left), "a");
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
                            Expression::Call { runtime, left, .. } => {
                                assert_eq!(*runtime, Some(Runtime::Static));
                                // b
                                assert_expr_path!(parser, parser.tree.get(*left), "b");
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
                assert_node!(parser.tree, *left, Expression::Call { left, .. } => {
                    // y.sqrt
                    assert_expr_path!(parser, parser.tree.get(*left), "y.sqrt");
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
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, expression } => {
            // keyof
            assert_eq!(*operator, TypeUnaryOperator::Keyof);
            assert_node!(parser.tree, *expression, Expression::TypeUnary { operator, expression } => {
                // typeof
                assert_eq!(*operator, TypeUnaryOperator::Typeof);
                assert_node!(parser.tree, *expression, Expression::TypeUnary { operator, expression } => {
                    // infer
                    assert_eq!(*operator, TypeUnaryOperator::Infer);
                    assert_expr_path!(parser, parser.tree.get(*expression), "Value");
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
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, expression } => {
            assert_eq!(*operator, TypeUnaryOperator::AsConst);
            assert_expr_path!(parser, parser.tree.get(*expression), "Value");
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
        assert_node!(parser.tree, expr_id, Expression::Definition(definition_id) => {
            assert_node!(parser.tree, *definition_id, Definition::Function { meta, dynamic_parameters, return_type, .. } => {
                // isStringy
                assert_string!(parser, meta.name.unwrap().string(), "isStringy");
                assert_eq!(dynamic_parameters.len(), 1);
                // value: any
                assert_node!(parser.tree, dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "value");
                    assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Any));
                });
                // asserts value is string
                assert_node!(parser.tree, return_type.unwrap(), Expression::TypeUnary { operator, expression } => {
                    assert_eq!(*operator, TypeUnaryOperator::Asserts);
                    assert_node!(parser.tree, *expression, Expression::TypeBinary { left, operator, right, .. } => {
                        // value is string
                        assert_expr_path!(parser, parser.tree.get(*left), "value");
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
        assert_node!(parser.tree, expr_id, Expression::LetType { meta: DefinitionMeta { name, .. }, value, .. } => {
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
        assert_node!(parser.tree, expr_id, Expression::Let { mutability, meta: DefinitionMeta { name: _, .. }, value, .. } => {
            assert_eq!(*mutability, ScopedMutability::Unscoped { mutability: Mutability::Immutable });
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
    }
}
