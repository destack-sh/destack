use crate::diagnostic::Error;
use crate::tests::{
    create_aggregate, create_isolate, run_mir, run_mir_expect, run_mir_ok, run_mir_with_ok,
};
use destack_heap::{RawPointer, STRING_TYPE_ALIAS, Value, ValueTag};
use destack_mir::parse::{ParseOptions, Parser};
use destack_source::FileId;

/// Managed allocation creates one managed allocation and returns a reference.
#[test]
fn test_managed_allocate() {
    let mir = r#"
function @alloc() -> ref<managed readonly i32> {
block0:
    v0: ref<managed readonly i32> = managed.alloc i32
    return v0
}"#;
    let output = run_mir_ok(mir, "alloc", &[]);
    assert!(output.value.is_managed_reference());
    assert_eq!(output.managed_allocation_count, 1);
}

/// Load and store instructions read and write managed allocations.
#[test]
fn test_load_store() {
    let mir = r#"
function @load_store() -> i32 {
block0:
    v0: ref<managed readonly i32> = managed.alloc i32
    v1: i32 = iconst 42i32
    store v0, v1
    v2: i32 = load v0
    return v2
}"#;
    run_mir_expect(mir, "load_store", &[], Value::int32(42));
}

/// Store rejects unsupported address spaces in the VM.
#[test]
fn test_store_unsupported_address_space() {
    // define a shared address space store
    let mir = r#"
function @store_shared(v0: ref<raw addrspace(shared) i32>) -> void {
block0(v0: ref<raw addrspace(shared) i32>):
    v1: i32 = iconst 1i32
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
function @store_stack(v0: ref<raw addrspace(stack) i32>) -> void {
block0(v0: ref<raw addrspace(stack) i32>):
    v1: i32 = iconst 1i32
    store v0, v1
    return
}"#;

    // run with a heap pointer to trigger mismatch
    let pointer = Value::raw_pointer(RawPointer::new(1));
    let err = run_mir(mir, "store_stack", &[pointer]).expect_err("expected failure");

    // confirm the address space mismatch
    assert!(matches!(err.error, Error::InvalidAddressSpace { .. }));
}

/// External VM contexts can allocate and mutate explicit shared-memory regions.
#[test]
fn test_external_context_shared_bytes_roundtrip() {
    let mut isolate = create_isolate(
        r#"
function @noop() -> void {
block0:
    return
}"#,
    );

    let pointer = isolate.with_memory(|isolate, memory| {
        isolate.with_runtime_context(memory, |context| {
            let pointer = context
                .allocate_shared_bytes(&[1, 2, 3])
                .expect("shared allocation should succeed");
            let initial = context
                .shared_bytes(pointer)
                .expect("shared bytes should decode");

            assert_eq!(initial, vec![1, 2, 3]);

            context
                .write_shared_bytes(pointer, &[7, 8, 9, 10])
                .expect("shared bytes should write");

            pointer
        })
    });

    assert_eq!(
        Value::shared_pointer(pointer).tag(),
        ValueTag::SharedPointer
    );
    assert_eq!(
        Value::shared_pointer(pointer).as_shared_pointer(),
        Some(pointer)
    );
    assert_eq!(
        isolate.shared.bytes_to_vec(pointer),
        Some(vec![7, 8, 9, 10])
    );
}

/// Array allocation creates one managed allocation with multiple elements.
#[test]
fn test_managed_allocate_array() {
    let mir = r#"
function @alloc_array() -> ref<managed readonly i32> {
block0:
    v0: i64 = iconst 10i64
    v1: ref<managed readonly i32> = managed.alloc_array i32, v0
    return v1
}"#;
    let output = run_mir_ok(mir, "alloc_array", &[]);
    assert!(output.value.is_managed_reference());
    assert_eq!(output.managed_allocation_count, 1);
}

/// Extract field reads a component from a tuple value.
#[test]
fn test_extract_field() {
    let mir = r#"
function @get_first(v0: (i32, i32)) -> i32 {
block0(v0: (i32, i32)):
    v1: i32 = field.get v0, 0
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
    v2: (i32, i32) = field.set v0, 0, v1
    v3: i32 = field.get v2, 0
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
    v2: i32 = element.get v0, v1
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
    v3: [i32; 3] = element.set v0, v1, v2
    v4: i32 = element.get v3, v1
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
    v0: ref<managed readonly (i32, i32)> = managed.alloc (i32, i32)
    v1: i32 = iconst 42i32
    store v0, v1
    v2: ref<borrowed readonly i32> = field.addr v0, 0
    v3: i32 = load v2
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
    v1: i32 = field.get v0, 5
    return v1
}"#;

    let err = Parser::parse(FileId::new(0), mir, ParseOptions::default())
        .expect_err("expected parse failure");

    assert_eq!(
        err.message,
        "metadata invariant violation: field.get field index 5 out of bounds for tuple with 1 elements"
    );
}

/// Out-of-bounds array access produces an error.
#[test]
fn test_invalid_array_access() {
    let mir = r#"
function @bad_elem(v0: [i32; 3], v1: i64) -> i32 {
block0(v0: [i32; 3], v1: i64):
    v2: i32 = element.get v0, v1
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

/// Exceeding the managed allocation limit produces an allocation error.
#[test]
fn test_allocation_limit() {
    let mir = r#"
function @alloc_many() -> void {
block0:
    v0: i32 = iconst 0i32
    jump block1(v0)
block1(v1: i32):
    v2: ref<managed readonly i32> = managed.alloc i32
    v3: i32 = iconst 1i32
    v4: i32 = iadd v1, v3
    v5: i32 = iconst 2000i32
    v6: bool = icmp_slt v4, v5
    branch v6, block1(v4), block2
block2:
    return
}"#;
    let result = run_mir(mir, "alloc_many", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::AllocationFailed));
}

/// Raw allocation creates one raw allocation and returns a raw pointer.
#[test]
fn test_raw_allocate() {
    let mir = r#"
function @raw_alloc() -> ref<raw readonly i32> {
block0:
    v0: ref<raw readonly i32> = raw.alloc i32
    return v0
}"#;
    let output = run_mir_ok(mir, "raw_alloc", &[]);
    assert!(output.value.as_raw_pointer().is_some());
    assert_eq!(output.raw_allocation_count, 1);
}

/// Raw free deallocates a raw pointer.
#[test]
fn test_raw_free() {
    let mir = r#"
function @raw_alloc_free() -> i32 {
block0:
    v0: ref<raw readonly i32> = raw.alloc i32
    v1: i32 = iconst 42i32
    store v0, v1
    v2: i32 = load v0
    raw.free v0
    return v2
}"#;
    let output = run_mir_ok(mir, "raw_alloc_free", &[]);
    assert_eq!(output.value, Value::int32(42));
    // after free, raw heap should be empty
    assert_eq!(output.raw_allocation_count, 0);
}

/// Raw free on invalid pointer produces an error.
#[test]
fn test_raw_free_invalid() {
    let mir = r#"
function @double_free() -> void {
block0:
    v0: ref<raw readonly i32> = raw.alloc i32
    raw.free v0
    raw.free v0
    return
}"#;
    let result = run_mir(mir, "double_free", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::InvalidManagedReference));
}

/// String header fields expose UTF-16 and UTF-8 lengths.
#[test]
fn test_string_header_lengths() {
    let mir = [
        STRING_TYPE_ALIAS,
        r#"global @literal:string:unicode_he: ref<managed readonly @String> = "h\u{00E9}" ; readonly

function @len_utf16() -> u32 {
block0:
    v0: ref<managed readonly @String> = global.const @literal:string:unicode_he
    v1: u32 = field.get v0, 0
    return v1
}

function @len_bytes() -> u32 {
block0:
    v0: ref<managed readonly @String> = global.const @literal:string:unicode_he
    v1: u32 = field.get v0, 1
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
        r#"global @literal:string:Hi: ref<managed readonly @String> = "Hi" ; readonly
global @literal:string:abc: ref<managed readonly @String> = "abc" ; readonly

function @first_byte() -> u8 {
block0:
    v0: ref<managed readonly @String> = global.const @literal:string:Hi
    v1: ref<raw u8> = field.get v0, 5
    v2: u8 = load v1
    return v2
}

function @memcmp_self() -> i32 {
block0:
    v0: ref<managed readonly @String> = global.const @literal:string:abc
    v1: ref<raw u8> = field.get v0, 5
    v2: u64 = iconst 3u64
    v3: i32 = intrinsic.memcmp(v1, v1, v2)
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
    v0: ref<raw addrspace(stack) readonly i32> = stack.alloc i32
    v1: i32 = iconst 99i32
    store v0, v1
    v2: i32 = load v0
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
    v0: ref<raw addrspace(stack) readonly (i32, i32)> = stack.alloc (i32, i32)
    v1: i32 = iconst 10i32
    v2: i32 = iconst 20i32
    store v0, v1
    v3: ref<borrowed readonly i32> = field.addr v0, 0
    v4: i32 = load v3
    return v4
}"#;
    run_mir_expect(mir, "stack_struct", &[], Value::int32(10));
}

/// Null raw pointer dereference produces an error.
#[test]
fn test_null_pointer_load() {
    let mir = r#"
function @null_load(v0: ref<raw readonly i32>) -> i32 {
block0(v0: ref<raw readonly i32>):
    v1: i32 = load v0
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
function @null_store(v0: ref<raw i32>, v1: i32) -> void {
block0(v0: ref<raw i32>, v1: i32):
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
    v0: ref<raw readonly i32> = raw.alloc i32
    v1: i32 = iconst 42i32
    store v0, v1
    raw.free v0
    v2: i32 = load v0
    return v2
}"#;
    let result = run_mir(mir, "use_after_free", &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err.error, Error::InvalidManagedReference));
}
