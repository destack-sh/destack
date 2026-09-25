use crate::parse::{DeclarationHeader, TypeKeywordHeader, TypePosition, TypeStop};
use crate::{ParseStart, Parser, ParserError, ParserResult};

use tspp_dir::{
    Declaration, Keyword, LocalNodeId, Mutability, TokenType, TypeDeclaration, TypeExpression,
    TypeKind,
};
use tspp_source::{ByteRange, NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Parse a type alias or expression.
    ///
    /// Examples:
    /// ```tspp
    /// type T = int32
    /// type T = foo()
    /// type T = { a: int32, b: boolean } | true
    /// type 1 | 2 | 3
    /// readonly T
    /// newtype T = int32
    /// newtype Foo<T> = Baz<T> | null
    /// newtype T = { a: int32, b: boolean } | true
    /// ```
    pub(crate) fn parse_type_declaration(
        &mut self,
        start: &ParseStart,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let keyword_start = self.mark_parse_start();
        let keyword = self.eat_keyword_in(&[Keyword::Type, Keyword::Readonly, Keyword::Newtype])?;
        let keyword_range = self.range_since(&keyword_start);
        let type_keyword = TypeKeywordHeader::from_keyword(keyword)
            .ok_or_else(|| ParserError::unexpected(self.peek_token().range()))?;

        // named aliases are declarations, not type operands
        if self.peek_identifier_type_alias() {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        self.parse_type_keyword_body(start, keyword_range, type_keyword, stop)
    }

    /// Return true when the current identifier head starts a type alias.
    pub(in crate::parse) fn peek_identifier_type_alias(&self) -> bool {
        // require one identifier head first
        if !self.peek_is(TokenType::Identifier) {
            return false;
        }

        // `name = ...`
        if self.peek_next_token_type() != TokenType::LessThan {
            return self.peek_next_token_type() == TokenType::Assign;
        }

        self.peek_token_after_angle_group(2, 1)
            .is_none_or(|token| token.is(TokenType::Assign))
    }

    /// Return true when the current type keyword starts an alias declaration.
    pub(in crate::parse) fn peek_type_keyword_alias(&self) -> bool {
        let name = self.peek_next_token();
        if !name.is(TokenType::Identifier) || name.is_on_new_line() {
            return false;
        }

        let after_name = self.peek_token_type_at(2);
        if after_name == TokenType::Assign {
            return true;
        }

        if after_name != TokenType::LessThan {
            return false;
        }

        self.peek_token_after_angle_group(3, 1)
            .is_none_or(|token| token.is(TokenType::Assign))
    }

    /// Parse a named type alias declaration.
    ///
    /// Examples:
    /// ```tspp
    /// Value = string
    /// Value<T> = Result<T, Error>
    /// Value = { id: string }
    /// ```
    #[inline(never)]
    pub(in crate::parse) fn parse_type_alias(
        &mut self,
        start: &ParseStart,
        header: DeclarationHeader,
        type_keyword: TypeKeywordHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        let (name, name_range) = self.eat_name_with_range()?;
        let generic_parameter_container_start = self.mark_parse_start();
        let generic_parameters = self
            .parse_generic_parameters_if_present(true)?
            .unwrap_or_default();
        let generic_parameter_container_range = (!generic_parameters.is_empty())
            .then(|| self.range_since(&generic_parameter_container_start));

        // alias assignment
        self.eat_token(TokenType::Assign)?;

        // backing visibility ahead of a newtype value
        let backing_visibility = self.parse_visibility_if_present();

        // alias value
        let value_id = self.parse_type_alias_value(TypeStop::default())?;

        // declaration node
        let declaration = Declaration::Type(TypeDeclaration {
            name,
            backing_visibility,
            export: header.export,
            is_shared: header.is_shared,
            is_ambient: header.is_ambient,
            is_nominal: type_keyword.kind == TypeKind::Nominal,
            mutability: type_keyword.mutability,
            generic_parameters,
            where_clauses: Vec::new(),
            value: value_id,
        });
        let declaration_id = self.insert_node(declaration, self.range_since(start));
        self.tree.set_main_range(declaration_id, name_range);
        if let Some(range) = generic_parameter_container_range {
            self.tree.set_side_range(
                declaration_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                range,
            );
        }

        Ok(declaration_id)
    }

    /// Parse a type keyword expression body.
    ///
    /// Examples:
    /// ```tspp
    /// readonly string
    /// readonly string[]
    /// readonly { id: string }
    /// ```
    fn parse_type_keyword_body(
        &mut self,
        start: &ParseStart,
        keyword_range: ByteRange,
        type_keyword: TypeKeywordHeader,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let right = self.parse_type(TypePosition::Type, stop.nest())?;
        if type_keyword.mutability != Some(Mutability::Immutable) {
            return Ok(right);
        }

        let expression_id = self.insert_node(
            TypeExpression::Readonly { target_type: right },
            self.range_since(start),
        );
        self.tree.set_main_range(expression_id, keyword_range);

        Ok(expression_id)
    }

    /// Parse one type alias value.
    ///
    /// Examples:
    /// ```tspp
    /// intrinsic
    /// string | number
    /// { id: string }
    /// ```
    fn parse_type_alias_value(
        &mut self,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let value = self.parse_type(TypePosition::Type, stop.nest())?;

        // reject optional type suffixes outside tuple and parameter heads
        if self.peek_is(TokenType::Maybe) {
            return Err(ParserError::unexpected(self.peek_token_span()));
        }

        Ok(value)
    }
}
