use super::assert_format_eq;

/// Format reference loads, stores, lifetime operations, barriers, and drops canonically.
#[test]
fn test_format_reference_operations() {
    assert_format_eq(
        r#"
function f0

function f1 {
    load r4, r0,8
store r0,r4,8
pin.local.managed r4
unpin.local.managed r4
barrier.local.managed r4,r3,r3
drop r0,f0
free.local.unique r2
return r4
}
"#,
        r#"
function f0

function f1 {
    load r4, r0, 8
    store r0, r4, 8
    pin.local.managed r4
    unpin.local.managed r4
    barrier.local.managed r4, r3, r3
    drop r0, f0
    free.local.unique r2
    return r4
}
"#,
    );
}
