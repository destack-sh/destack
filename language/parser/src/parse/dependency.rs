use crate::{Parser, ParserError, ParserResult};

use destack_core::StringId;
use destack_dir::{
    Argument, DependencyBinding, DependencyForm, DependencyItem, Expression, ImportAttribute,
    ImportAttributeClause, ImportAttributeClauseKind, ImportAttributeValue, Key, Keyword,
    LocalNodeId, Name, NodeType, Property, TokenLiteral, TokenType,
};
use destack_source::{NodeSpanList, NodeSpanRegion, NodeSpanType, Span};

/// One parsed import attribute clause plus parser owned source spans.
#[derive(Debug, Clone)]
struct ParsedImportAttributeClause {
    /// The decoded import attribute clause.
    clause: ImportAttributeClause,
    /// The full source span of the clause.
    span: Span,
    /// The source spans of the attribute entries.
    attribute_spans: Vec<Span>,
}

impl Parser {
    /// Eat an import declaration (including the `import` keyword and an optional body).
    ///
    /// Examples:
    /// ```
    /// import "foo"
    /// import "foo.bar"
    /// import * as foo from "foo"
    /// import { bar, baz } from "foo"
    /// import Default, { type Item } from "foo"
    /// import foo as baz with { bar: true }
    /// ```
    pub fn eat_import(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        // keyword
        self.eat_keyword(Keyword::Import)?;

        // source form
        let form = if self.should_parse_import_type_modifier() {
            self.bump(); // eat type
            Some(DependencyForm::Type)
        } else {
            None
        };

        // binding
        let mut has_binding = false;
        let items = if self.peek_dependency_binding_is() {
            has_binding = true;
            let allow_type_modifier = form != Some(DependencyForm::Type);
            Some(self.eat_dependency_items_block(allow_type_modifier, false)?)
        } else {
            None
        };

        if has_binding {
            self.eat_keyword(Keyword::From)?;
        }

        // allow bare import targets on the next line (`import\n"foo"` and comment separated forms)
        let (target, target_span) = self.eat_dependency_target_with_span()?;

        // arguments
        let parsed_attributes = self.eat_dependency_arguments_maybe()?;
        let attributes = parsed_attributes
            .as_ref()
            .map(|parsed_attributes| parsed_attributes.clause.clone());

        // import
        let import_id = self.insert_node(
            Expression::Import {
                form: form.unwrap_or(DependencyForm::Plain),
                target,
                items,
                attributes,
            },
            self.get_span_from(&start),
        );

        // set main span to the import target string
        self.tree.set_main_span(import_id, target_span);

        // set import attribute source spans
        if let Some(parsed_attributes) = parsed_attributes {
            self.set_import_attribute_clause_spans(import_id, &parsed_attributes)?;
        }

        Ok(import_id)
    }

    /// Decide whether `type` after `import` is a type-only modifier.
    fn should_parse_import_type_modifier(&mut self) -> bool {
        if !self.is_keyword(Keyword::Type) {
            return false;
        }

        // binding forms like `import type { ... }` or `import type * as`
        if matches!(
            self.token_type_at_offset(1),
            TokenType::OpenBrace | TokenType::Multiply
        ) {
            return true;
        }

        // identifier bindings like `import type A from "a"`
        if self.token_type_at_offset(1) != TokenType::Identifier {
            return false;
        }

        if self.keyword_at_offset(1) == Some(Keyword::From) {
            return self.token_type_at_offset(2) == TokenType::Assign
                || self.keyword_at_offset(2) == Some(Keyword::From);
        }

        true
    }

