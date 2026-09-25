use crate::parse::error::ParserResultExt;
use crate::parse::{DeclarationHeader, TypePosition, TypeStop};
use crate::{ParseStart, Parser, ParserResult};

use tspp_dir::{Declaration, ExtensionDeclaration, Keyword, LocalNodeId, NodeType, TokenType};
use tspp_source::{NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Parse one extension declaration.
    ///
    /// Examples:
    /// ```tspp
    /// extension<T> of Vector<T> implements Iterable<T> {}
    /// ```
    pub(crate) fn parse_extension(
        &mut self,
        start: &ParseStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        // extension
        self.eat_keyword(Keyword::Extension)?;

        // extension Name
        let (name, name_range) = if self.peek_name_start() && !self.peek_is_keyword(Keyword::Of) {
            let (name, range) = self.eat_name_with_range()?;

            (Some(name), Some(range))
        } else {
            (None, None)
        };

        // <parameters>
        let generic_parameter_container_start = self.mark_parse_start();
        let generic_parameters = self.parse_generic_parameters_if_present(false)?;
        let generic_parameter_container_range = generic_parameters
            .as_ref()
            .map(|_| self.range_since(&generic_parameter_container_start));

        // of
        self.eat_keyword(Keyword::Of)?;

        // target
        let target_start = self.mark_parse_start();
        let target_type = self.parse_type_or_recover_missing(
            TypePosition::Type,
            TypeStop::IMPLEMENTS,
            NodeType::Declaration,
        )?;

        // record the full type source range for the target type
        self.tree.set_side_range(
            target_type,
            NodeSpanType::Region(NodeSpanRegion::Type),
            self.range_since(&target_start),
        );

        // implements types
        let implements_types = self.parse_implements_types_if_present()?;

        // where constraints
        let where_clauses = self.parse_where_clauses()?;

        // { members }
        self.eat_token_before(TokenType::OpenBrace, TokenType::CloseBrace)
            .in_node(NodeType::Declaration)?;
        let members = self.parse_members()?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        // retain the complete declaration
        let extension_id = self.insert_node(
            Declaration::Extension(ExtensionDeclaration {
                name,
                export: header.export,
                is_ambient: header.is_ambient,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses,
                target_type,
                implements_types: implements_types.unwrap_or_default(),
                members,
            }),
            self.range_since(start),
        );

        // select the declared name or the anonymous extension target
        let selection_range = name_range.or_else(|| self.tree.get_main_range(target_type));
        if let Some(range) = selection_range {
            self.tree.set_main_range(extension_id, range);
        }
        if let Some(range) = generic_parameter_container_range {
            self.tree.set_side_range(
                extension_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                range,
            );
        }

        Ok(extension_id)
    }
}
