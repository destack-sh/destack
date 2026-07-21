use super::assert_format_eq;

/// Format reference and multiword value types canonically.
#[test]
fn test_format_value_types() {
    assert_format_eq(
        r#"
type Item
export function values(r0:ref<managed,space(local)>,r1:slice<Item,borrowed,space(shared)>,r3:int128):void{
return
}
"#,
        r#"
type Item

export function values(
    r0: ref<managed, space(local)>,
    r1: slice<Item, borrowed, space(shared)>,
    r3: int128,
): void {
    return
}
"#,
    );
}
