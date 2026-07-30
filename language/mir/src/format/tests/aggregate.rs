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
    v2: Pair = aggregate (v0, v1)
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
    v3: (int32, float64, boolean) = aggregate (v0, v1, v2)
    return v3
}
"#,
    );
}

/// Formats tuple aggregate construction with a named type canonically.
#[test]
fn test_format_named_tuple_aggregate() {
    assert_format_eq(
        r#"
type Triple = (int32, float64, boolean);

function makeTuple(v0: int32, v1: float64, v2: boolean): Triple {
entry(v0: int32, v1: float64, v2: boolean):
    v3: Triple = aggregate (v0, v1, v2)
    return v3
}
"#,
        r#"
type Triple = (int32, float64, boolean);

function makeTuple(v0: int32, v1: float64, v2: boolean): Triple {
entry(v0: int32, v1: float64, v2: boolean):
    v3: Triple = aggregate (v0, v1, v2)
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
    v3: [int32; 3] = aggregate (v0, v1, v2)
    return v3
}
"#,
    );
}

/// Formats vector aggregate construction canonically.
#[test]
fn test_format_vector_aggregate() {
    assert_format(
        r#"
function makeVector(v0: int32, v1: int32, v2: int32, v3: int32): vector<int32, 4> {
entry(v0: int32, v1: int32, v2: int32, v3: int32):
    v4: vector<int32, 4> = aggregate (v0, v1, v2, v3)
    return v4
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
    v2: int32 = field.get v0, 0
    v3: int32 = element.get v1, 2
    return v2
}
"#,
    );
}

/// Formats dynamic value construction and projections canonically.
#[test]
fn test_format_dynamic_access() {
    assert_format(
        r#"
type Writer {
    write: fn() => uint32;
}

type FileWriter { }

function dynamicAccess(v0: ref<FileWriter, managed, readonly>): typeId {
entry(v0: ref<FileWriter, managed, readonly>):
    v1: dynamic<Writer, managed, mutable> = dynamic.bind v0, FileWriter
    v2: typeId = dynamic.type v1
    v3: ref<void, managed, readonly> = dynamic.payload v1
    return v2
}
"#,
    );
}
