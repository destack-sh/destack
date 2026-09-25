use crate::parse::r#type::operator::{TypeOperator, TypePrefixOperator};
use crate::parse::{DeclarationHeader, TypePosition, TypeStop};
use crate::{ParseStart, Parser, ParserError, ParserResult};
use smallvec::{SmallVec, smallvec};
use tspp_dir::{
    Access, LocalNodeId, Mutability, NodeType, OperatorPrecedence, RangeEnd, TokenType,
    TypeExpression, VarianceBound,
};
use tspp_source::{ByteRange, NodeSpanBoundary, NodeSpanType};

/// One consumed type prefix operation.
#[derive(Debug, Copy, Clone)]
enum TypePrefix {
    /// One ordinary type prefix.
    Unary {
        /// The classified prefix operation.
        operator: TypePrefixOperator,
        /// The operator source range.
        range: ByteRange,
    },
    /// One owned reference prefix.
    Owned {
        /// The mutability modifier.
        mutability: Option<Mutability>,
        /// The variance modifier.
        variance: Option<VarianceBound>,
        /// The operator source range.
        range: ByteRange,
    },
    /// One borrowed reference prefix.
    Borrowed {
        /// The named borrow lifetime.
        lifetime: Option<LocalNodeId<TypeExpression>>,
        /// The borrow access.
        access: Option<Access>,
        /// The variance modifier.
        variance: Option<VarianceBound>,
        /// The operator source range.
        range: ByteRange,
    },
    /// One pointer prefix.
    Pointer {
        /// The mutability modifier.
        mutability: Option<Mutability>,
        /// The operator source range.
        range: ByteRange,
    },
}

impl Parser {
    /// Return whether one token offset starts a symbolic memory type prefix.
    pub(in crate::parse) fn peek_memory_type_prefix_at(&self, offset: usize) -> bool {
        matches!(
            self.peek_token_type_at(offset),
            TokenType::ElementwiseAnd
                | TokenType::ElementwiseXor
                | TokenType::LogicalAnd
                | TokenType::Multiply
        )
    }

    /// Return whether one token offset can start a type operand.
    pub(in crate::parse) fn peek_type_operand_start_at(&self, offset: usize) -> bool {
        let token = self.peek_token_type_at(offset);
        let keyword = (token == TokenType::Identifier)
            .then(|| self.peek_keyword_at(offset))
            .flatten();

        // accept every type prefix
        if TypePrefixOperator::from_token(token, keyword).is_some()
            || self.peek_memory_type_prefix_at(offset)
        {
            return true;
        }

        // accept every primary type head
        if matches!(
            token,
            TokenType::Identifier
                | TokenType::OpenParenthesis
                | TokenType::LessThan
                | TokenType::OpenBracket
                | TokenType::OpenBrace
                | TokenType::ElementwiseOr
                | TokenType::TemplateString
                | TokenType::TemplateStringStart
                | TokenType::Literal
                | TokenType::Range
                | TokenType::RangeInclusive
        ) {
            return true;
        }

        // signed scalar types require one literal after the sign
        matches!(token, TokenType::Add | TokenType::Subtract)
            && self.peek_token_type_at(offset + 1) == TokenType::Literal
    }

    /// Parse one type operand through all prefix and postfix operations.
    #[inline(never)]
    pub(in crate::parse::r#type) fn parse_type_operand(
        &mut self,
        position: TypePosition,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        // parse prefixes only when the operand actually has one
        let prefixes = if let Some(first) = self.parse_type_prefix()? {
            let mut prefixes: SmallVec<[TypePrefix; 4]> = smallvec![first];
            while let Some(prefix) = self.parse_type_prefix()? {
                prefixes.push(prefix);
            }

            Some(prefixes)
        } else {
            None
        };

        // parse the primary type and every postfix
        let primary_start = self.mark_parse_start();
        let mut ty = if prefixes.is_some() && self.peek_type_expression_recovery_boundary() {
            self.recover_missing_type_expression_here(NodeType::TypeExpression)
        } else {
            let ty = self.parse_type_primary(&primary_start, position, stop)?;

            self.parse_type_postfix(&primary_start, ty, stop)?
        };

        // fold consumed prefixes from the operand outward
        if let Some(prefixes) = prefixes {
            for prefix in prefixes.into_iter().rev() {
                ty = self.insert_type_prefix(prefix, ty);
            }
        }

        Ok(ty)
    }

