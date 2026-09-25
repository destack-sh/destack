use crate::lex::{InvalidEscape, cook};
use crate::parse::{ExpressionPosition, ExpressionStop};
use crate::{Parser, ParserError, ParserResult};

use tspp_core::StringId;
use tspp_dir::{
    Argument, DependencyBinding, DependencyItem, Expression, ImportAttribute,
    ImportAttributeClause, ImportAttributeClauseKind, ImportAttributeValue, Keyword, LocalNodeId,
    Name, NodeType, Property, TokenLiteral, TokenType,
};
use tspp_source::{ByteRange, NodeSpanList, NodeSpanRegion, NodeSpanType};

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
    /// ```tspp
    /// import "foo"
    /// import "foo.bar"
    /// import * as foo from "foo"
    /// import { bar, baz } from "foo"
    /// import Default, { Item } from "foo"
    /// import foo as baz with { bar: true }
    /// ```
    pub(crate) fn parse_import(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        // keyword
        self.eat_keyword(Keyword::Import)?;

        // reject type-only imports
        if self.peek_import_type_marker() {
            return Err(ParserError::type_dependency(self.peek_token().range()));
        }

        // binding
        let items = if self.peek_dependency_binding() {
            Some(self.parse_dependency_items(false)?)
        } else {
            None
        };

        if items.is_some() {
            self.eat_keyword(Keyword::From)?;
        }

        // allow bare import targets on the next line (`import\n"foo"` and comment separated forms)
        let (target, target_range) = self.eat_dependency_target_with_range()?;

        // arguments
        let import_clause = self.parse_import_clause()?;
        let attributes = import_clause
            .as_ref()
            .map(|import_clause| import_clause.clause.clone());

        // import
        let import_id = self.insert_node(
            Expression::Import {
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

    /// Decide whether `type` after `import` marks a type-only import.
    fn peek_import_type_marker(&self) -> bool {
        if !self.peek_is_keyword(Keyword::Type) {
            return false;
        }

        // clause forms like `import type { ... }` and `import type * as ns`
        if matches!(
            self.peek_token_type_at(1),
            TokenType::OpenBrace | TokenType::Multiply
        ) {
            return true;
        }

        // identifier forms like `import type A from "a"` and `import type A, { B } from "a"`
        self.peek_token_type_at(1) == TokenType::Identifier
            && (self.peek_keyword_at(2) == Some(Keyword::From)
                || self.peek_token_type_at(2) == TokenType::Comma)
    }

    /// Decide whether `type` after `export` marks a type-only export.
    ///
    /// Only clause forms are markers, `export type Foo = ...` declares an alias.
    fn peek_export_type_marker(&self) -> bool {
        self.peek_is_keyword(Keyword::Type)
            && matches!(
                self.peek_token_type_at(1),
                TokenType::OpenBrace | TokenType::Multiply
            )
    }

    /// Parse an export declaration (including the `export` keyword and an optional body).
    ///
    /// Examples:
    /// ```tspp
    /// export "foo"
    /// export * from "foo"
    /// export * as foo from "foo"
    /// export { bar, baz } from "foo"
    /// export { bar as bar, baz }
    /// export default foo
    /// ```
    pub(crate) fn parse_export(&mut self) -> ParserResult<LocalNodeId<Expression>> {
        let start = self.mark_parse_start();

        self.eat_keyword(Keyword::Export)?;

        // export default <expression>
        if self.peek_is_keyword(Keyword::Default) {
            self.bump();

            // reject export default enum declarations
            if self.peek_is_keyword(Keyword::Enum) {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            let value =
                self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?;
            let item = self.insert_node(
                DependencyItem::Binding {
                    binding: DependencyBinding::Default,
                    name: None,
                    alias: None,
                    value: Some(value),
                },
                self.range_since(&start),
            );
            let export = self.insert_node(
                Expression::Export {
                    target: None,
                    items: vec![item],
                    attributes: None,
                },
                self.range_since(&start),
            );
            return Ok(export);
        }

        // reject type-marked exports
        if self.peek_export_type_marker() {
            return Err(ParserError::type_dependency(self.peek_token().range()));
        }

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
            let import_clause = self.parse_import_clause()?;
            let attributes = import_clause
                .as_ref()
                .map(|import_clause| import_clause.clause.clone());
            let item = DependencyItem::Binding {
                binding: DependencyBinding::Namespace,
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

        // require a binding after export
        if !self.peek_dependency_binding() {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        // binding
        let items = self.parse_dependency_items(true)?;

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
            self.parse_import_clause()?
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
            Expression::Literal(value) => ImportAttributeValue::Literal(*value),
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
                    let Property::Field { name, value, .. } = self.tree.get(property_id) else {
                        return Err(ParserError::unexpected(self.tree.get_range(property_id)));
                    };
                    let value = self.decode_import_attribute_value(*value)?;

                    attributes.push(ImportAttribute { key: *name, value });
                }

                ImportAttributeValue::Object(attributes)
            }
            _ => return Err(ParserError::unexpected(self.tree.get_range(expression_id))),
        })
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
    fn parse_import_clause(&mut self) -> ParserResult<Option<ImportClause>> {
        // attribute clause head
        if !self.peek_is_keyword(Keyword::With) {
            return Ok(None);
        }

        let start = self.mark_parse_start();
        self.bump();

        // parse the attribute clause body
        self.eat_token_before(TokenType::OpenBrace, TokenType::CloseBrace)?;
        let mut attributes = Vec::new();
        let mut attribute_ranges = Vec::new();
        while self.has_more_tokens() && !self.peek_is(TokenType::CloseBrace) {
            let attribute_start = self.mark_parse_start();
            let (key, _) = self.eat_name_with_range()?;
            self.eat_token(TokenType::Colon)?;
            let value =
                self.parse_expression(ExpressionPosition::Value, ExpressionStop::default())?;
            let value = self.decode_import_attribute_value(value)?;

            attributes.push(ImportAttribute { key, value });
            attribute_ranges.push(self.range_since(&attribute_start));

            // require a separator or the end of the list
            let has_comma = self.peek_is(TokenType::Comma);
            let has_separator = has_comma || self.peek_is_on_new_line();
            let is_list_end = !self.has_more_tokens() || self.peek_is(TokenType::CloseBrace);
            if !has_separator && !is_list_end {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }

            // consume an explicit separator
            if has_comma {
                self.bump();
            }
        }
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Expression)?;
        let range = self.range_since(&start);

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
    /// ```tspp
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

        let content = cook(self.string_literal_str(token))
            .map_err(|InvalidEscape| ParserError::expected(token, TokenType::Literal))?;
        let string_id = self.strings.intern(&content);
        self.bump();

        Ok((string_id, token.range()))
    }

    /// Parse a dependency items block.
    ///
    /// Examples:
    /// ```tspp
    /// foo
    /// * as foo
    /// Default, { a, b }
    /// { a, b }
    /// ```
    fn parse_dependency_items(
        &mut self,
        allow_literal_alias: bool,
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
            let documentation = self.parse_documentation();
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
                name: None,
                alias: Some(alias),
                value: None,
            };
            let item_id = self.insert_node(item, self.range_since(&start));
            self.tree.set_main_range(item_id, alias_range);
            self.attach_documentation(item_id, documentation);
            items.push(item_id);
        }

        // `* as foo` (can follow a default import)
        if self.peek_is(TokenType::Multiply) && self.peek_next_keyword() == Some(Keyword::As) {
            let documentation = self.parse_documentation();
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
                name: None,
                alias: Some(alias),
                value: None,
            };
            let item_id = self.insert_node(item, self.range_since(&start));
            self.tree.set_main_range(item_id, alias_range);
            self.attach_documentation(item_id, documentation);
            items.push(item_id);
        }

        // main items
        if items.is_empty() || self.peek_is(TokenType::OpenBrace) {
            self.eat_token(TokenType::OpenBrace)?;

            while !self.peek_is(TokenType::CloseBrace) {
                let item_start = self.mark_parse_start();
                let documentation = self.parse_documentation();
                let decorators = self.parse_decorators();

                let item = match self.parse_dependency_item(allow_literal_alias) {
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
                if !matches!(self.tree.get(item), DependencyItem::Error) {
                    self.attach_documentation(item, documentation);
                }
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
    /// ```tspp
    /// geometry
    /// geometry as geom
    /// ```
    pub(crate) fn parse_dependency_item(
        &mut self,
        allow_literal_alias: bool,
    ) -> ParserResult<LocalNodeId<DependencyItem>> {
        let start = self.mark_parse_start();

        // reject type-only dependency items
        if self.peek_dependency_type_marker() {
            return Err(ParserError::type_dependency(self.peek_token().range()));
        }

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

    /// Decide whether `type` marks a type-only dependency item.
    fn peek_dependency_type_marker(&self) -> bool {
        // require `type` keyword
        if !self.peek_is_keyword(Keyword::Type) {
            return false;
        }

        // separate `{ type Foo }` from the `{ type as alias }` binding
        matches!(
            self.peek_token_type_at(1),
            TokenType::Identifier | TokenType::Literal
        ) && self.peek_keyword_at(1) != Some(Keyword::As)
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