    /// Eat an export declaration (including the `export` keyword and an optional body).
    ///
    /// Examples:
    /// ```
    /// export "foo"
    /// export * from "foo"
    /// export * as foo from "foo"
    /// export { bar, baz } from "foo"
    /// export { bar as bar, baz }
    /// export default foo
    /// ```
    pub fn eat_export(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.span_start();

        self.eat_keyword(Keyword::Export)?;

        // export default <expression>
        if self.is_keyword(Keyword::Default) {
            self.bump(); // eat default

            // reject export default enum declarations
            if self.is_keyword(Keyword::Enum) {
                return Err(ParserError::unexpected(self.peek()?.span));
            }

            let value =
                self.eat_expression(self.flags.not_in_position().not_in_sequence_expression())?;
            let item = self.insert_node(
                DependencyItem::Binding {
                    binding: DependencyBinding::Default,
                    form: Some(DependencyForm::Plain),
                    name: None,
                    alias: None,
                    value: Some(value),
                },
                self.get_span_from(&start),
            );
            let export = self.insert_node(
                Expression::Export {
                    form: DependencyForm::Plain,
                    target: None,
                    items: vec![item],
                    attributes: None,
                },
                self.get_span_from(&start),
            );
            return Ok(export);
        }

        // source form
        let form = if self.is_keyword(Keyword::Type) {
            self.bump(); // eat type
            Some(DependencyForm::Type)
        } else {
            None
        };

        // export * from
        let has_namespace_reexport_from =
            self.peek_is(TokenType::Multiply) && { self.next_keyword() == Some(Keyword::From) };
        if has_namespace_reexport_from {
            self.bump(); // eat *
            self.eat_keyword(Keyword::From)?;
            let (target, target_span) = self.eat_dependency_target_with_span()?;
            let parsed_attributes = self.eat_dependency_arguments_maybe()?;
            let attributes = parsed_attributes
                .as_ref()
                .map(|parsed_attributes| parsed_attributes.clause.clone());
            let item = DependencyItem::Binding {
                binding: DependencyBinding::Namespace,
                form: None,
                name: None,
                alias: None,
                value: None,
            };
            let item_id = self.insert_node(item, self.get_span_from(&start));
            let export = self.insert_node(
                Expression::Export {
                    form: form.unwrap_or(DependencyForm::Plain),
                    target: Some(target),
                    items: vec![item_id],
                    attributes,
                },
                self.get_span_from(&start),
            );

            // set main span to the export target string
            self.tree.set_main_span(export, target_span);

            // set import attribute source spans
            if let Some(parsed_attributes) = parsed_attributes {
                self.set_import_attribute_clause_spans(export, &parsed_attributes)?;
            }

            return Ok(export);
        }

        // require a binding after export and optional type modifier
        if self.peek_dependency_binding().is_err() {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        // binding
        let allow_type_modifier = form != Some(DependencyForm::Type);
        let items = self.eat_dependency_items_block(allow_type_modifier, true)?;
        let has_from_target = self.is_keyword(Keyword::From);
        let (target, target_span) = if has_from_target {
            self.eat_keyword(Keyword::From)?;
            let (target, span) = self.eat_dependency_target_with_span()?;
            (Some(target), Some(span))
        } else {
            (None, None)
        };

        // assertions or attributes
        let parsed_attributes = if target.is_some() {
            self.eat_dependency_arguments_maybe()?
        } else {
            None
        };
        let attributes = parsed_attributes
            .as_ref()
            .map(|parsed_attributes| parsed_attributes.clause.clone());

        // `export { default }` without `from` is invalid
        // (default is a reserved word and can't be a local binding)
        if target.is_none() {
            for item_id in &items {
                let item = self.tree.get(*item_id);
                if let DependencyItem::Binding {
                    binding: DependencyBinding::Default,
                    name: None,
                    ..
                } = item
                {
                    let span = self
                        .tree
                        .get_main_span(*item_id)
                        .unwrap_or(self.tree.get_span(*item_id));
                    return Err(ParserError::unexpected(span));
                }
            }
        }

        // export
        let export_id = self.insert_node(
            Expression::Export {
                form: form.unwrap_or(DependencyForm::Plain),
                target,
                items,
                attributes,
            },
            self.get_span_from(&start),
        );

        // set main span to the export target string if present
        if let Some(target_span) = target_span {
            self.tree.set_main_span(export_id, target_span);
        }

        // set import attribute source spans
        if let Some(parsed_attributes) = parsed_attributes {
            self.set_import_attribute_clause_spans(export_id, &parsed_attributes)?;
        }

        Ok(export_id)
    }

    /// Decode one parsed expression node into one import attribute value.
    ///
    /// Import attribute values are plain data, not syntax nodes, so this is a
    /// boundary decode from the richer expression tree into the static attribute model.
    fn decode_import_attribute_value(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> ParserResult<ImportAttributeValue> {
        let expression = self.tree.get(expression_id).clone();

        Ok(match expression {
            Expression::ScalarLiteral(value) => ImportAttributeValue::ScalarLiteral(value),
            Expression::ArrayExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());

                for element_id in elements {
                    let Argument::Positional { value } = self.tree.get(element_id) else {
                        return Err(ParserError::unexpected(self.tree.get_span(element_id)));
                    };

                    values.push(self.decode_import_attribute_value(*value)?);
                }

                ImportAttributeValue::Array(values)
            }
            Expression::ObjectExpression { properties } => {
                let mut attributes = Vec::with_capacity(properties.len());

                for property_id in properties {
                    let Property::Field { key, value, .. } = self.tree.get(property_id) else {
                        return Err(ParserError::unexpected(self.tree.get_span(property_id)));
                    };

                    let Key::Name(key) = *key else {
                        return Err(ParserError::unexpected(self.tree.get_span(property_id)));
                    };
                    let value = self.decode_import_attribute_value(*value)?;

                    attributes.push(ImportAttribute { key, value });
                }

                ImportAttributeValue::Object(attributes)
            }
            _ => return Err(ParserError::unexpected(self.tree.get_span(expression_id))),
        })
    }

