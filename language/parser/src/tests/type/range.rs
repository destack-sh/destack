use crate::assert_node;
use crate::tests::*;
use destack_ast::*;

/// Assert one type expression is a named reference.
fn assert_reference(tree: &Tree, type_id: LocalNodeId<TypeExpression>) {
    assert_node!(tree, type_id, TypeExpression::Reference { generic_arguments, .. } => {
        assert!(generic_arguments.is_empty());
    });
}

#[test]
fn test_parse_half_open_range_type() {
    let mut test = TestParser::new("type Window = Start..End");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Range { start, end, end_kind } => {
                assert_eq!(*end_kind, RangeEnd::Open);
                assert_reference(&parser.tree, start.expect("expected start bound"));
                assert_reference(&parser.tree, end.expect("expected end bound"));
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_inclusive_range_type() {
    let mut test = TestParser::new("type Window = Start..=End");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Range { start, end, end_kind } => {
                assert_eq!(*end_kind, RangeEnd::Inclusive);
                assert!(start.is_some());
                assert!(end.is_some());
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_open_ended_range_types() {
    let mut test = TestParser::new(
        r"
type From = Start..
type To = ..End
type ToInclusive = ..=End
type Full = ..
",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 4);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Range { start, end, end_kind } => {
                assert_eq!(*end_kind, RangeEnd::Open);
                assert_reference(&parser.tree, start.expect("expected start bound"));
                assert!(end.is_none());
            });
        });
    });
    assert_node!(parser.tree, expressions[1], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Range { start, end, end_kind } => {
                assert_eq!(*end_kind, RangeEnd::Open);
                assert!(start.is_none());
                assert_reference(&parser.tree, end.expect("expected end bound"));
            });
        });
    });
    assert_node!(parser.tree, expressions[2], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Range { start, end, end_kind } => {
                assert_eq!(*end_kind, RangeEnd::Inclusive);
                assert!(start.is_none());
                assert_reference(&parser.tree, end.expect("expected end bound"));
            });
        });
    });
    assert_node!(parser.tree, expressions[3], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Range { start, end, end_kind } => {
                assert_eq!(*end_kind, RangeEnd::Open);
                assert!(start.is_none());
                assert!(end.is_none());
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_full_range_type_before_newline() {
    let mut test = TestParser::new(
        r"
type Full = ..
type Other = Value
",
    );
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[0], Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Range { start, end, end_kind } => {
                assert_eq!(*end_kind, RangeEnd::Open);
                assert!(start.is_none());
                assert!(end.is_none());
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_range_type_recovers_missing_inclusive_end() {
    let mut test = TestParser::new("type Window = Start..=");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Type(TypeDeclaration { value, .. }) => {
            assert_node!(parser.tree, *value, TypeExpression::Range { start, end, end_kind } => {
                assert_eq!(*end_kind, RangeEnd::Inclusive);
                assert_reference(&parser.tree, start.expect("expected start bound"));
                assert!(end.is_some());
            });
        });
    });
}
