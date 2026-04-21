use crate::parse::expression::common::DeclarationHeader;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser};

use destack_ast::{
    Argument, Declaration, DependencyItem, DependencyKind, DependencyMode, Expression,
    ImportAliasDeclaration, ImportAliasTarget, ImportAttribute, ImportAttributeClause,
    ImportAttributeClauseKind, ImportAttributeValue, ImportSource, ImportTarget, Keyword,
    LiteralType, LocalNodeId, Name, NodeType, Property, ScalarLiteral, TokenType,
};
use destack_core::StringId;
use destack_source::{NodeSpanType, Span};

/// One leading triple slash directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TripleSlashDirective<'a> {
    /// A `/// <reference path="..." />` directive.
    ReferencePath(&'a str),
    /// A `/// <reference types="..." />` directive.
    ReferenceTypes(&'a str),
    /// A `/// <reference lib="..." />` directive.
    ReferenceLib(&'a str),
    /// A `/// <reference no-default-lib="true" />` directive.
    NoDefaultLib(&'a str),
}

impl Parser {
    /// Parse leading triple slash directives as type imports.
    pub(crate) fn parse_leading_triple_slash_reference_imports(
        &mut self,
    ) -> (Vec<LocalNodeId<Expression>>, bool) {
        // triple slash directives only exist in declaration-oriented typed sources
        if !self.language.is_typescript() {
            return (Vec::new(), false);
        }

        let mut imports = Vec::new();
        let text = self.file.text().to_string();
        let bytes = text.as_bytes();
        let mut offset = 0usize;
        let mut in_block_comment = false;

        // scan only the leading trivia and directives section
        while offset < bytes.len() {
            let line_start = offset;
            let mut line_end = line_start;
            while line_end < bytes.len() && bytes[line_end] != b'\n' && bytes[line_end] != b'\r' {
                line_end += 1;
            }

            let line = &text[line_start..line_end];
            let trimmed = line.trim_start();
            let mut consume_line = true;

            // continue an existing block comment
            if in_block_comment {
                if let Some(end_index) = trimmed.find("*/") {
                    in_block_comment = false;
                    let after = trimmed[end_index + 2..].trim_start();
                    if !after.is_empty() {
                        consume_line = false;
                    }
                }
            }
            // skip empty lines and top level shebang before declarations
            else if trimmed.is_empty() || line_start == 0 && trimmed.starts_with("#!") {
                // nothing to do
            }
            // parse triple slash reference directives
            else if let Some(directive) = Self::triple_slash_directive(trimmed) {
                if let Some(import_id) =
                    self.triple_slash_directive_import(directive, line_start, line_end)
                {
                    imports.push(import_id);
                }
            }
            // skip regular line comments
            else if trimmed.starts_with("//") {
                // nothing to do
            }
            // skip block comments before declarations
            else if trimmed.starts_with("/*") {
                if let Some(end_index) = trimmed.find("*/") {
                    let after = trimmed[end_index + 2..].trim_start();
                    if !after.is_empty() {
                        consume_line = false;
                    }
                } else {
                    in_block_comment = true;
                }
            }
            // stop once real source content starts
            else {
                break;
            }

            if !consume_line {
                break;
            }

            while line_end < bytes.len() && (bytes[line_end] == b'\n' || bytes[line_end] == b'\r') {
                line_end += 1;
            }
            offset = line_end;
        }

        (imports, offset >= bytes.len())
    }

    /// Build one import expression from one supported triple slash directive.
    fn triple_slash_directive_import(
        &mut self,
        directive: TripleSlashDirective<'_>,
        line_start: usize,
        line_end: usize,
    ) -> Option<LocalNodeId<Expression>> {
        // select the import source and target for supported directives
        let (source, target) = match directive {
            TripleSlashDirective::ReferencePath(target) => {
                (ImportSource::ReferencePathDirective, target)
            }
            TripleSlashDirective::ReferenceTypes(target) => {
                (ImportSource::ReferenceTypesDirective, target)
            }
            TripleSlashDirective::ReferenceLib(target) => {
                (ImportSource::ReferenceLibDirective, target)
            }
            TripleSlashDirective::NoDefaultLib(target) => {
                (ImportSource::ReferenceNoDefaultLibDirective, target)
            }
        };

        // insert the directive import expression
        let target = self.strings.intern(target);
        let span = Span::new(self.file_id, line_start as u32, line_end as u32);
        let import = Expression::Import {
            source,
            kind: DependencyKind::Type,
            target: ImportTarget::String(target),
            items: None,
            attributes: None,
            arguments: None,
        };

        Some(self.insert_node(import, span))
    }

    /// Parse one triple slash directive line.
    fn triple_slash_directive(line: &str) -> Option<TripleSlashDirective<'_>> {
        let directive = line.strip_prefix("///")?.trim_start();
        let directive = directive.strip_prefix("<reference")?;

        // parse a path directive
        if let Some(path) = Self::triple_slash_reference_attribute_value(directive, "path") {
            return Some(TripleSlashDirective::ReferencePath(path));
        }

        // parse a types directive
        if let Some(types) = Self::triple_slash_reference_attribute_value(directive, "types") {
            return Some(TripleSlashDirective::ReferenceTypes(types));
        }

        // parse a lib directive
        if let Some(lib) = Self::triple_slash_reference_attribute_value(directive, "lib") {
            return Some(TripleSlashDirective::ReferenceLib(lib));
        }

        // parse no default lib directives here, semantics are handled elsewhere
        if let Some(no_default_lib) =
            Self::triple_slash_reference_attribute_value(directive, "no-default-lib")
        {
            return Some(TripleSlashDirective::NoDefaultLib(no_default_lib));
        }

        None
    }

    /// Parse one quoted attribute value from one triple slash reference directive.
    fn triple_slash_reference_attribute_value<'a>(
        directive: &'a str,
        name: &str,
    ) -> Option<&'a str> {
        let mut search_start = 0usize;

        // scan matching attributes with stable boundaries
        while search_start < directive.len() {
            let relative_index = directive[search_start..].find(name)?;
            let index = search_start + relative_index;

            // require a stable attribute boundary before the name
            let before = directive[..index].chars().next_back();
            if before.is_some_and(|character| {
                !character.is_whitespace() && character != '<' && character != '/'
            }) {
                search_start = index + name.len();
                continue;
            }

            // require an equals separator after the attribute name
            let mut remainder = directive[index + name.len()..].trim_start();
            let Some(without_equals) = remainder.strip_prefix('=') else {
                search_start = index + name.len();
                continue;
            };
            remainder = without_equals.trim_start();

            // require one quoted value
            let quote = remainder.chars().next()?;
            if quote != '"' && quote != '\'' {
                search_start = index + name.len();
                continue;
            }

            // extract the quoted attribute value
            let remainder = &remainder[1..];
            let value_end = remainder.find(quote)?;
            return Some(&remainder[..value_end]);
        }

        None
    }
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

        // parse the first argument as the import target
        let target_options = self
            .options
            .nested()
            .not_in_position()
            .not_in_sequence_expression();
        let target_expression =
            self.eat_expression_or_recover_missing(target_options, NodeType::Expression)?;

        // keep static string targets interned when no decorators are attached
        let target_has_decorators = !self.tree.get_decorators(target_expression.id).is_empty();
        let target = match self.tree.get(target_expression) {
            Expression::ScalarLiteral(ScalarLiteral::String(target))
                if !target_has_decorators && !self.lexer.has_comment_tokens() =>
            {
                ImportTarget::String(*target)
            }
            _ => ImportTarget::Expression {
                target: target_expression,
            },
        };

        // parse optional import attributes argument(s)
        self.eat_newlines_maybe()?;
        let arguments = if self.peek_is(TokenType::Comma) || self.peek_is(TokenType::Newline) {
            self.eat_item_stop_with_newlines()?;
            if self.peek_is(TokenType::CloseParenthesis) || self.peek_is(TokenType::End) {
                Some(vec![])
            } else {
                let argument_options = self.options.nested();
                let arguments = self.with_options(argument_options, |parser| {
                    parser.eat_positional_arguments_body(TokenType::CloseParenthesis)
                })?;
                Some(arguments)
            }
        } else {
            None
        };
        self.eat_newlines_maybe()?;
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;

        // import
        let import_id = self.insert_node(
            Expression::Import {
                source: ImportSource::ImportCall,
                kind: DependencyKind::Value,
                target,
                items: None,
                attributes: None,
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

        // allow multiline import heads before bindings or bare targets
        self.eat_newlines_maybe()?;

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
                let expression_id = self.build_import_alias(
                    &start,
                    DeclarationHeader::default(),
                    kind,
                    name,
                    name_span,
                    target,
                );
                return Ok(expression_id);
            }

            let path = self.eat_path()?;
            let target = ImportAliasTarget::Path { path };
            let expression_id = self.build_import_alias(
                &start,
                DeclarationHeader::default(),
                kind,
                name,
                name_span,
                target,
            );
            return Ok(expression_id);
        }

        // binding
        let mut has_binding = false;
        let items = if self.peek_dependency_binding_is() {
            has_binding = true;
            let allow_type_modifier = kind != Some(DependencyKind::Type);
            Some(self.eat_dependency_items_block(allow_type_modifier, false)?)
        } else {
            None
        };

        if has_binding {
            self.eat_newlines_maybe()?;
            self.eat_keyword(Keyword::From)?;
            self.eat_newlines_maybe()?;
        }

        // allow bare import targets on the next line (`import\n"foo"` and comment separated forms)
        self.eat_newlines_maybe()?;
        let (target, target_span) = self.eat_dependency_target_with_span()?;

        // arguments
        let attributes = self.eat_dependency_arguments_maybe()?;

        // import
        let import_id = self.insert_node(
            Expression::Import {
                source: ImportSource::ImportStatement,
                kind: kind.unwrap_or(DependencyKind::Value),
                target: ImportTarget::String(target),
                items,
                attributes,
                arguments: None,
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
    fn eat_import_equals_name_with_span(&mut self) -> ParseResult<(StringId, Span)> {
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
    fn try_eat_import_equals_require_target(&mut self) -> ParseResult<Option<(StringId, Span)>> {
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
        self.eat_close_token_or_recover_missing(TokenType::CloseParenthesis, NodeType::Expression)?;
        Ok(Some((target, target_span)))
    }

    /// Eat `export import Foo = Bar.Baz` as an exported import alias.
    pub(crate) fn eat_export_import_equals(
        &mut self,
        start: &ParserMark,
        header: DeclarationHeader,
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
            let target = ImportAliasTarget::Require { target };
            let expression_id =
                self.build_import_alias(start, header, kind, name, name_span, target);
            return Ok(expression_id);
        }

        // build the exported import alias
        let path = self.eat_path()?;
        let target = ImportAliasTarget::Path { path };
        let expression_id = self.build_import_alias(start, header, kind, name, name_span, target);
        Ok(expression_id)
    }

    /// Build an import alias declaration.
    fn build_import_alias(
        &mut self,
        start: &ParserMark,
        header: DeclarationHeader,
        kind: Option<DependencyKind>,
        name: StringId,
        name_span: Span,
        target: ImportAliasTarget,
    ) -> LocalNodeId<Expression> {
        let declaration = Declaration::ImportAlias(ImportAliasDeclaration {
            name: Name::Identifier(name),
            export: header.export,
            ambient: header.ambient,
            kind: kind.unwrap_or(DependencyKind::Value),
            target,
        });
        let declaration_id = self.insert_node(declaration, self.get_span_from(start));
        self.tree.set_main_span(declaration_id, name_span);
        let expression = Expression::Declaration(declaration_id);
        self.insert_node(expression, self.get_span_from(start))
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
        self.eat_newlines_maybe()?;

        // export default <expression>
        if self.is_keyword(Keyword::Default) {
            self.bump(); // eat default

            // reject export default enum declarations
            if self.is_keyword(Keyword::Enum) {
                return Err(ParseError::unexpected(self.peek()?.span));
            }

            let value =
                self.eat_expression(self.options.not_in_position().not_in_sequence_expression())?;
            let item = self.insert_node(
                DependencyItem::Item {
                    mode: DependencyMode::Default,
                    kind: Some(DependencyKind::Value),
                    name: None,
                    alias: None,
                    value: Some(value),
                },
                self.get_span_from(&start),
            );
            let export = self.insert_node(
                Expression::Export {
                    kind: DependencyKind::Value,
                    target: None,
                    items: vec![item],
                    attributes: None,
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
            let export_id = self.insert_node(
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
            let item = self.insert_node(
                DependencyItem::Item {
                    mode: DependencyMode::Namespace,
                    kind: Some(DependencyKind::Value),
                    name: None,
                    alias: None,
                    value: Some(value),
                },
                self.get_span_from(&start),
            );
            let export = self.insert_node(
                Expression::Export {
                    kind: DependencyKind::Value,
                    target: None,
                    items: vec![item],
                    attributes: None,
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
        let has_namespace_reexport_from = self.peek_is(TokenType::Multiply) && {
            let from_index = self.next_non_newline_index_from(self.pos_index() + 1);
            self.keyword_for_index(from_index) == Some(Keyword::From)
        };
        if has_namespace_reexport_from {
            self.bump(); // eat *
            self.eat_newlines_maybe()?;
            self.eat_keyword(Keyword::From)?;
            self.eat_newlines_maybe()?;
            let (target, target_span) = self.eat_dependency_target_with_span()?;
            let attributes = self.eat_dependency_arguments_maybe()?;
            let item = DependencyItem::Item {
                mode: DependencyMode::Namespace,
                kind: None,
                name: None,
                alias: None,
                value: None,
            };
            let item_id = self.insert_node(item, self.get_span_from(&start));
            let export = self.insert_node(
                Expression::Export {
                    kind: kind.unwrap_or(DependencyKind::Value),
                    target: Some(target),
                    items: vec![item_id],
                    attributes,
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
        let has_from_target = self.is_keyword(Keyword::From)
            || self.peek_is(TokenType::Newline) && self.is_keyword_after_newlines(Keyword::From);
        let (target, target_span) = if has_from_target {
            self.eat_newlines_maybe()?;
            self.eat_keyword(Keyword::From)?;
            self.eat_newlines_maybe()?;
            let (target, span) = self.eat_dependency_target_with_span()?;
            (Some(target), Some(span))
        } else {
            (None, None)
        };

        // assertions or attributes
        let attributes = if target.is_some() {
            self.eat_dependency_arguments_maybe()?
        } else {
            None
        };

        // `export { default }` without `from` is invalid
        // (default is a reserved word and can't be a local binding)
        if target.is_none() {
            for item_id in &items {
                let item = self.tree.get(*item_id);
                if let DependencyItem::Item {
                    mode: DependencyMode::Default,
                    name: None,
                    ..
                } = item
                {
                    let span = self
                        .tree
                        .get_main_span(*item_id)
                        .unwrap_or(self.tree.get_span(*item_id));
                    return Err(ParseError::unexpected(span));
                }
            }
        }

        // export
        let export_id = self.insert_node(
            Expression::Export {
                kind: kind.unwrap_or(DependencyKind::Value),
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

        Ok(export_id)
    }

    /// Decode one parsed expression node into one import attribute value.
    ///
    /// Import attribute values are plain data, not syntax nodes, so this is a
    /// boundary decode from the richer expression tree into the static attribute model.
    fn decode_import_attribute_value(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> ParseResult<ImportAttributeValue> {
        let expression = self.tree.get(expression_id).clone();

        Ok(match expression {
            Expression::ScalarLiteral(value) => ImportAttributeValue::ScalarLiteral(value),
            Expression::ArrayExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());

                for element_id in elements {
                    let Argument::Positional { value } = self.tree.get(element_id) else {
                        return Err(ParseError::unexpected(self.tree.get_span(element_id)));
                    };

                    values.push(self.decode_import_attribute_value(*value)?);
                }

                ImportAttributeValue::Array(values)
            }
            Expression::ObjectExpression { ty, properties } => {
                if ty.is_some() {
                    return Err(ParseError::unexpected(self.tree.get_span(expression_id)));
                }

                let mut attributes = Vec::with_capacity(properties.len());

                for property_id in properties {
                    let Property::Field { key, value, .. } = self.tree.get(property_id) else {
                        return Err(ParseError::unexpected(self.tree.get_span(property_id)));
                    };

                    let destack_ast::Key::Name(key) = *key else {
                        return Err(ParseError::unexpected(self.tree.get_span(property_id)));
                    };
                    let value = self.decode_import_attribute_value(*value)?;

                    attributes.push(ImportAttribute { key, value });
                }

                ImportAttributeValue::Object(attributes)
            }
            _ => return Err(ParseError::unexpected(self.tree.get_span(expression_id))),
        })
    }

    /// Decode one parsed named argument into one import attribute entry.
    fn decode_import_attribute(
        &mut self,
        argument_id: LocalNodeId<Argument>,
    ) -> ParseResult<ImportAttribute> {
        let argument = self.tree.get(argument_id).clone();

        let Argument::Named { name, value } = argument else {
            return Err(ParseError::unexpected(self.tree.get_span(argument_id)));
        };

        let value = self.decode_import_attribute_value(value)?;

        Ok(ImportAttribute { key: name, value })
    }

    /// Eat dependency arguments for import/export attributes.
    fn eat_dependency_arguments_maybe(&mut self) -> ParseResult<Option<ImportAttributeClause>> {
        // attribute clause head
        if !self.is_keyword(Keyword::With) {
            return Ok(None);
        }

        self.bump(); // eat with

        // attribute clause body
        self.eat_newlines_maybe()?;
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)?;
        let argument_options = self.options.nested();
        let arguments = self.with_options(argument_options, |parser| {
            parser.eat_arguments_body(TokenType::CloseBrace)
        })?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Expression)?;

        // decoded attributes
        let mut attributes = Vec::with_capacity(arguments.len());

        for argument_id in arguments {
            attributes.push(self.decode_import_attribute(argument_id)?);
        }

        Ok(Some(ImportAttributeClause {
            kind: ImportAttributeClauseKind::With,
            attributes,
        }))
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
        if self.peek_is(TokenType::OpenBrace) || self.peek_is(TokenType::Multiply) {
            return true;
        }

        if self.peek_is(TokenType::Identifier) {
            let next_index = self.next_non_newline_index_from(self.pos_index() + 1);
            let next_token_type = self.token_type_at(next_index);
            return next_token_type == TokenType::Comma
                || self.keyword_for_index(next_index) == Some(Keyword::From);
        }

        false
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
    fn eat_dependency_target_with_span(&mut self) -> ParseResult<(StringId, Span)> {
        let token = *self.peek_token(TokenType::Literal)?;

        // module targets accept:
        // - regular string literals, including unterminated ones for recovery
        // - terminated single quoted one character literals
        let is_valid_target = matches!(
            token.token.literal,
            Some(LiteralType::String {
                has_invalid_escape: false,
                ..
            })
        ) || matches!(
            token.token.literal,
            Some(LiteralType::Character {
                is_terminated: true,
                ..
            })
        );
        if !is_valid_target {
            return Err(ParseError::expected(token.span, TokenType::Literal));
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
    ) -> ParseResult<Vec<LocalNodeId<DependencyItem>>> {
        let mut items: Vec<LocalNodeId<DependencyItem>> = Vec::new();

        // `Default,` or `foo from`
        let can_start_default_item = if self.peek_is(TokenType::Identifier) {
            let next_index = self.next_non_newline_index_from(self.pos_index() + 1);
            let next_token_type = self.token_type_at(next_index);
            next_token_type == TokenType::Comma
                || self.keyword_for_index(next_index) == Some(Keyword::From)
        } else {
            false
        };

        if can_start_default_item {
            let start = self.mark_span();
            let (alias, alias_span) = self.eat_identifier_with_span()?;

            // parse optional default binding separator
            if self.peek_is(TokenType::Comma) {
                self.bump(); // eat comma
                self.eat_newlines_maybe()?;

                // require a supported binding continuation
                if !self.peek_is(TokenType::OpenBrace) && !self.peek_is(TokenType::Multiply) {
                    return Err(ParseError::unexpected(self.peek()?.span));
                }
            }
            // allow line breaks before `from`
            else {
                self.eat_newlines_maybe()?;
            }

            let item = DependencyItem::Item {
                mode: DependencyMode::Default,
                kind: None,
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
            let start = self.mark_span();
            self.bump(); // eat *
            self.bump(); // eat as
            let (alias, alias_span) = if self.peek_is(TokenType::Literal) {
                self.eat_string_literal_with_span()?
            } else {
                self.eat_identifier_with_span()?
            };
            let item = DependencyItem::Item {
                mode: DependencyMode::Namespace,
                kind: None,
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
            self.eat_newlines_maybe()?;

            while !self.peek_is(TokenType::CloseBrace) {
                let item_start = self.mark_span();

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

                self.eat_newlines_maybe()?;

                if self.peek_is(TokenType::CloseBrace) {
                    break;
                }

                if self.peek_comma_is() {
                    self.eat_item_stop_with_newlines()?;

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

                return Err(ParseError::unexpected(self.peek()?.span));
            }

            self.eat_newlines_maybe()?;
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
            let has_alias_separator = self.is_keyword(Keyword::As)
                || self.peek_is(TokenType::Colon)
                || self.peek_is(TokenType::Newline)
                    && (self.is_keyword_after_newlines(Keyword::As)
                        || self.is_token_after_newlines(self.pos(), TokenType::Colon));
            let (alias, alias_span) = if has_alias_separator {
                self.eat_newlines_maybe()?;
                self.bump(); // eat `as` or `:`
                self.eat_newlines_maybe()?;
                let (alias, alias_span) =
                    self.eat_dependency_item_alias_with_span(allow_literal_alias)?;
                (Some(alias), Some(alias_span))
            } else {
                (None, None)
            };

            // item
            let item = self.insert_node(
                DependencyItem::Item {
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
            let has_alias_separator = self.is_keyword(Keyword::As)
                || self.peek_is(TokenType::Colon)
                || self.peek_is(TokenType::Newline)
                    && (self.is_keyword_after_newlines(Keyword::As)
                        || self.is_token_after_newlines(self.pos(), TokenType::Colon));
            let (alias, alias_span) = if has_alias_separator {
                self.eat_newlines_maybe()?;
                self.bump(); // eat `as` or `:`
                self.eat_newlines_maybe()?;
                let (alias, alias_span) =
                    self.eat_dependency_item_alias_with_span(allow_literal_alias)?;
                (Some(alias), Some(alias_span))
            } else {
                (None, None)
            };

            // item
            let item = self.insert_node(
                DependencyItem::Item {
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
    fn eat_dependency_item_name_with_span(&mut self) -> ParseResult<(Name, Span)> {
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
    ) -> ParseResult<(StringId, Span)> {
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
                token.token.ty == TokenType::Literal
                    && matches!(token.token.literal, Some(LiteralType::Boolean { .. }))
            })
        {
            let span = self.peek()?.span;
            let alias = self.get_span_str(span).to_string();
            let alias = self.strings.intern(&alias);
            self.bump();
            return Ok((alias, span));
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
        Declaration, DependencyItem, DependencyKind, DependencyMode, Expression,
        ImportAliasDeclaration, ImportAliasTarget, ImportAttributeClauseKind, ImportAttributeValue,
        ImportSource, ImportTarget, LocalNodeId, Name, ScalarLiteral,
    };
    use destack_source::LanguageType;

    use crate::{
        Parser, TestParser, assert_expression_path, assert_node, assert_path, assert_string,
    };

    fn assert_import_target_string(parser: &Parser, target: &ImportTarget, expected: &str) {
        assert_node!(target, ImportTarget::String(target) => {
            assert_string!(parser, *target, expected);
        });
    }

    fn import_items(
        items: &Option<Vec<LocalNodeId<DependencyItem>>>,
    ) -> &[LocalNodeId<DependencyItem>] {
        items.as_deref().expect("expected import specifier shell")
    }

    fn assert_bare_import(items: &Option<Vec<LocalNodeId<DependencyItem>>>) {
        assert!(items.is_none());
    }

    fn assert_empty_import_shell(
        items: &Option<Vec<LocalNodeId<DependencyItem>>>,
    ) -> &[LocalNodeId<DependencyItem>] {
        let items = import_items(items);
        assert!(items.is_empty());
        items
    }

    #[test]
    fn test_parse_import_simple() {
        // import sample
        let mut test = TestParser::new("import \"destack\"");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // import
        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_bare_import(items);
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
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: Some(alias),.. } => {
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

        // import sample.module with { bar: true }
        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, attributes, .. } => {
            // sample.module
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            assert_bare_import(items);
            assert_import_target_string(&parser, target, "destack.geometry");

            // with { bar: true }
            let attributes = attributes.as_ref().expect("expected attributes");
            assert_eq!(attributes.kind, ImportAttributeClauseKind::With);
            let entries = &attributes.attributes;
            assert_eq!(entries.len(), 1);
            assert_string!(parser, entries[0].key.string(), "bar");
            assert_eq!(
                entries[0].value,
                ImportAttributeValue::ScalarLiteral(ScalarLiteral::Boolean(true))
            );
        });
    }

    #[test]
    fn test_parse_import_path_with_missing_attribute_close_brace() {
        let mut test = TestParser::new("import \"destack.geometry\" with { bar: true");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_eq!(parser.errors.len(), 1);

        assert_node!(parser.tree, import_id, Expression::Import { attributes, .. } => {
            let attributes = attributes.as_ref().expect("expected attributes");
            assert_eq!(attributes.kind, ImportAttributeClauseKind::With);
            assert_eq!(attributes.attributes.len(), 1);
            assert_string!(parser, attributes.attributes[0].key.string(), "bar");
            assert_eq!(
                attributes.attributes[0].value,
                ImportAttributeValue::ScalarLiteral(ScalarLiteral::Boolean(true))
            );
        });
    }

    #[test]
    fn test_parse_import_path_with_nested_attributes() {
        let mut test = TestParser::new(
            "import \"destack.geometry\" with { mode: \"json\", options: { eager: true, levels: [1, 2] } }",
        );
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { attributes, .. } => {
            let attributes = attributes.as_ref().expect("expected attributes");
            assert_eq!(attributes.kind, ImportAttributeClauseKind::With);
            assert_eq!(attributes.attributes.len(), 2);

            assert_string!(parser, attributes.attributes[0].key.string(), "mode");
            assert_eq!(
                attributes.attributes[0].value,
                ImportAttributeValue::ScalarLiteral(ScalarLiteral::String(parser.strings.intern("json")))
            );

            assert_string!(parser, attributes.attributes[1].key.string(), "options");
            assert_eq!(
                attributes.attributes[1].value,
                ImportAttributeValue::Object(vec![
                    destack_ast::ImportAttribute {
                        key: Name::Identifier(parser.strings.intern("eager")),
                        value: ImportAttributeValue::ScalarLiteral(ScalarLiteral::Boolean(true)),
                    },
                    destack_ast::ImportAttribute {
                        key: Name::Identifier(parser.strings.intern("levels")),
                        value: ImportAttributeValue::Array(vec![
                            ImportAttributeValue::ScalarLiteral(ScalarLiteral::Integer(1)),
                            ImportAttributeValue::ScalarLiteral(ScalarLiteral::Integer(2)),
                        ]),
                    },
                ])
            );
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
            let items = import_items(items);
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem::Item { kind, name: Some(name), alias, .. } => {
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "Vector2");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem::Item { kind, name: Some(name), alias: Some(alias),.. } => {
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
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_string!(parser, *alias, "geom");
            });
            assert_import_target_string(&parser, target, "ds/geometry");
        });
    }

    #[test]
    fn test_parse_import_with_newline_before_from() {
        let mut test = TestParser::new("import { A }\nfrom 'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // parse multiline named import with from on the next line
        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "A");
                assert!(alias.is_none());
            });
            assert_import_target_string(&parser, target, "foo");
        });
    }

    #[test]
    fn test_parse_import_default_with_newline_before_from() {
        let mut test = TestParser::new(
            "import HeaderNavigationButton
from 'foo'",
        );
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // parse multiline default import with from on the next line
        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_eq!(*kind, None);
                assert_string!(parser, *alias, "HeaderNavigationButton");
            });
            assert_import_target_string(&parser, target, "foo");
        });
    }

    #[test]
    fn test_parse_import_default_with_newline_block_items() {
        let mut test = TestParser::new(
            "import Default,
{ type Item }
from 'foo'",
        );
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // parse multiline default plus named imports
        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            let items = import_items(items);
            assert_eq!(items.len(), 2);

            // default binding
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind: None, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "Default");
            });

            // type named binding
            assert_node!(parser.tree, items[1], DependencyItem::Item { mode, kind, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, Some(DependencyKind::Type));
                assert_string!(parser, name.string(), "Item");
                assert!(alias.is_none());
            });

            assert_import_target_string(&parser, target, "foo");
        });
    }

    #[test]
    fn test_parse_import_default_with_newline_comment_before_from() {
        let mut test = TestParser::new(
            "import BreakoutRooms
// @ts-ignore
from 'foo'",
        );
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // parse default import with comment between binding and from
        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind: None, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "BreakoutRooms");
            });
            assert_import_target_string(&parser, target, "foo");
        });
    }

    #[test]
    fn test_parse_import_with_newline_after_from() {
        let mut test = TestParser::new(
            "import { goBack } from
'foo'",
        );
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        // parse import target on the next line after from
        assert_node!(parser.tree, import_id, Expression::Import { source, kind, target, items, .. } => {
            assert_eq!(*source, ImportSource::ImportStatement);
            assert_eq!(*kind, DependencyKind::Value);
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "goBack");
                assert!(alias.is_none());
            });
            assert_import_target_string(&parser, target, "foo");
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

            let items = import_items(items);
            assert_eq!(items.len(), 2);
            assert_node!(parser.tree, items[0], DependencyItem::Item { kind, name: Some(name), alias,.. } => {
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "StructuredObject");
                assert!(alias.is_none());
            });
            assert_node!(parser.tree, items[1], DependencyItem::Item { kind, name: Some(name), alias,.. } => {
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
            let items = import_items(items);
            assert_eq!(items.len(), 2);
            // Default
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind: None, name: None, alias: Some(alias),.. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "Default");
            });
            // { type Item }
            assert_node!(parser.tree, items[1], DependencyItem::Item { mode, kind, name: Some(name), alias: None,.. } => {
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
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { kind, name: Some(name), alias, .. } => {
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
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_import_target_string(&parser, target, "./a");
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: Some(alias), .. } => {
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
            let _items = assert_empty_import_shell(items);
            assert_import_target_string(&parser, target, "foo");
        });
    }

    #[test]
    fn test_parse_import_named_alias_after_newline_comment() {
        let mut test =
            TestParser::new("import {\n  a\n  // keep alias on next line\n  as b\n} from 'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_import_target_string(&parser, target, "foo");
            assert_node!(parser.tree, items[0], DependencyItem::Item { name: Some(name), alias: Some(alias), .. } => {
                assert_string!(parser, name.string(), "a");
                assert_string!(parser, *alias, "b");
            });
        });
    }

    #[test]
    fn test_parse_bare_import_with_newline_after_comment() {
        let mut test = TestParser::new("import // keep target on next line\n'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
            assert_bare_import(items);
            assert_import_target_string(&parser, target, "foo");
        });
    }

    #[test]
    fn test_parse_import_block_with_newline_after_comment() {
        let mut test = TestParser::new("import // keep binding on next line\n{} from 'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
            let _items = assert_empty_import_shell(items);
            assert_import_target_string(&parser, target, "foo");
        });
    }

    #[test]
    fn test_parse_import_type_string_specifier() {
        let mut test = TestParser::new(r#"import { type "string" as foo } from "foo""#);
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_import_target_string(&parser, target, "foo");
            assert_node!(parser.tree, items[0], DependencyItem::Item { kind, name: Some(name), alias: Some(alias), .. } => {
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
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { kind, name: Some(name), alias: Some(alias), .. } => {
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
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { kind, name: Some(name), alias, .. } => {
                assert_eq!(*kind, Some(DependencyKind::Type));
                assert_string!(parser, name.string(), "as");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_import_type_in_import_type_recovers_error_item() {
        // preserve one broken item inside import type blocks
        let mut test = TestParser::new("import type { type Foo } from 'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { kind, items, target, .. } => {
            assert_eq!(*kind, DependencyKind::Type);
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_import_target_string(&parser, target, "foo");
            assert_node!(parser.tree, items[0], DependencyItem::Error);
        });
    }

    #[test]
    fn test_parse_import_block_recovers_broken_alias_item() {
        // preserve valid siblings after one broken import item
        let mut test = TestParser::new("import { Foo as, Bar } from 'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
            let items = import_items(items);
            assert_eq!(items.len(), 2);
            assert_import_target_string(&parser, target, "foo");
            assert_node!(parser.tree, items[0], DependencyItem::Error);
            assert_node!(parser.tree, items[1], DependencyItem::Item { mode, kind, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "Bar");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_import_block_recovers_missing_close_before_from() {
        // preserve the import clause when `}` is omitted before from
        let mut test = TestParser::new("import { Foo from 'foo'");
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_import_target_string(&parser, target, "foo");
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "Foo");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_import_block_keeps_statement_owner_in_missing_close_gap() {
        // keep the import expression enclosing the whitespace gap before `from`
        let source = "import { Widget,  from \"./types.ds\";";
        let cursor = source.find("  from").unwrap() as u32 + 1;
        let mut test = TestParser::new(source);
        let mut parser = test.prepare();
        let roots = parser.parse();
        let root_id = roots[0];

        assert_node!(parser.tree, root_id, Expression::Import { items, target, .. } => {
                let items = import_items(items);
                assert_eq!(items.len(), 1);
                assert_import_target_string(&parser, target, "./types.ds");
                assert_node!(parser.tree, items[0], DependencyItem::Item { name: Some(name), .. } => {
                    assert_string!(parser, name.string(), "Widget");
                });
        });

        let enclosing = parser.tree.source_map.get_enclosing_spans(cursor, cursor);
        assert!(enclosing.iter().any(|span| span.idx == root_id.id));
    }

    #[test]
    fn test_parse_import_keeps_target_main_span_for_unterminated_path() {
        // keep the import expression and target main span around the trailing path byte
        let source = "import { } from \"./u";
        let cursor = source.len() as u32;
        let probe = cursor.saturating_sub(1);
        let mut test = TestParser::new(source);
        let mut parser = test.prepare();
        let roots = parser.parse();
        let root_id = roots[0];

        assert_node!(parser.tree, root_id, Expression::Import { target, .. } => {
                assert_import_target_string(&parser, target, "./u");
        });

        let enclosing = parser.tree.source_map.get_enclosing_spans(probe, probe);
        assert!(enclosing.iter().any(|span| span.idx == root_id.id));

        let main_span = parser.tree.source_map.get_main(root_id.id).unwrap();
        assert!(main_span.contains(probe));
    }

    #[test]
    fn test_parse_export_block_recovers_broken_alias_item() {
        // preserve valid siblings after one broken export item
        let mut test = TestParser::new("export { Foo as, Bar } from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        assert_node!(parser.tree, export_id, Expression::Export { items, target: Some(target), .. } => {
            assert_eq!(items.len(), 2);
            assert_string!(parser, *target, "foo");
            assert_node!(parser.tree, items[0], DependencyItem::Error);
            assert_node!(parser.tree, items[1], DependencyItem::Item { mode, kind, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "Bar");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_type_identifier_name() {
        // treat type as a value name in named exports
        let mut test = TestParser::new("export { type } from 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        assert_node!(parser.tree, export_id, Expression::Export { items, .. } => {
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { kind, name: Some(name), alias, .. } => {
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "type");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_clause_after_comment_newline_keyword() {
        let mut test =
            TestParser::new_with_options("export //comment\n{}", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        assert_node!(parser.tree, export_id, Expression::Export { target, items, .. } => {
            assert!(target.is_none());
            assert!(items.is_empty());
        });
    }

    #[test]
    fn test_parse_export_specifier_alias_after_comment_newline() {
        let mut test = TestParser::new_with_options(
            "export {\n  bar as // comment\n  baz,\n} from 'foo'",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        assert_node!(parser.tree, export_id, Expression::Export { target: Some(target), items, .. } => {
            assert_string!(parser, *target, "foo");
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind: None, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, name.string(), "bar");
                assert_string!(parser, *alias, "baz");
            });
        });
    }

    #[test]
    fn test_parse_import_specifier_alias_after_comment_newline() {
        let mut test = TestParser::new_with_options(
            "import {\n  bar as // comment\n  baz,\n} from 'foo'",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();
        let import_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
            assert_import_target_string(&parser, target, "foo");
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind: None, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, name.string(), "bar");
                assert_string!(parser, *alias, "baz");
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
            let items = import_items(items);
            assert_eq!(items.len(), 2);
            // default: a
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind: None, name: None, alias: Some(alias),.. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "a");
            });
            // namespace: * as b
            assert_node!(parser.tree, items[1], DependencyItem::Item { mode, kind: None, name: None, alias: Some(alias),.. } => {
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
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias(ImportAliasDeclaration { name, kind, target, .. }) => {
                assert_eq!(*kind, DependencyKind::Value);
                assert_string!(parser, name.string(), "A");
                assert!(matches!(target, ImportAliasTarget::Path { .. }));
                let ImportAliasTarget::Path { path } = target else {
                    unreachable!("expected import alias path");
                };
                assert_path!(parser, *path, "B.C");
            });
        });
    }

    #[test]
    fn test_parse_import_equals_require() {
        let mut test = TestParser::new(r#"import a = require("a")"#);
        let mut parser = test.prepare();
        let expression_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias(ImportAliasDeclaration { name, kind, target, .. }) => {
                assert_eq!(*kind, DependencyKind::Value);
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
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias(ImportAliasDeclaration { name, kind, target, .. }) => {
                assert_eq!(*kind, DependencyKind::Type);
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
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias(ImportAliasDeclaration { name, kind, target, .. }) => {
                assert_eq!(*kind, DependencyKind::Type);
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
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias(ImportAliasDeclaration { name, kind, target, .. }) => {
                assert_eq!(*kind, DependencyKind::Type);
                assert_string!(parser, name.string(), "Alias");
                assert!(matches!(target, ImportAliasTarget::Path { .. }));
                let ImportAliasTarget::Path { path } = target else {
                    unreachable!("expected import alias path");
                };
                assert_path!(parser, *path, "Namespace.Value");
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
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias(ImportAliasDeclaration { name, kind, target, .. }) => {
                assert_eq!(*kind, DependencyKind::Type);
                assert_string!(parser, name.string(), "Alias");
                assert!(matches!(target, ImportAliasTarget::Path { .. }));
                let ImportAliasTarget::Path { path } = target else {
                    unreachable!("expected import alias path");
                };
                assert_path!(parser, *path, "Value");
            });
        });
    }

    #[test]
    fn test_parse_import_type_modifier_equals_require() {
        let mut test = TestParser::new(r#"import type React = require("pkg")"#);
        let mut parser = test.prepare();
        let expression_id = parser.eat_import().unwrap();

        assert_node!(parser.tree, expression_id, Expression::Declaration(decl_id) => {
            assert_node!(parser.tree, *decl_id, Declaration::ImportAlias(ImportAliasDeclaration { name, kind, target, .. }) => {
                assert_eq!(*kind, DependencyKind::Type);
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias,.. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "CreateUIMessage");
                assert!(alias.is_none());
            });
            // UIMessage
            assert_node!(parser.tree, items[1], DependencyItem::Item { mode, kind, name: Some(name), alias,.. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "UIMessage");
                assert!(alias.is_none());
            });
        });
    }

    #[test]
    fn test_parse_export_with_newline_before_from() {
        let mut test = TestParser::new("export { A }\nfrom 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        // parse multiline named export with from on the next line
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias, .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "A");
                assert!(alias.is_none());
            });
            assert_string!(parser, *target, "foo");
        });
    }

    #[test]
    fn test_parse_export_namespace_with_newline_before_from() {
        let mut test = TestParser::new("export *\nfrom 'foo'");
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        // parse multiline namespace export with from on the next line
        assert_node!(parser.tree, export_id, Expression::Export { kind, target: Some(target), items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert_eq!(items.len(), 1);
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: None, .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
            });
            assert_string!(parser, *target, "foo");
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: None, .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: None, .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: Some(alias), .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias: None, .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias: None, .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Namespace);
                assert_string!(parser, *alias, "foo");
            });
            assert_string!(parser, *target, "foo");
        });
    }

    #[test]
    fn test_parse_triple_slash_reference_path_leading_import() {
        let mut test = TestParser::new_with_options(
            r#"/// <reference path="global.d.ts" />
export as namespace Foo"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse_without_trivia();

        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Import { source, kind, target, items, arguments, .. } => {
            assert_eq!(*source, ImportSource::ReferencePathDirective);
            assert_eq!(*kind, DependencyKind::Type);
            assert_bare_import(items);
            assert!(arguments.is_none());
            assert_import_target_string(&parser, target, "global.d.ts");
        });
        assert_node!(parser.tree, expressions[1], Expression::ExportNamespace { name } => {
            assert_string!(parser, *name, "Foo");
        });
    }

    #[test]
    fn test_parse_triple_slash_reference_path_stops_after_code() {
        let mut test = TestParser::new_with_options(
            r#"export as namespace Foo
/// <reference path="./late.d.ts" />"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse_without_trivia();

        assert_eq!(expressions.len(), 1);
        assert_node!(parser.tree, expressions[0], Expression::ExportNamespace { name } => {
            assert_string!(parser, *name, "Foo");
        });
    }

    #[test]
    fn test_parse_triple_slash_reference_types_leading_import() {
        let mut test = TestParser::new_with_options(
            r#"/// <reference types="node" />
export as namespace Foo"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse_without_trivia();

        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Import { source, kind, target, items, arguments, .. } => {
            assert_eq!(*source, ImportSource::ReferenceTypesDirective);
            assert_eq!(*kind, DependencyKind::Type);
            assert_bare_import(items);
            assert!(arguments.is_none());
            assert_import_target_string(&parser, target, "node");
        });
        assert_node!(parser.tree, expressions[1], Expression::ExportNamespace { name } => {
            assert_string!(parser, *name, "Foo");
        });
    }

    #[test]
    fn test_parse_triple_slash_reference_lib_leading_import() {
        let mut test = TestParser::new_with_options(
            r#"/// <reference lib="dom" />
export as namespace Foo"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse_without_trivia();

        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Import { source, kind, target, items, arguments, .. } => {
            assert_eq!(*source, ImportSource::ReferenceLibDirective);
            assert_eq!(*kind, DependencyKind::Type);
            assert_bare_import(items);
            assert!(arguments.is_none());
            assert_import_target_string(&parser, target, "dom");
        });
        assert_node!(parser.tree, expressions[1], Expression::ExportNamespace { name } => {
            assert_string!(parser, *name, "Foo");
        });
    }

    #[test]
    fn test_parse_triple_slash_directive_only_file_with_banner_comment() {
        let mut test = TestParser::new_with_options(
            r#"/*! *****************************************************************************
Copyright (c) Microsoft Corporation.
***************************************************************************** */

/// <reference lib="es2024" />
/// <reference lib="esnext" />"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        // keep both leading directives as type imports
        assert_eq!(expressions.len(), 2);

        assert_node!(parser.tree, expressions[0], Expression::Import { source, kind, target, items, arguments, .. } => {
            assert_eq!(*source, ImportSource::ReferenceLibDirective);
            assert_eq!(*kind, DependencyKind::Type);
            assert_bare_import(items);
            assert!(arguments.is_none());
            assert_import_target_string(&parser, target, "es2024");
        });

        assert_node!(parser.tree, expressions[1], Expression::Import { source, kind, target, items, arguments, .. } => {
            assert_eq!(*source, ImportSource::ReferenceLibDirective);
            assert_eq!(*kind, DependencyKind::Type);
            assert_bare_import(items);
            assert!(arguments.is_none());
            assert_import_target_string(&parser, target, "esnext");
        });
    }
    #[test]
    fn test_parse_triple_slash_no_default_lib_is_preserved() {
        let mut test = TestParser::new_with_options(
            r#"/// <reference no-default-lib="true" />
export as namespace Foo"#,
            LanguageType::TypeScriptDeclaration,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse_without_trivia();

        assert_eq!(expressions.len(), 2);
        assert_node!(parser.tree, expressions[0], Expression::Import { source, kind, target, items, arguments, .. } => {
            assert_eq!(*source, ImportSource::ReferenceNoDefaultLibDirective);
            assert_eq!(*kind, DependencyKind::Type);
            assert_bare_import(items);
            assert!(arguments.is_none());
            assert_import_target_string(&parser, target, "true");
        });
        assert_node!(parser.tree, expressions[1], Expression::ExportNamespace { name } => {
            assert_string!(parser, *name, "Foo");
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: None, value: Some(value), .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: None, value: Some(value), .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias, .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "bar");
            });
            // baz as baz
            assert_node!(parser.tree, items[1], DependencyItem::Item { mode, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_string!(parser, name.string(), "baz");
                assert_string!(parser, *alias, "baz");
            });
            // biz
            assert_node!(parser.tree, items[2], DependencyItem::Item { mode, name: Some(name), alias, .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, name: None, alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Default);
                assert_string!(parser, *alias, "bar");
            });
            // default as baz
            assert_node!(parser.tree, items[1], DependencyItem::Item { mode, name: None, alias: Some(alias), .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias, .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias: Some(alias), .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "viteLegacyPluginCjs");
                assert_string!(parser, *alias, "module.exports");
            });
        });
    }

    #[test]
    fn test_parse_export_keyword_literal_alias_without_target() {
        // source: export { true_instance as true, false_instance as false, null_instance as null }
        let source =
            "export { true_instance as true, false_instance as false, null_instance as null }";
        let mut test = TestParser::new(source);
        let mut parser = test.prepare();
        let export_id = parser.eat_export().unwrap();

        // export
        assert_node!(parser.tree, export_id, Expression::Export { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Value);
            assert!(target.is_none());
            assert_eq!(items.len(), 3);

            // true alias
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "true_instance");
                assert_string!(parser, *alias, "true");
            });

            // false alias
            assert_node!(parser.tree, items[1], DependencyItem::Item { mode, kind, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "false_instance");
                assert_string!(parser, *alias, "false");
            });

            // null alias
            assert_node!(parser.tree, items[2], DependencyItem::Item { mode, kind, name: Some(name), alias: Some(alias), .. } => {
                assert_eq!(*mode, DependencyMode::Item);
                assert_eq!(*kind, None);
                assert_string!(parser, name.string(), "null_instance");
                assert_string!(parser, *alias, "null");
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias, .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias, .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias: Some(alias), .. } => {
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
            assert_node!(parser.tree, items[0], DependencyItem::Item { mode, kind, name: Some(name), alias: Some(alias), .. } => {
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

    #[test]
    fn test_parse_root_import_named_binding_from_source() {
        let mut test =
            TestParser::new_with_options("import {a} from 'a';", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        test.assert_no_errors(&parser);
        assert_eq!(expressions.len(), 1);
        assert_node!(parser.tree, expressions[0], Expression::Import { target, items, .. } => {
            assert_import_target_string(&parser, target, "a");
            let items = import_items(items);
            assert_eq!(items.len(), 1);
        });
    }

    #[test]
    fn test_parse_root_import_default_and_namespace() {
        let mut test =
            TestParser::new_with_options("import a, * as b from 'a';", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        test.assert_no_errors(&parser);
        assert_eq!(expressions.len(), 1);
        assert_node!(parser.tree, expressions[0], Expression::Import { target, items, .. } => {
            assert_import_target_string(&parser, target, "a");
            let items = import_items(items);
            assert_eq!(items.len(), 2);
        });
    }

    #[test]
    fn test_parse_root_empty_type_import() {
        let mut test =
            TestParser::new_with_options("import type {} from 'a';", LanguageType::TypeScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        test.assert_no_errors(&parser);
        assert_eq!(expressions.len(), 1);
        assert_node!(parser.tree, expressions[0], Expression::Import { kind, target, items, .. } => {
            assert_eq!(*kind, DependencyKind::Type);
            assert_import_target_string(&parser, target, "a");
            let _items = assert_empty_import_shell(items);
        });
    }

    #[test]
    fn test_parse_root_export_named_binding_from_source() {
        let mut test =
            TestParser::new_with_options("export {a} from 'a';", LanguageType::JavaScript);
        let mut parser = test.prepare();
        let expressions = parser.parse();

        test.assert_no_errors(&parser);
        assert_eq!(expressions.len(), 1);
        assert_node!(parser.tree, expressions[0], Expression::Export { target, items, .. } => {
            assert!(target.is_some());
            assert_eq!(items.len(), 1);
        });
    }
}
