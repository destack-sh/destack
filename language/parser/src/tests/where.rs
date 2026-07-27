use destack_dir::{
    CommentKind, Declaration, Expression, IntegerType, NodeType, TokenType, TypeExpression,
    TypeLiteral, WhereClause, WhereRelation,
};
use destack_source::{NodeSpanRegion, NodeSpanType};

use crate::{TestParser, assert_comment, assert_expression_path, assert_node, assert_path};

/// Parse a repeated Pattern placeholder as one complete where clause.
#[test]
fn test_parse_pattern_where_clause_placeholder() {
    let test = TestParser::new("function example<T>(): void where $$$CLAUSES {}");
    let mut parser = test.prepare_pattern();
    let roots = parser.parse();

    TestParser::assert_no_errors(&parser);
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

#[test]
fn test_parse_where_type_assertion() {
    let test = TestParser::new("where T: int32");
    let mut parser = test.prepare();
    let clauses = parser.parse_where(Default::default()).unwrap();

    // where T: int32
    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation, left, right } => {
        assert_eq!(*relation, WhereRelation::Satisfies);
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_node!(parser.tree, *right, TypeExpression::Literal { value } => {
            assert_eq!(
                *value,
                TypeLiteral::Integer(IntegerType::Fixed { width: 32, is_signed: true,
                })
            );
        });
    });
}

#[test]
fn test_parse_where_negative_capability() {
    let test = TestParser::new("where T: !Unpin");
    let mut parser = test.prepare();
    let clauses = parser.parse_where(Default::default()).unwrap();

    TestParser::assert_no_errors(&parser);

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

#[test]
fn test_parse_where_equality_constraint() {
    let test = TestParser::new("where T.Output == U");
    let mut parser = test.prepare();
    let clauses = parser.parse_where(Default::default()).unwrap();

    TestParser::assert_no_errors(&parser);

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation, left, right } => {
        assert_eq!(*relation, WhereRelation::Equals);
        assert_expression_path!(parser, parser.tree.get(*left), "T.Output");
        assert_expression_path!(parser, parser.tree.get(*right), "U");
    });

    let type_range = parser
        .tree
        .get_side_span(clauses[0], NodeSpanType::Region(NodeSpanRegion::Type))
        .expect("expected where type span");
    assert_eq!(parser.span_str(type_range), "== U");
}

#[test]
fn test_parse_where_multiple_clauses() {
    let input = "where T: Numeric, U: Copy, V: Comparable";
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let clauses = parser.parse_where(Default::default()).unwrap();

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

#[test]
fn test_parse_where_parenthesized_multiline() {
    let input = r##"where (
  T: Numeric
  U: Copy,
  V: Comparable
)"##;
    let test = TestParser::new(input);
    let mut parser = test.prepare();
    let clauses = parser.parse_where(Default::default()).unwrap();

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

#[test]
fn test_parse_parenthesized_where_with_missing_close_parenthesis() {
    let test = TestParser::new("where (T: Numeric, U: Copy");
    let mut parser = test.prepare();
    let clauses = parser.parse_where(Default::default()).unwrap();

    TestParser::assert_errors(
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

/// Ensure where clauses record main and type spans.
#[test]
fn test_where_clause_spans() {
    let test = TestParser::new("where T: Numeric");
    let mut parser = test.prepare();
    let clauses = parser.parse_where(Default::default()).unwrap();
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

#[test]
fn test_where_clause_constraint_with_boundary_comment() {
    let source = "where T: // bound-note\nNumeric";
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let clauses = parser.parse_where(Default::default()).unwrap();
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

#[test]
fn test_parse_where_type_expression_left() {
    let test = TestParser::new("where BaseOf<Borrowed>: Clone");
    let mut parser = test.prepare();
    let clauses = parser.parse_where(Default::default()).unwrap();

    TestParser::assert_no_errors(&parser);

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "BaseOf");
        assert_expression_path!(parser, parser.tree.get(*right), "Clone");
    });
}

#[test]
fn test_recover_where_implements_separator() {
    let test = TestParser::new("where T implements Clone");
    let mut parser = test.prepare();
    let clauses = parser.parse_where(Default::default()).unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.range_str(parser.errors[0].range()), "implements");

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_expression_path!(parser, parser.tree.get(*right), "Clone");
    });
}

#[test]
fn test_recover_where_extends_separator() {
    let test = TestParser::new("where T extends Clone");
    let mut parser = test.prepare();
    let clauses = parser.parse_where(Default::default()).unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.range_str(parser.errors[0].range()), "extends");

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { relation: _, left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_expression_path!(parser, parser.tree.get(*right), "Clone");
    });
}
