use destack_dir::{
    Expression, LocalNodeId, Mutability, Name, Pattern, PatternField, RangeEnd, ScalarLiteral,
    TokenType, Tree, TypeExpression,
};
use destack_source::LanguageType;

use crate::{
    TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
};

fn assert_integer_expression(tree: &Tree, id: LocalNodeId<Expression>, value: i64) {
    assert_node!(tree, id, Expression::ScalarLiteral(ScalarLiteral::Integer(actual)) => {
        assert_eq!(*actual, value);
    });
}

/// Build one deeply nested tuple pattern.
fn nested_tuple_pattern_source(depth: usize) -> String {
    let mut source = String::new();

    // open tuple patterns
    for _ in 0..depth {
        source.push('(');
    }

    source.push_str("value");

    // close tuple patterns
    for _ in 0..depth {
        source.push(')');
    }

    source
}

#[test]
fn test_parse_pattern_wildcard() {
    // _
    let mut test = TestParser::new("_");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();
    assert_node!(parser.tree, pattern_id, Pattern::Wildcard);
}

/// Parse a deeply nested tuple pattern without overflowing the parser stack.
#[test]
fn test_parse_deeply_nested_tuple_pattern() {
    let source = nested_tuple_pattern_source(1024);
    let mut test = TestParser::new(&source);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields } => {
        assert_eq!(fields.len(), 1);
    });
}

#[test]
fn test_parse_pattern_underscore_identifier() {
    // _ in TypeScript patterns is a normal binding name
    let mut test = TestParser::new_with_language("_", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();
    assert_node!(parser.tree, pattern_id, Pattern::Binding { name, pattern: None } => {
        assert_string!(parser, *name, "_");
    });
}

#[test]
fn test_parse_tuple_pattern_with_missing_close_parenthesis() {
    let mut test = TestParser::new("(first, second");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_eq!(parser.errors.len(), 1);

    assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields } => {
        assert_eq!(fields.len(), 2);
    });
}

#[test]
fn test_parse_computed_pattern_field_with_missing_close_bracket() {
    let mut test = TestParser::new("{ [key: value }");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_eq!(parser.errors.len(), 1);

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Computed { key, pattern, .. } => {
            assert_expression_path!(parser, parser.tree.get(*key), "key");
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, .. } => {
                assert_string!(parser, *name, "value");
            });
        });
    });
}

#[test]
fn test_parse_pattern_reference() {
    // &_
    let mut test = TestParser::new("&_");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();
    // &
    assert_node!(parser.tree, pattern_id,
        Pattern::BorrowOf { mutability: Some(mutability), right } => {
            assert_eq!(*mutability, Mutability::Mutable);
            // _
            assert_node!(parser.tree, *right, Pattern::Wildcard)
        }
    );

    // &1
    let mut test = TestParser::new("&1");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();
    // &
    assert_node!(parser.tree, pattern_id, Pattern::BorrowOf { mutability: Some(mutability), right } => {
        assert_eq!(*mutability, Mutability::Mutable);
        // 1
        assert_node!(parser.tree, *right, Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    })
}

#[test]
fn test_parse_pattern_value() {
    // ^x
    let mut test = TestParser::new("^x");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();
    assert_node!(parser.tree, pattern_id, Pattern::MoveOf { mutability: Some(mutability), right } => {
        assert_eq!(*mutability, Mutability::Mutable);
        assert_node!(parser.tree, *right, Pattern::Binding { name, pattern: None } => {
            assert_string!(parser, *name, "x");
        });
    });
}

#[test]
fn test_parse_pattern_undefined_literal() {
    let mut test = TestParser::new("undefined");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    // undefined
    assert_node!(parser.tree, pattern_id, Pattern::Expression { value } => {
        assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Undefined));
    });
}

