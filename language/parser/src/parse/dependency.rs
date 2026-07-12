use crate::parse::context::{ExpressionContext, FunctionContext};
use crate::{Parser, ParserError, ParserResult};

use destack_core::StringId;
use destack_dir::{
    Argument, DependencyBinding, DependencyForm, DependencyItem, Expression, ImportAttribute,
    ImportAttributeClause, ImportAttributeClauseKind, ImportAttributeValue, Key, Keyword,
    LocalNodeId, Name, NodeType, Property, TokenLiteral, TokenType,
};
use destack_source::{ByteRange, NodeSpanList, NodeSpanRegion, NodeSpanType};

/// One import attribute clause and its source ranges.
#[derive(Debug, Clone)]
struct ImportClause {
    /// The decoded import attribute clause.
    clause: ImportAttributeClause,
    /// The full source range of the clause.
    range: ByteRange,
    /// The source ranges of the attribute entries.
    attribute_ranges: Vec<ByteRange>,
}

impl Parser {
    /// Parse an import declaration (including the `import` keyword and an optional body).
    ///
    /// Examples:
    /// ```ds
    /// import "foo"
    /// import "foo.bar"
    /// import * as foo from "foo"
    /// import { bar, baz } from "foo"
    /// import Default, { type Item } from "foo"
    /// import foo as baz with { bar: true }
    /// ```
    pub(crate) fn parse_import(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        // keyword
        self.eat_keyword(Keyword::Import)?;

        // source form
        let form = if self.peek_import_type_modifier() {
            self.bump();
            DependencyForm::Type
        } else {
            DependencyForm::Plain
        };

        // binding
        let items = if self.peek_dependency_binding() {
            let allow_type_modifier = form != DependencyForm::Type;
            Some(self.parse_dependency_items(allow_type_modifier, false, function)?)
        } else {
            None
        };

        if items.is_some() {
            self.eat_keyword(Keyword::From)?;
        }

        // allow bare import targets on the next line (`import\n"foo"` and comment separated forms)
        let (target, target_range) = self.eat_dependency_target_with_range()?;

        // arguments
        let import_clause = self.parse_import_clause(function)?;
        let attributes = import_clause
            .as_ref()
            .map(|import_clause| import_clause.clause.clone());

        // import
        let import_id = self.insert_node(
            Expression::Import {
                form,
                target,
                items,
                attributes,
            },
            self.range_since(&start),
        );

        // set the main source range to the import target string
        self.tree.set_main_range(import_id, target_range);

        // set import attribute source ranges
        if let Some(import_clause) = import_clause {
            self.set_import_attribute_ranges(import_id, &import_clause)?;
        }

        Ok(import_id)
    }

    /// Decide whether `type` after `import` is a type-only modifier.
    fn peek_import_type_modifier(&self) -> bool {
        if !self.peek_is_keyword(Keyword::Type) {
            return false;
        }

        // binding forms like `import type { ... }` or `import type * as`
        if matches!(
            self.peek_token_type_at(1),
            TokenType::OpenBrace | TokenType::Multiply
        ) {
            return true;
        }

        // identifier bindings like `import type A from "a"`
        if self.peek_token_type_at(1) != TokenType::Identifier {
            return false;
        }

        if self.peek_keyword_at(1) == Some(Keyword::From) {
            return self.peek_token_type_at(2) == TokenType::Assign
                || self.peek_keyword_at(2) == Some(Keyword::From);
        }

        true
    }

