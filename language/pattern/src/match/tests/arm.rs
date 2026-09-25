use tspp_dir as dir;

use crate::tests::TestMatcher;

/// Match a selected arm across pattern and expression node families.
#[test]
fn test_match_expression_arm() {
    TestMatcher::context(
        "match (value) { Err($ERROR) => $BODY }",
        dir::NodeType::MatchArm,
        r#"
match (result) {
    Err(error) => recover(error);
    Ok(value) => value
}
"#,
    )
    .assert(
        r#"
match (result) {
    Err(error) => recover(error);
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match ERROR.node="error" BODY.node="recover(error)"
    Ok(value) => value
}
"#,
    );
}

/// Match nominal object patterns and concrete named fields.
#[test]
fn test_match_nominal_object_arm() {
    TestMatcher::context(
        "match (value) { Point { x: $X, y: 0 } => $BODY }",
        dir::NodeType::MatchArm,
        r#"
match (point) {
    Point { x: left, y: 0 } => left;
    Point { x: right, y: 1 } => right
}
"#,
    )
    .assert(
        r#"
match (point) {
    Point { x: left, y: 0 } => left;
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match X.node="left" BODY.node="left"
    Point { x: right, y: 1 } => right
}
"#,
    );
}
