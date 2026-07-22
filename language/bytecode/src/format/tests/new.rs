use super::assert_format_eq;

/// Format value and slice `new` operations canonically.
#[test]
fn test_format_new() {
    assert_format_eq(
        r#"
type Point

export function allocate(r0:uint64):ref<managed,space(local)>{
r1:ref<managed,space(local)>=new.local.managed.zeroed Point
r2:uninit<slice<Point,managed,space(local)>>=new.local.managed.slice.uninit Point,r0
r4:slice<Point,managed,space(local)>=new.complete r2
return r1
}
"#,
        r#"
type Point

export function allocate(r0: uint64): ref<managed, space(local)> {
    r1: ref<managed, space(local)> = new.local.managed.zeroed Point
    r2: uninit<slice<Point, managed, space(local)>> = new.local.managed.slice.uninit Point, r0
    r4: slice<Point, managed, space(local)> = new.complete r2
    return r1
}
"#,
    );
}
