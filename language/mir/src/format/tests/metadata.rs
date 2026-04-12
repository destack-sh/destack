use super::assert_format;

/// Formats kernel metadata canonically.
#[test]
fn test_format_roundtrip_function_metadata() {
    assert_format(
        r#"
@executionModel(kernel)
@workgroupSize(8, 1, 1)
function kernel(): void {
entry0:
    return
}
"#,
    );
}

/// Formats graphics stage metadata canonically.
#[test]
fn test_format_roundtrip_function_stage_metadata() {
    assert_format(
        r#"
@executionModel(graphics)
@executionStage(vertex)
function vertexMain(): void {
entry0:
    return
}
"#,
    );
}

/// Formats item attributes on declarations and fields canonically.
#[test]
fn test_format_roundtrip_item_attributes() {
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
global Count: int32, readonly = 1int32

function usePoint(value0: Point): int32 {
entry0(value0: Point):
    value1: int32 = field.get value0, 0
    return value1
}
"#,
    );
}