#[test]
fn test_parse_pattern_dereference_binding() {
    let mut test = TestParser::new("*value");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Binding { name, pattern: None } => {
            assert_string!(parser, *name, "value");
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_wildcard() {
    let mut test = TestParser::new("*_");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Wildcard);
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_literal() {
    let mut test = TestParser::new("*42");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Expression { value } => {
            assert_integer_expression(&parser.tree, *value, 42);
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_range() {
    let mut test = TestParser::new("*0..10");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
            assert_integer_expression(&parser.tree, *start, 0);
            assert_integer_expression(&parser.tree, *end, 10);
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_tuple() {
    let mut test = TestParser::new("*(x, y)");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Tuple { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "x");
            });
            assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "y");
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_sequence() {
    let mut test = TestParser::new("*[head, ...tail]");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Sequence { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "head");
            });
            assert_node!(parser.tree, fields[1], PatternField::Spread { pattern: Some(pattern) } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "tail");
                });
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_object() {
    let mut test = TestParser::new("*{ x, y }");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Object { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "x");
            });
            assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "y");
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_tagged_tuple() {
    let mut test = TestParser::new("*Result.Ok(value)");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Newtype { ty, fields } => {
            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                assert_path!(parser, *path, "Result.Ok");
            });
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "value");
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_tagged_object() {
    let mut test = TestParser::new("*Point { x, y }");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::NominalObject { ty, fields } => {
            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                assert_path!(parser, *path, "Point");
            });
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "x");
            });
            assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                assert_name!(parser, *name, "y");
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_before_borrow() {
    let mut test = TestParser::new("*&readonly inner");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::BorrowOf { mutability: Some(mutability), right } => {
            assert_eq!(*mutability, Mutability::Immutable);
            assert_node!(parser.tree, *right, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "inner");
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_before_move() {
    let mut test = TestParser::new("*^exclusive item");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::MoveOf { mutability: Some(mutability), right } => {
            assert_eq!(*mutability, Mutability::Exclusive);
            assert_node!(parser.tree, *right, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "item");
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_identifier() {
    let mut test = TestParser::new("x");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();
    assert_node!(parser.tree, pattern_id, Pattern::Binding { name, pattern: None } => {
        assert_string!(parser, *name, "x");
    });
}

#[test]
fn test_parse_pattern_path() {
    let mut test = TestParser::new("MyEnum.A");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();
    assert_node!(parser.tree, pattern_id, Pattern::Expression { value } => {
        assert_expression_path!(parser, parser.tree.get(*value), "MyEnum.A");
    });
}

#[test]
fn test_parse_pattern_range_half_open() {
    let mut test = TestParser::new("0..10");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_integer_expression(&parser.tree, *start, 0);
        assert_integer_expression(&parser.tree, *end, 10);
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_inclusive() {
    let mut test = TestParser::new("0..=10");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Inclusive);
        assert_integer_expression(&parser.tree, *start, 0);
        assert_integer_expression(&parser.tree, *end, 10);
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_open_ended() {
    let mut test = TestParser::new("0..");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: None, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_integer_expression(&parser.tree, *start, 0);
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_startless() {
    let mut test = TestParser::new("..10");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: None, end: Some(end), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_integer_expression(&parser.tree, *end, 10);
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_startless_inclusive() {
    let mut test = TestParser::new("..=10");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: None, end: Some(end), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Inclusive);
        assert_integer_expression(&parser.tree, *end, 10);
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_identifier_bounds() {
    let mut test = TestParser::new("MIN..MAX");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_node!(parser.tree, *start, Expression::Identifier { name } => {
            assert_string!(parser, *name, "MIN");
        });
        assert_node!(parser.tree, *end, Expression::Identifier { name } => {
            assert_string!(parser, *name, "MAX");
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_path_bounds() {
    let mut test = TestParser::new("Limits.Min..=Limits.Max");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Inclusive);
        assert_expression_path!(parser, parser.tree.get(*start), "Limits.Min");
        assert_expression_path!(parser, parser.tree.get(*end), "Limits.Max");
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_union() {
    let mut test = TestParser::new("0..10 | 20..30");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
        assert_eq!(patterns.len(), 2);
        assert_node!(parser.tree, patterns[0], Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
            assert_integer_expression(&parser.tree, *start, 0);
            assert_integer_expression(&parser.tree, *end, 10);
        });
        assert_node!(parser.tree, patterns[1], Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Open);
            assert_integer_expression(&parser.tree, *start, 20);
            assert_integer_expression(&parser.tree, *end, 30);
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_stops_before_match_arrow() {
    let mut test = TestParser::new("0.. => value");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: None, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_integer_expression(&parser.tree, *start, 0);
    });
    assert!(parser.errors.is_empty());
    assert!(parser.peek_is(TokenType::ArrowWide));
}

#[test]
fn test_parse_pattern_range_recovers_inclusive_end_before_union() {
    let mut test = TestParser::new("0..= | 1");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
        assert_eq!(patterns.len(), 2);
        assert_node!(parser.tree, patterns[0], Pattern::Range { start: Some(_), end: Some(_), end_kind } => {
            assert_eq!(*end_kind, RangeEnd::Inclusive);
        });
        assert_node!(parser.tree, patterns[1], Pattern::Expression { value } => {
            assert_integer_expression(&parser.tree, *value, 1);
        });
    });
}

#[test]
fn test_parse_pattern_range_recovers_bare_range() {
    let mut test = TestParser::new("..");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_node!(parser.tree, pattern_id, Pattern::Range { start: None, end: Some(_), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
    });
}

#[test]
fn test_parse_pattern_range_recovers_missing_inclusive_end() {
    let mut test = TestParser::new("0..=");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(_), end: Some(_), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Inclusive);
    });
}

#[test]
fn test_parse_pattern_tuple() {
    let mut test = TestParser::new("(x: 1, 2, y, z, ...)");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    // (x: 1, 2, y, z, ...)
    assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
        assert_eq!(fields.len(), 5);

        // x: 1
        assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
            assert_name!(parser, *name, "x");
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });

        // 2
        assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
        });

        // y
        assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, is_shorthand: true } => {
            assert_name!(parser, *name, "y");
        });

        // z
        assert_node!(parser.tree, fields[3], PatternField::Named { name, pattern: None, is_shorthand: true } => {
            assert_name!(parser, *name, "z");
        });

        // ...
        assert_node!(parser.tree, fields[4], PatternField::Spread { pattern: None } => {
        });
    });
}

