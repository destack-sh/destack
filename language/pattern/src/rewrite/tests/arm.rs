use tspp_dir as dir;

use crate::tests::TestRewriter;

/// Rewrite one selected match arm with captures from multiple node families.
#[test]
fn test_rewrite_match_arm() {
    TestRewriter::context(
        "match (value) { Err($ERROR) => $BODY }",
        "match (value) { Err($ERROR) => log($BODY) }",
        dir::NodeType::MatchArm,
        "match (result) { Err(error) => recover(error); Ok(value) => value }",
    )
    .assert(
        r#"
match (result) {
    Err(error) => log(recover(error))
    Ok(value) => value
}
"#,
    );
}
