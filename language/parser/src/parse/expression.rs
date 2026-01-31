use std::str::FromStr;

use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Argument, AssignOperator, Asynchrony, BinaryOperator, BindingAnchor, Declaration,
    DeclarationAbstraction, DeclarationDescriptor, DeclarationKind, DependencyMode, EnumKind,
    Expression, IfCondition, IfKind, InfixOperator, Keyword, LiteralType, LocalNodeId, NodeType,
    PostfixPosition, TokenSpan, TokenType, TypeBinaryOperator, TypeKind, TypeUnaryOperator,
    UnaryOperator,
};
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
    // special case for shift right (`>>`) and unsigned shift right (`>>>`) to avoid ungluing ambiguity
    if !options.in_static
        && !options.in_tree_literal
        && !options.in_type
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

        // spread (...x) is not a valid standalone expression in JS/TS
        if operator == UnaryOperator::Spread && !self.language.is_destack() {
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
        to_infix_operator(
            token_str,
            token,
            next_token,
            next_next_token,
            self.options,
            self.language,
            false,
        )
    }

    /// Peek a next infix operator.
    #[inline]
    pub fn peek_next_infix_operator(&self) -> ParseResult<(InfixOperator, u8)> {
        let token = self.peek_next()?;
        let token_str = self.get_span_str(token.span);
        let next_token = self.peek_next_next()?;
        let next_next_token = self.peek_next_next_next()?;
        to_infix_operator(
            token_str,
            token,
            next_token,
            next_next_token,
            self.options,
            self.language,
            true,
        )
    }

    /// Peek an infix operator after any newlines.
    #[inline]
    pub fn peek_infix_operator_after_newlines(&self) -> ParseResult<(InfixOperator, u8)> {
        let mut pos = self.pos() as usize;
        while let Some(token) = self.tokens.get(pos + 1)
            && token.token.ty == TokenType::Newline
        {
            pos += 1;
        }
        let token = self
            .tokens
            .get(pos + 1)
            .ok_or(ParseError::unexpected(self.eof_token.span))?;
        let token_str = self.get_span_str(token.span);
        let next_token = self.tokens.get(pos + 2).unwrap_or(&self.eof_token);
        let next_next_token = self.tokens.get(pos + 3).unwrap_or(&self.eof_token);
        to_infix_operator(
            token_str,
            token,
            next_token,
            next_next_token,
            self.options,
            self.language,
            true,
        )
    }

    /// Check whether a parsed static argument list can be followed in expression position.
    #[inline]
    pub fn can_follow_type_arguments_in_expression(&self) -> bool {
        if self.peek_any_stop().is_ok() {
            return true;
        }
        if self
            .peek_token_in(&[
                TokenType::CloseParenthesis,
                TokenType::CloseBracket,
                TokenType::CloseBrace,
            ])
            .is_ok()
        {
            return true;
        }
        if self
            .peek_token_in(&[
                TokenType::OpenParenthesis,
                TokenType::OpenBracket,
                TokenType::Dot,
            ])
            .is_ok()
        {
            return true;
        }
        if self.peek_token(TokenType::Maybe).is_ok() {
            return true;
        }
        // allow heritage terminators after static arguments
        if self.options.in_super_type {
            if self.peek_token(TokenType::OpenBrace).is_ok() {
                return true;
            }
            if self.peek_keyword(Keyword::Implements).is_ok()
                || self.peek_keyword(Keyword::With).is_ok()
                || self.peek_keyword(Keyword::Where).is_ok()
            {
                return true;
            }
        }
        if self.peek_infix_operator().is_ok() || self.peek_assign_operator().is_ok() {
            return true;
        }
        false
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
                let start = ParserMark::new(span.start as usize, None, false);
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
        else {
            let base = self.pos() as usize;
            let mut offset = 0;
            let mut newline_count = 0;
            while let Some(token) = self.tokens.get(base + offset)
                && token.token.ty == TokenType::Newline
            {
                newline_count += 1;
                offset += 1;
            }
            let is_member = newline_count > 0
                && matches!(
                    self.tokens.get(base + offset),
                    Some(token) if token.token.ty == TokenType::Dot
                )
                && matches!(
                    self.tokens.get(base + offset + 1),
                    Some(token) if token.token.ty == token_type
                );
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
    fn is_optional_chain_after_maybe(&self) -> bool {
        // require ?. before we look at the target
        if self.peek_next_token(TokenType::Dot).is_err() {
            return false;
        }

        // accept valid optional chain targets after ?.
        let next_next_token_type = self
            .tokens
            .get(self.pos() as usize + 2)
            .map(|token| token.token.ty);
        matches!(
            next_next_token_type,
            Some(
                TokenType::Identifier
                    | TokenType::OpenBracket
                    | TokenType::OpenParenthesis
                    | TokenType::Hash
                    | TokenType::LessThan
                    | TokenType::ShiftLeft
                    | TokenType::TemplateStringStart
                    | TokenType::TemplateString
            )
        )
    }

    /// Peek a private member access using `.#`.
    #[inline]
    fn peek_private_member(&self) -> ParseResult<u8> {
        // immediate private member access
        if self.peek_token(TokenType::Dot).is_ok()
            && self.peek_next_token(TokenType::Hash).is_ok()
            && self.peek_next_next_token(TokenType::Identifier).is_ok()
        {
            Ok(3)
        }
        // private member access across newline
        else {
            let base = self.pos() as usize;
            let mut offset = 0;
            let mut newline_count = 0;
            while let Some(token) = self.tokens.get(base + offset)
                && token.token.ty == TokenType::Newline
            {
                newline_count += 1;
                offset += 1;
            }
            let is_member = newline_count > 0
                && matches!(
                    self.tokens.get(base + offset),
                    Some(token) if token.token.ty == TokenType::Dot
                )
                && matches!(
                    self.tokens.get(base + offset + 1),
                    Some(token) if token.token.ty == TokenType::Hash
                )
                && matches!(
                    self.tokens.get(base + offset + 2),
                    Some(token) if token.token.ty == TokenType::Identifier
                );
            if is_member {
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

    /// Check whether a `{` in statement position should be parsed as an object literal.
    /// NOTE #Cleanup: can_parse_object_literal_in_statement_position is ugly (might not be fixable..)
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
        let Some(next_token) = self.tokens.get(open_pos as usize + 1) else {
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
            let Some(after_close) = self.tokens.get(after_close_pos as usize + 1) else {
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
            let Some(after_key) = self.tokens.get(key_pos as usize + 1) else {
                return false;
            };

            return matches!(after_key.token.ty, TokenType::Colon | TokenType::Maybe);
        }

        false
    }

    /// Eat an expression.
    pub fn eat_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        destack_base::ensure_sufficient_stack(|| self.eat_expression_inner())
    }

    /// Eat an expression body without stack growth.
    fn eat_expression_inner(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark();

        // labelled statement or expression (like `label: while(...)` or `label: loop {}`)
        if self.peek_token(TokenType::Identifier).is_ok()
            && self.peek_next_token(TokenType::Colon).is_ok()
        {
            // label targets that are always expressions
            let is_labelled_expression = self.peek_next_next_keyword(Keyword::While).is_ok()
                || self.peek_next_next_keyword(Keyword::Do).is_ok()
                || self.peek_next_next_keyword(Keyword::For).is_ok()
                || self.peek_next_next_keyword(Keyword::Loop).is_ok()
                || self.peek_next_next_keyword(Keyword::If).is_ok()
                || self.peek_next_next_keyword(Keyword::Switch).is_ok()
                || self.peek_next_next_keyword(Keyword::Try).is_ok()
                || self.peek_next_next_keyword(Keyword::With).is_ok();

            // labelled blocks are only allowed in statement position
            let is_labelled_block = self.peek_next_next_token(TokenType::OpenBrace).is_ok();
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
                let body = self.eat_expression()?;
                let labelled_id = self.tree.insert(
                    Expression::Labelled { label, body },
                    self.get_span_from(start),
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

            // export namespace
            let is_export_namespace = self.peek_keyword(Keyword::As).is_ok()
                && self.peek_next_keyword(Keyword::Namespace).is_ok();
            if is_export_namespace {
                self.rewind(start);
                let export = self.eat_export()?;
                return Ok(export);
            }

            // just parse the export if followed by dependency items or module export
            let keyword = self.peek_any_keyword().ok();
            let is_not_declaration_keyword =
                keyword.is_none() || !DECLARATION_KEYWORDS.contains(&keyword.unwrap());
            let is_export_type_binding = self.peek_keyword(Keyword::Type).is_ok()
                && (self.peek_next_token(TokenType::OpenBrace).is_ok()
                    || self.peek_next_token(TokenType::Multiply).is_ok());
            if mode == Some(DependencyMode::Namespace)
                || is_export_type_binding
                || is_not_declaration_keyword && self.peek_dependency_binding().is_ok()
                || mode == Some(DependencyMode::Default) && is_not_declaration_keyword
            {
                self.rewind(start);
                let export = self.eat_export()?;
                return Ok(export);
            }

            descriptor.export = mode;
        }

        // kind (declare must not be followed by newline, similar to abstract)
        let is_declare_identifier = self.peek_next_token(TokenType::Identifier).is_ok()
            && self.peek_next().is_ok_and(|token| {
                let token_str = self.get_token_str(*token);
                token_str == "global"
                    || self.language.supports_module_declaration() && token_str == "module"
            });
        descriptor.kind = if self.peek_keyword(Keyword::Declare).is_ok()
            && self.peek_next_token(TokenType::Newline).is_err()
            && (self
                .peek_next_any_keyword()
                .is_ok_and(|kw| DECLARATION_KEYWORDS.contains(&kw))
                || is_declare_identifier)
        {
            self.bump(); // eat declare
            DeclarationKind::Declaration
        } else {
            DeclarationKind::Definition
        };

        // abstraction
        descriptor.abstraction = if self.peek_keyword(Keyword::Abstract).is_ok()
            && !self.options.in_variant
            && self.peek_next_token(TokenType::Newline).is_err()
            && self
                .peek_next_any_keyword()
                .is_ok_and(|kw| DECLARATION_KEYWORDS.contains(&kw))
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

        // global declaration
        if (descriptor.kind == DeclarationKind::Declaration || self.language.is_declaration())
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
            return Ok(self.tree.insert(
                Expression::Declaration(global_id),
                self.get_span_from(start),
            ));
        }

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

                // allow leading elementwise operators in type expressions
                let expression = self.tree.get(expression_id);
                if !matches!(
                    expression,
                    Expression::Binary { operator, .. }
                        if *operator == leading_binary_operator
                ) && !self.options.in_type
                {
                    return Err(ParseError::unexpected(self.get_span_from(start)));
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
                let lambda_id = self.eat_function(start, descriptor, false, false)?;
                self.tree.insert(
                    Expression::Declaration(lambda_id),
                    self.get_span_from(start),
                )
            }
            // parenthesis
            // may be tuple, lambda, or parenthesized expression
            else if token_type == TokenType::OpenParenthesis {
                // find the matching close and the following token
                let open_pos = self.pos();
                let closing_pos = self.find_matching_close(
                    None,
                    TokenType::OpenParenthesis,
                    TokenType::CloseParenthesis,
                )?;
                let closing_pos_for_follow = self.skip_newlines(closing_pos)?;

                // detect top level commas for Destack tuples
                let has_top_level_comma = self.language.is_destack()
                    && self.has_token_before_matching_close(
                        open_pos,
                        closing_pos,
                        TokenType::Comma,
                        self.options.in_type,
                    )?;

                // detect lambda when followed by arrow or colon
                let next_token_type = self
                    .tokens
                    .get(closing_pos_for_follow as usize + 1)
                    .map(|token| token.token.ty);
                let has_arrow = matches!(
                    next_token_type,
                    Some(TokenType::Arrow | TokenType::ArrowWide)
                );
                let has_colon = matches!(next_token_type, Some(TokenType::Colon));
                let is_colon_lambda_allowed = !self.options.in_before_type
                    && has_colon
                    && !(self.options.in_type
                        && self.options.in_ternary_condition
                        && has_top_level_comma);
                let mut lambda_expression_id = None;

                // parse lambda when we see a likely arrow or colon
                if has_arrow || is_colon_lambda_allowed {
                    // avoid colon lambdas that are actually ternary type tuples
                    if self.options.in_ternary_condition && has_colon {
                        let speculative_start = self.mark();
                        let speculative_start_idx = self.tree.next_id();
                        if let Ok(lambda_id) = self.eat_function(start, descriptor, false, false) {
                            let should_accept = match self.tree.get(lambda_id) {
                                Declaration::Function { body, .. } => {
                                    body.is_some() || self.options.in_type
                                }
                                _ => true,
                            };
                            if should_accept {
                                lambda_expression_id = Some(self.tree.insert(
                                    Expression::Declaration(lambda_id),
                                    self.get_span_from(start),
                                ));
                            } else {
                                self.restore(speculative_start, speculative_start_idx);
                            }
                        } else {
                            self.restore(speculative_start, speculative_start_idx);
                        }
                    } else {
                        let lambda_id = self.eat_function(start, descriptor, false, false)?;
                        lambda_expression_id = Some(self.tree.insert(
                            Expression::Declaration(lambda_id),
                            self.get_span_from(start),
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
                    if self.peek_token(TokenType::CloseParenthesis).is_ok() {
                        self.bump(); // eat closing parenthesis

                        // in Destack: empty tuple
                        if self.language.is_destack() {
                            self.tree.insert(
                                Expression::TupleExpression { elements: vec![] },
                                self.get_span_from(start),
                            )
                        }
                        // in JS or TS: empty sequence expression
                        else {
                            self.tree.insert(
                                Expression::SequenceExpression {
                                    expressions: vec![],
                                },
                                self.get_span_from(start),
                            )
                        }
                    }
                    // tuple when we see a named element or top level comma
                    else if (self.language.is_destack()
                        && self.peek_token(TokenType::Identifier).is_ok()
                        && self.peek_next_token(TokenType::Colon).is_ok())
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
                            self.get_span_from(start),
                        )
                    }
                    // tuple or parenthesized expression for the remaining cases
                    else {
                        let inner_start = self.pos();
                        let mut inner_options = self.options.nested().in_parenthesis();
                        if self.options.in_type {
                            inner_options = inner_options.in_type();
                        }
                        let expression_id =
                            self.with_options(inner_options, |parser| parser.eat_expression())?;
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

            // pointer types
            else if self.options.in_type && self.peek_token(TokenType::Multiply).is_ok() {
                self.bump(); // eat *
                let mutability = self.eat_mutability_maybe()?;
                let right = self.with_options(self.options.not_in_position(), |parser| {
                    parser.eat_expression()
                })?;
                let expression = Expression::PointerOf { mutability, right };
                self.tree.insert(expression, self.get_span_from(start))
            }
            // unary prefix operations
            else if let Ok(operator) = self.peek_unary_prefix_operator() {
                let operator_start = self.mark();
                self.bump(); // eat unary operator (always because right associative)
                let operator_span = self.get_span_from(operator_start);
                let mut right_options = self
                    .options
                    .not_in_position()
                    .in_left_precedence(operator.precedence());
                if self.options.in_type_conditional_right {
                    right_options = right_options.in_type_conditional_right();
                }
                let right = self.with_options(right_options, |parser| parser.eat_expression())?;
                let expression = Expression::Unary { operator, right };
                let expression_id = self.tree.insert(expression, self.get_span_from(start));
                self.tree.set_main_span(expression_id, operator_span);
                expression_id
            }
            // type unary operations
            else if let Ok(operator) = self.peek_type_unary_prefix_operator() {
                let operator_start = self.mark();
                self.bump(); // eat type unary operator (always because right associative)
                let operator_span = self.get_span_from(operator_start);
                let mut right_options = self
                    .options
                    .not_in_position()
                    .in_type()
                    .in_left_precedence(operator.precedence());
                if self.options.in_type_conditional_right {
                    right_options = right_options.in_type_conditional_right();
                }
                let right = self.with_options(right_options, |parser| parser.eat_expression())?;
                let expression = Expression::TypeUnary { operator, right };
                let expression_id = self.tree.insert(expression, self.get_span_from(start));
                self.tree.set_main_span(expression_id, operator_span);
                expression_id
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

            // namespace or module declaration
            else if (keyword == Some(Keyword::Namespace)
                || self.language.supports_module_declaration()
                    && self.peek_identifier_str("module").is_ok())
                && DECLARATION_START_TOKENS.contains(&next_token_type)
            {
                let namespace_id = self.eat_namespace(start, descriptor)?;
                self.tree.insert(
                    Expression::Declaration(namespace_id),
                    self.get_span_from(start),
                )
            }
            // struct / class
            else if (keyword == Some(Keyword::Struct) || keyword == Some(Keyword::Class))
                && DECLARATION_START_TOKENS.contains(&next_token_type)
            {
                let struct_id = self.eat_struct_or_class(start, descriptor)?;
                self.tree.insert(
                    Expression::Declaration(struct_id),
                    self.get_span_from(start),
                )
            }
            // enum
            else if keyword == Some(Keyword::Enum)
                && DECLARATION_START_TOKENS.contains(&next_token_type)
            {
                let enum_id = self.eat_enum(start, EnumKind::Enum, descriptor)?;
                self.tree
                    .insert(Expression::Declaration(enum_id), self.get_span_from(start))
            }
            // const enum
            else if keyword == Some(Keyword::Const)
                && self.peek_next_keyword(Keyword::Enum).is_ok()
            {
                self.eat_keyword(Keyword::Const)?;
                let enum_id = self.eat_enum(start, EnumKind::Const, descriptor)?;
                self.tree
                    .insert(Expression::Declaration(enum_id), self.get_span_from(start))
            }
            // newtype interface (nominal interface)
            else if keyword == Some(Keyword::Newtype)
                && self.peek_next_keyword(Keyword::Interface).is_ok()
            {
                self.eat_keyword(Keyword::Newtype)?;
                let interface_id = self.eat_interface(start, descriptor, TypeKind::Nominal)?;
                self.tree.insert(
                    Expression::Declaration(interface_id),
                    self.get_span_from(start),
                )
            }
            // interface (structural interface)
            else if keyword == Some(Keyword::Interface)
                && DECLARATION_START_TOKENS.contains(&next_token_type)
            {
                let interface_id = self.eat_interface(start, descriptor, TypeKind::Structural)?;
                self.tree.insert(
                    Expression::Declaration(interface_id),
                    self.get_span_from(start),
                )
            }
            // extension
            else if keyword == Some(Keyword::Extension)
                && DECLARATION_START_TOKENS.contains(&next_token_type)
            {
                let extension_id = self.eat_extension(start, descriptor)?;
                self.tree.insert(
                    Expression::Declaration(extension_id),
                    self.get_span_from(start),
                )
            }
            // async function or async identifier with static arguments
            else if keyword == Some(Keyword::Async)
                && [
                    TokenType::Identifier,
                    TokenType::OpenParenthesis,
                    TokenType::LessThan,
                    TokenType::At,
                    TokenType::Multiply, // function* generator
                ]
                .contains(&next_token_type)
            {
                // prefer parsing an async function when possible
                let speculative_start = self.mark();
                let speculative_start_idx = self.tree.next_id();
                if let Ok(function_id) = self.eat_function(start, descriptor, false, false) {
                    self.tree.insert(
                        Expression::Declaration(function_id),
                        self.get_span_from(start),
                    )
                } else {
                    self.restore(speculative_start, speculative_start_idx);

                    // parse async as an identifier path
                    let (path, last_span) = self
                        .eat_path_with_last_span()
                        .for_node_type(NodeType::Expression)?;

                    // eat optional static arguments with backtracking on failure
                    let static_arguments = if self.peek_token(TokenType::LessThan).is_ok()
                        || self.peek_token(TokenType::ShiftLeft).is_ok()
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

                    // parse a direct call when static arguments are present
                    if static_arguments.is_some()
                        && self.peek_token(TokenType::OpenParenthesis).is_ok()
                        && !self.options.in_new_receiver
                    {
                        let receiver = Expression::Path {
                            path,
                            static_arguments: None,
                        };
                        let receiver_id = self.tree.insert(receiver, self.get_span_from(start));
                        self.tree.set_main_span(receiver_id, last_span);
                        self.eat_call(receiver_id, static_arguments, PostfixPosition::Direct)?
                    } else {
                        let expression = Expression::Path {
                            path,
                            static_arguments,
                        };
                        let expression_id = self.tree.insert(expression, self.get_span_from(start));
                        self.tree.set_main_span(expression_id, last_span);
                        expression_id
                    }
                }
            }
            // function
            else if (keyword == Some(Keyword::Function)
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
                let function_id = self.eat_function(start, descriptor, false, false)?;
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
            // this
            else if keyword == Some(Keyword::This) {
                self.bump(); // eat this
                self.tree
                    .insert(Expression::This, self.get_span_from(start))
            }
            // new
            else if keyword == Some(Keyword::New)
                && !self.options.in_type
                && [
                    TokenType::Identifier,
                    TokenType::OpenParenthesis,
                    TokenType::OpenBrace,
                    TokenType::LessThan,
                ]
                .contains(&next_token_type)
            {
                self.eat_new()?
            }
            // delete
            else if keyword == Some(Keyword::Delete) && next_token_type == TokenType::Identifier {
                self.eat_delete()?
            }
            // type import expression
            else if self.options.in_type
                && keyword == Some(Keyword::Import)
                && next_token_type == TokenType::OpenParenthesis
            {
                self.eat_type_import_expression()?
            }
            // import call
            else if keyword == Some(Keyword::Import)
                && next_token_type == TokenType::OpenParenthesis
            {
                self.eat_import_call_expression(start)?
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
                // export import equals binding
                if descriptor.export.is_some() && self.peek_import_equals_after_import() {
                    self.eat_export_import_equals(start, descriptor)?
                } else {
                    self.eat_import()?
                }
            }
            // type infer
            else if self.options.in_type && keyword == Some(Keyword::Infer) {
                self.eat_type_infer_expression()?
            }
            // type predicate
            else if self.options.in_type && keyword == Some(Keyword::Asserts) {
                self.eat_type_predicate_asserts()?
            }
            // let
            else if keyword == Some(Keyword::Let)
                || keyword == Some(Keyword::Var)
                || keyword == Some(Keyword::Const)
            {
                self.eat_let(start, descriptor)?
            }
            // using
            else if keyword == Some(Keyword::Using) {
                self.eat_using(start, descriptor, Asynchrony::Sync)?
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
                self.eat_type(start, descriptor)?
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
            // await using
            else if keyword == Some(Keyword::Await)
                && self.peek_next_keyword(Keyword::Using).is_ok()
            {
                self.eat_using(start, descriptor, Asynchrony::Async)?
            }
            // await
            else if keyword == Some(Keyword::Await) {
                self.eat_await()?
            }
            // comptime
            else if keyword == Some(Keyword::Comptime) && self.language.is_destack() {
                self.eat_comptime()?
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
            // debugger
            else if keyword == Some(Keyword::Debugger) {
                self.bump(); // eat `debugger`
                self.tree
                    .insert(Expression::Debugger, self.get_span_from(start))
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
                        let properties = self
                            .with_options(self.options.not_in_position().in_type(), |parser| {
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
                        self.get_span_from(start),
                    )
                }
            }
            // block
            else if self.peek_block().is_ok() {
                let block_id = self.eat_block()?;
                self.tree
                    .insert(Expression::Block(block_id), self.get_span_from(start))
            }
            // statically parameterized lambda: <T>(...) or <T,>(...) #Cleanup
            // (also handles multiline in type context: `<\nT\n>(...) => ...`)
            else if token_type == TokenType::LessThan
                && {
                    let cond1 = self.peek_next_token(TokenType::Identifier).is_ok();
                    let cond2 = self.options.in_type
                        && self.peek_next_token(TokenType::Newline).is_ok()
                        && self
                            .peek_token_after_newlines(self.pos(), TokenType::Identifier)
                            .is_ok();
                    cond1 || cond2
                }
                && (self.peek_next_next_token(TokenType::Comma).is_ok()
                    || self
                        .find_matching_close(None, TokenType::LessThan, TokenType::GreaterThan)
                        .ok()
                        .and_then(|gt_pos| {
                            // must be `<T>(...` pattern, skip newlines in type context
                            let after_gt_pos = if self.options.in_type {
                                self.skip_newlines(gt_pos).ok()?
                            } else {
                                gt_pos
                            };
                            let after_gt = self.tokens.get(after_gt_pos as usize + 1)?;
                            if after_gt.token.ty != TokenType::OpenParenthesis {
                                return None;
                            }
                            // find `)` and check for `:` or `=>` after (skip newlines in type context)
                            let parenthesis_close = self
                                .find_matching_close(
                                    Some(after_gt_pos + 1),
                                    TokenType::OpenParenthesis,
                                    TokenType::CloseParenthesis,
                                )
                                .ok()?;
                            let after_close_pos = if self.options.in_type {
                                self.skip_newlines(parenthesis_close).ok()?
                            } else {
                                parenthesis_close
                            };
                            let after_parenthesis_close =
                                self.tokens.get(after_close_pos as usize + 1)?;
                            (after_parenthesis_close.token.ty == TokenType::Colon
                                || after_parenthesis_close.token.ty == TokenType::Arrow
                                || after_parenthesis_close.token.ty == TokenType::ArrowWide)
                                .then_some(true)
                        })
                        .is_some())
            {
                let function_id = self.eat_function(start, descriptor, false, false)?;
                self.tree.insert(
                    Expression::Declaration(function_id),
                    self.get_span_from(start),
                )
            }
            // typescript type assertion: <T>expr
            else if token_type == TokenType::LessThan
                && self.language.is_typescript()
                && !self.language.supports_jsx()
                && !self.options.in_type
            {
                self.eat_type_assertion(start)?
            }
            // tree literal
            else if self.language.supports_jsx()
                && token_type == TokenType::LessThan
                && !self.options.in_type
                && self.peek_tree_literal().is_ok()
            {
                self.with_options(self.options.not_in_position(), |parser| {
                    parser.eat_tree_literal()
                })?
            }
            // template literal
            else if self.peek_template_literal().is_ok() {
                if self.options.in_type {
                    self.eat_type_template_literal_expression()?
                } else {
                    let template_literal = self.eat_template_literal()?;
                    self.tree.insert(
                        Expression::TemplateExpression {
                            value: template_literal,
                        },
                        self.get_span_from(start),
                    )
                }
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
            // private identifier
            else if token_type == TokenType::Hash
                && self.peek_next_token(TokenType::Identifier).is_ok()
            {
                self.bump(); // eat #
                let (name, name_span) = self.eat_identifier_with_span()?;
                let expression_id = self.tree.insert(
                    Expression::PrivateIdentifier { name },
                    self.get_span_from(start),
                );
                self.tree.set_main_span(expression_id, name_span);
                expression_id
            }
            // alias / path / statically parameterized call
            else if token_type == TokenType::Identifier {
                let (path, last_span) = self
                    .eat_path_with_last_span()
                    .for_node_type(NodeType::Expression)?;

                // speculatively unwrap postfix static parameterisation with `<` or `<<`
                //  (might also be just a comparison operator)
                //  `<<` (ShiftLeft) handles cases like `Extends<<T>() => ...>`
                let static_arguments = if self.peek_token(TokenType::LessThan).is_ok()
                    || self.peek_token(TokenType::ShiftLeft).is_ok()
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
                if static_arguments.is_some()
                    && self.peek_token(TokenType::OpenParenthesis).is_ok()
                    && !self.options.in_new_receiver
                {
                    let receiver = Expression::Path {
                        path,
                        static_arguments: None,
                    };
                    let receiver_id = self.tree.insert(receiver, self.get_span_from(start));
                    self.tree.set_main_span(receiver_id, last_span);
                    self.eat_call(receiver_id, static_arguments, PostfixPosition::Direct)?
                } else {
                    let expression = Expression::Path {
                        path,
                        static_arguments,
                    };
                    let expression_id = self.tree.insert(expression, self.get_span_from(start));
                    self.tree.set_main_span(expression_id, last_span);
                    expression_id
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
            // stop before ternary boundary so postfix parsing does not consume ':'
            if self.options.in_ternary_condition
                && (self.peek_token(TokenType::Colon).is_ok()
                    || self.peek_newline().is_ok()
                        && self.peek_next_token(TokenType::Colon).is_ok())
            {
                break;
            }
            // stop before static boundary so postfix parsing does not consume '>'
            if self.options.in_static
                && (self.peek_token(TokenType::GreaterThan).is_ok()
                    || self.peek_newline().is_ok()
                        && self.peek_next_token(TokenType::GreaterThan).is_ok())
            {
                break;
            }

            let has_direct_call = self.peek_token(TokenType::OpenParenthesis).is_ok();
            let has_direct_call_after_newlines = self.peek_newline().is_ok()
                && self
                    .peek_token_after_newlines(self.pos(), TokenType::OpenParenthesis)
                    .is_ok();
            let has_indirect_call = self.peek_token(TokenType::Dot).is_ok()
                && self.peek_next_token(TokenType::OpenParenthesis).is_ok();
            let can_direct_call = (has_direct_call || has_direct_call_after_newlines)
                && !matches!(self.tree.get(left_expression_id), Expression::Maybe { .. })
                && !self.options.in_new_receiver;
            let should_parse_call = can_direct_call || has_indirect_call;
            // unary postfix operations
            if let Ok(operator) = self.peek_unary_postfix_operator() {
                let operator_start = self.mark();
                self.bump(); // eat unary operator
                let operator_span = self.get_span_from(operator_start);
                left_expression_id = self.tree.insert(
                    Expression::Unary {
                        operator,
                        right: left_expression_id,
                    },
                    self.get_span_from(start),
                );
                self.tree.set_main_span(left_expression_id, operator_span);
            }
            // type unary postfix operations
            else if let Ok(operator) = self.peek_type_unary_postfix_operator() {
                // avoid consuming conditional type ? as a type maybe
                let operator_start = self.mark();
                self.bump(); // eat type unary operator
                if operator == TypeUnaryOperator::AsConst {
                    self.bump(); // eat second token
                }
                let operator_span = self.get_span_from(operator_start);
                left_expression_id = self.tree.insert(
                    Expression::TypeUnary {
                        operator,
                        right: left_expression_id,
                    },
                    self.get_span_from(start),
                );
                self.tree.set_main_span(left_expression_id, operator_span);
            }
            // range (`..`, `..=`)
            else if self.peek_token(TokenType::Range).is_ok() {
                self.bump(); // eat ..
                let is_inclusive = if self.peek_token(TokenType::Assign).is_ok() {
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
            // private member (also works across newline)
            else if let Ok(distance) = self.peek_private_member() {
                self.bump_by(distance - 1); // keep the identifier
                let (name, name_span) = self.eat_identifier_with_span()?;
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
                    Expression::PrivateMember {
                        left: left_expression_id,
                        name,
                        static_arguments,
                    },
                    self.get_span_from(start),
                );
                self.tree.set_main_span(left_expression_id, name_span);
            }
            // member (also works across newline)
            else if let Ok(distance) = self.peek_member(TokenType::Identifier) {
                self.bump_by(distance - 1); // keep the identifier
                let (name, name_span) = self.eat_identifier_with_span()?;
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
                self.tree.set_main_span(left_expression_id, name_span);
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
            else if should_parse_call {
                if self.peek_newline().is_ok()
                    && self
                        .peek_token_after_newlines(self.pos(), TokenType::OpenParenthesis)
                        .is_ok()
                {
                    self.eat_newlines_maybe()?; // eat newlines
                }
                let position = if self.peek_token(TokenType::Dot).is_ok() {
                    self.bump(); // eat .
                    PostfixPosition::Indirect
                } else {
                    PostfixPosition::Direct
                };
                left_expression_id = self.eat_call(left_expression_id, None, position)?;
            }
            // statically parameterized call or instantiation expression (like `(expr)<T>()` or `(expr)<T>`)
            else if (self.peek_token(TokenType::LessThan).is_ok()
                || self.peek_token(TokenType::ShiftLeft).is_ok()
                || self.peek_token(TokenType::Dot).is_ok()
                    && (self.peek_next_token(TokenType::LessThan).is_ok()
                        || self.peek_next_token(TokenType::ShiftLeft).is_ok()))
                && !self.options.in_new_receiver
                && !self.options.in_tree_literal
                && !self.language.is_javascript()
            {
                // decide whether this is a direct or optional chain static argument list
                let has_indirect_static = self.peek_token(TokenType::Dot).is_ok()
                    && (self.peek_next_token(TokenType::LessThan).is_ok()
                        || self.peek_next_token(TokenType::ShiftLeft).is_ok());
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
                    if self.peek_token(TokenType::OpenParenthesis).is_ok() {
                        left_expression_id =
                            self.eat_call(left_expression_id, Some(static_arguments), position)?;
                    }
                    // instantiation expression
                    else if self.can_follow_type_arguments_in_expression() {
                        left_expression_id = self.tree.insert(
                            Expression::Instantiation {
                                left: left_expression_id,
                                static_arguments,
                            },
                            self.get_span_from(start),
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
            else if self.peek_token(TokenType::Maybe).is_ok()
                || self.peek_token(TokenType::Dot).is_ok()
                    && self.peek_next_token(TokenType::Maybe).is_ok()
                || self.peek_newline().is_ok() && self.peek_next_token(TokenType::Maybe).is_ok()
                || self.peek_newline().is_ok()
                    && self.peek_next_token(TokenType::Dot).is_ok()
                    && self.peek_next_next_token(TokenType::Maybe).is_ok()
            {
                let type_conditional_operands = if self.options.in_type {
                    self.split_type_conditional_operands(left_expression_id)
                } else {
                    None
                };
                let is_type_conditional = type_conditional_operands.is_some();
                self.eat_newlines_maybe()?; // eat newlines
                // maybe or maybe dot (followed by a delimiter/stop, but not preceded by a newline)
                let is_postfix_maybe = !self.options.in_type
                    && self.peek_token(TokenType::Maybe).is_ok()
                    && (self.peek_next_any_stop().is_ok()
                        && self.language.is_destack()
                        && self.prev_token_type() != TokenType::Newline
                        || self.peek_next_any_close_parenthesis().is_ok()
                        || self.is_optional_chain_after_maybe()
                        || self.peek_next_assign_operator().is_ok());
                if is_postfix_maybe {
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
                else if !self.options.in_type
                    && self.peek_token(TokenType::Dot).is_ok()
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
                            modifiers: None,
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
        let left_is_statement = self.options.in_statement_position
            && self
                .tree
                .get(left_expression_id)
                .ends_statement_on_newline();
        while self.peek().is_ok() {
            // statement expressions do not continue across newlines
            if left_is_statement && self.peek_token(TokenType::Newline).is_ok() {
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
                else if self.peek_token(TokenType::Newline).is_ok()
                    && let Ok((operator, operator_offset)) = self.peek_next_infix_operator()
                    && (self.options.left_precedence.is_none()
                        || self.options.left_precedence.unwrap() < operator.precedence())
                {
                    (operator, operator_offset)
                }
                // infix operator after multiple newlines in type expressions
                else if self.peek_token(TokenType::Newline).is_ok()
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
            if self.peek_token(TokenType::Newline).is_ok() {
                self.eat_newlines_maybe()?; // eat newlines
            }

            // capture operator span before eating
            let operator_start = self.mark();
            self.bump_by(operator_offset); // eat infix operator
            let operator_span = self.get_span_from(operator_start);

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
            let left_expression =
                self.make_infix_expression(left_expression_id, right_operator, right_expression_id);
            left_expression_id = self.tree.insert(left_expression, self.get_span_from(start));

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

        // value ternary after infix to keep lowest precedence
        // NOTE #Cleanup: having multiple ternary parse locations feels icky (but non-trivial to "fix")
        if !self.options.in_type
            && self.options.left_precedence.is_none()
            && (self.peek_token(TokenType::Maybe).is_ok()
                || self.peek_newline().is_ok() && self.peek_next_token(TokenType::Maybe).is_ok())
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
            left_expression_id = self.tree.insert(expression, self.get_span_from(start));
        }

        // sequence expression (comma operator) in JS/TS
        if !self.options.in_type
            && self.options.left_precedence.is_none()
            && self.options.allow_sequence_expression
            && self.language.is_typescript()
            && (self.peek_token(TokenType::Comma).is_ok()
                || self.peek_newline().is_ok() && self.peek_next_token(TokenType::Comma).is_ok())
        {
            let mut expressions = vec![left_expression_id];
            loop {
                self.eat_newlines_maybe()?;
                if self.peek_token(TokenType::Comma).is_err() {
                    break;
                }
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;
                let expression_id = self.with_options(
                    self.options.not_in_position().not_in_sequence_expression(),
                    |parser| parser.eat_expression(),
                )?;
                expressions.push(expression_id);
                self.eat_newlines_maybe()?;
            }

            let expression = Expression::SequenceExpression { expressions };
            left_expression_id = self.tree.insert(expression, self.get_span_from(start));
        }

        // type conditional expression
        if self.options.in_type
            && (self.peek_token(TokenType::Maybe).is_ok()
                || self.peek_newline().is_ok() && self.peek_next_token(TokenType::Maybe).is_ok())
        {
            // avoid consuming nested conditional tokens in the right side
            let conditional_operands = self.split_type_conditional_operands(left_expression_id);
            if self.options.in_type_conditional_right && conditional_operands.is_none() {
                return Ok(left_expression_id);
            }
            let optional_tuple_pos = if self.peek_token(TokenType::Maybe).is_ok() {
                Some(self.pos())
            } else if self.peek_newline().is_ok() && self.peek_next_token(TokenType::Maybe).is_ok()
            {
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
            left_expression_id = self.tree.insert(expression, self.get_span_from(start));
        }

        Ok(left_expression_id)
    }

    /// Eat a TypeScript type assertion expression (`<T>expr`).
    fn eat_type_assertion(&mut self, start: ParserMark) -> ParseResult<LocalNodeId<Expression>> {
        let static_arguments = self.with_options(self.options.in_type(), |parser| {
            parser.eat_static_arguments()
        })?;

        let type_expression = if static_arguments.len() == 1 {
            let argument_id = static_arguments[0];
            match self.tree.get(argument_id) {
                Argument::Positional { value, .. } => *value,
                _ => {
                    return Err(ParseError::expected(
                        self.get_span_from(start),
                        TokenType::Identifier,
                    ));
                }
            }
        } else {
            return Err(ParseError::expected(
                self.get_span_from(start),
                TokenType::Identifier,
            ));
        };

        let value = self.with_options(self.options.not_in_position(), |parser| {
            parser.eat_expression()
        })?;

        let expression = Expression::TypeBinary {
            left: value,
            operator: TypeBinaryOperator::Cast,
            right: type_expression,
        };
        Ok(self.tree.insert(expression, self.get_span_from(start)))
    }
}
#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, AssignOperator, BinaryOperator, Block, Declaration, DeclarationDescriptor,
        Declarator, DependencyItem, DependencyKind, DependencyMode, EnumField, EnumKind,
        Expression, FunctionKind, IfCondition, ImportAliasTarget, ImportSource, IntType, Key,
        Mutability, Name, Parameter, Pattern, PatternField, PostfixPosition, Property,
        ScalarLiteral, TypeBinaryOperator, TypeLiteral, TypePredicateSubject, TypeUnaryOperator,
        UnaryOperator, VarianceBound,
    };
    use destack_source::LanguageType;

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

    /// Parse a bare this expression.
    #[test]
    fn test_parse_this_expression() {
        let mut test = TestParser::new("this");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expression_id, Expression::This);
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
        let mut test = TestParser::new("if x extends Foo {\n    body\n}");
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
        let mut test = TestParser::new("if (T instanceof Foo) {\n    value\n}");
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
        let mut test = TestParser::new("{ [key]() {} }");
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
        let mut test = TestParser::new("true\n\t? 1\n\t: 2");
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
                assert_node!(parser.tree, *expression, Expression::Path { path, static_arguments } => {
                    assert_path!(parser, *path, "f");
                    assert!(static_arguments.as_ref().is_some_and(|args| args.len() == 1));
                });
            });
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

    /// Parse a TypeScript type assertion expression.
    #[test]
    fn test_parse_type_assertion() {
        let mut test = TestParser::new_with_options("<Foo>bar", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        assert_node!(parser.tree, expr_id, Expression::TypeBinary { left, operator, right } => {
            assert_eq!(*operator, TypeBinaryOperator::Cast);
            assert_expression_path!(parser, parser.tree.get(*left), "bar");
            assert_expression_path!(parser, parser.tree.get(*right), "Foo");
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
        let mut test = TestParser::new("&mut self.foo()");
        let mut parser = test.prepare();
        let expr_id = parser.eat_expression().unwrap();
        // &mut self.foo()
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

    /// Sequence expression with type literal elements.
    #[test]
    fn test_parse_sequence_expression_with_type_literal() {
        let options = LanguageType::JavaScript;
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
