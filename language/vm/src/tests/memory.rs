use crate::IsolateOptions;
use crate::diagnostic::Error;
use crate::tests::{
    TestIsolate, assert_runtime_error_matches, create_isolate, run_mir, run_mir_expect, run_mir_ok,
    run_mir_with, run_mir_with_ok, stamp_well_known_string_type_for_tests,
};
use destack_heap::{
    Heap, HeapLayoutOptions, HeapLimits, LayoutId, MemoryContext, RawPointer, ReferenceMap,
    STRING_TYPE_ALIAS, SharedSpace, StringLayout, Value, ValueTag,
};
use destack_mir::parse::{ParseOptions, Parser};
use destack_mir::{ManagedReferenceLayout, ManagedReferenceRepresentation, Storage, TypeAlias};
use destack_source::FileId;

/// Build one test isolate with explicit MIR storage metadata.
fn create_isolate_with_storage(mir_text: &str, storage: Storage) -> TestIsolate {
    let (mut tree, strings) = Parser::parse(
        FileId::new(0),
        mir_text,
        ParseOptions {
            pointer_bytes: storage.native_pointer_bytes,
        },
    )
    .validate()
    .expect("failed to parse MIR");

    // keep the helper honest: parse must produce the requested layout directly
    assert_eq!(tree.metadata.layout.storage, storage);

    // keep raw MIR tests explicit about the well known String contract
    stamp_well_known_string_type_for_tests(&mut tree, &strings);

    let mut isolate = crate::Isolate::build_with_options(tree, strings, IsolateOptions::test())
        .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
    let mut heap = Heap::with_limits_and_layout(
        HeapLimits::default(),
        HeapLayoutOptions {
            managed_reference_bytes: storage.managed_reference_layout.bytes,
            ..Default::default()
        },
    );
    let mut shared = SharedSpace::new();
    let mut memory = MemoryContext::new(&mut heap, &mut shared);

    // initialize isolate globals against the authoritative heap
    isolate
        .initialize(&mut memory)
        .unwrap_or_else(|error| panic!("failed to initialize isolate globals: {error}"));

    TestIsolate {
        isolate,
        heap,
        shared,
    }
}

