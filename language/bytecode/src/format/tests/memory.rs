use super::assert_format_eq;

/// Format memory operations and pointer calculations canonically.
#[test]
fn test_format_memory() {
    assert_format_eq(
        r#"
function f0 {
    pointer.frame r6, r4
store.int32 r6,r3
load.int32 r7, r6
copy.bytes r1->r0,r2
prefetch.read r1
return r7
}
"#,
        r#"
function f0 {
    pointer.frame r6, r4
    store.int32 r6, r3
    load.int32 r7, r6
    copy.bytes r1 -> r0, r2
    prefetch.read r1
    return r7
}
"#,
    );
}