    /// Decode one parsed named argument into one import attribute entry.
    fn decode_import_attribute(
        &mut self,
        argument_id: LocalNodeId<Argument>,
    ) -> ParserResult<ImportAttribute> {
        let argument = self.tree.get(argument_id).clone();

        let Argument::Named { name, value } = argument else {
            return Err(ParserError::unexpected(self.tree.get_span(argument_id)));
        };

        let value = self.decode_import_attribute_value(value)?;

        Ok(ImportAttribute { key: name, value })
    }

    /// Set source spans for one parsed import attribute clause.
    fn set_import_attribute_clause_spans(
        &mut self,
        node_id: LocalNodeId<Expression>,
        parsed_attributes: &ParsedImportAttributeClause,
    ) -> ParserResult<()> {
        self.tree.set_side_span(
            node_id,
            NodeSpanType::Region(NodeSpanRegion::Clause),
            parsed_attributes.span,
        );

        for (index, attribute_span) in parsed_attributes.attribute_spans.iter().enumerate() {
            let Ok(segment) = u16::try_from(index) else {
                return Err(ParserError::unexpected(*attribute_span));
            };
            self.tree.set_side_span(
                node_id,
                NodeSpanType::ListItem(NodeSpanList::Entry, segment),
                *attribute_span,
            );
        }

        Ok(())
    }

    /// Eat dependency arguments for import/export attributes.
    fn eat_dependency_arguments_maybe(
        &mut self,
    ) -> ParserResult<Option<ParsedImportAttributeClause>> {
        // attribute clause head
        if !self.is_keyword(Keyword::With) {
            return Ok(None);
        }

        let start = self.span_start();
        self.bump(); // eat with

        // attribute clause body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)?;
        let argument_flags = self.flags.nested();
        let arguments = self.with_flags(argument_flags, |parser| {
            parser.eat_arguments_body(TokenType::CloseBrace)
        })?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Expression)?;
        let span = self.get_span_from(&start);
        let attribute_spans = arguments
            .iter()
            .map(|argument_id| self.tree.get_span(*argument_id))
            .collect();

        // decoded attributes
        let mut attributes = Vec::with_capacity(arguments.len());

        for argument_id in arguments {
            attributes.push(self.decode_import_attribute(argument_id)?);
        }

        let clause = ImportAttributeClause {
            kind: ImportAttributeClauseKind::With,
            attributes,
        };

