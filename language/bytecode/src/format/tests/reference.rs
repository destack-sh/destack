use super::assert_format_eq;

/// Format reference loads, stores, lifetime operations, barriers, and drops canonically.
#[test]
fn test_format_reference_operations() {
    assert_format_eq(
        r#"
type Point

export function references(r0:pointer,r1:ref<managed,space(local)>,r2:ref<unique,space(local)>,r3:uint64):ref<managed,space(local)>{
r4:ref<managed,space(local)>=load r0,Point
store r0,r4,Point
pin r4
unpin r4
barrier r4,r3,r3
drop r0:Point
free r2
return r4
}
"#,
        r#"
type Point

export function references(
    r0: pointer,
    r1: ref<managed, space(local)>,
    r2: ref<unique, space(local)>,
    r3: uint64,
): ref<managed, space(local)> {
    r4: ref<managed, space(local)> = load r0, Point
    store r0, r4, Point
    pin r4
    unpin r4
    barrier r4, r3, r3
    drop r0: Point
    free r2
    return r4
}
"#,
    );
}
