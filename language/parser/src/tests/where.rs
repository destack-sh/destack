use tspp_dir::{
    CommentKind, Declaration, Expression, IntegerType, NodeType, TokenType, TypeExpression,
    TypeLiteral, WhereClause, WhereRelation,
};
use tspp_source::{NodeSpanRegion, NodeSpanType};

use crate::{
    TestParser, assert_comment, assert_expression_path, assert_node, assert_path, assert_string,
};

/// Parse a repeated Pattern placeholder as one complete where clause.
#[test]
fn test_parse_pattern_where_clause_placeholder() {
    let test = TestParser::new("function example<T>(): void where $$$CLAUSES {}");
    let mut parser = test.prepare_pattern();
    let roots = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_node!(parser.tree, roots[0], Expression::Declaration(value) => {
        assert_node!(parser.tree, *value, Declaration::Function(declaration) => {
            let clauses = &declaration.signature.where_clauses;
            assert_eq!(clauses.len(), 1);
            assert_node!(parser.tree, clauses[0], WhereClause { left, right, .. } => {
                assert_node!(parser.tree, *left, TypeExpression::Error);
                assert_node!(parser.tree, *right, TypeExpression::Error);
            });
        });
    });
}

/// Parse a where clause bounding a parameter by a keyword type.
#[test]
fn test_parse_where_type_assertion() {
    let test = TestParser::new("where T: int32");
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();

    // where T: int32
    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation, left, right } => {
        assert_eq!(*relation, WhereRelation::Satisfies);
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_node!(parser.tree, *right, TypeExpression::Keyword { value } => {
            assert_eq!(
                *value,
                TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                })
            );
        });
    });
}

/// Parse a where clause bounding a parameter by a negated capability.
#[test]
fn test_parse_where_negative_capability() {
    let test = TestParser::new("where T: !Unpin");
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_node!(parser.tree, *right, TypeExpression::Not { target_type } => {
            assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Unpin");
            });
        });
    });
}

/// Parse a where clause equating a projected type with a parameter.
#[test]
fn test_parse_where_equality_constraint() {
    let test = TestParser::new("where T.Output == U");
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation, left, right } => {
        assert_eq!(*relation, WhereRelation::Equal);
        assert_expression_path!(parser, parser.tree.get(*left), "T.Output");
        assert_expression_path!(parser, parser.tree.get(*right), "U");
    });

    let type_range = parser
        .tree
        .get_side_span(clauses[0], NodeSpanType::Region(NodeSpanRegion::Type))
        .expect("expected where type span");
    assert_eq!(parser.span_str(type_range), "== U");
}

/// Parse several comma separated where clauses.
#[test]
fn test_parse_where_multiple_clauses() {
    let input = "where T: Numeric, U: Copy, V: Comparable";
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();

    assert_eq!(clauses.len(), 3);

    assert_node!(parser.tree, clauses[0], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Numeric");
        });
    });
    assert_node!(parser.tree, clauses[1], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "U");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Copy");
        });
    });
    assert_node!(parser.tree, clauses[2], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "V");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Comparable");
        });
    });
}

/// Parse parenthesized where clauses spread over several lines.
#[test]
fn test_parse_where_parenthesized_multiline() {
    let input = r##"where (
  T: Numeric
  U: Copy,
  V: Comparable
)"##;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();

    assert_eq!(clauses.len(), 3);

    assert_node!(parser.tree, clauses[0], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Numeric");
        });
    });
    assert_node!(parser.tree, clauses[1], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "U");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Copy");
        });
    });
    assert_node!(parser.tree, clauses[2], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "V");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Comparable");
        });
    });
}

