use destack_dir::{
    Exclusivity, Expression, Literal, LocalNodeId, Mutability, Name, Pattern, PatternField,
    RangeEnd, TokenType, Tree, TypeExpression,
};

use crate::{
    TestParser, assert_expression_path, assert_name, assert_node, assert_path, assert_string,
};

fn assert_integer_expression(tree: &Tree, id: LocalNodeId<Expression>, value: i64) {
    assert_node!(tree, id, Expression::Literal(Literal::Integer(actual)) => {
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
    let test = TestParser::new("_");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();
    assert_node!(parser.tree, pattern_id, Pattern::Wildcard);
}

/// Parse a deeply nested tuple pattern without overflowing the parser stack.
#[test]
fn test_parse_deeply_nested_tuple_pattern() {
    let source = nested_tuple_pattern_source(1024);
    let test = TestParser::new(&source);
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields } => {
        assert_eq!(fields.len(), 1);
    });
}

#[test]
fn test_parse_tuple_pattern_with_missing_close_parenthesis() {
    let test = TestParser::new("(first, second");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_eq!(parser.errors.len(), 1);

    assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields } => {
        assert_eq!(fields.len(), 2);
    });
}

#[test]
fn test_parse_computed_pattern_field_with_missing_close_bracket() {
    let test = TestParser::new("{ [key: value }");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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
    for (source, access, exclusion) in [
        ("&_", Mutability::Mutable, None),
        ("&readonly _", Mutability::Immutable, None),
        (
            "&exclusive _",
            Mutability::Mutable,
            Some(Exclusivity::Exclusive),
        ),
        (
            "&readonly exclusive _",
            Mutability::Immutable,
            Some(Exclusivity::Exclusive),
        ),
        (
            "&exclusive readonly _",
            Mutability::Immutable,
            Some(Exclusivity::Exclusive),
        ),
        (
            "&exclusive const _",
            Mutability::Immutable,
            Some(Exclusivity::Exclusive),
        ),
    ] {
        let test = TestParser::new(source);
        let mut parser = test.prepare();
        let pattern_id = parser.parse_pattern().unwrap();

        assert_node!(parser.tree, pattern_id, Pattern::BorrowOf { mutability, exclusivity, right } => {
            assert_eq!((*mutability, *exclusivity), (Some(access), exclusion));
            assert_node!(parser.tree, *right, Pattern::Wildcard);
        });
        assert_eq!(parser.peek_token_type(), TokenType::End);
        test.assert_no_errors(&parser);
    }

    // &1
    let test = TestParser::new("&1");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();
    // &
    assert_node!(parser.tree, pattern_id, Pattern::BorrowOf { mutability: Some(mutability), right, exclusivity: None } => {
        assert_eq!(*mutability, Mutability::Mutable);
        // 1
        assert_node!(parser.tree, *right, Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        });
    })
}

#[test]
fn test_parse_pattern_reference_chain_compact() {
    let test = TestParser::new("&&item");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::BorrowOf { mutability, right, exclusivity: None } => {
        assert_eq!(*mutability, Some(Mutability::Mutable));
        assert_node!(parser.tree, *right, Pattern::BorrowOf { mutability, right, exclusivity: None } => {
            assert_eq!(*mutability, Some(Mutability::Mutable));
            assert_node!(parser.tree, *right, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "item");
            });
        });
    });

    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_value() {
    // ^x
    let test = TestParser::new("^x");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();
    assert_node!(parser.tree, pattern_id, Pattern::MoveOf { mutability: Some(mutability), right } => {
        assert_eq!(*mutability, Mutability::Mutable);
        assert_node!(parser.tree, *right, Pattern::Binding { name, pattern: None } => {
            assert_string!(parser, *name, "x");
        });
    });
}

#[test]
fn test_parse_pattern_undefined_literal() {
    let test = TestParser::new("undefined");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    // undefined
    assert_node!(parser.tree, pattern_id, Pattern::Expression { value } => {
        assert_node!(parser.tree, *value, Expression::Literal(Literal::Undefined));
    });
}

