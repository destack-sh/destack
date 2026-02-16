use super::annotation::PendingDecorators;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserMark};

use destack_ast::{
    Declaration, DeclarationDescriptor, EnumField, EnumKind, Generics, Heritage, Keyword,
    LiteralType, LocalNodeId, Member, Name, NodeType, TemplateLiteral, TokenType,
};
use destack_source::Span;

impl Parser {
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
    pub fn eat_enum(
        &mut self,
        start: &ParserMark,
        kind: EnumKind,
        mut descriptor: DeclarationDescriptor,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let _timing = self.timing_scope(tags::PARSE_ENUM);
        // keyword
        self.eat_keyword(Keyword::Enum)?;

        // optional name
        let name_span = if let Some((name, span)) = self.eat_name_maybe_with_span()? {
            descriptor = descriptor.with_name(name);
            Some(span)
        } else {
            None
        };

        // optional static parameters: < ... >
        let static_parameters = self
            .eat_static_parameters_maybe(false)
            .for_node_type(NodeType::Declaration)?;

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
        self.eat_newlines_maybe()?;
        let (fields, members) = self.eat_enum_body().for_node_type(NodeType::Declaration)?;
        self.eat_token(TokenType::CloseBrace)?;

        let generics = Generics::new(static_parameters, where_clauses);
        let heritage = Heritage::new(extends_types, implements_types);
        let enum_id = self.tree.insert(
            Declaration::Enum {
                descriptor,
                kind,
                generics,
                heritage,
                fields,
                members,
            },
            self.get_span_from(start),
        );

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(enum_id, span);
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
            let cursor = self.sync_to_scanner_cursor();
            let token_type = cursor.token_type;

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
            else if self.is_any_stop() {
                self.eat_any_stop_with_newlines()?;
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
                let member_options = self.options.nested().in_variant();
                let member_result = self.with_options(member_options, |parser| {
                    parser.try_eat_member(TokenType::Newline)
                })?;
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
            && (self.peek_next_is(TokenType::Assign)
                || self.peek_next_is(TokenType::Newline)
                || self.peek_next_is(TokenType::Comma)
                || self.peek_next_is(TokenType::Semicolon)
                || self.peek_next_is(TokenType::CloseBrace));

        is_computed_name || is_bare_name
    }

    /// Eat a single enum field and return it as a UnionField node id.
    fn eat_enum_field(&mut self) -> ParseResult<LocalNodeId<EnumField>> {
        let start = self.mark_span();
        let (name, name_span) = self
            .eat_enum_field_name_with_span()
            .for_node_type(NodeType::EnumField)?;

        // optional `= <expr>` value
        let value = if self.peek_is(TokenType::Assign) {
            self.eat_token(TokenType::Assign)?;
            let value =
                self.eat_expression(self.options.not_in_position().not_in_sequence_expression())?;
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
            let start = self.mark_span();
            self.bump(); // eat open bracket
            self.eat_newlines_maybe()?;

            let name = if self.peek_is(TokenType::Literal)
                && matches!(
                    self.peek()?.token.literal,
                    Some(LiteralType::String { .. } | LiteralType::Character { .. })
                ) {
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

            self.eat_newlines_maybe()?;
            self.eat_token(TokenType::CloseBracket)?;
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
        Annotation, AnnotationPosition, Blank, Comment, CommentStyle, Declaration,
        DeclarationDescriptor, DeclarationKind, Decorator, EnumField, EnumKind, Expression,
        Parameter, ScalarLiteral, WhereClause,
    };

    use crate::{TestParser, assert_node, assert_path, assert_string};

    #[test]
    fn test_parse_enum_with_extends_types() {
        let mut test = TestParser::new(
            r###"
enum Foo extends Day {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let enum_id = parser
            .eat_enum(&start, EnumKind::Enum, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum { descriptor, generics, heritage, fields, members, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");
            assert!(members.is_empty());
            assert!(fields.is_empty());
            assert!(generics.is_empty());

            assert!(heritage.implements_types.is_none());

            let supers = heritage.extends_types.as_ref().expect("expected extends types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let enum_id = parser
            .eat_enum(&start, EnumKind::Enum, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum { descriptor, fields, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert!(descriptor.name.is_none());
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let enum_id = parser
            .eat_enum(&start, EnumKind::Enum, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum { descriptor, fields, generics, heritage, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            // Foo
            assert_string!(parser, descriptor.name.unwrap().string(), "Foo");

            // extends: Day
            assert!(generics.is_empty());

            let supers = heritage.extends_types.as_ref().expect("expected extends types");
            assert_eq!(supers.len(), 1);
            assert_node!(parser.tree, supers[0], Expression::Path { path, .. } => {
                assert_path!(parser, *path, "Day");
            });
            assert!(heritage.implements_types.is_none());

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
        parser.eat_newline().unwrap();
        let start = parser.mark();
        let enum_id = parser
            .eat_enum(&start, EnumKind::Enum, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum { fields, .. } => {
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
    fn test_parse_enum_with_static_parameters() {
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let enum_id = parser
            .eat_enum(&start, EnumKind::Enum, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum { descriptor, generics, fields, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            // Machine
            assert_string!(parser, descriptor.name.unwrap().string(), "Machine");

            // <T: int32 = 3, IsSomething: boolean = true>
            assert!(!generics.is_empty());
            assert!(generics.static_parameters.is_some());
            let static_parameters = generics.static_parameters.as_ref().unwrap();
            assert_eq!(static_parameters.len(), 2);
            // T: int32 = 3
            assert_node!(parser.tree, static_parameters[0], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "T");
            });
            // IsSomething: boolean = true
            assert_node!(parser.tree, static_parameters[1], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "IsSomething");
            });

            assert_eq!(fields.len(), 3);
        });
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let enum_id = parser
            .eat_enum(&start, EnumKind::Enum, DeclarationDescriptor::default())
            .unwrap();
        assert_node!(parser.tree, enum_id, Declaration::Enum { descriptor, generics, fields, .. } => {
            assert_eq!(descriptor.kind, DeclarationKind::Definition);
            assert!(!generics.is_empty());

            // where Requirement: Interface
            let where_clauses = generics.where_clauses.as_ref().expect("expected where clauses");
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Requirement");
                assert_node!(parser.tree, *right, Expression::Path { path, .. } => {
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

        let expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert!(
                parser.tree.get_annotations(declaration_id.id).is_empty(),
                "expected no annotations on enum declaration owner"
            );
            assert_node!(parser.tree, *declaration_id, Declaration::Enum { fields, members, .. } => {
                assert!(fields.is_empty());
                assert!(members.is_empty());
            });
        });

        assert!(
            parser.tree.get_annotations(expression_id.id).is_empty(),
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

        let expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Enum { fields, .. } => {
                assert_eq!(fields.len(), 1);

                let annotations = parser.tree.get_annotations(fields[0].id);
                assert_eq!(annotations.len(), 5);

                assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_eq!(*style, CommentStyle::Slash);
                        assert_string!(parser, *string, "before-first");
                    });
                });
                assert_node!(parser.tree, annotations[1], Annotation::Decorator { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Decorator { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Path { path, .. } => {
                            assert_path!(parser, *path, "first");
                        });
                    });
                });
                assert_node!(parser.tree, annotations[2], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_eq!(*style, CommentStyle::Slash);
                        assert_string!(parser, *string, "between");
                    });
                });
                assert_node!(parser.tree, annotations[3], Annotation::Decorator { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Decorator { expression } => {
                        assert_node!(parser.tree, *expression, Expression::Path { path, .. } => {
                            assert_path!(parser, *path, "second");
                        });
                    });
                });
                assert_node!(parser.tree, annotations[4], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_eq!(*style, CommentStyle::Slash);
                        assert_string!(parser, *string, "before-name");
                    });
                });
            });
        });
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

        let expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Enum { fields, .. } => {
                assert_eq!(fields.len(), 2);

                let first_annotations = parser.tree.get_annotations(fields[0].id);
                assert_eq!(first_annotations.len(), 1);
                assert_node!(parser.tree, first_annotations[0], Annotation::Comment { node, position } => {
                    assert_eq!(*position, AnnotationPosition::LinePostfixBoundary);
                    assert_node!(parser.tree, *node, Comment { string, style } => {
                        assert_eq!(*style, CommentStyle::Slash);
                        assert_string!(parser, *string, "a-tail");
                    });
                });

                let second_annotations = parser.tree.get_annotations(fields[1].id);
                assert_eq!(second_annotations.len(), 2);
                assert_node!(parser.tree, second_annotations[0], Annotation::Blank { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPrefix);
                    assert_node!(parser.tree, *node, Blank { lines } => {
                        assert_eq!(*lines, 1);
                    });
                });
                assert_node!(parser.tree, second_annotations[1], Annotation::Blank { node, position } => {
                    assert_eq!(*position, AnnotationPosition::BlockPostfix);
                    assert_node!(parser.tree, *node, Blank { lines } => {
                        assert_eq!(*lines, 1);
                    });
                });
            });
        });
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

        let expression_id = parser.unwrap_statement_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(_declaration_id) => {
            let annotations = parser.tree.get_annotations(expression_id.id);
            assert_eq!(annotations.len(), 1);
            assert_node!(parser.tree, annotations[0], Annotation::Comment { node, position } => {
                assert_eq!(*position, AnnotationPosition::LinePrefix);
                assert_node!(parser.tree, *node, Comment { string, style } => {
                    assert_eq!(*style, CommentStyle::Star);
                    assert_string!(parser, *string, "enum-body");
                });
            });
        });
    }
}