/// Recover the clauses of a parenthesized where list missing its closing parenthesis.
#[test]
fn test_parse_parenthesized_where_with_missing_close_parenthesis() {
    let test = TestParser::new("where (T: Numeric, U: Copy");
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();

    test.assert_errors(
        &parser,
        &[(
            Some(NodeType::WhereClause),
            Some(TokenType::End),
            Some(TokenType::CloseParenthesis),
            "",
        )],
    );
    assert_eq!(clauses.len(), 2);

    // T: Numeric
    assert_node!(parser.tree, clauses[0], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Numeric");
        });
    });

    // U: Copy
    assert_node!(parser.tree, clauses[1], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "U");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Copy");
        });
    });
}

/// Where clauses record their main span and their type span.
#[test]
fn test_where_clause_spans() {
    let test = TestParser::new("where T: Numeric");
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();
    let clause_id = clauses[0];

    // main span
    let main_span = parser
        .tree
        .get_main_span(clause_id)
        .expect("expected main span");
    assert_eq!(parser.span_str(main_span), "T");

    // type span
    let type_range = parser
        .tree
        .get_side_span(clause_id, NodeSpanType::Region(NodeSpanRegion::Type))
        .expect("expected type span");
    assert_eq!(parser.span_str(type_range), ": Numeric");
}

/// Parse a where clause holding a line comment before its bound.
#[test]
fn test_where_clause_constraint_with_interleaved_comment() {
    let source = "where T: // bound-note\nNumeric";
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();
    parser.finalize_comments();

    assert_eq!(clauses.len(), 1);

    // T: Numeric
    assert_node!(parser.tree, clauses[0], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Numeric");
        });
    });

    // : // bound-note\nNumeric
    let type_range = parser
        .tree
        .get_side_span(clauses[0], NodeSpanType::Region(NodeSpanRegion::Type))
        .expect("expected where type span");
    assert_eq!(parser.span_str(type_range), ": // bound-note\nNumeric");

    // // bound-note
    assert_eq!(parser.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "bound-note");
}

/// Parse a where clause whose left side is a type application.
#[test]
fn test_parse_where_type_expression_left() {
    let test = TestParser::new("where BaseOf<Borrowed>: Clone");
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "BaseOf");
        assert_expression_path!(parser, parser.tree.get(*right), "Clone");
    });
}

/// Parse one tick outlives clause with tick operands on both sides.
#[test]
fn test_parse_where_lifetime_outlives() {
    let test = TestParser::new("where 'a: 'b");
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation, left, right } => {
        assert_eq!(*relation, WhereRelation::Satisfies);
        assert_node!(parser.tree, *left, TypeExpression::Lifetime { name } => {
            assert_string!(parser, *name, "'a");
        });
        assert_node!(parser.tree, *right, TypeExpression::Lifetime { name } => {
            assert_string!(parser, *name, "'b");
        });
    });
}

/// Parse a region meet as the right side of one where clause.
#[test]
fn test_parse_where_region_meet_bound() {
    let test = TestParser::new("where 'a: 'b & S");
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation, left, right } => {
        assert_eq!(*relation, WhereRelation::Satisfies);
        assert_node!(parser.tree, *left, TypeExpression::Lifetime { name } => {
            assert_string!(parser, *name, "'a");
        });
        assert_node!(parser.tree, *right, TypeExpression::Intersection { elements } => {
            assert_eq!(elements.len(), 2);
            assert_node!(parser.tree, elements[0], TypeExpression::Lifetime { name } => {
                assert_string!(parser, *name, "'b");
            });
            assert_node!(parser.tree, elements[1], TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "S");
            });
        });
    });
}

/// Recover a where clause written with implements in place of a colon.
#[test]
fn test_recover_where_implements_separator() {
    let test = TestParser::new("where T implements Clone");
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.range_str(parser.errors[0].range()), "implements");

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_expression_path!(parser, parser.tree.get(*right), "Clone");
    });
}

/// Recover a where clause written with extends in place of a colon.
#[test]
fn test_recover_where_extends_separator() {
    let test = TestParser::new("where T extends Clone");
    let mut parser = test.prepare();
    let clauses = parser.parse_where().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.range_str(parser.errors[0].range()), "extends");

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_expression_path!(parser, parser.tree.get(*right), "Clone");
    });
}
