use destack_dir::{
    ClassDeclaration, CommentKind, Declaration, Expression, GenericParameter, IntegerType, Literal,
    Member, Name, Parameter, StructDeclaration, TypeExpression, TypeLiteral, Visibility,
    WhereClause,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

use crate::parse::DeclarationHeader;
use crate::{
    TestParser, assert_comment, assert_expression_path, assert_node, assert_path, assert_string,
};

#[test]
fn test_parse_struct_requires_name() {
    let test = TestParser::new(
        r###"
struct { public x: int32, readonly y: boolean }
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let error = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
        .unwrap_err();

    assert_eq!(parser.range_str(error.range()), "{");
}

#[test]
fn test_parse_class_requires_name_in_statement_position() {
    let test = TestParser::new(
        r###"
class {}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let error = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
        .unwrap_err();

    assert_eq!(parser.range_str(error.range()), "{");
}

#[test]
fn test_report_class_comma_separated_members() {
    // source: class Foo { x: int32, y: int32 }
    let test = TestParser::new(
        r###"
class Foo { x: int32, y: int32 }
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let error = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
        .unwrap_err();

    assert_eq!(parser.range_str(error.range()), ",");
}

#[test]
fn test_report_struct_comma_separated_members() {
    // source: struct Foo { x: int32, y: int32 }
    let test = TestParser::new(
        r###"
struct Foo { x: int32, y: int32 }
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let error = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
        .unwrap_err();

    assert_eq!(parser.range_str(error.range()), ",");
}

#[test]
fn test_parse_shared_struct_declaration() {
    let test = TestParser::new("shared struct Channel<T> {}");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);

    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Struct(StructDeclaration { name, is_shared, .. }) => {
            assert_string!(parser, name.string(), "Channel");
            assert!(*is_shared);
        });
    });
}

/// Parse const and fixed-array forms together inside a struct.
#[test]
fn test_parse_struct_const_forms() {
    let test = TestParser::new(
        r#"struct StaticBuffer<const Size: uint> {
    const {
        assert(Size > 0 && Size <= 65536);
    }
    tag: Size extends 4 ? "small" : "large";
    data: [uint8; Size];
}"#,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);

    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Struct(StructDeclaration { members, .. }) => {
            assert_eq!(members.len(), 3);
            assert_node!(parser.tree, members[0], Member::ConstBlock { .. });
            assert_node!(parser.tree, members[1], Member::Field { .. });
            assert_node!(parser.tree, members[2], Member::Field { .. });
        });
    });
}

#[test]
fn test_report_struct_extends() {
    let test = TestParser::new(
        r###"
struct Foo extends Bar implements Baz {
    value: int32;
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let struct_id = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
        .unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.range_str(parser.errors[0].range()), "extends");
    assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { implements_types, members, .. }) => {
        assert_eq!(implements_types.len(), 1);
        assert_eq!(members.len(), 1);
    });
}

#[test]
fn test_parse_class_with_multiple_extends_for_lineage_validation() {
    let test = TestParser::new(
        r###"
class Combined extends First, Second {}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let class_id = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
        .unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.range_str(parser.errors[0].range()), "Second");
    assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { name, extends_type: Some(extends_type), .. }) => {
        assert_string!(parser, name.expect("expected class name").string(), "Combined");
        assert_expression_path!(parser, parser.tree.get(*extends_type), "First");
    });
}

#[test]
fn test_parse_class_with_empty_extends_for_lineage_validation() {
    let test = TestParser::new(
        r###"
class Counter extends {}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let class_id = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
        .unwrap();
    assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { name, extends_type, .. }) => {
        assert_string!(parser, name.expect("expected class name").string(), "Counter");
        assert!(extends_type.is_none());
    });
}

#[test]
fn test_parse_class_member_method_parameter_type_then_default_value() {
    let test = TestParser::new(
        r#"class LicensingStore {
  usersLimitReached(userCount: number, userLimit = get(this.store).userLimit) {
    return userCount >= userLimit
  }
}"#,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let class_id = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
        .unwrap();

    // parse one class method that mixes typed and defaulted parameters
    assert_node!(parser.tree, class_id, Declaration::Class(ClassDeclaration { members, .. }) => {
        assert_eq!(members.len(), 1);

        // usersLimitReached(userCount: number, userLimit = get(this.store).userLimit)
        assert_node!(parser.tree, members[0], Member::Method { name: Some(Name::Identifier(name)), signature, body: Some(_), .. } => {
            assert_string!(parser, *name, "usersLimitReached");
            assert_eq!(signature.parameters.len(), 2);
            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, declared_type: Some(ty), default, .. } => {
                assert_string!(parser, *name, "userCount");
                assert!(default.is_none());
                assert_node!(parser.tree, *ty, TypeExpression::Keyword { value } => {
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
    let test = TestParser::new(
        r"class Child extends Base // extends-tail
{
  value = 1
}",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { extends_type: Some(extends_type), members, .. }) => {
            assert_eq!(members.len(), 1);
            let extends_annotations = parser.tree.get_decorators(extends_type.id);
            assert!(extends_annotations.is_empty());

            let member_annotations = parser.tree.get_decorators(members[0].id);
            assert!(member_annotations.is_empty());
        });
    });
    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "extends-tail");

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_class_implement_list_comments_on_interface_types() {
    let test = TestParser::new(
        r"class Child implements First, // impl-first
Second // impl-second
{
  value = 1
}",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
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
    assert_eq!(parser.comments().len(), 2);
    assert_comment!(parser, 0, CommentKind::Line, "impl-first");
    assert_comment!(parser, 1, CommentKind::Line, "impl-second");
}

#[test]
fn test_parse_declare_class_head_comment_before_generics_on_declaration_owner() {
    let test = TestParser::new(
        r"declare class Box // box-head
<T> implements Item<T>, Other {
  value: T
}",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert!(
        parser.errors.is_empty(),
        "unexpected parser errors: {:?}",
        parser.errors
    );
    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        let first_static_parameter_id = assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { generic_parameters, .. }) => {
            assert_eq!(generic_parameters.len(), 1);
            generic_parameters[0]
        });

        let annotations = parser.tree.get_decorators(first_static_parameter_id.id);
        assert!(annotations.is_empty());
    });
    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "box-head");
}