#[test]
fn test_parse_pattern_dereference_binding() {
    let test = TestParser::new("*value");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Binding { name, pattern: None } => {
            assert_string!(parser, *name, "value");
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_wildcard() {
    let test = TestParser::new("*_");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Wildcard);
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_literal() {
    let test = TestParser::new("*42");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Expression { value } => {
            assert_integer_expression(&parser.tree, *value, 42);
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_range() {
    let test = TestParser::new("*0..10");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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
    let test = TestParser::new("*(x, y)");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Tuple { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "x");
                });
            });
            assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "y");
                });
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_dereference_sequence() {
    let test = TestParser::new("*[head, ...tail]");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::Sequence { fields } => {
            assert_eq!(fields.len(), 2);
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "head");
                });
            });
            assert_node!(parser.tree, fields[1], PatternField::Rest { pattern: Some(pattern) } => {
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
    let test = TestParser::new("*{ x, y }");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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

/// Parse a dereferenced nominal tuple pattern.
#[test]
fn test_parse_pattern_dereference_nominal_tuple() {
    let test = TestParser::new("*Result.Ok(value)");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::NominalTuple { ty, fields } => {
            assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
                assert_path!(parser, *path, "Result.Ok");
            });
            assert_eq!(fields.len(), 1);
            assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                    assert_string!(parser, *name, "value");
                });
            });
        });
    });
    test.assert_no_errors(&parser);
}

/// Parse a dereferenced nominal object pattern.
#[test]
fn test_parse_pattern_dereference_nominal_object() {
    let test = TestParser::new("*Point { x, y }");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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
    let test = TestParser::new("*&readonly inner");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::BorrowOf { mutability: Some(mutability), right, exclusivity: None } => {
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
    let test = TestParser::new("*^item");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::DereferenceOf { right } => {
        assert_node!(parser.tree, *right, Pattern::MoveOf { mutability: Some(mutability), right } => {
            assert_eq!(*mutability, Mutability::Mutable);
            assert_node!(parser.tree, *right, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "item");
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_identifier() {
    let test = TestParser::new("x");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();
    assert_node!(parser.tree, pattern_id, Pattern::Binding { name, pattern: None } => {
        assert_string!(parser, *name, "x");
    });
}

#[test]
fn test_parse_pattern_path() {
    let test = TestParser::new("MyEnum.A");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();
    assert_node!(parser.tree, pattern_id, Pattern::Expression { value } => {
        assert_expression_path!(parser, parser.tree.get(*value), "MyEnum.A");
    });
}

#[test]
fn test_parse_pattern_range_half_open() {
    let test = TestParser::new("0..10");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_integer_expression(&parser.tree, *start, 0);
        assert_integer_expression(&parser.tree, *end, 10);
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_inclusive() {
    let test = TestParser::new("0..=10");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Inclusive);
        assert_integer_expression(&parser.tree, *start, 0);
        assert_integer_expression(&parser.tree, *end, 10);
    });
    test.assert_no_errors(&parser);
}

/// Fold a signed integer into one range-pattern bound.
#[test]
fn test_parse_signed_pattern_range() {
    let test = TestParser::new("-10..=-5");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Inclusive);
        assert_integer_expression(&parser.tree, *start, -10);
        assert_integer_expression(&parser.tree, *end, -5);
    });
    test.assert_no_errors(&parser);
}

/// Fold explicit signs into every numeric literal pattern family.
#[test]
fn test_parse_signed_numeric_patterns() {
    let test = TestParser::new("-1 | +2n | -3.5");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
        assert_eq!(patterns.len(), 3);
        assert_node!(parser.tree, patterns[0], Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(-1)));
        });
        assert_node!(parser.tree, patterns[1], Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Bigint(2)));
        });
        assert_node!(parser.tree, patterns[2], Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Float(value)) => {
                assert_eq!(*value, -3.5);
            });
        });
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_open_ended() {
    let test = TestParser::new("0..");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: None, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_integer_expression(&parser.tree, *start, 0);
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_startless() {
    let test = TestParser::new("..10");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: None, end: Some(end), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_integer_expression(&parser.tree, *end, 10);
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_startless_inclusive() {
    let test = TestParser::new("..=10");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: None, end: Some(end), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Inclusive);
        assert_integer_expression(&parser.tree, *end, 10);
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_identifier_bounds() {
    let test = TestParser::new("MIN..MAX");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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
    let test = TestParser::new("Limits.Min..=Limits.Max");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: Some(end), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Inclusive);
        assert_expression_path!(parser, parser.tree.get(*start), "Limits.Min");
        assert_expression_path!(parser, parser.tree.get(*end), "Limits.Max");
    });
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_range_union() {
    let test = TestParser::new("0..10 | 20..30");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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
    let test = TestParser::new("0.. => value");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(start), end: None, end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
        assert_integer_expression(&parser.tree, *start, 0);
    });
    assert!(parser.errors.is_empty());
    assert!(parser.peek_is(TokenType::ArrowWide));
}

