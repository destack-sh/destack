use crate::{Parser, ParserResult};

use destack_dir::{
    Keyword, LocalNodeId, MappedTypeModifier, NodeType, TokenType, TypeExpression,
    TypeMappedParameter,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

/// Parsed mapped type head.
struct MappedTypeHead {
    /// The mapped parameter node.
    parameter: LocalNodeId<TypeMappedParameter>,
    /// The full mapped head span.
    span: destack_source::Span,
    /// The mapped key name span.
    name_span: destack_source::Span,
}

/// Parsed mapped type value.
struct MappedTypeValue {
    /// The optional mapped value type.
    value: Option<LocalNodeId<TypeExpression>>,
    /// The optional mapped value span.
    span: Option<destack_source::Span>,
}

impl Parser {
    /// Eat one mapped type expression.
    ///
    /// Examples:
    /// ```ds
    /// { [K in keyof T]: T[K] }
    /// { readonly [K in keyof T]?: T[K] }
    /// { [K in keyof T as `get${K}`]: T[K] }
    /// ```
    pub fn eat_type_mapped_expression(&mut self) -> ParserResult<LocalNodeId<TypeExpression>> {
        let start = self.span_start();

        // mapped body: `{ ... }`
        self.eat_token(TokenType::OpenBrace)?;

        let readonly = self.eat_type_mapped_readonly_modifier()?;
        let head = self.eat_type_mapped_head()?;
        let optional = self.eat_type_mapped_optional_modifier()?;
        let value = self.eat_type_mapped_value()?;

        if self.peek_is(TokenType::Semicolon) || self.peek_is(TokenType::Comma) {
            self.bump();
        }

        // mapped value trailing boundary
        let mapped_close_start = self.peek()?.span.start;
        if let Some(value) = value.value {
            self.set_node_trailing_span(value, mapped_close_start);
        }

        self.eat_type_token_or_recover_missing(TokenType::CloseBrace, NodeType::TypeExpression)?;

        // mapped type node
        let mapped_id = self.insert_node(
            TypeExpression::Mapped {
                parameter: head.parameter,
                readonly,
                optional,
                value: value.value,
            },
            self.get_span_from(&start),
        );
        self.tree
            .set_side_span(mapped_id, NodeSpanType::Head, head.span);
        if let Some(value_type_span) = value.span {
            self.tree.set_side_span(
                mapped_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                value_type_span,
            );
        }
        self.tree.set_main_span(mapped_id, head.name_span);

        Ok(mapped_id)
    }

    /// Return true when the current token starts a mapped type head.
    pub(crate) fn can_start_type_mapped_expression(&mut self) -> bool {
        // mapped types always start with `{`
        if !self.peek_is(TokenType::OpenBrace) {
            return false;
        }

        self.token_offset_starts_mapped_type_head(1)
    }

    /// Return whether one token offset starts a mapped type head.
    fn token_offset_starts_mapped_type_head(&mut self, mut offset: usize) -> bool {
        if matches!(
            self.token_type_at_offset(offset),
            TokenType::Add | TokenType::Subtract
        ) {
            return self.keyword_at_offset(offset + 1) == Some(Keyword::Readonly);
        }

        if self.keyword_at_offset(offset) == Some(Keyword::Readonly) {
            offset += 1;
        }

        self.token_type_at_offset(offset) == TokenType::OpenBracket
            && self.token_type_at_offset(offset + 1) == TokenType::Identifier
            && self.keyword_at_offset(offset + 2) == Some(Keyword::In)
    }

    /// Eat a mapped type head.
    ///
    /// Examples:
    /// ```ds
    /// [K in keyof T]
    /// [K in keyof T as `get${K}`]
    /// [P in keyof Model as P]
    /// ```
    fn eat_type_mapped_head(&mut self) -> ParserResult<MappedTypeHead> {
        let start = self.span_start();
        self.eat_token(TokenType::OpenBracket)?;

        let (name, name_span) = self.eat_identifier_with_span()?;
        self.eat_keyword(Keyword::In)?;
        let source_type = self.eat_type_expression_or_recover_missing(
            self.flags
                .not_in_position()
                .in_type()
                .in_type_mapped_constraint(),
            NodeType::TypeExpression,
        )?;
        let key_remap = self.eat_type_mapped_key_remap(source_type)?;

        self.eat_type_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;
        let span = self.get_span_from(&start);
        let parameter = self.insert_node(
            TypeMappedParameter {
                name,
                source_type,
                key_remap,
            },
            span,
        );
        self.tree.set_main_span(parameter, name_span);

        Ok(MappedTypeHead {
            parameter,
            span,
            name_span,
        })
    }

    /// Eat a mapped key remap when present.
    ///
    /// Examples:
    /// ```ds
    /// as K
    /// as `get${K}`
    /// as Exclude<K, "id">
    /// ```
    fn eat_type_mapped_key_remap(
        &mut self,
        source_type: LocalNodeId<TypeExpression>,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        if !self.is_keyword(Keyword::As) {
            return Ok(None);
        }

        let as_span = self.eat_keyword(Keyword::As)?.span;
        self.set_node_trailing_span(source_type, as_span.start);
        let remap_expression = self.eat_type_expression_or_recover_missing(
            self.flags.not_in_position().in_type(),
            NodeType::TypeExpression,
        )?;
        self.set_node_leading_span(remap_expression, as_span.end);

        Ok(Some(remap_expression))
    }

    /// Eat a mapped value type when present.
    ///
    /// Examples:
    /// ```ds
    /// : T[K]
    /// : readonly T[K]
    /// : T[K] | undefined
    /// ```
    fn eat_type_mapped_value(&mut self) -> ParserResult<MappedTypeValue> {
        if !self.peek_is(TokenType::Colon) {
            return Ok(MappedTypeValue {
                value: None,
                span: None,
            });
        }

        let start = self.span_start();
        self.eat_token(TokenType::Colon)?;
        let boundary_start = self.prev_token_end();
        let value = self.eat_type_expression_or_recover_missing(
            self.flags.not_in_position().in_type(),
            NodeType::TypeExpression,
        )?;
        self.set_node_leading_span(value, boundary_start);
        let span = self.get_span_from(&start);

        Ok(MappedTypeValue {
            value: Some(value),
            span: Some(span),
        })
    }

    /// Eat one mapped readonly modifier.
    ///
    /// Examples:
    /// ```ds
    /// readonly [K in keyof T]
    /// +readonly [K in keyof T]
    /// -readonly [K in keyof T]
    /// ```
    fn eat_type_mapped_readonly_modifier(&mut self) -> ParserResult<MappedTypeModifier> {
        // readonly modifier: `readonly`, `+readonly`, `-readonly`
        if self.is_keyword(Keyword::Readonly) {
            self.bump();
            return Ok(MappedTypeModifier::Present);
        }

        // +/- readonly
        let modifier = if self.peek_is(TokenType::Subtract) {
            MappedTypeModifier::Remove
        } else if self.peek_is(TokenType::Add) {
            MappedTypeModifier::Add
        } else {
            return Ok(MappedTypeModifier::None);
        };

        // allow line breaks between `+` or `-` and `readonly`
        let has_readonly_after_operator = self.keyword_at_offset(1) == Some(Keyword::Readonly);
        if has_readonly_after_operator {
            self.bump();
            self.bump();
            return Ok(modifier);
        }

        Ok(MappedTypeModifier::None)
    }

    /// Eat one mapped optional modifier.
    ///
    /// Examples:
    /// ```ds
    /// [K in keyof T]?
    /// [K in keyof T]+?
    /// [K in keyof T]-?
    /// ```
    fn eat_type_mapped_optional_modifier(&mut self) -> ParserResult<MappedTypeModifier> {
        // ?
        if self.peek_is(TokenType::Maybe) {
            self.bump();
            return Ok(MappedTypeModifier::Present);
        }

        // -?
        if self.token_type_at_offset(1) == TokenType::Maybe {
            if self.peek_is(TokenType::Subtract) {
                self.bump();
                self.bump();
                return Ok(MappedTypeModifier::Remove);
            }

            if self.peek_is(TokenType::Add) {
                self.bump();
                self.bump();
                return Ok(MappedTypeModifier::Add);
            }
        }

        Ok(MappedTypeModifier::None)
    }
}
