use super::assert_format;

/// Formats function attributes canonically.
#[test]
fn test_format_function_metadata() {
    assert_format(
        r#"
@cold
@inline
function kernel(): void {
entry:
    return
}
"#,
    );
}

/// Formats staged function attributes canonically.
#[test]
fn test_format_function_stage_metadata() {
    assert_format(
        r#"
@profile("interactive")
@priority(1)
function vertexMain(): void {
entry:
    return
}
"#,
    );
}

/// Formats item attributes on declarations and fields canonically.
#[test]
fn test_format_item_attributes() {
    assert_format(
        r#"
@packed
type Point {
    @offset(0)
    x: int32;
    @offset(4)
    y: int32;
}

@section(".rodata")
readonly global Count: int32 = 1

function usePoint(v0: Point): int32 {
entry(v0: Point):
    v1: int32 = field.get v0, 0
    return v1
}
"#,
    );
}
