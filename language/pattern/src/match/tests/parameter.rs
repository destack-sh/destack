use tspp_dir as dir;

use crate::tests::TestMatcher;

/// Match callable parameters selected from declaration context.
#[test]
fn test_match_parameter() {
    TestMatcher::context(
        "function f($NAME: $TYPE): void {}",
        dir::NodeType::Parameter,
        r#"
function first(value: string): void {}
function second(count: int32): void {}
"#,
    )
    .assert(
        r#"
function first(value: string): void {}
               ^^^^^^^^^^^^^ match NAME.name="value" TYPE.node="string"
function second(count: int32): void {}
                ^^^^^^^^^^^^ match NAME.name="count" TYPE.node="int32"
"#,
    );
}
