use crate::parse::expression::common::DeclarationHeader;
use crate::parse::timing::tags;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{
    Declaration, Expression, Keyword, LocalNodeId, Mutability, TokenType, TypeDeclaration,
    TypeExpression, TypeKind,
};

impl Parser {
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
    ) -> ParseResult<LocalNodeId<Expression>> {
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

        // alias, or expression with generic parameters
        if self.peek_identifier_is()
            && (self.peek_next_is(TokenType::LessThan)
                || self.is_token_after_newlines(self.pos(), TokenType::Assign))
        {
            // identifier
            // (speculative because we don't know yet if we'll have a `=` afterwards)
            let speculative_start = (self.mark(), self.tree.next_id());
            let (name, name_span) = if let Some((n, s)) = self.eat_name_maybe_with_span()? {
                (Some(n), Some(s))
            } else {
                (None, None)
            };
            let static_parameter_open = if self.peek_is(TokenType::LessThan) {
                Some(self.pos())
            } else {
                None
            };

            // generic parameters
            // speculative: may fail for type expressions like Foo<T[number]>
            let generic_parameters = match self.eat_generic_parameters_maybe(true) {
                Ok(params) => params,
                Err(error) => {
                    // keep hard failures for incomplete type parameter lists
                    let has_matching_type_parameter_close =
                        static_parameter_open.is_some_and(|open_pos| {
                            self.find_matching_close_maybe(
                                Some(open_pos),
                                TokenType::LessThan,
                                TokenType::GreaterThan,
                            )
                            .is_some()
                        });
                    if !has_matching_type_parameter_close {
                        return Err(error);
                    }

                    // otherwise restore and fall through to type expressions
                    self.restore(speculative_start.0.clone(), speculative_start.1);
                    None
                }
            };

            // if followed by =, then it's a type alias
            if self.peek_is(TokenType::Assign)
                || self.is_token_after_newlines(self.pos(), TokenType::Assign)
            {
                let _timing = self.timing_scope(tags::PARSE_TYPE_DECLARATION);
                // =
                self.eat_newlines_maybe()?;
                self.eat_token(TokenType::Assign)?;
                self.eat_newlines_maybe()?;

                // aliased type value
                let name = name.expect("type declarations require a name here");
                let name_span = name_span.expect("type declarations require a name span here");
                let mut value_options = self.options.not_in_position().in_type();
                if self.options.is_in_type_conditional_right() {
                    value_options = value_options.in_type_conditional_right();
                }
                let value_id =
                    self.with_options(value_options, |parser| parser.eat_type_alias_value())?;

                // type declaration wrapped in expression
                let declaration = Declaration::Type(TypeDeclaration {
                    name,
                    export: header.export,
                    ambient: header.ambient,
                    is_nominal: kind == TypeKind::Nominal,
                    mutability,
                    generic_parameters: generic_parameters.unwrap_or_default(),
                    where_clauses: vec![],
                    value: value_id,
                });
                let declaration_id = self.insert_node(declaration, self.get_span_from(start));

                // set main span to the name identifier
                self.tree.set_main_span(declaration_id, name_span);

                let expression = Expression::Declaration(declaration_id);
                Ok(self.insert_node(expression, self.get_span_from(start)))
            }
            // otherwise it's a type expression with generic arguments
            else {
                // re-parse from before the name to get generic arguments properly
                self.restore(speculative_start.0.clone(), speculative_start.1);
                let mut right_options = self.options.not_in_position().in_type();
                if self.options.is_in_type_conditional_right() {
                    right_options = right_options.in_type_conditional_right();
                }
                let right =
                    self.with_options(right_options, |parser| parser.eat_type_expression())?;
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

                Ok(self.insert_type_expression_value(expression_id))
            }
        }
        // type expression
        else {
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

            Ok(self.insert_type_expression_value(expression_id))
        }
    }

    /// Eat one type expression in the current parser scope.
    pub(crate) fn eat_type_expression(&mut self) -> ParseResult<LocalNodeId<TypeExpression>> {
        let expression_id = self.eat_expression(self.options)?;
        self.expect_type_expression_value(expression_id)
    }

    /// Eat one type alias value.
    fn eat_type_alias_value(&mut self) -> ParseResult<LocalNodeId<TypeExpression>> {
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

        self.eat_type_expression()
    }
}