#[test]
fn test_parse_pattern_range_recovers_inclusive_end_before_union() {
    let test = TestParser::new("0..= | 1");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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
    let test = TestParser::new("..");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_node!(parser.tree, pattern_id, Pattern::Range { start: None, end: Some(_), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Open);
    });
}

#[test]
fn test_parse_pattern_range_recovers_missing_inclusive_end() {
    let test = TestParser::new("0..=");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_node!(parser.tree, pattern_id, Pattern::Range { start: Some(_), end: Some(_), end_kind } => {
        assert_eq!(*end_kind, RangeEnd::Inclusive);
    });
}

#[test]
fn test_parse_pattern_tuple() {
    let test = TestParser::new("(x: 1, 2, y, z, ...)");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    // (x: 1, 2, y, z, ...)
    assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
        assert_eq!(fields.len(), 5);

        // x: 1
        assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: Some(pattern) } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
                });
            });
        });

        // 2
        assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
            });
        });

        // y
        assert_node!(parser.tree, fields[2], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "y");
            });
        });

        // z
        assert_node!(parser.tree, fields[3], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "z");
            });
        });

        // ...
        assert_node!(parser.tree, fields[4], PatternField::Rest { pattern: None } => {
        });
    });
}

#[test]
fn test_parse_pattern_tuple_with_path() {
    let test = TestParser::new("Result.Success(_, ...)");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    // Result.Success(_, ..)
    assert_node!(parser.tree, pattern_id, Pattern::NominalTuple { ty, fields } => {
        // Result.Success
        let main_span = parser
            .tree
            .get_main_span(*ty)
            .expect("expected nominal type main span");
        assert_eq!(parser.span_str(main_span), "Success");
        let head_span = parser
            .tree
            .get_head_span(*ty)
            .expect("expected nominal type head span");
        assert_eq!(parser.span_str(head_span), "Result");
        assert_eq!(fields.len(), 2);

        // _
        assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Wildcard);
        });

        // ..
        assert_node!(parser.tree, fields[1], PatternField::Rest { pattern: None } => {
        });
    });
}

#[test]
fn test_parse_pattern_object_field_const_alias() {
    let test = TestParser::new("{ const: value, title }");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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
fn test_parse_pattern_tuple_newline_separated() {
    let test = TestParser::new(
        "
(
    x: 1
    2,
    ...
)",
    );
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    // (x: 1, 2, ..)
    assert_node!(parser.tree, pattern_id, Pattern::Tuple { fields, .. } => {
        assert_eq!(fields.len(), 3);

        // x: 1
        assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: Some(pattern) } => {
                assert_string!(parser, *name, "x");
                assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                    assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
                });
            });
        });

        // 2
        assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
            });
        });

        // ..
        assert_node!(parser.tree, fields[2], PatternField::Rest { pattern: None } => {
        });
    });
}

#[test]
fn test_parse_pattern_union() {
    // 1 | 2 | 3
    let test = TestParser::new("1 | 2 | 3");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
        assert_eq!(patterns.len(), 3);

        // 1
        assert_node!(parser.tree, patterns[0], Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        });

        // 2
        assert_node!(parser.tree, patterns[1], Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
        });

        // 3
        assert_node!(parser.tree, patterns[2], Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(3)));
        });
    });
}

#[test]
fn test_parse_pattern_union_before_type() {
    let test = TestParser::new("success | &failure: Result");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern_before_type().unwrap();

    test.assert_no_errors(&parser);
    assert!(parser.peek_is(TokenType::Colon));
    assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
        assert_eq!(patterns.len(), 2);
        assert_node!(parser.tree, patterns[0], Pattern::Binding { name, pattern: None } => {
            assert_string!(parser, *name, "success");
        });
        assert_node!(parser.tree, patterns[1], Pattern::BorrowOf { right, .. } => {
            assert_node!(parser.tree, *right, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "failure");
            });
        });
    });
}

#[test]
fn test_parse_pattern_struct_anonymous() {
    let test = TestParser::new("{ x: 1, y, z, w: 4, ... }");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 5);

        // x: 1
        assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
            assert_name!(parser, *name, "x");
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
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
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(4)));
            });
        });

        // ..
        assert_node!(parser.tree, fields[4], PatternField::Rest { pattern: None } => {
        });
    });
}

