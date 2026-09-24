use super::assert_format_eq;

/// Format reference loads, stores, barriers, and drops canonically.
#[test]
fn test_format_reference_operations() {
    assert_format_eq(
        r#"
external function f0

function f1 {
    memory.load r4, r0,8
memory.store r0,r4,8
barrier r4,r3,r3
drop r0,f0
release r2
free r2
return r4
}
"#,
        r#"
external function f0

function f1 {
    memory.load r4, r0, 8
    memory.store r0, r4, 8
    barrier r4, r3, r3
    drop r0, f0
    release r2
    free r2
    return r4
}
"#,
    );
}