#[test]
fn test_parse_pattern_tuple_with_path() {
    let mut test = TestParser::new("Result.Success(_, ...)");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    // Result.Success(_, ..)
    assert_node!(parser.tree, pattern_id, Pattern::Newtype { ty, fields } => {
        // Result.Success
        let _ = ty; // ty is required for Newtype
        assert_eq!(fields.len(), 2);

        // _
        assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Wildcard);
        });

        // ..
        assert_node!(parser.tree, fields[1], PatternField::Spread { pattern: None } => {
        });
    });
}

#[test]
fn test_parse_pattern_object_field_const_alias() {
    let mut test =
        TestParser::new_with_language("{ const: value, title }", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    // { const: value, title }
    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 2);

        // const: value
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern) } => {
            assert_name!(parser, *name, "const");
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "value");
            });
        });

        // title
        assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "title");
        });
    });
}

#[test]
fn test_parse_pattern_tuple_spread_non_terminal() {
    let mut test = TestParser::new("(x, ...rest, z)");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
        assert_eq!(fields.len(), 3);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, is_shorthand: true } => {
            assert_name!(parser, *name, "x");
        });
        assert_node!(parser.tree, fields[1], PatternField::Spread { pattern: Some(pattern) } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "rest");
            });
        });
        assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, is_shorthand: true } => {
            assert_name!(parser, *name, "z");
        });
    });
}

#[test]
fn test_parse_pattern_tuple_newline_separated() {
    let mut test = TestParser::new(
        "
(
    x: 1
    2,
    ...
)",
    );
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    // (x: 1, 2, ..)
    assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
        assert_eq!(fields.len(), 3);

        // x: 1
        assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
            assert_name!(parser, *name, "x");
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });

        // 2
        assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
        });

        // ..
        assert_node!(parser.tree, fields[2], PatternField::Spread { pattern: None } => {
        });
    });
}

