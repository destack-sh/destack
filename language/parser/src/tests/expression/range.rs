use crate::tests::TestParser;
use crate::{assert_node, assert_string};
use destack_dir::{BinaryOperator, Expression, RangeEnd, ScalarLiteral};

#[test]
fn test_parse_half_open_range_expression() {
    let mut test = TestParser::new("1..10");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::RangeExpression { start, end, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_node!(parser.tree, start.expect("expected start bound"), Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
            assert_eq!(*value, 1);
        });
        assert_node!(parser.tree, end.expect("expected end bound"), Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
            assert_eq!(*value, 10);
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_inclusive_range_expression() {
    let mut test = TestParser::new("1..=10");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::RangeExpression { start, end, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Inclusive);
        assert_node!(parser.tree, start.expect("expected start bound"), Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
            assert_eq!(*value, 1);
        });
        assert_node!(parser.tree, end.expect("expected end bound"), Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
            assert_eq!(*value, 10);
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_open_ended_range_expressions() {
    let mut test = TestParser::new("1..\n..10\n..=10\n..");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 4);
    assert_node!(parser.tree, expressions[0], Expression::RangeExpression { start, end, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_node!(parser.tree, start.expect("expected start bound"), Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
            assert_eq!(*value, 1);
        });
        assert!(end.is_none());
    });
    assert_node!(parser.tree, expressions[1], Expression::RangeExpression { start, end, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert!(start.is_none());
        assert_node!(parser.tree, end.expect("expected end bound"), Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
            assert_eq!(*value, 10);
        });
    });
    assert_node!(parser.tree, expressions[2], Expression::RangeExpression { start, end, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Inclusive);
        assert!(start.is_none());
        assert_node!(parser.tree, end.expect("expected end bound"), Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
            assert_eq!(*value, 10);
        });
    });
    assert_node!(parser.tree, expressions[3], Expression::RangeExpression { start, end, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert!(start.is_none());
        assert!(end.is_none());
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_range_expression_precedence() {
    let mut test = TestParser::new("start + 1..end * 2");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::RangeExpression { start, end, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_node!(parser.tree, start.expect("expected start bound"), Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::Add);
        });
        assert_node!(parser.tree, end.expect("expected end bound"), Expression::Binary { operator, .. } => {
            assert_eq!(*operator, BinaryOperator::Multiply);
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_negative_range_start_expression() {
    let mut test = TestParser::new("-3..3");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::RangeExpression { start, end, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_node!(parser.tree, start.expect("expected start bound"), Expression::Unary { .. });
        assert_node!(parser.tree, end.expect("expected end bound"), Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
            assert_eq!(*value, 3);
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_range_index_expression() {
    let mut test = TestParser::new("items[1..count]");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_node!(parser.tree, expression_id, Expression::Index { left, index, .. } => {
        assert_node!(parser.tree, *left, Expression::Identifier { name } => {
            assert_string!(parser, *name, "items");
        });
        assert_node!(parser.tree, index.expect("expected index"), Expression::RangeExpression { start, end, end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
            assert_node!(parser.tree, start.expect("expected start bound"), Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
                assert_eq!(*value, 1);
            });
            assert_node!(parser.tree, end.expect("expected end bound"), Expression::Identifier { name } => {
                assert_string!(parser, *name, "count");
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_range_index_expression_forms() {
    let mut test =
        TestParser::new("items[1..=count]\nitems[start..]\nitems[..end]\nitems[..=end]\nitems[..]");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 5);
    assert_node!(parser.tree, expressions[0], Expression::Index { index: Some(index), .. } => {
        assert_node!(parser.tree, *index, Expression::RangeExpression { start, end, end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Inclusive);
            assert_node!(parser.tree, start.expect("expected start bound"), Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
                assert_eq!(*value, 1);
            });
            assert_node!(parser.tree, end.expect("expected end bound"), Expression::Identifier { name } => {
                assert_string!(parser, *name, "count");
            });
        });
    });
    assert_node!(parser.tree, expressions[1], Expression::Index { index: Some(index), .. } => {
        assert_node!(parser.tree, *index, Expression::RangeExpression { start, end, end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
            assert_node!(parser.tree, start.expect("expected start bound"), Expression::Identifier { name } => {
                assert_string!(parser, *name, "start");
            });
            assert!(end.is_none());
        });
    });
    assert_node!(parser.tree, expressions[2], Expression::Index { index: Some(index), .. } => {
        assert_node!(parser.tree, *index, Expression::RangeExpression { start, end, end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
            assert!(start.is_none());
            assert_node!(parser.tree, end.expect("expected end bound"), Expression::Identifier { name } => {
                assert_string!(parser, *name, "end");
            });
        });
    });
    assert_node!(parser.tree, expressions[3], Expression::Index { index: Some(index), .. } => {
        assert_node!(parser.tree, *index, Expression::RangeExpression { start, end, end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Inclusive);
            assert!(start.is_none());
            assert_node!(parser.tree, end.expect("expected end bound"), Expression::Identifier { name } => {
                assert_string!(parser, *name, "end");
            });
        });
    });
    assert_node!(parser.tree, expressions[4], Expression::Index { index: Some(index), .. } => {
        assert_node!(parser.tree, *index, Expression::RangeExpression { start, end, end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
            assert!(start.is_none());
            assert!(end.is_none());
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_full_range_expression_before_newline() {
    let mut test = TestParser::new("..\nvalue");
    let mut parser = test.prepare();
    let expressions = parser.parse();

    assert_eq!(expressions.len(), 2);
    assert_node!(parser.tree, expressions[0], Expression::RangeExpression { start, end, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert!(start.is_none());
        assert!(end.is_none());
    });
    assert_node!(parser.tree, expressions[1], Expression::Identifier { name } => {
        assert_string!(parser, *name, "value");
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_range_expression_recovers_missing_inclusive_end() {
    let mut test = TestParser::new("1..=");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_node!(parser.tree, expression_id, Expression::RangeExpression { start, end, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Inclusive);
        assert_node!(parser.tree, start.expect("expected start bound"), Expression::ScalarLiteral(ScalarLiteral::Integer(value)) => {
            assert_eq!(*value, 1);
        });
        assert!(end.is_some());
    });
}
