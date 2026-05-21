use destack_dir::{
    CommentKind, Declaration, Decorator, DecoratorPosition, EnumDeclaration, EnumField, EnumKind,
    Expression, GenericParameter, NodeType, ScalarLiteral, TypeExpression, WhereClause,
};
use destack_source::{LanguageType, NodeSpanRegion, NodeSpanType};

use crate::parse::DeclarationHeader;
use crate::{
    TestParser, assert_comment, assert_expression_path, assert_node, assert_path, assert_string,
};

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
fn test_parse_recovers_enum_without_body() {
    let mut test = TestParser::new_with_language(
        r#"
enum;
enum A;
"#,
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[0], Expression::Error);
    assert_node!(parser.tree, expressions[1], Expression::Error);
    test.assert_error_leaves(
        &parser,
        &[
            (Some(NodeType::Expression), None, "enum"),
            (None, None, ";"),
            (Some(NodeType::Expression), None, ";"),
        ],
    );
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
            assert_expression_path!(parser, parser.tree.get(*left), "Requirement");
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

    let expression_id = parser.unwrap_label_expression(expressions[0]);
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

    let expression_id = parser.unwrap_label_expression(expressions[0]);
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

    let expression_id = parser.unwrap_label_expression(expressions[0]);
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

    let expression_id = parser.unwrap_label_expression(expressions[0]);
    assert_node!(parser.tree, expression_id, Expression::Declaration(_declaration_id) => {
        let annotations = parser.tree.get_decorators(expression_id.id);
        assert!(annotations.is_empty());
    });
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::SingleLineBlock, " enum-body");
}
