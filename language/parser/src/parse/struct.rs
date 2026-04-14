#![allow(clippy::type_complexity)]

use crate::parse::expression::common::DeclarationHeader;
use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{
    ClassDeclaration, Declaration, Keyword, LocalNodeId, NodeType, StructDeclaration, TokenType,
};

impl Parser {
    /// Eat a struct or class declaration.
    ///
    /// The parser accepts `extends` for both, but structs cannot semantically
    /// use extends (use embedding instead). This is validated in the analyze phase.
    /// Struct declarations require a name.
    ///
    /// Struct examples:
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
    ///     ...Bar;              // embedding for composition
    ///     static x: int32 = 7; // constant
    ///
    ///     myFunc() { }
    /// };
    /// ```
    ///
    /// Class examples:
    /// ```
    /// class Foo extends Bar { // classes can extend
    ///     myField: int32;
    /// };
    /// ```
    pub(crate) fn eat_struct_or_class(
        &mut self,
        start: &ParserMark,
        header: DeclarationHeader,
        allow_anonymous_class: bool,
    ) -> ParseResult<LocalNodeId<Declaration>> {
        let _timing = self.timing_scope(tags::PARSE_STRUCT);
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
        let generic_parameters = self
            .eat_generic_parameters_maybe(false)
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
        self.eat_newlines_maybe()?;
        self.try_eat_token(TokenType::OpenBrace, TokenType::CloseBrace)
            .for_node_type(NodeType::Declaration)?;
        self.eat_newlines_maybe()?;
        let member_options = self.options.nested().in_variant();
        let members = self.with_options(member_options, |parser| parser.eat_members(false))?;
        self.eat_close_token_or_recover_missing(TokenType::CloseBrace, NodeType::Declaration)?;

        // struct or class
        let declaration = if is_class {
            Declaration::Class(ClassDeclaration {
                name,
                export: header.export,
                ambient: header.ambient,
                is_abstract: header.is_abstract,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses: where_clauses.unwrap_or_default(),
                extends_type: extends_types.and_then(|mut types| {
                    if types.is_empty() {
                        None
                    } else {
                        Some(types.remove(0))
                    }
                }),
                implements_types: implements_types.unwrap_or_default(),
                members,
            })
        } else {
            Declaration::Struct(StructDeclaration {
                name: name.expect("structs require a name here"),
                export: header.export,
                ambient: header.ambient,
                generic_parameters: generic_parameters.unwrap_or_default(),
                where_clauses: where_clauses.unwrap_or_default(),
                implements_types: implements_types.unwrap_or_default(),
                embedded_types: extends_types.unwrap_or_default(),
                members,
            })
        };
        let declaration_id = self.insert_node(declaration, self.get_span_from(start));

        // set main span to the name identifier
        if let Some(span) = name_span {
            self.tree.set_main_span(declaration_id, span);
        }

        Ok(declaration_id)
    }
}

#[cfg(test)]
mod tests {
    use destack_ast::{
        ClassDeclaration, CommentKind, Declaration, Expression, GenericParameter, IntType, Key,
        Member, Name, Parameter, ScalarLiteral, StructDeclaration, TypeExpression, TypeLiteral,
        Visibility, WhereClause,
    };
    use destack_source::{LanguageType, NodeSpanType};

    use crate::parse::expression::common::DeclarationHeader;
    use crate::{
        TestParser, assert_comment, assert_expression_path, assert_node, assert_path, assert_string,
    };

