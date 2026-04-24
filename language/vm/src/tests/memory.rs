use crate::diagnostic::Error;
use crate::tests::{
    assert_materialized_plain, assert_runtime_error, assert_runtime_error_matches, create_isolate,
    create_isolate_with_storage, run_mir, run_mir_expect, run_mir_ok, run_mir_with,
    run_mir_with_ok,
};
use crate::{SharedHeap, Value, ValueTag};
use destack_engine::MaterializedValue;
use destack_heap::{
    AccountingRegion, Allocator, HeapError, HeapReference, Payload, RawPointer,
    SharedHeapReference, SharedRawBudget, SharedRawLimits,
};
use destack_mir::parse::{ParseOptions, Parser};
use destack_mir::{LayoutTable, ReferenceMap, Storage};
use destack_source::FileId;

/// Return the aggregate payload bytes for one materialized value.
fn aggregate_bytes(value: &MaterializedValue) -> &[u8] {
    let MaterializedValue::Aggregate { bytes, .. } = value else {
        panic!("expected aggregate value, got {value:?}");
    };

    bytes
}

/// Decode one native-width heap reference from materialized bytes.
fn decode_heap_reference(bytes: &[u8], offset: usize) -> HeapReference {
    let mut raw = [0u8; 8];
    raw[..HeapReference::BYTE_LEN]
        .copy_from_slice(&bytes[offset..offset + HeapReference::BYTE_LEN]);

    HeapReference::from_bits(u64::from_le_bytes(raw) as usize)
}

/// Decode one native-width usize value from materialized bytes.
fn decode_usize_value(bytes: &[u8], offset: usize) -> Value {
    let mut raw = [0u8; 8];
    let byte_len = std::mem::size_of::<usize>();
    raw[..byte_len].copy_from_slice(&bytes[offset..offset + byte_len]);

    Value::uint(u64::from_le_bytes(raw), usize::BITS as u8)
}

/// Heap allocation creates one heap allocation and returns a reference.
#[test]
fn test_new_allocates_heap_reference() {
    let mir = r#"
function alloc(): ref<int32, managed, readonly> {
b0:
    v0: ref<int32, managed, readonly> = new int32
    return v0
}"#;
    let output = run_mir_ok(mir, "alloc", &[]);
    assert!(matches!(
        output.value,
        destack_engine::MaterializedValue::HeapReference(_)
    ));
    assert_eq!(output.heap_allocation_count, 1);
}