        Ok(Some(ParsedImportAttributeClause {
            clause,
            span,
            attribute_spans,
        }))
    }

    /// Peek a dependency binding.
    pub(crate) fn peek_dependency_binding(&mut self) -> ParserResult<()> {
        if self.peek_dependency_binding_is() {
            Ok(())
        } else {
            Err(ParserError::unexpected(self.peek()?.span))
        }
    }

    /// Return true when the next tokens can start a dependency binding.
    #[inline]
    pub(crate) fn peek_dependency_binding_is(&mut self) -> bool {
        if self.peek_is(TokenType::OpenBrace) || self.peek_is(TokenType::Multiply) {
            return true;
        }

        if self.peek_is(TokenType::Identifier) {
            let next_token_type = self.next_token_type();
            return next_token_type == TokenType::Comma
                || self.next_keyword() == Some(Keyword::From);
        }

        false
    }

    /// Return true when tokens after `import` can start an import statement.
    pub(crate) fn can_start_import_statement(&mut self) -> bool {
        matches!(
            self.next_token_type(),
            TokenType::Identifier | TokenType::OpenBrace | TokenType::Multiply | TokenType::Literal
        )
    }

    /// Eat an dependency target and return both the string and its span.
    ///
    /// Examples:
    /// ```
    /// "foo"
    /// "foo/bar:something"
    /// ```
    fn eat_dependency_target_with_span(&mut self) -> ParserResult<(StringId, Span)> {
        let token = *self.peek_token(TokenType::Literal)?;

        // module targets accept regular string literals, including unterminated ones for recovery
        let is_valid_target = matches!(
            token.token.literal(),
            Some(TokenLiteral::String {
                has_invalid_escape: false,
                ..
            })
        );
        if !is_valid_target {
            return Err(ParserError::expected(token.span, TokenType::Literal));
        }

        let content = self.get_string_literal_str(token).to_owned();
        let string_id = self.strings.intern(&content);
        self.bump();

        Ok((string_id, token.span))
    }

    /// Eat a dependency items block.
    ///
    /// Examples:
    /// ```
    /// foo
    /// * as foo
    /// Default, { a, b }
    /// { a, b }
    /// ```
    fn eat_dependency_items_block(
        &mut self,
        allow_type_modifier: bool,
        allow_literal_alias: bool,
    ) -> ParserResult<Vec<LocalNodeId<DependencyItem>>> {
        let mut items: Vec<LocalNodeId<DependencyItem>> = Vec::new();

        // `Default,` or `foo from`
        let can_start_default_item = if self.peek_is(TokenType::Identifier) {
            self.next_token_type() == TokenType::Comma || self.next_keyword() == Some(Keyword::From)
        } else {
            false
        };

        if can_start_default_item {
            let start = self.span_start();
            let (alias, alias_span) = self.eat_identifier_with_span()?;

            // parse optional default binding separator
            if self.peek_is(TokenType::Comma) {
                self.bump(); // eat comma

                // require a supported binding continuation
                if !self.peek_is(TokenType::OpenBrace) && !self.peek_is(TokenType::Multiply) {
                    return Err(ParserError::unexpected(self.peek()?.span));
                }
            }
            let item = DependencyItem::Binding {
                binding: DependencyBinding::Default,
                form: None,
                name: None,
                alias: Some(alias),
                value: None,
            };
            let item_id = self.insert_node(item, self.get_span_from(&start));
            self.tree.set_main_span(item_id, alias_span);
            items.push(item_id);
        }

        // `* as foo` (can follow a default import)
        if self.peek_is(TokenType::Multiply) && self.is_next_keyword(Keyword::As) {
            let start = self.span_start();
            self.bump(); // eat *
            self.bump(); // eat as
            let (alias, alias_span) = if self.peek_is(TokenType::Literal) {
                self.eat_string_literal_with_span()?
            } else {
                self.eat_identifier_with_span()?
            };
            let item = DependencyItem::Binding {
                binding: DependencyBinding::Namespace,
                form: None,
                name: None,
                alias: Some(alias),
                value: None,
            };
            let item_id = self.insert_node(item, self.get_span_from(&start));
            self.tree.set_main_span(item_id, alias_span);
            items.push(item_id);
        }

        // main items
        if items.is_empty() || self.peek_is(TokenType::OpenBrace) {
            self.eat_token(TokenType::OpenBrace)?;

            while !self.peek_is(TokenType::CloseBrace) {
                let item_start = self.span_start();

                let item = match self.eat_dependency_item(allow_type_modifier, allow_literal_alias)
                {
                    Ok(item) => item,
                    Err(error) => {
                        self.try_recover_in_item_list(
                            &item_start,
                            TokenType::CloseBrace,
                            Some(error),
                        )?;

                        self.insert_node(DependencyItem::Error, self.get_span_from(&item_start))
                    }
                };

                items.push(item);

                if self.peek_is(TokenType::CloseBrace) {
                    break;
                }

                if self.peek_comma_is() {
                    self.eat_item_stop()?;

                    // recover a missing close brace before the clause boundary
                    if self.is_keyword(Keyword::From)
                        || Self::is_any_stop_token(self.peek_token_type())
                    {
                        break;
                    }

                    continue;
                }

                if self.is_keyword(Keyword::From) || Self::is_any_stop_token(self.peek_token_type())
                {
                    break;
                }

                return Err(ParserError::unexpected(self.peek()?.span));
            }

            self.eat_close_token_or_recover_missing_with(
                TokenType::CloseBrace,
                NodeType::DependencyItem,
                |parser, token_type| {
                    parser.is_keyword(Keyword::From) || Self::is_any_stop_token(token_type)
                },
            )?;
        }

        Ok(items)
    }

    /// Eat a dependency item (like `geometry` or `geometry as geom`).
    ///
    /// Examples:
    /// ```
    /// geometry
    /// geometry as geom
    /// ```
    pub(crate) fn eat_dependency_item(
        &mut self,
        allow_type_modifier: bool,
        allow_literal_alias: bool,
    ) -> ParserResult<LocalNodeId<DependencyItem>> {
        let start = self.span_start();

        // source form
        let form = if self.should_parse_dependency_type_modifier() {
            if !allow_type_modifier {
                let span = self.peek()?.span;
                return Err(ParserError::unexpected(span));
            }
            self.bump(); // eat type
            Some(DependencyForm::Type)
        } else {
            None
        };

        // default
        if self.is_keyword(Keyword::Default) {
            self.bump(); // eat default

            // alias
            let has_alias_separator =
                self.is_keyword(Keyword::As) || self.peek_is(TokenType::Colon);
            let (alias, alias_span) = if has_alias_separator {
                self.bump(); // eat `as` or `:`
                let (alias, alias_span) =
                    self.eat_dependency_item_alias_with_span(allow_literal_alias)?;
                (Some(alias), Some(alias_span))
            } else {
                (None, None)
            };

            // item
            let item = self.insert_node(
                DependencyItem::Binding {
                    binding: DependencyBinding::Default,
                    form,
                    name: None,
                    alias,
                    value: None,
                },
                self.get_span_from(&start),
            );
            if let Some(alias_span) = alias_span {
                self.tree.set_main_span(item, alias_span);
            }
            Ok(item)
        }
        // item
        else {
            // name
            let (name, name_span) = self.eat_dependency_item_name_with_span()?;

            // alias
            let has_alias_separator =
                self.is_keyword(Keyword::As) || self.peek_is(TokenType::Colon);
            let (alias, alias_span) = if has_alias_separator {
                self.bump(); // eat `as` or `:`
                let (alias, alias_span) =
                    self.eat_dependency_item_alias_with_span(allow_literal_alias)?;
                (Some(alias), Some(alias_span))
            } else {
                (None, None)
            };

            // item
            let item = self.insert_node(
                DependencyItem::Binding {
                    binding: DependencyBinding::Named,
                    form,
                    name: Some(name),
                    alias,
                    value: None,
                },
                self.get_span_from(&start),
            );
            self.tree
                .set_side_span(item, NodeSpanType::Region(NodeSpanRegion::Type), name_span);
            let main_span = alias_span.unwrap_or(name_span);
            self.tree.set_main_span(item, main_span);
            Ok(item)
        }
    }

    /// Decide whether `type` should be parsed as a dependency item modifier.
    fn should_parse_dependency_type_modifier(&mut self) -> bool {
        // require `type` keyword
        if !self.is_keyword(Keyword::Type) {
            return false;
        }

        // require a name after `type`
        if !matches!(
            self.token_type_at_offset(1),
            TokenType::Identifier | TokenType::Literal
        ) {
            return false;
        }

        // handle `type as` disambiguation
        if self.is_next_keyword(Keyword::As) {
            if self.token_type_at_offset(2) != TokenType::Identifier {
                return true;
            }

            if self.is_next_next_keyword(Keyword::As) {
                return self.token_type_at_offset(3) == TokenType::Identifier;
            }

            return false;
        }

        true
    }

    /// Eat a dependency item name (identifier or string literal) and its span.
    fn eat_dependency_item_name_with_span(&mut self) -> ParserResult<(Name, Span)> {
        if self.peek_is(TokenType::Identifier) {
            let (name, span) = self.eat_identifier_with_span()?;
            return Ok((Name::Identifier(name), span));
        }

        if self.peek_string_literal_is() {
            let (name, span) = self.eat_string_literal_with_span()?;
            return Ok((Name::String(name), span));
        }

        Err(ParserError::expected(
            self.peek()?.span,
            TokenType::Identifier,
        ))
    }

    /// Eat a dependency alias and return its interned string and span.
    fn eat_dependency_item_alias_with_span(
        &mut self,
        allow_literal_alias: bool,
    ) -> ParserResult<(StringId, Span)> {
        // identifier aliases are always valid
        if self.peek_is(TokenType::Identifier) {
            return self.eat_identifier_with_span();
        }

        // export specifiers also allow string literal aliases
        if allow_literal_alias && self.peek_string_literal_is() {
            return self.eat_string_literal_with_span();
        }

        // export specifiers also allow keyword like literal aliases: true, false
        if allow_literal_alias
            && self.peek().is_ok_and(|token| {
                token.token.ty() == TokenType::Literal
                    && matches!(token.token.literal(), Some(TokenLiteral::Boolean { .. }))
            })
        {
            let span = self.peek()?.span;
            let alias = self.get_span_str(span).to_string();
            let alias = self.strings.intern(&alias);
            self.bump();
            return Ok((alias, span));
        }

        // all other forms are invalid aliases
        Err(ParserError::expected(
            self.peek()?.span,
            TokenType::Identifier,
        ))
    }
}
