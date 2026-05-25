use crate::parse::DeclarationHeader;
use crate::parse::scope::TypeScope;
use crate::parse::r#type::operator::TypeUnaryOperator;
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};
use destack_dir::{
    BinaryOperator, LocalNodeId, NodeType, OperatorPrecedence, ScalarLiteral, TokenType,
    TypeExpression, UnaryOperator,
};
use destack_source::{NodeSpanBoundary, NodeSpanType, Span};

impl Parser {
    /// Eat type prefix operators or one primary type.
    ///
    /// Examples:
    /// ```ds
    /// keyof T
    /// readonly string[]
    /// (value: string) => number
    /// ```
    pub(super) fn eat_type_prefix_or_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        match self.peek_token_type() {
            TokenType::Identifier => self.eat_identifier_type_primary(start),
            TokenType::Not => self.eat_type_prefix(start, TypeUnaryOperator::Not),
            TokenType::OpenParenthesis if self.can_start_parenthesized_function_type() => {
                self.eat_function_type_expression(start, DeclarationHeader::default())
            }
            TokenType::OpenParenthesis => self.eat_parenthesized_type(start),
            TokenType::LessThan if self.peek_generic_arrow_after_type_parameters(false) => {
                self.eat_function_type_expression(start, DeclarationHeader::default())
            }
            TokenType::OpenBracket => self.eat_bracket_type(start),
            TokenType::OpenBrace => self.eat_type_object_primary(start),
            TokenType::ElementwiseOr => {
                self.eat_type_leading_binary_list(start, BinaryOperator::ElementwiseOr)
            }
            TokenType::ElementwiseAnd if !self.language.is_destack() => {
                self.eat_type_leading_binary_list(start, BinaryOperator::ElementwiseAnd)
            }
            TokenType::TemplateString | TokenType::TemplateStringStart
                if self.is_template_literal_start() =>
            {
                self.eat_type_template_literal_expression()
            }
            TokenType::Add | TokenType::Subtract => self.eat_type_signed_scalar_primary(start),
            TokenType::Literal if self.is_scalar_literal_start() => {
                let value = self.eat_scalar_literal()?;

                Ok(self.insert_node(
                    TypeExpression::ScalarLiteral { value },
                    self.get_span_from(start),
                ))
            }
            TokenType::Range | TokenType::RangeInclusive if self.language.is_destack() => {
                self.eat_type_startless_range(start)
            }
            TokenType::ElementwiseAnd | TokenType::ElementwiseXor if self.language.is_destack() => {
                let token_type = self.peek_token_type();
                self.eat_type_reference_operator(start, token_type)
            }
            TokenType::Multiply => self.eat_type_pointer_prefix(start),
            _ => Err(ParserError::unexpected(self.peek()?.span)),
        }
    }

    /// Eat an identifier or keyword primary in type space.
    ///
    /// Examples:
    /// ```ds
    /// User
    /// keyof T
    /// type Alias = string
    /// ```
    fn eat_identifier_type_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        if self.can_start_construct_type_expression() {
            return self.eat_function_type_expression(start, DeclarationHeader::default());
        }

        if let Some(operator) = self.peek_type_unary_prefix_operator_maybe() {
            return self.eat_type_prefix(start, operator);
        }

        if let Some(keyword) = self.current_keyword()
            && let Some(type_expression_id) = self.eat_type_keyword_expression(start, keyword)?
        {
            return Ok(type_expression_id);
        }

        self.eat_type_reference_primary(start)
    }

    /// Eat a literal signed by the current token when present.
    ///
    /// Examples:
    /// ```ds
    /// -1
    /// +1
    /// -3.14
    /// ```
    fn eat_type_signed_scalar_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let Some(operator) = self.peek_unary_prefix_operator_maybe() else {
            return Err(ParserError::unexpected(self.peek()?.span));
        };
        if !matches!(operator, UnaryOperator::Plus | UnaryOperator::Negate) {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        self.bump();
        let value = self.eat_scalar_literal()?;
        let value = match (operator, value) {
            (UnaryOperator::Negate, ScalarLiteral::Integer(number)) => {
                ScalarLiteral::Integer(-number)
            }
            (UnaryOperator::Negate, ScalarLiteral::Bigint(number)) => {
                ScalarLiteral::Bigint(-number)
            }
            (UnaryOperator::Negate, ScalarLiteral::Float(number)) => ScalarLiteral::Float(-number),
            (_, value) => value,
        };

        Ok(self.insert_node(
            TypeExpression::ScalarLiteral { value },
            self.get_span_from(start),
        ))
    }

    /// Parse an object primary in type space.
    ///
    /// Examples:
    /// ```ds
    /// { id: string }
    /// { readonly [K in keyof T]?: T[K] }
    /// { call(value: string): number }
    /// ```
    fn eat_type_object_primary(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        if self.can_start_type_mapped_expression() {
            return self.eat_type_mapped_expression();
        }

        let flags = self.flags.not_in_position();
        let members = self.with_flags(flags, |parser| parser.eat_type_object_literal())?;

        Ok(self.insert_node(
            TypeExpression::Object { members },
            self.get_span_from(start),
        ))
    }

    /// Parse a pointer type prefix.
    ///
    /// Examples:
    /// ```ds
    /// *T
    /// *mut T
    /// *shared Node
    /// ```
    fn eat_type_pointer_prefix(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let operator_start = self.span_start();
        self.bump();
        let operator_span = self.get_span_from(&operator_start);

        let mutability = self.eat_reference_mutability_maybe()?;
        let target_type = self.eat_type_prefix_operand(OperatorPrecedence::Prefix as u16)?;

        let id = self.insert_node(
            TypeExpression::PointerOf {
                mutability,
                target_type,
            },
            self.get_span_from(start),
        );
        self.tree.set_main_span(id, operator_span);

        Ok(id)
    }

    /// Parse a type list with a leading separator.
    ///
    /// Examples:
    /// ```ds
    /// | A
    /// | A | B
    /// & A & B
    /// ```
    fn eat_type_leading_binary_list(
        &mut self,
        start: &ParserSpanStart,
        operator: BinaryOperator,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let mut elements = Vec::new();
        let minimum_precedence = operator.precedence();
        let operator_span = start.token_span();

        // elements
        while matches!(
            (self.peek_token_type(), operator),
            (TokenType::ElementwiseOr, BinaryOperator::ElementwiseOr)
                | (TokenType::ElementwiseAnd, BinaryOperator::ElementwiseAnd)
        ) {
            let element = self.eat_type_leading_binary_element(minimum_precedence)?;
            elements.push(element);
        }

        // node
        let first_element = elements.first().copied();
        let source_span = if let (Some(first), Some(last)) = (elements.first(), elements.last()) {
            let first_span = self.tree.get_span(*first);
            let last_span = self.tree.get_span(*last);

            Span::new(first_span.file, first_span.start, last_span.end)
        } else {
            self.get_span_from(start)
        };
        let expression = if operator == BinaryOperator::ElementwiseOr {
            TypeExpression::Union { elements }
        } else {
            TypeExpression::Intersection { elements }
        };
        let id = self.insert_node(expression, source_span);

        // spans
        if let Some(first) = first_element {
            let head_span = self.type_expression_head_span(first);
            self.tree.set_head_span(id, head_span);
            self.tree.set_side_span(
                id,
                NodeSpanType::Boundary(NodeSpanBoundary::Leading),
                Span::new(source_span.file, start.token_start(), source_span.start),
            );
            self.tree.set_side_span(
                id,
                NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator),
                operator_span,
            );
        }

        Ok(id)
    }

    /// Parse one element in a leading type binary list.
    ///
    /// Examples:
    /// ```ds
    /// | A
    /// | readonly A
    /// | A<T>
    /// ```
    fn eat_type_leading_binary_element(
        &mut self,
        minimum_precedence: u16,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        self.bump();
        let flags = self
            .flags
            .not_in_position()
            .in_type()
            .disallow_type_conditional();
        let scope = TypeScope::from_flags(flags).at_precedence(Some(minimum_precedence));

        if Self::is_type_expression_boundary_token(self.peek_token_type()) {
            return Ok(self.recover_missing_type_expression_here(NodeType::TypeExpression));
        }

        self.eat_type_operand(scope)
    }

    /// Parse one type prefix operator.
    ///
    /// Examples:
    /// ```ds
    /// keyof T
    /// readonly T
    /// local shared T
    /// ```
    pub(crate) fn eat_type_prefix(
        &mut self,
        start: &ParserSpanStart,
        operator: TypeUnaryOperator,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let operator_start = self.span_start();
        self.bump();
        let operator_span = self.get_span_from(&operator_start);
        let target_type = self.eat_type_prefix_operand(operator.precedence())?;

        let expression = match operator {
            TypeUnaryOperator::Keyof => TypeExpression::KeyOf { target_type },
            TypeUnaryOperator::Readonly => TypeExpression::Readonly { target_type },
            TypeUnaryOperator::Local => TypeExpression::Local { target_type },
            TypeUnaryOperator::Shared => TypeExpression::Shared { target_type },
            TypeUnaryOperator::Not => TypeExpression::Not { target_type },
        };

        let id = self.insert_node(expression, self.get_span_from(start));
        self.tree.set_main_span(id, operator_span);

        Ok(id)
    }

    /// Parse the operand after one type prefix operator.
    ///
    /// Examples:
    /// ```ds
    /// T
    /// keyof T
    /// Array<T>
    /// ```
    fn eat_type_prefix_operand(
        &mut self,
        minimum_precedence: u16,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let flags = self.type_nested_flags();
        let scope = TypeScope::from_flags(flags).at_precedence(Some(minimum_precedence));

        if Self::is_type_expression_boundary_token(self.peek_token_type()) {
            return Ok(self.recover_missing_type_expression_here(NodeType::TypeExpression));
        }

        self.eat_type_operand(scope)
    }

    /// Return whether a type can receive tagged object literal construction.
    pub(crate) fn can_start_tagged_object_literal_type(
        &self,
        type_expression_id: LocalNodeId<TypeExpression>,
    ) -> bool {
        matches!(
            self.tree.get(type_expression_id),
            TypeExpression::Reference { .. }
                | TypeExpression::Member { .. }
                | TypeExpression::Declaration { .. }
                | TypeExpression::FunctionTypeDeclaration(_)
                | TypeExpression::ConstructorTypeDeclaration(_)
        )
    }
}
