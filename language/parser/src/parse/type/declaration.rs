use crate::parse::expression::common::DeclarationHeader;
use crate::parse::timing::tags;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{
    Declaration, Keyword, LocalNodeId, Mutability, TokenType, TypeDeclaration, TypeExpression,
    TypeKind,
};

impl Parser {
    /// Return true when the current identifier head starts a type alias.
    fn identifier_starts_type_alias(&mut self) -> bool {
        // require one identifier head first
        if !self.peek_identifier_is() {
            return false;
        }

        // `name = ...`
        if !self.peek_next_is(TokenType::LessThan) {
            return self.is_token_after_newlines(self.pos(), TokenType::Assign);
        }

        // `name<...> = ...`
        let open_index = self.index_for_next() as u32;
        let Some(close_index) = self.find_matching_close_maybe(
            Some(open_index),
            TokenType::LessThan,
            TokenType::GreaterThan,
        ) else {
            return true;
        };

        let follow_index = self.next_non_newline_index_from(close_index as usize + 1);

        self.token_type_at(follow_index) == TokenType::Assign
    }

    /// Eat a type alias or expression.
    ///
    /// Examples:
    /// ```
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
        start: &ParserMark,
        header: DeclarationHeader,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let _timing = self.timing_scope(tags::PARSE_TYPE);

        let keyword_start = self.mark_span();
        let keyword: Keyword =
            self.eat_keyword_in(&[Keyword::Type, Keyword::Readonly, Keyword::Newtype])?;
        let keyword_span = self.get_span_from(&keyword_start);

        // `type` and `readonly` stay structural, `newtype` is nominal
        let kind = match keyword {
            Keyword::Type => TypeKind::Structural,
            Keyword::Readonly => TypeKind::Structural,
            Keyword::Newtype => TypeKind::Nominal,
            _ => unreachable!(),
        };

        // `readonly type` records immutable alias mutability
        let mutability = if keyword == Keyword::Readonly {
            Some(Mutability::Immutable)
        } else {
            None
        };

        // named alias heads commit before we consume the identifier
        if self.identifier_starts_type_alias() {
            let _timing = self.timing_scope(tags::PARSE_TYPE_DECLARATION);

            // alias head
            let (name, name_span) = self.eat_name_with_span()?;
            let generic_parameters = self.eat_generic_parameters_maybe(true)?.unwrap_or_default();

            // `=`
            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::Assign)?;
            self.eat_newlines_maybe()?;

            // aliased type value
            let mut value_options = self.options.not_in_position().in_type();
            if self.options.is_in_type_conditional_right() {
                value_options = value_options.in_type_conditional_right();
            }
            let value_id = self.with_options(value_options, |parser| {
                parser.eat_type_alias_right_hand_side()
            })?;

            // declaration node
            let declaration = Declaration::Type(TypeDeclaration {
                name,
                export: header.export,
                ambient: header.ambient,
                is_nominal: kind == TypeKind::Nominal,
                mutability,
                generic_parameters,
                where_clauses: vec![],
                value: value_id,
            });
            let declaration_id = self.insert_node(declaration, self.get_span_from(start));
            self.tree.set_main_span(declaration_id, name_span);

            return Ok(self.insert_node(
                TypeExpression::Declaration {
                    declaration: declaration_id,
                },
                self.get_span_from(start),
            ));
        }

        // otherwise parse one regular type expression body
        let mut right_options = self.options.not_in_position().in_type();
        if self.options.is_in_type_conditional_right() {
            right_options = right_options.in_type_conditional_right();
        }
        let right = self.with_options(right_options, |parser| parser.eat_type_expression())?;
        let expression_id = if mutability == Some(Mutability::Immutable) {
            let expression_id = self.insert_node(
                TypeExpression::Readonly { target_type: right },
                self.get_span_from(start),
            );
            self.tree.set_main_span(expression_id, keyword_span);
            expression_id
        } else {
            right
        };

        Ok(expression_id)
    }

    /// Eat one type expression in the current parser scope.
    ///
    /// Examples:
    /// ```
    /// Foo.Bar<T>
    /// { a: string, b: number }
    /// value is string
    /// T extends U ? X : Y
    /// ```
    pub(crate) fn eat_type_expression(&mut self) -> ParseResult<LocalNodeId<TypeExpression>> {
        if self.options.is_in_type() {
            return self.eat_type_expression_inner_with_stack_guard();
        }

        self.with_options(self.options.in_type(), |parser| {
            parser.eat_type_expression()
        })
    }

    /// Eat one type alias value.
    fn eat_type_alias_right_hand_side(&mut self) -> ParseResult<LocalNodeId<TypeExpression>> {
        // bare intrinsic marker
        if self.peek_identifier_is() {
            let reference = *self.peek()?;
            let next_index = self.next_non_newline_index_from(self.pos_index().saturating_add(1));
            let is_bare_intrinsic = self.get_span_str(reference.span) == "intrinsic"
                && self.with_pos(next_index, |parser| parser.is_type_expression_boundary());

            if is_bare_intrinsic {
                self.bump(); // eat intrinsic

                return Ok(self.insert_node(TypeExpression::Intrinsic, reference.span));
            }
        }

        // otherwise parse one regular type expression
        self.eat_type_expression()
    }
}
