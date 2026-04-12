use super::{assert_format, assert_format_to};

/// Formats richer reference and builtin types canonically.
#[test]
fn test_format_roundtrip_reference_and_builtin_types() {
    assert_format(
        r#"
function pointerSized(value0: isize, value1: usize, value2: typeDescriptor, value3: typeId, value4: ref?<int32, managed>, value5: ref<int32, raw, addressSpace(shared)>, value6: ref<int32, raw, addressSpace(7)>, value7: ref<int32, owned, readonly>): ref?<int32, managed> {
entry0(value0: isize, value1: usize, value2: typeDescriptor, value3: typeId, value4: ref?<int32, managed>, value5: ref<int32, raw, addressSpace(shared)>, value6: ref<int32, raw, addressSpace(7)>, value7: ref<int32, owned, readonly>):
    return value4
}
"#,
    );
}

/// Formats aggregate and callable type forms canonically.
#[test]
fn test_format_roundtrip_aggregate_and_callable_types() {
    assert_format(
        r#"
function shapes(value0: (int32, float64, boolean), value1: int32[10], value2: fn(int32, int32) -> int64, value3: closure(int32) -> int32, value4: { x: int32, y: float64 }): { x: int32, y: float64 } {
entry0(value0: (int32, float64, boolean), value1: int32[10], value2: fn(int32, int32) -> int64, value3: closure(int32) -> int32, value4: { x: int32, y: float64 }):
    return value4
}
"#,
    );
}

/// Formats recursive and named type declarations canonically.
#[test]
fn test_format_roundtrip_type_declarations() {
    assert_format_to(
        r#"
type Point {
    x: int32;
    y: float64;
}

type Node {
    value: int64;
    next: ref<Node, managed>;
}

function usePoint(v0: ref<Point, managed>, v1: ref<Node, managed>): ref<Point, managed> {
b0(v0: ref<Point, managed>, v1: ref<Node, managed>):
    return v0
}
"#,
        r#"
type Point {
    x: int32;
    y: float64;
}

type Node {
    value: int64;
    next: ref<Node, managed>;
}

function usePoint(value0: ref<Point, managed>, value1: ref<Node, managed>): ref<Point, managed> {
entry0(value0: ref<Point, managed>, value1: ref<Node, managed>):
    return value0
}
"#,
    );
}
