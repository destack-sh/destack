use crate::tests::TestRewriter;

/// Reject overlapping selected roots without an explicit overlap policy.
#[test]
fn test_reject_overlapping_rewrites() {
    TestRewriter::new(
        "$CALLEE($VALUE)",
        "wrap($CALLEE($VALUE))",
        "outer(inner(value))",
    )
    .assert_diagnostics(
        r#"
error[overlapping-rewrites]: selected rewrites overlap
 ──▶ tspp:pattern-test:1:7
  │
1 │ outer(inner(value))
  │ ------^^^^^^^^^^^^- this rewrite overlaps another selected rewrite
  │ │
  │ the other rewrite was selected here
  │

 = help: make the pattern select non-overlapping roots
for more information about an error, run `tspp explain overlapping-rewrites`
"#,
    );
}
