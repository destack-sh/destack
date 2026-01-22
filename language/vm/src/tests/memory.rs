use crate::diagnostic::Error;
use crate::memory::{RawPointer, STRING_TYPE_ALIAS, Value};
use crate::tests::{create_aggregate, run_mir, run_mir_expect, run_mir_ok, run_mir_with_ok};

/// Managed allocation creates a heap cell and returns a reference.
#[test]
fn test_managed_allocate() {
    let mir = r#"
function @alloc() -> ref<managed i32> {
block0:
    v0 = managed.alloc i32 -> ref<managed i32>
    return v0
}"#;
    let output = run_mir_ok(mir, "alloc", &[]);
    assert!(output.value.is_managed_reference());
    assert_eq!(output.heap_cells, 1);
}

/// Load and store instructions read and write heap cells.
#[test]
fn test_load_store() {
    let mir = r#"
function @load_store() -> i32 {
block0:
    v0 = managed.alloc i32 -> ref<managed i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0 -> i32
    return v2
}"#;
    run_mir_expect(mir, "load_store", &[], Value::int32(42));
}

/// Store rejects unsupported address spaces in the VM.
#[test]
fn test_store_unsupported_address_space() {
    // define a shared address space store
    let mir = r#"
function @store_shared(v0: ref<raw addrspace(shared) mut i32>) -> void {
block0(v0: ref<raw addrspace(shared) mut i32>):
    v1 = iconst 1i32
    store v0, v1
    return
}"#;

    // run and capture the error
    let pointer = Value::raw_pointer(RawPointer::new(1));
    let err = run_mir(mir, "store_shared", &[pointer]).expect_err("expected failure");

    // confirm the address space is rejected
    assert!(matches!(err.error, Error::UnsupportedAddressSpace { .. }));
}

/// Store rejects mismatched address space pointers.
#[test]
fn test_store_invalid_address_space() {
    // define a stack address space store
    let mir = r#"
function @store_stack(v0: ref<raw addrspace(stack) mut i32>) -> void {
block0(v0: ref<raw addrspace(stack) mut i32>):
    v1 = iconst 1i32
    store v0, v1
    return
}"#;

    // run with a heap pointer to trigger mismatch
    let pointer = Value::raw_pointer(RawPointer::new(1));
    let err = run_mir(mir, "store_stack", &[pointer]).expect_err("expected failure");

    // confirm the address space mismatch
    assert!(matches!(err.error, Error::InvalidAddressSpace { .. }));
}

/// Array allocation creates a heap cell with multiple slots.
#[test]
fn test_managed_allocate_array() {
    let mir = r#"
function @alloc_array() -> ref<managed i32> {
block0:
    v0 = iconst 10i64
    v1 = managed.alloc_array i32, v0 -> ref<managed i32>
    return v1
}"#;
    let output = run_mir_ok(mir, "alloc_array", &[]);
    assert!(output.value.is_managed_reference());
    assert_eq!(output.heap_cells, 1);
}

/// Extract field reads a component from a tuple value.
#[test]
fn test_extract_field() {
    let mir = r#"
function @get_first(v0: (i32, i32)) -> i32 {
block0(v0: (i32, i32)):
    v1 = field.get v0, 0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "get_first", |interp| {
        let agg = create_aggregate(interp, vec![Value::int32(10), Value::int32(20)]);
        vec![agg]
    });
    assert_eq!(output.value, Value::int32(10));
}

/// Insert field creates a new tuple with one component replaced.
#[test]
fn test_insert_field() {
    // test that field.set modifies field 0 correctly
    let mir = r#"
function @set_and_get(v0: (i32, i32), v1: i32) -> i32 {
block0(v0: (i32, i32), v1: i32):
    v2 = field.set v0, 0, v1
    v3 = field.get v2, 0
    return v3
}"#;
    let output = run_mir_with_ok(mir, "set_and_get", |interp| {
        let agg = create_aggregate(interp, vec![Value::int32(10), Value::int32(20)]);
        vec![agg, Value::int32(99)]
    });
    assert_eq!(output.value, Value::int32(99));
}