    #[test]
    fn test_parse_struct_requires_name() {
        let mut test = TestParser::new(
            r###"
struct { public x: int32, readonly y: boolean }
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let result = parser.eat_struct_or_class(&start, DeclarationHeader::default(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_struct_with_extends_types() {
        let mut test = TestParser::new(
            r###"
struct Foo extends Bar {}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { name, embedded_types, implements_types, members, .. }) => {
            assert_string!(parser, name.string(), "Foo");
            assert!(members.is_empty());
            assert!(implements_types.is_empty());
            assert_eq!(embedded_types.len(), 1);
            assert_node!(parser.tree, embedded_types[0], TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Bar");
            });
        });
    }

    #[test]
    fn test_parse_javascript_class_rejects_parenthesized_binary_extends_expression() {
        let mut test =
            TestParser::new_with_options("class A extends (a + b) {}", LanguageType::JavaScript);
        let mut parser = test.prepare();

        let start = parser.mark();
        let result = parser.eat_struct_or_class(&start, DeclarationHeader::default(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_class_keeps_unparenthesized_decorated_extends_head_unwrapped() {
        let mut test = TestParser::new_with_options(
            "class Outer extends\n@deco\nclass {} {}",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let start = parser.mark();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { extends_type: Some(extends_type), .. }) => {
            assert_node!(parser.tree, *extends_type, TypeExpression::Declaration { declaration: declaration_id } => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { .. }));
            });
        });
    }

    #[test]
    fn test_parse_class_keeps_parenthesized_decorated_extends_head_unwrapped() {
        let mut test = TestParser::new_with_options(
            "class Outer extends (@deco class Base {}) {}",
            LanguageType::JavaScript,
        );
        let mut parser = test.prepare();

        let start = parser.mark();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { extends_type: Some(extends_type), .. }) => {
            assert_node!(parser.tree, *extends_type, TypeExpression::Declaration { declaration: declaration_id } => {
                assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { name, .. }) => {
                    assert_string!(parser, name.expect("expected class name").string(), "Base");
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { name, extends_type: Some(extends_type), .. }) => {
            assert_string!(parser, name.expect("expected class name").string(), "Combined");
            assert_node!(parser.tree, *extends_type, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "First");
            });
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let class_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { name, extends_type, .. }) => {
            assert_string!(parser, name.expect("expected class name").string(), "Counter");
            assert!(extends_type.is_none());
        });
    }

    #[test]
    fn test_parse_class_member_method_parameter_type_then_default_value_typescript() {
        let mut test = TestParser::new_with_options(
            r#"class LicensingStore {
  usersLimitReached(userCount: number, userLimit = get(this.store).userLimit) {
    return userCount >= userLimit
  }
}"#,
            LanguageType::TypeScript,
        );
        let mut parser = test.prepare();

        let start = parser.mark();
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
        let mut test = TestParser::new_with_options(
            r"class Child extends Base // extends-tail
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

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
        assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
            assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { extends_type: Some(extends_type), members, .. }) => {
                assert_eq!(members.len(), 1);
                let extends_annotations = parser.tree.get_decorators(extends_type.id);
                assert!(extends_annotations.is_empty());

                let member_annotations = parser.tree.get_decorators(members[0].id);
                assert!(member_annotations.is_empty());
            });
        });
        assert_eq!(parser.tree.comments().len(), 1);
        assert_comment!(parser, 0, CommentKind::Line, "extends-tail");
    }

    #[test]
    fn test_parse_class_implement_list_comments_on_interface_types() {
        let mut test = TestParser::new_with_options(
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

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
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
        let mut test = TestParser::new_with_options(
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

        let expression_id = parser.unwrap_labelled_expression(expressions[0]);
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
    fn test_parse_struct_with_spread() {
        let mut test = TestParser::new(
            r###"
struct Foo<T: Numeric> extends Boz implements Quux {
    ...Bar
    ...Baz

    a: T
    b?: T
    c: T
    private d: int32 = 4
}
"###,
        );
        let mut parser = test.prepare();
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { name, generic_parameters, embedded_types, implements_types, members, .. }) => {
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

            // Boz
            assert_eq!(embedded_types.len(), 1);
            assert_node!(parser.tree, embedded_types[0], TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Boz");
            });
            assert_eq!(implements_types.len(), 1);
            assert_node!(parser.tree, implements_types[0], TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Quux");
            });

            assert_eq!(members.len(), 6);

            // ..Bar
            assert_node!(parser.tree, members[0], Member::Embed { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "Bar");
            });
            // ..Baz
            assert_node!(parser.tree, members[1], Member::Embed { value, .. } => {
                assert_expression_path!(parser, parser.tree.get(*value), "Baz");
            });
            // a: T
            assert_node!(parser.tree, members[2], Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: Some(ty), default: None, .. } => {
                assert_string!(parser, *name, "a");
                assert_node!(parser.tree, *ty, TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            // b?: T
            assert_node!(parser.tree, members[3], Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: Some(ty), is_optional, default: None, .. } => {
                assert!(*is_optional);
                assert_string!(parser, *name, "b");
                assert_expression_path!(parser, parser.tree.get(*ty), "T");
            });
            // c: T
            assert_node!(parser.tree, members[4], Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: Some(ty), default: None, .. } => {
                assert_string!(parser, *name, "c");
                assert_node!(parser.tree, *ty, TypeExpression::Reference { path, .. } => {
                    assert_path!(parser, *path, "T");
                });
            });
            // private d: int32 = 4
            assert_node!(parser.tree, members[5], Member::Field { key: Key::Name(Name::Identifier(name)), declared_type: Some(ty), default: Some(value), visibility, .. } => {
                assert_eq!(*visibility, Some(Visibility::Private));
                assert_string!(parser, *name, "d");
                assert_node!(parser.tree, *ty, TypeExpression::Literal { value } => {
                    assert_eq!(*value, TypeLiteral::Int(IntType::Arbitrary {
                        width: Some(32),
                        is_signed: true,
                    }));
                });
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
            });
        });
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
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
        parser.eat_newline().unwrap();

        let start = parser.mark();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();
        assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { members, .. }) => {
            assert_eq!(members.len(), 1);
        });
    }

    #[test]
    fn test_parse_struct_heritage_type_spans() {
        let mut test = TestParser::new("struct Foo extends Bar.Baz implements Qux {}");
        let mut parser = test.prepare();

        let start = parser.mark();
        let struct_id = parser
            .eat_struct_or_class(&start, DeclarationHeader::default(), false)
            .unwrap();

        // spans on extends and implements types
        assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { embedded_types, implements_types, .. }) => {
            assert_eq!(embedded_types.len(), 1);
            let extends_span = parser
                .tree
                .get_side_span(embedded_types[0], NodeSpanType::Type)
                .expect("expected extends type span");
            assert_eq!(parser.get_span_str(extends_span), "Bar.Baz");
            assert_eq!(implements_types.len(), 1);
            let implements_span = parser
                .tree
                .get_side_span(implements_types[0], NodeSpanType::Type)
                .expect("expected implements type span");
            assert_eq!(parser.get_span_str(implements_span), "Qux");
        });
    }
}
