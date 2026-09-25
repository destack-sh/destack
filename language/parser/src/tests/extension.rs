use tspp_dir::{
    Access, Declaration, ExtensionDeclaration, GenericArgument, GenericParameter, IntegerType,
    Member, Parameter, ThisForm, TypeExpression, TypeLiteral, WhereClause,
};
use tspp_source::{NodeSpanRegion, NodeSpanType};

use crate::parse::DeclarationHeader;
use crate::{TestParser, assert_expression_path, assert_node, assert_path, assert_string};

#[test]
fn test_parse_extension_simple() {
    let test = TestParser::new(
        r###"
extension of Foo {
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let extension_id = parser
        .parse_extension(&start, DeclarationHeader::default())
        .unwrap();
    assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { generic_parameters, implements_types, target_type, .. }) => {
        assert!(generic_parameters.is_empty());
        assert!(implements_types.is_empty());

        // Foo
        assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Foo");
        });
    });
}

#[test]
fn test_report_extension_comma_separated_members() {
    let test = TestParser::new(
        r###"
extension of Foo {
    value: int32,
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let error = parser
        .parse_extension(&start, DeclarationHeader::default())
        .unwrap_err();

    assert_eq!(parser.range_str(error.range()), ",");
}

#[test]
fn test_parse_extension_target_type_span() {
    let test = TestParser::new("extension of Foo.Bar {}");
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let extension_id = parser
        .parse_extension(&start, DeclarationHeader::default())
        .unwrap();

    // retain the target as the anonymous declaration selection
    assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { target_type, .. }) => {
        let span = parser
            .tree
            .get_side_span(*target_type, NodeSpanType::Region(NodeSpanRegion::Type))
            .expect("expected target type span");
        assert_eq!(parser.span_str(span), "Foo.Bar");

        let selection = parser
            .tree
            .get_main_span(extension_id)
            .expect("expected extension selection");
        assert_eq!(parser.span_str(selection), "Bar");
    });
}

#[test]
fn test_parse_extension_with_generic_arguments_and_alias() {
    let test = TestParser::new(
        r###"
extension MyExt of Foo<int32> {
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let extension_id = parser
        .parse_extension(&start, DeclarationHeader::default())
        .unwrap();
    assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { name, generic_parameters, implements_types, target_type, .. }) => {
        assert_string!(parser, name.unwrap().string(), "MyExt");
        assert!(generic_parameters.is_empty());
        assert!(implements_types.is_empty());

        // Foo<int32>
        assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "Foo");

            assert_eq!(generic_arguments.len(), 1);
            // int32
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::Integer(IntegerType::Fixed { width, is_signed }) } => {
                        assert_eq!(*width, 32);
                        assert!(*is_signed);
                    });
            });
        });
    });
}

#[test]
fn test_parse_extension_with_implements_type() {
    let test = TestParser::new(
        r###"
extension of Bar<int32> implements Baz {
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let extension_id = parser
        .parse_extension(&start, DeclarationHeader::default())
        .unwrap();
    assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { generic_parameters, implements_types, target_type, .. }) => {
        assert!(generic_parameters.is_empty());
        assert!(!implements_types.is_empty());

        // Bar<int32>
        assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
            assert_path!(parser, *path, "Bar");

            assert_eq!(generic_arguments.len(), 1);
            // int32
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Keyword { value: TypeLiteral::Integer(IntegerType::Fixed { width, is_signed }) } => {
                        assert_eq!(*width, 32);
                        assert!(*is_signed);
                    });
            });
        });

        assert_eq!(implements_types.len(), 1);
        assert_node!(parser.tree, implements_types[0], TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Baz");
        });
    });
}

#[test]
fn test_parse_extension_with_generic_parameters() {
    let test = TestParser::new(
        r###"
extension<U> of Bar<T> implements Baz<T> {
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let extension_id = parser
        .parse_extension(&start, DeclarationHeader::default())
        .unwrap();
    assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { name, generic_parameters, implements_types, target_type, .. }) => {
        assert!(name.is_none()); // anonymous
        assert_eq!(generic_parameters.len(), 1);

        // extension<U>
        assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, default, .. } => {
            assert_string!(parser, *name, "U");
            assert!(constraint.is_none());
            assert!(default.is_none());
        });

        // Bar<T>
        assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
            // Bar
            assert_path!(parser, *path, "Bar");
            // <T>
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments: _ } => {
                        assert_path!(parser, *path, "T");
                    });
            });
        });

        // : Baz<T>
        assert_eq!(implements_types.len(), 1);
        assert_node!(parser.tree, implements_types[0], TypeExpression::Reference { path, generic_arguments } => {
            // Baz
            assert_path!(parser, *path, "Baz");
            // <T>
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments: _ } => {
                        assert_path!(parser, *path, "T");
                    });
            });
        });
    });

    let generic_parameter_span = parser
        .tree
        .get_side_span(
            extension_id,
            NodeSpanType::Region(NodeSpanRegion::GenericParameters),
        )
        .expect("missing extension generic parameter span");
    assert_eq!(parser.span_str(generic_parameter_span), "<U>");
}

