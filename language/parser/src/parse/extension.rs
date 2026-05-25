use crate::parse::DeclarationHeader;
use crate::parse::prelude::*;
use crate::{Parser, ParserResult, ParserSpanStart};

use destack_dir::{Declaration, ExtensionDeclaration, Keyword, LocalNodeId, NodeType, TokenType};
use destack_source::{NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Eat an extension (incl. `extension` keyword).
    ///
    /// Examples:
    /// ```
    /// extension of Foo {
    ///     ...
    /// }
    ///
    /// extension MyExt of Foo<int32> {
    ///     ...
    /// }
    ///
    /// extension of Bar<int32> implements Baz {
    ///     ...
    /// }
    ///
    /// extension MyExt<T> of Bar<T> implements Baz {
    ///     ...
    /// }
    ///
    /// extension<T> of Bar<T> implements Baz {
    ///     ...
    /// }
    /// ```
    pub(crate) fn eat_extension(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        // keyword
        self.eat_keyword(Keyword::Extension)?;

        // name and generic parameters
        let (generic_parameters, generic_parameter_container_span, name, name_span) =
            // named extension
            if self.peek_name_is() && !self.is_keyword(Keyword::Of) {
                let (name, span) = self.eat_name_with_span()?;

                // `<T>`, only the generic parameter container belongs to this span
                let generic_parameter_container_start = self.span_start();
                let generic_parameters = self.eat_generic_parameters_maybe(false)?;
                let generic_parameter_container_span = generic_parameters
                    .as_ref()
                    .map(|_| self.get_span_from(&generic_parameter_container_start));

                (
                    generic_parameters,
                    generic_parameter_container_span,
                    Some(name),
                    Some(span),
                )
            }
            // anonymous extension
            else {
                // `<T>`, anonymous extensions may start with generic parameters
                let generic_parameter_container_start = self.span_start();
                let generic_parameters = self.eat_generic_parameters_maybe(false)?;
                let generic_parameter_container_span = generic_parameters
                    .as_ref()
                    .map(|_| self.get_span_from(&generic_parameter_container_start));

                (generic_parameters, generic_parameter_container_span, None, None)
            };

        // `of` keyword
        self.eat_keyword(Keyword::Of)?;

        // target type
        let target_start = self.span_start();
        let target_type = self.eat_type_expression_or_recover_missing(
            self.flags
                .nested()
                .in_super_type()
                .in_before_block()
                .in_type(),
            NodeType::Declaration,
        )?;

        // record the full type span for the target type
        self.tree.set_side_span(
            target_type,
            NodeSpanType::Region(NodeSpanRegion::Type),
            self.get_span_from(&target_start),
        );

        // implements types
        let implements_types = self.eat_implements_types_if_present()?;

        // where
        let where_clauses = self.eat_where_maybe()?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;
        let members = self.eat_members(false)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        // extension
        let extension_id = self.insert_node(
            Declaration::Extension(ExtensionDeclaration {
                name,
                export: header.export,
                is_ambient: header.is_ambient,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses: where_clauses.unwrap_or_default(),
                target_type,
                implements_types: implements_types.unwrap_or_default(),
                members,
            }),
            self.get_span_from(start),
        );

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(extension_id, span);
        }
        if let Some(span) = generic_parameter_container_span {
            self.tree.set_side_span(
                extension_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                span,
            );
        }

        Ok(extension_id)
    }
}