/// Extract element reads from an array at a dynamic index.
#[test]
fn test_extract_element() {
    let mir = r#"
function @get_elem(v0: [i32; 3], v1: i64) -> i32 {
block0(v0: [i32; 3], v1: i64):
    v2 = element.get v0, v1
    return v2
}"#;
    // test element 0
    let output = run_mir_with_ok(mir, "get_elem", |interp| {
        let arr = create_aggregate(
            interp,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );
        vec![arr, Value::uint64(0)]
    });
    assert_eq!(output.value, Value::int32(10));

    // test element 1
    let output = run_mir_with_ok(mir, "get_elem", |interp| {
        let arr = create_aggregate(
            interp,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );
        vec![arr, Value::uint64(1)]
    });
    assert_eq!(output.value, Value::int32(20));

    // test element 2
    let output = run_mir_with_ok(mir, "get_elem", |interp| {
        let arr = create_aggregate(
            interp,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );
        vec![arr, Value::uint64(2)]
    });
    assert_eq!(output.value, Value::int32(30));
}

/// Insert element creates a new array with one element replaced.
#[test]
fn test_insert_element() {
    // test that element.set modifies the correct element
    let mir = r#"
function @set_and_get(v0: [i32; 3], v1: i64, v2: i32) -> i32 {
block0(v0: [i32; 3], v1: i64, v2: i32):
    v3 = element.set v0, v1, v2
    v4 = element.get v3, v1
    return v4
}"#;
    let output = run_mir_with_ok(mir, "set_and_get", |interp| {
        let arr = create_aggregate(
            interp,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );
        vec![arr, Value::uint64(1), Value::int32(99)]
    });
    assert_eq!(output.value, Value::int32(99));
}

/// Extract field works on heap-allocated objects.
#[test]
fn test_heap_field_access() {
    let mir = r#"
function @heap_field() -> i32 {
block0:
    v0 = managed.alloc (i32, i32) -> ref<managed (i32, i32)>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = load v2 -> i32
    return v3
}"#;
    run_mir_expect(mir, "heap_field", &[], Value::int32(42));
}

/// Out-of-bounds field access produces an error.
#[test]
fn test_invalid_field_access() {
    let mir = r#"
function @bad_field(v0: (i32,)) -> i32 {
block0(v0: (i32,)):
    v1 = field.get v0, 5
    return v1
}"#;
    let result = crate::tests::run_mir_with(mir, "bad_field", |interp| {
        let agg = create_aggregate(interp, vec![Value::int32(10)]);
        vec![agg]
    });
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
    v2 = element.get v0, v1
    return v2
}"#;
    let result = crate::tests::run_mir_with(mir, "bad_elem", |interp| {
        let arr = create_aggregate(
            interp,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );
        vec![arr, Value::uint64(100)]
    });
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
    v2 = managed.alloc i32 -> ref<managed i32>
    v3 = iconst 1i32
    v4 = iadd v1, v3
    v5 = iconst 2000i32
    v6 = icmp_slt v4, v5
    branch v6, block1(v4), block2
block2:
    return
}"#;
    let result = run_mir(mir, "alloc_many", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::AllocationFailed));
}

/// Raw allocation creates a heap cell and returns a raw pointer.
#[test]
fn test_raw_allocate() {
    let mir = r#"
function @raw_alloc() -> ref<raw i32> {
block0:
    v0 = raw.alloc i32 -> ref<raw i32>
    return v0
}"#;
    let output = run_mir_ok(mir, "raw_alloc", &[]);
    assert!(output.value.as_raw_pointer().is_some());
    assert_eq!(output.raw_heap_cells, 1);
}

/// Raw free deallocates a raw pointer.
#[test]
fn test_raw_free() {
    let mir = r#"
function @raw_alloc_free() -> i32 {
block0:
    v0 = raw.alloc i32 -> ref<raw i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0 -> i32
    raw.free v0
    return v2
}"#;
    let output = run_mir_ok(mir, "raw_alloc_free", &[]);
    assert_eq!(output.value, Value::int32(42));
    // after free, raw heap should be empty
    assert_eq!(output.raw_heap_cells, 0);
}