#[test]
fn test_parse_struct_with_implements() {
    let test = TestParser::new(
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

    let start = parser.mark_parse_start();
    let struct_id = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
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
        assert_node!(parser.tree, members[0], Member::Field { name: Name::Identifier(name), declared_type: Some(ty), default: None, .. } => {
            assert_string!(parser, *name, "a");
            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "T");
            });
        });
        // b?: T
        assert_node!(parser.tree, members[1], Member::Field { name: Name::Identifier(name), declared_type: Some(ty), is_optional, default: None, .. } => {
            assert!(*is_optional);
            assert_string!(parser, *name, "b");
            assert_expression_path!(parser, parser.tree.get(*ty), "T");
        });
        // c: T
        assert_node!(parser.tree, members[2], Member::Field { name: Name::Identifier(name), declared_type: Some(ty), default: None, .. } => {
            assert_string!(parser, *name, "c");
            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "T");
            });
        });
        // private d: int32 = 4
        assert_node!(parser.tree, members[3], Member::Field { name: Name::Identifier(name), declared_type: Some(ty), default: Some(value), visibility, .. } => {
            assert_eq!(*visibility, Some(Visibility::Private));
            assert_string!(parser, *name, "d");
            assert_node!(parser.tree, *ty, TypeExpression::Keyword { value } => {
                assert_eq!(*value, TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                }));
            });
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(4)));
        });
    });
}

#[test]
fn test_parse_class_records_generic_parameter_container_range() {
    let test = TestParser::new(
        r###"
class Box<T> {}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let class_id = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
        .unwrap();

    let generic_parameter_span = parser
        .tree
        .get_side_span(
            class_id,
            NodeSpanType::Region(NodeSpanRegion::GenericParameters),
        )
        .expect("missing class generic parameter span");

    assert_eq!(parser.span_str(generic_parameter_span), "<T>");
}

#[test]
fn test_parse_struct_with_where_clause() {
    let test = TestParser::new(
        r###"
struct Foo where Guard: Limit {
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let struct_id = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
        .unwrap();
    assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { generic_parameters, where_clauses, members, .. }) => {
        assert!(members.is_empty());
        assert!(generic_parameters.is_empty());

        // where Guard: Limit
        assert_eq!(where_clauses.len(), 1);
        assert_node!(parser.tree, where_clauses[0], WhereClause { relation: _, left, right } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Guard");
            assert_expression_path!(parser, parser.tree.get(*right), "Limit");
        });
    });
}

#[test]
fn test_parse_struct_with_private_member_function() {
    let test = TestParser::new(
        r###"
struct Foo {
    private enqueue<M: F<"mutation">>() {
        unreachable();
    }
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let struct_id = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
        .unwrap();
    assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { members, .. }) => {
        assert_eq!(members.len(), 1);
    });
}

#[test]
fn test_parse_struct_implements_type_spans() {
    let test = TestParser::new("struct Foo implements Qux {}");
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let struct_id = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
        .unwrap();

    // spans on implements types
    assert_node!(parser.tree, struct_id, Declaration::Struct(StructDeclaration { implements_types, .. }) => {
        assert_eq!(implements_types.len(), 1);
        let implements_span = parser
            .tree
            .get_side_span(implements_types[0], NodeSpanType::Region(NodeSpanRegion::Type))
            .expect("expected implements type span");
        assert_eq!(parser.span_str(implements_span), "Qux");
    });
}

#[test]
fn test_parse_struct_negative_implements_type() {
    let test = TestParser::new("struct Node implements !Unpin {}");
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let struct_id = parser
        .parse_struct_or_class(&start, DeclarationHeader::default(), false)
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

/// A repeated declaration modifier is reported at its second occurrence and the declaration still parses.
#[test]
fn test_reject_a_repeated_declaration_modifier() {
    let test = TestParser::new("shared shared class User {}");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.range_str(parser.errors[0].range()), "shared");
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Class(ClassDeclaration { name, is_shared, .. }) => {
            assert_string!(parser, name.expect("expected name").string(), "User");
            assert!(*is_shared);
        });
    });
}
