use tspp_dir as dir;

use crate::tests::TestMatcher;

/// Match object assignment patterns and their fields.
#[test]
fn test_match_assignment_pattern() {
    TestMatcher::context(
        "({ value: $TARGET, $$$REST } = source)",
        dir::NodeType::AssignPattern,
        r#"
({ value: first, other } = source)
({ value: second } = source)
({ other } = source)
"#,
    )
    .assert(
        r#"
({ value: first, other } = source)
 ^^^^^^^^^^^^^^^^^^^^^^^ match TARGET.node="first" REST.nodes=["other"]
({ value: second } = source)
 ^^^^^^^^^^^^^^^^^ match TARGET.node="second" REST.nodes=[]
({ other } = source)
"#,
    );
}
