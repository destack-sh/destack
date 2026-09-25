use tspp_dir as dir;

use crate::tests::TestRewriter;

/// Preserve repeated destructuring fields through a contextual rewrite.
#[test]
fn test_rewrite_pattern_fields() {
    TestRewriter::context(
        "match (value) { Point { $$$FIELDS } => $BODY }",
        "match (value) { Point { $$$FIELDS } => trace($BODY) }",
        dir::NodeType::MatchArm,
        "match (point) { Point {} => empty; Point { x, y } => x }",
    )
    .assert(
        r#"
match (point) {
    Point {} => trace(empty)
    Point { x, y } => trace(x)
}
"#,
    );
}
