use destack_dir::{CommentKind, IntegerType, NodeType, TypeExpression, TypeLiteral, WhereClause};
use destack_source::{NodeSpanRegion, NodeSpanType};

use crate::{TestParser, assert_comment, assert_expression_path, assert_node, assert_path};

#[test]
fn test_parse_where_type_assertion() {
    let mut test = TestParser::new("where T: int32");
    let mut parser = test.prepare();
    let clauses = parser.eat_where().unwrap();

    // where T: int32
    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
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
    let mut test = TestParser::new("where T: !Unpin");
    let mut parser = test.prepare();
    let clauses = parser.eat_where().unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_node!(parser.tree, *right, TypeExpression::Not { target_type } => {
            assert_node!(parser.tree, *target_type, TypeExpression::Reference { path, .. } => {
                assert_path!(parser, *path, "Unpin");
            });
        });
    });
}

#[test]
fn test_parse_where_multiple_clauses() {
    let input = "where T: Numeric, U: Copy, V: Comparable";
    let mut test = TestParser::new(input);
    let mut parser = test.prepare();
    let clauses = parser.eat_where().unwrap();

    assert_eq!(clauses.len(), 3);

    assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Numeric");
        });
    });
    assert_node!(parser.tree, clauses[1], WhereClause { left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "U");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Copy");
        });
    });
    assert_node!(parser.tree, clauses[2], WhereClause { left, right } => {
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
    let mut test = TestParser::new(input);
    let mut parser = test.prepare();
    let clauses = parser.eat_where().unwrap();

    assert_eq!(clauses.len(), 3);

    assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Numeric");
        });
    });
    assert_node!(parser.tree, clauses[1], WhereClause { left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "U");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Copy");
        });
    });
    assert_node!(parser.tree, clauses[2], WhereClause { left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "V");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Comparable");
        });
    });
}

#[test]
fn test_parse_parenthesized_where_with_missing_close_parenthesis() {
    let mut test = TestParser::new("where (T: Numeric, U: Copy");
    let mut parser = test.prepare();
    let clauses = parser.eat_where().unwrap();

    test.assert_error_leaves(&parser, &[(Some(NodeType::WhereClause), None, "")]);
    assert_eq!(clauses.len(), 2);

    // T: Numeric
    assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Numeric");
        });
    });

    // U: Copy
    assert_node!(parser.tree, clauses[1], WhereClause { left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "U");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Copy");
        });
    });
}

/// Ensure where clauses record main and type spans.
#[test]
fn test_where_clause_spans() {
    let mut test = TestParser::new("where T: Numeric");
    let mut parser = test.prepare();
    let clauses = parser.eat_where().unwrap();
    let clause_id = clauses[0];

    // main span
    let main_span = parser
        .tree
        .get_main_span(clause_id)
        .expect("expected main span");
    assert_eq!(parser.get_span_str(main_span), "T");

    // type span
    let type_span = parser
        .tree
        .get_side_span(clause_id, NodeSpanType::Region(NodeSpanRegion::Type))
        .expect("expected type span");
    assert_eq!(parser.get_span_str(type_span), ": Numeric");
}

#[test]
fn test_where_clause_constraint_with_boundary_comment() {
    let source = "where T: // bound-note\nNumeric";
    let mut test = TestParser::new(source);
    let mut parser = test.prepare();
    let clauses = parser.eat_where().unwrap();
    parser.attach_comments();

    assert_eq!(clauses.len(), 1);

    // T: Numeric
    assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_node!(parser.tree, *right, TypeExpression::Reference { path, .. } => {
            assert_path!(parser, *path, "Numeric");
        });
    });

    // : // bound-note\nNumeric
    let type_span = parser
        .tree
        .get_side_span(clauses[0], NodeSpanType::Region(NodeSpanRegion::Type))
        .expect("expected where type span");
    assert_eq!(parser.get_span_str(type_span), ": // bound-note\nNumeric");

    // // bound-note
    assert_eq!(parser.tree.comments().len(), 1);
    assert_comment!(parser, 0, CommentKind::Line, "bound-note");
}

#[test]
fn test_parse_where_type_expression_left() {
    let mut test = TestParser::new("where BaseOf<Borrowed>: Clone");
    let mut parser = test.prepare();
    let clauses = parser.eat_where().unwrap();

    test.assert_no_errors(&parser);

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "BaseOf");
        assert_expression_path!(parser, parser.tree.get(*right), "Clone");
    });
}

#[test]
fn test_recover_where_implements_separator() {
    let mut test = TestParser::new("where T implements Clone");
    let mut parser = test.prepare();
    let clauses = parser.eat_where().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(
        parser.get_span_str(parser.errors[0].leaf_span()),
        "implements"
    );

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_expression_path!(parser, parser.tree.get(*right), "Clone");
    });
}

#[test]
fn test_recover_where_extends_separator() {
    let mut test = TestParser::new("where T extends Clone");
    let mut parser = test.prepare();
    let clauses = parser.eat_where().unwrap();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.get_span_str(parser.errors[0].leaf_span()), "extends");

    assert_eq!(clauses.len(), 1);
    assert_node!(parser.tree, clauses[0], WhereClause { left, right } => {
        assert_expression_path!(parser, parser.tree.get(*left), "T");
        assert_expression_path!(parser, parser.tree.get(*right), "Clone");
    });
}
