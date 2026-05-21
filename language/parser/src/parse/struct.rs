#![allow(clippy::type_complexity)]

use crate::parse::DeclarationHeader;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserSpanStart};

use destack_dir::{
    ClassDeclaration, Declaration, Keyword, LocalNodeId, NodeType, StructDeclaration, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Eat a struct or class declaration.
    ///
    /// The parser accepts `extends` for classes and `implements` for structs.
    /// Class declarations treat `extends` as one superclass expression.
    /// Struct declarations require a name.
    ///
    /// Struct forms:
    /// ```
    /// struct Bar {
    ///     myField: int32;
    ///     myOtherField: boolean;
    /// };
    ///
    /// struct Foo<T> implements Drawable { // structs can implement interfaces
    ///     myField: int32;
    ///     myOtherField: T;
    ///
    ///     static x: int32 = 7; // constant
    ///
    ///     myFunc() { }
    /// };
    /// ```
    ///
    /// Class forms:
    /// ```
    /// class Foo extends Bar { // classes can extend
    ///     myField: int32;
    /// };
    /// ```
    pub(crate) fn eat_struct_or_class(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
        allow_anonymous_class: bool,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        // keyword
        let keyword = self
            .eat_keyword_in(&[Keyword::Struct, Keyword::Class])
            .for_node_type(NodeType::Declaration)?;
        let is_class = keyword == Keyword::Class;

        // optional name / key
        let has_heritage_keyword =
            self.is_keyword(Keyword::Extends) || self.is_keyword(Keyword::Implements);

        // require a name for class and struct declarations
        let allow_anonymous = is_class && allow_anonymous_class;
        let (name, name_span) = if !allow_anonymous {
            let (name, span) = self.eat_name_with_span()?;
            (Some(name), Some(span))
        } else if has_heritage_keyword {
            (None, None)
        } else if let Some((name, span)) = self.eat_name_maybe_with_span()? {
            (Some(name), Some(span))
        } else {
            (None, None)
        };

        // optional generic parameters: < ... >
        let generic_parameter_container_start = self.span_start();
        let generic_parameters = self
            .eat_generic_parameters_maybe(false)
            .for_node_type(NodeType::Declaration)?;
        let generic_parameter_container_span = generic_parameters
            .as_ref()
            .map(|_| self.get_span_from(&generic_parameter_container_start));

        // optional extends clause
        let unexpected_extends_span = if !is_class && self.is_keyword(Keyword::Extends) {
            Some(self.peek()?.span)
        } else {
            None
        };
        let extends_clause = if is_class {
            self.eat_extends_expressions_if_present()
                .for_node_type(NodeType::Declaration)?
        } else if unexpected_extends_span.is_some() {
            self.eat_extends_types_if_present()
                .for_node_type(NodeType::Declaration)?;
            None
        } else {
            None
        };
        if let Some(span) = unexpected_extends_span {
            self.error(&ParseError::unexpected_for(span, NodeType::Declaration));
        }

        // optional implements types
        let implements_types = self
            .eat_implements_types_if_present()
            .for_node_type(NodeType::Declaration)?;

        // where
        let where_clauses = self
            .eat_where_maybe()
            .for_node_type(NodeType::Declaration)?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;
        let member_flags = self.flags.nested().in_variant();
        let members = self.with_flags(member_flags, |parser| parser.eat_members(false))?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        // struct or class
        let declaration = if is_class {
            let extends_clause = extends_clause.and_then(|mut expressions| {
                if expressions.is_empty() {
                    None
                } else {
                    Some(expressions.remove(0))
                }
            });
            let (extends_expression, extends_generic_arguments) =
                if let Some(expression_id) = extends_clause {
                    let (extends_expression, extends_generic_arguments) =
                        self.split_instantiation_expression(expression_id);
                    (Some(extends_expression), extends_generic_arguments)
                } else {
                    (None, vec![])
                };

            Declaration::Class(ClassDeclaration {
                name,
                export: header.export,
                is_ambient: header.is_ambient,
                is_abstract: header.is_abstract,
                is_final: header.is_final,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses: where_clauses.unwrap_or_default(),
                extends_expression,
                extends_generic_arguments,
                implements_types: implements_types.unwrap_or_default(),
                members,
            })
        } else {
            let Some(name) = name else {
                return Err(ParseError::unexpected(self.get_span_from(start)));
            };

            Declaration::Struct(StructDeclaration {
                name,
                export: header.export,
                is_ambient: header.is_ambient,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses: where_clauses.unwrap_or_default(),
                implements_types: implements_types.unwrap_or_default(),
                members,
            })
        };
        let declaration_id = self.insert_node(declaration, self.get_span_from(start));

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(declaration_id, span);
        }
        if let Some(span) = generic_parameter_container_span {
            self.tree.set_side_span(
                declaration_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                span,
            );
        }

        Ok(declaration_id)
    }
}
