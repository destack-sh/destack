use crate::diagnostic::Error;
use crate::memory::Value;
use crate::tests::{expect_evaluate_mir, run_mir, run_mir_ok};

/// Managed allocation creates a heap cell and returns a reference.
#[test]
fn test_managed_allocate() {
    let mir = r#"
function @alloc() -> ref<i32> {
block0:
    v0 = managed_allocate i32
    return v0
}
"#;
    let output = run_mir_ok(mir, "alloc", &[]);
    assert!(matches!(output.value, Value::ManagedReference(_)));
    assert_eq!(output.heap_cells, 1);
}

/// Load and store instructions read and write heap cells.
#[test]
fn test_load_store() {
    let mir = r#"
function @load_store() -> i32 {
block0:
    v0 = managed_allocate i32
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0
    return v2
}
"#;
    expect_evaluate_mir(mir, "load_store", &[], Value::int32(42));
}

/// Array allocation creates a heap cell with multiple slots.
#[test]
fn test_managed_allocate_array() {
    let mir = r#"
function @alloc_array() -> ref<i32> {
block0:
    v0 = iconst 10i64
    v1 = managed_allocate_array i32, v0
    return v1
}
"#;
    let output = run_mir_ok(mir, "alloc_array", &[]);
    assert!(matches!(output.value, Value::ManagedReference(_)));
    assert_eq!(output.heap_cells, 1);
}

/// Extract field reads a component from a tuple value.
#[test]
fn test_extract_field() {
    let mir = r#"
function @get_first(v0: (i32, i32)) -> i32 {
block0(v0: (i32, i32)):
    v1 = extract_field v0, 0
    return v1
}
"#;
    let aggregate = Value::Aggregate(vec![Value::int32(10), Value::int32(20)].into_boxed_slice());
    expect_evaluate_mir(mir, "get_first", &[aggregate], Value::int32(10));
}

/// Insert field creates a new tuple with one component replaced.
#[test]
fn test_insert_field() {
    let mir = r#"
function @set_first(v0: (i32, i32), v1: i32) -> (i32, i32) {
block0(v0: (i32, i32), v1: i32):
    v2 = insert_field v0, 0, v1
    return v2
}
"#;
    let aggregate = Value::Aggregate(vec![Value::int32(10), Value::int32(20)].into_boxed_slice());
    let result = run_mir_ok(mir, "set_first", &[aggregate, Value::int32(99)]);
    assert_eq!(
        result.value,
        Value::Aggregate(vec![Value::int32(99), Value::int32(20)].into_boxed_slice())
    );
}

/// Extract element reads from an array at a dynamic index.
#[test]
fn test_extract_element() {
    let mir = r#"
function @get_elem(v0: [i32; 3], v1: i64) -> i32 {
block0(v0: [i32; 3], v1: i64):
    v2 = extract_element v0, v1
    return v2
}
"#;
    let array = Value::Aggregate(vec![Value::int32(10), Value::int32(20), Value::int32(30)].into_boxed_slice());
    expect_evaluate_mir(mir, "get_elem", &[array.clone(), Value::uint64(0)], Value::int32(10));
    expect_evaluate_mir(mir, "get_elem", &[array.clone(), Value::uint64(1)], Value::int32(20));
    expect_evaluate_mir(mir, "get_elem", &[array, Value::uint64(2)], Value::int32(30));
}

/// Insert element creates a new array with one element replaced.
#[test]
fn test_insert_element() {
    let mir = r#"
function @set_elem(v0: [i32; 3], v1: i64, v2: i32) -> [i32; 3] {
block0(v0: [i32; 3], v1: i64, v2: i32):
    v3 = insert_element v0, v1, v2
    return v3
}
"#;
    let array = Value::Aggregate(vec![Value::int32(10), Value::int32(20), Value::int32(30)].into_boxed_slice());
    let result = run_mir_ok(mir, "set_elem", &[array, Value::uint64(1), Value::int32(99)]);
    assert_eq!(
        result.value,
        Value::Aggregate(vec![Value::int32(10), Value::int32(99), Value::int32(30)].into_boxed_slice())
    );
}

/// Extract field works on heap-allocated objects.
#[test]
fn test_heap_field_access() {
    let mir = r#"
function @heap_field() -> i32 {
block0:
    v0 = managed_allocate (i32, i32)
    v1 = iconst 42i32
    store v0, v1
    v2 = extract_field v0, 0
    return v2
}
"#;
    expect_evaluate_mir(mir, "heap_field", &[], Value::int32(42));
}

/// Out-of-bounds field access produces an error.
#[test]
fn test_invalid_field_access() {
    let mir = r#"
function @bad_field(v0: (i32,)) -> i32 {
block0(v0: (i32,)):
    v1 = extract_field v0, 5
    return v1
}
"#;
    let aggregate = Value::Aggregate(vec![Value::int32(10)].into_boxed_slice());
    let result = run_mir(mir, "bad_field", &[aggregate]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::InvalidFieldAccess { .. }));
}

/// Out-of-bounds array access produces an error.
#[test]
fn test_invalid_array_access() {
    let mir = r#"
function @bad_elem(v0: [i32; 3], v1: i64) -> i32 {
block0(v0: [i32; 3], v1: i64):
    v2 = extract_element v0, v1
    return v2
}
"#;
    let array = Value::Aggregate(vec![Value::int32(10), Value::int32(20), Value::int32(30)].into_boxed_slice());
    let result = run_mir(mir, "bad_elem", &[array, Value::uint64(100)]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::InvalidArrayAccess { .. }));
}

/// Exceeding the heap cell limit produces an allocation error.
#[test]
fn test_allocation_limit() {
    let mir = r#"
function @alloc_many() -> void {
block0:
    v0 = iconst 0i32
    jump block1(v0)
block1(v1: i32):
    v2 = managed_allocate i32
    v3 = iconst 1i32
    v4 = iadd v1, v3
    v5 = iconst 2000i32
    v6 = icmp_slt v4, v5
    branch v6, block1(v4), block2
block2:
    return
}
"#;
    let result = run_mir(mir, "alloc_many", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::AllocationFailed));
}
