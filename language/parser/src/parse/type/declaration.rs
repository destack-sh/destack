use crate::parse::DeclarationHeader;
use crate::{Parser, ParserError, ParserResult, ParserSpanStart};

use destack_dir::{
    Declaration, Keyword, LocalNodeId, Mutability, TokenType, TypeDeclaration, TypeExpression,
    TypeKind,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

/// Parsed type keyword header.
#[derive(Debug, Copy, Clone)]
pub(in crate::parse) struct TypeKeywordHeader {
    /// The type declaration kind implied by the keyword.
    kind: TypeKind,
    /// The alias mutability implied by the keyword.
    mutability: Option<Mutability>,
}

impl Parser {
    /// Eat a type alias or expression.
    ///
    /// Examples:
    /// ```ds
    /// type T = int32
    /// type T = foo()
    /// type T = { a: int32, b: boolean } | true
    /// type 1 | 2 | 3
    /// readonly T
    /// newtype T = int32
    /// newtype Foo<T> = Baz<T> | null
    /// newtype T = { a: int32, b: boolean } | true
    /// ```
    pub(crate) fn eat_type(
        &mut self,
        start: &ParserSpanStart,
        _header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let keyword_start = self.span_start();
        let keyword = self.eat_keyword_in(&[Keyword::Type, Keyword::Readonly, Keyword::Newtype])?;
        let keyword_span = self.get_span_from(&keyword_start);
        let type_keyword = self.type_keyword_header(keyword)?;

        // named aliases are declarations, not type operands
        if self.identifier_starts_type_alias() {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        self.eat_type_keyword_body_expression(start, keyword_span, type_keyword)
    }

    /// Return true when the current identifier head starts a type alias.
    pub(in crate::parse) fn identifier_starts_type_alias(&mut self) -> bool {
        // require one identifier head first
        if !self.peek_identifier_is() {
            return false;
        }

        // `name = ...`
        if self.next_token_type() != TokenType::LessThan {
            return self.next_token_type() == TokenType::Assign;
        }

        self.lookahead(|parser| {
            parser.bump();
            parser.bump();

            parser
                .scan_angle_follow_token_after_open(1)
                .is_none_or(|token_type| token_type == TokenType::Assign)
        })
    }

    /// Return true when the current type keyword starts an alias declaration.
    pub(in crate::parse) fn type_keyword_starts_alias_declaration(&mut self) -> bool {
        let name = self.next_token();
        if !name.is(TokenType::Identifier) || name.is_on_new_line() {
            return false;
        }

        let after_name = self.token_type_at_offset(2);
        if after_name == TokenType::Assign {
            return true;
        }

        if after_name != TokenType::LessThan {
            return false;
        }

        self.lookahead(|parser| {
            parser.bump();
            parser.bump();
            parser.bump();

            parser
                .scan_angle_follow_token_after_open(1)
                .is_none_or(|token_type| token_type == TokenType::Assign)
        })
    }

    /// Return type keyword metadata.
    pub(in crate::parse) fn type_keyword_header(
        &mut self,
        keyword: Keyword,
    ) -> ParserResult<TypeKeywordHeader> {
        let kind = match keyword {
            Keyword::Type | Keyword::Readonly => TypeKind::Structural,
            Keyword::Newtype => TypeKind::Nominal,
            _ => return Err(ParserError::unexpected(self.anchor_span_here())),
        };

        let mutability = if keyword == Keyword::Readonly {
            Some(Mutability::Immutable)
        } else {
            None
        };

        Ok(TypeKeywordHeader { kind, mutability })
    }

    /// Eat a named type alias declaration.
    ///
    /// Examples:
    /// ```ds
    /// Value = string
    /// Value<T> = Result<T, Error>
    /// Value = { id: string }
    /// ```
    pub(in crate::parse) fn eat_type_alias_declaration(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
        type_keyword: TypeKeywordHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        let (name, name_span) = self.eat_name_with_span()?;
        let generic_parameter_container_start = self.span_start();
        let generic_parameters = self.eat_generic_parameters_maybe(true)?.unwrap_or_default();
        let generic_parameter_container_span = (!generic_parameters.is_empty())
            .then(|| self.get_span_from(&generic_parameter_container_start));

        // alias assignment
        self.eat_token(TokenType::Assign)?;

        // alias value
        let value_id = self.eat_type_alias_value()?;

        // declaration node
        let declaration = Declaration::Type(TypeDeclaration {
            name,
            export: header.export,
            is_ambient: header.is_ambient,
            is_nominal: type_keyword.kind == TypeKind::Nominal,
            mutability: type_keyword.mutability,
            generic_parameters,
            where_clauses: vec![],
            value: value_id,
        });
        let declaration_id = self.insert_node(declaration, self.get_span_from(start));
        self.tree.set_main_span(declaration_id, name_span);
        if let Some(span) = generic_parameter_container_span {
            self.tree.set_side_span(
                declaration_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                span,
            );
        }

        Ok(declaration_id)
    }

    /// Eat a type keyword expression body.
    ///
    /// Examples:
    /// ```ds
    /// readonly string
    /// readonly string[]
    /// readonly { id: string }
    /// ```
    fn eat_type_keyword_body_expression(
        &mut self,
        start: &ParserSpanStart,
        keyword_span: destack_source::Span,
        type_keyword: TypeKeywordHeader,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let right = self.eat_type_keyword_body()?;
        if type_keyword.mutability != Some(Mutability::Immutable) {
            return Ok(right);
        }

        let expression_id = self.insert_node(
            TypeExpression::Readonly { target_type: right },
            self.get_span_from(start),
        );
        self.tree.set_main_span(expression_id, keyword_span);

        Ok(expression_id)
    }

    /// Eat one type expression in the current parser scope.
    ///
    /// Examples:
    /// ```ds
    /// Foo.Bar<T>
    /// { a: string, b: number }
    /// T extends U ? X : Y
    /// ```
    pub(crate) fn eat_type_expression(&mut self) -> ParserResult<LocalNodeId<TypeExpression>> {
        if self.flags.is_in_type() {
            return self.eat_type_expression_in_flags(self.flags);
        }

        self.eat_type_expression_in_flags(self.flags.in_type())
    }

    /// Eat one type alias value.
    ///
    /// Examples:
    /// ```ds
    /// intrinsic
    /// string | number
    /// { id: string }
    /// ```
    fn eat_type_alias_value(&mut self) -> ParserResult<LocalNodeId<TypeExpression>> {
        // bare intrinsic marker
        if self.peek_identifier_is() {
            let reference = self.peek()?;
            let is_bare_intrinsic = self.get_span_str(reference.span) == "intrinsic"
                && Self::is_type_expression_boundary_token(self.next_token_type());

            if is_bare_intrinsic {
                self.bump(); // eat intrinsic

                return Ok(self.insert_node(TypeExpression::Intrinsic, reference.span));
            }
        }

        let value = self.eat_type_keyword_body()?;

        // reject optional type suffixes outside tuple and parameter heads
        if self.peek_is(TokenType::Maybe) {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        Ok(value)
    }

    /// Eat one type expression after a type keyword.
    ///
    /// Examples:
    /// ```ds
    /// string | number
    /// readonly T
    /// T extends U ? X : Y
    /// ```
    fn eat_type_keyword_body(&mut self) -> ParserResult<LocalNodeId<TypeExpression>> {
        let mut value_flags = self.flags.not_in_position().in_type();
        if self.flags.is_in_type_conditional_right() {
            value_flags = value_flags.in_type_conditional_right();
        }

        self.with_flags(value_flags, |parser| parser.eat_type_expression())
    }
}
