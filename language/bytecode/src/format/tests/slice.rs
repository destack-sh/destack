use super::assert_format_eq;

/// Format slice views and lengths as two-word values.
#[test]
fn test_format_slice_operations() {
    assert_format_eq(
        r#"
type Point

export function sliceRange(r0:slice<Point,managed,space(local)>,r2:uint64):(slice<Point,managed,space(local)>,uint64){
r3:slice<Point,managed,space(local)>=slice.view r0,r2,r2
r5:uint64=slice.length r3
return r3,r5
}
"#,
        r#"
type Point

export function sliceRange(r0: slice<Point, managed, space(local)>, r2: uint64): (
    slice<Point, managed, space(local)>,
    uint64
) {
    r3: slice<Point, managed, space(local)> = slice.view r0, r2, r2
    r5: uint64 = slice.length r3
    return r3, r5
}
"#,
    );
}