#[test]
fn test_parse_pattern_named_default_after_comment_newline() {
    let test = TestParser::new("{d //comment\n= b}");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: Some(pattern), .. } => {
            assert_name!(parser, *name, "d");
            assert_node!(parser.tree, *pattern, Pattern::Default { pattern, value } => {
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
    let test = TestParser::new("{ 0: fieldNameOrOptions, 1: from, length: argc }");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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
fn test_parse_pattern_struct_string_name_aliases() {
    let test = TestParser::new(r#"{ "false": decorators, "true": metadata }"#);
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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
fn test_parse_pattern_struct_computed_field() {
    let test = TestParser::new("{ [key]: value }");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Computed { key, pattern } => {
            let main_span = parser
                .tree
                .get_main_span(fields[0])
                .expect("expected computed key main span");
            assert_eq!(parser.span_str(main_span), "[key]");

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
    let test = TestParser::new("Vector2 { x: 0, y }");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::NominalObject { ty, fields } => {
        assert_node!(parser.tree, *ty, TypeExpression::Reference { path, generic_arguments: _ } => {
            assert_path!(parser, *path, "Vector2");
        });
        let main_span = parser
            .tree
            .get_main_span(*ty)
            .expect("expected nominal type main span");
        assert_eq!(parser.span_str(main_span), "Vector2");
        assert_eq!(fields.len(), 2);

        // x: 0
        assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: Some(pattern), is_shorthand: false } => {
            assert_name!(parser, *name, "x");
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(0)));
            });
        });

        // y
        assert_node!(parser.tree, fields[1], PatternField::Named { name, pattern: None, is_shorthand: true } => {
            assert_name!(parser, *name, "y");
        });
    });
}

/// Parse one nominal object pattern nested inside another.
#[test]
fn test_parse_nominal_object_pattern_with_nested_nominal_object_field() {
    let test = TestParser::new("Shape.Line { start: Point { x, y }, end }");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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
    test.assert_no_errors(&parser);
}

#[test]
fn test_parse_pattern_unbound_rest() {
    let test = TestParser::new("[1, ...]");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        assert_eq!(fields.len(), 2);
        // 1
        assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
            });
        });
        // ...
        assert_node!(parser.tree, fields[1], PatternField::Rest { pattern: None } => {
        });
    });
}

/// Parse prefix and suffix fields around an unbound rest.
#[test]
fn test_parse_pattern_unbound_middle_rest() {
    let test = TestParser::new("[first, ..., last]");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        assert_eq!(fields.len(), 3);
        assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "first");
            });
        });
        assert_node!(parser.tree, fields[1], PatternField::Rest { pattern: None });
        assert_node!(parser.tree, fields[2], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "last");
            });
        });
    });

    test.assert_no_errors(&parser);
}

/// Parse a rest field with a sequence pattern.
#[test]
fn test_parse_pattern_nested_rest() {
    let test = TestParser::new("[...[x, y]]");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Rest { pattern: Some(pattern) } => {
            assert_node!(parser.tree, *pattern, Pattern::Sequence { fields } => {
                assert_eq!(fields.len(), 2);
                assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
                    assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                        assert_string!(parser, *name, "x");
                    });
                });
                assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
                    assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                        assert_string!(parser, *name, "y");
                    });
                });
            });
        });
    });
}

#[test]
fn test_parse_pattern_object_readonly_shorthand() {
    let test = TestParser::new("{ readonly }");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "readonly");
        });
    });
}

#[test]
fn test_parse_pattern_object_readonly_shorthand_with_newline() {
    let test = TestParser::new("{ readonly\n}");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 1);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, is_shorthand: true, pattern: None } => {
            assert_name!(parser, *name, "readonly");
        });
    });
}
#[test]
fn test_parse_pattern_array_readonly_identifier() {
    let test = TestParser::new("[readonly, setReadonly]");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        assert_eq!(fields.len(), 2);
        assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "readonly");
            });
        });
        assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "setReadonly");
            });
        });
    });
}

#[test]
fn test_report_pattern_object_readonly_modifier_with_name() {
    let test = TestParser::new("{ readonly value }");
    let mut parser = test.prepare();
    let error = parser.parse_pattern().unwrap_err();

    assert_eq!(parser.range_str(error.range()), "value");
}

#[test]
fn test_parse_pattern_object_rest_newline_before_terminator() {
    let test = TestParser::new("{\n  onSuccess,\n  ...rest\n}");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 2);
        assert_node!(parser.tree, fields[0], PatternField::Named { name, pattern: None, is_shorthand: true } => {
            assert_name!(parser, *name, "onSuccess");
        });
        assert_node!(parser.tree, fields[1], PatternField::Rest { pattern: Some(pattern) } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None, .. } => {
                assert_string!(parser, *name, "rest");
            });
        });
    });
}

