use crate::{ParseResult, Parser};

use destack_ast::{
    Keyword, LocalNodeId, NodeType, TokenType, TypeExpression, TypeMappedParameter, TypeModifier,
};

impl Parser {
    /// Return true when the current token starts a mapped type head.
    pub(crate) fn can_start_type_mapped_expression(&mut self) -> bool {
        // mapped types always start with `{`
        if !self.peek_is(TokenType::OpenBrace) {
            return false;
        }

        // skip the opening `{`
        let mut look_index = self.next_non_newline_index_from(self.pos_index().saturating_add(1));

        // `+readonly` and `-readonly` only start mapped types
        let look_token_type = self.token_type_at(look_index);
        if look_token_type == TokenType::Add || look_token_type == TokenType::Subtract {
            look_index = self.next_non_newline_index_from(look_index.saturating_add(1));
            return self.token_type_at(look_index) == TokenType::Identifier
                && self.keyword_for_index(look_index) == Some(Keyword::Readonly);
        }

        // optional `readonly`
        if look_token_type == TokenType::Identifier
            && self.keyword_for_index(look_index) == Some(Keyword::Readonly)
        {
            look_index = self.next_non_newline_index_from(look_index.saturating_add(1));
        }

        // mapped heads require `[` after the modifier prefix
        if self.token_type_at(look_index) != TokenType::OpenBracket {
            return false;
        }

        // mapped keys must start with an identifier
        let name_index = self.next_non_newline_index_from(look_index.saturating_add(1));
        if self.token_type_at(name_index) != TokenType::Identifier {
            return false;
        }

        // mapped heads require `in` after the key name
        let in_index = self.next_non_newline_index_from(name_index.saturating_add(1));
        self.token_type_at(in_index) == TokenType::Identifier
            && self.keyword_for_index(in_index) == Some(Keyword::In)
    }

    /// Eat one mapped type expression.
    ///
    /// Examples:
    /// ```
    /// { [K in keyof T]: T[K] }
    /// { readonly [K in keyof T]?: T[K] }
    /// { [K in keyof T as `get${K}`]: T[K] }
    /// ```
    pub fn eat_type_mapped_expression(&mut self) -> ParseResult<LocalNodeId<TypeExpression>> {
        let start = self.mark_span();

        // mapped body: `{ ... }`
        self.eat_token(TokenType::OpenBrace)?;
        self.eat_newlines_maybe()?;

        // readonly modifier: `readonly`, `+readonly`, `-readonly`
        let readonly = self.eat_type_mapped_readonly_modifier()?;

        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::OpenBracket)?;
        self.eat_newlines_maybe()?;

        // parameter: `[K in keyof T]`
        let name = self.eat_identifier()?;
        self.eat_newlines_maybe()?;
        self.eat_keyword(Keyword::In)?;
        self.eat_newlines_maybe()?;
        let source_type = self.eat_type_expression_or_recover_missing(
            self.options
                .not_in_position()
                .not_in_left_precedence()
                .in_type()
                .in_type_mapped_constraint(),
            NodeType::TypeExpression,
        )?;

        // key remap: `[K in T as ...]`
        let mut key_remap = None;
        self.eat_newlines_maybe()?;
        if self.is_keyword(Keyword::As) {
            let as_span = self.eat_keyword(Keyword::As)?.span;
            self.set_node_trailing_span(source_type, as_span.start);
            let remap_boundary_start = as_span.end;
            self.eat_newlines_maybe()?;
            let remap_expression = self.eat_type_expression_or_recover_missing(
                self.options
                    .not_in_position()
                    .not_in_left_precedence()
                    .in_type(),
                NodeType::TypeExpression,
            )?;
            self.set_node_leading_span(remap_expression, remap_boundary_start);

            key_remap = Some(remap_expression);
        }

        self.eat_newlines_maybe()?;
        self.eat_type_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;

        // optional modifier: `?`, `+?`, `-?`
        let optional = self.eat_type_mapped_optional_modifier()?;

        // mapped value: `: T`
        self.eat_newlines_maybe()?;
        let value = if self.peek_is(TokenType::Colon) {
            self.eat_token(TokenType::Colon)?;
            let value_boundary_start = self.prev_token_end();
            self.eat_newlines_maybe()?;
            let value = self.eat_type_expression_node_or_recover_missing(
                self.options
                    .not_in_position()
                    .not_in_left_precedence()
                    .in_type(),
                NodeType::TypeExpression,
            )?;
            self.set_node_leading_span(value, value_boundary_start);
            value
        } else {
            self.recover_missing_type_expression_here(NodeType::TypeExpression)
        };
        self.eat_newlines_maybe()?;
        if self.peek_is(TokenType::Semicolon) || self.peek_is(TokenType::Comma) {
            self.bump();
            self.eat_newlines_maybe()?;
        }

        // mapped value trailing boundary
        let mapped_close_start = self.peek()?.span.start;
        self.set_node_trailing_span(value, mapped_close_start);

        self.eat_type_token_or_recover_missing(TokenType::CloseBrace, NodeType::TypeExpression)?;

        // mapped type node
        let parameter = TypeMappedParameter {
            name,
            source_type,
            key_remap,
        };
        let mapped_id = self.insert_node(
            TypeExpression::Mapped {
                parameter,
                readonly,
                optional,
                value,
            },
            self.get_span_from(&start),
        );

        Ok(mapped_id)
    }

    /// Eat one mapped readonly modifier.
    fn eat_type_mapped_readonly_modifier(&mut self) -> ParseResult<TypeModifier> {
        // readonly modifier: `readonly`, `+readonly`, `-readonly`
        if self.is_keyword(Keyword::Readonly) {
            self.bump(); // eat readonly
            return Ok(TypeModifier::Present);
        }

        // +/- readonly
        let modifier = if self.peek_is(TokenType::Subtract) {
            TypeModifier::Remove
        } else if self.peek_is(TokenType::Add) {
            TypeModifier::Add
        } else {
            return Ok(TypeModifier::None);
        };

        // allow line breaks between `+` or `-` and `readonly`
        let readonly_index = self.next_non_newline_index_from(self.pos_index().saturating_add(1));
        let has_readonly_after_operator = self.token_type_at(readonly_index)
            == TokenType::Identifier
            && self.keyword_for_index(readonly_index) == Some(Keyword::Readonly);
        if has_readonly_after_operator {
            self.bump(); // eat + or -
            self.eat_newlines_maybe()?;
            self.bump(); // eat readonly
            return Ok(modifier);
        }

        Ok(TypeModifier::None)
    }

    /// Eat one mapped optional modifier.
    fn eat_type_mapped_optional_modifier(&mut self) -> ParseResult<TypeModifier> {
        // ?
        if self.peek_is(TokenType::Maybe) {
            self.bump(); // eat ?
            return Ok(TypeModifier::Present);
        }

        // -?
        if self.peek_is(TokenType::Subtract) && self.peek_next_is(TokenType::Maybe) {
            self.bump(); // eat -
            self.bump(); // eat ?
            return Ok(TypeModifier::Remove);
        }

        // +?
        if self.peek_is(TokenType::Add) && self.peek_next_is(TokenType::Maybe) {
            self.bump(); // eat +
            self.bump(); // eat ?
            return Ok(TypeModifier::Add);
        }

        Ok(TypeModifier::None)
    }
}
