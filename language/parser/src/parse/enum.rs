use super::PendingDecorators;
use crate::parse::expression::common::DeclarationHeader;
use crate::parse::parser::ParserFlags;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserSpanStart};

use destack_ast::{
    Declaration, EnumDeclaration, EnumField, EnumKind, Keyword, LiteralType, LocalNodeId, Member,
    Name, NodeType, TemplateLiteral, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

impl Parser {
    /// Return parser contexts for enum members.
    #[inline]
    fn enum_member_contexts(&self) -> (ParserFlags, ParserFlags) {
        let ambient_context = self.flags.nested().with_variant(true);
        let expression_context = self.flags.nested();
        (ambient_context, expression_context)
    }

    /// Eat an enum declaration.
    ///
    /// Examples:
    /// ```
    /// // anonymous enum (for use as a value)
    /// enum { Success, Failure }
    ///
    /// enum _ {} // explicit anonymous enum (for disambiguation)
    ///
    /// enum Foo {
    ///     A // semicolon optional
    ///     B
    ///     C
    ///
    ///     function myFunc() { // nested declaration
    ///     }
    /// }
    ///
    /// enum Foo {
    ///     Baz = 1
    ///     Qux = 2
    /// }
    ///
    /// enum Machine<T: int32 = 3, IsSomething: boolean = true> {
    ///     A = 1
    ///     B = T
    ///     @if(IsSomething)
    ///     C = 3
    /// }
    /// ```
    pub(crate) fn eat_enum(
        &mut self,
        start: &ParserSpanStart,
        kind: EnumKind,
        header: DeclarationHeader,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        // keyword
        self.eat_keyword(Keyword::Enum)?;

        // optional name
        let (name, name_span) = if let Some((name, span)) = self.eat_name_maybe_with_span()? {
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

        // optional extends types
        let extends_types = self
            .eat_extends_types_maybe()
            .for_node_type(NodeType::Declaration)?;

        // optional implements types
        let implements_types = self
            .eat_implements_types_maybe()
            .for_node_type(NodeType::Declaration)?;

        // where
        let where_clauses = self
            .eat_where_maybe()
            .for_node_type(NodeType::Declaration)?;

        // body
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;
        let (fields, members) = self.eat_enum_body().for_node_type(NodeType::Declaration)?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        let enum_id = self.insert_node(
            Declaration::Enum(EnumDeclaration {
                name,
                export: header.export,
                ambient: header.ambient,
                kind,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses: where_clauses.unwrap_or_default(),
                implements_types: implements_types.or(extends_types).unwrap_or_default(),
                fields,
                members,
            }),
            self.get_span_from(start),
        );

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(enum_id, span);
        }
        if let Some(span) = generic_parameter_container_span {
            self.tree.set_side_span(
                enum_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                span,
            );
        }

        Ok(enum_id)
    }

    /// Eat an enum body (without the header or `{` and `}`)
    #[allow(clippy::type_complexity)]
    fn eat_enum_body(
        &mut self,
    ) -> ParseResult<(Vec<LocalNodeId<EnumField>>, Vec<LocalNodeId<Member>>)> {
        // eat everything
        let mut fields: Vec<LocalNodeId<EnumField>> = Vec::new();
        let mut members: Vec<LocalNodeId<Member>> = Vec::new();
        let mut pending_decorators = PendingDecorators::new();

        while self.has_more_tokens() {
            let token_type = self.peek_token_type();

            // stop on closing brace
            if token_type == TokenType::CloseBrace {
                if !pending_decorators.is_empty() {
                    let error = ParseError::unexpected(self.peek()?.span);
                    self.error(&error);
                    pending_decorators.clear();
                }

                break;
            }
            // consume any stop
            else if Self::is_any_stop_token(token_type) {
                self.eat_any_stop()?;
            }
            // consume decorator prefixes
            else if token_type == TokenType::At {
                let decorators = self.eat_decorators_maybe()?;
                pending_decorators.extend(decorators);
            }
            // enum field
            else if self.peek_enum_field_is() {
                let field = self.eat_enum_field().for_node_type(NodeType::EnumField)?;
                if !pending_decorators.is_empty() {
                    self.attach_decorators(field.id, std::mem::take(&mut pending_decorators));
                }
                fields.push(field);
            }
            // (static) members
            else {
                let (member_ambient_context, member_expression_context) =
                    self.enum_member_contexts();
                let member_result = self.with_flags(
                    self.flags
                        .with_ambient_context(member_ambient_context)
                        .with_expression_context(member_expression_context),
                    |parser| parser.try_eat_member(),
                )?;
                let member_id = member_result;
                if !pending_decorators.is_empty() {
                    self.attach_decorators(member_id.id, std::mem::take(&mut pending_decorators));
                }
                members.push(member_id);
            }
        }

        Ok((fields, members))
    }

    /// Return true when the next tokens can start an enum field.
    #[inline]
    fn peek_enum_field_is(&mut self) -> bool {
        let is_computed_name = self.peek_is(TokenType::OpenBracket);
        let is_bare_name = (self.peek_name_is() || self.peek_numeric_literal_is())
            && (self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::Assign)
            }) || self.lookahead(|parser| {
                parser.bump();
                parser.current_token_is_on_new_line()
            }) || self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::Comma)
            }) || self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::Semicolon)
            }) || self.lookahead(|parser| {
                parser.bump();
                parser.peek_is(TokenType::CloseBrace)
            }));

        is_computed_name || is_bare_name
    }

    /// Eat a single enum field and return it as a UnionField node id.
    fn eat_enum_field(&mut self) -> ParseResult<LocalNodeId<EnumField>> {
        let start = self.span_start();
        let (name, name_span) = self
            .eat_enum_field_name_with_span()
            .for_node_type(NodeType::EnumField)?;

        // optional `= <expr>` value
        let value = if self.peek_is(TokenType::Assign) {
            self.eat_token(TokenType::Assign)?;
            let value = self.eat_expression_or_recover_missing(
                self.flags.not_in_position().not_in_sequence_expression(),
                NodeType::EnumField,
            )?;
            Some(value)
        } else {
            None
        };

        let field_id = self
            .tree
            .insert(EnumField { name, value }, self.get_span_from(&start));

        // set main span to the name identifier
        self.tree.set_main_span(field_id, name_span);
        Ok(field_id)
    }

    /// Eat an enum field name, including computed string/number names.
    fn eat_enum_field_name_with_span(&mut self) -> ParseResult<(Name, Span)> {
        if self.peek_is(TokenType::OpenBracket) {
            let start = self.span_start();
            self.bump(); // eat open bracket

            let name = if self.peek_is(TokenType::Literal)
                && matches!(self.peek()?.token.literal, Some(LiteralType::String { .. }))
            {
                let token = *self.peek()?;
                let content = self.get_string_literal_str(token).to_owned();
                let string_id = self.strings.intern(&content);
                self.bump();
                Name::String(string_id)
            } else if self.peek_numeric_literal_is() {
                let token = *self.peek_numeric_literal()?;
                let key_str = self.file.span_str(token.span);
                let string_id = self.strings.intern(key_str);
                self.bump();
                Name::Number(string_id)
            } else if self.peek_is(TokenType::TemplateString) {
                let template = self.eat_template_literal()?;
                match template {
                    TemplateLiteral::String { string } => Name::String(string),
                    TemplateLiteral::InterpolatedString { .. } => {
                        return Err(ParseError::unexpected(self.peek()?.span));
                    }
                }
            } else {
                return Err(ParseError::unexpected(self.peek()?.span));
            };

            self.eat_close_token_or_recover_missing(TokenType::CloseBracket, NodeType::Expression)?;
            Ok((name, self.get_span_from(&start)))
        } else if self.peek_numeric_literal_is() {
            let token = *self.peek_numeric_literal()?;
            let key_str = self.file.span_str(token.span);
            let string_id = self.strings.intern(key_str);
            self.bump();
            Ok((Name::Number(string_id), token.span))
        } else {
            self.eat_name_with_span()
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        CommentKind, Declaration, Decorator, DecoratorPosition, EnumDeclaration, EnumField,
        EnumKind, Expression, GenericParameter, ScalarLiteral, TypeExpression, WhereClause,
    };
    use destack_source::{NodeSpanRegion, NodeSpanType};

    use crate::parse::expression::common::DeclarationHeader;
    use crate::{TestParser, assert_comment, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_enum_with_extends_types() {
        let mut test = TestParser::new(
            r###"
enum Foo extends Day {}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let enum_id = parser
            .eat_enum(&start, EnumKind::Enum, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum(EnumDeclaration { name, generic_parameters, implements_types, fields, members, .. }) => {
            assert_string!(parser, name.unwrap().string(), "Foo");
            assert!(members.is_empty());
            assert!(fields.is_empty());
            assert!(generic_parameters.is_empty());
            assert_eq!(implements_types.len(), 1);
            assert_node!(parser.tree, implements_types[0], TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Day");
            });
        });
    }

    #[test]
    fn test_parse_enum_anonymous_simple() {
        let mut test = TestParser::new(
            r###"
enum {
    Success
    Failure
}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let enum_id = parser
            .eat_enum(&start, EnumKind::Enum, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum(EnumDeclaration { name, fields, .. }) => {
            assert!(name.is_none());
            assert_eq!(fields.len(), 2);
            // Success
            assert_node!(parser.tree, fields[0], EnumField { name, value } => {
                assert_string!(parser, name.string(), "Success");
                assert!(value.is_none());
            });
            // Failure
            assert_node!(parser.tree, fields[1], EnumField { name, value } => {
                assert_string!(parser, name.string(), "Failure");
                assert!(value.is_none());
            });
        });
    }

    #[test]
    fn test_parse_enum_with_type_name_and_values() {
        let mut test = TestParser::new(
            r###"
enum Foo extends Day {

    Baz = 1

    Qux = 2
}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let enum_id = parser
            .eat_enum(&start, EnumKind::Enum, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum(EnumDeclaration { name, fields, generic_parameters, implements_types, .. }) => {
            // Foo
            assert_string!(parser, name.unwrap().string(), "Foo");

            // extends: Day
            assert!(generic_parameters.is_empty());
            assert_eq!(implements_types.len(), 1);
            assert_node!(parser.tree, implements_types[0], TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Day");
            });

            assert_eq!(fields.len(), 2);

            // Baz = 1
            assert_node!(parser.tree, fields[0], EnumField { name, value } => {
                assert_string!(parser, name.string(), "Baz");
                assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });

            // Qux = 2
            assert_node!(parser.tree, fields[1], EnumField { name, value } => {
                assert_string!(parser, name.string(), "Qux");
                assert_node!(parser.tree, value.unwrap(), Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
        });
    }

    /// Computed string enum keys should parse as string names.
    #[test]
    fn test_parse_enum_computed_string_names() {
        let mut test = TestParser::new(
            r###"
enum CHAR {
    ['\v'] = 0x0B,
    ["\f"] = 0x0C,
    [`\r`] = 0x0D,
}
"###,
        );
        let mut parser = test.prepare();
        let start = parser.span_start();
        let enum_id = parser
            .eat_enum(&start, EnumKind::Enum, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum(EnumDeclaration { fields, .. }) => {
            assert_eq!(fields.len(), 3);
            assert_node!(parser.tree, fields[0], EnumField { name, value } => {
                assert_string!(parser, name.string(), "\\v");
                assert!(value.is_some());
            });
            assert_node!(parser.tree, fields[1], EnumField { name, value } => {
                assert_string!(parser, name.string(), "\\f");
                assert!(value.is_some());
            });
            assert_node!(parser.tree, fields[2], EnumField { name, value } => {
                assert_string!(parser, name.string(), "\\r");
                assert!(value.is_some());
            });
        });
    }

    #[test]
    fn test_parse_enum_with_generic_parameters() {
        let mut test = TestParser::new(
            r###"
enum Machine<T: int32 = 3, IsSomething: boolean = true> {
    A = 1
    B = T
    @if(IsSomething)
    C = 3
}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let enum_id = parser
            .eat_enum(&start, EnumKind::Enum, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum(EnumDeclaration { name, generic_parameters, fields, .. }) => {
            // Machine
            assert_string!(parser, name.unwrap().string(), "Machine");

            // <T: int32 = 3, IsSomething: boolean = true>
            assert_eq!(generic_parameters.len(), 2);
            // T: int32 = 3
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, .. } => {
                assert_string!(parser, *name, "T");
            });
            // IsSomething: boolean = true
            assert_node!(parser.tree, generic_parameters[1], GenericParameter::Type { name, .. } => {
                assert_string!(parser, *name, "IsSomething");
            });

            assert_eq!(fields.len(), 3);
        });

        let generic_parameter_span = parser
            .tree
            .get_side_span(
                enum_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
            )
            .expect("missing enum generic parameter span");
        assert_eq!(
            parser.get_span_str(generic_parameter_span),
            "<T: int32 = 3, IsSomething: boolean = true>"
        );
    }

    #[test]
    fn test_parse_enum_with_where_clause() {
        let mut test = TestParser::new(
            r###"
enum Foo where Requirement: Interface {
    Value
}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let enum_id = parser
            .eat_enum(&start, EnumKind::Enum, DeclarationHeader::default())
            .unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum(EnumDeclaration { where_clauses, fields, .. }) => {
            assert_eq!(where_clauses.len(), 1);

            // where Requirement: Interface
            assert_node!(parser.tree, where_clauses[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Requirement");
                assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "Interface");
                });
            });

            assert_eq!(fields.len(), 1);
        });
    }

    #[test]
    fn test_parse_enum_with_dangling_item_decorator_reports_error_and_no_attachment() {
        let mut test = TestParser::new(
            r###"
enum Value {
    @dangling
}
"###,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 1);
        assert_eq!(
            parser.errors.len(),
            1,
            "expected one dangling decorator parse error"
        );
        assert_eq!(parser.file.span_str(parser.errors[0].leaf_span()), "}");

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert!(
                parser.tree.get_decorators(declaration_id.id).is_empty(),
                "expected no annotations on enum declaration owner"
            );
            assert_node!(parser.tree, *declaration_id, Declaration::Enum(EnumDeclaration { fields, members, .. }) => {
                assert!(fields.is_empty());
                assert!(members.is_empty());
            });
        });

        assert!(
            parser.tree.get_decorators(expression_id.id).is_empty(),
            "expected no attached annotation nodes for dangling decorator"
        );
    }

    #[test]
    fn test_parse_enum_field_interleaved_comments_and_decorators() {
        let mut test = TestParser::new(
            r#"enum Value {
// before-first
@first
// between
@second
// before-name
Entry
}"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Enum(EnumDeclaration { fields, .. }) => {
                assert_eq!(fields.len(), 1);

                let annotations = parser.tree.get_decorators(fields[0].id);
                assert_eq!(annotations.len(), 2);

                assert_node!(parser.tree, annotations[0], Decorator { expression, position } => {
                    assert_eq!(*position, DecoratorPosition::BlockPrefix);
                        assert_node!(parser.tree, *expression, Expression::Identifier { name } => {
                            assert_string!(parser, *name, "first");
                        });
                });
                assert_node!(parser.tree, annotations[1], Decorator { expression, position } => {
                    assert_eq!(*position, DecoratorPosition::BlockPrefix);
                        assert_node!(parser.tree, *expression, Expression::Identifier { name } => {
                            assert_string!(parser, *name, "second");
                        });
                });
            });
        });
        assert_eq!(parser.tree.comments().len(), 3);
        assert_comment!(parser, 0, CommentKind::Line, "before-first");
        assert_comment!(parser, 1, CommentKind::Line, "between");
        assert_comment!(parser, 2, CommentKind::Line, "before-name");
    }

    #[test]
    fn test_parse_enum_field_trailing_and_blank_seams() {
        let mut test = TestParser::new(
            r#"enum Value {
A // a-tail

B

}"#,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Enum(EnumDeclaration { fields, .. }) => {
                assert_eq!(fields.len(), 2);

                let first_annotations = parser.tree.get_decorators(fields[0].id);
                assert!(first_annotations.is_empty());

                let second_annotations = parser.tree.get_decorators(fields[1].id);
                assert!(second_annotations.is_empty());
            });
        });
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "a-tail");
    }

    #[test]
    fn test_parse_enum_body_boundary_comment_on_declaration_owner() {
        let mut test = TestParser::new("enum Value /* enum-body */ { A }");
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(_declaration_id) => {
            let annotations = parser.tree.get_decorators(expression_id.id);
            assert!(annotations.is_empty());
        });
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::SingleLineBlock, " enum-body");
    }
}
