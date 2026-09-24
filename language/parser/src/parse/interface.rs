use crate::parse::error::ParserResultExt;
use crate::parse::{DeclarationHeader, TypeMemberContainerKind};
use crate::{ParseStart, Parser, ParserError, ParserResult};

use destack_dir::{
    Declaration, InterfaceDeclaration, Keyword, LocalNodeId, NodeType, TokenType, TypeKind,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Parse one structural or nominal interface declaration.
    ///
    /// Examples:
    /// ```ds
    /// interface Collection<T> extends Iterable<T> {}
    /// ```
    pub(crate) fn parse_interface(
        &mut self,
        start: &ParseStart,
        header: DeclarationHeader,
        kind: TypeKind,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        // interface
        self.eat_keyword(Keyword::Interface)
            .in_node(NodeType::Declaration)?;

        // interface keyword cannot be followed by a newline
        if self.peek_is_on_new_line() {
            let error = ParserError::unexpected(self.peek_token_span());
            self.report_error(error);
        }

        // interface Name
        let (name, name_range) =
            if let Some((name, range)) = self.eat_name_with_range_if_present()? {
                (Some(name), Some(range))
            } else {
                (None, None)
            };

        // <parameters>
        let generic_parameter_container_start = self.mark_parse_start();
        let generic_parameters = self.parse_generic_parameters_if_present(true)?;
        let generic_parameter_container_range = generic_parameters
            .as_ref()
            .map(|_| self.range_since(&generic_parameter_container_start));

        // extends Base
        let extends_types = self.parse_extends_types_if_present()?;

        // where constraints
        let where_clauses = self.parse_where_clauses()?;

        // { members }
        self.eat_token_before(TokenType::OpenBrace, TokenType::CloseBrace)
            .in_node(NodeType::Declaration)?;

        let member_container_kind = TypeMemberContainerKind::from(kind);
        let members = self.parse_type_members(member_container_kind)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        // retain the complete declaration
        let interface_id = self.insert_node(
            Declaration::Interface(InterfaceDeclaration {
                name,
                export: header.export,
                is_shared: header.is_shared,
                is_ambient: header.is_ambient,
                is_nominal: kind == TypeKind::Nominal,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses,
                extends_types: extends_types.unwrap_or_default(),
                members,
            }),
            self.range_since(start),
        );

        // set the main source range to the name identifier
        if let Some(range) = name_range {
            self.tree.set_main_range(interface_id, range);
        }
        if let Some(range) = generic_parameter_container_range {
            self.tree.set_side_range(
                interface_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                range,
            );
        }

        Ok(interface_id)
    }
}