    /// Parse one type prefix when present.
    fn parse_type_prefix(&mut self) -> ParserResult<Option<TypePrefix>> {
        let token_type = self.peek_token_type();
        let keyword = (token_type == TokenType::Identifier)
            .then(|| self.peek_keyword())
            .flatten();

        // parse one ordinary type prefix
        if let Some(operator) = TypePrefixOperator::from_token(token_type, keyword) {
            let range = self.peek_token().range();
            self.bump();

            return Ok(Some(TypePrefix::Unary { operator, range }));
        }

        // stop when no symbolic memory prefix follows
        if !self.peek_memory_type_prefix_at(0) {
            return Ok(None);
        }

        // parse one pointer prefix
        if token_type == TokenType::Multiply {
            let range = self.peek_token().range();
            self.bump();
            let mutability = Some(self.parse_reference_mutability());

            return Ok(Some(TypePrefix::Pointer { mutability, range }));
        }

        // parse one owned or borrowed reference prefix
        let token = self.eat_reference_prefix_operator()?.token;
        let lifetime =
            match token.is(TokenType::ElementwiseAnd) && self.peek_is(TokenType::Lifetime) {
                true => Some(self.parse_lifetime_type()),
                false => None,
            };
        let prefix = if token.is(TokenType::ElementwiseAnd) {
            let access = Some(self.parse_borrow_access()?);
            let variance = self.parse_variance_bound_if_present();
            TypePrefix::Borrowed {
                lifetime,
                access,
                variance,
                range: token.range(),
            }
        } else {
            let mutability = Some(self.parse_reference_mutability());
            let variance = self.parse_variance_bound_if_present();
            TypePrefix::Owned {
                mutability,
                variance,
                range: token.range(),
            }
        };

        Ok(Some(prefix))
    }

    /// Fold one consumed type prefix around its operand.
    fn insert_type_prefix(
        &mut self,
        prefix: TypePrefix,
        target_type: LocalNodeId<TypeExpression>,
    ) -> LocalNodeId<TypeExpression> {
        let (ty, operator_range) = match prefix {
            TypePrefix::Unary { operator, range } => {
                let ty = match operator {
                    TypePrefixOperator::Keyof => TypeExpression::KeyOf { target_type },
                    TypePrefixOperator::Readonly => TypeExpression::Readonly { target_type },
                    TypePrefixOperator::Not => TypeExpression::Not { target_type },
                };

                (ty, range)
            }
            TypePrefix::Owned {
                mutability,
                variance,
                range,
            } => (
                TypeExpression::OwnedOf {
                    mutability,
                    variance,
                    target_type,
                },
                range,
            ),
            TypePrefix::Borrowed {
                lifetime,
                access,
                variance,
                range,
            } => (
                TypeExpression::BorrowedOf {
                    lifetime,
                    access,
                    variance,
                    target_type,
                },
                range,
            ),
            TypePrefix::Pointer { mutability, range } => (
                TypeExpression::PointerOf {
                    mutability,
                    target_type,
                },
                range,
            ),
        };
        let target_range = self.tree.get_range(target_type);
        let source_range = ByteRange {
            start: operator_range.start,
            end: target_range.end,
        };
        let ty = self.insert_node(ty, source_range);
        self.tree.set_main_range(ty, operator_range);

        ty
    }

    /// Parse one peeked tick name into a lifetime type expression.
    fn parse_lifetime_type(&mut self) -> LocalNodeId<TypeExpression> {
        let range = self.peek_token().range();
        let name = self.intern_range(range);
        self.bump();

        let lifetime = self.insert_node(TypeExpression::Lifetime { name }, range);
        self.tree.set_main_range(lifetime, range);

        lifetime
    }