#[test]
fn test_parse_extension_named_with_generic_parameters() {
    let test = TestParser::new(
        r###"
extension MyExt<U> of Bar<T> implements Baz<T> {
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let extension_id = parser
        .parse_extension(&start, DeclarationHeader::default())
        .unwrap();
    assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { name, generic_parameters, implements_types, target_type, .. }) => {
        assert_string!(parser, name.unwrap().string(), "MyExt"); // named
        assert_eq!(generic_parameters.len(), 1);

        // MyExt<U>
        assert_node!(parser.tree, generic_parameters[0], GenericParameter::Type { name, constraint, default, .. } => {
            assert_string!(parser, *name, "U");
            assert!(constraint.is_none());
            assert!(default.is_none());
        });

        // Bar<T>
        assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, generic_arguments } => {
            // Bar
            assert_path!(parser, *path, "Bar");
            // <T>
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments: _ } => {
                        assert_path!(parser, *path, "T");
                    });
            });
        });

        // implements Baz<T>
        assert_eq!(implements_types.len(), 1);
        assert_node!(parser.tree, implements_types[0], TypeExpression::Reference { path, generic_arguments } => {
            // Baz
            assert_path!(parser, *path, "Baz");
            // <T>
            assert_eq!(generic_arguments.len(), 1);
            assert_node!(parser.tree, generic_arguments[0], GenericArgument::Type { value } => {
                    assert_node!(parser.tree, *value, TypeExpression::Reference { path, generic_arguments: _ } => {
                        assert_path!(parser, *path, "T");
                    });
            });
        });
    });

    let generic_parameter_span = parser
        .tree
        .get_side_span(
            extension_id,
            NodeSpanType::Region(NodeSpanRegion::GenericParameters),
        )
        .expect("missing named extension generic parameter span");
    assert_eq!(parser.span_str(generic_parameter_span), "<U>");
}

#[test]
fn test_parse_extension_with_path_name_and_where() {
    let test = TestParser::new(
        r###"
extension of Foo where Guard: Limit {
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let extension_id = parser
        .parse_extension(&start, DeclarationHeader::default())
        .unwrap();
    assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { where_clauses, target_type, .. }) => {
        assert_eq!(where_clauses.len(), 1);

        // where Guard: Limit
        assert_node!(parser.tree, where_clauses[0], WhereClause { relation: _, left, right } => {
            assert_expression_path!(parser, parser.tree.get(*left), "Guard");
            assert_expression_path!(parser, parser.tree.get(*right), "Limit");
        });

        // Foo target_type
        assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Foo");
        });
    });
}

#[test]
fn test_parse_extension_method_with_explicit_this_parameter() {
    let test = TestParser::new(
        r###"
extension<T> of Slice<T> {
    indexSet(this: &Slice<T>, i: number, value: T): void {
        undefined!;
    }
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let extension_id = parser
        .parse_extension(&start, DeclarationHeader::default())
        .unwrap();

    assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { members, .. }) => {
        assert_eq!(members.len(), 1);
        assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
            assert!(signature.this_parameter.is_some());
            assert_eq!(signature.this_form, Some(ThisForm::Explicit));
            assert_eq!(signature.parameters.len(), 2);

            let this_parameter_id = signature.this_parameter.expect("expected explicit this parameter");
            assert_node!(parser.tree, this_parameter_id, Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "this");
                assert!(declared_type.is_some());
            });

            assert_node!(parser.tree, signature.parameters[0], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "i");
            });
            assert_node!(parser.tree, signature.parameters[1], Parameter::Named { name, .. } => {
                assert_string!(parser, *name, "value");
            });
        });
    });
}

#[test]
fn test_parse_extension_method_with_borrowed_this_parameter() {
    let test = TestParser::new(
        r###"
extension<T> of Slice<T> {
    indexSet(&this, i: number, value: T): void {
        undefined!;
    }
}
"###,
    );
    let mut parser = test.prepare();

    let start = parser.mark_parse_start();
    let extension_id = parser
        .parse_extension(&start, DeclarationHeader::default())
        .unwrap();

    // indexSet(&this, i: number, value: T): void
    assert_node!(parser.tree, extension_id, Declaration::Extension(ExtensionDeclaration { members, .. }) => {
        assert_eq!(members.len(), 1);
        assert_node!(parser.tree, members[0], Member::Method { signature, .. } => {
            assert_eq!(signature.this_form, Some(ThisForm::Implicit));
            assert_eq!(signature.parameters.len(), 2);

            let this_parameter = signature.this_parameter.expect("expected this parameter");
            assert_node!(parser.tree, this_parameter, Parameter::Named { name, declared_type, .. } => {
                assert_string!(parser, *name, "this");
                assert_node!(parser.tree, declared_type.unwrap(), TypeExpression::BorrowedOf { access, target_type, .. } => {
                    assert_eq!(*access, Some(Access::Mutable));
                    assert_node!(parser.tree, *target_type, TypeExpression::This);
                });
            });
        });
    });
}