/// Raw free on invalid pointer produces an error.
#[test]
fn test_raw_free_invalid() {
    let mir = r#"
function @double_free() -> void {
block0:
    v0 = raw.alloc i32 -> ref<raw i32>
    raw.free v0
    raw.free v0
    return
}"#;
    let result = run_mir(mir, "double_free", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::InvalidHeapHandle));
}

/// String header fields expose UTF-16 and UTF-8 lengths.
#[test]
fn test_string_header_lengths() {
    let mir = [
        STRING_TYPE_ALIAS,
        r#"global @literal:string:unicode_he: ref<managed @String> = "h\u{00E9}" ; const

function @len_utf16() -> u32 {
block0:
    v0 = global.const @literal:string:unicode_he
    v1 = field.get v0, 0
    return v1
}

function @len_bytes() -> u32 {
block0:
    v0 = global.const @literal:string:unicode_he
    v1 = field.get v0, 1
    return v1
}"#,
    ]
    .concat();
    run_mir_expect(&mir, "len_utf16", &[], Value::uint32(2));
    run_mir_expect(&mir, "len_bytes", &[], Value::uint32(3));
}

/// String data pointers expose raw bytes for mem intrinsics.
#[test]
fn test_string_payload_bytes() {
    let mir = [
        STRING_TYPE_ALIAS,
        r#"global @literal:string:Hi: ref<managed @String> = "Hi" ; const
global @literal:string:abc: ref<managed @String> = "abc" ; const

function @first_byte() -> u8 {
block0:
    v0 = global.const @literal:string:Hi
    v1 = field.get v0, 5
    v2 = load v1 -> u8
    return v2
}

function @memcmp_self() -> i32 {
block0:
    v0 = global.const @literal:string:abc
    v1 = field.get v0, 5
    v2 = iconst 3u64
    v3 = intrinsic.memcmp(v1, v1, v2)
    return v3
}"#,
    ]
    .concat();
    run_mir_expect(&mir, "first_byte", &[], Value::uint(72, 8));
    run_mir_expect(&mir, "memcmp_self", &[], Value::int32(0));
}

/// Stack allocation creates frame-local storage.
#[test]
fn test_stack_allocate() {
    let mir = r#"
function @stack_alloc() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 99i32
    store v0, v1
    v2 = load v0 -> i32
    return v2
}"#;
    run_mir_expect(mir, "stack_alloc", &[], Value::int32(99));
}

/// Stack allocation with field access.
#[test]
fn test_stack_allocate_struct() {
    let mir = r#"
function @stack_struct() -> i32 {
block0:
    v0 = stack.alloc (i32, i32) -> ref<raw addrspace(stack) (i32, i32)>
    v1 = iconst 10i32
    v2 = iconst 20i32
    store v0, v1
    v3 = field.addr v0, 0 -> ref<borrowed i32>
    v4 = load v3 -> i32
    return v4
}"#;
    run_mir_expect(mir, "stack_struct", &[], Value::int32(10));
}

/// Null raw pointer dereference produces an error.
#[test]
fn test_null_pointer_load() {
    let mir = r#"
function @null_load(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = load v0 -> i32
    return v1
}"#;
    let result = run_mir(mir, "null_load", &[Value::raw_pointer(RawPointer::NULL)]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::NullPointerDereference));
}

/// Null raw pointer store produces an error.
#[test]
fn test_null_pointer_store() {
    let mir = r#"
function @null_store(v0: ref<raw mut i32>, v1: i32) -> void {
block0(v0: ref<raw mut i32>, v1: i32):
    store v0, v1
    return
}"#;
    let result = run_mir(
        mir,
        "null_store",
        &[Value::raw_pointer(RawPointer::NULL), Value::int32(42)],
    );
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::NullPointerDereference));
}

/// Use-after-free on raw pointer produces an error.
#[test]
fn test_use_after_free() {
    let mir = r#"
function @use_after_free() -> i32 {
block0:
    v0 = raw.alloc i32 -> ref<raw i32>
    v1 = iconst 42i32
    store v0, v1
    raw.free v0
    v2 = load v0 -> i32
    return v2
}"#;
    let result = run_mir(mir, "use_after_free", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::InvalidHeapHandle));
}
