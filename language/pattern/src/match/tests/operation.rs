use tspp_dir as dir;

use crate::tests::TestMatcher;

/// Compose disjunction and negation with one structural fragment.
#[test]
fn test_match_boolean_operations() {
    TestMatcher::new(
        "target($VALUE)",
        r#"
target(first)
target(second)
"#,
    )
    .any(&[dir::NodeType::Declaration, dir::NodeType::Expression])
    .not(dir::NodeType::Declaration)
    .assert(
        r#"
target(first)
^^^^^^^^^^^^^ match VALUE.node="first"
target(second)
^^^^^^^^^^^^^^ match VALUE.node="second"
"#,
    );
}
