use crate::tests::TestMatcher;

/// Return enclosing and nested matches in source order.
#[test]
fn test_order_nested_matches() {
    TestMatcher::new(
        "$CALLEE($VALUE)",
        r#"
outer(inner(value))
"#,
    )
    .assert(
        r#"
outer(inner(value))
^^^^^^^^^^^^^^^^^^^ match CALLEE.node="outer" VALUE.node="inner(value)"
      ^^^^^^^^^^^^ match CALLEE.node="inner" VALUE.node="value"
"#,
    );
}
