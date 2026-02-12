use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

use destack_ast::{
    Argument, Declaration, DeclarationDescriptor, DependencyItem, DependencyKind, DependencyMode,
    Expression, ImportAliasTarget, ImportSource, ImportTarget, Keyword, LocalNodeId, Name,
    TokenType,
};
use destack_base::StringId;
use destack_source::NodeSpanType;

impl Parser {
    /// Eat a dynamic import call expression (`import("foo")`).
    pub fn eat_import_call_expression(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // keyword
        self.eat_keyword(Keyword::Import)?;

        // open call
        self.eat_token(TokenType::OpenParenthesis)?;
        self.eat_newlines_maybe()?;

        // dynamic imports require at least one argument
        if self.peek_is(TokenType::CloseParenthesis) {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // parse the first argument as the import target
        let target_expression = self.eat_expression(
            self.options
                .nested()
                .not_in_position()
                .not_in_sequence_expression(),
        )?;

        // keep static string targets as interned strings
        let target = match self.tree.get(target_expression) {
            Expression::ScalarLiteral(destack_ast::ScalarLiteral::String(target)) => {
                ImportTarget::String(*target)
            }
            _ => ImportTarget::Expression {
                target: target_expression,
            },
        };

        // parse optional import attributes argument(s)
        self.eat_newlines_maybe()?;
        let arguments = if self.is_item_stop() {
            self.eat_item_stop_with_newlines()?;
            if self.peek_is(TokenType::CloseParenthesis) {
                Some(vec![])
            } else {
                let arguments = self.with_options(self.options.nested(), |parser| {
                    parser.eat_positional_arguments_body(TokenType::CloseParenthesis)
                })?;
                Some(arguments)
            }
        } else {
            None
        };
        self.eat_newlines_maybe()?;
        self.eat_token(TokenType::CloseParenthesis)?;

        // import
        let import_id = self.tree.insert(
            Expression::Import {
                source: ImportSource::ImportCall,
                kind: DependencyKind::Value,
                target,
                items: vec![],
                arguments,
            },
            self.get_span_from(start),
        );

        // set main span to the first argument
        let target_span = self.tree.get_span(target_expression);
        self.tree.set_main_span(import_id, target_span);

        Ok(import_id)
    }

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
    /// import A = B.C
    /// import a = require("a")
    /// ```
    pub fn eat_import(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let _timing = self.timing_scope(tags::PARSE_IMPORT);
        let start = self.mark_span();

        // keyword
        self.eat_keyword(Keyword::Import)?;

        // skip newlines before a type modifier
        if self.peek_is(TokenType::Newline) && self.is_keyword_after_newlines(Keyword::Type) {
            self.eat_newlines_maybe()?;
        }

        // kind
        let kind = if self.should_parse_import_type_modifier() {
            self.bump(); // eat type
            self.eat_newlines_maybe()?;
            Some(DependencyKind::Type)
        } else {
            None
        };

        // import equals: `import A = B.C` or `import a = require("a")`
        if self.peek_is(TokenType::Identifier) && self.peek_next_is(TokenType::Assign) {
            let (name, name_span) = self.eat_import_equals_name_with_span()?;
            self.eat_token(TokenType::Assign)?;

            // require import equals
            if let Some((target, _target_span)) = self.try_eat_import_equals_require_target()? {
                let target = ImportAliasTarget::Require { target };
                let descriptor = DeclarationDescriptor::default();
                let expression_id =
                    self.build_import_alias(&start, descriptor, kind, name, name_span, target);
                return Ok(expression_id);
            }

            let value = if kind == Some(DependencyKind::Type) {
                self.with_options(self.options.not_in_position().in_type(), |parser| {
                    parser.eat_expression(parser.options)
                })?
            } else {
                self.with_options(self.options.not_in_position(), |parser| {
                    parser.eat_expression(parser.options)
                })?
            };
            let descriptor = DeclarationDescriptor::default();
            let target = ImportAliasTarget::Path { value };
            let expression_id =
                self.build_import_alias(&start, descriptor, kind, name, name_span, target);
            return Ok(expression_id);
        }

        // binding
        let mut has_binding = false;
        let items = if self.peek_dependency_binding_is() {
            has_binding = true;
            let allow_type_modifier = kind != Some(DependencyKind::Type);
            self.eat_dependency_items_block(allow_type_modifier, false)?
        } else {
            vec![]
        };

        if has_binding {
            self.eat_keyword(Keyword::From)?;
        }
        let (target, target_span) = self.eat_dependency_target_with_span()?;

        // arguments
        let arguments = self.eat_dependency_arguments_maybe()?;

        // import
        let import_id = self.tree.insert(
            Expression::Import {
                source: ImportSource::ImportStatement,
                kind: kind.unwrap_or(DependencyKind::Value),
                target: ImportTarget::String(target),
                items,
                arguments,
            },
            self.get_span_from(&start),
        );

        // set main span to the import target string
        self.tree.set_main_span(import_id, target_span);

        Ok(import_id)
    }

    /// Check whether the tokens after the current `import` keyword form an import equals clause.
    pub(crate) fn peek_import_equals_after_import(&mut self) -> bool {
        let mut pos = self.pos_index() + 1;
        pos = self.next_non_newline_index_from(pos);

        // skip optional type modifier
        if self.keyword_for_index(pos) == Some(Keyword::Type) {
            pos = self.next_non_newline_index_from(pos + 1);
        }

        // require `name =`
        self.token_ref_at(pos)
            .is_some_and(|token| token.token.ty == TokenType::Identifier)
            && {
                let after = self.next_non_newline_index_from(pos + 1);
                self.token_ref_at(after)
                    .is_some_and(|token| token.token.ty == TokenType::Assign)
            }
    }

    /// Decide whether `type` after `import` is a type-only modifier.
    fn should_parse_import_type_modifier(&mut self) -> bool {
        if !self.is_keyword(Keyword::Type) {
            return false;
        }

        // examine the token after type
        let mut pos = self.pos_index() + 1;
        pos = self.next_non_newline_index_from(pos);
        let token = self.token_ref_at(pos);

        // binding forms like `import type { ... }` or `import type * as`
        if matches!(
            token,
            Some(token)
                if token.token.ty == TokenType::OpenBrace
                    || token.token.ty == TokenType::Multiply
        ) {
            return true;
        }

        // identifier bindings like `import type A = B.C`
        if token.is_some_and(|token| token.token.ty == TokenType::Identifier) {
            if self.keyword_for_index(pos) == Some(Keyword::From) {
                let mut after_from = pos + 1;
                after_from = self.next_non_newline_index_from(after_from);
                if self
                    .token_ref_at(after_from)
                    .is_some_and(|token| token.token.ty == TokenType::Assign)
                    || self.keyword_for_index(after_from) == Some(Keyword::From)
                {
                    return true;
                }
                return false;
            }
            return true;
        }

        false
    }

    /// Eat an import equals binding name and return its span.
    fn eat_import_equals_name_with_span(
        &mut self,
    ) -> ParseResult<(StringId, destack_source::Span)> {
        // identifier alias
        if self.peek_is(TokenType::Identifier) {
            return self.eat_identifier_with_span();
        }

        // unexpected token
        Err(ParseError::expected(
            self.peek()?.span,
            TokenType::Identifier,
        ))
    }

    /// Eat `require("a")` and return its target.
    fn try_eat_import_equals_require_target(
        &mut self,
    ) -> ParseResult<Option<(StringId, destack_source::Span)>> {
        if !(self.peek_identifier_str_is("require")
            && self.peek_next_is(TokenType::OpenParenthesis)
            && self.peek_next_next_is(TokenType::Literal)
            && self.peek_next_next_next_is(TokenType::CloseParenthesis))
        {
            return Ok(None);
        }

        self.bump(); // eat require
        self.bump(); // eat (
        let (target, target_span) = self.eat_string_literal_with_span()?;
        self.eat_token(TokenType::CloseParenthesis)?;
        Ok(Some((target, target_span)))
    }

    /// Eat `export import Foo = Bar.Baz` as an exported import alias.
    pub fn eat_export_import_equals(
        &mut self,
        start: &ParserMark,
        mut descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Expression>> {
        // import keyword
        self.eat_keyword(Keyword::Import)?;
        self.eat_newlines_maybe()?;

        // kind
        let kind = if self.is_keyword(Keyword::Type) {
            self.bump(); // eat type
            self.eat_newlines_maybe()?;
            Some(DependencyKind::Type)
        } else {
            None
        };

        // name and assignment
        let (name, name_span) = self.eat_import_equals_name_with_span()?;
        self.eat_token(TokenType::Assign)?;

        // require import equals
        if let Some((target, _target_span)) = self.try_eat_import_equals_require_target()? {
            // normalize descriptor export mode
            if descriptor.export.is_none() {
                descriptor.export = Some(DependencyMode::Item);
            }

            let target = ImportAliasTarget::Require { target };
            let expression_id =
                self.build_import_alias(start, descriptor, kind, name, name_span, target);
            return Ok(expression_id);
        }

        // value expression
        let value = if kind == Some(DependencyKind::Type) {
            self.with_options(self.options.not_in_position().in_type(), |parser| {
                parser.eat_expression(parser.options)
            })?
        } else {
            self.with_options(self.options.not_in_position(), |parser| {
                parser.eat_expression(parser.options)
            })?
        };
        // normalize descriptor export mode
        if descriptor.export.is_none() {
            descriptor.export = Some(DependencyMode::Item);
        }

        // build the exported import alias
        let target = ImportAliasTarget::Path { value };
        let expression_id =
            self.build_import_alias(start, descriptor, kind, name, name_span, target);
        Ok(expression_id)
    }

    /// Build an import alias declaration.
    fn build_import_alias(
        &mut self,
        start: &ParserMark,
        mut descriptor: DeclarationDescriptor,
        kind: Option<DependencyKind>,
        name: StringId,
        name_span: destack_source::Span,
        target: ImportAliasTarget,
    ) -> LocalNodeId<Expression> {
        descriptor.name = Some(Name::Identifier(name));
        let declaration = Declaration::ImportAlias {
            descriptor,
            kind: kind.unwrap_or(DependencyKind::Value),
            target,
        };
        let declaration_id = self.tree.insert(declaration, self.get_span_from(start));
        self.tree.set_main_span(declaration_id, name_span);
        let expression = Expression::Declaration(declaration_id);
        self.tree.insert(expression, self.get_span_from(start))
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
    /// export = foo
    /// ```
    pub fn eat_export(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        let start = self.mark_span();

        self.eat_keyword(Keyword::Export)?;

        // export default <expression>
        if self.is_keyword(Keyword::Default) {
            self.bump(); // eat default

            // reject export default enum declarations
            if self.is_keyword(Keyword::Enum) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let value =
                self.eat_expression(self.options.not_in_position().not_in_sequence_expression())?;
            let item = self.tree.insert(
                DependencyItem {
                    mode: DependencyMode::Default,
                    kind: Some(DependencyKind::Value),
                    name: None,
                    alias: None,
                    value: Some(value),
                },
                self.get_span_from(&start),
            );
            let export = self.tree.insert(
                Expression::Export {
                    kind: DependencyKind::Value,
                    target: None,
                    items: vec![item],
                    arguments: None,
                },
                self.get_span_from(&start),
            );
            return Ok(export);
        }
        // export as namespace Foo
        else if self.is_keyword(Keyword::As) && self.is_next_keyword(Keyword::Namespace) {
            self.bump(); // eat as
            self.bump(); // eat namespace
            let (name, name_span) = self.eat_identifier_with_span()?;
            let export_id = self.tree.insert(
                Expression::ExportNamespace { name },
                self.get_span_from(&start),
            );
            self.tree.set_main_span(export_id, name_span);
            return Ok(export_id);
        }
        // export =
        else if self.peek_is(TokenType::Assign) {
            self.bump(); // eat assign
            let value =
                self.eat_expression(self.options.not_in_position().not_in_sequence_expression())?;
            let item = self.tree.insert(
                DependencyItem {
                    mode: DependencyMode::Namespace,
                    kind: Some(DependencyKind::Value),
                    name: None,
                    alias: None,
                    value: Some(value),
                },
                self.get_span_from(&start),
            );
            let export = self.tree.insert(
                Expression::Export {
                    kind: DependencyKind::Value,
                    target: None,
                    items: vec![item],
                    arguments: None,
                },
                self.get_span_from(&start),
            );
            return Ok(export);
        }

        // kind
        let kind = if self.is_keyword(Keyword::Type) {
            self.bump(); // eat type
            Some(DependencyKind::Type)
        } else {
            None
        };

        // export * from
        if self.peek_is(TokenType::Multiply) && self.is_next_keyword(Keyword::From) {
            self.bump(); // eat *
            self.bump(); // eat from
            let (target, target_span) = self.eat_dependency_target_with_span()?;
            let arguments = self.eat_dependency_arguments_maybe()?;
            let item = DependencyItem {
                mode: DependencyMode::Namespace,
                kind: None,
                name: None,
                alias: None,
                value: None,
            };
            let item_id = self.tree.insert(item, self.get_span_from(&start));
            let export = self.tree.insert(
                Expression::Export {
                    kind: kind.unwrap_or(DependencyKind::Value),
                    target: Some(target),
                    items: vec![item_id],
                    arguments,
                },
                self.get_span_from(&start),
            );

            // set main span to the export target string
            self.tree.set_main_span(export, target_span);

            return Ok(export);
        }

        // require a binding after export and optional type modifier
        if self.peek_dependency_binding().is_err() {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        // binding
        let allow_type_modifier = kind != Some(DependencyKind::Type);
        let items = self.eat_dependency_items_block(allow_type_modifier, true)?;
        let (target, target_span) = if self.is_keyword(Keyword::From) {
            self.bump(); // eat from
            let (target, span) = self.eat_dependency_target_with_span()?;
            (Some(target), Some(span))
        } else {
            (None, None)
        };

        // assertions or attributes (parsed for conformance)
        let arguments = if target.is_some() {
            self.eat_dependency_arguments_maybe()?
        } else {
            None
        };

        // `export { default }` without `from` is invalid
        // (default is a reserved word and can't be a local binding)
        if target.is_none() {
            for item_id in &items {
                let item = self.tree.get(*item_id);
                if item.mode == DependencyMode::Default && item.name.is_none() {
                    let span = self
                        .tree
                        .get_main_span(*item_id)
                        .unwrap_or(self.tree.get_span(*item_id));
                    return Err(ParseError::unexpected(span));
                }
            }
        }

        // export
        let export_id = self.tree.insert(
            Expression::Export {
                kind: kind.unwrap_or(DependencyKind::Value),
                target,
                items,
                arguments,
            },
            self.get_span_from(&start),
        );

        // set main span to the export target string if present
        if let Some(target_span) = target_span {
            self.tree.set_main_span(export_id, target_span);
        }

        Ok(export_id)
    }

    /// Eat dependency arguments for import/export assertions or attributes.
    fn eat_dependency_arguments_maybe(
        &mut self,
    ) -> ParseResult<Option<Vec<LocalNodeId<Argument>>>> {
        if self.is_keyword(Keyword::With) || self.is_keyword(Keyword::Assert) {
            self.bump(); // eat with or assert
            self.eat_newlines_maybe()?;
            self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)?;
            let arguments = self.with_options(self.options.nested(), |parser| {
                parser.eat_arguments_body(TokenType::CloseBrace)
            })?;
            self.eat_token(TokenType::CloseBrace)?;
            Ok(Some(arguments))
        } else {
            Ok(None)
        }
    }

    /// Peek a dependency binding.
    pub(crate) fn peek_dependency_binding(&mut self) -> ParseResult<()> {
        if self.peek_dependency_binding_is() {
            Ok(())
        } else {
            Err(ParseError::unexpected(self.peek()?.span))
        }
    }

    /// Return true when the next tokens can start a dependency binding.
    #[inline]
    pub(crate) fn peek_dependency_binding_is(&mut self) -> bool {
        self.peek_is(TokenType::OpenBrace)
            || self.peek_is(TokenType::Multiply)
            || (self.peek_is(TokenType::Identifier)
                && (self.peek_next_is(TokenType::Comma) || self.is_next_keyword(Keyword::From)))
    }

    /// Return true when tokens after `import` can start an import statement.
    pub(crate) fn can_start_import_statement(&mut self) -> bool {
        let after_import = self.pos().saturating_add(1);
        self.is_token_after_newlines(after_import, TokenType::Identifier)
            || self.is_token_after_newlines(after_import, TokenType::OpenBrace)
            || self.is_token_after_newlines(after_import, TokenType::Multiply)
            || self.is_token_after_newlines(after_import, TokenType::Literal)
    }

    /// Eat an dependency target and return both the string and its span.
    ///
    /// Examples:
    /// ```
    /// "foo"
    /// "foo/bar:something"
    /// ```
    fn eat_dependency_target_with_span(&mut self) -> ParseResult<(StringId, destack_source::Span)> {
        let (string_id, span) = self.eat_string_literal_with_span()?;
        Ok((string_id, span))
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
    ) -> ParseResult<Vec<LocalNodeId<DependencyItem>>> {
        let mut items: Vec<LocalNodeId<DependencyItem>> = Vec::new();

        // `Default,` or `foo from`
        if self.peek_is(TokenType::Identifier)
            && (self.peek_next_is(TokenType::Comma) || self.is_next_keyword(Keyword::From))
        {
            let start = self.mark_span();
            let (alias, alias_span) = self.eat_identifier_with_span()?;
            if self.peek_is(TokenType::Comma) {
                self.bump(); // eat comma (leave from)
            }
            let item = DependencyItem {
                mode: DependencyMode::Default,
                kind: None,
                name: None,
                alias: Some(alias),
                value: None,
            };
            let item_id = self.tree.insert(item, self.get_span_from(&start));
            self.tree.set_main_span(item_id, alias_span);
            items.push(item_id);
        }

        // `* as foo` (can follow a default import)
        if self.peek_is(TokenType::Multiply) && self.is_next_keyword(Keyword::As) {
            let start = self.mark_span();
            self.bump(); // eat *
            self.bump(); // eat as
            let (alias, alias_span) = if self.peek_is(TokenType::Literal) {
                self.eat_string_literal_with_span()?
            } else {
                self.eat_identifier_with_span()?
            };
            let item = DependencyItem {
                mode: DependencyMode::Namespace,
                kind: None,
                name: None,
                alias: Some(alias),
                value: None,
            };
            let item_id = self.tree.insert(item, self.get_span_from(&start));
            self.tree.set_main_span(item_id, alias_span);
            items.push(item_id);
        }

        // main items
        if items.is_empty() || self.peek_is(TokenType::OpenBrace) {
            self.eat_token(TokenType::OpenBrace)?;
            self.eat_newlines_maybe()?;
            while !self.peek_is(TokenType::CloseBrace) {
                let item = self.eat_dependency_item(allow_type_modifier, allow_literal_alias)?;
                items.push(item);

                self.eat_newlines_maybe()?;
                if self.peek_is(TokenType::CloseBrace) {
                    break;
                }
                if self.peek_comma_is() {
                    self.eat_item_stop_with_newlines()?;
                    continue;
                } else {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
            }
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseBrace)?;
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
    ) -> ParseResult<LocalNodeId<DependencyItem>> {
        let start = self.mark_span();

        // kind
        let kind = if self.should_parse_dependency_type_modifier() {
            if !allow_type_modifier {
                let span = self.peek()?.span;
                return Err(ParseError::unexpected(span));
            }
            self.bump(); // eat type
            Some(DependencyKind::Type)
        } else {
            None
        };

        // default
        if self.is_keyword(Keyword::Default) {
            self.bump(); // eat default

            // alias
            let (alias, alias_span) =
                if self.is_keyword(Keyword::As) || self.peek_is(TokenType::Colon) {
                    self.bump(); // eat `as` or `:`
                    let (alias, alias_span) =
                        self.eat_dependency_item_alias_with_span(allow_literal_alias)?;
                    (Some(alias), Some(alias_span))
                } else {
                    (None, None)
                };

            // item
            let item = self.tree.insert(
                DependencyItem {
                    mode: DependencyMode::Default,
                    kind,
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
            let (alias, alias_span) =
                if self.is_keyword(Keyword::As) || self.peek_is(TokenType::Colon) {
                    self.bump(); // eat `as` or `:`
                    let (alias, alias_span) =
                        self.eat_dependency_item_alias_with_span(allow_literal_alias)?;
                    (Some(alias), Some(alias_span))
                } else {
                    (None, None)
                };

            // item
            let item = self.tree.insert(
                DependencyItem {
                    mode: DependencyMode::Item,
                    kind,
                    name: Some(name),
                    alias,
                    value: None,
                },
                self.get_span_from(&start),
            );
            self.tree.set_side_span(item, NodeSpanType::Type, name_span);
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
        if !self.peek_next_is(TokenType::Identifier) && !self.peek_next_is(TokenType::Literal) {
            return false;
        }

        // handle `type as` disambiguation
        if self.is_next_keyword(Keyword::As) {
            if !self.peek_next_next_is(TokenType::Identifier) {
                return true;
            }

            if self.is_next_next_keyword(Keyword::As) {
                return self.peek_next_next_next_is(TokenType::Identifier);
            }

            return false;
        }

        true
    }

    /// Eat a dependency item name (identifier or string literal) and its span.
    fn eat_dependency_item_name_with_span(&mut self) -> ParseResult<(Name, destack_source::Span)> {
        if self.peek_is(TokenType::Identifier) {
            let (name, span) = self.eat_identifier_with_span()?;
            return Ok((Name::Identifier(name), span));
        }

        if self.peek_string_literal_is() {
            let (name, span) = self.eat_string_literal_with_span()?;
            return Ok((Name::String(name), span));
        }

        Err(ParseError::expected(
            self.peek()?.span,
            TokenType::Identifier,
        ))
    }

    /// Eat a dependency alias and return its interned string and span.
    fn eat_dependency_item_alias_with_span(
        &mut self,
        allow_literal_alias: bool,
    ) -> ParseResult<(StringId, destack_source::Span)> {
        // identifier aliases are always valid
        if self.peek_is(TokenType::Identifier) {
            return self.eat_identifier_with_span();
        }

        // export specifiers also allow string literal aliases
        if allow_literal_alias && self.peek_string_literal_is() {
            return self.eat_string_literal_with_span();
        }

        // all other forms are invalid aliases
        Err(ParseError::expected(
            self.peek()?.span,
            TokenType::Identifier,
        ))
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        Argument, Declaration, DependencyItem, DependencyKind, DependencyMode, Expression,
        ImportAliasTarget, ImportSource, ImportTarget, Name, ScalarLiteral,
    };
    use destack_source::LanguageType;

    use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

    fn assert_import_target_string(parser: &crate::Parser, target: &ImportTarget, expected: &str) {
        assert_node!(target, ImportTarget::String(target) => {
            assert_string!(parser, *target, expected);
        });
    }

    #[test]
    fn test_parse_import_simple() {
        // import destack
        let mut test = TestParser::new("import \"destack\"");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import
        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 0);
            assert_import_target_string(&parser, target, "destack");
        });
    }

    #[test]
    fn test_parse_import_from_expression() {
        let mut test = TestParser::new("import os from 'os'");
        let mut parser = test.prepare();
        let expression_id = parser.eat_expression(parser.options).unwrap();

        // import os from 'os'
        assert_node!(parser.tree, expression_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias),.. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "os");
            });
            assert_import_target_string(&parser, target, "os");
        });
    }

    #[test]
    fn test_parse_import_path_with_arguments() {
        let mut test = TestParser::new("import \"destack.geometry\" with { bar: true }");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import destack.geometry with { bar: true }
        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, arguments, .. } => {
            // destack.geometry
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 0);
            assert_import_target_string(&parser, target, "destack.geometry");
            // with { bar: true }
            let arguments = arguments.as_ref().expect("expected arguments");
            assert_eq!(arguments.len(), 1);
            assert_node!(parser.tree, arguments[0], Argument::Named { modifiers: _, name, value } => {
                assert_string!(parser, name.string(), "bar");
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Boolean(true)));
            });
        });
    }

    #[test]
    fn test_parse_import_with_prefix_items() {
        let mut test = TestParser::new("import { Vector2, Vector3 as V3 } from \"ds.geometry\"");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name: Some(name), alias, .. } => {
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "Vector2");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem { kind, name: Some(name), alias: Some(alias),.. } => {
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "Vector3");
                assert_string!(parser, *alias, "V3");
            });
            assert_import_target_string(&parser, target, "ds.geometry");
        });
    }

    #[test]
    fn test_parse_import_as_alias() {
        let mut test = TestParser::new(r#"import * as geom from "ds/geometry""#);
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import * as geom from ds.geometry
        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_string!(parser, *alias, "geom");
            });
            assert_import_target_string(&parser, target, "ds/geometry");
        });
    }

    #[test]
    fn test_parse_import_with_type() {
        let mut test = TestParser::new(
            "
import {
  StructuredObject,
  type StructuredObjectOptions,
} from './lib/object.ng';
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let import_id = parser.eat_import().unwrap();
        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_import_target_string(&parser, target, "./lib/object.ng");

            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name: Some(name), alias,.. } => {
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "StructuredObject");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem { kind, name: Some(name), alias,.. } => {
                assert_eq!(*kind, Some(DependencyKind::Type));
                assert_string!(parser, name.string(), "StructuredObjectOptions");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_import_with_default_and_block() {
        let mut test = TestParser::new("import Default, { type Item } from 'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 2);
            // Default
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind: None, name: None, alias: Some(alias),.. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "Default");
            });
            // { type Item }
            assert_node!(parser.tree, items[1], DependencyItem { mode, kind, name: Some(name), alias: None,.. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, Some(DependencyKind::Type));
                assert_string!(parser, name.string(), "Item");
            });
            // `foo`
            assert_import_target_string(&parser, target, "foo");
        });
    }

    #[test]
    fn test_parse_import_type_identifier_name() {
        // treat type as a value name in named imports
        let mut test = TestParser::new("import { type } from 'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { items, .. } => {
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name: Some(name), alias, .. } => {
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "type");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_import_type_as_default_name() {
        let mut test = TestParser::new("import type from './a'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
            assert_eq!(items.len(), 1);
            assert_import_target_string(&parser, target, "./a");
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "type");
            });
        });
    }

    #[test]
    fn test_parse_import_type_empty_block() {
        let mut test = TestParser::new("import type {} from 'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
            assert!(items.is_empty());
            assert_import_target_string(&parser, target, "foo");
        });
    }

    #[test]
    fn test_parse_import_type_string_specifier() {
        let mut test = TestParser::new(r#"import { type "string" as foo } from "foo""#);
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
            assert_eq!(items.len(), 1);
            assert_import_target_string(&parser, target, "foo");
            assert_node!(parser.tree, items[0], DependencyItem { kind, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*kind, Some(DependencyKind::Type));
                assert!(matches!(name, Name::String(_)));
                assert_string!(parser, name.string(), "string");
                assert_string!(parser, *alias, "foo");
            });
        });
    }

    #[test]
    fn test_parse_import_type_as_value_alias() {
        // treat type as a value name when followed by as as
        let mut test = TestParser::new("import { type as as } from 'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { items, .. } => {
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "type");
                assert_string!(parser, *alias, "as");
            });
        });
    }

    #[test]
    fn test_parse_import_type_only_named_as() {
        // treat type as a modifier when followed by as then close brace
        let mut test = TestParser::new("import { type as } from 'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { items, .. } => {
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name: Some(name), alias, .. } => {
                assert_eq!(*kind, Some(DependencyKind::Type));
                assert_string!(parser, name.string(), "as");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_import_type_in_import_type_error() {
        // reject type modifiers inside import type blocks
        let mut test = TestParser::new("import type { type Foo } from 'foo'");
        let mut parser = test.prepare();
        assert!(parser.eat_import().is_err());
    }

    #[test]
    fn test_parse_export_type_identifier_name() {
        // treat type as a value name in named exports
        let mut test = TestParser::new("export { type } from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        assert_node!(parser.tree, export_id, Expression::Export { items, .. } => {
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { kind, name: Some(name), alias, .. } => {
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "type");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_import_default_and_namespace() {
        // combined default import + namespace import
        let mut test = TestParser::new(r#"import a, * as b from "foo""#);
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 2);
            // default: a
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind: None, name: None, alias: Some(alias),.. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "a");
            });
            // namespace: * as b
            assert_node!(parser.tree, items[1], DependencyItem { mode, kind: None, name: None, alias: Some(alias),.. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_string!(parser, *alias, "b");
            });
            assert_import_target_string(&parser, target, "foo");
        });
    }

    #[test]
    fn test_parse_import_equals_namespace() {
        let mut test = TestParser::new("import A = B.C");
        let mut parser = test.prepare();
        let expression_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
                assert_eq!(*kind, DependencyKind::Value);
                let name = descriptor.name.expect("import alias name");
                assert_string!(parser, name.string(), "A");
                assert!(matches!(target, ImportAliasTarget::Path { .. }));
                let ImportAliasTarget::Path { value } = target else {
                    unreachable!("expected import alias path");
                };
                assert_expression_path!(parser, parser.tree.get(*value), "B.C");
            });
        });
    }

    #[test]
    fn test_parse_import_equals_require() {
        let mut test = TestParser::new(r#"import a = require("a")"#);
        let mut parser = test.prepare();
        let expression_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
                assert_eq!(*kind, DependencyKind::Value);
                let name = descriptor.name.expect("import alias name");
                assert_string!(parser, name.string(), "a");
                assert!(matches!(target, ImportAliasTarget::Require { .. }));
                let ImportAliasTarget::Require { target } = target else {
                    unreachable!("expected import alias require");
                };
                assert_string!(parser, *target, "a");
            });
        });
    }

    #[test]
    fn test_parse_import_type_equals_require() {
        let mut test = TestParser::new(r#"import type MyType = require("pkg")"#);
        let mut parser = test.prepare();
        let expression_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
                assert_eq!(*kind, DependencyKind::Type);
                let name = descriptor.name.expect("import alias name");
                assert_string!(parser, name.string(), "MyType");
                assert!(matches!(target, ImportAliasTarget::Require { .. }));
                let ImportAliasTarget::Require { target } = target else {
                    unreachable!("expected import alias require");
                };
                assert_string!(parser, *target, "pkg");
            });
        });
    }

    #[test]
    fn test_parse_import_type_equals_require_with_newlines() {
        let mut test = TestParser::new_with_options(
            "import type\nMyType = require(\"pkg\")",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
                assert_eq!(*kind, DependencyKind::Type);
                let name = descriptor.name.expect("import alias name");
                assert_string!(parser, name.string(), "MyType");
                assert!(matches!(target, ImportAliasTarget::Require { .. }));
                let ImportAliasTarget::Require { target } = target else {
                    unreachable!("expected import alias require");
                };
                assert_string!(parser, *target, "pkg");
            });
        });
    }

    #[test]
    fn test_parse_import_type_equals_path() {
        let mut test = TestParser::new_with_options(
            r#"import type Alias = Namespace.Value"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expression_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
                assert_eq!(*kind, DependencyKind::Type);
                let name = descriptor.name.expect("import alias name");
                assert_string!(parser, name.string(), "Alias");
                assert!(matches!(target, ImportAliasTarget::Path { .. }));
                let ImportAliasTarget::Path { value } = target else {
                    unreachable!("expected import alias path");
                };
                assert_expression_path!(parser, parser.tree.get(*value), "Namespace.Value");
            });
        });
    }

    #[test]
    fn test_parse_import_type_equals_identifier() {
        let mut test =
            TestParser::new_with_options(r#"import type Alias = Value"#, LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expression_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
                assert_eq!(*kind, DependencyKind::Type);
                let name = descriptor.name.expect("import alias name");
                assert_string!(parser, name.string(), "Alias");
                assert!(matches!(target, ImportAliasTarget::Path { .. }));
                let ImportAliasTarget::Path { value } = target else {
                    unreachable!("expected import alias path");
                };
                assert_expression_path!(parser, parser.tree.get(*value), "Value");
            });
        });
    }

    #[test]
    fn test_parse_import_type_modifier_equals_require() {
        let mut test = TestParser::new(r#"import type React = require("pkg")"#);
        let mut parser = test.prepare();
        let expression_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias { descriptor, kind, target } => {
                assert_eq!(*kind, DependencyKind::Type);
                let name = descriptor.name.expect("import alias name");
                assert_string!(parser, name.string(), "React");
                assert!(matches!(target, ImportAliasTarget::Require { .. }));
                let ImportAliasTarget::Require { target } = target else {
                    unreachable!("expected import alias require");
                };
                assert_string!(parser, *target, "pkg");
            });
        });
    }

    #[test]
    fn test_parse_export_with_block() {
        let mut test = TestParser::new(
            "
export type { CreateUIMessage, UIMessage }
",
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Type);
            assert!(target.is_none());
            assert_eq!(items.len(), 2);
            // CreateUIMessage
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind, name: Some(name), alias,.. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "CreateUIMessage");
                assert!(alias.is_none());
            });
            // UIMessage
            assert_node!(parser.tree, items[1], DependencyItem { mode, kind, name: Some(name), alias,.. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "UIMessage");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_with_namespace() {
        let mut test = TestParser::new("export * from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: None, .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
            });
            assert_string!(parser, *target, "foo");
        });
    }

    #[test]
    fn test_parse_export_type_with_namespace() {
        let mut test = TestParser::new("export type * from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Type);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: None, .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
            });
            assert_string!(parser, *target, "foo");
        });
    }

    #[test]
    fn test_parse_export_type_namespace_string_alias() {
        let mut test = TestParser::new(r#"export type * as "ns2" from 'foo'"#);
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Type);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_string!(parser, *alias, "ns2");
            });
            assert_string!(parser, *target, "foo");
        });
    }

    #[test]
    fn test_parse_export_type_item_reexport() {
        let mut test = TestParser::new("export { type Options } from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind, name: Some(name), alias: None, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, Some(DependencyKind::Type));
                assert_string!(parser, name.string(), "Options");
            });
            assert_string!(parser, *target, "foo");
        });
    }

    #[test]
    fn test_parse_export_type_reexport_block() {
        let mut test = TestParser::new("export type { Options } from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Type);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind, name: Some(name), alias: None, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "Options");
            });
            assert_string!(parser, *target, "foo");
        });
    }

    #[test]
    fn test_parse_export_with_namespace_alias() {
        let mut test = TestParser::new("export * as foo from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_string!(parser, *alias, "foo");
            });
            assert_string!(parser, *target, "foo");
        });
    }

    #[test]
    fn test_parse_export_as_namespace() {
        let mut test = TestParser::new_with_options(
            "export as namespace Foo",
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::ExportNamespace { name } => {
            assert_string!(parser, *name, "Foo");
        });
    }

    #[test]
    fn test_parse_export_with_module_export() {
        let mut test = TestParser::new("export = foo");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: None, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: None, value: Some(value), .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_expression_path!(parser, parser.tree.get(*value), "foo");
            });
        });
    }

    #[test]
    fn test_parse_export_default_from_item() {
        let mut test = TestParser::new("export default foo");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: None, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: None, value: Some(value), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_expression_path!(parser, parser.tree.get(*value), "foo");
            });
        });
    }

    #[test]
    fn test_parse_export_default_from_target() {
        let mut test = TestParser::new("export { default } from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias, .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_default_from_target_with_alias_and_items() {
        let mut test = TestParser::new("export { default as bar, baz as baz, biz } from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert_eq!(items.len(), 3);
            // default as bar
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "bar");
            });
            // baz as baz
            assert_node!(parser.tree, items[1], DependencyItem { mode, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, name.string(), "baz");
                assert_string!(parser, *alias, "baz");
            });
            // biz
            assert_node!(parser.tree, items[2], DependencyItem { mode, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, name.string(), "biz");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_default_with_multiple_aliases() {
        let mut test = TestParser::new("export { default as bar, default as baz } from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_string!(parser, *target, "foo");
            assert_eq!(items.len(), 2);
            // default as bar
            assert_node!(parser.tree, items[0], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "bar");
            });
            // default as baz
            assert_node!(parser.tree, items[1], DependencyItem { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "baz");
            });
        });
    }

    #[test]
    fn test_reject_export_type_without_binding() {
        // source: export type
        let source = "export type";
        let mut test = TestParser::new_with_options("export type", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let error = parser.eat_export().unwrap_err();

        // eof
        assert_eq!(parser.get_span_str(error.leaf_span()), "");
        assert_eq!(error.leaf_span().start, source.len() as u32);
    }

    #[test]
    fn test_reject_export_default_enum() {
        // source: export default enum A { X, Y, Z }
        let mut test = TestParser::new("export default enum A { X, Y, Z }");
        let mut parser = test.prepare();
        let error = parser.eat_export().unwrap_err();

        // enum
        assert_eq!(parser.get_span_str(error.leaf_span()), "enum");
    }

    #[test]
    fn test_parse_export_keyword_name_without_target() {
        // source: export { if }
        let mut test = TestParser::new("export { if }");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        assert_node!(parser.tree, export_id, Expression::Export { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(target.is_none());
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "if");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_keyword_alias_without_target() {
        // source: export { if as foo }
        let mut test = TestParser::new("export { if as foo }");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        assert_node!(parser.tree, export_id, Expression::Export { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(target.is_none());
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "if");
                assert_string!(parser, *alias, "foo");
            });
        });
    }

    #[test]
    fn test_parse_export_keyword_string_alias_without_target() {
        // source: export { viteLegacyPluginCjs as 'module.exports' }
        let mut test = TestParser::new("export { viteLegacyPluginCjs as 'module.exports' }");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        assert_node!(parser.tree, export_id, Expression::Export { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(target.is_none());
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "viteLegacyPluginCjs");
                assert_string!(parser, *alias, "module.exports");
            });
        });
    }

    #[test]
    fn test_parse_export_as_identifier_without_target() {
        // source: export { as }
        let mut test = TestParser::new("export { as }");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        assert_node!(parser.tree, export_id, Expression::Export { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(target.is_none());
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "as");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_type_identifier_without_target() {
        // source: export { type }
        let mut test = TestParser::new_with_options("export { type }", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        assert_node!(parser.tree, export_id, Expression::Export { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(target.is_none());
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "type");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_named_type_with_keyword_alias_without_target() {
        // source: export { type as if }
        let mut test =
            TestParser::new_with_options("export { type as if }", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        assert_node!(parser.tree, export_id, Expression::Export { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(target.is_none());
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "type");
                assert_string!(parser, *alias, "if");
            });
        });
    }

    #[test]
    fn test_parse_export_type_only_as_as_keyword_alias_without_target() {
        // source: export { type as as if }
        let mut test =
            TestParser::new_with_options("export { type as as if }", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        assert_node!(parser.tree, export_id, Expression::Export { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(target.is_none());
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem { mode, kind, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, Some(DependencyKind::Type));
                assert_string!(parser, name.string(), "as");
                assert_string!(parser, *alias, "if");
            });
        });
    }

    #[test]
    fn test_reject_export_function_without_name() {
        let mut test = TestParser::new_with_options(
            "export function(option: any): void",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let result = parser.eat_export();
        assert!(result.is_err());
    }
}