    /// Parse one primary type without prefix or postfix operations.
    fn parse_type_primary(
        &mut self,
        start: &ParseStart,
        position: TypePosition,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        match self.peek_token_type() {
            TokenType::Identifier => self.parse_identifier_type_primary(start, stop),
            TokenType::Lifetime => Ok(self.parse_lifetime_type()),
            TokenType::OpenParenthesis if self.peek_parenthesized_function_type(position, stop) => {
                self.parse_function_type(start, DeclarationHeader::default())
            }
            TokenType::OpenParenthesis => self.parse_parenthesized_type(start, stop),
            TokenType::LessThan if self.peek_generic_function_type() => {
                self.parse_function_type(start, DeclarationHeader::default())
            }
            TokenType::OpenBracket => self.parse_bracket_type(start, stop),
            TokenType::OpenBrace => self.parse_object_type_primary(start, stop),
            TokenType::ElementwiseOr => {
                self.parse_leading_type_list(position, stop, TypeOperator::Union)
            }
            TokenType::TemplateString | TokenType::TemplateStringStart
                if self.peek_template_literal_start() =>
            {
                self.parse_type_template_literal(stop)
            }
            TokenType::Add | TokenType::Subtract => {
                let value = self.parse_signed_numeric_literal()?;

                Ok(self.insert_node(TypeExpression::Literal { value }, self.range_since(start)))
            }
            TokenType::Literal if self.peek_scalar_literal_start() => {
                let value = self.parse_scalar_literal()?;

                Ok(self.insert_node(TypeExpression::Literal { value }, self.range_since(start)))
            }
            TokenType::Range | TokenType::RangeInclusive => {
                self.parse_startless_range_type(start, position, stop)
            }
            _ => Err(ParserError::unexpected(self.peek_token_span())),
        }
    }

    /// Parse one identifier-shaped primary type.
    fn parse_identifier_type_primary(
        &mut self,
        start: &ParseStart,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        if self.peek_construct_type() {
            return self.parse_function_type(start, DeclarationHeader::default());
        }
        if let Some(keyword) = self.peek_keyword()
            && let Some(ty) = self.parse_type_keyword_expression(start, keyword, stop)?
        {
            return Ok(ty);
        }

        self.parse_type_reference(start, stop)
    }

    /// Parse one object or mapped primary type.
    fn parse_object_type_primary(
        &mut self,
        start: &ParseStart,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        if self.peek_mapped_type() {
            return self.parse_mapped_type(stop);
        }

        let members = self.parse_type_object_literal()?;

        Ok(self.insert_node(TypeExpression::Object { members }, self.range_since(start)))
    }

    /// Parse a type list with one leading separator.
    fn parse_leading_type_list(
        &mut self,
        position: TypePosition,
        stop: TypeStop,
        operator: TypeOperator,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let operator_range = self.peek_token().range();
        self.bump();
        let mut elements = Vec::new();
        let first = self.parse_type_at(position, stop, operator.precedence())?;
        let mut last = first;
        elements.push(first);

        while self.peek_is(TokenType::ElementwiseOr) {
            self.bump();
            let element = self.parse_type_at(position, stop, operator.precedence())?;
            last = element;
            elements.push(element);
        }

        let first_range = self.tree.get_range(first);
        let last_range = self.tree.get_range(last);
        let source_range = ByteRange {
            start: first_range.start,
            end: last_range.end,
        };
        let ty = if operator == TypeOperator::Union {
            TypeExpression::Union { elements }
        } else {
            TypeExpression::Intersection { elements }
        };
        let ty = self.insert_node(ty, source_range);
        self.tree
            .set_head_range(ty, self.type_expression_head_range(first));
        self.tree.set_side_range(
            ty,
            NodeSpanType::Boundary(NodeSpanBoundary::Leading),
            ByteRange {
                start: operator_range.start,
                end: first_range.start,
            },
        );
        self.tree.set_side_range(
            ty,
            NodeSpanType::Boundary(NodeSpanBoundary::LeadingOperator),
            operator_range,
        );

        Ok(ty)
    }

    /// Parse one startless range type.
    fn parse_startless_range_type(
        &mut self,
        start: &ParseStart,
        position: TypePosition,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let operator_range = self.peek_token().range();
        let end_kind = if self.peek_is(TokenType::RangeInclusive) {
            RangeEnd::Inclusive
        } else {
            RangeEnd::Open
        };
        self.bump();
        let end = if self.peek_type_range_end_omitted() {
            if end_kind == RangeEnd::Open {
                None
            } else {
                Some(self.recover_missing_type_expression_here(NodeType::TypeExpression))
            }
        } else {
            Some(self.parse_type_at(position, stop, OperatorPrecedence::Range)?)
        };
        let ty = self.insert_node(
            TypeExpression::Range {
                start: None,
                end,
                end_kind,
            },
            self.range_since(start),
        );
        self.tree.set_main_range(ty, operator_range);

        Ok(ty)
    }
}
