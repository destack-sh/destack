use super::{assert_format, assert_format_eq};

/// Formats struct aggregate construction canonically.
#[test]
fn test_format_struct_aggregate() {
    assert_format(
        r#"
type Pair {
    left: int32;
    right: int32;
}

function makePair(v0: int32, v1: int32): Pair {
entry(v0: int32, v1: int32):
    v2: Pair = struct Pair (v0, v1)
    return v2
}
"#,
    );
}

/// Formats tuple aggregate construction canonically.
#[test]
fn test_format_tuple_aggregate() {
    assert_format(
        r#"
function makeTuple(v0: int32, v1: float64, v2: boolean): (int32, float64, boolean) {
entry(v0: int32, v1: float64, v2: boolean):
    v3: (int32, float64, boolean) = tuple (int32, float64, boolean) (v0, v1, v2)
    return v3
}
"#,
    );
}

/// Formats tuple aggregate construction with a named alias canonically.
#[test]
fn test_format_tuple_alias_aggregate() {
    assert_format_eq(
        r#"
type Triple = (int32, float64, boolean);

function makeTuple(v0: int32, v1: float64, v2: boolean): Triple {
entry(v0: int32, v1: float64, v2: boolean):
    v3: Triple = tuple Triple (v0, v1, v2)
    return v3
}
"#,
        r#"
type Triple = (int32, float64, boolean);

function makeTuple(v0: int32, v1: float64, v2: boolean): Triple {
entry(v0: int32, v1: float64, v2: boolean):
    v3: Triple = tuple Triple (v0, v1, v2)
    return v3
}
"#,
    );
}

/// Formats array aggregate construction canonically.
#[test]
fn test_format_array_aggregate() {
    assert_format(
        r#"
function makeArray(v0: int32, v1: int32, v2: int32): [int32; 3] {
entry(v0: int32, v1: int32, v2: int32):
    v3: [int32; 3] = array [int32; 3] (v0, v1, v2)
    return v3
}
"#,
    );
}

/// Formats aggregate field operations canonically.
#[test]
fn test_format_aggregate_access() {
    assert_format(
        r#"
function aggregateAccess(v0: (int32, float64), v1: [int32; 10]): int32 {
entry(v0: (int32, float64), v1: [int32; 10]):
    v3: int32 = field.get v0, 0
    v4: int32 = field.get v1, 2
    return v3
}
"#,
    );
}

/// Formats dynamic descriptor projections canonically.
#[test]
fn test_format_dynamic_access() {
    assert_format(
        r#"
type Writer {
    write: fn() => uint32;
}

function dynamicAccess(v0: dynamic<Writer>): typeId {
entry(v0: dynamic<Writer>):
    v1: typeId = dynamic.type v0
    v2: ref<void, raw, mutable> = dynamic.payload v0
    return v1
}
"#,
    );
}