#[test]
fn test_parse_pattern_union() {
    // 1 | 2 | 3
    let mut test = TestParser::new("1 | 2 | 3");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
        assert_eq!(patterns.len(), 3);

        // 1
        assert_node!(parser.tree, patterns[0], Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });

        // 2
        assert_node!(parser.tree, patterns[1], Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
        });

        // 3
        assert_node!(parser.tree, patterns[2], Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(3)));
        });
    });
}

#[test]
fn test_parse_pattern_struct_anonymous() {
    let mut test = TestParser::new("{ x: 1, y, z, w: 4, ... }");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 5);

        // x: 1
        assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
            assert_name!(parser, *name, "x");
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });

        // y
        assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, is_shorthand: true } => {
            assert_name!(parser, *name, "y");
        });

        // z
        assert_node!(parser.tree, fields[2], PatternField::Named { name, pattern: None, is_shorthand: true } => {
            assert_name!(parser, *name, "z");
        });

        // w: 4
        assert_node!(parser.tree, fields[3], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
            assert_name!(parser, *name, "w");
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(4)));
            });
        });

        // ..
        assert_node!(parser.tree, fields[4], PatternField::Spread { pattern: None } => {
        });
    });
}

#[test]
fn test_parse_pattern_named_default_after_comment_newline() {
    let mut test = TestParser::new_with_language("{d //comment\n= b}", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: Some(pattern), .. } => {
            assert_name!(parser, *name, "d");
            assert_node!(parser.tree, *pattern, Pattern::Assign { pattern, value } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                    assert_string!(parser, *name, "d");
                });
                assert_expression_path!(parser, parser.tree.get(*value), "b");
            });
        });
    });
}

#[test]
fn test_parse_pattern_struct_numeric_name_aliases() {
    let mut test = TestParser::new("{ 0: fieldNameOrOptions, 1: from, length: argc }");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 3);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern), .. } => {
            assert_node!(name, Name::Index(index) => {
                assert_eq!(*index, 0);
            });
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "fieldNameOrOptions");
            });
        });
        assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern), .. } => {
            assert_node!(name, Name::Index(index) => {
                assert_eq!(*index, 1);
            });
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "from");
            });
        });
        assert_node!(parser.tree, fields[2], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern), .. } => {
            assert_name!(parser, *name, "length");
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "argc");
            });
        });
    });
}

#[test]
fn test_parse_pattern_struct_boolean_name_aliases() {
    let mut test = TestParser::new_with_language(
        "{ false: decorators, true: metadata }",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 2);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern), .. } => {
            assert_name!(parser, *name, "false");
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "decorators");
            });
        });
        assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern), .. } => {
            assert_name!(parser, *name, "true");
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "metadata");
            });
        });
    });
}

#[test]
fn test_parse_pattern_struct_numeric_literal_field() {
    let mut test = TestParser::new_with_language("{ 5 }", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(5)));
            });
        });
    });
}

#[test]
fn test_parse_pattern_struct_computed_field() {
    let mut test = TestParser::new("{ [key]: value }");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Computed { key, pattern } => {
            assert_node!(parser.tree, *key, Expression::Identifier { name } => {
                assert_string!(parser, *name, "key");
            });
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "value");
            });
        });
    });
}

#[test]
fn test_parse_pattern_struct_with_path() {
    let mut test = TestParser::new("Vector2 { x: 0, y }");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::NominalObject { ty, fields } => {
        assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
            assert_path!(parser, *path, "Vector2");
        });
        assert_eq!(fields.len(), 2);

        // x: 0
        assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
            assert_name!(parser, *name, "x");
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(0)));
            });
        });

        // y
        assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, is_shorthand: true } => {
            assert_name!(parser, *name, "y");
        });
    });
}