/// Load and store instructions read and write heap allocations.
#[test]
fn test_load_store() {
    let mir = r#"
function loadStore(): int32 {
b0:
    v0: ref<int32, managed, readonly> = new int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
    run_mir_expect(mir, "loadStore", &[], Value::int32(42));
}

/// Store rejects mismatched shared address space pointers.
#[test]
fn test_store_shared_invalid_address_space() {
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

    // confirm the address space mismatch
    assert_runtime_error_matches!(result, Error::InvalidAddressSpace { .. });
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

/// External VM contexts can allocate and mutate explicit shared raw-space allocations.
#[test]
fn test_external_context_shared_bytes_roundtrip() {
    let mut isolate = create_isolate(
        r#"
function noop(): void {
b0:
    return
}"#,
    );

    let pointer = isolate.with_heaps(|isolate, heap, shared| {
        isolate.with_runtime_context(heap, shared, Default::default(), |context| {
            let pointer = context
                .allocate_shared_bytes(&[1, 2, 3])
                .expect("shared allocation should succeed");
            let initial = context
                .read_shared_bytes(pointer)
                .expect("shared bytes should decode");

            assert_eq!(initial, vec![1, 2, 3]);

            let pointer = context
                .write_shared_bytes(pointer, &[7, 8, 9, 10])
                .expect("shared bytes should write");

            pointer
        })
    });

    assert_eq!(
        Value::shared_raw_pointer(pointer).tag(),
        ValueTag::SharedRawPointer
    );
    assert_eq!(
        Value::shared_raw_pointer(pointer).as_shared_raw_pointer(),
        Some(pointer)
    );
    assert_eq!(
        isolate.shared.read_raw_bytes(pointer),
        Ok(vec![7, 8, 9, 10])
    );
}

/// Encode shared heap references as first-class runtime values.
#[test]
fn test_shared_heap_reference_value_roundtrip() {
    let reference = SharedHeapReference::new(7);

    assert_eq!(
        Value::shared_heap_reference(reference).tag(),
        ValueTag::SharedHeapReference
    );
    assert_eq!(
        Value::shared_heap_reference(reference).as_shared_heap_reference(),
        Some(reference)
    );
}

/// Share unchanged shared allocations across image roundtrips and detach only touched allocations.
#[test]
fn test_roundtrip_shared_memory_image() {
    let options = destack_heap::HeapOptions::shared();
    let allocator = std::sync::Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_arena_bytes)
            .expect("valid explicit allocator options should build"),
    );
    let shared = SharedHeap::with_allocator_limits_layouts_and_options(
        allocator.clone(),
        std::sync::Arc::new(LayoutTable::new()),
        destack_heap::SharedHeapLimits::default(),
        options.clone(),
    )
    .expect("shared heap should build");

    // capture two allocations so only one has to detach later
    let first = shared
        .allocate_raw(6, Payload::Bytes(&[1, 2, 3, 4, 5, 6]))
        .expect("shared allocation should succeed");
    let second = shared
        .allocate_raw(6, Payload::Bytes(&[7, 8, 9, 10, 11, 12]))
        .expect("shared allocation should succeed");
    let image = shared.image();
    let restored = SharedHeap::from_image_with_allocator_limits_and_options(
        allocator,
        &image,
        destack_heap::SharedHeapLimits::default(),
        options,
    )
    .expect("shared image restore should succeed");
    let restored_image = restored.image();

    // untouched pages should still share after restore
    assert_eq!(
        image.raw.entry(0).unwrap().pages,
        restored_image.raw.entry(0).unwrap().pages
    );
    assert_eq!(
        image.raw.entry(1).unwrap().pages,
        restored_image.raw.entry(1).unwrap().pages
    );

    // mutating one allocation should detach only that allocation
    let first = restored
        .replace_raw_bytes(first, &[9, 2, 3, 4, 5, 6])
        .expect("shared replace should succeed");
    let mutated_image = restored.image();

    assert_ne!(
        image.raw.entry(0).unwrap().pages,
        mutated_image.raw.entry(0).unwrap().pages
    );
    assert_eq!(
        image.raw.entry(1).unwrap().pages,
        mutated_image.raw.entry(1).unwrap().pages
    );
    assert_eq!(restored.read_raw_bytes(first), Ok(vec![9, 2, 3, 4, 5, 6]));
    assert_eq!(
        restored.read_raw_bytes(second),
        Ok(vec![7, 8, 9, 10, 11, 12])
    );
}

/// Shared raw-space budgeting counts committed page bytes.
#[test]
fn test_shared_raw_budget_tracks_committed_usage() {
    let options = destack_heap::HeapOptions::shared();
    let allocator = std::sync::Arc::new(
        Allocator::try_new(options.page_bytes, options.allocator_arena_bytes)
            .expect("valid explicit allocator options should build"),
    );
    let shared = SharedHeap::with_allocator_limits_layouts_and_options(
        allocator,
        std::sync::Arc::new(LayoutTable::new()),
        destack_heap::SharedHeapLimits::default(),
        options,
    )
    .expect("shared heap should build");
    let limits = SharedRawLimits { max_bytes: Some(8) };

    // commit one shared allocation first
    shared
        .allocate_raw(4, Payload::Bytes(&[1, 2, 3, 4]))
        .expect("nested shared allocation should succeed");

    // the next allocation must see the committed usage immediately
    let mapped_delta = shared.raw_alloc_mapped_byte_delta(5);
    let mapped_bytes = u64::try_from(mapped_delta).expect("allocation should map more bytes");
    let used_bytes = shared.raw_active_bytes() + mapped_bytes;
    let budget = SharedRawBudget::new(limits, shared.raw_active_bytes());
    let error = budget
        .check_mapped_byte_delta(mapped_delta)
        .expect_err("outer shared allocation should honor nested usage");

    assert_eq!(
        error,
        HeapError::LimitExceeded {
            region: AccountingRegion::SharedRaw,
            used_bytes,
            max_bytes: 8,
        }
    );
}

