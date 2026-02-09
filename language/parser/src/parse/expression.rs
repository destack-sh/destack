use std::str::FromStr;

use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Argument, AssignOperator, Asynchrony, BinaryOperator, BindingAnchor, Block, BlockFormat,
    Declaration, DeclarationAbstraction, DeclarationDescriptor, DeclarationKind, DependencyMode,
    EnumKind, Expression, FunctionKind, IfCondition, IfKind, InfixOperator, Keyword, LiteralType,
    LocalNodeId, NodeType, PostfixPosition, Token, TokenSpan, TokenType, TypeBinaryOperator,
    TypeKind, TypeUnaryOperator, UnaryOperator,
};
use destack_base::StringId;
use destack_source::LanguageType;

pub static DECLARATION_KEYWORDS: [Keyword; 23] = [
    Keyword::Declare,
    Keyword::Abstract,
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
    Keyword::Using,
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

pub static PATTERN_START_TOKENS: [TokenType; 6] = [
    TokenType::Identifier,
    TokenType::Literal,
    TokenType::ElementwiseAnd,
    TokenType::OpenParenthesis,
    TokenType::OpenBrace,
    TokenType::OpenBracket,
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

/// Result of parsing declaration modifiers.
enum DescriptorHead {
    /// Parsed declaration descriptor.
    Descriptor(DeclarationDescriptor),
    /// Parsed expression that consumed the modifiers.
    Expression(LocalNodeId<Expression>),
}

/// Make an infix operator (in context).
#[inline]
fn to_infix_operator(
    token_str: &str,
    token: &TokenSpan,
    next_token: &TokenSpan,
    next_next_token: &TokenSpan,
    options: ParserOptions,
    language: LanguageType,
    has_newline: bool,
) -> ParseResult<(InfixOperator, u8)> {
    // `>>` and `>>>` require adjacent tokens: comments or trivia between `>` tokens must not glue
    let has_adjacent_shift_tokens = token.span.end == next_token.span.start;
    let has_adjacent_unsigned_shift_tokens =
        has_adjacent_shift_tokens && next_token.span.end == next_next_token.span.start;

    // special case for shift right (`>>`) and unsigned shift right (`>>>`) to avoid ungluing ambiguity
    if !options.in_static
        && !options.in_tree_literal
        && !options.in_type
        && token.token.ty == TokenType::GreaterThan
        && next_token.token.ty == TokenType::GreaterThan
        && has_adjacent_shift_tokens
    {
        if next_next_token.token.ty == TokenType::GreaterThan && has_adjacent_unsigned_shift_tokens
        {
            Ok((InfixOperator::Binary(BinaryOperator::UnsignedShiftRight), 3))
        } else {
            Ok((InfixOperator::Binary(BinaryOperator::ShiftRight), 2))
        }
    }
    // regular binary operator
    // (only a subset of binary operators are allowed in static and tree contexts)
    else if let Some(binary_operator) = BinaryOperator::from_token(token_str, token.token.ty)
        && (!options.in_type
            || options.in_static
            || matches!(
                binary_operator,
                BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
            ))
        && (!options.in_static || !NOT_IN_STATIC_BINARY_OPERATORS.contains(&binary_operator))
        && (!options.in_tree_literal || !NOT_IN_TREE_BINARY_OPERATORS.contains(&binary_operator))
        && (!options.in_for_each || !NOT_IN_FOR_EACH_BINARY_OPERATORS.contains(&binary_operator))
    {
        Ok((InfixOperator::Binary(binary_operator), 1))
    }
    // regular type binary operator
    // (forbidden in super type clauses, avoid newline glue in TS mode)
    else if !options.in_super_type
        && let Some(type_binary_operator) =
            TypeBinaryOperator::from_token(token_str, token.token.ty)
        && (!options.in_type_mapped_constraint || type_binary_operator != TypeBinaryOperator::Cast)
        && (!options.in_for_each || type_binary_operator != TypeBinaryOperator::In)
        && (options.in_type
            || matches!(
                type_binary_operator,
                TypeBinaryOperator::Cast | TypeBinaryOperator::Satisfies
            )
            || (language.is_destack()
                && matches!(
                    type_binary_operator,
                    TypeBinaryOperator::Extends
                        | TypeBinaryOperator::Implements
                        | TypeBinaryOperator::Is
                )))
        && (language.is_destack() || !has_newline)
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
    /// Unwrap statement and label wrappers to get the underlying expression.
    pub(crate) fn unwrap_statement_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let mut current = expression_id;
        loop {
            let expression = self.tree.get(current);
            match expression {
                Expression::Statement(inner) => {
                    current = *inner;
                }
                Expression::Labelled { body, .. } => {
                    current = *body;
                }
                _ => break,
            }
        }
        current
    }

    /// Return true when an expression is a lambda declaration without wrapping parentheses.
    pub(crate) fn is_unparenthesized_lambda_expression(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        matches!(
            self.tree.get(expression_id),
            Expression::Declaration(declaration_id)
                if matches!(
                    self.tree.get(*declaration_id),
                    Declaration::Function { signature, .. }
                        if signature.kind == FunctionKind::Lambda
                )
        )
    }

    /// Return true when an expression can be used as an unparenthesized tagged template tag.
    fn tagged_template_tag_is_valid(&self, expression_id: LocalNodeId<Expression>) -> bool {
        // unparenthesized lambdas cannot be tagged template receivers
        if self.is_unparenthesized_lambda_expression(expression_id) {
            return false;
        }

        // unparenthesized unary expressions are not valid tagged template receivers
        !matches!(self.tree.get(expression_id), Expression::Unary { .. })
    }

    /// Peek a unary prefix operator.
    #[inline]
    pub fn peek_unary_prefix_operator(&mut self) -> ParseResult<UnaryOperator> {
        let token = *self.peek()?;
        if token.token.ty == TokenType::Identifier && !self.options.in_type {
            let token_str = self.get_span_str(token.span);
            if let Ok(keyword) = Keyword::from_str(token_str)
                && let Some(operator) = UnaryOperator::from_prefix_keyword(keyword)
            {
                return Ok(operator);
            }
        }
        let operator = UnaryOperator::from_prefix_token(token.token.ty)
            .ok_or(ParseError::unexpected(token.span))?;

        // dereference (*x) is not valid in JS/TS compatibility mode
        if operator == UnaryOperator::Dereference && !self.language.is_destack() {
            return Err(ParseError::unexpected(token.span));
        }

        // spread (...x) is not a valid standalone expression in JS/TS
        if operator == UnaryOperator::Spread && !self.language.is_destack() {
            return Err(ParseError::unexpected(token.span));
        }

        Ok(operator)
    }

    /// Peek a unary postfix operator.
    #[inline]
    pub fn peek_unary_postfix_operator(&mut self) -> ParseResult<UnaryOperator> {
        let token = *self.peek()?;
        UnaryOperator::from_postfix_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Peek a type unary operator.
    #[inline]
    pub fn peek_type_unary_prefix_operator(&mut self) -> ParseResult<TypeUnaryOperator> {
        let token = *self.peek()?;
        let token_str = self.get_span_str(token.span);
        TypeUnaryOperator::from_prefix_token(token_str, token.token.ty)
            .ok_or(ParseError::unexpected(token.span))
    }

    /// Peek a type unary postfix operator.
    #[inline]
    pub fn peek_type_unary_postfix_operator(&mut self) -> ParseResult<TypeUnaryOperator> {
        let token = *self.peek()?;
        let next_token = *self.peek_next()?;
        let token_str = self.get_span_str(token.span);
        let next_token_str = self.get_span_str(next_token.span);
        TypeUnaryOperator::from_postfix_token(token_str, next_token_str, token.token.ty)
            .ok_or(ParseError::unexpected(token.span))
    }

    /// Peek a next type unary operator.
    #[inline]
    pub fn peek_next_type_unary_operator(&mut self) -> ParseResult<TypeUnaryOperator> {
        let token = *self.peek_next()?;
        let token_str = self.get_span_str(token.span);
        TypeUnaryOperator::from_prefix_token(token_str, token.token.ty)
            .ok_or(ParseError::unexpected(token.span))
    }

    /// Peek an assign operator.
    #[inline]
    pub fn peek_assign_operator(&mut self) -> ParseResult<AssignOperator> {
        let token = *self.peek()?;
        AssignOperator::from_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Peek next assign operator.
    #[inline]
    pub fn peek_next_assign_operator(&mut self) -> ParseResult<AssignOperator> {
        let token = *self.peek_next()?;
        AssignOperator::from_token(token.token.ty).ok_or(ParseError::unexpected(token.span))
    }

    /// Return true when a token index could be an infix or assign operator.
    #[inline]
    fn has_infix_or_assign_operator_at_index(&mut self, index: usize) -> bool {
        let token_type = self.token_type_at(index);
        if AssignOperator::from_token(token_type).is_some() {
            return true;
        }
        if token_type != TokenType::Identifier {
            return BinaryOperator::from_token("", token_type).is_some();
        }
        if self.has_active_split() {
            return false;
        }
        matches!(
            self.keyword_for_index(index),
            Some(
                Keyword::In
                    | Keyword::InstanceOf
                    | Keyword::As
                    | Keyword::Is
                    | Keyword::Satisfies
                    | Keyword::Extends
                    | Keyword::Implements
            )
        )
    }

    /// Peek an infix operator.
    #[inline]
    pub fn peek_infix_operator(&mut self) -> ParseResult<(InfixOperator, u8)> {
        let token = *self.peek()?;
        let next_token = *self.peek_next()?;
        let next_next_token = *self.peek_next_next()?;
        let token_str = self.get_span_str(token.span);
        to_infix_operator(
            token_str,
            &token,
            &next_token,
            &next_next_token,
            self.options,
            self.language,
            false,
        )
    }

    /// Peek a next infix operator.
    #[inline]
    pub fn peek_next_infix_operator(&mut self) -> ParseResult<(InfixOperator, u8)> {
        let token = *self.peek_next()?;
        let next_token = *self.peek_next_next()?;
        let next_next_token = *self.peek_next_next_next()?;
        let token_str = self.get_span_str(token.span);
        to_infix_operator(
            token_str,
            &token,
            &next_token,
            &next_next_token,
            self.options,
            self.language,
            true,
        )
    }

    /// Peek an infix operator after any newlines.
    #[inline]
    pub fn peek_infix_operator_after_newlines(&mut self) -> ParseResult<(InfixOperator, u8)> {
        let mut pos = self.pos() as usize;
        loop {
            self.token_stream.ensure_token(pos + 1);
            let Some(token) = self.tokens().get(pos + 1) else {
                break;
            };
            if token.token.ty != TokenType::Newline {
                break;
            }
            pos += 1;
        }
        let eof_span = self.eof_span();
        let token = *self
            .token_ref_at(pos + 1)
            .ok_or(ParseError::unexpected(eof_span))?;
        let next_token = self.token_at(pos + 2).unwrap_or(TokenSpan {
            token: Token::end(),
            span: self.eof_span(),
        });
        let next_next_token = self.token_at(pos + 3).unwrap_or(TokenSpan {
            token: Token::end(),
            span: self.eof_span(),
        });
        let token_str = self.get_span_str(token.span);
        to_infix_operator(
            token_str,
            &token,
            &next_token,
            &next_next_token,
            self.options,
            self.language,
            true,
        )
    }

    /// Check whether a parsed static argument list can be followed in expression position.
    #[inline]
    pub fn can_follow_type_arguments_in_expression(&mut self) -> bool {
        let index = if self.peek_is(TokenType::Newline) {
            self.next_non_newline_index_from(self.pos_index())
        } else {
            self.pos_index()
        };
        self.can_follow_type_arguments_at_index(index)
    }

    /// Check whether a static argument list can be followed by a specific token.
    fn can_follow_type_arguments_at_index(&mut self, index: usize) -> bool {
        let token_type = self.token_type_at(index);

        // allow end and static closers
        if token_type == TokenType::End {
            return true;
        }
        if self.options.in_static && token_type == TokenType::GreaterThan {
            return true;
        }

        // allow ternary and arrow continuations
        if self.options.in_ternary_condition && token_type == TokenType::Colon {
            return true;
        }
        if self.options.in_type && matches!(token_type, TokenType::Arrow | TokenType::ArrowWide) {
            return true;
        }

        // allow statement-start keywords after static args in new receivers
        if self.options.in_new_receiver
            && token_type == TokenType::Identifier
            && self.keyword_for_index(index).is_some()
        {
            return true;
        }

        // allow stops and delimiters
        if matches!(
            token_type,
            TokenType::Comma | TokenType::Semicolon | TokenType::Newline | TokenType::End
        ) {
            return true;
        }
        if matches!(
            token_type,
            TokenType::CloseParenthesis | TokenType::CloseBracket | TokenType::CloseBrace
        ) {
            return true;
        }
        if matches!(
            token_type,
            TokenType::OpenParenthesis | TokenType::OpenBracket | TokenType::Dot
        ) {
            return true;
        }
        if token_type == TokenType::Maybe {
            return true;
        }
        if matches!(
            token_type,
            TokenType::TemplateString | TokenType::TemplateStringStart
        ) {
            return true;
        }

        // allow heritage terminators after static arguments
        if self.options.in_super_type {
            if token_type == TokenType::OpenBrace {
                return true;
            }
            if token_type == TokenType::Identifier
                && matches!(
                    self.keyword_for_index(index),
                    Some(Keyword::Implements | Keyword::With | Keyword::Where)
                )
            {
                return true;
            }
        }

        if self.has_infix_or_assign_operator_at_index(index) {
            return true;
        }

        false
    }

    /// Check whether static arguments can be followed by an object literal.
    #[inline]
    fn can_follow_type_arguments_in_object_literal(&mut self) -> bool {
        self.peek_is(TokenType::OpenBrace)
    }

    /// Eat static arguments in expression position if the follow token allows it.
    fn eat_static_arguments_in_expression(
        &mut self,
        allow_object_literal: bool,
    ) -> Option<Vec<LocalNodeId<Argument>>> {
        if !self.peek_is(TokenType::LessThan) && !self.peek_is(TokenType::ShiftLeft) {
            return None;
        }

        // avoid path-attached static arguments in typescript expression positions
        if self.language.is_typescript()
            && !self.options.in_type
            && !self.options.in_decorator
            && !self.options.in_new_receiver
        {
            return None;
        }

        let speculative_start = self.mark();
        let speculative_start_idx = self.tree.next_id();
        match self.eat_static_arguments() {
            Ok(static_arguments) => {
                // in type or decorator context, type arguments are always valid
                if self.options.in_type || self.options.in_decorator {
                    return Some(static_arguments);
                }

                // validate that a follow token makes sense for a type argument list
                let mut can_follow = self.can_follow_type_arguments_in_expression();
                if allow_object_literal && self.can_follow_type_arguments_in_object_literal() {
                    can_follow = true;
                }
                if can_follow {
                    Some(static_arguments)
                } else {
                    self.restore(speculative_start, speculative_start_idx);
                    None
                }
            }
            Err(_err) => {
                self.restore(speculative_start, speculative_start_idx);
                None
            }
        }
    }

    /// Eat a TypeScript angle bracket type assertion.
    fn eat_type_assertion_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let operator_start = self.mark();
        let static_arguments = self.eat_static_arguments()?;
        let operator_span = self.get_span_from(&operator_start);

        // type assertions require exactly one positional type argument
        if static_arguments.len() != 1 {
            let unexpected_span = static_arguments
                .get(1)
                .map(|argument_id| self.tree.get_span(*argument_id))
                .unwrap_or(operator_span);
            return Err(ParseError::unexpected(unexpected_span));
        }

        // extract the asserted type expression
        let asserted_type = match self.tree.get(static_arguments[0]) {
            Argument::Positional {
                modifiers: None,
                value,
            } => *value,
            _ => {
                return Err(ParseError::unexpected(
                    self.tree.get_span(static_arguments[0]),
                ));
            }
        };

        // parse the asserted value expression
        let right_options = self
            .options
            .not_in_position()
            .in_left_precedence(TypeBinaryOperator::Cast.precedence());
        let asserted_value = self.with_options(right_options, |parser| parser.eat_expression())?;

        // lower to the same cast node used by `as`
        let expression_id = self.tree.insert(
            Expression::TypeBinary {
                left: asserted_value,
                operator: TypeBinaryOperator::Cast,
                right: asserted_type,
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(expression_id, operator_span);
        Ok(expression_id)
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
            InfixOperator::TypeBinary(type_binary_operator) => {
                if self.options.in_type
                    && type_binary_operator == TypeBinaryOperator::Is
                    && let Some(subject) = self.type_predicate_subject_from_expression(left)
                {
                    return Expression::TypePredicate {
                        asserts: false,
                        subject,
                        target: Some(right),
                    };
                }
                Expression::TypeBinary {
                    left,
                    operator: type_binary_operator,
                    right,
                }
            }
            InfixOperator::Assign(assign_operator) => Expression::Assign {
                left,
                operator: assign_operator,
                right,
            },
        }
    }

    /// Extract the subject and constraint for a type conditional.
    /// Pull union and intersection chains into the right side when they wrap `extends`.
    /// #Architecture: split_type_conditional_operands is localized reassociation for conditional types.
    /// Since we parse type expressions and expressions in the same pass, we have to post patch type precedence.
    fn split_type_conditional_operands(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<(LocalNodeId<Expression>, LocalNodeId<Expression>)> {
        let (operator, left_id, _right_id) = match self.tree.get(expression_id) {
            Expression::TypeBinary {
                operator: TypeBinaryOperator::Extends,
                left,
                right,
            } => {
                return Some((*left, *right));
            }
            Expression::Binary {
                operator,
                left,
                right,
            } => (*operator, *left, *right),
            _ => return None,
        };

        // only normalize union and intersection chains
        if !matches!(
            operator,
            BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
        ) {
            return None;
        }

        // peel a left leaning extends and reattach the union or intersection on the right
        let (extends_left, extends_right) = self.split_type_conditional_operands(left_id)?;
        if let Expression::Binary { left, .. } = self.tree.get_mut(expression_id) {
            *left = extends_right;
        }

        Some((extends_left, expression_id))
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
                let start = ParserMark::from_span(span);
                self.try_recover(&start, recover, Some(err))?;
                let error_id = self
                    .tree
                    .insert(Expression::Error, self.get_span_from(&start));
                Ok(error_id)
            }
        }
    }

    /// Return true when a token can appear as a static member name after `.`.
    #[inline]
    fn token_is_member_name(token: &TokenSpan) -> bool {
        token.token.ty == TokenType::Identifier
            || matches!(token.token.literal, Some(LiteralType::Boolean { .. }))
    }

    /// Return true when js or ts sees decimal integer member access without a separator.
    #[inline]
    fn invalid_decimal_integer_member_access(
        &mut self,
        left_expression_id: LocalNodeId<Expression>,
        distance: u8,
    ) -> bool {
        if !(self.language.is_javascript() || self.language.is_typescript()) {
            return false;
        }
        if distance != 2 {
            return false;
        }

        if !matches!(
            self.tree.get(left_expression_id),
            Expression::ScalarLiteral(destack_ast::ScalarLiteral::Integer(_))
        ) {
            return false;
        }

        let left_span = self.tree.get_span(left_expression_id);
        let Some(dot_token) = self.prev().copied() else {
            return false;
        };
        if dot_token.token.ty != TokenType::Dot {
            return false;
        }
        if left_span.end != dot_token.span.start {
            return false;
        }

        let literal = self.file.span_str(left_span);
        let is_non_decimal_prefix = literal.starts_with("0x")
            || literal.starts_with("0X")
            || literal.starts_with("0o")
            || literal.starts_with("0O")
            || literal.starts_with("0b")
            || literal.starts_with("0B");
        if is_non_decimal_prefix {
            return false;
        }

        true
    }

    /// Eat a static member name and return both the name and its span.
    #[inline]
    fn eat_member_name_with_span(&mut self) -> ParseResult<(StringId, destack_source::Span)> {
        // identifier member name
        if self.peek_is(TokenType::Identifier) {
            self.eat_identifier_with_span()
        }
        // boolean literal member name
        else if self.peek_is(TokenType::Literal)
            && self
                .peek()
                .is_ok_and(|token| matches!(token.token.literal, Some(LiteralType::Boolean { .. })))
        {
            let token = *self.eat()?;
            let text = self.get_token_str(token).to_owned();
            let name = self.strings.intern(&text);
            Ok((name, token.span))
        }
        // invalid member name
        else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Peek a member access with an IdentifierName compatible token.
    /// Returns the total distance to eat (including the newlines, dot, and token).
    #[inline]
    fn peek_member_name(&mut self) -> ParseResult<u8> {
        // immediate member access
        if self.peek_is(TokenType::Dot) && self.peek_next().is_ok_and(Self::token_is_member_name) {
            Ok(2)
        }
        // member access across newline
        else {
            let base = self.pos() as usize;
            let mut offset = 0;
            let mut newline_count = 0;
            loop {
                self.token_stream.ensure_token(base + offset);
                let Some(token) = self.tokens().get(base + offset) else {
                    break;
                };
                if token.token.ty != TokenType::Newline {
                    break;
                }
                newline_count += 1;
                offset += 1;
            }
            self.token_stream.ensure_token(base + offset + 1);
            let is_member = newline_count > 0
                && matches!(
                    self.tokens().get(base + offset),
                    Some(token) if token.token.ty == TokenType::Dot
                )
                && self
                    .tokens()
                    .get(base + offset + 1)
                    .is_some_and(Self::token_is_member_name);
            if is_member {
                let distance = newline_count + 2;
                let distance = u8::try_from(distance).unwrap_or(u8::MAX);
                return Ok(distance);
            }

            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Check whether `?.` starts an optional chaining segment.
    #[inline]
    fn is_optional_chain_after_maybe(&mut self) -> bool {
        // require ?. before we look at the target
        if !self.peek_next_is(TokenType::Dot) {
            return false;
        }

        // accept valid optional chain targets after ?.
        let next_next_token_type = self.token_type_at(self.pos() as usize + 2);
        matches!(
            next_next_token_type,
            TokenType::Identifier
                | TokenType::OpenBracket
                | TokenType::OpenParenthesis
                | TokenType::Hash
                | TokenType::LessThan
                | TokenType::ShiftLeft
                | TokenType::TemplateStringStart
                | TokenType::TemplateString
        )
    }

    /// Return true when optional chaining starts after one or more newlines.
    #[inline]
    fn optional_chain_starts_after_newlines(&mut self) -> bool {
        if !self.peek_is(TokenType::Newline) {
            return false;
        }

        let next_index = self.next_non_newline_index_from(self.pos_index());
        self.token_stream.ensure_token(next_index + 1);
        let Some(next_token) = self.tokens().get(next_index) else {
            return false;
        };

        if next_token.token.ty == TokenType::Maybe {
            return true;
        }

        next_token.token.ty == TokenType::Dot
            && self
                .tokens()
                .get(next_index + 1)
                .is_some_and(|token| token.token.ty == TokenType::Maybe)
    }

    /// Check whether `asserts` starts a type predicate.
    #[inline]
    fn can_start_type_predicate_asserts(&mut self) -> bool {
        if self.peek_keyword(Keyword::Asserts).is_err() {
            return false;
        }

        let mut pos = self.pos() as usize;
        loop {
            self.token_stream.ensure_token(pos + 1);
            let Some(token) = self.tokens().get(pos + 1) else {
                break;
            };
            if token.token.ty != TokenType::Newline {
                break;
            }
            pos += 1;
        }

        if self.keyword_for_index(pos + 1) == Some(Keyword::This) {
            return true;
        }

        self.tokens()
            .get(pos + 1)
            .is_some_and(|token| token.token.ty == TokenType::Identifier)
    }

    /// Peek a private member access using `.#`.
    #[inline]
    fn peek_private_member(&mut self) -> ParseResult<u8> {
        // immediate private member access
        if self.peek_is(TokenType::Dot)
            && self.peek_next_is(TokenType::Hash)
            && self.peek_next_next_token(TokenType::Identifier).is_ok()
        {
            // require the hash and identifier to be adjacent
            let hash_index = self.pos_index() + 1;
            let ident_index = self.pos_index() + 2;
            self.check_tokens_are_adjacent(hash_index, ident_index)?;

            Ok(3)
        }
        // private member access across newline
        else {
            let base = self.pos() as usize;
            let mut offset = 0;
            let mut newline_count = 0;
            loop {
                self.token_stream.ensure_token(base + offset);
                let Some(token) = self.tokens().get(base + offset) else {
                    break;
                };
                if token.token.ty != TokenType::Newline {
                    break;
                }
                newline_count += 1;
                offset += 1;
            }
            self.token_stream.ensure_token(base + offset + 1);
            let is_member = newline_count > 0
                && matches!(
                    self.tokens().get(base + offset),
                    Some(token) if token.token.ty == TokenType::Dot
                )
                && matches!(
                    self.tokens().get(base + offset + 1),
                    Some(token) if token.token.ty == TokenType::Hash
                )
                && matches!(
                    self.tokens().get(base + offset + 2),
                    Some(token) if token.token.ty == TokenType::Identifier
                );
            if is_member {
                // require the hash and identifier to be adjacent
                let hash_index = base + offset + 1;
                let ident_index = base + offset + 2;
                self.check_tokens_are_adjacent(hash_index, ident_index)?;

                let distance = newline_count + 3;
                let distance = u8::try_from(distance).unwrap_or(u8::MAX);
                return Ok(distance);
            }

            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Eat an expression that might be paranthesized (skip the parenthesis if present).
    pub fn eat_expression_parenthesized_maybe(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();
        if self.peek_is(TokenType::OpenParenthesis) {
            self.bump(); // eat open parenthesis
            self.eat_newlines_maybe()?;
            let expression_id = self.eat_expression()?;
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseParenthesis)?;
            self.tree
                .set_span(expression_id, self.get_span_from(&start));
            Ok(expression_id)
        } else {
            self.eat_expression()
        }
    }

    /// Check whether a `{` in statement position should be parsed as an object literal.
    /// NOTE #Cleanup: can_parse_object_literal_in_statement_position is ugly and might not be fixable.
    fn can_parse_object_literal_in_statement_position(&mut self) -> bool {
        // only allow this in destack files
        if !self.language.is_destack() {
            return false;
        }

        // avoid object literals when a block is expected
        if self.options.in_before_block {
            return false;
        }

        // skip newlines after the opening brace
        let open_pos = match self.skip_newlines(self.pos()) {
            Ok(pos) => pos,
            Err(_) => return false,
        };
        self.token_stream.ensure_token(open_pos as usize + 1);
        let Some(next_token) = self.tokens().get(open_pos as usize + 1) else {
            return false;
        };

        // spread property start
        if next_token.token.ty == TokenType::Spread {
            return true;
        }

        // computed key start: require a clear property marker after the closing bracket
        if next_token.token.ty == TokenType::OpenBracket {
            let open_bracket_pos = open_pos + 1;
            let close_bracket_pos = match self.find_matching_close(
                Some(open_bracket_pos),
                TokenType::OpenBracket,
                TokenType::CloseBracket,
            ) {
                Ok(pos) => pos,
                Err(_) => return false,
            };

            let after_close_pos = match self.skip_newlines(close_bracket_pos) {
                Ok(pos) => pos,
                Err(_) => return false,
            };
            self.token_stream.ensure_token(after_close_pos as usize + 1);
            let Some(after_close) = self.tokens().get(after_close_pos as usize + 1) else {
                return false;
            };

            return matches!(after_close.token.ty, TokenType::Colon | TokenType::Maybe);
        }

        // identifier or literal key with an explicit value marker
        if next_token.token.ty == TokenType::Identifier || next_token.token.ty == TokenType::Literal
        {
            // only allow string or number literal keys
            if next_token.token.ty == TokenType::Literal {
                let literal = next_token.token.literal;
                let is_key_literal = matches!(
                    literal,
                    Some(
                        LiteralType::String { .. }
                            | LiteralType::Int { .. }
                            | LiteralType::Float { .. }
                    )
                );
                if !is_key_literal {
                    return false;
                }
            }

            // check for a colon or optional marker after the key
            let key_pos = match self.skip_newlines(open_pos + 1) {
                Ok(pos) => pos,
                Err(_) => return false,
            };
            self.token_stream.ensure_token(key_pos as usize + 1);
            let Some(after_key) = self.tokens().get(key_pos as usize + 1) else {
                return false;
            };

            return matches!(after_key.token.ty, TokenType::Colon | TokenType::Maybe);
        }

        false
    }

    /// Check whether a using declaration can be parsed at the current position.
    fn can_parse_using_declaration(
        &mut self,
        descriptor: &DeclarationDescriptor,
        asynchrony: Asynchrony,
    ) -> bool {
        // speculatively parse a using declaration
        let speculative_start = self.mark();
        let speculative_start_idx = self.tree.next_id();
        let result = self
            .eat_using(&speculative_start, descriptor.clone(), asynchrony)
            .is_ok();
        self.restore(speculative_start, speculative_start_idx);
        result
    }

    /// Eat declaration modifiers and return a descriptor or a parsed expression.
    fn eat_declaration_descriptor(&mut self, start: &ParserMark) -> ParseResult<DescriptorHead> {
        let mut descriptor: DeclarationDescriptor = DeclarationDescriptor::default();

        // decorators parse as expressions only
        if self.options.in_decorator {
            return Ok(DescriptorHead::Descriptor(descriptor));
        }

        // declaration modifiers only start on identifiers
        if !self.peek_is(TokenType::Identifier) {
            return Ok(DescriptorHead::Descriptor(descriptor));
        }

        // check for a modifier keyword or a global or module identifier
        let pos = self.pos_index();
        let keyword = if self.has_active_split() {
            self.peek_any_keyword().ok()
        } else {
            self.keyword_for_index(pos)
        };
        let is_modifier_keyword = matches!(
            keyword,
            Some(Keyword::Export | Keyword::Declare | Keyword::Abstract | Keyword::Static)
        );
        let is_global_identifier = self
            .global_identifier
            .is_some_and(|id| self.identifier_for_index(pos) == Some(id));
        let is_module_identifier = self.language.supports_module_declaration()
            && self
                .module_identifier
                .is_some_and(|id| self.identifier_for_index(pos) == Some(id));
        if !is_modifier_keyword && !is_global_identifier && !is_module_identifier {
            return Ok(DescriptorHead::Descriptor(descriptor));
        }

        // export modifier
        if self.peek_keyword(Keyword::Export).is_ok() {
            self.bump(); // eat export
            let export_mode = if self.peek_keyword(Keyword::Default).is_ok() {
                self.bump(); // eat default
                Some(DependencyMode::Default)
            } else if self.peek_is(TokenType::Assign) {
                self.bump(); // eat assign
                Some(DependencyMode::Namespace)
            } else {
                Some(DependencyMode::Item)
            };

            // export namespace handled by export statement parsing
            let is_export_namespace = self.peek_keyword(Keyword::As).is_ok()
                && self.peek_next_keyword(Keyword::Namespace).is_ok();
            if is_export_namespace {
                self.rewind(start.clone());
                let export = self.eat_export()?;
                return Ok(DescriptorHead::Expression(export));
            }

            // export dependencies handled by export statement parsing
            let next_keyword = self.peek_any_keyword().ok();
            let has_module_identifier_declaration = self.language.supports_module_declaration()
                && self.peek_identifier_str("module").is_ok();
            let has_declaration_keyword = next_keyword
                .is_some_and(|kw| DECLARATION_KEYWORDS.contains(&kw))
                || has_module_identifier_declaration;

            // reject export default enum declarations
            if export_mode == Some(DependencyMode::Default)
                && self.peek_keyword(Keyword::Enum).is_ok()
            {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let is_export_type_binding = self.peek_keyword(Keyword::Type).is_ok()
                && (self.peek_next_is(TokenType::OpenBrace)
                    || self.peek_next_is(TokenType::Multiply)
                    || self.peek_next_is(TokenType::Semicolon)
                    || self.peek_next_is(TokenType::Newline)
                    || self.peek_next_is(TokenType::End));
            let is_invalid_export_form = !has_declaration_keyword
                && !self.peek_is(TokenType::At)
                && self.peek_dependency_binding().is_err()
                && self.peek_keyword_after_newlines(Keyword::Import).is_err();
            let is_export_dependency = export_mode == Some(DependencyMode::Namespace)
                || is_export_type_binding
                || (!has_declaration_keyword && self.peek_dependency_binding().is_ok())
                || (export_mode == Some(DependencyMode::Default) && !has_declaration_keyword)
                || is_invalid_export_form;
            if is_export_dependency {
                self.rewind(start.clone());
                let export = self.eat_export()?;
                return Ok(DescriptorHead::Expression(export));
            }

            descriptor.export = export_mode;

            // allow decorators after export modifier
            if self.peek_is(TokenType::At) {
                self.eat_decorators_prefix_maybe()?;
            }
        }

        // skip newlines before export import equals
        if descriptor.export.is_some()
            && self.peek_is(TokenType::Newline)
            && self.peek_keyword_after_newlines(Keyword::Import).is_ok()
        {
            self.eat_newlines_maybe()?;
        }

        // declare modifier
        let is_declare = self.peek_keyword(Keyword::Declare).is_ok();
        let direct_index = self.pos_index() + 1;

        // locate a declare target
        let declare_target_index = if is_declare {
            if self.is_declare_target_at(direct_index) {
                Some(direct_index)
            } else if self.keyword_for_index(direct_index) == Some(Keyword::Abstract) {
                let after_abstract = direct_index + 1;
                let target_index = self.next_non_newline_index_from(after_abstract);
                if target_index == after_abstract && self.is_declare_target_at(target_index) {
                    Some(target_index)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // report newline errors for declare forms that must be contiguous
        let declare_newline_error_span =
            if is_declare && self.keyword_for_index(direct_index) == Some(Keyword::Abstract) {
                let after_abstract = direct_index + 1;
                let target_index = self.next_non_newline_index_from(after_abstract);
                if target_index > after_abstract && self.is_declare_target_at(target_index) {
                    self.token_stream.ensure_token(after_abstract);
                    self.tokens().get(after_abstract).map(|token| token.span)
                } else {
                    None
                }
            } else if is_declare && self.keyword_for_index(direct_index) == Some(Keyword::Type) {
                let after_type = direct_index + 1;
                let name_index = self.next_non_newline_index_from(after_type);
                if name_index > after_type
                    && self
                        .tokens()
                        .get(name_index)
                        .is_some_and(|token| token.token.ty == TokenType::Identifier)
                {
                    self.token_stream.ensure_token(after_type);
                    self.tokens().get(after_type).map(|token| token.span)
                } else {
                    None
                }
            } else {
                None
            };

        if let Some(span) = declare_newline_error_span {
            let error = ParseError::unexpected(span);
            self.error(&error);
        }

        let declare_has_target = declare_target_index.is_some();
        descriptor.kind = if is_declare && declare_has_target {
            self.bump(); // eat declare
            DeclarationKind::Declaration
        } else {
            DeclarationKind::Definition
        };

        // abstraction modifier
        descriptor.abstraction = if self.peek_keyword(Keyword::Abstract).is_ok()
            && !self.options.in_variant
            && !self.peek_next_is(TokenType::Newline)
            && self
                .peek_next_any_keyword()
                .is_ok_and(|kw| DECLARATION_KEYWORDS.contains(&kw))
        {
            self.bump(); // eat abstract
            DeclarationAbstraction::Abstract
        } else {
            DeclarationAbstraction::Concrete
        };

        // anchor modifier
        descriptor.anchor = if self.peek_keyword(Keyword::Static).is_ok() {
            self.bump(); // eat static
            BindingAnchor::Static
        } else {
            BindingAnchor::Instance
        };

        // global declaration
        if (descriptor.kind == DeclarationKind::Declaration
            || self.language.is_declaration()
            || self.options.in_declare_context)
            && self.peek_identifier_str("global").is_ok()
            && self
                .peek_token_after_newlines(self.pos(), TokenType::OpenBrace)
                .is_ok()
        {
            let mut global_descriptor = descriptor;
            if global_descriptor.kind == DeclarationKind::Definition {
                // global declarations are always declarations
                global_descriptor.kind = DeclarationKind::Declaration;
            }
            let global_id = self.eat_global(start, global_descriptor)?;
            let expression_id = self.tree.insert(
                Expression::Declaration(global_id),
                self.get_span_from(start),
            );
            return Ok(DescriptorHead::Expression(expression_id));
        }

        Ok(DescriptorHead::Descriptor(descriptor))
    }

    /// Check whether a token index starts a declare target keyword.
    fn is_declare_keyword_target_at(&mut self, index: usize) -> bool {
        let keyword = self.keyword_for_index(index);
        keyword.is_some_and(|kw| kw != Keyword::Declare && DECLARATION_KEYWORDS.contains(&kw))
    }

    /// Check whether a token index starts a declare identifier target.
    fn is_declare_identifier_at(&mut self, index: usize) -> bool {
        self.token_stream.ensure_token(index);
        let Some(token) = self.tokens().get(index) else {
            return false;
        };
        if token.token.ty != TokenType::Identifier {
            return false;
        }
        let token_str = self.get_span_str(token.span);
        token_str == "global"
            || self.language.supports_module_declaration() && token_str == "module"
    }

    /// Check whether a token index starts a declare await using target.
    fn is_declare_await_using_at(&mut self, index: usize) -> bool {
        if self.keyword_for_index(index) != Some(Keyword::Await) {
            return false;
        }
        let mut after = index + 1;
        loop {
            self.token_stream.ensure_token(after);
            let Some(token) = self.tokens().get(after) else {
                break;
            };
            if token.token.ty != TokenType::Newline {
                break;
            }
            after += 1;
        }
        self.keyword_for_index(after) == Some(Keyword::Using)
    }

    /// Check whether a token index starts a declare target.
    fn is_declare_target_at(&mut self, index: usize) -> bool {
        self.is_declare_keyword_target_at(index)
            || self.is_declare_identifier_at(index)
            || self.is_declare_await_using_at(index)
    }

    /// Return true when the current keyword is followed by a matching member.
    fn keyword_member_access_is(
        &mut self,
        member_name: &str,
        allow_newlines: bool,
    ) -> ParseResult<bool> {
        // require dot member access
        let dot_index = if allow_newlines {
            if self
                .peek_token_after_newlines(self.pos(), TokenType::Dot)
                .is_err()
            {
                return Ok(false);
            }
            self.next_non_newline_index_from(self.pos_index() + 1)
        } else {
            if !self.peek_next_is(TokenType::Dot) {
                return Ok(false);
            }
            self.index_for_next()
        };

        // require identifier member
        let identifier_index = if allow_newlines {
            self.next_non_newline_index_from(dot_index + 1)
        } else {
            dot_index + 1
        };
        self.token_stream.ensure_token(identifier_index);
        let Some(identifier_token) = self.tokens().get(identifier_index).copied() else {
            return Err(ParseError::unexpected(self.peek()?.span));
        };
        if identifier_token.token.ty != TokenType::Identifier {
            return Err(ParseError::unexpected(identifier_token.span));
        }

        // match the member name from cached interned identifiers when available
        let matches_member_name = self
            .identifier_for_index(identifier_index)
            .is_some_and(|identifier| self.strings.get(identifier) == member_name)
            || self.get_span_str(identifier_token.span) == member_name;
        if matches_member_name {
            Ok(true)
        } else {
            Err(ParseError::unexpected(identifier_token.span))
        }
    }

    /// Eat a keyword-led expression when possible.
    fn eat_keyword_expression(
        &mut self,
        start: &ParserMark,
        descriptor: DeclarationDescriptor,
        keyword: Keyword,
        next_token_type: TokenType,
        is_declaration_start: bool,
    ) -> ParseResult<Option<LocalNodeId<Expression>>> {
        match keyword {
            // namespace declaration
            Keyword::Namespace if is_declaration_start => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let namespace_id = self.eat_namespace(start, descriptor)?;
                Ok(Some(self.tree.insert(
                    Expression::Declaration(namespace_id),
                    self.get_span_from(start),
                )))
            }
            // struct declaration
            Keyword::Struct
                if self.language.is_destack()
                    && (is_declaration_start || next_token_type == TokenType::Newline) =>
            {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let allow_anonymous_class = !self.options.in_statement_position
                    || descriptor.export == Some(DependencyMode::Default);
                let struct_id =
                    self.eat_struct_or_class(start, descriptor, allow_anonymous_class)?;
                Ok(Some(self.tree.insert(
                    Expression::Declaration(struct_id),
                    self.get_span_from(start),
                )))
            }
            // class declaration
            Keyword::Class if is_declaration_start || next_token_type == TokenType::Newline => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let allow_anonymous_class = !self.options.in_statement_position
                    || descriptor.export == Some(DependencyMode::Default);
                let struct_id =
                    self.eat_struct_or_class(start, descriptor, allow_anonymous_class)?;
                Ok(Some(self.tree.insert(
                    Expression::Declaration(struct_id),
                    self.get_span_from(start),
                )))
            }
            // enum declaration
            Keyword::Enum if is_declaration_start => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let enum_id = self.eat_enum(start, EnumKind::Enum, descriptor)?;
                Ok(Some(self.tree.insert(
                    Expression::Declaration(enum_id),
                    self.get_span_from(start),
                )))
            }
            // const enum or binding declaration
            Keyword::Const => {
                // look ahead for const enum
                let next_keyword = if next_token_type == TokenType::Identifier {
                    self.keyword_for_index(self.index_for_next())
                } else {
                    None
                };

                // parse const enum declaration
                if next_keyword == Some(Keyword::Enum) {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                    self.eat_keyword(Keyword::Const)?;
                    let enum_id = self.eat_enum(start, EnumKind::Const, descriptor)?;
                    Ok(Some(self.tree.insert(
                        Expression::Declaration(enum_id),
                        self.get_span_from(start),
                    )))
                // otherwise parse binding declaration
                } else {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_BINDING);
                    Ok(Some(self.eat_let(start, descriptor)?))
                }
            }
            // newtype interface or alias declaration
            Keyword::Newtype => {
                // look ahead for newtype interface
                let next_keyword = if next_token_type == TokenType::Identifier {
                    self.keyword_for_index(self.index_for_next())
                } else {
                    None
                };

                // parse newtype interface declaration
                if next_keyword == Some(Keyword::Interface) {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                    self.eat_keyword(Keyword::Newtype)?;
                    let interface_id = self.eat_interface(start, descriptor, TypeKind::Nominal)?;
                    Ok(Some(self.tree.insert(
                        Expression::Declaration(interface_id),
                        self.get_span_from(start),
                    )))
                // otherwise parse newtype alias declaration
                } else {
                    // require a valid type alias start
                    let can_start_type_alias = matches!(
                        next_token_type,
                        TokenType::Identifier
                            | TokenType::OpenBrace
                            | TokenType::OpenParenthesis
                            | TokenType::OpenBracket
                            | TokenType::Literal
                    );

                    // parse the alias when it can start
                    if can_start_type_alias {
                        let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                        Ok(Some(self.eat_type(start, descriptor)?))
                    // otherwise bail
                    } else {
                        Ok(None)
                    }
                }
            }
            // interface declaration
            Keyword::Interface if is_declaration_start || next_token_type == TokenType::Newline => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let interface_id = self.eat_interface(start, descriptor, TypeKind::Structural)?;
                Ok(Some(self.tree.insert(
                    Expression::Declaration(interface_id),
                    self.get_span_from(start),
                )))
            }
            // extension declaration
            Keyword::Extension
                if self.language.is_destack()
                    && self.options.in_statement_position
                    && is_declaration_start =>
            {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let extension_id = self.eat_extension(start, descriptor)?;
                Ok(Some(self.tree.insert(
                    Expression::Declaration(extension_id),
                    self.get_span_from(start),
                )))
            }
            // async declaration or async path
            Keyword::Async => {
                // avoid async generic parses when tree literal disambiguation is active
                if self.language.is_typescript()
                    && self.options.left_precedence.is_some()
                    && (self.peek_next_is(TokenType::LessThan)
                        || self.peek_next_is(TokenType::ShiftLeft))
                {
                    return Ok(None);
                }

                // require a valid function signature start
                let can_start_signature = matches!(
                    next_token_type,
                    TokenType::Identifier
                        | TokenType::OpenParenthesis
                        | TokenType::LessThan
                        | TokenType::At
                        | TokenType::Multiply
                );
                if !can_start_signature {
                    return Ok(None);
                }

                // parse async function with speculative rollback
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let speculative_start = self.mark();
                let speculative_start_idx = self.tree.next_id();
                if let Ok(function_id) = self.eat_function(start, descriptor, false, false) {
                    // accept only when not a rejected lambda signature
                    let should_accept = match self.tree.get(function_id) {
                        Declaration::Function {
                            signature, body, ..
                        } => {
                            !(signature.kind == FunctionKind::Lambda
                                && body.is_none()
                                && !self.options.in_type)
                        }
                        _ => true,
                    };

                    // accept the parsed function
                    if should_accept {
                        Ok(Some(self.tree.insert(
                            Expression::Declaration(function_id),
                            self.get_span_from(start),
                        )))
                    // otherwise fall back to async path
                    } else {
                        self.restore(speculative_start, speculative_start_idx);
                        let (path, last_span) = self
                            .eat_path_with_last_span()
                            .for_node_type(NodeType::Expression)?;
                        let static_arguments = self.eat_static_arguments_in_expression(false);

                        // parse call when static arguments apply
                        if static_arguments.is_some()
                            && self.peek_is(TokenType::OpenParenthesis)
                            && !self.options.in_new_receiver
                        {
                            let receiver = Expression::Path {
                                path,
                                static_arguments: None,
                            };
                            let receiver_id = self.tree.insert(receiver, self.get_span_from(start));
                            self.tree.set_main_span(receiver_id, last_span);
                            Ok(Some(self.eat_call(
                                receiver_id,
                                static_arguments,
                                PostfixPosition::Direct,
                            )?))
                        // otherwise build a path expression
                        } else {
                            let expression = Expression::Path {
                                path,
                                static_arguments,
                            };
                            let expression_id =
                                self.tree.insert(expression, self.get_span_from(start));
                            self.tree.set_main_span(expression_id, last_span);
                            Ok(Some(expression_id))
                        }
                    }
                // fall back to async path when signature parsing failed
                } else {
                    self.restore(speculative_start, speculative_start_idx);
                    let (path, last_span) = self
                        .eat_path_with_last_span()
                        .for_node_type(NodeType::Expression)?;
                    let static_arguments = self.eat_static_arguments_in_expression(false);

                    // parse call when static arguments apply
                    if static_arguments.is_some()
                        && self.peek_is(TokenType::OpenParenthesis)
                        && !self.options.in_new_receiver
                    {
                        let receiver = Expression::Path {
                            path,
                            static_arguments: None,
                        };
                        let receiver_id = self.tree.insert(receiver, self.get_span_from(start));
                        self.tree.set_main_span(receiver_id, last_span);
                        Ok(Some(self.eat_call(
                            receiver_id,
                            static_arguments,
                            PostfixPosition::Direct,
                        )?))
                    }
                    // otherwise build a path expression
                    else {
                        let expression = Expression::Path {
                            path,
                            static_arguments,
                        };
                        let expression_id = self.tree.insert(expression, self.get_span_from(start));
                        self.tree.set_main_span(expression_id, last_span);
                        Ok(Some(expression_id))
                    }
                }
            }
            // function or method declaration
            Keyword::Function | Keyword::Abstract | Keyword::Override => {
                // require a valid function signature start
                let can_start_signature = matches!(
                    next_token_type,
                    TokenType::Identifier
                        | TokenType::OpenParenthesis
                        | TokenType::LessThan
                        | TokenType::At
                        | TokenType::Multiply
                );
                if !can_start_signature {
                    return Ok(None);
                }

                // parse function declaration
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let function_id = self.eat_function(start, descriptor, false, false)?;
                Ok(Some(self.tree.insert(
                    Expression::Declaration(function_id),
                    self.get_span_from(start),
                )))
            }
            // new signature declaration in type positions
            Keyword::New if self.options.in_type => {
                // require a valid function signature start
                let can_start_signature = matches!(
                    next_token_type,
                    TokenType::Identifier
                        | TokenType::OpenParenthesis
                        | TokenType::LessThan
                        | TokenType::At
                        | TokenType::Multiply
                );
                if can_start_signature {
                    // parse constructor signature
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                    let function_id = self.eat_function(start, descriptor, false, false)?;
                    Ok(Some(self.tree.insert(
                        Expression::Declaration(function_id),
                        self.get_span_from(start),
                    )))
                // otherwise bail
                } else {
                    Ok(None)
                }
            }
            // variant method declaration
            Keyword::Get | Keyword::Set | Keyword::Constructor if self.options.in_variant => {
                // require a valid function signature start
                let can_start_signature = matches!(
                    next_token_type,
                    TokenType::Identifier
                        | TokenType::OpenParenthesis
                        | TokenType::LessThan
                        | TokenType::At
                        | TokenType::Multiply
                );
                if !can_start_signature {
                    return Ok(None);
                }

                // parse variant method declaration
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                let function_id = self.eat_function(start, descriptor, false, false)?;
                Ok(Some(self.tree.insert(
                    Expression::Declaration(function_id),
                    self.get_span_from(start),
                )))
            }
            // this expression
            Keyword::This => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                self.bump(); // eat this
                Ok(Some(
                    self.tree
                        .insert(Expression::This, self.get_span_from(start)),
                ))
            }
            // super expression
            Keyword::Super => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                self.bump(); // eat super
                Ok(Some(
                    self.tree
                        .insert(Expression::Super, self.get_span_from(start)),
                ))
            }
            // new expression
            Keyword::New if !self.options.in_type => {
                // require a valid new expression start
                let can_start_new_expression = matches!(
                    next_token_type,
                    TokenType::Identifier
                        | TokenType::OpenParenthesis
                        | TokenType::OpenBrace
                        | TokenType::LessThan
                );
                if can_start_new_expression {
                    // parse new expression
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                    Ok(Some(self.eat_new()?))
                }
                // allow `new.target` to fall back to path parsing
                else if next_token_type == TokenType::Dot {
                    if self.keyword_member_access_is("target", false)? {
                        Ok(None)
                    } else {
                        Err(ParseError::unexpected(self.peek()?.span))
                    }
                }
                // otherwise reject `new` in expression position
                else {
                    Err(ParseError::unexpected(self.peek()?.span))
                }
            }
            // delete expression
            Keyword::Delete if next_token_type != TokenType::Colon => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                Ok(Some(self.eat_delete()?))
            }
            // type only import expression
            Keyword::Import
                if self.options.in_type && next_token_type == TokenType::OpenParenthesis =>
            {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DEPENDENCY);
                Ok(Some(self.eat_type_import_expression()?))
            }
            // import call expression
            Keyword::Import if next_token_type == TokenType::OpenParenthesis => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DEPENDENCY);
                Ok(Some(self.eat_import_call_expression(start)?))
            }
            // import declaration or import meta
            Keyword::Import => {
                // treat `import.meta` as a path and reject other member access
                if self
                    .peek_token_after_newlines(self.pos(), TokenType::Dot)
                    .is_ok()
                {
                    if self.keyword_member_access_is("meta", true)? {
                        return Ok(None);
                    }
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                // require a valid import start
                let can_start_import = matches!(
                    next_token_type,
                    TokenType::Multiply
                        | TokenType::Identifier
                        | TokenType::OpenBrace
                        | TokenType::Literal
                ) || self.can_start_import_statement();
                if !can_start_import {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                // parse import or export import equals declaration
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_DEPENDENCY);
                if descriptor.export.is_some() && self.peek_import_equals_after_import() {
                    Ok(Some(self.eat_export_import_equals(start, descriptor)?))
                } else {
                    Ok(Some(self.eat_import()?))
                }
            }
            // infer type expression
            Keyword::Infer if self.options.in_type => Ok(Some(self.eat_type_infer_expression()?)),
            // asserts type predicate
            Keyword::Asserts if self.options.in_type => {
                // allow asserts predicate only when grammar supports it
                if self.can_start_type_predicate_asserts() {
                    Ok(Some(self.eat_type_predicate_asserts()?))
                // otherwise bail
                } else {
                    Ok(None)
                }
            }
            // let or var binding declaration
            Keyword::Let | Keyword::Var => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_BINDING);
                Ok(Some(self.eat_let(start, descriptor)?))
            }
            // using declaration
            Keyword::Using => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_BINDING);
                if self.can_parse_using_declaration(&descriptor, Asynchrony::Sync) {
                    Ok(Some(self.eat_using(start, descriptor, Asynchrony::Sync)?))
                // otherwise bail
                } else {
                    Ok(None)
                }
            }
            // type or readonly type alias declaration
            Keyword::Type | Keyword::Readonly => {
                let next_keyword = if next_token_type == TokenType::Identifier {
                    self.keyword_for_index(self.index_for_next())
                } else {
                    None
                };
                let next_index = self.index_for_next();
                let after_next_index = self.next_non_newline_index_from(next_index + 1);
                let after_next_token_type = self.token_type_at(after_next_index);
                let starts_type_operator = matches!(
                    next_keyword,
                    Some(
                        Keyword::As
                            | Keyword::Satisfies
                            | Keyword::Extends
                            | Keyword::Implements
                            | Keyword::In
                            | Keyword::InstanceOf
                            | Keyword::Is
                    )
                ) && !matches!(
                    after_next_token_type,
                    TokenType::Assign
                        | TokenType::LessThan
                        | TokenType::ShiftLeft
                        | TokenType::SaturatingShiftLeft
                );

                // require a valid type alias start
                let can_start_type_alias = matches!(
                    next_token_type,
                    TokenType::Identifier
                        | TokenType::OpenBrace
                        | TokenType::OpenParenthesis
                        | TokenType::OpenBracket
                        | TokenType::Literal
                ) && !starts_type_operator;

                // parse type alias when it can start
                if can_start_type_alias {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_DECLARATION);
                    Ok(Some(self.eat_type(start, descriptor)?))
                // otherwise bail
                } else {
                    Ok(None)
                }
            }
            // if
            Keyword::If => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_if()?))
            }
            // while
            Keyword::While => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_while()?))
            }
            // do while
            Keyword::Do if self.is_do_while_statement(next_token_type) => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_while()?))
            }
            // for
            Keyword::For => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_for()?))
            }
            // loop
            Keyword::Loop if self.language.is_destack() && self.peek_next_block().is_ok() => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_loop()?))
            }
            // try
            Keyword::Try => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_try()?))
            }
            // switch
            Keyword::Switch => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_match()?))
            }
            // match
            Keyword::Match if self.language.is_destack() => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_match()?))
            }
            // break
            Keyword::Break => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_break()?))
            }
            // continue
            Keyword::Continue => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_continue()?))
            }
            // await expression or await using
            Keyword::Await => {
                // reject await in contexts that forbid it
                if self.options.forbid_await {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }

                // parse await using when allowed
                if self.can_parse_using_declaration(&descriptor, Asynchrony::Async) {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_BINDING);
                    Ok(Some(self.eat_using(
                        start,
                        descriptor,
                        Asynchrony::Async,
                    )?))
                // otherwise parse await expression
                } else {
                    let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                    Ok(Some(self.eat_await()?))
                }
            }
            // comptime expression
            Keyword::Comptime if self.language.is_destack() => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_EXPRESSION);
                Ok(Some(self.eat_comptime()?))
            }
            // yield statement
            Keyword::Yield if self.options.in_generator => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_yield()?))
            }
            // throw statement
            Keyword::Throw => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_throw()?))
            }
            // return statement
            Keyword::Return => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                Ok(Some(self.eat_return()?))
            }
            // debugger statement
            Keyword::Debugger => {
                let _timing = self.timing_scope(tags::PARSE_KEYWORD_CONTROL);
                self.bump(); // eat `debugger`
                Ok(Some(
                    self.tree
                        .insert(Expression::Debugger, self.get_span_from(start)),
                ))
            }
            // fall through when not a keyword expression
            _ => Ok(None),
        }
    }

    /// Eat an identifier path expression with optional static arguments.
    fn eat_identifier_expression_path(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _identifier_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_IDENTIFIER);
        let (path, last_span) = self
            .eat_path_with_last_span()
            .for_node_type(NodeType::Expression)?;

        // speculatively unwrap postfix static parameterisation with `<` or `<<`
        //  (might also be just a comparison operator)
        //  `<<` (ShiftLeft) handles cases like `Extends<<T>() => ...>`
        let static_arguments = self.eat_static_arguments_in_expression(true);

        // immediately parse call if we have static arguments
        // (so we can stuff the arguments into the call expression)
        if static_arguments.is_some()
            && self.peek_is(TokenType::OpenParenthesis)
            && !self.options.in_new_receiver
        {
            let _call_timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX_CALL);
            let receiver = Expression::Path {
                path,
                static_arguments: None,
            };
            let receiver_id = self.tree.insert(receiver, self.get_span_from(start));
            self.tree.set_main_span(receiver_id, last_span);
            self.eat_call(receiver_id, static_arguments, PostfixPosition::Direct)
        } else {
            let expression = Expression::Path {
                path,
                static_arguments,
            };
            let expression_id = self.tree.insert(expression, self.get_span_from(start));
            self.tree.set_main_span(expression_id, last_span);
            Ok(expression_id)
        }
    }

    /// Return true if `<` starts a generic arrow function signature.
    fn can_start_generic_arrow_expression(&mut self) -> bool {
        if !self.language.is_typescript() && !self.language.is_destack() {
            return false;
        }
        if !self.peek_is(TokenType::LessThan) {
            return false;
        }
        if self.language.supports_jsx() && self.has_shift_left_tree_static_arguments() {
            return false;
        }
        if self.language.supports_jsx() && self.can_start_tree_literal() {
            return false;
        }

        // require disambiguators only when explicitly requested by settings
        let require_disambiguator =
            self.options.disallow_ambiguous_tree_literal && !self.options.in_type;
        self.peek_generic_arrow_after_type_parameters(require_disambiguator)
    }

    /// Return true if `<` starts a tree literal without committing tokens.
    pub(crate) fn can_start_tree_literal(&mut self) -> bool {
        if !self.language.supports_jsx() || self.options.in_type {
            return false;
        }
        if !self.peek_is(TokenType::LessThan) {
            return false;
        }
        if self.has_shift_left_tree_static_arguments() {
            return self.peek_tree_literal().is_ok();
        }

        let mark = self.mark();
        let require_disambiguator = self.options.disallow_ambiguous_tree_literal;
        let is_disambiguated_generic =
            self.peek_generic_arrow_after_type_parameters(require_disambiguator);
        self.rewind(mark);
        if is_disambiguated_generic {
            return false;
        }

        self.peek_tree_literal().is_ok()
    }

    /// Return true when a newline is followed by a tree literal start.
    fn can_start_tree_literal_after_newline(&mut self) -> bool {
        // require jsx support
        if !self.language.supports_jsx() {
            return false;
        }

        // require a leading newline token
        if !self.peek_is(TokenType::Newline) {
            return false;
        }

        // locate the next non newline token
        let next = self.next_non_newline_index_from(self.pos_index());
        if next == self.pos_index() {
            return false;
        }

        // ensure the token exists for probing
        self.token_stream.ensure_token(next);
        if next >= self.tokens().len() {
            return false;
        }

        // probe from the next token with in_type disabled
        self.with_pos(next, |parser| {
            let mut options = parser.options;
            options.in_type = false;
            parser
                .with_options(options, |parser| Ok(parser.can_start_tree_literal()))
                .unwrap_or(false)
        })
    }

    fn has_shift_left_tree_static_arguments(&mut self) -> bool {
        // snapshot parser state for lookahead
        let mark = self.mark();

        // probe for `<Identifier <<` without consuming tokens
        let has_shift_left = (|| {
            // find the identifier after the current position
            let identifier_index = self.next_non_newline_index_from(self.pos_index() + 1);
            let identifier_token = self.token_ref_at(identifier_index)?;
            if identifier_token.token.ty != TokenType::Identifier {
                return Some(false);
            }

            // find the operator after the identifier
            let after_identifier = self.next_non_newline_index_from(identifier_index + 1);
            let after_token = self.token_ref_at(after_identifier)?;
            Some(matches!(
                after_token.token.ty,
                TokenType::ShiftLeft | TokenType::SaturatingShiftLeft
            ))
        })()
        .unwrap_or(false);

        // restore parser state
        self.rewind(mark);

        has_shift_left
    }

    /// Return true if the current `do` token starts a do-while statement.
    /// Destack requires `do { ... } while ...`, while JS/TS allow `do` with any statement.
    fn is_do_while_statement(&mut self, next_token_type: TokenType) -> bool {
        // allow JS/TS do while forms
        if !self.language.is_destack() {
            return true;
        }

        // destack requires a block after do
        if next_token_type != TokenType::OpenBrace {
            return false;
        }

        // locate the matching close brace
        let open_index = self.index_for_next();
        let matching = match self.find_matching_close(
            Some(open_index as u32),
            TokenType::OpenBrace,
            TokenType::CloseBrace,
        ) {
            Ok(pos) => pos,
            Err(_) => return false,
        };

        // skip newlines after the block
        let after_close = match self.skip_newlines(matching) {
            Ok(pos) => pos,
            Err(_) => return false,
        };

        // check for a trailing while keyword
        let after_close_index = after_close as usize + 1;
        self.token_stream.ensure_token(after_close_index);
        let Some(after_token) = self.tokens().get(after_close_index) else {
            return false;
        };
        if after_token.token.ty != TokenType::Identifier {
            return false;
        }

        self.keyword_for_index(after_close_index) == Some(Keyword::While)
    }

    /// Eat an expression.
    pub fn eat_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_EXPRESSION);

        destack_base::ensure_sufficient_stack(|| self.eat_expression_inner())
    }

    /// Eat an expression body without stack growth.
    fn eat_expression_inner(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        // consume decorator prefixes before parsing the next expression
        if !self.options.in_decorator && self.peek_is(TokenType::At) {
            self.eat_decorators_prefix_maybe()?;
        }

        let start = self.mark();

        // labelled statement or expression (like `label: while(...)` or `label: loop {}`)
        // decorators treat keywords as identifiers, so skip label parsing there
        if !self.options.in_decorator
            && self.peek_is(TokenType::Identifier)
            && self.peek_next_is(TokenType::Colon)
        {
            let next_next_index = self.index_for_next_next();
            let next_next_token = self.token_at(next_next_index);
            let next_next_keyword = next_next_token
                .filter(|token| token.token.ty == TokenType::Identifier)
                .and_then(|_| self.keyword_for_index(next_next_index));
            // label targets that are always expressions
            let is_labelled_expression = matches!(
                next_next_keyword,
                Some(
                    Keyword::While
                        | Keyword::Do
                        | Keyword::For
                        | Keyword::Loop
                        | Keyword::If
                        | Keyword::Switch
                        | Keyword::Try
                        | Keyword::With
                )
            );

            // labelled blocks are only allowed in statement position
            let is_labelled_block =
                next_next_token.is_some_and(|token| token.token.ty == TokenType::OpenBrace);
            let can_parse_label = if self.options.in_statement_position
                && !self.language.is_destack()
            {
                true
            } else {
                is_labelled_expression || (self.options.in_statement_position && is_labelled_block)
            };
            if can_parse_label {
                let (label, label_span) = self.eat_identifier_with_span()?;
                self.eat_colon()?;
                // allow empty statement bodies in labelled statements
                let body = if self.peek_is(TokenType::Semicolon) {
                    let body_start = self.mark();
                    self.bump(); // eat semicolon
                    let block_id = self.tree.insert(
                        Block {
                            format: BlockFormat::Implicit,
                            expressions: Vec::new(),
                        },
                        self.get_span_from(&body_start),
                    );
                    self.tree
                        .insert(Expression::Block(block_id), self.get_span_from(&body_start))
                } else {
                    self.eat_expression()?
                };
                // reject labelled declarations that are invalid labelled items in JS/TS
                if !self.language.is_destack() && self.is_single_statement_declaration(body) {
                    return Err(ParseError::unexpected(self.tree.get_span(body)));
                }
                let labelled_id = self.tree.insert(
                    Expression::Labelled { label, body },
                    self.get_span_from(&start),
                );
                self.tree.set_main_span(labelled_id, label_span);
                return Ok(labelled_id);
            }
        }

        //
        // ------------------------------------------------------------
        // Modifiers
        // ------------------------------------------------------------
        //

        let descriptor = match self.eat_declaration_descriptor(&start)? {
            DescriptorHead::Descriptor(descriptor) => descriptor,
            DescriptorHead::Expression(expression_id) => return Ok(expression_id),
        };

        //
        // ------------------------------------------------------------
        // Main expression
        // ------------------------------------------------------------
        //

        let mut left_expression_id: LocalNodeId<Expression> = {
            let _timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY);
            let token_type = self.peek_token_type();

            //
            // ------------------------------------------------------------
            // Grouping
            // ------------------------------------------------------------
            //

            // identifier paths and keyword expressions
            match token_type {
                TokenType::Identifier => {
                    // identifier context setup
                    let next_token_type = self.peek_next_token_type();
                    let is_declaration_start = DECLARATION_START_TOKENS.contains(&next_token_type);
                    let module_identifier_matches = !self.options.in_decorator
                        && self.language.supports_module_declaration()
                        && self.module_identifier.is_some_and(|id| {
                            self.identifier_for_index(self.pos_index()) == Some(id)
                        });
                    let is_module_declaration_start =
                        module_identifier_matches && is_declaration_start;
                    let mut primary_expression_id = None;

                    // shorthand lambda function value
                    if !self.options.in_type
                        && !self.options.in_match_case
                        && (next_token_type == TokenType::Arrow
                            || next_token_type == TokenType::ArrowWide)
                    {
                        let lambda_id =
                            self.eat_function(&start, descriptor.clone(), false, false)?;
                        primary_expression_id = Some(self.tree.insert(
                            Expression::Declaration(lambda_id),
                            self.get_span_from(&start),
                        ));
                    }

                    // keyword and split state
                    let has_active_split = self.has_active_split();
                    let keyword = if self.options.in_decorator && !self.options.in_type {
                        let decorator_keyword = if has_active_split {
                            self.peek_any_keyword().ok()
                        } else {
                            self.keyword_for_index(self.pos_index())
                        };
                        match decorator_keyword {
                            Some(
                                Keyword::Await
                                | Keyword::This
                                | Keyword::New
                                | Keyword::Delete
                                | Keyword::Typeof
                                | Keyword::Void,
                            ) => decorator_keyword,
                            _ => None,
                        }
                    } else if has_active_split {
                        self.peek_any_keyword().ok()
                    } else {
                        self.keyword_for_index(self.pos_index())
                    };
                    let is_unary_keyword = matches!(keyword, Some(Keyword::Typeof | Keyword::Void));
                    let is_type_unary_keyword =
                        matches!(keyword, Some(Keyword::Typeof | Keyword::Keyof));

                    // fast path for non-keyword identifiers
                    if primary_expression_id.is_none()
                        && keyword.is_none()
                        && !has_active_split
                        && !self.options.in_decorator
                    {
                        // prefer module declarations when the identifier matches the module root
                        if is_module_declaration_start {
                            let namespace_id = self.eat_namespace(&start, descriptor.clone())?;
                            primary_expression_id = Some(self.tree.insert(
                                Expression::Declaration(namespace_id),
                                self.get_span_from(&start),
                            ));
                        } else {
                            // prefer contextual type literals when in type or static positions
                            let should_try_type_literal =
                                if self.options.in_type || self.options.in_static {
                                    true
                                } else {
                                    self.identifier_for_index(self.pos_index())
                                        .is_some_and(|id| {
                                            let ids = &self.type_literal_identifiers;
                                            id == ids.undefined
                                                || id == ids.unknown
                                                || id == ids.object
                                                || id == ids.null_
                                                || id == ids.any
                                                || id == ids.never
                                        })
                                };
                            if should_try_type_literal
                                && let Ok(type_literal) = self.peek_type_literal()
                            {
                                let _literal_timing =
                                    self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                                let type_literal = self.eat_type_literal(Some(type_literal))?;
                                primary_expression_id = Some(self.tree.insert(
                                    Expression::TypeLiteral(type_literal),
                                    self.get_span_from(&start),
                                ));
                            } else {
                                // fall back to an identifier path
                                primary_expression_id =
                                    Some(self.eat_identifier_expression_path(&start)?);
                            }
                        }
                    }

                    // unary prefix operations
                    if primary_expression_id.is_none() && is_unary_keyword && !self.options.in_type
                    {
                        let operator = match keyword {
                            Some(Keyword::Typeof) => UnaryOperator::Typeof,
                            Some(Keyword::Void) => UnaryOperator::Void,
                            _ => unreachable!(),
                        };
                        let operator_start = self.mark();
                        self.bump(); // eat unary operator (always because right associative)
                        let operator_span = self.get_span_from(&operator_start);
                        let mut right_options = self
                            .options
                            .not_in_position()
                            .in_left_precedence(operator.precedence());
                        if self.options.in_type_conditional_right {
                            right_options = right_options.in_type_conditional_right();
                        }
                        let right =
                            self.with_options(right_options, |parser| parser.eat_expression())?;

                        // unparenthesized arrow functions are not unary operands
                        if self.is_unparenthesized_lambda_expression(right) {
                            return Err(ParseError::unexpected(self.tree.get_span(right)));
                        }

                        let expression = Expression::Unary { operator, right };
                        let expression_id =
                            self.tree.insert(expression, self.get_span_from(&start));
                        self.tree.set_main_span(expression_id, operator_span);
                        primary_expression_id = Some(expression_id);
                    }

                    // type unary operations
                    if primary_expression_id.is_none() && is_type_unary_keyword {
                        let operator = match keyword {
                            Some(Keyword::Typeof) => TypeUnaryOperator::Typeof,
                            Some(Keyword::Keyof) => TypeUnaryOperator::Keyof,
                            _ => unreachable!(),
                        };
                        let operator_start = self.mark();
                        self.bump(); // eat type unary operator (always because right associative)
                        let operator_span = self.get_span_from(&operator_start);
                        let mut right_options = self
                            .options
                            .not_in_position()
                            .in_type()
                            .in_left_precedence(operator.precedence());
                        if self.options.in_type_conditional_right {
                            right_options = right_options.in_type_conditional_right();
                        }
                        let right =
                            self.with_options(right_options, |parser| parser.eat_expression())?;
                        let expression = Expression::TypeUnary { operator, right };
                        let expression_id =
                            self.tree.insert(expression, self.get_span_from(&start));
                        self.tree.set_main_span(expression_id, operator_span);
                        primary_expression_id = Some(expression_id);
                    }

                    // do block expression or do-while block
                    if primary_expression_id.is_none() && keyword == Some(Keyword::Do) {
                        if self.is_do_while_statement(next_token_type) {
                            primary_expression_id = Some(self.eat_while()?);
                        } else if next_token_type == TokenType::OpenBrace {
                            let block_id = self.eat_block()?;
                            primary_expression_id =
                                Some(self.tree.insert(
                                    Expression::Block(block_id),
                                    self.get_span_from(&start),
                                ));
                        }
                    }

                    // keyword or contextual module declaration
                    if primary_expression_id.is_none()
                        && keyword.is_none()
                        && is_module_declaration_start
                    {
                        // parse contextual module declarations after other identifier paths
                        let namespace_id = self.eat_namespace(&start, descriptor.clone())?;
                        primary_expression_id = Some(self.tree.insert(
                            Expression::Declaration(namespace_id),
                            self.get_span_from(&start),
                        ));
                    }

                    if primary_expression_id.is_none()
                        && let Some(keyword) = keyword
                    {
                        // parse keyword expressions and declaration starters
                        let _keyword_timing =
                            self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_KEYWORD);
                        if let Some(keyword_expression_id) = self.eat_keyword_expression(
                            &start,
                            descriptor.clone(),
                            keyword,
                            next_token_type,
                            is_declaration_start,
                        )? {
                            primary_expression_id = Some(keyword_expression_id);
                        }
                    }

                    // type literal
                    if primary_expression_id.is_none() {
                        // late fallback for contextual type literals
                        let should_try_type_literal =
                            if self.options.in_type || self.options.in_static {
                                true
                            } else {
                                self.identifier_for_index(self.pos_index())
                                    .is_some_and(|id| {
                                        let ids = &self.type_literal_identifiers;
                                        id == ids.undefined
                                            || id == ids.unknown
                                            || id == ids.object
                                            || id == ids.null_
                                            || id == ids.any
                                            || id == ids.never
                                    })
                            };
                        if should_try_type_literal
                            && let Ok(type_literal) = self.peek_type_literal()
                        {
                            let _literal_timing =
                                self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                            let type_literal = self.eat_type_literal(Some(type_literal))?;
                            primary_expression_id = Some(self.tree.insert(
                                Expression::TypeLiteral(type_literal),
                                self.get_span_from(&start),
                            ));
                        }
                    }

                    if let Some(primary_expression_id) = primary_expression_id {
                        primary_expression_id
                    } else {
                        // final fallback for identifier paths
                        self.eat_identifier_expression_path(&start)?
                    }
                }
                _ => {
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

                        // allow leading elementwise operators in type expressions
                        let expression = self.tree.get(expression_id);
                        if !matches!(
                            expression,
                            Expression::Binary { operator, .. }
                                if *operator == leading_binary_operator
                        ) && !self.options.in_type
                        {
                            return Err(ParseError::unexpected(self.get_span_from(&start)));
                        }

                        // expand span
                        self.tree
                            .set_span(expression_id, self.get_span_from(&start));

                        // forward the expression (no need to parse further here)
                        return Ok(expression_id);
                    }
                    // parenthesis
                    // may be tuple, lambda, or parenthesized expression
                    else if token_type == TokenType::OpenParenthesis {
                        let _group_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_GROUP);
                        // look ahead for lambda and tuple cues without committing tokens
                        let lookahead_mark = self.mark();
                        let lookahead_result = (|| {
                            let open_pos = self.pos();
                            let closing_pos = self.find_matching_close(
                                None,
                                TokenType::OpenParenthesis,
                                TokenType::CloseParenthesis,
                            )?;
                            let closing_pos_for_follow = self.skip_newlines(closing_pos)?;
                            let has_top_level_comma = self.language.is_destack()
                                && self.has_token_before_matching_close(
                                    open_pos,
                                    closing_pos,
                                    TokenType::Comma,
                                    self.options.in_type,
                                )?;
                            let next_token_type = self
                                .token_ref_at(closing_pos_for_follow as usize + 1)
                                .map(|token| token.token.ty);
                            let has_arrow = matches!(
                                next_token_type,
                                Some(TokenType::Arrow | TokenType::ArrowWide)
                            );
                            let has_colon = matches!(next_token_type, Some(TokenType::Colon));
                            Ok((has_top_level_comma, has_arrow, has_colon))
                        })();
                        self.rewind(lookahead_mark);
                        let (has_top_level_comma, has_arrow, has_colon) = lookahead_result?;
                        let is_colon_lambda_allowed = has_colon
                            && (self.language.is_destack() || self.language.is_typescript())
                            && !self.options.in_before_type
                            && !self.options.in_match_case
                            && (!self.options.in_type || !has_top_level_comma);
                        let mut lambda_expression_id = None;

                        // parse lambda when we see a likely arrow or colon
                        if (has_arrow || is_colon_lambda_allowed)
                            && !self.options.in_arrow_return_type
                        {
                            // avoid colon lambdas that steal ternary delimiters
                            if self.options.in_ternary_condition && has_colon {
                                let speculative_start = self.mark();
                                let speculative_start_idx = self.tree.next_id();
                                if let Ok(lambda_id) =
                                    self.eat_function(&start, descriptor, false, false)
                                {
                                    let has_ternary_delimiter = self.peek_is(TokenType::Colon)
                                        || self
                                            .peek_token_after_newlines(self.pos(), TokenType::Colon)
                                            .is_ok();
                                    let should_accept = match self.tree.get(lambda_id) {
                                        Declaration::Function { body, .. } => {
                                            (body.is_some() || self.options.in_type)
                                                && has_ternary_delimiter
                                        }
                                        _ => has_ternary_delimiter,
                                    };
                                    if should_accept {
                                        lambda_expression_id = Some(self.tree.insert(
                                            Expression::Declaration(lambda_id),
                                            self.get_span_from(&start),
                                        ));
                                    } else {
                                        self.restore(speculative_start, speculative_start_idx);
                                    }
                                } else {
                                    self.restore(speculative_start, speculative_start_idx);
                                }
                            } else {
                                let lambda_id =
                                    self.eat_function(&start, descriptor, false, false)?;
                                lambda_expression_id = Some(self.tree.insert(
                                    Expression::Declaration(lambda_id),
                                    self.get_span_from(&start),
                                ));
                            }
                        }

                        // tuple or parenthesized expression
                        if let Some(lambda_expression_id) = lambda_expression_id {
                            lambda_expression_id
                        } else {
                            self.bump(); // eat open parenthesis
                            self.eat_newlines_maybe()?;

                            // empty tuple or sequence when we immediately see a closing parenthesis
                            if self.peek_is(TokenType::CloseParenthesis) {
                                self.bump(); // eat closing parenthesis

                                // in Destack: empty tuple
                                if self.language.is_destack() {
                                    self.tree.insert(
                                        Expression::TupleExpression { elements: vec![] },
                                        self.get_span_from(&start),
                                    )
                                }
                                // in JS or TS: empty sequence expression
                                else {
                                    self.tree.insert(
                                        Expression::SequenceExpression {
                                            expressions: vec![],
                                        },
                                        self.get_span_from(&start),
                                    )
                                }
                            }
                            // tuple when we see a named element or top level comma
                            else if (self.language.is_destack()
                                && self.peek_is(TokenType::Identifier)
                                && self.peek_next_is(TokenType::Colon))
                                || has_top_level_comma
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
                                    self.get_span_from(&start),
                                )
                            }
                            // tuple or parenthesized expression for the remaining cases
                            else {
                                let inner_start = self.pos();
                                let mut inner_options = self.options.nested().in_parenthesis();
                                inner_options.allow_sequence_expression = true;
                                if self.options.in_type {
                                    inner_options = inner_options.in_type();
                                }
                                let expression_id = self.with_options(inner_options, |parser| {
                                    parser.eat_expression()
                                })?;
                                self.eat_newlines_maybe()?;
                                self.eat_token(TokenType::CloseParenthesis)?;
                                let inner_token_type = self.token_type_at(inner_start as usize);
                                match self.tree.get(expression_id) {
                                    // if it was a tuple starting here, expand it to cover the entire span
                                    //  (except if that tuple has its own parenthesis already when nesting)
                                    Expression::TupleExpression { .. }
                                        if inner_token_type != TokenType::OpenParenthesis =>
                                    {
                                        self.tree
                                            .set_span(expression_id, self.get_span_from(&start));
                                        expression_id
                                    }
                                    // if it was a sequence expression starting here, expand it to cover the entire span
                                    Expression::SequenceExpression { .. }
                                        if inner_token_type != TokenType::OpenParenthesis =>
                                    {
                                        self.tree
                                            .set_span(expression_id, self.get_span_from(&start));
                                        expression_id
                                    }
                                    // otherwise it was a manually parenthesized expression, wrap it
                                    _ => self.tree.insert(
                                        Expression::Parenthesized {
                                            expression: expression_id,
                                        },
                                        self.get_span_from(&start),
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

                    // pointer types
                    else if self.options.in_type && self.peek_is(TokenType::Multiply) {
                        self.bump(); // eat *
                        let mutability = self.eat_reference_mutability_maybe()?;
                        let right = self
                            .with_options(self.options.not_in_position(), |parser| {
                                parser.eat_expression()
                            })?;
                        let expression = Expression::PointerOf { mutability, right };
                        self.tree.insert(expression, self.get_span_from(&start))
                    }
                    // unary prefix operations
                    else if let Ok(operator) = self.peek_unary_prefix_operator() {
                        let operator_start = self.mark();
                        self.bump(); // eat unary operator (always because right associative)
                        let operator_span = self.get_span_from(&operator_start);
                        let mut right_options = self
                            .options
                            .not_in_position()
                            .in_left_precedence(operator.precedence());
                        if self.options.in_type_conditional_right {
                            right_options = right_options.in_type_conditional_right();
                        }
                        let right =
                            self.with_options(right_options, |parser| parser.eat_expression())?;
                        let expression = Expression::Unary { operator, right };
                        let expression_id =
                            self.tree.insert(expression, self.get_span_from(&start));
                        self.tree.set_main_span(expression_id, operator_span);
                        expression_id
                    }
                    // type unary operations
                    else if let Ok(operator) = self.peek_type_unary_prefix_operator() {
                        let operator_start = self.mark();
                        self.bump(); // eat type unary operator (always because right associative)
                        let operator_span = self.get_span_from(&operator_start);
                        let mut right_options = self
                            .options
                            .not_in_position()
                            .in_type()
                            .in_left_precedence(operator.precedence());
                        if self.options.in_type_conditional_right {
                            right_options = right_options.in_type_conditional_right();
                        }
                        let right =
                            self.with_options(right_options, |parser| parser.eat_expression())?;
                        let expression = Expression::TypeUnary { operator, right };
                        let expression_id =
                            self.tree.insert(expression, self.get_span_from(&start));
                        self.tree.set_main_span(expression_id, operator_span);
                        expression_id
                    }
                    // value (`^` or `^readonly` or `^T`)
                    else if self.peek_is(TokenType::ElementwiseXor) && self.language.is_destack()
                    {
                        self.bump(); // eat ^
                        let mutability = self.eat_reference_mutability_maybe()?;
                        let variance = self.eat_variance_bound_maybe()?;
                        let right = self
                            .with_options(self.options.not_in_position(), |parser| {
                                parser.eat_expression()
                            })?;
                        let expression = Expression::ValueOf {
                            mutability,
                            variance,
                            right,
                        };
                        self.tree.insert(expression, self.get_span_from(&start))
                    }
                    // reference (`&` or `&var` or `&T`)
                    else if self.peek_is(TokenType::ElementwiseAnd) && self.language.is_destack()
                    {
                        self.bump(); // eat &
                        let mutability = self.eat_reference_mutability_maybe()?;
                        let variance = self.eat_variance_bound_maybe()?;
                        let right = self
                            .with_options(self.options.not_in_position(), |parser| {
                                parser.eat_expression()
                            })?;
                        let expression = Expression::ReferenceOf {
                            mutability,
                            variance,
                            right,
                        };
                        self.tree.insert(expression, self.get_span_from(&start))
                    }
                    //
                    // ------------------------------------------------------------
                    // Literals / Aliases / Values
                    // ------------------------------------------------------------
                    //
                    // array literal
                    else if token_type == TokenType::OpenBracket {
                        let elements = self
                            .with_options(self.options.not_in_position(), |parser| {
                                parser.eat_array_literal()
                            })?;
                        self.tree.insert(
                            Expression::ArrayExpression { elements },
                            self.get_span_from(&start),
                        )
                    }
                    // object literal
                    else if token_type == TokenType::OpenBrace
                        && (!self.options.in_statement_position
                            || self.can_parse_object_literal_in_statement_position())
                    {
                        // prefer mapped types in type positions
                        if self.options.in_type {
                            let speculative_start = self.mark();
                            let speculative_start_idx = self.tree.next_id();
                            if let Ok(mapped_id) = self.eat_type_mapped_expression() {
                                mapped_id
                            } else {
                                self.restore(speculative_start, speculative_start_idx);
                                let properties = self.with_options(
                                    self.options.not_in_position().in_type(),
                                    |parser| parser.eat_object_literal(),
                                )?;
                                self.tree.insert(
                                    Expression::ObjectExpression {
                                        ty: None,
                                        properties,
                                    },
                                    self.get_span_from(&start),
                                )
                            }
                        }
                        // fall back to object literal
                        else {
                            let properties = self
                                .with_options(self.options.not_in_position(), |parser| {
                                    parser.eat_object_literal()
                                })?;
                            self.tree.insert(
                                Expression::ObjectExpression {
                                    ty: None,
                                    properties,
                                },
                                self.get_span_from(&start),
                            )
                        }
                    }
                    // block
                    else if self.peek_block().is_ok() {
                        let block_id = self.eat_block()?;
                        self.tree
                            .insert(Expression::Block(block_id), self.get_span_from(&start))
                    }
                    // statically parameterized lambda: <T>(...) or <T,>(...)
                    // (also handles multiline in type context: `<\nT\n>(...) => ...`)
                    else if token_type == TokenType::LessThan
                        && self.can_start_generic_arrow_expression()
                    {
                        let function_id = self.eat_function(&start, descriptor, false, false)?;
                        self.tree.insert(
                            Expression::Declaration(function_id),
                            self.get_span_from(&start),
                        )
                    }
                    // typescript angle bracket type assertion
                    else if token_type == TokenType::LessThan
                        && self.language.is_typescript()
                        && !self.language.supports_jsx()
                        && !self.options.in_type
                        && !self.options.in_new_receiver
                        && !self.options.disallow_ambiguous_tree_literal
                    {
                        self.eat_type_assertion_expression(&start)?
                    }
                    // tree literal
                    else if token_type == TokenType::LessThan && self.can_start_tree_literal() {
                        self.with_options(self.options.not_in_position(), |parser| {
                            parser.eat_tree_literal()
                        })?
                    }
                    // template literal
                    else if self.is_template_literal_start() {
                        let _literal_timing =
                            self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                        if self.options.in_type {
                            self.eat_type_template_literal_expression()?
                        } else {
                            let template_literal = self.eat_template_literal()?;
                            self.tree.insert(
                                Expression::TemplateExpression {
                                    value: template_literal,
                                },
                                self.get_span_from(&start),
                            )
                        }
                    }
                    // scalar literal
                    else if self.is_scalar_literal_start() {
                        let _literal_timing =
                            self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                        let scalar_literal = self.eat_scalar_literal()?;
                        self.tree.insert(
                            Expression::ScalarLiteral(scalar_literal),
                            self.get_span_from(&start),
                        )
                    }
                    // type literal
                    // (type literals are contextual, most are only parsed inside type context to avoid shadowing)
                    else if token_type == TokenType::Not
                        && let Ok(type_literal) = self.peek_type_literal()
                    {
                        let _literal_timing =
                            self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_LITERAL);
                        let type_literal = self.eat_type_literal(Some(type_literal))?;
                        self.tree.insert(
                            Expression::TypeLiteral(type_literal),
                            self.get_span_from(&start),
                        )
                    }
                    // private identifier
                    else if token_type == TokenType::Hash
                        && self.peek_next_is(TokenType::Identifier)
                    {
                        // require the hash and identifier to be adjacent
                        let hash_index = self.pos_index();
                        let ident_index = hash_index + 1;
                        self.check_tokens_are_adjacent(hash_index, ident_index)?;

                        self.bump(); // eat #
                        let (name, name_span) = self.eat_identifier_with_span()?;
                        let expression_id = self.tree.insert(
                            Expression::PrivateIdentifier { name },
                            self.get_span_from(&start),
                        );
                        self.tree.set_main_span(expression_id, name_span);
                        expression_id
                    }
                    //
                    // ------------------------------------------------------------
                    // Error
                    // ------------------------------------------------------------
                    //
                    else {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }
                }
            }
        };

        // statement expressions do not accept postfix or infix operators
        if self.options.in_statement_position {
            let expression = self.tree.get(left_expression_id);
            if expression.is_top_level_statement() {
                let is_lambda_declaration = match expression {
                    Expression::Declaration(declaration_id) => {
                        matches!(
                            self.tree.get(*declaration_id),
                            Declaration::Function { signature, .. }
                                if signature.kind == FunctionKind::Lambda
                        )
                    }
                    _ => false,
                };
                if !is_lambda_declaration {
                    return Ok(left_expression_id);
                }
            }
        }

        //
        // ------------------------------------------------------------
        // Postfix operations
        // ------------------------------------------------------------
        //

        {
            let _timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX);
            // struct literal postfix with `{` (like `Vector2 { x: 0, y }`)
            if let Expression::Path { .. } = self.tree.get(left_expression_id)
                && self.peek_is(TokenType::OpenBrace)
                && !self.options.in_before_block
                && self.language.is_destack()
            {
                let properties = self.eat_object_literal()?;
                left_expression_id = self.tree.insert(
                    Expression::ObjectExpression {
                        ty: Some(left_expression_id),
                        properties,
                    },
                    self.get_span_from(&start),
                );
            }
            // eat all regular postfix operators
            while self.has_more_tokens() {
                // stop before ternary or switch case boundary so postfix parsing does not consume ':'
                if (self.options.in_ternary_condition || self.options.in_match_case)
                    && (self.peek_is(TokenType::Colon)
                        || self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Colon))
                {
                    break;
                }
                // stop before static boundary so postfix parsing does not consume '>'
                if self.options.in_static
                    && (self.peek_is(TokenType::GreaterThan)
                        || self.peek_is(TokenType::Newline)
                            && self.peek_next_is(TokenType::GreaterThan))
                {
                    break;
                }

                if self.is_template_literal_start() {
                    // tagged template receivers must be left hand side expressions
                    if !self.tagged_template_tag_is_valid(left_expression_id) {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }

                    let template_literal = self.eat_template_literal()?;
                    left_expression_id = self.tree.insert(
                        Expression::TaggedTemplateExpression {
                            tag: left_expression_id,
                            value: template_literal,
                        },
                        self.get_span_from(&start),
                    );
                    continue;
                }

                // call parsing flags
                let has_direct_call = self.peek_is(TokenType::OpenParenthesis);
                let has_direct_call_after_newlines = self.peek_is(TokenType::Newline)
                    && self
                        .peek_token_after_newlines(self.pos(), TokenType::OpenParenthesis)
                        .is_ok();
                let has_indirect_call =
                    self.peek_is(TokenType::Dot) && self.peek_next_is(TokenType::OpenParenthesis);
                let can_direct_call = (has_direct_call || has_direct_call_after_newlines)
                    && !self.options.in_type
                    && !matches!(self.tree.get(left_expression_id), Expression::Maybe { .. })
                    && !self.options.in_new_receiver;
                let should_parse_call =
                    (can_direct_call || has_indirect_call) && !self.options.in_type;

                // unary postfix operations
                if let Ok(operator) = self.peek_unary_postfix_operator() {
                    let operator_start = self.mark();
                    self.bump(); // eat unary operator
                    let operator_span = self.get_span_from(&operator_start);
                    left_expression_id = self.tree.insert(
                        Expression::Unary {
                            operator,
                            right: left_expression_id,
                        },
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(left_expression_id, operator_span);
                }
                // type unary postfix operations
                else if let Ok(operator) = self.peek_type_unary_postfix_operator() {
                    // avoid consuming conditional type ? as a type maybe
                    let operator_start = self.mark();
                    self.bump(); // eat type unary operator
                    if matches!(
                        operator,
                        TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime
                    ) {
                        self.bump(); // eat second token
                    }
                    let operator_span = self.get_span_from(&operator_start);
                    left_expression_id = self.tree.insert(
                        Expression::TypeUnary {
                            operator,
                            right: left_expression_id,
                        },
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(left_expression_id, operator_span);
                }
                // range (`..`, `..=`)
                else if self.peek_is(TokenType::Range) {
                    self.bump(); // eat ..
                    let is_inclusive = if self.peek_is(TokenType::Assign) {
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
                        self.get_span_from(&start),
                    );
                }
                // private member (also works across newline)
                else if let Ok(distance) = self.peek_private_member() {
                    self.bump_by(distance - 1); // keep the identifier
                    let (name, name_span) = self.eat_identifier_with_span()?;
                    // speculatively unwrap postfix static parameterisation with `<`
                    //  (might also be just a comparison operator)
                    let static_arguments = self.eat_static_arguments_in_expression(false);
                    left_expression_id = self.tree.insert(
                        Expression::PrivateMember {
                            left: left_expression_id,
                            name,
                            static_arguments,
                        },
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(left_expression_id, name_span);
                }
                // member (also works across newline)
                else if let Ok(distance) = self.peek_member_name() {
                    self.bump_by(distance - 1); // keep the identifier

                    // in js and ts: decimal integer literals need a separator before member access
                    if self.invalid_decimal_integer_member_access(left_expression_id, distance) {
                        return Err(ParseError::unexpected(self.prev().expect("peeked").span));
                    }

                    let (name, name_span) = self.eat_member_name_with_span()?;
                    // speculatively unwrap postfix static parameterisation with `<`
                    //  (might also be just a comparison operator)
                    let static_arguments = self.eat_static_arguments_in_expression(false);
                    left_expression_id = self.tree.insert(
                        Expression::Member {
                            left: left_expression_id,
                            name,
                            static_arguments,
                        },
                        self.get_span_from(&start),
                    );
                    self.tree.set_main_span(left_expression_id, name_span);
                }
                // index (like `[]`)
                else if self.peek_is(TokenType::OpenBracket)
                    && !matches!(self.tree.get(left_expression_id), Expression::Maybe { .. })
                    || self.peek_is(TokenType::Dot) && self.peek_next_is(TokenType::OpenBracket)
                {
                    let position = if self.peek_is(TokenType::Dot) {
                        self.bump(); // eat .
                        PostfixPosition::Indirect
                    } else {
                        PostfixPosition::Direct
                    };
                    left_expression_id = self.eat_index(left_expression_id, position)?;
                }
                // call (like `()`)
                else if should_parse_call {
                    // unparenthesized arrow functions cannot be direct call receivers
                    if has_direct_call
                        && self.is_unparenthesized_lambda_expression(left_expression_id)
                    {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }

                    let _call_timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX_CALL);
                    if self.peek_is(TokenType::Newline)
                        && self
                            .peek_token_after_newlines(self.pos(), TokenType::OpenParenthesis)
                            .is_ok()
                    {
                        self.eat_newlines_maybe()?; // eat newlines
                    }
                    let position = if self.peek_is(TokenType::Dot) {
                        self.bump(); // eat .
                        PostfixPosition::Indirect
                    } else {
                        PostfixPosition::Direct
                    };
                    left_expression_id = self.eat_call(left_expression_id, None, position)?;
                }
                // statically parameterized call or instantiation expression (like `(expr)<T>()` or `(expr)<T>`)
                else if (self.peek_is(TokenType::LessThan)
                    || self.peek_is(TokenType::ShiftLeft)
                    || self.peek_is(TokenType::Dot)
                        && (self.peek_next_is(TokenType::LessThan)
                            || self.peek_next_is(TokenType::ShiftLeft)))
                    && !self.options.in_new_receiver
                    && !self.options.in_tree_literal
                    && !self.language.is_javascript()
                {
                    // decide whether this is a direct or optional chain static argument list
                    let has_indirect_static = self.peek_is(TokenType::Dot)
                        && (self.peek_next_is(TokenType::LessThan)
                            || self.peek_next_is(TokenType::ShiftLeft));
                    let is_optional_chain =
                        matches!(self.tree.get(left_expression_id), Expression::Maybe { .. });
                    if !has_indirect_static && is_optional_chain {
                        break;
                    }
                    if has_indirect_static && !is_optional_chain {
                        break;
                    }

                    // speculatively try to parse static arguments
                    let speculative_start = self.mark();
                    let speculative_start_idx = self.tree.next_id();
                    let position = if has_indirect_static {
                        self.bump(); // eat .
                        PostfixPosition::Indirect
                    } else {
                        PostfixPosition::Direct
                    };
                    if let Ok(static_arguments) = self.eat_static_arguments() {
                        // call with static arguments
                        if self.peek_is(TokenType::OpenParenthesis) {
                            left_expression_id = self.eat_call(
                                left_expression_id,
                                Some(static_arguments),
                                position,
                            )?;
                        }
                        // instantiation expression
                        else if self.can_follow_type_arguments_in_expression() {
                            left_expression_id = self.tree.insert(
                                Expression::Instantiation {
                                    left: left_expression_id,
                                    static_arguments,
                                },
                                self.get_span_from(&start),
                            );
                        } else {
                            self.restore(speculative_start, speculative_start_idx);
                            break;
                        }
                    } else {
                        // not static arguments, restore and exit postfix loop
                        self.restore(speculative_start, speculative_start_idx);
                        break;
                    }
                }
                // optional chaining or type conditional boundary
                // (like `x?`, `x.?`, `x?.`)
                else if self.peek_is(TokenType::Maybe)
                    || self.peek_is(TokenType::Dot) && self.peek_next_is(TokenType::Maybe)
                    || self.optional_chain_starts_after_newlines()
                {
                    // capture type conditional operands when in type contexts
                    let type_conditional_operands = if self.options.in_type {
                        self.split_type_conditional_operands(left_expression_id)
                    } else {
                        None
                    };
                    let is_type_conditional = type_conditional_operands.is_some();

                    // normalize newlines before checking postfix markers
                    self.eat_newlines_maybe()?; // eat newlines

                    // classify optional chain and postfix maybe
                    let is_optional_chain_after_maybe = self.is_optional_chain_after_maybe();
                    let is_direct_postfix_maybe = self.language.is_destack()
                        && (self.peek_next_any_stop().is_ok()
                            && self.prev_token_type() != TokenType::Newline
                            || self.peek_next_any_close_parenthesis().is_ok()
                            || self.peek_next_assign_operator().is_ok());
                    let is_postfix_maybe = !self.options.in_type
                        && self.peek_is(TokenType::Maybe)
                        && (is_optional_chain_after_maybe || is_direct_postfix_maybe);

                    // postfix maybe
                    if is_postfix_maybe {
                        self.bump(); // eat ?
                        // (don't consume delimiter/stop)
                        left_expression_id = self.tree.insert(
                            Expression::Maybe {
                                left: left_expression_id,
                                position: PostfixPosition::Direct,
                            },
                            self.get_span_from(&start),
                        );
                    }
                    // dot maybe
                    else if !self.options.in_type
                        && self.peek_is(TokenType::Dot)
                        && self.peek_next_is(TokenType::Maybe)
                    {
                        self.bump(); // eat .
                        self.bump(); // eat ?
                        left_expression_id = self.tree.insert(
                            Expression::Maybe {
                                left: left_expression_id,
                                position: PostfixPosition::Indirect,
                            },
                            self.get_span_from(&start),
                        );
                    }
                    // type conditional expressions in type contexts
                    else {
                        if !self.options.in_type || !is_type_conditional {
                            break;
                        }

                        self.bump(); // eat ?
                        self.eat_newlines_maybe()?;
                        let Some((left, right)) = type_conditional_operands else {
                            break;
                        };
                        // type conditional expression
                        let then_expression_id = self.with_options(
                            self.options
                                .not_in_position()
                                .in_type()
                                .in_ternary_condition(),
                            |parser| parser.eat_expression(),
                        )?;
                        self.eat_newlines_maybe()?;
                        self.eat_colon()?;
                        self.eat_newlines_maybe()?;
                        let mut else_options = self.options.not_in_position().in_type();
                        if self.options.in_type_conditional_right {
                            else_options = else_options.in_type_conditional_right();
                        }
                        let else_expression_id =
                            self.with_options(else_options, |parser| parser.eat_expression())?;
                        let expression = Expression::TypeConditional {
                            left,
                            right,
                            then_type: then_expression_id,
                            else_type: else_expression_id,
                        };
                        left_expression_id =
                            self.tree.insert(expression, self.get_span_from(&start));
                    }
                }
                // must
                else if self.peek_is(TokenType::Not)
                    || self.peek_is(TokenType::Dot) && self.peek_next_is(TokenType::Not)
                {
                    let position = if self.peek_is(TokenType::Dot) {
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
                        self.get_span_from(&start),
                    );
                }
                // tuple (Destack) or sequence expression (JS/TS)
                // (if we have a delimiter following an expression inside parentheses)
                else if self.options.in_parenthesis && self.peek_is(TokenType::Comma) {
                    self.bump(); // eat comma
                    self.eat_newlines_maybe()?;
                    if self.language.is_destack() {
                        // build a tuple
                        let first_element_id = self.tree.insert(
                            Argument::Positional {
                                modifiers: None,
                                value: left_expression_id,
                            },
                            self.get_span_from(&start),
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
                            self.get_span_from(&start),
                        );
                    } else {
                        // build a sequence expression (comma operator)
                        let mut expressions = vec![left_expression_id];
                        // parse remaining expressions until we see the close parenthesis
                        // (mirrors eat_sequence_literal_body behavior for consistency)
                        while !self.peek_is(TokenType::CloseParenthesis) {
                            // consume any comma delimiter
                            if self.peek_is(TokenType::Comma) {
                                self.bump(); // eat comma
                                self.eat_newlines_maybe()?;
                                continue;
                            }
                            // parse next expression
                            let expr_id = self.with_options(
                                self.options.not_in_position().not_in_sequence_expression(),
                                |parser| parser.eat_expression(),
                            )?;
                            expressions.push(expr_id);
                            self.eat_newlines_maybe()?;
                        }
                        // build sequence expression
                        left_expression_id = self.tree.insert(
                            Expression::SequenceExpression { expressions },
                            self.get_span_from(&start),
                        );
                    }
                }
                // done
                else {
                    break;
                }
            }
        }

        //
        // ------------------------------------------------------------
        // Infix operations (binary and assign, left associative)
        // ------------------------------------------------------------
        //

        // eat infix expressions while left precedence is weaker than right precedence
        // track statement newline boundaries before entering the loop
        let left_is_statement = self.options.in_statement_position
            && self
                .tree
                .get(left_expression_id)
                .ends_statement_on_newline();

        {
            let _timing = self.timing_scope(tags::PARSE_EXPRESSION_INFIX);
            while self.has_more_tokens() {
                // stop before conditional boundaries so infix lookahead does not lex past `?`
                if self.peek_is(TokenType::Maybe)
                    || self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Maybe)
                {
                    break;
                }
                // stop before ternary or match case boundary so infix lookahead does not lex past `:`
                if (self.options.in_ternary_condition || self.options.in_match_case)
                    && (self.peek_is(TokenType::Colon)
                        || self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Colon))
                {
                    break;
                }
                // statement expressions do not continue across newlines
                if left_is_statement && self.peek_is(TokenType::Newline) {
                    break;
                }
                // type expressions stop before tree literals on a new line
                if self.options.in_type
                    && self.peek_is(TokenType::Newline)
                    && self.can_start_tree_literal_after_newline()
                {
                    break;
                }
                let (right_operator, operator_offset) = {
                    // infix operator on same line with higher precedence
                    if let Ok((operator, operator_offset)) = self.peek_infix_operator()
                        && (self.options.left_precedence.is_none()
                            || self.options.left_precedence.unwrap() < operator.precedence())
                    {
                        (operator, operator_offset)
                    }
                    // infix operator on next line with higher precedence
                    else if self.peek_is(TokenType::Newline)
                        && let Ok((operator, operator_offset)) = self.peek_next_infix_operator()
                        && (self.options.left_precedence.is_none()
                            || self.options.left_precedence.unwrap() < operator.precedence())
                    {
                        (operator, operator_offset)
                    }
                    // infix operator after multiple newlines in type expressions
                    else if self.peek_is(TokenType::Newline)
                        && self.options.in_type
                        && let Ok((operator, operator_offset)) =
                            self.peek_infix_operator_after_newlines()
                        && matches!(
                            operator,
                            InfixOperator::Binary(
                                BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                            )
                        )
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
                if self.peek_is(TokenType::Newline) {
                    self.eat_newlines_maybe()?; // eat newlines
                }

                // capture operator span before eating
                let operator_start = self.mark();
                self.bump_by(operator_offset); // eat infix operator
                let operator_span = self.get_span_from(&operator_start);

                self.eat_newlines_maybe()?; // allow newlines after infix operator

                // eat right expression
                let subject_id = left_expression_id;
                let mut right_options = self
                    .options
                    .not_in_position()
                    .in_left_precedence(right_operator.precedence());

                // type binary operators parse the right side as a type expression
                if matches!(right_operator, InfixOperator::TypeBinary(_)) {
                    right_options = right_options.in_type();
                }
                if self.options.in_type_conditional_right
                    || matches!(
                        right_operator,
                        InfixOperator::TypeBinary(TypeBinaryOperator::Extends)
                    )
                {
                    right_options = right_options.in_type_conditional_right();
                }
                let right_expression_id =
                    self.with_options(right_options, |parser| parser.eat_expression())?;

                // combine into new left expression
                let left_expression = self.make_infix_expression(
                    left_expression_id,
                    right_operator,
                    right_expression_id,
                );
                left_expression_id = self
                    .tree
                    .insert(left_expression, self.get_span_from(&start));

                // set main span to the operator
                self.tree.set_main_span(left_expression_id, operator_span);

                // use the subject identifier for type predicate spans
                if matches!(
                    self.tree.get(left_expression_id),
                    Expression::TypePredicate { .. }
                ) {
                    let subject_span = self
                        .tree
                        .get_main_span(subject_id)
                        .unwrap_or_else(|| self.tree.get_span(subject_id));
                    self.tree.set_main_span(left_expression_id, subject_span);
                }
            }
        }

        // value ternary after infix to keep lowest precedence
        // NOTE #Cleanup: having multiple ternary parse locations feels icky (but non-trivial to "fix")
        if !self.options.in_type
            && self.options.left_precedence.is_none()
            && (self.peek_is(TokenType::Maybe)
                || self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Maybe))
        {
            self.eat_newlines_maybe()?;
            self.bump(); // eat ?
            self.eat_newlines_maybe()?;
            let then_expression_id = self.with_options(
                self.options
                    .not_in_position()
                    .in_ternary_condition()
                    .not_in_sequence_expression(),
                |parser| parser.eat_expression(),
            )?;
            self.eat_newlines_maybe()?;
            self.eat_colon()?;
            self.eat_newlines_maybe()?;
            let else_expression_id = self.with_options(
                self.options.not_in_position().not_in_sequence_expression(),
                |parser| parser.eat_expression(),
            )?;
            let expression = Expression::If {
                kind: IfKind::Ternary,
                condition: IfCondition::Expression {
                    condition: left_expression_id,
                },
                then_expression: then_expression_id,
                else_expression: Some(else_expression_id),
            };
            left_expression_id = self.tree.insert(expression, self.get_span_from(&start));
        }

        // sequence expression (comma operator) in JS/TS
        if !self.options.in_type
            && self.options.left_precedence.is_none()
            && self.options.allow_sequence_expression
            && (self.language.is_typescript() || self.language.is_javascript())
            && (self.peek_is(TokenType::Comma)
                || self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Comma))
        {
            let mut expressions = vec![left_expression_id];
            loop {
                if self.peek_is(TokenType::Newline) {
                    let has_comma_after_newlines = self
                        .peek_token_after_newlines(self.pos(), TokenType::Comma)
                        .is_ok();
                    if !has_comma_after_newlines {
                        break;
                    }
                    self.eat_newlines_maybe()?;
                }
                if !self.peek_is(TokenType::Comma) {
                    break;
                }
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;
                let expression_id = self.with_options(
                    self.options.not_in_position().not_in_sequence_expression(),
                    |parser| parser.eat_expression(),
                )?;
                expressions.push(expression_id);
            }

            let expression = Expression::SequenceExpression { expressions };
            left_expression_id = self.tree.insert(expression, self.get_span_from(&start));
        }

        // type conditional expression
        if self.options.in_type
            && (self.peek_is(TokenType::Maybe)
                || self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Maybe))
        {
            // avoid consuming nested conditional tokens in the right side
            let conditional_operands = self.split_type_conditional_operands(left_expression_id);
            if self.options.in_type_conditional_right && conditional_operands.is_none() {
                return Ok(left_expression_id);
            }
            let optional_tuple_pos = if self.peek_is(TokenType::Maybe) {
                Some(self.pos())
            } else if self.peek_is(TokenType::Newline) && self.peek_next_is(TokenType::Maybe) {
                Some(self.pos().saturating_add(1))
            } else {
                None
            };
            let is_optional_tuple = optional_tuple_pos.is_some_and(|pos| {
                self.peek_token_after_newlines(pos, TokenType::Comma)
                    .is_ok()
                    || self
                        .peek_token_after_newlines(pos, TokenType::CloseBracket)
                        .is_ok()
            });
            let Some((left, right)) = conditional_operands else {
                if is_optional_tuple {
                    return Ok(left_expression_id);
                }
                return Err(ParseError::unexpected(self.peek()?.span));
            };

            self.eat_newlines_maybe()?;
            self.bump(); // eat ?
            self.eat_newlines_maybe()?;
            let then_expression_id = self.with_options(
                self.options
                    .not_in_position()
                    .in_type()
                    .in_ternary_condition(),
                |parser| parser.eat_expression(),
            )?;
            self.eat_newlines_maybe()?;
            self.eat_colon()?;
            self.eat_newlines_maybe()?;
            let mut else_options = self.options.not_in_position().in_type();
            if self.options.in_type_conditional_right {
                else_options = else_options.in_type_conditional_right();
            }
            let else_expression_id =
                self.with_options(else_options, |parser| parser.eat_expression())?;
            let expression = Expression::TypeConditional {
                left,
                right,
                then_type: then_expression_id,
                else_type: else_expression_id,
            };
            left_expression_id = self.tree.insert(expression, self.get_span_from(&start));
        }

        Ok(left_expression_id)
    }
}
#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, AssignOperator, Asynchrony, BinaryOperator, Block, Declaration,
        DeclarationDescriptor, Declarator, DependencyItem, DependencyKind, DependencyMode,
        EnumField, EnumKind, Expression, FunctionKind, IfCondition, IfKind, ImportAliasTarget,
        ImportSource, IntType, Key, Mutability, Name, Parameter, Pattern, PatternField,
        PostfixPosition, Property, ScalarLiteral, TypeBinaryOperator, TypeLiteral,
        TypePredicateSubject, TypeUnaryOperator, UnaryOperator, VarianceBound,
    };
    use destack_source::LanguageType;

    use crate::{
        TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
    };

    /// Disambiguate using import meta as a path.
    #[test]
    fn test_parse_import_as_path() {
        let mut test = TestParser::new("import.meta.env");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        assert_expression_path!(parser, parser.tree.get(expression_id), "import.meta.env");
    }

    /// Parse a bare this expression.
    #[test]
    fn test_parse_this_expression() {
        let mut test = TestParser::new("this");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::This);
    }

    /// Parse a bare super expression.
    #[test]
    fn test_parse_super_expression() {
        let mut test = TestParser::new_with_options("super", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::Super);
    }

    /// Parse super member access.
    #[test]
    fn test_parse_super_member_expression() {
        let mut test = TestParser::new_with_options("super.value", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // super.value
        assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
            assert_node!(parser.tree, *left, Expression::Super);
            assert_string!(parser, *name, "value");
        });
    }

    /// Parse this member access in variant context.
    #[test]
    fn test_parse_this_member_expression_in_variant_context() {
        let mut test =
            TestParser::new_with_options("this.port1.onmessage", LanguageType::TypeScript);
        let mut parser = test.prepare();
        parser.options.in_variant = true;
        let expression_id = parser.eat_expression().unwrap();

        // this.port1.onmessage
        assert_node!(parser.tree, expression_id, Expression::Member { left, name, .. } => {
            assert_string!(parser, *name, "onmessage");
            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "port1");
                assert_node!(parser.tree, *left, Expression::This);
            });
        });
    }

    /// Parse this member access in call arguments in variant context.
    #[test]
    fn test_parse_call_argument_this_member_expression_in_variant_context() {
        let mut test = TestParser::new_with_options(
            "setTimeout(this.port1.onmessage, 0)",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        parser.options.in_variant = true;
        let expression_id = parser.eat_expression().unwrap();

        // setTimeout(this.port1.onmessage, 0)
        assert_node!(parser.tree, expression_id, Expression::Call { dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 2);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                // this.port1.onmessage
                assert_node!(parser.tree, *value, Expression::Member { left, name, .. } => {
                    assert_string!(parser, *name, "onmessage");
                    assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                        assert_string!(parser, *name, "port1");
                        assert_node!(parser.tree, *left, Expression::This);
                    });
                });
            });
        });
    }

    /// Parse a private identifier used in an in expression.
    #[test]
    fn test_parse_private_identifier_in_expression() {
        let mut test = TestParser::new_with_options("#a in this", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        // #a in this
        assert_node!(parser.tree, expression_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::In);
            assert_node!(parser.tree, *left, Expression::PrivateIdentifier { name } => {
                assert_string!(parser, *name, "a");
            });
            assert_node!(parser.tree, *right, Expression::This);
        });
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

    /// Parse an if extends condition without consuming the block.
    #[test]
    fn test_parse_if_extends_type_reference() {
        let mut test = TestParser::new(
            r#"if x extends Foo {
    body
}"#,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::If { condition, then_expression, else_expression, .. } => {
            assert!(else_expression.is_none());
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            // x extends Foo
            assert_node!(parser.tree, condition_id, Expression::TypeBinary { left, operator, right } => {
                assert_expression_path!(parser, parser.tree.get(*left), "x");
                assert_eq!(*operator, TypeBinaryOperator::Extends);
                assert_expression_path!(parser, parser.tree.get(*right), "Foo");
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

    /// Parse an if instanceof condition inside parentheses.
    #[test]
    fn test_parse_if_instanceof_type_reference() {
        let mut test = TestParser::new(
            r#"if (T instanceof Foo) {
    value
}"#,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::If { condition, then_expression, else_expression, .. } => {
            assert!(else_expression.is_none());
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            // T instanceof Foo
            assert_node!(parser.tree, condition_id, Expression::Binary { left, operator, right } => {
                assert_expression_path!(parser, parser.tree.get(*left), "T");
                assert_eq!(*operator, BinaryOperator::InstanceOf);
                assert_expression_path!(parser, parser.tree.get(*right), "Foo");
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
                assert_string!(parser, name.string(), "bar");
                assert!(alias.is_none());
            });
            // baz
            assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, name.string(), "baz");
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
                assert_string!(parser, name.string(), "bar");
                assert!(alias.is_none());
            });
            // baz
            assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, name.string(), "baz");
                assert!(alias.is_none());
            });
        });
    }

    /// Parse `export type { Foo, Bar } from "module"`.
    #[test]
    fn test_parse_export_expression_type_items_with_target() {
        let mut test = TestParser::new("export type { Foo, Bar } from \"module\"");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Type);
            assert_string!(parser, *target, "module");
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, name.string(), "Foo");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, name.string(), "Bar");
                assert!(alias.is_none());
            });
        });
    }

    /// Parse `export type { Foo }`.
    #[test]
    fn test_parse_export_expression_type_items_without_target() {
        let mut test = TestParser::new("export type { Foo }");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Export { kind, target: None, items, .. } => {
            assert_eq!(*kind, DependencyKind::Type);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, name.string(), "Foo");
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

    #[test]
    fn test_reject_export_type_without_binding_or_declaration() {
        let mut test = TestParser::new("export type");
        let mut parser = test.prepare();
        let result = parser.eat_expression();
        assert!(result.is_err());
    }

    /// Reject bare export path expressions in JavaScript.
    #[test]
    fn test_reject_export_path_expression_javascript() {
        // source: export foo
        let mut test = TestParser::new_with_options("export foo", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let error = parser.eat_expression().unwrap_err();

        // foo
        assert_eq!(parser.get_span_str(error.leaf_span()), "foo");
    }

    /// Reject bare export path expressions in Destack.
    #[test]
    fn test_reject_export_path_expression_destack() {
        // source: export foo
        let mut test = TestParser::new_with_options("export foo", LanguageType::Destack);
        let mut parser = test.prepare();
        let error = parser.eat_expression().unwrap_err();

        // foo
        assert_eq!(parser.get_span_str(error.leaf_span()), "foo");
    }

    #[test]
    fn test_reject_export_default_enum() {
        let mut test = TestParser::new("export default enum A { X, Y, Z }");
        let mut parser = test.prepare();
        let result = parser.eat_expression();
        assert!(result.is_err());
    }

    /// Parse `export import foo = bar.baz`.
    #[test]
    fn test_parse_export_import_equals() {
        let mut test = TestParser::new("export import atob = globalThis.atob");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
                assert!(descriptor.export.is_some());
                assert_eq!(*kind, DependencyKind::Value);
                let name = descriptor.name.expect("import alias name");
                assert_string!(parser, name.string(), "atob");
                match target {
                    ImportAliasTarget::Path { value } => {
                        assert_expression_path!(parser, parser.tree.get(*value), "globalThis.atob");
                    }
                    ImportAliasTarget::Require { .. } => {
                        panic!("expected import alias path");
                    }
                }
            });
        });
    }

    /// Parse `export import type React = require("react")`.
    #[test]
    fn test_parse_export_import_type_equals_require() {
        let mut test = TestParser::new(r#"export import type React = require("react")"#);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
                assert!(descriptor.export.is_some());
                assert_eq!(*kind, DependencyKind::Type);
                let name = descriptor.name.expect("import alias name");
                assert_string!(parser, name.string(), "React");
                match target {
                    ImportAliasTarget::Require { target } => {
                        assert_string!(parser, *target, "react");
                    }
                    ImportAliasTarget::Path { .. } => {
                        panic!("expected import alias require");
                    }
                }
            });
        });
    }

    #[test]
    fn test_parse_export_import_type_equals_require_with_newlines() {
        let mut test = TestParser::new_with_options(
            r#"
export
import
type
React = require("react")
"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
                assert!(descriptor.export.is_some());
                assert_eq!(*kind, DependencyKind::Type);
                let name = descriptor.name.expect("import alias name");
                assert_string!(parser, name.string(), "React");
                match target {
                    ImportAliasTarget::Require { target } => {
                        assert_string!(parser, *target, "react");
                    }
                    ImportAliasTarget::Path { .. } => {
                        panic!("expected import alias require");
                    }
                }
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
        assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: None, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert_eq!(items.len(), 2);
            // bar
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, name.string(), "bar");
                assert!(alias.is_none());
            });
            // baz
            assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, name.string(), "baz");
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
        assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: Some(arguments), .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
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

    /// Parse `import("foo")`.
    #[test]
    fn test_parse_import_call_expression() {
        let mut test = TestParser::new("import(\"foo\")");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: None, .. } => {
            assert_eq!(*source, ImportSource::ImportCall);
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert!(items.is_empty());
        });
    }

    /// Parse `import("foo", { assert: { type: "json" } })`.
    #[test]
    fn test_parse_import_call_with_assertions() {
        let mut test = TestParser::new("import(\"foo\", { assert: { type: \"json\" } })");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, arguments: Some(arguments), .. } => {
            assert_eq!(*source, ImportSource::ImportCall);
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert!(items.is_empty());
            assert_eq!(arguments.len(), 1);
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
                    assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
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

    /// Parse a statement-position object literal with a comment.
    #[test]
    fn test_parse_statement_position_object_literal_with_comment() {
        let mut test = TestParser::new("{ /* key */ a: 1 }");
        let mut parser = test.prepare();
        parser.options.in_statement_position = true;
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::ObjectExpression { ty: None, properties, .. } => {
            assert_eq!(properties.len(), 1);
            assert_node!(parser.tree, properties[0], Property::Field { modifiers: _, key: Some(Key::Name(Name::Identifier(name))), value: Some(value), default: None, .. } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
    }

    /// Parse a statement-position object literal with a computed key.
    #[test]
    fn test_parse_statement_position_object_literal_computed_key() {
        let mut test = TestParser::new("{ [key]: value }");
        let mut parser = test.prepare();
        parser.options.in_statement_position = true;
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::ObjectExpression { ty: None, properties, .. } => {
            assert_eq!(properties.len(), 1);
            assert_node!(parser.tree, properties[0], Property::Field { modifiers: _, key: Some(Key::Expression(key_id)), value: Some(value_id), default: None, .. } => {
                assert_expression_path!(parser, parser.tree.get(*key_id), "key");
                assert_expression_path!(parser, parser.tree.get(*value_id), "value");
            });
        });
    }

    /// Parse a statement-position block with assignments.
    #[test]
    fn test_parse_statement_position_block_with_assignment() {
        let mut test = TestParser::new("{ step = step + 1; return base + step; }");
        let mut parser = test.prepare();
        parser.options.in_statement_position = true;
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 2);
                assert_node!(parser.tree, expressions[0], Expression::Statement(_));
                assert_node!(parser.tree, expressions[1], Expression::Statement(statement_id) => {
                    assert_node!(parser.tree, *statement_id, Expression::Return { .. });
                });
            });
        });
    }

    /// Parse a statement-position block with an array literal.
    #[test]
    fn test_parse_statement_position_block_with_array_literal() {
        let mut test = TestParser::new("{ [] }");
        let mut parser = test.prepare();
        parser.options.in_statement_position = true;
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Block(block_id) => {
            assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
                let expression_id = match parser.tree.get(expressions[0]) {
                    Expression::Statement(statement_id) => *statement_id,
                    _ => expressions[0],
                };
                assert_node!(parser.tree, expression_id, Expression::ArrayExpression { elements } => {
                    assert!(elements.is_empty());
                });
            });
        });
    }

    /// Prefer a block over a computed method object literal in statement position.
    #[test]
    fn test_parse_statement_position_computed_method_as_block() {
        let mut test = TestParser::new("{ [key]()\n{} }");
        let mut parser = test.prepare();
        parser.options.in_statement_position = true;
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Block(_) => {});
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

    /// Parse an object literal in parenthesis.
    #[test]
    fn test_parse_object_literal_in_parenthesis() {
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
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            assert_node!(parser.tree, *then_expression, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            assert_node!(parser.tree, else_expression.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });
    }

    /// Parse a ternary if expression over multiple lines.
    #[test]
    fn test_parse_if_ternary_multiline() {
        let mut test = TestParser::new(
            r#"true
    ? 1
    : 2"#,
        );
        let mut parser = test.prepare();
        let if_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
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
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_expression_path!(parser, parser.tree.get(condition_id), "cond");
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
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::Path { path, .. } => {
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
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::Path { path, .. } => {
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
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::Path { path, .. } => {
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

    /// Parse a ternary if expression with a binary condition.
    #[test]
    fn test_parse_if_ternary_with_binary_condition() {
        let mut test = TestParser::new("x == 0 ? 1 : 2");
        let mut parser = test.prepare();
        let if_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, if_id, Expression::If { condition, then_expression, else_expression, .. } => {
            let condition_id = match condition {
                IfCondition::Expression { condition } => *condition,
                IfCondition::Let { .. } => panic!("expected expression condition"),
            };
            assert_node!(parser.tree, condition_id, Expression::Binary { left, operator, right } => {
                assert_eq!(*operator, BinaryOperator::Equal);
                assert_node!(parser.tree, *left, Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "x");
                });
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
            });
            assert_node!(parser.tree, *then_expression, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            assert_node!(parser.tree, else_expression.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
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

    /// Parse optional chaining after comment-separated newlines.
    #[test]
    fn test_parse_optional_chain_after_comment_newlines() {
        let input = "promise\n  .then(noop)\n  // comment\n  // comment\n  ?.catch(noop)";
        let mut test = TestParser::new_with_options(input, LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(parser.errors.is_empty());
        assert_eq!(expressions.len(), 1);

        let statement_id = match parser.tree.get(expressions[0]) {
            Expression::Statement(expression_id) => *expression_id,
            _ => expressions[0],
        };

        assert_node!(parser.tree, statement_id, Expression::Call { left, dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 1);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { modifiers: _, value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "noop");
            });

            assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                assert_string!(parser, *name, "catch");
                assert_node!(parser.tree, *left, Expression::Maybe { left: maybe_left, position: PostfixPosition::Direct } => {
                    assert_node!(parser.tree, *maybe_left, Expression::Call { .. });
                });
            });
        });
    }

    #[test]
    fn test_parse_instantiation_expression_with_index() {
        let mut test = TestParser::new_with_options("f[\"g\"]<number>", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Instantiation { left, static_arguments } => {
            assert_eq!(static_arguments.len(), 1);
            assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
            });
            assert_node!(parser.tree, *left, Expression::Index { left, index, position: PostfixPosition::Direct } => {
                assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                    assert!(static_arguments.is_none());
                    assert_path!(parser, *path, "f");
                });
                let index = index.expect("expected index expression");
                assert_node!(parser.tree, index, Expression::ScalarLiteral(ScalarLiteral::String(name)) => {
                    assert_string!(parser, *name, "g");
                });
            });
        });
    }

    #[test]
    fn test_parse_instantiation_expression_parenthesized() {
        let mut test =
            TestParser::new_with_options("(f<number>)<number>", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Instantiation { left, static_arguments } => {
            assert_eq!(static_arguments.len(), 1);
            assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::TypeLiteral(TypeLiteral::Number));
            });
            assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Instantiation { left, static_arguments } => {
                    assert_eq!(static_arguments.len(), 1);
                    assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                        assert_path!(parser, *path, "f");
                        assert!(static_arguments.is_none());
                    });
                });
            });
        });
    }

    /// Instantiation expressions can appear as assignment targets in parse output.
    #[test]
    fn test_parse_instantiation_expression_assignment() {
        let mut test = TestParser::new_with_options("f<T> = g", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Assign { left, right, .. } => {
            assert_node!(parser.tree, *left, Expression::Instantiation { left, static_arguments } => {
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                        assert!(static_arguments.is_none());
                        assert_path!(parser, *path, "T");
                    });
                });
                assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                    assert!(static_arguments.is_none());
                    assert_path!(parser, *path, "f");
                });
            });
            assert_expression_path!(parser, parser.tree.get(*right), "g");
        });
    }

    /// Instantiation expressions with members remain assignable targets in parse output.
    #[test]
    fn test_parse_instantiation_expression_member_assignment() {
        let mut test = TestParser::new_with_options("cls.myFunc<T> = g", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Assign { left, right, .. } => {
            assert_node!(parser.tree, *left, Expression::Instantiation { left, static_arguments } => {
                assert_eq!(static_arguments.len(), 1);
                assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                    assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                        assert!(static_arguments.is_none());
                        assert_path!(parser, *path, "T");
                    });
                });
                assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                    assert!(static_arguments.is_none());
                    assert_path!(parser, *path, "cls.myFunc");
                });
            });
            assert_expression_path!(parser, parser.tree.get(*right), "g");
        });
    }

    /// Parse parenthesized instantiation receivers before member access.
    #[test]
    fn test_parse_instantiation_expression_member_access_with_parentheses() {
        let mut test = TestParser::new_with_options("(f<T>).x", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Member { left, name, static_arguments } => {
            assert!(static_arguments.is_none());
            assert_string!(parser, *name, "x");
            assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Instantiation { left, static_arguments } => {
                    assert_eq!(static_arguments.len(), 1);
                    assert_node!(parser.tree, static_arguments[0], Argument::Positional { value, .. } => {
                        assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                            assert!(static_arguments.is_none());
                            assert_path!(parser, *path, "T");
                        });
                    });
                    assert_node!(parser.tree, *left, Expression::Path { path, static_arguments } => {
                        assert!(static_arguments.is_none());
                        assert_path!(parser, *path, "f");
                    });
                });
            });
        });
    }

    /// Instantiation expressions should parse in mixed operator contexts.
    #[test]
    fn test_parse_instantiation_expression_more_exprs() {
        let mut test = TestParser::new_with_options(
            r#"
f<x>, g<y>;
[f<x>];
f<x> ? g<y> : h<z>;
f<x> ^ g<y>;
f<x> & g<y>;
f<x> | g<y>;
f<x> && g<y>;
f<x> || g<y>;
{ f<x> }
f<x> ?? g<y>;
f<x> == g<y>;
f<x> === g<y>;
f<x> != g<y>;
f<x> !== g<y>;
"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();
        assert_eq!(expressions.len(), 14);

        // f<x>, g<y>
        let sequence_id = match parser.tree.get(expressions[0]) {
            Expression::Statement(expression_id) => *expression_id,
            _ => expressions[0],
        };
        assert_node!(parser.tree, sequence_id, Expression::SequenceExpression { expressions } => {
            assert_eq!(expressions.len(), 2);
            assert_node!(parser.tree, expressions[0], Expression::Instantiation { left, static_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "f");
                assert_eq!(static_arguments.len(), 1);
            });
            assert_node!(parser.tree, expressions[1], Expression::Instantiation { left, static_arguments } => {
                assert_expression_path!(parser, parser.tree.get(*left), "g");
                assert_eq!(static_arguments.len(), 1);
            });
        });

        // [f<x>]
        let array_id = match parser.tree.get(expressions[1]) {
            Expression::Statement(expression_id) => *expression_id,
            _ => expressions[1],
        };
        assert_node!(parser.tree, array_id, Expression::ArrayExpression { elements } => {
            assert_eq!(elements.len(), 1);
            assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Instantiation { left, static_arguments } => {
                    assert_expression_path!(parser, parser.tree.get(*left), "f");
                    assert_eq!(static_arguments.len(), 1);
                });
            });
        });

        // f<x> ? g<y> : h<z>
        let ternary_id = match parser.tree.get(expressions[2]) {
            Expression::Statement(expression_id) => *expression_id,
            _ => expressions[2],
        };
        assert_node!(parser.tree, ternary_id, Expression::If { kind, .. } => {
            assert_eq!(*kind, IfKind::Ternary);
        });

        // f<x> ?? g<y>
        let coalesce_id = match parser.tree.get(expressions[9]) {
            Expression::Statement(expression_id) => *expression_id,
            _ => expressions[9],
        };
        assert_node!(parser.tree, coalesce_id, Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::Coalesce);
        });

        // f<x> !== g<y>
        let strict_not_equal_id = match parser.tree.get(expressions[13]) {
            Expression::Statement(expression_id) => *expression_id,
            _ => expressions[13],
        };
        assert_node!(parser.tree, strict_not_equal_id, Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::NotEqualStrict);
        });
    }

    /// Parse a TypeScript call with string literal type arguments.
    #[test]
    fn test_parse_call_with_string_literal_type_arguments() {
        let mut test = TestParser::new_with_options(
            "accessor.getValue<\"auto\" | \"always\" | \"never\">(\"long\")",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
            assert_eq!(dynamic_arguments.len(), 1);
            let mut static_args = static_arguments.as_ref();
            if static_args.is_none()
                && let Expression::Member { name, static_arguments: Some(member_args), .. } =
                    parser.tree.get(*left)
                {
                    assert_string!(parser, *name, "getValue");
                    static_args = Some(member_args);
                }
            let static_args = static_args.expect("expected static arguments on call or member");
            assert_eq!(static_args.len(), 1);
            assert_node!(parser.tree, static_args[0], Argument::Positional { modifiers: _, value } => {
                assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                });
            });
        });
    }

    /// Parse a TypeScript call with shift-left static arguments.
    #[test]
    fn test_parse_call_with_shift_left_static_arguments() {
        let mut test =
            TestParser::new_with_options("f<<T>(v: T) => void>()", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "f");
            assert!(dynamic_arguments.is_empty());
            let static_args = static_arguments.as_ref().expect("expected static arguments");
            assert_eq!(static_args.len(), 1);
            assert_node!(parser.tree, static_args[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                        assert_eq!(signature.kind, FunctionKind::Lambda);
                        assert!(body.is_none());
                    });
                });
            });
        });
    }

    /// Parse shift-left static arguments in decorator context.
    #[test]
    fn test_parse_call_with_shift_left_static_arguments_in_decorator_context() {
        let mut test =
            TestParser::new_with_options("f<<T>(v: T) => void>()", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let options = parser
            .options
            .not_in_position()
            .in_left_precedence(u16::MAX)
            .not_in_sequence_expression()
            .in_decorator();
        let expr_id = parser
            .with_options(options, |parser| parser.eat_expression())
            .unwrap();
        assert_node!(parser.tree, expr_id, Expression::Call { left, static_arguments, dynamic_arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "f");
            assert!(dynamic_arguments.is_empty());
            let static_args = static_arguments.as_ref().expect("expected static arguments");
            assert_eq!(static_args.len(), 1);
        });
    }

    /// Parse a TypeScript arrow function parameter named `accessor`.
    #[test]
    fn test_parse_arrow_parameter_accessor_name() {
        let mut test = TestParser::new_with_options(
            "(accessor: ServicesAccessor) => accessor.get()",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                assert_eq!(signature.dynamic_parameters.len(), 1);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "accessor");
                    assert_expression_path!(parser, parser.tree.get(ty.unwrap()), "ServicesAccessor");
                });
            });
        });
    }

    /// Parse a TypeScript class expression with implements.
    #[test]
    fn test_parse_class_expression_with_implements() {
        let mut test = TestParser::new_with_options(
            "new (class implements Foo {})()",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::New { left, .. } => {
            assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, .. } => {
                        assert!(heritage.implements_types.is_some());
                    });
                });
            });
        });
    }

    /// Parse a TypeScript class expression when heritage starts on the next line.
    #[test]
    fn test_parse_class_expression_with_newline_implements() {
        let mut test = TestParser::new_with_options(
            "new (class\n  implements Foo\n{})()",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::New { left, .. } => {
            assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, .. } => {
                        assert!(heritage.implements_types.is_some());
                    });
                });
            });
        });
    }

    /// Parse a TypeScript class expression with multiline extends heritage.
    #[test]
    fn test_parse_class_expression_with_newline_extends() {
        let mut test = TestParser::new_with_options(
            "new (class\n  extends Foo<Bar>\n{})()",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::New { left, .. } => {
            assert_node!(parser.tree, *left, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Class { heritage, .. } => {
                        assert!(heritage.extends_types.is_some());
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

    /// Parse a generic lambda function value with a body.
    #[test]
    fn test_parse_generic_lambda_function_value() {
        let mut test = TestParser::new("<T,>(x: T): T => x");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                let generics = signature.generics.as_ref().expect("expected generics");
                let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
                assert_eq!(static_parameters.len(), 1);
                assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty: None, default: None, .. } => {
                    assert_string!(parser, *name, "T");
                });
                assert_eq!(signature.dynamic_parameters.len(), 1);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "x");
                    assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "T");
                    });
                });
                assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
                assert_node!(parser.tree, body.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "x");
                });
            });
        });
    }

    /// Parse a generic lambda function with a newline after `<`.
    #[test]
    fn test_parse_generic_lambda_function_value_multiline_after_less_than() {
        let mut test = TestParser::new_with_options(
            "<\nT extends string\n>(x: T) => x",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                let generics = signature.generics.as_ref().expect("expected generics");
                let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
                assert_eq!(static_parameters.len(), 1);
                assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "T");
                    assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
                });
                assert_eq!(signature.dynamic_parameters.len(), 1);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "x");
                    assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "T");
                    });
                });
                assert_node!(parser.tree, body.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "x");
                });
            });
        });
    }

    /// TSX generic arrows with extends constraints should parse as functions.
    #[test]
    fn test_parse_tsx_generic_arrow_with_extends() {
        let mut test = TestParser::new_with_options(
            "<P extends object>(x: P) => <Foo />",
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                let generics = signature.generics.as_ref().expect("expected generics");
                let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
                assert_eq!(static_parameters.len(), 1);
                assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "P");
                    assert_node!(parser.tree, ty.unwrap(), Expression::TypeLiteral(TypeLiteral::Object));
                });
                assert_eq!(signature.dynamic_parameters.len(), 1);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "x");
                    assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "P");
                    });
                });
                let body_id = body.expect("expected body");
                assert_node!(parser.tree, body_id, Expression::TreeExpression { left, arguments, elements } => {
                    let left_id = left.expect("expected tag");
                    assert_node!(parser.tree, left_id, Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "Foo");
                    });
                    assert!(arguments.as_ref().is_none_or(|items| items.is_empty()));
                    assert!(elements.as_ref().is_none_or(|items| items.is_empty()));
                });
            });
        });
    }

    /// Parse TSX generic arrows without explicit disambiguators.
    #[test]
    fn test_parse_tsx_generic_arrow_without_disambiguator() {
        let mut test = TestParser::new_with_options("<R>(x: R) => x", LanguageType::TypeScriptXml);
        let mut parser = test.prepare();

        // ambiguous TSX generics should not parse without disambiguators
        let result = parser.eat_expression();
        assert!(result.is_err());
    }

    /// Parse TSX generic arrows with trailing comma disambiguators.
    #[test]
    fn test_parse_tsx_generic_arrow_with_trailing_comma() {
        let mut test =
            TestParser::new_with_options("<T,>(x: T): T => x", LanguageType::TypeScriptXml);
        let mut parser = test.prepare();

        // parse a generic lambda with disambiguated type parameters
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);
                let generics = signature.generics.as_ref().expect("expected generics");
                let static_parameters = generics.static_parameters.as_ref().expect("expected static parameters");
                assert_eq!(static_parameters.len(), 1);
                assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, ty: None, default: None, .. } => {
                    assert_string!(parser, *name, "T");
                });
                assert_eq!(signature.dynamic_parameters.len(), 1);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, ty, .. } => {
                    assert_string!(parser, *name, "x");
                    assert_node!(parser.tree, ty.unwrap(), Expression::Path { path, .. } => {
                        assert_path!(parser, *path, "T");
                    });
                });
                assert_node!(parser.tree, signature.return_type.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
                assert_node!(parser.tree, body.unwrap(), Expression::Path { path, .. } => {
                    assert_path!(parser, *path, "x");
                });
            });
        });
    }

    /// Parse ternaries with typed arrow functions in TSX context.
    #[test]
    fn test_parse_tsx_ternary_typed_arrow_function() {
        let mut test = TestParser::new_with_options(
            "Math.random() > 0.5 ? (): void => foo() : (): void => bar()",
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::If { kind, condition, then_expression, else_expression } => {
            assert_eq!(*kind, IfKind::Ternary);
            assert_node!(condition, IfCondition::Expression { condition } => {
                assert_node!(parser.tree, *condition, Expression::Binary { operator, .. } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                });
            });
            assert_node!(parser.tree, *then_expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                    assert_eq!(signature.kind, FunctionKind::Lambda);
                    assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
                });
            });
            let else_id = else_expression.expect("expected else branch");
            assert_node!(parser.tree, else_id, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                    assert_eq!(signature.kind, FunctionKind::Lambda);
                    assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypeLiteral(TypeLiteral::Void));
                });
            });
        });
    }

    /// Parse TSX tree attributes with typed arrow function values.
    #[test]
    fn test_parse_tsx_tree_attribute_typed_arrow_value() {
        let mut test = TestParser::new_with_options(
            "<StyledComponent className={({ theme }): { [key: string]: any } => ({ color: theme.blue })} />",
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::TreeExpression { arguments, .. } => {
            let arguments = arguments.as_ref().expect("expected arguments");
            let class_name_argument = arguments.iter().copied().find(|argument_id| {
                matches!(
                    parser.tree.get(*argument_id),
                    Argument::Named { name, .. } if parser.strings.get(name.string()) == "className"
                )
            });
            let class_name_argument = class_name_argument.expect("expected className argument");
            assert_node!(parser.tree, class_name_argument, Argument::Named { name, value, .. } => {
                assert_name!(parser, *name, "className");
                assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                        assert_eq!(signature.kind, FunctionKind::Lambda);
                        let return_type = signature.return_type.expect("expected return type");
                        assert_node!(parser.tree, return_type, Expression::ObjectExpression { .. });
                    });
                });
            });
        });
    }

    /// Parse ternaries with typed arrow functions inside TSX tree attributes.
    #[test]
    fn test_parse_tsx_ternary_tree_attribute_typed_arrow() {
        let mut test = TestParser::new_with_options(
            "disabled ? <StyledComponent className={({ theme }): { [key: string]: any } => ({ color: theme.blue })} /> : null",
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::If { kind, then_expression, else_expression, .. } => {
            assert_eq!(*kind, IfKind::Ternary);
            assert_node!(parser.tree, *then_expression, Expression::TreeExpression { arguments, .. } => {
                let arguments = arguments.as_ref().expect("expected arguments");
                let class_name_argument = arguments.iter().copied().find(|argument_id| {
                    matches!(
                        parser.tree.get(*argument_id),
                        Argument::Named { name, .. } if parser.strings.get(name.string()) == "className"
                    )
                });
                let class_name_argument = class_name_argument.expect("expected className argument");
                assert_node!(parser.tree, class_name_argument, Argument::Named { name, value, .. } => {
                    assert_name!(parser, *name, "className");
                    assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                        assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, .. } => {
                            assert_eq!(signature.kind, FunctionKind::Lambda);
                            let return_type = signature.return_type.expect("expected return type");
                            assert_node!(parser.tree, return_type, Expression::ObjectExpression { .. });
                        });
                    });
                });
            });
            assert!(else_expression.is_some());
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

    /// Comparison operators should not be parsed as static arguments.
    #[test]
    fn test_parse_static_arguments_disambiguate_relational() {
        let mut test = TestParser::new("fn(x < y, x > y)");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Call { left, dynamic_arguments, .. } => {
            assert_expression_path!(parser, parser.tree.get(*left), "fn");
            assert_eq!(dynamic_arguments.len(), 2);
            assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                    assert_eq!(*operator, BinaryOperator::LessThan);
                });
            });
            assert_node!(parser.tree, dynamic_arguments[1], Argument::Positional { value, .. } => {
                assert_node!(parser.tree, *value, Expression::Binary { operator, .. } => {
                    assert_eq!(*operator, BinaryOperator::GreaterThan);
                });
            });
        });
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
                    assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
                        assert_node!(parser.tree, *value, Expression::Path { path, static_arguments } => {
                            assert_path!(parser, *path, "B");
                            assert!(static_arguments.is_some());
                            // C
                            assert_node!(parser.tree, static_arguments.as_ref().unwrap()[0], Argument::Positional { modifiers: _, value } => {
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
        let options = LanguageType::JavaScript;
        let mut test = TestParser::new_with_options("*x", options);
        let mut parser = test.prepare();
        // Should fail to parse *x as dereference in JS mode
        let result = parser.eat_expression();
        assert!(result.is_err());
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
            Expression::ReferenceOf { mutability: Some(mutability), right, .. } => {
                assert_eq!(*mutability, Mutability::Mutable);
                // x
                assert_expression_path!(parser, parser.tree.get(*right), "x");
            }
        );
    }

    /// Parse a reference to a member call.
    #[test]
    fn test_parse_reference_member_call() {
        let mut test = TestParser::new("&self.foo()");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // &self.foo()
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
        let mut test = TestParser::new("&readonly super T");
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
        let mut test = TestParser::new("^super T");
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

    /// Parse boolean IdentifierName member access in JavaScript.
    #[test]
    fn test_parse_member_boolean_identifier_name() {
        // source: a.true
        let mut test = TestParser::new_with_options("a.true", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // a.true
        assert_node!(parser.tree, expr_id, Expression::Member { name, .. } => {
            assert_string!(parser, *name, "true");
        });
    }

    /// Parse default IdentifierName member access in JavaScript.
    #[test]
    fn test_parse_member_default_identifier_name_after_parenthesized_await_import() {
        let mut test = TestParser::new_with_options(
            r#"(await import(join("file://", process.argv[2]))).default"#,
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // (await import(join("file://", process.argv[2]))).default
        assert_node!(parser.tree, expr_id, Expression::Member { name, .. } => {
            assert_string!(parser, *name, "default");
        });
    }

    /// Parse boolean IdentifierName property keys and accessors in JavaScript.
    #[test]
    fn test_parse_object_boolean_identifier_name_keys() {
        // source: { true: 1, false: 2, get true() {}, set false(value) {} }
        let mut test = TestParser::new_with_options(
            "{ true: 1, false: 2, get true() {}, set false(value) {} }",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // { true: 1, false: 2, get true() {}, set false(value) {} }
        assert_node!(parser.tree, expr_id, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 4);

            assert_node!(parser.tree, properties[0], Property::Field { key, value, .. } => {
                assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                    assert_string!(parser, *name, "true");
                });
                assert_node!(parser.tree, value.expect("expected field value"), Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });

            assert_node!(parser.tree, properties[1], Property::Field { key, value, .. } => {
                assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                    assert_string!(parser, *name, "false");
                });
                assert_node!(parser.tree, value.expect("expected field value"), Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });

            assert_node!(parser.tree, properties[2], Property::Method { key, signature, .. } => {
                assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                    assert_string!(parser, *name, "true");
                });
                assert_eq!(signature.mode, Some(destack_ast::FunctionMode::Getter));
            });

            assert_node!(parser.tree, properties[3], Property::Method { key, signature, .. } => {
                assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                    assert_string!(parser, *name, "false");
                });
                assert_eq!(signature.mode, Some(destack_ast::FunctionMode::Setter));
            });
        });
    }

    /// Parse regex literal in export default.
    #[test]
    fn test_parse_export_default_regex_literal() {
        // source: export default /foo/
        let mut test =
            TestParser::new_with_options("export default /foo/", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // export default /foo/
        assert_node!(parser.tree, expr_id, Expression::Export { target, items, .. } => {
            assert!(target.is_none());
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { value: Some(value), .. } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
            });
        });
    }

    /// Parse regex literal after assign with a newline.
    #[test]
    fn test_parse_regex_literal_after_assign_newline() {
        // source: var match =\n/^foo$/i.exec(str)
        let mut test = TestParser::new_with_options(
            "var match =\n/^foo$/i.exec(str)",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // var match =\n/^foo$/i.exec(str)
        assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
                assert_node!(parser.tree, value.expect("expected initializer"), Expression::Call { left, dynamic_arguments, .. } => {
                    assert_eq!(dynamic_arguments.len(), 1);
                    assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                        assert_string!(parser, *name, "exec");
                        assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
                    });
                });
            });
        });
    }

    /// Parse regex literal after an arrow.
    #[test]
    fn test_parse_regex_literal_after_arrow() {
        // source: () => /^foo$/.test(value)
        let mut test =
            TestParser::new_with_options("() => /^foo$/.test(value)", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { body, .. } => {
                let body = body.expect("expected body");
                assert_node!(parser.tree, body, Expression::Call { left, dynamic_arguments, .. } => {
                    assert_eq!(dynamic_arguments.len(), 1);
                    assert_node!(parser.tree, *left, Expression::Member { left, name, .. } => {
                        assert_string!(parser, *name, "test");
                        assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
                    });
                });
            });
        });
    }

    /// Parse regex literal with slash inside a character class.
    #[test]
    fn test_parse_regex_literal_with_character_class_slash() {
        // source: var a = /[\]/]/
        let mut test = TestParser::new_with_options("var a = /[\\]/]/", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // var a = /[\]/]/
        assert_node!(parser.tree, expr_id, Expression::Let { declarators, .. } => {
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { value, .. } => {
                assert_node!(parser.tree, value.expect("expected initializer"), Expression::ScalarLiteral(ScalarLiteral::RegexString { .. }));
            });
        });
    }

    /// Reject regex unicode escapes beyond the valid unicode scalar range.
    #[test]
    fn test_reject_regex_unicode_escape_out_of_range() {
        // source: /\u{110000}/u
        let mut test = TestParser::new_with_options("/\\u{110000}/u", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let error = parser.eat_expression().unwrap_err();

        assert_eq!(error.leaf_span().start, 0);
    }

    /// Reject unicode regex decimal escapes without matching capture groups.
    #[test]
    fn test_reject_regex_unicode_invalid_decimal_escape() {
        // source: /\1/u
        let mut test = TestParser::new_with_options("/\\1/u", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let error = parser.eat_expression().unwrap_err();

        assert_eq!(error.leaf_span().start, 0);
    }

    /// Reject unicode regex literals with lone quantifier opening braces.
    #[test]
    fn test_reject_regex_unicode_lone_opening_quantifier_brace() {
        // source: /{*/u
        let mut test = TestParser::new_with_options("/{*/u", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let error = parser.eat_expression().unwrap_err();

        assert_eq!(error.leaf_span().start, 0);
    }

    /// Reject unicode regex literals with invalid quantified lookaheads.
    #[test]
    fn test_reject_regex_unicode_quantified_lookahead() {
        // source: /(?!.){0,}?/u
        let mut test = TestParser::new_with_options("/(?!.){0,}?/u", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let error = parser.eat_expression().unwrap_err();

        assert_eq!(error.leaf_span().start, 0);
    }

    /// Reject unicode regex literals with lone quantifier closing braces.
    #[test]
    fn test_reject_regex_unicode_lone_closing_quantifier_brace() {
        // source: /}?/u
        let mut test = TestParser::new_with_options("/}?/u", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let error = parser.eat_expression().unwrap_err();

        assert_eq!(error.leaf_span().start, 0);
    }

    /// Parse unicode regex property escapes.
    #[test]
    fn test_parse_regex_unicode_property_escape() {
        // source: /\p{Emoji}/u
        let mut test = TestParser::new_with_options("/\\p{Emoji}/u", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(
            parser.tree,
            expr_id,
            Expression::ScalarLiteral(ScalarLiteral::RegexString { .. })
        );
    }

    /// Parse regex unicode escapes with long leading-zero code point forms.
    #[test]
    fn test_parse_regex_unicode_escape_with_long_leading_zeros() {
        // source: /[\u{0000000000000061}-\u{7A}]/u
        let mut test = TestParser::new_with_options(
            "/[\\u{0000000000000061}-\\u{7A}]/u",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // /[\u{0000000000000061}-\u{7A}]/u
        assert_node!(
            parser.tree,
            expr_id,
            Expression::ScalarLiteral(ScalarLiteral::RegexString { .. })
        );
    }

    /// Parse string literal with long leading-zero code point escapes.
    #[test]
    fn test_parse_string_unicode_escape_with_long_leading_zeros() {
        // source: "\u{00000000034}"
        let mut test =
            TestParser::new_with_options("\"\\u{00000000034}\"", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        // "\u{00000000034}"
        assert_node!(parser.tree, expr_id, Expression::ScalarLiteral(ScalarLiteral::String(string_id)) => {
            assert_string!(parser, *string_id, "\\u{00000000034}");
        });
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
        let mut test = TestParser::new(
            r#"a +
 b +
 c"#,
        );
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

    /// Type casts bind to the left side before addition.
    #[test]
    fn test_parse_precedence_cast_before_addition() {
        let mut test = TestParser::new("a as number + b");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a as number + b
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::Add);
                // a as number
                assert_node!(
                    parser.tree,
                    *left,
                    Expression::TypeBinary { left, operator, right } => {
                        assert_eq!(*operator, TypeBinaryOperator::Cast);
                        // a
                        assert_expression_path!(parser, parser.tree.get(*left), "a");
                        // number
                        assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Number));
                    }
                );
                // b
                assert_expression_path!(parser, parser.tree.get(*right), "b");
            }
        );
    }

    /// Parse a TypeScript angle bracket type assertion.
    #[test]
    fn test_parse_typescript_angle_type_assertion_expression() {
        let mut test = TestParser::new_with_options("<any>value", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::TypeBinary { left, operator, right } => {
            assert_eq!(*operator, TypeBinaryOperator::Cast);
            assert_expression_path!(parser, parser.tree.get(*left), "value");
            assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Any));
        });
    }

    /// Type casts bind to the full addition expression on the left.
    #[test]
    fn test_parse_precedence_cast_after_addition() {
        let mut test = TestParser::new("a + b as number");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a + b as number
        assert_node!(
            parser.tree,
            expr_id,
            Expression::TypeBinary { left, operator, right } => {
                assert_eq!(*operator, TypeBinaryOperator::Cast);
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
                // number
                assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Number));
            }
        );
    }

    /// Type casts bind to the left side before multiplication.
    #[test]
    fn test_parse_precedence_cast_before_multiply() {
        let mut test = TestParser::new("a as boolean * b");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a as boolean * b
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::Multiply);
                // a as boolean
                assert_node!(
                    parser.tree,
                    *left,
                    Expression::TypeBinary { left, operator, right } => {
                        assert_eq!(*operator, TypeBinaryOperator::Cast);
                        // a
                        assert_expression_path!(parser, parser.tree.get(*left), "a");
                        // boolean
                        assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Boolean));
                    }
                );
                // b
                assert_expression_path!(parser, parser.tree.get(*right), "b");
            }
        );
    }

    /// Type casts bind to the full multiplication expression on the left.
    #[test]
    fn test_parse_precedence_cast_after_multiply() {
        let mut test = TestParser::new("a * b as boolean");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a * b as boolean
        assert_node!(
            parser.tree,
            expr_id,
            Expression::TypeBinary { left, operator, right } => {
                assert_eq!(*operator, TypeBinaryOperator::Cast);
                // a * b
                assert_node!(
                    parser.tree,
                    *left,
                    Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::Multiply);
                        // a
                        assert_expression_path!(parser, parser.tree.get(*left), "a");
                        // b
                        assert_expression_path!(parser, parser.tree.get(*right), "b");
                    }
                );
                // boolean
                assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Boolean));
            }
        );
    }

    /// Type casts bind tighter than comparisons.
    #[test]
    fn test_parse_precedence_cast_before_comparison() {
        let mut test = TestParser::new("a >= b as number");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a >= b as number
        assert_node!(
            parser.tree,
            expr_id,
            Expression::Binary { left, operator, right, .. } => {
                assert_eq!(*operator, BinaryOperator::GreaterThanOrEqual);
                // a
                assert_expression_path!(parser, parser.tree.get(*left), "a");
                // b as number
                assert_node!(
                    parser.tree,
                    *right,
                    Expression::TypeBinary { left, operator, right } => {
                        assert_eq!(*operator, TypeBinaryOperator::Cast);
                        // b
                        assert_expression_path!(parser, parser.tree.get(*left), "b");
                        // number
                        assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Number));
                    }
                );
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

    /// Unary operator spans point at the operator token.
    #[test]
    fn test_parse_unary_operator_span() {
        let mut test = TestParser::new("-value");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
            assert_eq!(*operator, UnaryOperator::Negate);
            assert_expression_path!(parser, parser.tree.get(*right), "value");
        });

        let main_span = parser
            .tree
            .get_main_span(expr_id)
            .expect("expected unary operator span");
        assert_eq!(parser.get_span_str(main_span), "-");
    }

    #[test]
    fn test_parse_unary_postfix_operator_span() {
        let mut test = TestParser::new("value++");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Unary { operator, right } => {
            assert_eq!(*operator, UnaryOperator::PostIncrement);
            assert_expression_path!(parser, parser.tree.get(*right), "value");
        });

        let main_span = parser
            .tree
            .get_main_span(expr_id)
            .expect("expected unary postfix operator span");
        assert_eq!(parser.get_span_str(main_span), "++");
    }

    #[test]
    fn test_parse_unary_keyword_operators() {
        let mut test = TestParser::new("typeof foo; void 0");
        let mut parser = test.prepare();

        let typeof_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, typeof_id, Expression::Unary { operator, right } => {
            assert_eq!(*operator, UnaryOperator::Typeof);
            assert_expression_path!(parser, parser.tree.get(*right), "foo");
        });
        parser.eat_statement_stop_with_newlines().unwrap();

        let void_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, void_id, Expression::Unary { operator, right } => {
            assert_eq!(*operator, UnaryOperator::Void);
            assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
        });
    }

    /// Binary operator spans point at the operator token.
    #[test]
    fn test_parse_binary_operator_span() {
        let mut test = TestParser::new("left + right");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Binary { operator, left, right } => {
            assert_eq!(*operator, BinaryOperator::Add);
            assert_expression_path!(parser, parser.tree.get(*left), "left");
            assert_expression_path!(parser, parser.tree.get(*right), "right");
        });

        let main_span = parser
            .tree
            .get_main_span(expr_id)
            .expect("expected binary operator span");
        assert_eq!(parser.get_span_str(main_span), "+");
    }

    #[test]
    fn test_parse_binary_operator_multichar_span() {
        let mut test = TestParser::new("left === right");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Binary { operator, left, right } => {
            assert_eq!(*operator, BinaryOperator::EqualStrict);
            assert_expression_path!(parser, parser.tree.get(*left), "left");
            assert_expression_path!(parser, parser.tree.get(*right), "right");
        });

        let main_span = parser
            .tree
            .get_main_span(expr_id)
            .expect("expected binary operator span");
        assert_eq!(parser.get_span_str(main_span), "===");
    }

    #[test]
    fn test_parse_binary_operator_logical_span() {
        let mut test = TestParser::new("left && right");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Binary { operator, left, right } => {
            assert_eq!(*operator, BinaryOperator::And);
            assert_expression_path!(parser, parser.tree.get(*left), "left");
            assert_expression_path!(parser, parser.tree.get(*right), "right");
        });

        let main_span = parser
            .tree
            .get_main_span(expr_id)
            .expect("expected binary operator span");
        assert_eq!(parser.get_span_str(main_span), "&&");
    }

    #[test]
    fn test_parse_binary_operator_coalesce_span() {
        let mut test = TestParser::new("left ?? right");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Binary { operator, left, right } => {
            assert_eq!(*operator, BinaryOperator::Coalesce);
            assert_expression_path!(parser, parser.tree.get(*left), "left");
            assert_expression_path!(parser, parser.tree.get(*right), "right");
        });

        let main_span = parser
            .tree
            .get_main_span(expr_id)
            .expect("expected binary operator span");
        assert_eq!(parser.get_span_str(main_span), "??");
    }

    #[test]
    fn test_parse_assign_operator_span() {
        let mut test = TestParser::new("left += right");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Assign { operator, left, right } => {
            assert_eq!(*operator, AssignOperator::AddAssign);
            assert_expression_path!(parser, parser.tree.get(*left), "left");
            assert_expression_path!(parser, parser.tree.get(*right), "right");
        });

        let main_span = parser
            .tree
            .get_main_span(expr_id)
            .expect("expected assign operator span");
        assert_eq!(parser.get_span_str(main_span), "+=");
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

    /// Parse type prefix operators and infer bindings.
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
                assert_node!(parser.tree, *right, Expression::TypeInfer { name, constraint } => {
                    // infer
                    assert_string!(parser, *name, "Value");
                    assert!(constraint.is_none());
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

    /// Parse type unary postfix as comptime operation.
    #[test]
    fn test_parse_type_unary_postfix_as_comptime_expression() {
        let mut test = TestParser::new("Value as comptime");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // Value as comptime
        assert_node!(parser.tree, expr_id, Expression::TypeUnary { operator, right } => {
            assert_eq!(*operator, TypeUnaryOperator::AsComptime);
            assert_expression_path!(parser, parser.tree.get(*right), "Value");
        });
    }

    /// Parse async identifiers with `as` casts.
    #[test]
    fn test_parse_async_as_cast() {
        let mut test = TestParser::new_with_options("async as any", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::TypeBinary { left, operator, right } => {
            assert_eq!(*operator, TypeBinaryOperator::Cast);
            assert_expression_path!(parser, parser.tree.get(*left), "async");
            assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::Any));
        });
    }

    /// Parse async arrows with a parameter named `as`.
    #[test]
    fn test_parse_async_arrow_with_as_parameter() {
        let mut test = TestParser::new_with_options("async as => {}", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                assert_eq!(signature.asynchrony, Asynchrony::Async);
                assert_eq!(signature.kind, FunctionKind::Lambda);
                assert_eq!(signature.dynamic_parameters.len(), 1);
                assert_node!(parser.tree, signature.dynamic_parameters[0], Parameter::Named { name, .. } => {
                    assert_string!(parser, *name, "as");
                });
                assert!(body.is_some());
            });
        });
    }

    /// Parse `type as string` as a cast expression.
    #[test]
    fn test_parse_type_keyword_as_cast_expression() {
        let mut test = TestParser::new_with_options("type as string", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::TypeBinary { left, operator, right } => {
            assert_eq!(*operator, TypeBinaryOperator::Cast);
            assert_expression_path!(parser, parser.tree.get(*left), "type");
            assert_node!(parser.tree, *right, Expression::TypeLiteral(TypeLiteral::String));
        });
    }

    /// Parse type aliases named `as` and `satisfies`.
    #[test]
    fn test_parse_type_alias_named_as_or_satisfies() {
        let mut test = TestParser::new_with_options(
            "type as = 0;\ntype satisfies = 0;",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 2);

        assert_node!(parser.tree, expressions[0], Expression::Statement(statement_id) => {
            assert_node!(parser.tree, *statement_id, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Type { descriptor, value, .. } => {
                    assert_string!(parser, descriptor.name.unwrap().string(), "as");
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
                });
            });
        });

        assert_node!(parser.tree, expressions[1], Expression::Statement(statement_id) => {
            assert_node!(parser.tree, *statement_id, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Type { descriptor, value, .. } => {
                    assert_string!(parser, descriptor.name.unwrap().string(), "satisfies");
                    assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
                });
            });
        });
    }

    /// Reject angle bracket assertions in disallow ambiguous mode.
    #[test]
    fn test_reject_type_assertion_when_disallow_ambiguous_tree_literal() {
        let mut test = TestParser::new_with_options("<T>x", LanguageType::TypeScript);
        let mut parser = test.prepare();
        parser.options.disallow_ambiguous_tree_literal = true;

        let result = parser.eat_expression();
        assert!(result.is_err());
    }

    /// Reject ambiguous generic arrows in disallow ambiguous mode.
    #[test]
    fn test_reject_generic_arrow_when_disallow_ambiguous_tree_literal() {
        let mut test = TestParser::new_with_options("<T>() => 1", LanguageType::TypeScript);
        let mut parser = test.prepare();
        parser.options.disallow_ambiguous_tree_literal = true;

        let result = parser.eat_expression();
        assert!(result.is_err());
    }

    /// Reject angle bracket assertions in `new` receivers.
    #[test]
    fn test_reject_type_assertion_in_new_receiver() {
        let mut test = TestParser::new_with_options("new <any>Test2();", LanguageType::TypeScript);
        let mut parser = test.prepare();

        let result = parser.eat_expression();
        assert!(result.is_err());
    }

    /// Parse `type instanceof Foo` as a binary expression.
    #[test]
    fn test_parse_type_keyword_instanceof_expression() {
        let mut test =
            TestParser::new_with_options("type instanceof Foo", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Binary { left, operator, right } => {
            assert_eq!(*operator, BinaryOperator::InstanceOf);
            assert_expression_path!(parser, parser.tree.get(*left), "type");
            assert_expression_path!(parser, parser.tree.get(*right), "Foo");
        });
    }

    /// Parse `extension` as an identifier in TypeScript expressions.
    #[test]
    fn test_parse_extension_identifier_in_typescript_ternary_expression() {
        let mut test = TestParser::new_with_options(
            r#"typeof extension === "function" ? extension(cloned) : extension"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::If { kind, condition, then_expression, else_expression } => {
            assert_eq!(*kind, IfKind::Ternary);
            assert_node!(condition, IfCondition::Expression { condition } => {
                assert_node!(parser.tree, *condition, Expression::Binary { left, operator, right } => {
                    assert_eq!(*operator, BinaryOperator::EqualStrict);
                    assert_node!(parser.tree, *left, Expression::Unary { operator, right } => {
                        assert_eq!(*operator, UnaryOperator::Typeof);
                        assert_expression_path!(parser, parser.tree.get(*right), "extension");
                    });
                    assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::String(string)) => {
                        assert_string!(parser, *string, "function");
                    });
                });
            });

            assert_node!(parser.tree, *then_expression, Expression::Call { left, dynamic_arguments, .. } => {
                assert_expression_path!(parser, parser.tree.get(*left), "extension");
                assert_eq!(dynamic_arguments.len(), 1);
                assert_node!(parser.tree, dynamic_arguments[0], Argument::Positional { value, .. } => {
                    assert_expression_path!(parser, parser.tree.get(*value), "cloned");
                });
            });

            let else_expression_id = else_expression.expect("expected ternary else expression");
            assert_expression_path!(parser, parser.tree.get(else_expression_id), "extension");
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
                assert_node!(parser.tree, signature.return_type.unwrap(), Expression::TypePredicate { asserts, subject, target } => {
                    // asserts value is string
                    assert!(*asserts);
                    assert_eq!(*subject, TypePredicateSubject::Identifier(parser.strings.intern("value")));
                    assert_node!(parser.tree, target.unwrap(), Expression::TypeLiteral(TypeLiteral::String));
                });

                let predicate_id = signature.return_type.unwrap();
                let main_span = parser
                    .tree
                    .get_main_span(predicate_id)
                    .expect("expected predicate main span");
                assert_eq!(parser.get_span_str(main_span), "value");
            });
        });
    }

    /// Parse labelled statements with a label span.
    #[test]
    fn test_parse_labelled_statement_span() {
        let mut test = TestParser::new("label: loop {}");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expr_id, Expression::Labelled { label, .. } => {
            assert_string!(parser, *label, "label");
        });

        let main_span = parser
            .tree
            .get_main_span(expr_id)
            .expect("expected label main span");
        assert_eq!(parser.get_span_str(main_span), "label");
    }

    /// Reject labelled lexical declarations in javascript.
    #[test]
    fn test_reject_labelled_lexical_declaration_javascript() {
        // source: a: let a
        let mut test = TestParser::new_with_options("a: let a", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let _ = parser.parse();
        let diagnostic = parser
            .diagnostics
            .iter()
            .into_iter()
            .find(|diagnostic| diagnostic.code.starts_with("EP"))
            .expect("expected parse diagnostic");

        // let a
        assert_eq!(parser.get_span_str(diagnostic.primary_span.span), "let a");
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

    /// Parse a leading elementwise operator in a type expression with doc comments.
    #[test]
    fn test_parse_elementwise_leading_type_expression_with_docs() {
        let mut test = TestParser::new(
            r###"
type Target =
  /**
   * bun
   */
  | "bun"
  /**
   * node
   */
  | "node"
  /**
   * browser
   */
  | "browser"
            "###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();
        let expr_id = parser.eat_expression().unwrap();
        // type Target = | "bun" | "node" | "browser"
        assert_node!(parser.tree, expr_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::Type { descriptor: DeclarationDescriptor { name, .. }, value, .. } => {
                assert_string!(parser, name.unwrap().string(), "Target");
                assert_node!(parser.tree, *value, Expression::Binary { left, operator, right, .. } => {
                    assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                    assert_node!(parser.tree, *left, Expression::Binary { left, operator, right, .. } => {
                        assert_eq!(*operator, BinaryOperator::ElementwiseOr);
                        assert_node!(parser.tree, *left, Expression::ScalarLiteral(ScalarLiteral::String(bun_id)) => {
                            assert_string!(parser, *bun_id, "bun");
                        });
                        assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::String(node_id)) => {
                            assert_string!(parser, *node_id, "node");
                        });
                    });
                    assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::String(browser_id)) => {
                        assert_string!(parser, *browser_id, "browser");
                    });
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

    #[test]
    fn test_parse_new_without_arguments_missing_semicolon() {
        let options = LanguageType::TypeScript;
        let mut test = TestParser::new_with_options("new A<T> if (0);", options);
        let mut parser = test.prepare();
        let err = parser.try_eat_statement_expression_with_flag().unwrap_err();
        assert_eq!(err.leaf_span().start, 9);
    }

    /// Comma in parentheses parses as sequence expression.
    #[test]
    fn test_parse_sequence_expression() {
        let options = LanguageType::JavaScript;
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

    /// Comma operator parses as sequence expression in JS/TS.
    #[test]
    fn test_parse_sequence_expression_without_parens() {
        let options = LanguageType::TypeScript;
        let mut test = TestParser::new_with_options("a, b", options);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a, b
        assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
            assert_eq!(expressions.len(), 2);
            // a
            assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");
            // b
            assert_expression_path!(parser, parser.tree.get(expressions[1]), "b");
        });
    }

    #[test]
    fn test_parse_sequence_expression_with_ternary_tail() {
        let options = LanguageType::TypeScript;
        let mut test = TestParser::new_with_options("a && (b = 1, c = 2), d ? e : f", options);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // a && (b = 1, c = 2), d ? e : f
        assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
            assert_eq!(expressions.len(), 2);
            assert_node!(parser.tree, expressions[0], Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::And);
            });
            assert_node!(parser.tree, expressions[1], Expression::If { kind, .. } => {
                assert_eq!(*kind, IfKind::Ternary);
            });
        });
    }

    #[test]
    fn test_parse_sequence_expression_with_nested_ternary() {
        let options = LanguageType::TypeScript;
        let mut test = TestParser::new_with_options(
            "l === -1 && (s = !1, l = t + 1), a === 46 ? r === -1 ? r = t : n !== 1 && (n = 1) : r !== -1 && (n = -1)",
            options,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // l === -1 && (s = !1, l = t + 1), a === 46 ? r === -1 ? r = t : n !== 1 && (n = 1) : r !== -1 && (n = -1)
        assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
            assert_eq!(expressions.len(), 2);
            assert_node!(parser.tree, expressions[0], Expression::Binary { operator, .. } => {
                assert_eq!(*operator, BinaryOperator::And);
            });
            assert_node!(parser.tree, expressions[1], Expression::If { kind, .. } => {
                assert_eq!(*kind, IfKind::Ternary);
            });
        });
    }

    #[test]
    fn test_parse_export_const_ternary_object_literal_arrow_value() {
        let options = LanguageType::TypeScript;
        let mut test = TestParser::new_with_options(
            r#"export const reproValue = true ? {} : {
    reproFunc: (_: any): any => { },
};"#,
            options,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::Let { descriptor, declarators, .. } => {
            assert_eq!(descriptor.export, Some(DependencyMode::Item));
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern, .. } => {
                    assert_string!(parser, *name, "reproValue");
                    assert!(pattern.is_none());
                });
                let value_id = value.expect("expected initializer");
                assert_node!(parser.tree, value_id, Expression::If { kind, then_expression, else_expression, .. } => {
                    assert_eq!(*kind, IfKind::Ternary);
                    assert_node!(parser.tree, *then_expression, Expression::ObjectExpression { properties, .. } => {
                        assert!(properties.is_empty());
                    });
                    let else_expression = else_expression.expect("expected else branch");
                    assert_node!(parser.tree, else_expression, Expression::ObjectExpression { properties, .. } => {
                        assert_eq!(properties.len(), 1);
                        assert_node!(parser.tree, properties[0], Property::Field { key, value, default, .. } => {
                            assert!(default.is_none());
                            assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                                assert_string!(parser, *name, "reproFunc");
                            });
                            let value_id = value.expect("expected property value");
                            assert_node!(parser.tree, value_id, Expression::Declaration(declaration_id) => {
                                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                                    assert_eq!(signature.kind, FunctionKind::Lambda);
                                    let body_id = body.expect("expected function body");
                                    assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                                        assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                                            assert!(expressions.is_empty());
                                        });
                                    });
                                });
                            });
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_export_const_type_identifier_with_struct_value() {
        let options = LanguageType::TypeScript;
        let mut test = TestParser::new_with_options("export const type = struct", options);
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Let { descriptor, declarators, .. } => {
            assert_eq!(descriptor.export, Some(DependencyMode::Item));
            assert_eq!(declarators.len(), 1);
            assert_node!(parser.tree, declarators[0], Declarator { pattern, value, .. } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                    assert_string!(parser, *name, "type");
                });
                let value_id = value.expect("expected initializer");
                assert_expression_path!(parser, parser.tree.get(value_id), "struct");
            });
        });
    }

    #[test]
    fn test_parse_ternary_object_literal_arrow_value_expression() {
        let options = LanguageType::TypeScript;
        let mut test = TestParser::new_with_options(
            r#"true ? {} : {
    reproFunc: (_: any): any => { },
}"#,
            options,
        );
        let mut parser = test.prepare();
        let result = parser.with_options(
            parser
                .options
                .not_in_position()
                .not_in_sequence_expression(),
            |parser| parser.eat_expression(),
        );
        match result {
            Ok(expr_id) => {
                assert_node!(parser.tree, expr_id, Expression::If { kind, then_expression, else_expression, .. } => {
                    assert_eq!(*kind, IfKind::Ternary);
                    assert_node!(parser.tree, *then_expression, Expression::ObjectExpression { properties, .. } => {
                        assert!(properties.is_empty());
                    });
                    let else_expression = else_expression.expect("expected else branch");
                    assert_node!(parser.tree, else_expression, Expression::ObjectExpression { properties, .. } => {
                        assert_eq!(properties.len(), 1);
                        assert_node!(parser.tree, properties[0], Property::Field { key, value, default, .. } => {
                            assert!(default.is_none());
                            assert_node!(key, Some(Key::Name(Name::Identifier(name))) => {
                                assert_string!(parser, *name, "reproFunc");
                            });
                            let value_id = value.expect("expected property value");
                            assert_node!(parser.tree, value_id, Expression::Declaration(declaration_id) => {
                                assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body, .. } => {
                                    assert_eq!(signature.kind, FunctionKind::Lambda);
                                    let body_id = body.expect("expected function body");
                                    assert_node!(parser.tree, body_id, Expression::Block(block_id) => {
                                        assert_node!(parser.tree, *block_id, Block { expressions, .. } => {
                                            assert!(expressions.is_empty());
                                        });
                                    });
                                });
                            });
                        });
                    });
                });
            }
            Err(err) => panic!("unexpected error: {err:?}"),
        }
    }

    #[test]
    fn test_parse_object_literal_with_typed_arrow_value() {
        let options = LanguageType::TypeScript;
        let mut test = TestParser::new_with_options(
            r#"{
    reproFunc: (_: any): any => { },
}"#,
            options,
        );
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::ObjectExpression { properties, .. } => {
            assert_eq!(properties.len(), 1);
            assert_node!(parser.tree, properties[0], Property::Field { key: Some(Key::Name(Name::Identifier(name))), value: Some(value), .. } => {
                assert_string!(parser, *name, "reproFunc");
                assert_node!(parser.tree, *value, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Function { signature, body: Some(_), .. } => {
                        assert_eq!(signature.kind, FunctionKind::Lambda);
                        assert_eq!(signature.dynamic_parameters.len(), 1);
                    });
                });
            });
        });
    }

    /// Comma in parentheses parses as tuple expression.
    #[test]
    fn test_parse_tuple_expression() {
        let options = LanguageType::Destack;
        let mut test = TestParser::new_with_options("(a, b, c)", options);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // (a, b, c)
        assert_node!(parser.tree, expr_id, Expression::TupleExpression { elements } => {
            assert_eq!(elements.len(), 3);
            // a
            assert_node!(parser.tree, elements[0], Argument::Positional { modifiers: _, value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "a");
            });
            // b
            assert_node!(parser.tree, elements[1], Argument::Positional { modifiers: _, value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "b");
            });
            // c
            assert_node!(parser.tree, elements[2], Argument::Positional { modifiers: _, value } => {
                assert_expression_path!(parser, parser.tree.get(*value), "c");
            });
        });
    }

    /// Sequence expression with unary void.
    #[test]
    fn test_parse_sequence_expression_with_unary_void() {
        let options = LanguageType::JavaScript;
        let mut test = TestParser::new_with_options("(a, void 0, 1)", options);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // (a, void 0, 1)
        assert_node!(parser.tree, expr_id, Expression::SequenceExpression { expressions } => {
            assert_eq!(expressions.len(), 3);
            // a
            assert_expression_path!(parser, parser.tree.get(expressions[0]), "a");
            // void 0
            assert_node!(parser.tree, expressions[1], Expression::Unary { operator, right } => {
                assert_eq!(*operator, UnaryOperator::Void);
                assert_node!(parser.tree, *right, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
            });
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

    #[test]
    fn test_parse_arrow_body_with_anonymous_class_expression() {
        let mut test = TestParser::new_with_options(
            r###"<P extends Props>(
  wrapped: ComponentType<P>
) => class extends Component<Omit<P, keyof A> & Partial<B>, C> {
  static displayName = `x`;
}"###,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // lambda with class expression body
        assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function { signature, body: Some(body), .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);

                // anonymous class extends generic component
                assert_node!(parser.tree, *body, Expression::Declaration(class_id) => {
                    assert_node!(parser.tree, *class_id, Declaration::Class { descriptor, heritage, members, .. } => {
                        assert!(descriptor.name.is_none());
                        assert_eq!(members.len(), 1);

                        let extends_types = heritage.extends_types.as_ref().expect("expected extends type");
                        assert_eq!(extends_types.len(), 1);

                        // Component<Omit<...>, C>
                        assert_node!(parser.tree, extends_types[0], Expression::Path { path, static_arguments: Some(static_arguments) } => {
                            assert_path!(parser, *path, "Component");
                            assert_eq!(static_arguments.len(), 2);
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_arrow_body_with_multiline_class_heritage_static_arguments() {
        let mut test = TestParser::new_with_options(
            r###"<P extends Props>(
  wrapped: React.ComponentType<P>
) => class extends React.Component<
  Omit<P, keyof Props> & Partial<Props>,
  Props
> {
  static displayName = `x`;
}"###,
            LanguageType::TypeScriptXml,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();

        // lambda with multiline class heritage static arguments
        assert_node!(parser.tree, expression_id, Expression::Declaration(function_id) => {
            assert_node!(parser.tree, *function_id, Declaration::Function { signature, body: Some(body), .. } => {
                assert_eq!(signature.kind, FunctionKind::Lambda);

                // class extends React.Component<...>
                assert_node!(parser.tree, *body, Expression::Declaration(class_id) => {
                    assert_node!(parser.tree, *class_id, Declaration::Class { heritage, .. } => {
                        let extends_types = heritage.extends_types.as_ref().expect("expected extends type");
                        assert_eq!(extends_types.len(), 1);

                        // React.Component<Omit<...>, Props>
                        assert_node!(parser.tree, extends_types[0], Expression::Path { path, static_arguments: Some(static_arguments) } => {
                            assert_path!(parser, *path, "React.Component");
                            assert_eq!(static_arguments.len(), 2);
                        });
                    });
                });
            });
        });
    }
}