#[test]
fn test_parse_pattern_must() {
    let test = TestParser::new("1!");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Must(inner) => {
        assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        });
    });
}

#[test]
fn test_parse_pattern_union_with_must_arms() {
    let test = TestParser::new("1! | 2!");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
        assert_eq!(patterns.len(), 2);
        // 1!
        assert_node!(parser.tree, patterns[0], Pattern::Must(inner) => {
            assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
            });
        });
        // 2!
        assert_node!(parser.tree, patterns[1], Pattern::Must(inner) => {
            assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
            });
        });
    });
}

#[test]
fn test_parse_pattern_union_with_trailing_must_arm() {
    let test = TestParser::new("1 | 2!");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Union { patterns } => {
        assert_eq!(patterns.len(), 2);
        // 1
        assert_node!(parser.tree, patterns[0], Pattern::Expression { value } => {
            assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(1)));
        });
        // 2!
        assert_node!(parser.tree, patterns[1], Pattern::Must(inner) => {
            assert_node!(parser.tree, *inner, Pattern::Expression { value } => {
                assert_node!(parser.tree, *value, Expression::Literal(Literal::Integer(2)));
            });
        });
    });
}

#[test]
fn test_parse_pattern_array_elision() {
    // [,a] - elision before 'a'
    let test = TestParser::new("[,a]");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        assert_eq!(fields.len(), 2);

        // elision (empty slot)
        assert_node!(parser.tree, fields[0], PatternField::Elision);

        // a
        assert_node!(parser.tree, fields[1], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "a");
            });
        });
    });
}

#[test]
fn test_parse_pattern_array_multiple_elisions() {
    // [,,a] - two elisions before 'a'
    let test = TestParser::new("[,,a]");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        assert_eq!(fields.len(), 3);

        // first elision
        assert_node!(parser.tree, fields[0], PatternField::Elision);

        // second elision
        assert_node!(parser.tree, fields[1], PatternField::Elision);

        // a
        assert_node!(parser.tree, fields[2], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "a");
            });
        });
    });
}

#[test]
fn test_parse_pattern_array_trailing_elision() {
    // [a,] - element followed by trailing comma (not elision)
    let test = TestParser::new("[a,]");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Sequence { fields } => {
        // trailing comma doesn't create elision, just 'a'
        assert_eq!(fields.len(), 1);

        // a
        assert_node!(parser.tree, fields[0], PatternField::Positional { pattern } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { name, pattern: None } => {
                assert_string!(parser, *name, "a");
            });
        });
    });
}

#[test]
fn test_parse_object_pattern_defaults_do_not_consume_following_fields() {
    // {a,b=1,c:d,e:f=2,[g]:[h]}
    let test = TestParser::new("{a,b=1,c:d,e:f=2,[g]:[h]}");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 5);

        // b=1
        assert_node!(parser.tree, fields[1], PatternField::Named { is_shorthand: true, pattern: Some(pattern), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Default { .. });
        });

        // c:d
        assert_node!(parser.tree, fields[2], PatternField::Named { is_shorthand: false, pattern: Some(pattern), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Binding { .. });
        });

        // e:f=2
        assert_node!(parser.tree, fields[3], PatternField::Named { is_shorthand: false, pattern: Some(pattern), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Default { .. });
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
    let test = TestParser::new("{c, d:e=1, [f]:g=2, h=i}");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

    assert_node!(parser.tree, pattern_id, Pattern::Object { fields } => {
        assert_eq!(fields.len(), 4);

        // d:e=1
        assert_node!(parser.tree, fields[1], PatternField::Named { is_shorthand: false, pattern: Some(pattern), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Default { .. });
        });

        // [f]:g=2
        assert_node!(parser.tree, fields[2], PatternField::Computed { pattern, .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Default { .. });
        });

        // h=i
        assert_node!(parser.tree, fields[3], PatternField::Named { is_shorthand: true, pattern: Some(pattern), .. } => {
            assert_node!(parser.tree, *pattern, Pattern::Default { .. });
        });
    });
}

#[test]
fn test_parse_object_pattern_computed_field_with_newline_after_colon() {
    // { [key]:\nvalue }
    let test = TestParser::new("{ [key]:\nvalue }");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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
    let test = TestParser::new("{ source:\ntarget }");
    let mut parser = test.prepare();
    let pattern_id = parser.parse_pattern().unwrap();

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