/// Array allocation creates one slice value over one heap allocation.
#[test]
fn test_new_slice_allocates_slice_value() {
    let mir = r#"
function allocArray(): slice<int32> {
b0:
    v0: int64 = 10int64
    v1: slice<int32> = new.slice int32, v0
    return v1
}"#;
    let mut isolate = create_isolate(mir);
    let output = isolate
        .run_function_by_name("allocArray", &[])
        .expect("execution failed");
    let bytes = aggregate_bytes(&output.value);
    let data = Value::heap_reference(decode_heap_reference(bytes, 0));
    let len = decode_usize_value(bytes, HeapReference::BYTE_LEN);

    assert!(data.is_heap_reference());
    assert_eq!(len.as_uint(), Some(10));
    assert_eq!(len.as_uint_with_width(), Some((10, usize::BITS as u8)));

    assert_eq!(output.heap_allocation_count, 1);
}

/// Extract field reads a field from a tuple value.
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
    assert_eq!(assert_materialized_plain(&output.value), Value::int32(10));
}

/// Insert field creates a new tuple with one field replaced.
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
    assert_eq!(assert_materialized_plain(&output.value), Value::int32(99));
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

    assert_eq!(assert_materialized_plain(&output.value), Value::int32(109));
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
    assert_eq!(assert_materialized_plain(&output.value), Value::int32(10));

    // test element 1
    let output = run_mir_with_ok(mir, "getElem", |interp| {
        let ty = interp.parameter_type("getElem", 0);
        let arr = interp.materialize_value_for_type(
            ty,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );
        vec![arr, Value::uint64(1)]
    });
    assert_eq!(assert_materialized_plain(&output.value), Value::int32(20));

    // test element 2
    let output = run_mir_with_ok(mir, "getElem", |interp| {
        let ty = interp.parameter_type("getElem", 0);
        let arr = interp.materialize_value_for_type(
            ty,
            vec![Value::int32(10), Value::int32(20), Value::int32(30)],
        );
        vec![arr, Value::uint64(2)]
    });
    assert_eq!(assert_materialized_plain(&output.value), Value::int32(30));
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

    assert_eq!(assert_materialized_plain(&output.value), Value::int32(119));
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
    assert_eq!(assert_materialized_plain(&output.value), Value::int32(99));
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
    v0: ref<(int32, int32), managed, readonly> = new (int32, int32)
    v1: int32 = 42int32
    v2: ref<int32, managed, readonly> = field.address v0, 0
    store v2, v1
    v3: int32 = load v2
    return v3
}"#;
    run_mir_expect(mir, "heapField", &[], Value::int32(42));
}

/// Borrowed field addresses preserve heap storage.
#[test]
fn test_heap_borrowed_field_access() {
    let mir = r#"
function heapBorrowedField(): int32 {
b0:
    v0: ref<(int32, int32), managed, readonly> = new (int32, int32)
    v1: int32 = 42int32
    v2: ref<int32, borrowed, readonly> = field.address v0, 0
    store v2, v1
    v3: int32 = load v2
    return v3
}"#;
    run_mir_expect(mir, "heapBorrowedField", &[], Value::int32(42));
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
    v0: ref<Box, managed, readonly> = new Box
    return v0
}"#;

    let mut isolate = create_isolate(mir);
    let output = isolate
        .run_function_by_name("allocBox", &[])
        .expect("execution failed");
    let value = assert_materialized_plain(&output.value);
    let reference = value
        .as_heap_reference()
        .expect("heap allocation should return a heap reference");

    // layout backed objects should keep byte storage and ref offsets
    assert_eq!(isolate.heap.scan(reference), Ok(ReferenceMap::empty()));
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
    v2: ref<Box, managed, readonly> = new Box
    store v2, v1
    return v2
}"#;

    let mut isolate = create_isolate(mir);
    let output = isolate
        .run_function_by_name("makeBox", &[Value::int32(9)])
        .expect("execution failed");
    let value = assert_materialized_plain(&output.value);
    let reference = value
        .as_heap_reference()
        .expect("heap allocation should return a heap reference");
    let bytes = isolate
        .heap
        .read_heap_bytes(reference)
        .expect("managed object bytes should be readable");

    // the payload should be stored as raw layout bytes, not a boxed aggregate reference
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
    v0: ref<Packed, managed, readonly> = new Packed
    return v0
}"#;
    let storage = Storage {
        native_pointer_bytes: 8,
    };

    let mut isolate = create_isolate_with_storage(mir, storage);
    let output = isolate
        .run_function_by_name("allocPacked", &[])
        .expect("execution failed");
    let value = assert_materialized_plain(&output.value);
    let reference = value
        .as_heap_reference()
        .expect("heap allocation should return a heap reference");

    // pointer-shaped heap references should follow the canonical aggregate layout
    assert_eq!(isolate.heap.heap_byte_len(reference), Ok(24));
    assert_eq!(
        isolate.heap.scan(reference),
        Ok(ReferenceMap::Reference {
            local_offsets: vec![8].into_boxed_slice(),
            shared_offsets: Vec::new().into_boxed_slice(),
        })
    );
}

