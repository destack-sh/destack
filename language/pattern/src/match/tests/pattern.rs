use destack_dir as dir;

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

/// Match borrow patterns by access and exclusivity in either qualifier order.
#[test]
fn test_match_borrow_qualifiers() {
    TestMatcher::context(
        "const &readonly exclusive value = source;",
        dir::NodeType::Pattern,
        r#"
const &readonly exclusive value = first;
const &exclusive readonly value = second;
const &readonly value = third;
const &exclusive value = fourth;
"#,
    )
    .assert(
        r#"
const &readonly exclusive value = first;
      ^^^^^^^^^^^^^^^^^^^^^^^^^ match
const &exclusive readonly value = second;
      ^^^^^^^^^^^^^^^^^^^^^^^^^ match
const &readonly value = third;
const &exclusive value = fourth;
"#,
    );
}