    /// Parse an export declaration (including the `export` keyword and an optional body).
    ///
    /// Examples:
    /// ```ds
    /// export "foo"
    /// export * from "foo"
    /// export * as foo from "foo"
    /// export { bar, baz } from "foo"
    /// export { bar as bar, baz }
    /// export default foo
    /// ```
    pub(crate) fn parse_export(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        self.eat_keyword(Keyword::Export)?;

        // export default <expression>
        if self.peek_is_keyword(Keyword::Default) {
            self.bump();

            // reject export default enum declarations
            if self.peek_is_keyword(Keyword::Enum) {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            let value = self.parse_expression(ExpressionContext {
                function,
                ..ExpressionContext::default()
            })?;
            let item = self.insert_node(
                DependencyItem::Binding {
                    binding: DependencyBinding::Default,
                    form: Some(DependencyForm::Plain),
                    name: None,
                    alias: None,
                    value: Some(value),
                },
                self.range_since(&start),
            );
            let export = self.insert_node(
                Expression::Export {
                    form: DependencyForm::Plain,
                    target: None,
                    items: vec![item],
                    attributes: None,
                },
                self.range_since(&start),
            );
            return Ok(export);
        }

        // source form
        let form = if self.peek_is_keyword(Keyword::Type) {
            self.bump();
            DependencyForm::Type
        } else {
            DependencyForm::Plain
        };

        // namespace export
        if self.peek_is(TokenType::Multiply) {
            let item_start = self.mark_parse_start();
            self.bump();

            // optional exported namespace name
            let (alias, alias_range) = if self.peek_is_keyword(Keyword::As) {
                self.bump();
                if self.peek_is_keyword(Keyword::From) {
                    return Err(ParserError::unexpected(self.peek_token_span()));
                }

                let (alias, range) = self.eat_dependency_item_alias_with_range(true)?;
                (Some(alias), Some(range))
            } else {
                (None, None)
            };

            // required module target
            self.eat_keyword(Keyword::From)?;
            let (target, target_range) = self.eat_dependency_target_with_range()?;
            let import_clause = self.parse_import_clause(function)?;
            let attributes = import_clause
                .as_ref()
                .map(|import_clause| import_clause.clause.clone());
            let item = DependencyItem::Binding {
                binding: DependencyBinding::Namespace,
                form: None,
                name: None,
                alias,
                value: None,
            };
            let item_id = self.insert_node(item, self.range_since(&item_start));
            if let Some(alias_range) = alias_range {
                self.tree.set_main_range(item_id, alias_range);
            }

            // export declaration
            let export = self.insert_node(
                Expression::Export {
                    form,
                    target: Some(target),
                    items: vec![item_id],
                    attributes,
                },
                self.range_since(&start),
            );

            // set the main source range to the export target string
            self.tree.set_main_range(export, target_range);

            // set import attribute source ranges
            if let Some(import_clause) = import_clause {
                self.set_import_attribute_ranges(export, &import_clause)?;
            }

            return Ok(export);
        }

        // require a binding after export and optional type modifier
        if !self.peek_dependency_binding() {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        // binding
        let allow_type_modifier = form != DependencyForm::Type;
        let items = self.parse_dependency_items(allow_type_modifier, true, function)?;

        // optional target for named exports
        let has_from_target = self.peek_is_keyword(Keyword::From);
        let (target, target_range) = if has_from_target {
            self.eat_keyword(Keyword::From)?;
            let (target, range) = self.eat_dependency_target_with_range()?;
            (Some(target), Some(range))
        } else {
            (None, None)
        };

        // assertions or attributes
        let import_clause = if target.is_some() {
            self.parse_import_clause(function)?
        } else {
            None
        };
        let attributes = import_clause
            .as_ref()
            .map(|import_clause| import_clause.clause.clone());

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
                    let range = self
                        .tree
                        .get_main_range(*item_id)
                        .unwrap_or(self.tree.get_range(*item_id));
                    return Err(ParserError::unexpected(range));
                }
            }
        }

        // export
        let export_id = self.insert_node(
            Expression::Export {
                form,
                target,
                items,
                attributes,
            },
            self.range_since(&start),
        );

        // set the export target string as the main source range
        if let Some(target_range) = target_range {
            self.tree.set_main_range(export_id, target_range);
        }

        // set import attribute source ranges
        if let Some(import_clause) = import_clause {
            self.set_import_attribute_ranges(export_id, &import_clause)?;
        }

