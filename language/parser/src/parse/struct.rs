#![allow(clippy::type_complexity)]

use crate::parse::expression::common::DeclarationHeader;
use crate::parse::prelude::*;
use crate::{ParseError, ParseResult, Parser, ParserSpanStart};

use destack_dir::{
    ClassDeclaration, Declaration, Keyword, LocalNodeId, NodeType, StructDeclaration, TokenType,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

impl Parser {
    /// Eat a struct or class declaration.
    ///
    /// The parser accepts `extends` for classes and `implements` for structs.
    /// Class declarations treat `extends` as one superclass expression.
    /// Struct declarations require a name.
    ///
    /// Struct forms:
    /// ```
    /// struct Bar {
    ///     myField: int32;
    ///     myOtherField: boolean;
    /// };
    ///
    /// struct Foo<T> implements Drawable { // structs can implement interfaces
    ///     myField: int32;
    ///     myOtherField: T;
    ///
    ///     static x: int32 = 7; // constant
    ///
    ///     myFunc() { }
    /// };
    /// ```
    ///
    /// Class forms:
    /// ```
    /// class Foo extends Bar { // classes can extend
    ///     myField: int32;
    /// };
    /// ```
    pub(crate) fn eat_struct_or_class(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
        allow_anonymous_class: bool,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        // keyword
        let keyword = self
            .eat_keyword_in(&[Keyword::Struct, Keyword::Class])
            .for_node_type(NodeType::Declaration)?;
        let is_class = keyword == Keyword::Class;

        // optional name / key
        let has_heritage_keyword =
            self.is_keyword(Keyword::Extends) || self.is_keyword(Keyword::Implements);

        // require a name for class and struct declarations
        let allow_anonymous = is_class && allow_anonymous_class;
        let (name, name_span) = if !allow_anonymous {
            let (name, span) = self.eat_name_with_span()?;
            (Some(name), Some(span))
        } else if has_heritage_keyword {
            (None, None)
        } else if let Some((name, span)) = self.eat_name_maybe_with_span()? {
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

        // optional extends clause
        let unexpected_extends_span = if !is_class && self.is_keyword(Keyword::Extends) {
            Some(self.peek()?.span)
        } else {
            None
        };
        let extends_clause = if is_class {
            self.eat_extends_expressions_maybe()
                .for_node_type(NodeType::Declaration)?
        } else if unexpected_extends_span.is_some() {
            self.eat_extends_types_maybe()
                .for_node_type(NodeType::Declaration)?;
            None
        } else {
            None
        };
        if let Some(span) = unexpected_extends_span {
            self.error(&ParseError::unexpected_for(span, NodeType::Declaration));
        }

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
        let member_flags = self.flags.nested().in_variant();
        let members = self.with_flags(member_flags, |parser| parser.eat_members(false))?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        // struct or class
        let declaration = if is_class {
            let extends_clause = extends_clause.and_then(|mut expressions| {
                if expressions.is_empty() {
                    None
                } else {
                    Some(expressions.remove(0))
                }
            });
            let (extends_expression, extends_generic_arguments) =
                if let Some(expression_id) = extends_clause {
                    let (extends_expression, extends_generic_arguments) =
                        self.split_instantiation_expression(expression_id);
                    (Some(extends_expression), extends_generic_arguments)
                } else {
                    (None, vec![])
                };

            Declaration::Class(ClassDeclaration {
                name,
                export: header.export,
                is_ambient: header.is_ambient,
                is_abstract: header.is_abstract,
                is_final: header.is_final,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses: where_clauses.unwrap_or_default(),
                extends_expression,
                extends_generic_arguments,
                implements_types: implements_types.unwrap_or_default(),
                members,
            })
        } else {
            Declaration::Struct(StructDeclaration {
                name: name.expect("structs require a name here"),
                export: header.export,
                is_ambient: header.is_ambient,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses: where_clauses.unwrap_or_default(),
                implements_types: implements_types.unwrap_or_default(),
                members,
            })
        };
        let declaration_id = self.insert_node(declaration, self.get_span_from(start));

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(declaration_id, span);
        }
        if let Some(span) = generic_parameter_container_span {
            self.tree.set_side_span(
                declaration_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
                span,
            );
        }

        Ok(declaration_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_dir::{
        BinaryOperator, ClassDeclaration, CommentKind, Declaration, Expression, GenericParameter,
        IntegerType, Key, Member, Name, NodeType, Parameter, ScalarLiteral, StructDeclaration,
        TypeExpression, TypeLiteral, Visibility, WhereClause,
    };
    use destack_source::{LanguageType, NodeSpanRegion, NodeSpanType};

    use crate::parse::expression::common::DeclarationHeader;
    use crate::{
        ParserOptions, TestParser, assert_comment, assert_expression_path, assert_node,
        assert_path, assert_string,
    };

    #[test]
    fn test_parse_struct_requires_name() {
        let mut test = TestParser::new(
            r###"
struct { public x: int32, readonly y: boolean }
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let result = parser.eat_struct_or_class(&start, DeclarationHeader::default(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_class_requires_name_in_statement_position() {
        let mut test = TestParser::new(
            r###"
class {}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let result = parser.eat_struct_or_class(&start, DeclarationHeader::default(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_class_rejects_comma_separated_members() {
        // source: class Foo { x: int32, y: int32 }
        let mut test = TestParser::new(
            r###"
class Foo { x: int32, y: int32 }
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let result = parser.eat_struct_or_class(&start, DeclarationHeader::default(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_struct_rejects_comma_separated_members() {
        // source: struct Foo { x: int32, y: int32 }
        let mut test = TestParser::new(
            r###"
struct Foo { x: int32, y: int32 }
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let result = parser.eat_struct_or_class(&start, DeclarationHeader::default(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_struct_rejects_extends() {
        let mut test = TestParser::new(
            r###"
struct Foo extends Bar implements Baz {
    value: int32;
}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();

        assert_eq!(parser.errors.len(), 1);
        assert_eq!(parser.get_span_str(parser.errors[0].leaf_span()), "extends");
        assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { implements_types, members, .. }) => {
            assert_eq!(implements_types.len(), 1);
            assert_eq!(members.len(), 1);
        });
    }

    #[test]
    fn test_parse_class_with_parenthesized_binary_extends_expression() {
        let mut test =
            TestParser::new_with_language("class A extends (a + b) {}", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let start = parser.span_start();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { extends_expression: Some(extends_expression), .. }) => {
            assert_node!(parser.tree, *extends_expression, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Binary { operator, .. } => {
                    assert_eq!(*operator, BinaryOperator::Add);
                });
            });
        });
    }

    #[test]
    fn test_parse_class_with_parenthesized_sequence_extends_expression() {
        let mut test =
            TestParser::new_with_language("class A extends (a, b) {}", LanguageType::TypeScript);
        let mut parser = test.prepare();
        parser.apply_options(ParserOptions {
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        });

        let start = parser.span_start();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { extends_expression: Some(extends_expression), .. }) => {
            assert_node!(parser.tree, *extends_expression, Expression::SequenceExpression { expressions } => {
                assert_eq!(expressions.len(), 2);
            });
        });
    }

    #[test]
    fn test_reject_class_with_unparenthesized_as_extends_expression() {
        let mut parser = TestParser::new_with_language(
            "class A extends Base as Mixin {}",
            LanguageType::TypeScript,
        )
        .prepare();

        let start = parser.span_start();
        let error = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap_err();

        let (span, node_type, expected) = error.leaf_content();
        assert_eq!(node_type, Some(NodeType::Declaration));
        assert_eq!(expected, None);
        assert_eq!(parser.get_span_str(span), "as");
    }

    #[test]
    fn test_parse_class_with_parenthesized_as_extends_expression() {
        let mut test = TestParser::new_with_language(
            "class A extends (Base as Mixin) {}",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();

        assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { extends_expression: Some(extends_expression), .. }) => {
            assert_node!(parser.tree, *extends_expression, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::As { expression, target_type } => {
                    assert_expression_path!(parser, parser.tree.get(*expression), "Base");
                    assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
                        assert!(generic_arguments.is_empty());
                        assert_path!(parser, *path, "Mixin");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_class_keeps_unparenthesized_decorated_extends_head_unwrapped() {
        let mut test = TestParser::new_with_language(
            "class Outer extends\n@deco\nclass {} {}",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { extends_expression: Some(extends_expression), .. }) => {
            assert_node!(parser.tree, *extends_expression, Expression::Declaration(declaration_id) => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { .. }));
            });
        });
    }

    #[test]
    fn test_parse_class_keeps_parenthesized_decorated_extends_head_parenthesized() {
        let mut test = TestParser::new_with_language(
            "class Outer extends (@deco class Base {}) {}",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { extends_expression: Some(extends_expression), .. }) => {
            assert_node!(parser.tree, *extends_expression, Expression::Parenthesized { expression } => {
                assert_node!(parser.tree, *expression, Expression::Declaration(declaration_id) => {
                    assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { name, .. }) => {
                        assert_string!(parser, name.expect("expected class name").string(), "Base");
                    });
                });
            });
        });
    }

    #[test]
    fn test_parse_class_with_multiple_extends_for_lineage_validation() {
        let mut test = TestParser::new(
            r###"
class Combined extends First, Second {}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { name, extends_expression: Some(extends_expression), .. }) => {
            assert_string!(parser, name.expect("expected class name").string(), "Combined");
            assert_expression_path!(parser, parser.tree.get(*extends_expression), "First");
        });
    }

    #[test]
    fn test_parse_class_with_empty_extends_for_lineage_validation() {
        let mut test = TestParser::new(
            r###"
class Counter extends {}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { name, extends_expression, .. }) => {
            assert_string!(parser, name.expect("expected class name").string(), "Counter");
            assert!(extends_expression.is_none());
        });
    }

    #[test]
    fn test_parse_class_member_method_parameter_type_then_default_value() {
        let mut test = TestParser::new_with_language(
            r#"class LicensingStore {
  usersLimitReached(userCount: number, userLimit = get(this.store).userLimit) {
    return userCount >= userLimit
  }
}"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();

        // parse one class method that mixes typed and defaulted parameters
        assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { members, .. }) => {
            assert_eq!(members.len(), 1);

            // usersLimitReached(userCount: number, userLimit = get(this.store).userLimit)
            assert_node!(parser.tree, members[0], Member::Method { key: Some(Key::Name(Name::Identifier(name))), signature, body: Some(_), .. } => {
                assert_string!(parser, *name, "usersLimitReached");
                assert_eq!(signature.parameters.len(), 2);
                assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(ty), default, .. } => {
                    assert_string!(parser, *name, "userCount");
                    assert!(default.is_none());
                    assert_node!(parser.tree, *ty, TypeExpression::Literal { value } => {
                        assert_eq!(*value, TypeLiteral::Number);
                    });
                });
                assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, declared_type, default: Some(_), .. } => {
                    assert_string!(parser, *name, "userLimit");
                    assert!(declared_type.is_none());
                });
            });
        });

        // this class parses without recovery diagnostics
        test.assert_no_errors(&parser);
    }

    #[test]
    fn test_parse_class_superclass_boundary_comment_on_super_type() {
        let mut test = TestParser::new_with_language(
            r"class Child extends Base // extends-tail
{
  value = 1
}",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert_eq!(expressions.len(), 1);

        let expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { extends_expression: Some(extends_expression), members, .. }) => {
                assert_eq!(members.len(), 1);
                let extends_annotations = parser.tree.get_decorators(extends_expression.id);
                assert!(extends_annotations.is_empty());

                let member_annotations = parser.tree.get_decorators(members[0].id);
                assert!(member_annotations.is_empty());
            });
        });
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "extends-tail");

        test.assert_no_errors(&parser);
    }

    #[test]
    fn test_parse_class_implement_list_comments_on_interface_types() {
        let mut test = TestParser::new_with_language(
            r"class Child implements First, // impl-first
Second // impl-second
{
  value = 1
}",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { implements_types, members, .. }) => {
                assert_eq!(implements_types.len(), 2);
                assert_eq!(members.len(), 1);

                let first_annotations = parser.tree.get_decorators(implements_types[0].id);
                assert!(first_annotations.is_empty());

                let second_annotations = parser.tree.get_decorators(implements_types[1].id);
                assert!(second_annotations.is_empty());

                let member_annotations = parser.tree.get_decorators(members[0].id);
                assert!(member_annotations.is_empty());
            });
        });
        assert_eq!(parser.tree.comments().len(), 2);
        assert_comment!(parser, 0, CommentKind::Line, "impl-first");
        assert_comment!(parser, 1, CommentKind::Line, "impl-second");
    }

    #[test]
    fn test_parse_declare_class_head_comment_before_generics_on_declaration_owner() {
        let mut test = TestParser::new_with_language(
            r"declare class Box // box-head
<T> implements Item<T>, Other {
  value: T
}",
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();
        let expressions = parser.parse();

        assert!(
            parser.errors.is_empty(),
            "unexpected parser errors: {:?}",
            parser.errors
        );
        assert_eq!(expressions.len(), 1);

        let expression_id = parser.unwrap_label_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            let first_static_parameter_id = assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { generic_parameters, .. }) => {
                assert_eq!(generic_parameters.len(), 1);
                generic_parameters[0]
            });

            let annotations = parser.tree.get_decorators(first_static_parameter_id.id);
            assert!(annotations.is_empty());
        });
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "box-head");
    }

    #[test]
    fn test_parse_struct_with_implements() {
        let mut test = TestParser::new(
            r###"
struct Foo<T: Numeric> implements Quux {
    a: T
    b?: T
    c: T
    private d: int32 = 4
}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { name, generic_parameters, implements_types, members, .. }) => {
            assert_string!(parser, name.string(), "Foo");
            assert!(!generic_parameters.is_empty());

            // T: Numeric
            assert_eq!(generic_parameters.len(), 1);
            assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, .. } => {
                assert_string!(parser, *name, "T");
                let constraint = constraint.expect("expected constraint");
                assert_node!(parser.tree, constraint, TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "Numeric");
                });
            });

            assert_eq!(implements_types.len(), 1);
            assert_node!(parser.tree, implements_types[0], TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Quux");
            });

            assert_eq!(members.len(), 4);

            // a: T
            assert_node!(parser.tree, members[0], Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: Some(ty), default: None, .. } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *ty, TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            // b?: T
            assert_node!(parser.tree, members[1], Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: Some(ty), is_optional, default: None, .. } => {
                assert!(*is_optional);
                assert_string!(parser, *name, "b");
                assert_expression_path!(parser, parser.tree.get(*ty), "T");
            });
            // c: T
            assert_node!(parser.tree, members[2], Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: Some(ty), default: None, .. } => {
                assert_string!(parser, *name, "c");
                assert_node!(parser.tree, *ty, TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            // private d: int32 = 4
            assert_node!(parser.tree, members[3], Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: Some(ty), default: Some(value), visibility, .. } => {
                assert_eq!(*visibility, Some(Visibility::Private));
                assert_string!(parser, *name, "d");
                assert_node!(parser.tree, *ty, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                    }));
                });
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
            });
        });
    }

    #[test]
    fn test_parse_class_records_generic_parameter_container_span() {
        let mut test = TestParser::new(
            r###"
class Box<T> {}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();

        let generic_parameter_span = parser
            .tree
            .get_side_span(
                class_id,
                NodeSpanType::Region(NodeSpanRegion::GenericParameters),
            )
            .expect("missing class generic parameter span");

        assert_eq!(parser.get_span_str(generic_parameter_span), "<T>");
    }

    #[test]
    fn test_parse_struct_with_where_clause() {
        let mut test = TestParser::new(
            r###"
struct Foo where Guard: Limit {
}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { generic_parameters, where_clauses, members, .. }) => {
            assert!(members.is_empty());
            assert!(generic_parameters.is_empty());

            // where Guard: Limit
            assert_eq!(where_clauses.len(), 1);
            assert_node!(parser.tree, where_clauses[0], WhereClause { left, right } => {
                assert_string!(parser, *left, "Guard");
                assert_expression_path!(parser, parser.tree.get(*right), "Limit");
            });
        });
    }

    #[test]
    fn test_parse_struct_with_private_member_function() {
        let mut test = TestParser::new(
            r###"
struct Foo {
    private enqueue<M extends F<"mutation">>() {
        throw new Error("Not implemented");
    }
}
"###,
        );
        let mut parser = test.prepare();

        let start = parser.span_start();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { members, .. }) => {
            assert_eq!(members.len(), 1);
        });
    }

    #[test]
    fn test_parse_struct_implements_type_spans() {
        let mut test = TestParser::new("struct Foo implements Qux {}");
        let mut parser = test.prepare();

        let start = parser.span_start();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();

        // spans on implements types
        assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { implements_types, .. }) => {
            assert_eq!(implements_types.len(), 1);
            let implements_span = parser
                .tree
                .get_side_span(implements_types[0], NodeSpanType::Region(NodeSpanRegion::Type))
                .expect("expected implements type span");
            assert_eq!(parser.get_span_str(implements_span), "Qux");
        });
    }

    #[test]
    fn test_parse_struct_negative_implements_type() {
        let mut test = TestParser::new("struct Node implements !Unpin {}");
        let mut parser = test.prepare();

        let start = parser.span_start();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();

        test.assert_no_errors(&parser);

        assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { implements_types, .. }) => {
            assert_eq!(implements_types.len(), 1);
            assert_node!(parser.tree, implements_types[0], TypeExpression::Not { target_type } => {
                assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "Unpin");
                });
            });
        });
    }
}
