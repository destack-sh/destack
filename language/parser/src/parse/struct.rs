use crate::parse::DeclarationHeader;
use crate::parse::context::FunctionContext;
use crate::parse::error::ParserResultExt;
use crate::{ParseStart, Parser, ParserError, ParserResult};

use destack_dir::{
    ClassDeclaration, Declaration, Keyword, LocalNodeId, NodeType, StructDeclaration, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Parse one struct or class declaration.
    ///
    /// Examples:
    /// ```ds
    /// struct Point<T> { x: T; y: T }
    /// class Widget extends View {}
    /// ```
    pub(crate) fn parse_struct_or_class(
        &mut self,
        start: &ParseStart,
        header: DeclarationHeader,
        allow_anonymous_class: bool,
        function: FunctionContext,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        // struct or class
        let keyword = self
            .eat_keyword_in(&[Keyword::Struct, Keyword::Class])
            .in_node(NodeType::Declaration)?;
        let is_class = keyword == Keyword::Class;

        // struct|class Name
        let has_heritage_keyword =
            self.peek_is_keyword(Keyword::Extends) || self.peek_is_keyword(Keyword::Implements);

        // require a name for class and struct declarations
        let allow_anonymous = is_class && allow_anonymous_class;
        let (name, name_range) = if !allow_anonymous {
            let (name, range) = self.eat_name_with_range()?;
            (Some(name), Some(range))
        } else if has_heritage_keyword {
            (None, None)
        } else if let Some((name, range)) = self.eat_name_with_range_if_present()? {
            (Some(name), Some(range))
        } else {
            (None, None)
        };

        // <parameters>
        let generic_parameter_container_start = self.mark_parse_start();
        let generic_parameters = self
            .parse_generic_parameters_if_present(false, function)
            .in_node(NodeType::Declaration)?;
        let generic_parameter_container_range = generic_parameters
            .as_ref()
            .map(|_| self.range_since(&generic_parameter_container_start));

        // extends Base
        let unexpected_extends_range = if !is_class && self.peek_is_keyword(Keyword::Extends) {
            Some(self.peek_token().range())
        } else {
            None
        };
        let extends_clause = if is_class {
            self.parse_extends_types_if_present(function)
                .in_node(NodeType::Declaration)?
        } else if unexpected_extends_range.is_some() {
            self.parse_extends_types_if_present(function)
                .in_node(NodeType::Declaration)?;
            None
        } else {
            None
        };
        if let Some(range) = unexpected_extends_range {
            self.report_error(ParserError::unexpected(range).in_node(NodeType::Declaration));
        }

        // implements Trait
        let implements_types = self
            .parse_implements_types_if_present(function)
            .in_node(NodeType::Declaration)?;

        // where constraints
        let where_clauses = self
            .parse_where_clauses(function)
            .in_node(NodeType::Declaration)?;

        // { members }
        self.eat_token_before(TokenType::OpenBrace, TokenType::CloseBrace)
            .in_node(NodeType::Declaration)?;
        let members = self.parse_members(function)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        // struct or class
        let declaration = if is_class {
            let extends_type = extends_clause.and_then(|mut types| {
                if types.is_empty() {
                    return None;
                }

                // report every unsupported extra base from its first occurrence
                if let Some(extra_type) = types.get(1) {
                    let range = self.tree.get_range(*extra_type);
                    self.report_error(
                        ParserError::unexpected(range).in_node(NodeType::Declaration),
                    );
                }

                Some(types.remove(0))
            });

            Declaration::Class(ClassDeclaration {
                name,
                export: header.export,
                place: header.place,
                is_ambient: header.is_ambient,
                is_abstract: header.is_abstract,
                is_final: header.is_final,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses,
                extends_type,
                implements_types: implements_types.unwrap_or_default(),
                members,
            })
        } else {
            let Some(name) = name else {
                return Err(ParserError::unexpected(self.range_since(start)));
            };

            Declaration::Struct(StructDeclaration {
                name,
                export: header.export,
                place: header.place,
                is_ambient: header.is_ambient,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses,
                implements_types: implements_types.unwrap_or_default(),
                members,
            })
        };
        let declaration_id = self.insert_node(declaration, self.range_since(start));

        // set the name identifier as the main source range
        if let Some(range) = name_range {
            self.tree.set_main_range(declaration_id, range);
        }
        if let Some(range) = generic_parameter_container_range {
            self.tree.set_side_range(
                declaration_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                range,
            );
        }

        Ok(declaration_id)
    }
}