#[test]
fn test_parse_pattern_struct_with_nested_tagged_object_field() {
    let mut test = TestParser::new("Shape.Line { start: Point { x, y }, end }");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert!(parser.errors.is_empty());

    assert_node!(parser.tree, pattern_id, Pattern::NominalObject { ty, fields } => {
        assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
            assert_path!(parser, *path, "Shape.Line");
        });
        assert_eq!(fields.len(), 2);

        assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
            assert_name!(parser, *name, "start");
            assert_node!(parser.tree, *pattern, Pattern::NominalObject { ty, fields } => {
                assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                    assert_path!(parser, *path, "Point");
                });
                assert_eq!(fields.len(), 2);

                assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                    assert_name!(parser, *name, "x");
                });
                assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, is_shorthand: true } => {
                    assert_name!(parser, *name, "y");
                });
            });
        });

        assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, is_shorthand: true } => {
            assert_name!(parser, *name, "end");
        });
    });
}

#[test]
fn test_parse_pattern_slice() {
    let mut test = TestParser::new("[1, ...]");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        assert_eq!(fields.len(), 2);
        // 1
        assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
        // ...
        assert_node!(parser.tree, fields[1], PatternField::Spread { pattern: None } => {
        });
    });
}

/// Parse a spread field with a sequence pattern.
#[test]
fn test_parse_pattern_spread_array_pattern() {
    let mut test = TestParser::new("[...[x, y]]");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Spread { pattern: Some(pattern) } => {
            assert_node!(parser.tree, *pattern, Pattern::Sequence { fields } => {
                assert_eq!(fields.len(), 2);
                assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, .. } => {
                    assert_name!(parser, *name, "x");
                });
                assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, .. } => {
                    assert_name!(parser, *name, "y");
                });
            });
        });
    });
}

#[test]
fn test_parse_pattern_object_readonly_shorthand() {
    let mut test = TestParser::new_with_language("{ readonly }", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "readonly");
        });
    });
}

#[test]
fn test_parse_pattern_object_readonly_shorthand_with_newline() {
    let mut test = TestParser::new_with_language("{ readonly\n}", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "readonly");
        });
    });
}
#[test]
fn test_parse_pattern_object_readonly_shorthand_in_value_block_mode() {
    let mut test = TestParser::new("{ readonly }");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "readonly");
        });
    });
}

#[test]
fn test_parse_pattern_array_readonly_identifier() {
    let mut test =
        TestParser::new_with_language("[readonly, setReadonly]", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        assert_eq!(fields.len(), 2);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "readonly");
        });
        assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "setReadonly");
        });
    });
}

#[test]
fn test_parse_pattern_array_readonly_identifier_in_value_block_mode() {
    let mut test = TestParser::new("[readonly, setReadonly]");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        assert_eq!(fields.len(), 2);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "readonly");
        });
        assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "setReadonly");
        });
    });
}

#[test]
fn test_reject_pattern_object_readonly_modifier_with_name_in_value_block_mode() {
    let mut test = TestParser::new("{ readonly value }");
    let mut parser = test.prepare();

    let result = parser.eat_pattern();

    assert!(result.is_err());
}

#[test]
fn test_parse_pattern_object_spread_newline_before_terminator() {
    let mut test =
        TestParser::new_with_language("{\n  onSuccess,\n  ...rest\n}", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 2);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, is_shorthand: true } => {
            assert_name!(parser, *name, "onSuccess");
        });
        assert_node!(parser.tree, fields[1], PatternField::Spread { pattern: Some(pattern) } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "rest");
            });
        });
    });
}

#[test]
fn test_parse_pattern_must() {
    let mut test = TestParser::new("1!");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Must(inner) => {
        assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
    });
}

#[test]
fn test_parse_pattern_union_with_must_arms() {
    let mut test = TestParser::new("1! | 2!");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
        assert_eq!(patterns.len(), 2);
        // 1!
        assert_node!(parser.tree, patterns[0], Pattern::Must(inner) => {
            assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
            });
        });
        // 2!
        assert_node!(parser.tree, patterns[1], Pattern::Must(inner) => {
            assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
        });
    });
}

