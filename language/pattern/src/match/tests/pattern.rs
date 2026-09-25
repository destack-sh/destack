use tspp_dir as dir;

use crate::tests::TestMatcher;

/// Match zero or more fields inside a nominal object pattern.
#[test]
fn test_match_pattern_fields() {
    TestMatcher::context(
        "match (value) { Point { $$$FIELDS } => $BODY }",
        dir::NodeType::MatchArm,
        r#"
match (point) {
    Point {} => empty;
    Point { x, y } => x
}
"#,
    )
    .assert(
        r#"
match (point) {
    Point {} => empty;
    ^^^^^^^^^^^^^^^^^ match FIELDS.nodes=[] BODY.node="empty"
    Point { x, y } => x
    ^^^^^^^^^^^^^^^^^^^ match FIELDS.nodes=["x", "y"] BODY.node="x"
}
"#,
    );
}

/// Match borrow patterns by access.
#[test]
fn test_match_borrow_qualifiers() {
    TestMatcher::context(
        "const &immutable value = source;",
        dir::NodeType::Pattern,
        r#"
const &immutable value = first;
const &value = second;
const &readonly value = third;
const &exclusive value = fourth;
"#,
    )
    .assert(
        r#"
const &immutable value = first;
      ^^^^^^^^^^^^^^^^ match
const &value = second;
const &readonly value = third;
const &exclusive value = fourth;
"#,
    );
}