/// Managed allocation creates one managed allocation and returns a reference.
#[test]
fn test_managed_allocate() {
    let mir = r#"
function alloc(): ref<int32, managed, readonly> {
b0:
    v0: ref<int32, managed, readonly> = managed.alloc int32
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
function loadStore(): int32 {
b0:
    v0: ref<int32, managed, readonly> = managed.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
    run_mir_expect(mir, "loadStore", &[], Value::int32(42));
}

/// Store rejects unsupported address spaces in the VM.
#[test]
fn test_store_unsupported_address_space() {
    // define a shared address space store
    let mir = r#"
function storeShared(v0: ref<int32, raw, addressSpace(shared)>): void {
b0(v0: ref<int32, raw, addressSpace(shared)>):
    v1: int32 = 1int32
    store v0, v1
    return
}"#;

    // run and capture the error
    let pointer = Value::raw_pointer(RawPointer::new(1));
    let result = run_mir(mir, "storeShared", &[pointer]);

    // confirm the address space is rejected
    assert_runtime_error_matches!(result, Error::UnsupportedAddressSpace { .. });
}

/// Store rejects mismatched address space pointers.
#[test]
fn test_store_invalid_address_space() {
    // define a stack address space store
    let mir = r#"
function storeStack(v0: ref<int32, raw, addressSpace(stack)>): void {
b0(v0: ref<int32, raw, addressSpace(stack)>):
    v1: int32 = 1int32
    store v0, v1
    return
}"#;

    // run with a heap pointer to trigger mismatch
    let pointer = Value::raw_pointer(RawPointer::new(1));
    let result = run_mir(mir, "storeStack", &[pointer]);

    // confirm the address space mismatch
    assert_runtime_error_matches!(result, Error::InvalidAddressSpace { .. });
}

/// External VM contexts can allocate and mutate explicit shared-memory regions.
#[test]
fn test_external_context_shared_bytes_roundtrip() {
    let mut isolate = create_isolate(
        r#"
function noop(): void {
b0:
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
function allocArray(): ref<int32, managed, readonly> {
b0:
    v0: int64 = 10int64
    v1: ref<int32, managed, readonly> = managed.allocArray int32, v0
    return v1
}"#;
    let output = run_mir_ok(mir, "allocArray", &[]);
    assert!(output.value.is_managed_reference());
    assert_eq!(output.managed_allocation_count, 1);
}

/// Extract field reads a component from a tuple value.
#[test]
fn test_extract_field() {
    let mir = r#"
function getFirst(v0: (int32, int32)): int32 {
b0(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    return v1
}"#;
    let output = run_mir_with_ok(mir, "getFirst", |interp| {
        let ty = interp.parameter_type("getFirst", 0);
        let agg = interp.materialize_value_for_type(ty, vec![Value::int32(10), Value::int32(20)]);
        vec![agg]
    });
    assert_eq!(output.value, Value::int32(10));
}

/// Insert field creates a new tuple with one component replaced.
#[test]
fn test_insert_field() {
    // test that field.set modifies field 0 correctly
    let mir = r#"
function setAndGet(v0: (int32, int32), v1: int32): int32 {
b0(v0: (int32, int32), v1: int32):
    v2: (int32, int32) = field.set v0, 0, v1
    v3: int32 = field.get v2, 0
    return v3
}"#;
    let output = run_mir_with_ok(mir, "setAndGet", |interp| {
        let ty = interp.parameter_type("setAndGet", 0);
        let agg = interp.materialize_value_for_type(ty, vec![Value::int32(10), Value::int32(20)]);
        vec![agg, Value::int32(99)]
    });
    assert_eq!(output.value, Value::int32(99));
}

/// Field set returns one fresh tuple value instead of mutating the original.
#[test]
fn test_insert_field_does_not_alias_original() {
    let mir = r#"
function setWithoutAlias(v0: (int32, int32), v1: int32): int32 {
b0(v0: (int32, int32), v1: int32):
    v2: (int32, int32) = field.set v0, 0, v1
    v3: int32 = field.get v0, 0
    v4: int32 = field.get v2, 0
    v5: int32 = int.add v3, v4
    return v5
}"#;
    let output = run_mir_with_ok(mir, "setWithoutAlias", |interp| {
        let ty = interp.parameter_type("setWithoutAlias", 0);
        let tuple = interp.materialize_value_for_type(ty, vec![Value::int32(10), Value::int32(20)]);

        vec![tuple, Value::int32(99)]
    });

    assert_eq!(output.value, Value::int32(109));
}

/// Extract element reads from an array at a dynamic index.
#[test]
fn test_extract_element() {
    let mir = r#"
function getElem(v0: int32[3], v1: int64): int32 {
b0(v0: int32[3], v1: int64):
    v2: int32 = element.get v0, v1
    return v2
}"#;
    // test element 0
    let output = run_mir_with_ok(mir, "getElem", |interp| {
        let ty = interp.parameter_type("getElem", 0);
        let arr = interp.materialize_value_for_type(
            ty,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );
        vec![arr, Value::uint64(0)]
    });
    assert_eq!(output.value, Value::int32(10));

    // test element 1
    let output = run_mir_with_ok(mir, "getElem", |interp| {
        let ty = interp.parameter_type("getElem", 0);
        let arr = interp.materialize_value_for_type(
            ty,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );
        vec![arr, Value::uint64(1)]
    });
    assert_eq!(output.value, Value::int32(20));

    // test element 2
    let output = run_mir_with_ok(mir, "getElem", |interp| {
        let ty = interp.parameter_type("getElem", 0);
        let arr = interp.materialize_value_for_type(
            ty,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );
        vec![arr, Value::uint64(2)]
    });
    assert_eq!(output.value, Value::int32(30));
}

/// Dynamic element.get on one locally constructed array stays correct.
#[test]
fn test_extract_element_from_local_array() {
    let mir = r#"
function getLocalElem(v0: int64): int32 {
b0(v0: int64):
    v1: int32 = 10int32
    v2: int32 = 20int32
    v3: int32 = 30int32
    v4: int32[3] = array int32[3] (v1, v2, v3)
    v5: int32 = element.get v4, v0
    return v5
}"#;

    run_mir_expect(mir, "getLocalElem", &[Value::uint64(0)], Value::int32(10));
    run_mir_expect(mir, "getLocalElem", &[Value::uint64(1)], Value::int32(20));
    run_mir_expect(mir, "getLocalElem", &[Value::uint64(2)], Value::int32(30));
}

/// Element set returns one fresh array value instead of mutating the original.
#[test]
fn test_insert_element_does_not_alias_original() {
    let mir = r#"
function setWithoutAlias(v0: int32[3], v1: int64, v2: int32): int32 {
b0(v0: int32[3], v1: int64, v2: int32):
    v3: int32[3] = element.set v0, v1, v2
    v4: int32 = element.get v0, v1
    v5: int32 = element.get v3, v1
    v6: int32 = int.add v4, v5
    return v6
}"#;
    let output = run_mir_with_ok(mir, "setWithoutAlias", |interp| {
        let ty = interp.parameter_type("setWithoutAlias", 0);
        let array = interp.materialize_value_for_type(
            ty,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );

        vec![array, Value::uint64(1), Value::int32(99)]
    });

    assert_eq!(output.value, Value::int32(119));
}

/// Insert element creates a new array with one element replaced.
#[test]
fn test_insert_element() {
    // test that element.set modifies the correct element
    let mir = r#"
function setAndGet(v0: int32[3], v1: int64, v2: int32): int32 {
b0(v0: int32[3], v1: int64, v2: int32):
    v3: int32[3] = element.set v0, v1, v2
    v4: int32 = element.get v3, v1
    return v4
}"#;
    let output = run_mir_with_ok(mir, "setAndGet", |interp| {
        let ty = interp.parameter_type("setAndGet", 0);
        let arr = interp.materialize_value_for_type(
            ty,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );
        vec![arr, Value::uint64(1), Value::int32(99)]
    });
    assert_eq!(output.value, Value::int32(99));
}

/// Dynamic element.set on one locally constructed array stays correct.
#[test]
fn test_insert_element_on_local_array() {
    let mir = r#"
function setLocalAndGet(v0: int64, v1: int32): int32 {
b0(v0: int64, v1: int32):
    v2: int32 = 10int32
    v3: int32 = 20int32
    v4: int32 = 30int32
    v5: int32[3] = array int32[3] (v2, v3, v4)
    v6: int32[3] = element.set v5, v0, v1
    v7: int32 = element.get v5, v0
    v8: int32 = element.get v6, v0
    v9: int32 = int.add v7, v8
    return v9
}"#;

    run_mir_expect(
        mir,
        "setLocalAndGet",
        &[Value::uint64(1), Value::int32(99)],
        Value::int32(119),
    );
}

/// Extract field works on heap-allocated objects.
#[test]
fn test_heap_field_access() {
    let mir = r#"
function heapField(): int32 {
b0:
    v0: ref<(int32, int32), managed, readonly> = managed.alloc (int32, int32)
    v1: int32 = 42int32
    v2: ref<int32, managed, readonly> = field.address v0, 0
    store v2, v1
    v3: int32 = load v2
    return v3
}"#;
    run_mir_expect(mir, "heapField", &[], Value::int32(42));
}

/// Single field aggregates expose the stored field value.
#[test]
fn test_single_field_aggregate_roundtrips_field() {
    let mir = r#"
type Box {
    value: int32;
}

function readBox(v0: int32): int32 {
b0(v0: int32):
    v1: Box = struct Box (v0)
    v2: int32 = field.get v1, 0
    return v2
}"#;

    run_mir_expect(mir, "readBox", &[Value::int32(9)], Value::int32(9));
}

/// Managed nominal allocations use layout bytes rather than packed value storage.
#[test]
fn test_managed_nominal_allocation_uses_layout_storage() {
    let mir = r#"
type Box {
    value: int32;
}

function allocBox(): ref<Box, managed, readonly> {
b0:
    v0: ref<Box, managed, readonly> = managed.alloc Box
    return v0
}"#;

    let mut isolate = create_isolate(mir);
    let output = isolate
        .run_function_by_name("allocBox", &[])
        .expect("execution failed");
    let handle = output
        .value
        .as_managed_reference()
        .expect("managed allocation should return a managed reference");

    // layout backed objects should keep byte storage and ref offsets
    assert_eq!(
        isolate.heap.reference_map(handle).cloned(),
        Some(ReferenceMap::empty())
    );
}

/// Managed nominal stores roundtrip full aggregate payloads.
#[test]
fn test_managed_nominal_store_roundtrips_payload() {
    let mir = r#"
type Box {
    value: int32;
}

function makeBox(v0: int32): ref<Box, managed, readonly> {
b0(v0: int32):
    v1: Box = struct Box (v0)
    v2: ref<Box, managed, readonly> = managed.alloc Box
    store v2, v1
    return v2
}"#;

    let mut isolate = create_isolate(mir);
    let output = isolate
        .run_function_by_name("makeBox", &[Value::int32(9)])
        .expect("execution failed");
    let handle = output
        .value
        .as_managed_reference()
        .expect("managed allocation should return a managed reference");
    let bytes = isolate
        .heap
        .managed_bytes_to_vec(handle)
        .expect("managed object bytes should be readable");

    // the payload should be stored as raw layout bytes, not a boxed aggregate handle
    assert_eq!(u32::from_le_bytes(bytes[0..4].try_into().unwrap()), 9);
}

/// Managed nominal layout follows the canonical pointer-shaped object layout.
#[test]
fn test_managed_nominal_allocation_uses_modulus_alignment() {
    let mir = r#"
type Packed {
    first: uint8;
    inner: ref<int32, managed, readonly>;
    third: uint8;
}
function allocPacked(): ref<Packed, managed, readonly> {
b0:
    v0: ref<Packed, managed, readonly> = managed.alloc Packed
    return v0
}"#;
    let storage = Storage {
        native_pointer_bytes: 8,
        managed_reference_layout: ManagedReferenceLayout {
            bytes: 8,
            alignment: 8,
            representation: ManagedReferenceRepresentation::NativePointer,
        },
    };

    let mut isolate = create_isolate_with_storage(mir, storage);
    let output = isolate
        .run_function_by_name("allocPacked", &[])
        .expect("execution failed");
    let handle = output
        .value
        .as_managed_reference()
        .expect("managed allocation should return a managed reference");

    // pointer-shaped managed refs should follow the canonical aggregate layout
    assert_eq!(isolate.heap.managed_byte_len(handle), Some(24));
    assert_eq!(
        isolate.heap.reference_map(handle).cloned(),
        Some(ReferenceMap::ReferenceOffsets { offsets: vec![8] })
    );
}

/// Managed reference arrays use pointer-shaped element stride.
#[test]
fn test_managed_alloc_array_uses_pointer_stride() {
    let mir = r#"
function allocArray(): ref<ref<int32, managed, readonly>, managed, readonly> {
b0:
    v0: int64 = 2int64
    v1: ref<ref<int32, managed, readonly>, managed, readonly> = managed.allocArray ref<int32, managed, readonly>, v0
    return v1
}"#;
    let storage = Storage {
        native_pointer_bytes: 8,
        managed_reference_layout: ManagedReferenceLayout {
            bytes: 8,
            alignment: 8,
            representation: ManagedReferenceRepresentation::NativePointer,
        },
    };

    let mut isolate = create_isolate_with_storage(mir, storage);
    let output = isolate
        .run_function_by_name("allocArray", &[])
        .expect("execution failed");
    let handle = output
        .value
        .as_managed_reference()
        .expect("managed allocation should return a managed reference");

    // pointer-shaped managed refs should use pointer-sized repeated elements
    assert_eq!(isolate.heap.managed_byte_len(handle), Some(16));
    assert_eq!(
        isolate.heap.reference_map(handle).cloned(),
        Some(ReferenceMap::RepeatedReferenceOffsets {
            count: 2,
            element_size: 8,
            offsets: vec![0],
        })
    );
}

/// Managed field loads match whole-object load plus field extraction.
#[test]
fn test_managed_field_load_matches_whole_object_load() {
    let mir = r#"
type Holder {
    value: ref<int32, managed, readonly>;
}

function comparePaths(): int32 {
b0:
    v0: ref<int32, managed, readonly> = managed.alloc int32
    v1: int32 = 41int32
    store v0, v1
    v2: Holder = struct Holder (v0)
    v3: ref<Holder, managed, readonly> = managed.alloc Holder
    store v3, v2
    v4: ref<ref<int32, managed, readonly>, managed, readonly> = field.address v3, 0
    v5: ref<int32, managed, readonly> = load v4
    v6: Holder = load v3
    v7: ref<int32, managed, readonly> = field.get v6, 0
    v8: int32 = load v5
    v9: int32 = load v7
    v10: int32 = int.add v8, v9
    return v10
}"#;
    let mut isolate = create_isolate(mir);
    let output = isolate
        .run_function_by_name("comparePaths", &[])
        .expect("execution failed");

    // both paths should recover the same referenced payload
    assert_eq!(output.value, Value::int32(82));
}

/// Out-of-bounds field access produces an error.
#[test]
fn test_invalid_field_access() {
    let mir = r#"
function badField(v0: (int32,)): int32 {
b0(v0: (int32,)):
    v1: int32 = field.get v0, 5
    return v1
}"#;

    let err = Parser::parse(FileId::new(0), mir, ParseOptions::default())
        .validate()
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
function badElem(v0: int32[3], v1: int64): int32 {
b0(v0: int32[3], v1: int64):
    v2: int32 = element.get v0, v1
    return v2
}"#;
    let result = run_mir_with(mir, "badElem", |interp| {
        let ty = interp.parameter_type("badElem", 0);
        let arr = interp.materialize_value_for_type(
            ty,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );
        vec![arr, Value::uint64(100)]
    });
    assert_runtime_error_matches!(result, Error::InvalidArrayAccess { .. });
}

/// Exceeding the managed allocation limit produces an allocation error.
#[test]
fn test_allocation_limit() {
    let mir = r#"
function allocMany(): void {
b0:
    v0: int32 = 0int32
    jump b1(v0)
b1(v1: int32):
    v2: ref<int32, managed, readonly> = managed.alloc int32
    v3: int32 = 1int32
    v4: int32 = int.add v1, v3
    v5: int32 = 2000int32
    v6: boolean = int.lt.s v4, v5
    branch v6, b1(v4), b2
b2:
    return
}"#;
    let result = run_mir(mir, "allocMany", &[]);
    assert_runtime_error_matches!(result, Error::AllocationFailed);
}

/// Raw allocation creates one raw allocation and returns a raw pointer.
#[test]
fn test_raw_allocate() {
    let mir = r#"
function rawAlloc(): ref<int32, raw, readonly> {
b0:
    v0: ref<int32, raw, readonly> = raw.alloc int32
    return v0
}"#;
    let output = run_mir_ok(mir, "rawAlloc", &[]);
    assert!(output.value.as_raw_pointer().is_some());
    assert_eq!(output.raw_allocation_count, 1);
}

/// Raw free deallocates a raw pointer.
#[test]
fn test_raw_free() {
    let mir = r#"
function rawAllocFree(): int32 {
b0:
    v0: ref<int32, raw, readonly> = raw.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    raw.free v0
    return v2
}"#;
    let output = run_mir_ok(mir, "rawAllocFree", &[]);
    assert_eq!(output.value, Value::int32(42));
    // after free, raw heap should be empty
    assert_eq!(output.raw_allocation_count, 0);
}

/// Raw free on invalid pointer produces an error.
#[test]
fn test_raw_free_invalid() {
    let mir = r#"
function doubleFree(): void {
b0:
    v0: ref<int32, raw, readonly> = raw.alloc int32
    raw.free v0
    raw.free v0
    return
}"#;
    let result = run_mir(mir, "doubleFree", &[]);
    assert_runtime_error_matches!(result, Error::InvalidManagedReference);
}

/// String header fields expose UTF-16 and UTF-8 lengths.
#[test]
fn test_string_header_lengths() {
    let mir = [
        STRING_TYPE_ALIAS,
        r#"
global stringLiteralUnicodeHe: ref<String, managed, readonly>, readonly = "h\u{00E9}"

function lenUtf16(): uint32 {
b0:
    v0: ref<String, managed, readonly> = global.const stringLiteralUnicodeHe
    v1: uint32 = field.get v0, 0
    return v1
}

function lenBytes(): uint32 {
b0:
    v0: ref<String, managed, readonly> = global.const stringLiteralUnicodeHe
    v1: uint32 = field.get v0, 1
    return v1
}"#,
    ]
    .concat();
    run_mir_expect(&mir, "lenUtf16", &[], Value::uint32(2));
    run_mir_expect(&mir, "lenBytes", &[], Value::uint32(3));
}

/// String data pointers expose raw bytes for mem intrinsics.
#[test]
fn test_string_payload_bytes() {
    let mir = [
        STRING_TYPE_ALIAS,
        r#"
global stringLiteralHi: ref<String, managed, readonly>, readonly = "Hi"
global stringLiteralAbc: ref<String, managed, readonly>, readonly = "abc"

function firstByte(): uint8 {
b0:
    v0: ref<String, managed, readonly> = global.const stringLiteralHi
    v1: ref<uint8, raw> = field.get v0, 2
    v2: uint8 = load v1
    return v2
}

function memcmpSelf(): int32 {
b0:
    v0: ref<String, managed, readonly> = global.const stringLiteralAbc
    v1: ref<uint8, raw> = field.get v0, 2
    v2: uint64 = 3uint64
    v3: int32 = intrinsic.memcmp(v1, v1, v2)
    return v3
}"#,
    ]
    .concat();
    run_mir_expect(&mir, "firstByte", &[], Value::uint(72, 8));
    run_mir_expect(&mir, "memcmpSelf", &[], Value::int32(0));
}

/// String header pointers follow the active native pointer width.
#[test]
fn test_string_payload_bytes_under_pointer32_layout() {
    let mir = [
        STRING_TYPE_ALIAS,
        r#"
global stringLiteralHi: ref<String, managed, readonly>, readonly = "Hi"

function firstByte(): uint8 {
b0:
    v0: ref<String, managed, readonly> = global.const stringLiteralHi
    v1: ref<uint8, raw> = field.get v0, 2
    v2: uint8 = load v1
    return v2
}"#,
    ]
    .concat();
    let storage = Storage {
        native_pointer_bytes: 4,
        managed_reference_layout: ManagedReferenceLayout {
            bytes: 4,
            alignment: 4,
            representation: ManagedReferenceRepresentation::NativePointer,
        },
    };

    let mut isolate = create_isolate_with_storage(&mir, storage);
    let output = isolate
        .run_function_by_name("firstByte", &[])
        .expect("execution failed");

    // pointer-sized raw pointers should still roundtrip through the string header
    assert_eq!(output.value, Value::uint(72, 8));
}

/// Reject non-string managed allocations when decoding runtime strings.
#[test]
fn test_string_value_rejects_non_string_managed_reference() {
    let mir = [
        STRING_TYPE_ALIAS,
        r#"
function noop(): void {
b0:
    return
}"#,
    ]
    .concat();
    let mut isolate = create_isolate(&mir);
    let byte_len = StringLayout::new(8).byte_len();
    let bytes = vec![0u8; byte_len];
    let handle = isolate
        .heap
        .allocate_managed_bytes(&bytes, ReferenceMap::empty(), Some(LayoutId::new(99)))
        .expect("managed allocation should succeed");
    let value = Value::managed_reference(handle);
    let error = isolate
        .isolate
        .string_value(&isolate.heap, value)
        .expect_err("non-string managed allocation should not decode as string");

    assert_eq!(
        error,
        Error::TypeMismatch {
            expected: "string".to_string(),
            actual: "managed reference".to_string(),
        }
    );
}

/// Decode one canonical String object without requiring literal interning.
#[test]
fn test_string_value_accepts_canonical_string_layout_without_interner_entry() {
    let mir = [
        STRING_TYPE_ALIAS,
        "\nfunction noop(): void {\nb0:\n    return\n}",
    ]
    .concat();
    let (mut tree, strings) = Parser::parse(FileId::new(0), &mir, ParseOptions::default())
        .validate()
        .expect("failed to parse MIR");

    // keep raw MIR tests explicit about the well known String contract
    stamp_well_known_string_type_for_tests(&mut tree, &strings);

    let layout_id = tree
        .string_layout_id()
        .expect("missing canonical well known string layout id");
    let string_type_id = tree
        .string_type()
        .map(|type_id| type_id.id)
        .expect("missing canonical well known string type id");
    let layout = StringLayout::new(tree.metadata.layout.storage.native_pointer_bytes);
    let isolate = crate::Isolate::build_with_options(tree, strings, IsolateOptions::test())
        .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
    let mut heap = Heap::new();
    let payload = heap
        .allocate_raw_bytes(b"Hi")
        .expect("raw string payload should allocate");
    let mut header = vec![0u8; layout.byte_len()];

    let wrote_length_utf16 = layout.write_field(
        &mut header,
        StringLayout::LENGTH_UTF16_FIELD as u32,
        Value::uint32(2),
    );
    let wrote_length_bytes = layout.write_field(
        &mut header,
        StringLayout::LENGTH_BYTES_FIELD as u32,
        Value::uint32(2),
    );
    let wrote_data = layout.write_field(
        &mut header,
        StringLayout::DATA_FIELD as u32,
        Value::raw_pointer(payload),
    );

    assert!(wrote_length_utf16 && wrote_length_bytes && wrote_data);

    let handle = heap
        .allocate_managed_bytes(&header, ReferenceMap::empty(), Some(layout_id))
        .expect("canonical string allocation should succeed");
    assert!(heap.set_managed_type_id(handle, string_type_id));
    let value = Value::managed_reference(handle);

    // canonical String layout objects should decode even outside the literal interner
    assert_eq!(
        isolate
            .string_value(&heap, value)
            .expect("canonical string object should decode"),
        "Hi"
    );
}

/// Reject a user type that only matches the string header structurally.
#[test]
fn test_string_value_rejects_structurally_matching_non_builtin_layout() {
    let mir = [
        r#"
type Other {
    lengthUtf16: uint32;
    lengthBytes: uint32;
    data: ref<uint8, raw>;
}"#,
        STRING_TYPE_ALIAS,
        r#"
function noop(): void {
b0:
    return
}"#,
    ]
    .concat();
    let (mut tree, strings) = Parser::parse(FileId::new(0), &mir, ParseOptions::default())
        .validate()
        .expect("failed to parse MIR");
    stamp_well_known_string_type_for_tests(&mut tree, &strings);
    let other_layout_id = tree
        .iter_nodes::<TypeAlias>()
        .find_map(|(_, alias)| {
            if strings.get(alias.name) != "Other" {
                return None;
            }

            let ty = alias
                .ty
                .ty()
                .expect("type alias should be concrete after validation");

            tree.type_layout_id(ty)
        })
        .expect("missing Other layout id");
    let isolate = crate::Isolate::build_with_options(tree, strings, IsolateOptions::test())
        .unwrap_or_else(|error| panic!("failed to initialize isolate: {error}"));
    let mut heap = Heap::new();
    let byte_len = StringLayout::new(8).byte_len();
    let bytes = vec![0u8; byte_len];
    let handle = heap
        .allocate_managed_bytes(&bytes, ReferenceMap::empty(), Some(other_layout_id))
        .expect("managed allocation should succeed");
    let value = Value::managed_reference(handle);
    let error = isolate
        .string_value(&heap, value)
        .expect_err("structurally matching non-builtin layout should not decode as string");

    assert_eq!(
        error,
        Error::TypeMismatch {
            expected: "string".to_string(),
            actual: "managed reference".to_string(),
        }
    );
}

/// Stack allocation creates frame-local storage.
#[test]
fn test_stack_allocate() {
    let mir = r#"
function stackAlloc(): int32 {
b0:
    v0: ref<int32, raw, readonly, addressSpace(stack)> = stack.alloc int32
    v1: int32 = 99int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
    run_mir_expect(mir, "stackAlloc", &[], Value::int32(99));
}

/// Stack allocation with field access.
#[test]
fn test_stack_allocate_struct() {
    let mir = r#"
function stackStruct(): int32 {
b0:
    v0: ref<(int32, int32), raw, readonly, addressSpace(stack)> = stack.alloc (int32, int32)
    v1: int32 = 10int32
    v2: ref<int32, borrowed, readonly> = field.address v0, 0
    store v2, v1
    v3: int32 = load v2
    return v3
}"#;
    run_mir_expect(mir, "stackStruct", &[], Value::int32(10));
}

/// Stack allocation stores managed references under a 32 bit pointer layout.
#[test]
fn test_stack_allocate_pointer32_managed_reference_field() {
    let mir = r#"
type Packed {
    first: uint8;
    inner: ref<int32, managed, readonly>;
    third: uint8;
}
function stackPacked(): int32 {
b0:
    v0: ref<int32, managed, readonly> = managed.alloc int32
    v1: int32 = 77int32
    store v0, v1
    v2: ref<Packed, raw, readonly, addressSpace(stack)> = stack.alloc Packed
    v3: ref<ref<int32, managed, readonly>, borrowed, readonly> = field.address v2, 1
    store v3, v0
    v4: ref<int32, managed, readonly> = load v3
    v5: int32 = load v4
    return v5
}"#;
    let storage = Storage {
        native_pointer_bytes: 4,
        managed_reference_layout: ManagedReferenceLayout {
            bytes: 4,
            alignment: 4,
            representation: ManagedReferenceRepresentation::NativePointer,
        },
    };

    let mut isolate = create_isolate_with_storage(mir, storage);
    let output = isolate
        .run_function_by_name("stackPacked", &[])
        .expect("execution failed");

    // stack storage should preserve managed reference payloads on 32 bit targets
    assert_eq!(output.value, Value::int32(77));
}

/// Null raw pointer dereference produces an error.
#[test]
fn test_null_pointer_load() {
    let mir = r#"
function nullLoad(v0: ref<int32, raw, readonly>): int32 {
b0(v0: ref<int32, raw, readonly>):
    v1: int32 = load v0
    return v1
}"#;
    let result = run_mir(mir, "nullLoad", &[Value::raw_pointer(RawPointer::NULL)]);
    assert_runtime_error_matches!(result, Error::NullPointerDereference);
}

/// Null raw pointer store produces an error.
#[test]
fn test_null_pointer_store() {
    let mir = r#"
function nullStore(v0: ref<int32, raw>, v1: int32): void {
b0(v0: ref<int32, raw>, v1: int32):
    store v0, v1
    return
}"#;
    let result = run_mir(
        mir,
        "nullStore",
        &[Value::raw_pointer(RawPointer::NULL), Value::int32(42)],
    );
    assert_runtime_error_matches!(result, Error::NullPointerDereference);
}

/// Use-after-free on raw pointer produces an error.
#[test]
fn test_use_after_free() {
    let mir = r#"
function useAfterFree(): int32 {
b0:
    v0: ref<int32, raw, readonly> = raw.alloc int32
    v1: int32 = 42int32
    store v0, v1
    raw.free v0
    v2: int32 = load v0
    return v2
}"#;
    let result = run_mir(mir, "useAfterFree", &[]);
    assert_runtime_error_matches!(result, Error::InvalidManagedReference);
}
