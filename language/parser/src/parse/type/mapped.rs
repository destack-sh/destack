use crate::parse::{TypePosition, TypeStop};
use crate::{Parser, ParserResult};

use tspp_dir::{
    Keyword, LocalNodeId, MappedTypeModifier, NodeType, TokenType, TypeExpression,
    TypeMappedParameter,
};
use tspp_source::{ByteRange, NodeSpanRegion, NodeSpanType};

/// One mapped type head.
struct MappedTypeHead {
    /// The mapped parameter node.
    parameter: LocalNodeId<TypeMappedParameter>,
    /// The full mapped head range.
    range: ByteRange,
    /// The mapped key name range.
    name_range: ByteRange,
}

/// One mapped type value.
struct MappedTypeValue {
    /// The mapped value type.
    value: LocalNodeId<TypeExpression>,
    /// The mapped value range.
    range: ByteRange,
}

impl Parser {
    /// Parse one mapped type expression.
    ///
    /// Examples:
    /// ```tspp
    /// { [K in keyof T]: T[K] }
    /// { readonly [K in keyof T]?: T[K] }
    /// { [K in keyof T as `get${K}`]: T[K] }
    /// ```
    pub(crate) fn parse_mapped_type(
        &mut self,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let start = self.mark_parse_start();

        // mapped body: `{ ... }`
        self.eat_token(TokenType::OpenBrace)?;

        let readonly = self.parse_type_mapped_readonly_modifier();
        let head = self.parse_mapped_type_head(stop)?;
        let optional = self.parse_type_mapped_optional_modifier();
        let value = self.parse_mapped_type_value(stop)?;

        // consume an optional member terminator
        if self.peek_is(TokenType::Semicolon) || self.peek_is(TokenType::Comma) {
            self.bump();
        }

        // mapped value trailing boundary
        let mapped_close_start = self.peek_token_span().span.start;
        if let Some(value) = value.as_ref() {
            self.set_node_trailing_range(value.value, mapped_close_start);
        }

        self.eat_type_token_or_recover_missing(TokenType::CloseBrace, NodeType::TypeExpression)?;

        // mapped type node
        let value_id = value.as_ref().map(|value| value.value);
        let mapped_id = self.insert_node(
            TypeExpression::Mapped {
                parameter: head.parameter,
                readonly,
                optional,
                value: value_id,
            },
            self.range_since(&start),
        );
        self.tree
            .set_side_range(mapped_id, NodeSpanType::Head, head.range);
        if let Some(value) = value {
            self.tree.set_side_range(
                mapped_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                value.range,
            );
        }
        self.tree.set_main_range(mapped_id, head.name_range);

        Ok(mapped_id)
    }

    /// Return true when the current token starts a mapped type head.
    pub(crate) fn peek_mapped_type(&self) -> bool {
        // mapped types always start with `{`
        if !self.peek_is(TokenType::OpenBrace) {
            return false;
        }

        self.peek_mapped_type_at(1)
    }

    /// Return whether one token offset starts a mapped type head.
    fn peek_mapped_type_at(&self, mut offset: usize) -> bool {
        if matches!(
            self.peek_token_type_at(offset),
            TokenType::Add | TokenType::Subtract
        ) {
            return self.peek_keyword_at(offset + 1) == Some(Keyword::Readonly);
        }

        if self.peek_keyword_at(offset) == Some(Keyword::Readonly) {
            offset += 1;
        }

        self.peek_token_type_at(offset) == TokenType::OpenBracket
            && self.peek_token_type_at(offset + 1) == TokenType::Identifier
            && self.peek_keyword_at(offset + 2) == Some(Keyword::In)
    }

