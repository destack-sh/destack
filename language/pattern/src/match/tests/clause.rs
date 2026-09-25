use tspp_dir as dir;

use crate::tests::TestMatcher;

/// Match a catch clause selected from its required parse context.
#[test]
fn test_match_catch_clause() {
    TestMatcher::context(
        "try {} catch ($ERROR: $TYPE) { $BODY }",
        dir::NodeType::Catch,
        r#"
try {
    work()
} catch (error: NetworkError) {
    recover(error)
}
"#,
    )
    .assert(
        r#"
try {
    work()
} catch (error: NetworkError) {
  ^ match:start ERROR.node="error" TYPE.node="NetworkError" BODY.node="recover(error)"
    recover(error)
}
^ match:end
"#,
    );
}
