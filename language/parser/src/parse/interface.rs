use crate::parse::prelude::*;
use crate::parse::{DeclarationHeader, TypeMemberBodyMode};
use crate::{ParseResult, Parser, ParserSpanStart};

use destack_dir::{
    Declaration, InterfaceDeclaration, Keyword, LocalNodeId, NodeType, TokenType, TypeKind,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Eat an Interface.
    ///
    /// Interfaces can be structural (default) or nominal (`newtype interface`).
    /// The `kind` parameter determines which.
    ///
    /// Examples:
    /// ```
    /// interface { // anonymous interface
    ///     ...
    /// }
    ///
    /// interface Foo extends Baz { // Foo extends Baz
    ///     ..Bar
    ///     ..Boz
    ///
    ///     myField: int32
    ///     myOtherField: boolean | Vector2
    ///
    ///     static x: int32 // constant
    ///     foo() => int32
    ///
    ///     myFunc() { // nested declaration, default implementation
    ///     }
    /// }
    ///
    /// interface Baz<T> {
    ///     ..Bar
    ///
    ///     isThing: true
    ///
    ///     function baz() => T // semicolon optional
    /// }
    ///
    /// // Nominal interface - requires explicit `implements`
    /// newtype interface Add<T, R = Self> {
    ///     add(other: T): R
    /// }
    ///
    /// // Marker trait - nominal, no methods
    /// newtype interface Send {}
    /// ```
    pub(crate) fn eat_interface(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
        kind: TypeKind,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        // typed interface heads do not admit tree literals
        let allow_tree_literals = if self.language.is_typescript() {
            let allow_tree_literals = self.allow_tree_literals();
            self.set_allow_tree_literals(false);
            Some(allow_tree_literals)
        } else {
            None
        };

        let result = (|| {
            // keyword
            self.eat_keyword(Keyword::Interface)
                .for_node_type(NodeType::Declaration)?;

            // interface keyword cannot be followed by a newline
            if self.current_token_is_on_new_line() {
                let error = ParseError::unexpected(self.peek()?.span);
                self.error(&error);
            }

            // optional name / key
            let (name, name_span) = if let Some((name, span)) = self.eat_name_maybe_with_span()? {
                (Some(name), Some(span))
            } else {
                (None, None)
            };

            // optional generic parameters: < ... >
            let generic_parameter_container_start = self.span_start();
            let generic_parameters = self.eat_generic_parameters_maybe(true)?;
            let generic_parameter_container_span = generic_parameters
                .as_ref()
                .map(|_| self.get_span_from(&generic_parameter_container_start));

            // optional extends
            let extends = self.eat_interface_extends_if_present()?;

            // where
            let where_clauses = self.eat_where_maybe()?;

            // body
            self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
                .for_node_type(NodeType::Declaration)?;

            // parse interface members in type context
            let member_flags = self.flags.nested().in_type();
            let member_body_mode = TypeMemberBodyMode::for_interface(kind);
            let members = self.with_flags(member_flags, |parser| {
                parser.eat_type_members(member_body_mode)
            })?;
            self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

            // interface
            let interface_id = self.insert_node(
                Declaration::Interface(InterfaceDeclaration {
                    name,
                    export: header.export,
                    is_ambient: header.is_ambient,
                    is_nominal: kind == TypeKind::Nominal,
                    generic_parameters: generic_parameters.unwrap_or_default(),
                    where_clauses: where_clauses.unwrap_or_default(),
                    extends: extends.unwrap_or_default(),
                    members,
                }),
                self.get_span_from(start),
            );

            // set main span to the name identifier
            if let Some(span) = name_span {
                self.tree.set_main_span(interface_id, span);
            }
            if let Some(span) = generic_parameter_container_span {
                self.tree.set_side_span(
                    interface_id,
                    NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                    span,
                );
            }

            Ok(interface_id)
        })();

        if let Some(allow_tree_literals) = allow_tree_literals {
            self.set_allow_tree_literals(allow_tree_literals);
        }

        result
    }
}
