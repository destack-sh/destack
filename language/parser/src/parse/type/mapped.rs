use crate::{ParseResult, Parser};

use destack_dir::{
    Keyword, LocalNodeId, MappedTypeModifier, NodeType, TokenType, TypeExpression,
    TypeMappedParameter,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Return true when the current token starts a mapped type head.
    pub(crate) fn can_start_type_mapped_expression(&mut self) -> bool {
        // mapped types always start with `{`
        if !self.peek_is(TokenType::OpenBrace) {
            return false;
        }

        self.lookahead(|parser| {
            // skip the opening `{`
            parser.bump();

            // `+readonly` and `-readonly` only start mapped types
            if parser.peek_is(TokenType::Add) || parser.peek_is(TokenType::Subtract) {
                parser.bump();
                return parser.is_keyword(Keyword::Readonly);
            }

            // optional `readonly`
            if parser.is_keyword(Keyword::Readonly) {
                parser.bump();
            }

            // mapped heads require `[K in`
            if !parser.peek_is(TokenType::OpenBracket) {
                return false;
            }
            parser.bump();

            if !parser.peek_is(TokenType::Identifier) {
                return false;
            }
            parser.bump();

            parser.is_keyword(Keyword::In)
        })
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
        let start = self.span_start();

        // mapped body: `{ ... }`
        self.eat_token(TokenType::OpenBrace)?;

        // readonly modifier: `readonly`, `+readonly`, `-readonly`
        let readonly = self.eat_type_mapped_readonly_modifier()?;

        let mapped_head_start = self.span_start();
        self.eat_token(TokenType::OpenBracket)?;

        // parameter: `[K in keyof T]`
        let (name, name_span) = self.eat_identifier_with_span()?;
        self.eat_keyword(Keyword::In)?;
        let source_type = self.eat_type_expression_or_recover_missing(
            self.flags
                .not_in_position()
                .not_in_left_precedence()
                .in_type()
                .in_type_mapped_constraint(),
            NodeType::TypeExpression,
        )?;

        // key remap: `[K in T as ...]`
        let mut key_remap = None;
        if self.is_keyword(Keyword::As) {
            let as_span = self.eat_keyword(Keyword::As)?.span;
            self.set_node_trailing_span(source_type, as_span.start);
            let remap_boundary_start = as_span.end;
            let remap_expression = self.eat_type_expression_or_recover_missing(
                self.flags
                    .not_in_position()
                    .not_in_left_precedence()
                    .in_type(),
                NodeType::TypeExpression,
            )?;
            self.set_node_leading_span(remap_expression, remap_boundary_start);

            key_remap = Some(remap_expression);
        }

        self.eat_type_token_or_recover_missing(TokenType::CloseBracket, NodeType::TypeExpression)?;
        let mapped_head_span = self.get_span_from(&mapped_head_start);

        // optional modifier: `?`, `+?`, `-?`
        let optional = self.eat_type_mapped_optional_modifier()?;

        // mapped value: `: T`
        let (value, value_type_span) = if self.peek_is(TokenType::Colon) {
            let value_type_start = self.span_start();
            self.eat_token(TokenType::Colon)?;
            let value_boundary_start = self.prev_token_end();
            let value = self.eat_type_expression_node_or_recover_missing(
                self.flags
                    .not_in_position()
                    .not_in_left_precedence()
                    .in_type(),
                NodeType::TypeExpression,
            )?;
            self.set_node_leading_span(value, value_boundary_start);
            let value_type_span = self.get_span_from(&value_type_start);
            (Some(value), Some(value_type_span))
        } else {
            (None, None)
        };
        if self.peek_is(TokenType::Semicolon) || self.peek_is(TokenType::Comma) {
            self.bump();
        }

        // mapped value trailing boundary
        let mapped_close_start = self.peek()?.span.start;
        if let Some(value) = value {
            self.set_node_trailing_span(value, mapped_close_start);
        }

        self.eat_type_token_or_recover_missing(TokenType::CloseBrace, NodeType::TypeExpression)?;

        // mapped parameter node
        let parameter = self.insert_node(
            TypeMappedParameter {
                name,
                source_type,
                key_remap,
            },
            mapped_head_span,
        );

        self.tree.set_main_span(parameter, name_span);

        // mapped type node
        let mapped_id = self.insert_node(
            TypeExpression::Mapped {
                parameter,
                readonly,
                optional,
                value,
            },
            self.get_span_from(&start),
        );
        self.tree
            .set_side_span(mapped_id, NodeSpanType::Head, mapped_head_span);
        if let Some(value_type_span) = value_type_span {
            self.tree.set_side_span(
                mapped_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                value_type_span,
            );
        }
        self.tree.set_main_span(mapped_id, name_span);

        Ok(mapped_id)
    }

    /// Eat one mapped readonly modifier.
    fn eat_type_mapped_readonly_modifier(&mut self) -> ParseResult<MappedTypeModifier> {
        // readonly modifier: `readonly`, `+readonly`, `-readonly`
        if self.is_keyword(Keyword::Readonly) {
            self.bump(); // eat readonly
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
        let has_readonly_after_operator = self.lookahead(|parser| {
            parser.bump();
            parser.is_keyword(Keyword::Readonly)
        });
        if has_readonly_after_operator {
            self.bump(); // eat + or -
            self.bump(); // eat readonly
            return Ok(modifier);
        }

        Ok(MappedTypeModifier::None)
    }

    /// Eat one mapped optional modifier.
    fn eat_type_mapped_optional_modifier(&mut self) -> ParseResult<MappedTypeModifier> {
        // ?
        if self.peek_is(TokenType::Maybe) {
            self.bump(); // eat ?
            return Ok(MappedTypeModifier::Present);
        }

        // -?
        if self.peek_is(TokenType::Subtract)
            && self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::Maybe)
            })
        {
            self.bump(); // eat -
            self.bump(); // eat ?
            return Ok(MappedTypeModifier::Remove);
        }

        // +?
        if self.peek_is(TokenType::Add)
            && self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::Maybe)
            })
        {
            self.bump(); // eat +
            self.bump(); // eat ?
            return Ok(MappedTypeModifier::Add);
        }

        Ok(MappedTypeModifier::None)
    }
}