/// Slices of heap references use pointer-shaped element stride.
#[test]
fn test_new_slice_uses_pointer_stride_for_heap_references() {
    let mir = r#"
function allocArray(): slice<ref<int32, managed, readonly>> {
b0:
    v0: int64 = 2int64
    v1: slice<ref<int32, managed, readonly>> = new.slice ref<int32, managed, readonly>, v0
    return v1
}"#;
    let storage = Storage {
        native_pointer_bytes: 8,
    };

    let mut isolate = create_isolate_with_storage(mir, storage);
    let output = isolate
        .run_function_by_name("allocArray", &[])
        .expect("execution failed");
    let bytes = aggregate_bytes(&output.value);
    let reference = decode_heap_reference(bytes, 0);

    // pointer-shaped heap refs should use pointer-sized repeated elements
    assert_eq!(isolate.heap.heap_byte_len(reference), Ok(16));
    assert_eq!(
        isolate.heap.scan(reference),
        Ok(ReferenceMap::RepeatedReference {
            count: 2,
            stride: 8,
            local_offsets: vec![0].into_boxed_slice(),
            shared_offsets: Vec::new().into_boxed_slice(),
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
    v0: ref<int32, managed, readonly> = new int32
    v1: int32 = 41int32
    store v0, v1
    v2: Holder = struct Holder (v0)
    v3: ref<Holder, managed, readonly> = new Holder
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
    assert_eq!(assert_materialized_plain(&output.value), Value::int32(82));
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
    assert_runtime_error(
        result,
        Error::InvalidArrayAccess {
            index: 100,
            length: 3,
        },
    );
}

/// Exceeding the heap allocation limit produces an allocation error.
#[test]
fn test_allocation_limit() {
    let mir = r#"
function allocMany(): void {
b0:
    v0: int32 = 0int32
    jump b1(v0)
b1(v1: int32):
    v2: ref<int32, managed, readonly> = new int32
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
    assert!(
        assert_materialized_plain(&output.value)
            .as_raw_pointer()
            .is_some()
    );
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
    assert_eq!(assert_materialized_plain(&output.value), Value::int32(42));
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
    assert_runtime_error_matches!(result, Error::InvalidRawPointer);
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
    v2: ref<int32, borrowed, readonly, addressSpace(stack)> = field.address v0, 0
    store v2, v1
    v3: int32 = load v2
    return v3
}"#;
    run_mir_expect(mir, "stackStruct", &[], Value::int32(10));
}

/// Stack allocation rejects heap-reference storage narrower than the host heap.
#[test]
fn test_stack_allocate_pointer32_heap_reference_field() {
    let mir = r#"
type Packed {
    first: uint8;
    inner: ref<int32, managed, readonly>;
    third: uint8;
}
function stackPacked(): int32 {
b0:
    v0: ref<int32, managed, readonly> = new int32
    v1: int32 = 77int32
    store v0, v1
    v2: ref<Packed, raw, readonly, addressSpace(stack)> = stack.alloc Packed
    v3: ref<ref<int32, managed, readonly>, borrowed, readonly, addressSpace(stack)> = field.address v2, 1
    store v3, v0
    v4: ref<int32, managed, readonly> = load v3
    v5: int32 = load v4
    return v5
}"#;
    let storage = Storage {
        native_pointer_bytes: 4,
    };
    let error = match std::panic::catch_unwind(|| create_isolate_with_storage(mir, storage)) {
        Ok(_) => panic!("narrow heap reference storage should be rejected loudly"),
        Err(error) => error,
    };
    let message = if let Some(message) = error.downcast_ref::<String>() {
        message.as_str()
    } else if let Some(message) = error.downcast_ref::<&str>() {
        message
    } else {
        panic!("unexpected panic payload");
    };

    assert_eq!(
        message,
        "failed to initialize isolate: EM040: incompatible pointer width: module 4 bytes, host 8"
    );
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

/// Use-after-free on raw pointer surfaces the underlying heap error.
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
    assert_runtime_error_matches!(result, Error::InvalidRawPointer);
}
