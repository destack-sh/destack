use super::assert_format_eq;

/// Format value and slice allocations with direct site identities.
#[test]
fn test_format_new() {
    assert_format_eq(
        r#"
function f0(): t0 {
    new.local.managed.zeroed r1, a0
new.local.managed.slice.uninit r2:r3, a1,r0
return r1
}
"#,
        r#"
function f0(): t0 {
    new.local.managed.zeroed r1, a0
    new.local.managed.slice.uninit r2:r3, a1, r0
    return r1
}
"#,
    );
}
