use super::assert_format_eq;

/// Format reference loads, stores, lifetime operations, barriers, and drops canonically.
#[test]
fn test_format_reference_operations() {
    assert_format_eq(
        r#"
external function f0

function f1 {
    memory.load r4, r0,8
memory.store r0,r4,8
pin r4: ref<managed, local>
unpin r4: ref<managed, local>
barrier r4,r3,r3: ref<managed, local>
drop r0,f0
free r2: ref<unique, local>
return r4
}
"#,
        r#"
external function f0

function f1 {
    memory.load r4, r0, 8
    memory.store r0, r4, 8
    pin r4: ref<managed, local>
    unpin r4: ref<managed, local>
    barrier r4, r3, r3: ref<managed, local>
    drop r0, f0
    free r2: ref<unique, local>
    return r4
}
"#,
    );
}