    /// Parse a mapped type head.
    ///
    /// Examples:
    /// ```tspp
    /// [K in keyof T]
    /// [K in keyof T as `get${K}`]
    /// [P in keyof Model as P]
    /// ```
    fn parse_mapped_type_head(&mut self, stop: TypeStop) -> ParserResult<MappedTypeHead> {
        let documentation = self.parse_documentation();
        let start = self.mark_parse_start();
        self.eat_token(TokenType::OpenBracket)?;

        let (name, name_range) = self.eat_identifier_with_range()?;
        self.eat_keyword(Keyword::In)?;
        let source_type = self.parse_type_or_recover_missing(
            TypePosition::Type,
            stop.nest(),
            NodeType::TypeExpression,
        )?;
        let key_remap = self.parse_mapped_type_key(source_type, stop)?;

        self.eat_type_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;
        let range = self.range_since(&start);
        let parameter = self.insert_node(
            TypeMappedParameter {
                name,
                source_type,
                key_remap,
            },
            range,
        );
        self.tree.set_main_range(parameter, name_range);
        self.attach_documentation(parameter, documentation);

        Ok(MappedTypeHead {
            parameter,
            range,
            name_range,
        })
    }

    /// Parse a mapped key remap when present.
    ///
    /// Examples:
    /// ```tspp
    /// as K
    /// as `get${K}`
    /// as Exclude<K, "id">
    /// ```
    fn parse_mapped_type_key(
        &mut self,
        source_type: LocalNodeId<TypeExpression>,
        stop: TypeStop,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        if !self.peek_is_keyword(Keyword::As) {
            return Ok(None);
        }

        let as_range = self.eat_keyword(Keyword::As)?.range();
        self.set_node_trailing_range(source_type, as_range.start);
        let remap_expression = self.parse_type_or_recover_missing(
            TypePosition::Type,
            stop.nest(),
            NodeType::TypeExpression,
        )?;
        self.set_node_leading_range(remap_expression, as_range.end);

        Ok(Some(remap_expression))
    }

    /// Parse a mapped value type when present.
    ///
    /// Examples:
    /// ```tspp
    /// : T[K]
    /// : readonly T[K]
    /// : T[K] | undefined
    /// ```
    fn parse_mapped_type_value(&mut self, stop: TypeStop) -> ParserResult<Option<MappedTypeValue>> {
        if !self.peek_is(TokenType::Colon) {
            return Ok(None);
        }

        let start = self.mark_parse_start();
        self.eat_token(TokenType::Colon)?;
        let boundary_start = self.peek_previous_token_end();
        let value = self.parse_type_or_recover_missing(
            TypePosition::Type,
            stop.nest(),
            NodeType::TypeExpression,
        )?;
        self.set_node_leading_range(value, boundary_start);
        let range = self.range_since(&start);

        Ok(Some(MappedTypeValue { value, range }))
    }

    /// Parse one mapped readonly modifier.
    ///
    /// Examples:
    /// ```tspp
    /// readonly [K in keyof T]
    /// +readonly [K in keyof T]
    /// -readonly [K in keyof T]
    /// ```
    fn parse_type_mapped_readonly_modifier(&mut self) -> MappedTypeModifier {
        // readonly modifier: `readonly`, `+readonly`, `-readonly`
        if self.peek_is_keyword(Keyword::Readonly) {
            self.bump();
            return MappedTypeModifier::Present;
        }

        // +/- readonly
        let modifier = if self.peek_is(TokenType::Subtract) {
            MappedTypeModifier::Remove
        } else if self.peek_is(TokenType::Add) {
            MappedTypeModifier::Add
        } else {
            return MappedTypeModifier::None;
        };

        // allow line breaks between `+` or `-` and `readonly`
        let has_readonly_after_operator = self.peek_keyword_at(1) == Some(Keyword::Readonly);
        if has_readonly_after_operator {
            self.bump();
            self.bump();
            return modifier;
        }

        MappedTypeModifier::None
    }

    /// Parse one mapped optional modifier.
    ///
    /// Examples:
    /// ```tspp
    /// [K in keyof T]?
    /// [K in keyof T]+?
    /// [K in keyof T]-?
    /// ```
    fn parse_type_mapped_optional_modifier(&mut self) -> MappedTypeModifier {
        // ?
        if self.peek_is(TokenType::Maybe) {
            self.bump();
            return MappedTypeModifier::Present;
        }

        // -?
        if self.peek_token_type_at(1) == TokenType::Maybe {
            if self.peek_is(TokenType::Subtract) {
                self.bump();
                self.bump();
                return MappedTypeModifier::Remove;
            }

            if self.peek_is(TokenType::Add) {
                self.bump();
                self.bump();
                return MappedTypeModifier::Add;
            }
        }

        MappedTypeModifier::None
    }
}
