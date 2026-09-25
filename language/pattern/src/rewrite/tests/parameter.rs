use tspp_dir as dir;

use crate::tests::TestRewriter;

/// Rewrite callable parameters selected from declaration context.
#[test]
fn test_rewrite_parameter() {
    TestRewriter::context(
        "function f($NAME: $TYPE): void {}",
        "function f($NAME?: $TYPE): void {}",
        dir::NodeType::Parameter,
        "function consume(value: string): void {}\n",
    )
    .assert("function consume(value?: string): void {}\n");
}
