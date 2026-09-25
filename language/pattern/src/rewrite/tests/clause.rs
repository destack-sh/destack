use tspp_dir as dir;

use crate::tests::TestRewriter;

/// Rewrite a catch clause selected from its required parse context.
#[test]
fn test_rewrite_catch_clause() {
    TestRewriter::context(
        "try {} catch ($ERROR: $TYPE) { $BODY }",
        "try {} catch ($ERROR: Wrapped<$TYPE>) { report($BODY); $BODY }",
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
} catch (error: Wrapped<NetworkError>) {
    report(recover(error));
    recover(error)
}
"#,
    );
}