#[test]
fn test_parse_pattern_union_with_trailing_must_arm() {
    let mut test = TestParser::new("1 | 2!");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
        assert_eq!(patterns.len(), 2);
        // 1
        assert_node!(parser.tree, patterns[0], Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(1)));
        });
        // 2!
        assert_node!(parser.tree, patterns[1], Pattern::Must(inner) => {
            assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::ScalarLiteral(ScalarLiteral::Integer(2)));
            });
        });
    });
}

#[test]
fn test_parse_pattern_array_elision() {
    // [,a] - elision before 'a'
    let mut test = TestParser::new("[,a]");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        assert_eq!(fields.len(), 2);

        // elision (empty slot)
        assert_node!(parser.tree, fields[0], PatternField::Elision);

        // a (identifiers are parsed as Named shorthand)
        assert_node!(parser.tree, fields[1], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "a");
        });
    });
}

#[test]
fn test_parse_pattern_array_multiple_elisions() {
    // [,,a] - two elisions before 'a'
    let mut test = TestParser::new("[,,a]");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        assert_eq!(fields.len(), 3);

        // first elision
        assert_node!(parser.tree, fields[0], PatternField::Elision);

        // second elision
        assert_node!(parser.tree, fields[1], PatternField::Elision);

        // a (identifiers are parsed as Named shorthand)
        assert_node!(parser.tree, fields[2], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "a");
        });
    });
}

#[test]
fn test_parse_pattern_array_trailing_elision() {
    // [a,] - element followed by trailing comma (not elision)
    let mut test = TestParser::new("[a,]");
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        // trailing comma doesn't create elision, just 'a'
        assert_eq!(fields.len(), 1);

        // a (identifiers are parsed as Named shorthand)
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "a");
        });
    });
}

#[test]
fn test_parse_object_pattern_defaults_do_not_consume_following_fields() {
    // {a,b=1,c:d,e:f=2,[g]:[h]}
    let mut test =
        TestParser::new_with_language("{a,b=1,c:d,e:f=2,[g]:[h]}", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 5);

        // b=1
        assert_node!(parser.tree, fields[1], PatternField::Named { is_shorthand: true, pattern: Some(pattern), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Assign { .. });
        });

        // c:d
        assert_node!(parser.tree, fields[2], PatternField::Named { is_shorthand: false, pattern: Some(pattern), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { .. });
        });

        // e:f=2
        assert_node!(parser.tree, fields[3], PatternField::Named { is_shorthand: false, pattern: Some(pattern), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Assign { .. });
        });

        // [g]:[h]
        assert_node!(parser.tree, fields[4], PatternField::Computed { pattern, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Sequence { .. });
        });
    });
}

#[test]
fn test_parse_object_pattern_alias_and_computed_defaults() {
    // {c, d:e=1, [f]:g=2, h=i}
    let mut test =
        TestParser::new_with_language("{c, d:e=1, [f]:g=2, h=i}", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 4);

        // d:e=1
        assert_node!(parser.tree, fields[1], PatternField::Named { is_shorthand: false, pattern: Some(pattern), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Assign { .. });
        });

        // [f]:g=2
        assert_node!(parser.tree, fields[2], PatternField::Computed { pattern, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Assign { .. });
        });

        // h=i
        assert_node!(parser.tree, fields[3], PatternField::Named { is_shorthand: true, pattern: Some(pattern), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Assign { .. });
        });
    });
}

#[test]
fn test_parse_object_pattern_computed_field_with_newline_after_colon() {
    // { [key]:\nvalue }
    let mut test = TestParser::new_with_language("{ [key]:\nvalue }", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);

        assert_node!(parser.tree, fields[0], PatternField::Computed { key, pattern } => {
            assert_node!(parser.tree, *key, Expression::Identifier { name } => {
                assert_string!(parser, *name, "key");
            });

            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "value");
            });
        });
    });
}

#[test]
fn test_parse_object_pattern_alias_with_newline_after_colon() {
    // { source:\ntarget }
    let mut test = TestParser::new_with_language("{ source:\ntarget }", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let pattern_id = parser.eat_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);

        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: false, pattern: Some(pattern) } => {
            assert_name!(parser, *name, "source");
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "target");
            });
        });
    });
}
