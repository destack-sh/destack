use super::{assert_format, assert_format_to};

/// Formats struct aggregate construction canonically.
#[test]
fn test_format_roundtrip_struct_aggregate() {
    assert_format(
        r#"
type Pair {
    left: int32;
    right: int32;
}

function makePair(value0: int32, value1: int32): Pair {
entry0(value0: int32, value1: int32):
    value2: Pair = struct Pair (value0, value1)
    return value2
}
"#,
    );
}

/// Formats tuple aggregate construction canonically.
#[test]
fn test_format_roundtrip_tuple_aggregate() {
    assert_format(
        r#"
function makeTuple(value0: int32, value1: float64, value2: boolean): (int32, float64, boolean) {
entry0(value0: int32, value1: float64, value2: boolean):
    value3: (int32, float64, boolean) = tuple (int32, float64, boolean) (value0, value1, value2)
    return value3
}
"#,
    );
}

/// Formats tuple aggregate construction with a named alias canonically.
#[test]
fn test_format_roundtrip_tuple_alias_aggregate() {
    assert_format_to(
        r#"
type Triple = (int32, float64, boolean)

function makeTuple(value0: int32, value1: float64, value2: boolean): Triple {
entry0(value0: int32, value1: float64, value2: boolean):
    value3: Triple = tuple Triple (value0, value1, value2)
    return value3
}
"#,
        r#"
type Triple = (int32, float64, boolean);

function makeTuple(value0: int32, value1: float64, value2: boolean): Triple {
entry0(value0: int32, value1: float64, value2: boolean):
    value3: Triple = tuple Triple (value0, value1, value2)
    return value3
}
"#,
    );
}

/// Formats array aggregate construction canonically.
#[test]
fn test_format_roundtrip_array_aggregate() {
    assert_format(
        r#"
function makeArray(value0: int32, value1: int32, value2: int32): int32[3] {
entry0(value0: int32, value1: int32, value2: int32):
    value3: int32[3] = array int32[3] (value0, value1, value2)
    return value3
}
"#,
    );
}

/// Formats aggregate field and element operations canonically.
#[test]
fn test_format_roundtrip_aggregate_access() {
    assert_format(
        r#"
function aggregateAccess(value0: (int32, float64), value1: int32[10], value2: int64): int32 {
entry0(value0: (int32, float64), value1: int32[10], value2: int64):
    value3: int32 = field.get value0, 0
    value4: int32 = element.get value1, value2
    return value3
}
"#,
    );
}