        Ok(export_id)
    }

    /// Decode one expression node into an import attribute value.
    ///
    /// Import attributes store values directly rather than retaining their expression nodes.
    fn decode_import_attribute_value(
        &self,
        expression_id: LocalNodeId<Expression>,
    ) -> ParserResult<ImportAttributeValue> {
        let expression = self.tree.get(expression_id);

        Ok(match expression {
            Expression::ScalarLiteral(value) => ImportAttributeValue::ScalarLiteral(*value),
            Expression::ArrayExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());

                for &element_id in elements {
                    let Argument::Positional { value } = self.tree.get(element_id) else {
                        return Err(ParserError::unexpected(self.tree.get_range(element_id)));
                    };

                    values.push(self.decode_import_attribute_value(*value)?);
                }

                ImportAttributeValue::Array(values)
            }
            Expression::ObjectExpression { properties } => {
                let mut attributes = Vec::with_capacity(properties.len());

                for &property_id in properties {
                    let Property::Field { key, value, .. } = self.tree.get(property_id) else {
                        return Err(ParserError::unexpected(self.tree.get_range(property_id)));
                    };

                    let Key::Name(key) = key else {
                        return Err(ParserError::unexpected(self.tree.get_range(property_id)));
                    };
                    let value = self.decode_import_attribute_value(*value)?;

                    attributes.push(ImportAttribute { key: *key, value });
                }

                ImportAttributeValue::Object(attributes)
            }
            _ => return Err(ParserError::unexpected(self.tree.get_range(expression_id))),
        })
    }

    /// Decode one named argument into an import attribute entry.
    fn decode_import_attribute(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> ParserResult<ImportAttribute> {
        let Argument::Named { name, value } = self.tree.get(argument_id) else {
            return Err(ParserError::unexpected(self.tree.get_range(argument_id)));
        };

        let value = self.decode_import_attribute_value(*value)?;

        Ok(ImportAttribute { key: *name, value })
    }

    /// Record source regions for one import attribute clause.
    fn set_import_attribute_ranges(
        &mut self,
        node_id: LocalNodeId<Expression>,
        import_clause: &ImportClause,
    ) -> ParserResult<()> {
        self.tree.set_side_range(
            node_id,
            NodeSpanType::Region(NodeSpanRegion::Attributes),
            import_clause.range,
        );

        for (index, attribute_range) in import_clause.attribute_ranges.iter().enumerate() {
            let Ok(segment) = u16::try_from(index) else {
                return Err(ParserError::unexpected(*attribute_range));
            };
            self.tree.set_side_range(
                node_id,
                NodeSpanType::ListItem(NodeSpanList::Entry, segment),
                *attribute_range,
            );
        }

        Ok(())
    }

    /// Parse dependency arguments for import/export attributes.
    fn parse_import_clause(
        &mut self,
        function: FunctionContext,
    ) -> ParserResult<Option<ImportClause>> {
        // attribute clause head
        if !self.peek_is_keyword(Keyword::With) {
            return Ok(None);
        }

        let start = self.mark_parse_start();
        self.bump();

        // attribute clause body
        self.eat_token_before(TokenType::OpenBrace, TokenType::CloseBrace)?;
        let arguments = self.parse_named_argument_list_body(
            TokenType::CloseBrace,
            ExpressionContext {
                function,
                ..ExpressionContext::default()
            },
        )?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Expression)?;
        let range = self.range_since(&start);
        let attribute_ranges = arguments
            .iter()
            .map(|argument_id| self.tree.get_range(*argument_id))
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

        Ok(Some(ImportClause {
            clause,
            range,
            attribute_ranges,
        }))
    }

    /// Return whether the current tokens can start a dependency binding.
    #[inline]
    pub(crate) fn peek_dependency_binding(&self) -> bool {
        if self.peek_is(TokenType::OpenBrace) || self.peek_is(TokenType::Multiply) {
            return true;
        }

        if self.peek_is(TokenType::Identifier) {
            let peek_next_token_type = self.peek_next_token_type();
            return peek_next_token_type == TokenType::Comma
                || self.peek_next_keyword() == Some(Keyword::From);
        }

        false
    }

    /// Return true when tokens after `import` can start an import statement.
    pub(crate) fn peek_import_statement(&self) -> bool {
        matches!(
            self.peek_next_token_type(),
            TokenType::Identifier | TokenType::OpenBrace | TokenType::Multiply | TokenType::Literal
        )
    }

    /// Parse a dependency target and return both the string and its byte range.
    ///
    /// Examples:
    /// ```ds
    /// "foo"
    /// "foo/bar:something"
    /// ```
    fn eat_dependency_target_with_range(&mut self) -> ParserResult<(StringId, ByteRange)> {
        let token = self.require_token(TokenType::Literal)?;

        // module targets accept regular string literals, including unterminated ones for recovery
        let is_valid_target = matches!(
            token.literal(),
            Some(TokenLiteral::String {
                has_invalid_escape: false,
                ..
            })
        );
        if !is_valid_target {
            return Err(ParserError::expected(token, TokenType::Literal));
        }

        let content = self.string_literal_str(token).to_owned();
        let string_id = self.strings.intern(&content);
        self.bump();

        Ok((string_id, token.range()))
    }

    /// Parse a dependency items block.
    ///
    /// Examples:
    /// ```ds
    /// foo
    /// * as foo
    /// Default, { a, b }
    /// { a, b }
    /// ```
    fn parse_dependency_items(
        &mut self,
        allow_type_modifier: bool,
        allow_literal_alias: bool,
        function: FunctionContext,
    ) -> ParserResult<Vec<LocalNodeId<DependencyItem>>> {
        let mut items: Vec<LocalNodeId<DependencyItem>> = Vec::new();

        // `Default,` or `foo from`
        let can_start_default_item = if self.peek_is(TokenType::Identifier) {
            self.peek_next_token_type() == TokenType::Comma
                || self.peek_next_keyword() == Some(Keyword::From)
        } else {
            false
        };

        if can_start_default_item {
            let start = self.mark_parse_start();
            let (alias, alias_range) = self.eat_identifier_with_range()?;

            // parse optional default binding separator
            if self.peek_is(TokenType::Comma) {
                self.bump();

                // require a supported binding continuation
                if !self.peek_is(TokenType::OpenBrace) && !self.peek_is(TokenType::Multiply) {
                    return Err(ParserError::unexpected(self.peek_token_span()));
                }
            }
            let item = DependencyItem::Binding {
                binding: DependencyBinding::Default,
                form: None,
                name: None,
                alias: Some(alias),
                value: None,
            };
            let item_id = self.insert_node(item, self.range_since(&start));
            self.tree.set_main_range(item_id, alias_range);
            items.push(item_id);
        }

        // `* as foo` (can follow a default import)
        if self.peek_is(TokenType::Multiply) && self.peek_next_keyword() == Some(Keyword::As) {
            let start = self.mark_parse_start();
            self.bump();
            self.bump();
            let (alias, alias_range) = if self.peek_is(TokenType::Literal) {
                self.eat_string_literal_with_range()?
            } else {
                self.eat_identifier_with_range()?
            };
            let item = DependencyItem::Binding {
                binding: DependencyBinding::Namespace,
                form: None,
                name: None,
                alias: Some(alias),
                value: None,
            };
            let item_id = self.insert_node(item, self.range_since(&start));
            self.tree.set_main_range(item_id, alias_range);
            items.push(item_id);
        }

        // main items
        if items.is_empty() || self.peek_is(TokenType::OpenBrace) {
            self.eat_token(TokenType::OpenBrace)?;

            while !self.peek_is(TokenType::CloseBrace) {
                let item_start = self.mark_parse_start();
                let decorators = self.parse_decorators(function);

                let item =
                    match self.parse_dependency_item(allow_type_modifier, allow_literal_alias) {
                        Ok(item) => item,
                        Err(error) => {
                            self.recover_list_item(
                                self.range_since(&item_start),
                                TokenType::CloseBrace,
                                error,
                            );

                            self.insert_node(DependencyItem::Error, self.range_since(&item_start))
                        }
                    };
                self.attach_decorators(item.id, decorators);

                items.push(item);

                if self.peek_is(TokenType::CloseBrace) {
                    break;
                }

                if self.peek_is(TokenType::Comma) {
                    self.eat_token(TokenType::Comma)?;

                    // recover a missing close brace before the clause boundary
                    if self.peek_is_keyword(Keyword::From)
                        || Self::is_any_stop_token(self.peek_token_type())
                    {
                        break;
                    }

                    continue;
                }

                if self.peek_is_keyword(Keyword::From)
                    || Self::is_any_stop_token(self.peek_token_type())
                {
                    break;
                }

                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            self.eat_close_token_or_recover_missing_with(
                TokenType::CloseBrace,
                NodeType::DependencyItem,
                |parser, token_type| {
                    parser.peek_is_keyword(Keyword::From) || Self::is_any_stop_token(token_type)
                },
            )?;
        }

        Ok(items)
    }

    /// Parse a dependency item (like `geometry` or `geometry as geom`).
    ///
    /// Examples:
    /// ```ds
    /// geometry
    /// geometry as geom
    /// ```
    pub(crate) fn parse_dependency_item(
        &mut self,
        allow_type_modifier: bool,
        allow_literal_alias: bool,
    ) -> ParserResult<LocalNodeId<DependencyItem>> {
        let start = self.mark_parse_start();

        // source form
        let form = if self.peek_dependency_type_modifier() {
            if !allow_type_modifier {
                let range = self.peek_token().range();
                return Err(ParserError::unexpected(range));
            }
            self.bump();
            Some(DependencyForm::Type)
        } else {
            None
        };

        // default
        if self.peek_is_keyword(Keyword::Default) {
            self.bump();

            // alias
            let has_alias_separator =
                self.peek_is_keyword(Keyword::As) || self.peek_is(TokenType::Colon);
            let (alias, alias_range) = if has_alias_separator {
                self.bump();
                let (alias, alias_range) =
                    self.eat_dependency_item_alias_with_range(allow_literal_alias)?;
                (Some(alias), Some(alias_range))
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
                self.range_since(&start),
            );
            if let Some(alias_range) = alias_range {
                self.tree.set_main_range(item, alias_range);
            }
            Ok(item)
        }
        // item
        else {
            // name
            let (name, name_range) = self.eat_dependency_item_name_with_range()?;

            // alias
            let has_alias_separator =
                self.peek_is_keyword(Keyword::As) || self.peek_is(TokenType::Colon);
            let (alias, alias_range) = if has_alias_separator {
                self.bump();
                let (alias, alias_range) =
                    self.eat_dependency_item_alias_with_range(allow_literal_alias)?;
                (Some(alias), Some(alias_range))
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
                self.range_since(&start),
            );
            self.tree
                .set_side_range(item, NodeSpanType::Region(NodeSpanRegion::Type), name_range);
            let main_range = alias_range.unwrap_or(name_range);
            self.tree.set_main_range(item, main_range);
            Ok(item)
        }
    }

    /// Decide whether `type` should be parsed as a dependency item modifier.
    fn peek_dependency_type_modifier(&self) -> bool {
        // require `type` keyword
        if !self.peek_is_keyword(Keyword::Type) {
            return false;
        }

        // require a name after `type`
        if !matches!(
            self.peek_token_type_at(1),
            TokenType::Identifier | TokenType::Literal
        ) {
            return false;
        }

        // handle `type as` disambiguation
        if self.peek_next_keyword() == Some(Keyword::As) {
            if self.peek_token_type_at(2) != TokenType::Identifier {
                return true;
            }

            if self.peek_keyword_at(2) == Some(Keyword::As) {
                return self.peek_token_type_at(3) == TokenType::Identifier;
            }

            return false;
        }

        true
    }

    /// Eat a dependency item name and return its value and byte range.
    fn eat_dependency_item_name_with_range(&mut self) -> ParserResult<(Name, ByteRange)> {
        if self.peek_is(TokenType::Identifier) {
            let (name, range) = self.eat_identifier_with_range()?;
            return Ok((Name::Identifier(name), range));
        }

        if self.peek_string_literal_start() {
            let (name, range) = self.eat_string_literal_with_range()?;
            return Ok((Name::String(name), range));
        }

        Err(ParserError::expected(
            self.peek_token().range(),
            TokenType::Identifier,
        ))
    }

    /// Eat a dependency alias and return its string ID and byte range.
    fn eat_dependency_item_alias_with_range(
        &mut self,
        allow_literal_alias: bool,
    ) -> ParserResult<(StringId, ByteRange)> {
        // identifier aliases are always valid
        if self.peek_is(TokenType::Identifier) {
            return self.eat_identifier_with_range();
        }

        // export specifiers also allow string literal aliases
        if allow_literal_alias && self.peek_string_literal_start() {
            return self.eat_string_literal_with_range();
        }

        // export specifiers also allow keyword like literal aliases: true, false
        let token = self.peek_token_span().token;
        let has_boolean_literal_alias = allow_literal_alias
            && token.ty() == TokenType::Literal
            && matches!(token.literal(), Some(TokenLiteral::Boolean { .. }));
        if has_boolean_literal_alias {
            let range = self.peek_token().range();
            let alias = self.range_str(range).to_string();
            let alias = self.strings.intern(&alias);
            self.bump();
            return Ok((alias, range));
        }

        // all other forms are invalid aliases
        Err(ParserError::expected(
            self.peek_token().range(),
            TokenType::Identifier,
        ))
    }
}
