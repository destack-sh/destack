use crate::Word;
use crate::tests::{
    create_machine, create_machine_with_data_layout, run_mir_expect, run_mir_ok,
    run_mir_with_frame_ok,
};
use destack_engine::Value;
use destack_heap::{HeapReference, SharedHeap, SharedHeapReference};
use destack_mir::{DataLayout, TraceMap};

/// Decode one native-width heap reference from materialized bytes.
fn decode_heap_reference(bytes: &[u8], offset: usize) -> HeapReference {
    let end = offset + HeapReference::BYTE_LEN;

    HeapReference::read_from_bytes(&bytes[offset..end]).expect("heap reference bytes should decode")
}

/// Decode one native-width shared heap reference from materialized bytes.
fn decode_shared_heap_reference(bytes: &[u8], offset: usize) -> SharedHeapReference {
    let end = offset + SharedHeapReference::BYTE_LEN;

    SharedHeapReference::read_from_bytes(&bytes[offset..end])
        .expect("shared heap reference bytes should decode")
}

/// Decode one native-width usize value from materialized bytes.
fn decode_usize(bytes: &[u8], offset: usize) -> usize {
    let mut raw = [0u8; 8];
    let byte_len = std::mem::size_of::<usize>();
    raw[..byte_len].copy_from_slice(&bytes[offset..offset + byte_len]);

    u64::from_le_bytes(raw) as usize
}

/// Read managed heap bytes for representation assertions.
fn read_heap_bytes(
    heap: &destack_heap::Heap,
    reference: HeapReference,
    byte_len: usize,
) -> Vec<u8> {
    let mut bytes = vec![0u8; byte_len];
    let address = heap.heap_base_address() + reference.offset();
    unsafe {
        std::ptr::copy_nonoverlapping(address as *const u8, bytes.as_mut_ptr(), byte_len);
    }

    bytes
}

/// Read shared heap bytes for representation assertions.
fn read_shared_heap_bytes(
    heap: &SharedHeap,
    reference: SharedHeapReference,
    byte_len: usize,
) -> Vec<u8> {
    let mut bytes = vec![0u8; byte_len];
    let address = heap.heap_base_address() + reference.offset();
    unsafe {
        std::ptr::copy_nonoverlapping(address as *const u8, bytes.as_mut_ptr(), byte_len);
    }

    bytes
}

/// Heap allocation creates one heap allocation and returns a reference.
#[test]
fn test_new_allocates_heap_reference() {
    let mir = r#"
function alloc(): ref<int32, managed, readonly> {
b0:
    v0: ref<int32, managed, readonly> = new.zeroed int32
    return v0
}"#;
    let output = run_mir_ok(mir, "alloc", &[]);
    assert!(matches!(output, Value::HeapReference(_)));
}

/// Shared heap allocation creates one mutator-local shared heap allocation.
#[test]
fn test_new_allocates_shared_heap_reference() {
    let mir = r#"
function alloc(): ref<int32, managed, readonly, space(shared)> {
b0:
    v0: ref<int32, managed, readonly, space(shared)> = new.zeroed int32
    return v0
}"#;
    let mut machine = create_machine(mir);
    let output = machine
        .run_function_by_name("alloc", &[])
        .expect("execution failed");
    let Value::SharedHeapReference(reference) = output else {
        panic!("expected shared heap reference, got {output:?}");
    };

    assert!(machine.shared_cache.contains_heap_reference(reference));
    assert!(!machine.shared_heap.is_heap_live(reference));
}

/// Freeing unique heap allocations releases local heap storage immediately.
#[test]
fn test_free_releases_unique_heap_allocation() {
    let mir = r#"
function freeUnique(): int32 {
b0:
    v0: ref<int32, unique, readonly> = new.zeroed int32
    free v0
    v1: int32 = 7int32
    return v1
}"#;
    let mut machine = create_machine(mir);
    let output = machine
        .run_function_by_name("freeUnique", &[])
        .expect("execution failed");

    assert_eq!(output, Value::int32(7));
    assert_eq!(machine.heap.heap_allocation_count(), 0);
}

/// Freeing unique shared heap allocations releases shared heap storage immediately.
#[test]
fn test_free_releases_unique_shared_heap_allocation() {
    let mir = r#"
function freeUnique(): int32 {
b0:
    v0: ref<int32, unique, readonly, space(shared)> = new.zeroed int32
    free v0
    v1: int32 = 7int32
    return v1
}"#;
    let mut machine = create_machine(mir);
    let output = machine
        .run_function_by_name("freeUnique", &[])
        .expect("execution failed");

    assert_eq!(output, Value::int32(7));
    assert_eq!(machine.shared_heap.heap_allocation_count(), 0);
}

/// Load and store instructions read and write heap allocations.
#[test]
fn test_load_store() {
    let mir = r#"
function loadStore(): int32 {
b0:
    v0: ref<int32, managed, readonly> = new.zeroed int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
    run_mir_expect(mir, "loadStore", &[], Value::int32(42));
}

/// Uninitialized heap allocation completes after explicit stores.
#[test]
fn test_new_uninit_completes_heap_reference() {
    let mir = r#"
function loadStore(): int32 {
b0:
    v0: uninit<ref<int32, managed, readonly>> = new.uninit int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, managed, readonly> = new.complete v0
    v3: int32 = load v2
    return v3
}"#;
    run_mir_expect(mir, "loadStore", &[], Value::int32(42));
}

/// Uninitialized frame allocation can be initialized by direct stores.
#[test]
fn test_frame_alloc_uninit_load_store() {
    let mir = r#"
function frameUninit(): int32 {
b0:
    v0: ref<int32, raw, readonly, space(frame)> = frame.alloc.uninit int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
    run_mir_expect(mir, "frameUninit", &[], Value::int32(42));
}

/// Encode shared heap references as first-class runtime values.
#[test]
fn test_shared_heap_reference_value_roundtrip() {
    let reference = SharedHeapReference::new(7);

    assert_eq!(
        Word::shared_heap_reference(reference).as_shared_heap_reference(),
        reference
    );
}
/// Array allocation creates one slice value over one heap allocation.
#[test]
fn test_new_slice_allocates_slice_value() {
    let mir = r#"
function allocArray(): slice<int32, managed> {
b0:
    v0: int64 = 10int64
    v1: slice<int32, managed> = new.slice.zeroed int32, v0
    return v1
}"#;
    let mut machine = create_machine(mir);
    let output = machine
        .run_function_by_name("allocArray", &[])
        .expect("execution failed");
    let Value::HeapReference(slice) = output else {
        panic!("expected heap slice value, got {output:?}");
    };
    let bytes = read_heap_bytes(&machine.heap, slice, 2 * HeapReference::BYTE_LEN);
    let data = Word::heap_reference(decode_heap_reference(&bytes, 0));
    let len = decode_usize(&bytes, HeapReference::BYTE_LEN);

    assert!(!data.as_heap_reference().is_null());
    assert_eq!(len, 10);
}

/// Shared array allocation creates one slice value over shared heap bytes.
#[test]
fn test_new_slice_allocates_shared_slice_value() {
    let mir = r#"
function allocArray(): slice<int32, managed, readonly, space(shared)> {
b0:
    v0: int64 = 10int64
    v1: slice<int32, managed, readonly, space(shared)> = new.slice.zeroed int32, v0
    return v1
}"#;
    let mut machine = create_machine(mir);
    let output = machine
        .run_function_by_name("allocArray", &[])
        .expect("execution failed");
    let Value::SharedHeapReference(slice) = output else {
        panic!("expected shared heap slice value, got {output:?}");
    };
    let bytes = read_shared_heap_bytes(
        &machine.shared_heap,
        slice,
        2 * SharedHeapReference::BYTE_LEN,
    );
    let data = Word::shared_heap_reference(decode_shared_heap_reference(&bytes, 0));
    let len = decode_usize(&bytes, SharedHeapReference::BYTE_LEN);

    assert!(!data.as_shared_heap_reference().is_null());
    assert_eq!(len, 10);
}

/// Slice element addresses can load and store backing elements.
#[test]
fn test_slice_element_address_loads_and_stores() {
    let mir = r#"
function accessSlice(): int32 {
b0:
    v0: int64 = 3int64
    v1: slice<int32, managed> = new.slice.zeroed int32, v0
    v2: int64 = 1int64
    v3: ref<int32, managed> = element.address v1, v2
    v4: int32 = 42int32
    store v3, v4
    v5: int32 = load v3
    return v5
}"#;

    run_mir_expect(mir, "accessSlice", &[], Value::int32(42));
}

/// Field get reads a field from a tuple value.
#[test]
fn test_field_get_reads_tuple_field() {
    let mir = r#"
function getFirst(v0: (int32, int32)): int32 {
b0(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    return v1
}"#;
    let output = run_mir_with_frame_ok(mir, "getFirst", |interp| {
        let ty = interp.parameter_type("getFirst", 0);
        let agg = interp.materialize_value_for_type(ty, vec![Word::int32(10), Word::int32(20)]);
        vec![agg]
    });
    assert_eq!(output, Value::int32(10));
}

/// Field set creates a new tuple with one field replaced.
#[test]
fn test_field_set_replaces_tuple_field() {
    let mir = r#"
function setAndGet(v0: (int32, int32), v1: int32): int32 {
b0(v0: (int32, int32), v1: int32):
    v2: (int32, int32) = field.set v0, 0, v1
    v3: int32 = field.get v2, 0
    return v3
}"#;
    let output = run_mir_with_frame_ok(mir, "setAndGet", |interp| {
        let ty = interp.parameter_type("setAndGet", 0);
        let agg = interp.materialize_value_for_type(ty, vec![Word::int32(10), Word::int32(20)]);
        vec![agg, Word::int32(99)]
    });
    assert_eq!(output, Value::int32(99));
}

/// Field set returns one fresh tuple value instead of mutating the original.
#[test]
fn test_field_set_preserves_source_tuple() {
    let mir = r#"
function setWithoutAlias(v0: (int32, int32), v1: int32): int32 {
b0(v0: (int32, int32), v1: int32):
    v2: (int32, int32) = field.set v0, 0, v1
    v3: int32 = field.get v0, 0
    v4: int32 = field.get v2, 0
    v5: int32 = int.add v3, v4
    return v5
}"#;
    let output = run_mir_with_frame_ok(mir, "setWithoutAlias", |interp| {
        let ty = interp.parameter_type("setWithoutAlias", 0);
        let tuple = interp.materialize_value_for_type(ty, vec![Word::int32(10), Word::int32(20)]);

        vec![tuple, Word::int32(99)]
    });

    assert_eq!(output, Value::int32(109));
}

/// Field get copies nested aggregate values.
#[test]
fn test_field_get_copies_nested_aggregate() {
    let mir = r#"
function getNested(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 20int32
    v2: (int32, int32) = tuple (int32, int32) (v0, v1)
    v3: int32 = 30int32
    v4: ((int32, int32), int32) = tuple ((int32, int32), int32) (v2, v3)
    v5: (int32, int32) = field.get v4, 0
    v6: int32 = field.get v5, 1
    return v6
}"#;
    run_mir_expect(mir, "getNested", &[], Value::int32(20));
}

/// Field set copies nested aggregate values.
#[test]
fn test_field_set_copies_nested_aggregate() {
    let mir = r#"
function setNested(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 20int32
    v2: (int32, int32) = tuple (int32, int32) (v0, v1)
    v3: int32 = 30int32
    v4: ((int32, int32), int32) = tuple ((int32, int32), int32) (v2, v3)
    v5: int32 = 40int32
    v6: int32 = 50int32
    v7: (int32, int32) = tuple (int32, int32) (v5, v6)
    v8: ((int32, int32), int32) = field.set v4, 0, v7
    v9: (int32, int32) = field.get v8, 0
    v10: int32 = field.get v9, 1
    return v10
}"#;
    run_mir_expect(mir, "setNested", &[], Value::int32(50));
}

/// Element get reads from an array at a fixed index.
#[test]
fn test_element_get_reads_array_element() {
    let mir = r#"
function getElem(v0: [int32; 3]): int32 {
b0(v0: [int32; 3]):
    v1: int32 = element.get v0, 1
    return v1
}"#;
    let output = run_mir_with_frame_ok(mir, "getElem", |interp| {
        let ty = interp.parameter_type("getElem", 0);
        let array = interp.materialize_value_for_type(
            ty,
            vec![Word::int32(10), Word::int32(20), Word::int32(30)],
        );

        vec![array]
    });

    assert_eq!(output, Value::int32(20));
}

/// Element get on one locally constructed array stays correct.
#[test]
fn test_element_get_reads_constructed_array() {
    let mir = r#"
function getLocalElem(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 20int32
    v2: int32 = 30int32
    v3: [int32; 3] = array [int32; 3] (v0, v1, v2)
    v4: int32 = element.get v3, 2
    return v4
}"#;

    run_mir_expect(mir, "getLocalElem", &[], Value::int32(30));
}

/// Element set returns one fresh array value instead of mutating the original.
#[test]
fn test_element_set_preserves_source_array() {
    let mir = r#"
function setWithoutAlias(v0: [int32; 3], v1: int32): int32 {
b0(v0: [int32; 3], v1: int32):
    v2: [int32; 3] = element.set v0, 1, v1
    v3: int32 = element.get v0, 1
    v4: int32 = element.get v2, 1
    v5: int32 = int.add v3, v4
    return v5
}"#;
    let output = run_mir_with_frame_ok(mir, "setWithoutAlias", |interp| {
        let ty = interp.parameter_type("setWithoutAlias", 0);
        let array = interp.materialize_value_for_type(
            ty,
            vec![Word::int32(10), Word::int32(20), Word::int32(30)],
        );

        vec![array, Word::int32(99)]
    });

    assert_eq!(output, Value::int32(119));
}

/// Element set creates a new array with one element replaced.
#[test]
fn test_element_set_replaces_array_element() {
    let mir = r#"
function setAndGet(v0: [int32; 3], v1: int32): int32 {
b0(v0: [int32; 3], v1: int32):
    v2: [int32; 3] = element.set v0, 1, v1
    v3: int32 = element.get v2, 1
    return v3
}"#;
    let output = run_mir_with_frame_ok(mir, "setAndGet", |interp| {
        let ty = interp.parameter_type("setAndGet", 0);
        let arr = interp.materialize_value_for_type(
            ty,
            vec![Word::int32(10), Word::int32(20), Word::int32(30)],
        );
        vec![arr, Word::int32(99)]
    });
    assert_eq!(output, Value::int32(99));
}

/// Element set on one locally constructed array stays correct.
#[test]
fn test_element_set_replaces_constructed_array_element() {
    let mir = r#"
function setLocalAndGet(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 10int32
    v2: int32 = 20int32
    v3: int32 = 30int32
    v4: [int32; 3] = array [int32; 3] (v1, v2, v3)
    v5: [int32; 3] = element.set v4, 1, v0
    v6: int32 = element.get v4, 1
    v7: int32 = element.get v5, 1
    v8: int32 = int.add v6, v7
    return v8
}"#;

    run_mir_expect(
        mir,
        "setLocalAndGet",
        &[Value::int32(99)],
        Value::int32(119),
    );
}

/// Element get copies nested aggregate values.
#[test]
fn test_element_get_copies_nested_aggregate() {
    let mir = r#"
function getNested(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 20int32
    v2: (int32, int32) = tuple (int32, int32) (v0, v1)
    v3: int32 = 30int32
    v4: int32 = 40int32
    v5: (int32, int32) = tuple (int32, int32) (v3, v4)
    v6: [(int32, int32); 2] = array [(int32, int32); 2] (v2, v5)
    v7: (int32, int32) = element.get v6, 1
    v8: int32 = field.get v7, 0
    return v8
}"#;
    run_mir_expect(mir, "getNested", &[], Value::int32(30));
}

/// Element set copies nested aggregate values.
#[test]
fn test_element_set_copies_nested_aggregate() {
    let mir = r#"
function setNested(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 20int32
    v2: (int32, int32) = tuple (int32, int32) (v0, v1)
    v3: int32 = 30int32
    v4: int32 = 40int32
    v5: (int32, int32) = tuple (int32, int32) (v3, v4)
    v6: [(int32, int32); 2] = array [(int32, int32); 2] (v2, v5)
    v7: int32 = 50int32
    v8: int32 = 60int32
    v9: (int32, int32) = tuple (int32, int32) (v7, v8)
    v10: [(int32, int32); 2] = element.set v6, 1, v9
    v11: (int32, int32) = element.get v10, 1
    v12: int32 = field.get v11, 1
    return v12
}"#;
    run_mir_expect(mir, "setNested", &[], Value::int32(60));
}

/// Field address projects one field inside a heap allocation.
#[test]
fn test_field_address_loads_heap_field() {
    let mir = r#"
function heapField(): int32 {
b0:
    v0: ref<(int32, int32), managed, readonly> = new.zeroed (int32, int32)
    v1: int32 = 42int32
    v2: ref<int32, managed, readonly> = field.address v0, 0
    store v2, v1
    v3: int32 = load v2
    return v3
}"#;
    run_mir_expect(mir, "heapField", &[], Value::int32(42));
}

/// Borrowed field addresses preserve heap allocation.
#[test]
fn test_heap_borrowed_field_access() {
    let mir = r#"
function heapBorrowedField(): int32 {
b0:
    v0: ref<(int32, int32), managed, readonly> = new.zeroed (int32, int32)
    v1: int32 = 42int32
    v2: ref<int32, borrowed, readonly> = field.address v0, 0
    store v2, v1
    v3: int32 = load v2
    return v3
}"#;
    run_mir_expect(mir, "heapBorrowedField", &[], Value::int32(42));
}

/// Struct field get reads a single stored field.
#[test]
fn test_struct_field_get_reads_single_field() {
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

/// Managed nominal allocations record their trace map.
#[test]
fn test_new_records_empty_trace_map_for_scalar_struct() {
    let mir = r#"
type Box {
    value: int32;
}

function allocBox(): ref<Box, managed, readonly> {
b0:
    v0: ref<Box, managed, readonly> = new.zeroed Box
    return v0
}"#;

    let mut machine = create_machine(mir);
    let output = machine
        .run_function_by_name("allocBox", &[])
        .expect("execution failed");
    let Value::HeapReference(reference) = output else {
        panic!("expected heap reference value");
    };

    assert_eq!(
        machine
            .heap
            .scan(reference, machine.machine.trace_table().as_ref()),
        Ok(TraceMap::empty())
    );
}

/// Managed nominal stores write the struct field bytes.
#[test]
fn test_store_writes_nominal_field_bytes() {
    let mir = r#"
type Box {
    value: int32;
}

function makeBox(v0: int32): ref<Box, managed, readonly> {
b0(v0: int32):
    v1: Box = struct Box (v0)
    v2: ref<Box, managed, readonly> = new.zeroed Box
    store v2, v1
    return v2
}"#;

    let mut machine = create_machine(mir);
    let output = machine
        .run_function_by_name("makeBox", &[Value::int32(9)])
        .expect("execution failed");
    let Value::HeapReference(reference) = output else {
        panic!("expected heap reference value");
    };
    let bytes = read_heap_bytes(&machine.heap, reference, 4);

    assert_eq!(u32::from_le_bytes(bytes[0..4].try_into().unwrap()), 9);
}

/// Managed nominal layout records padded reference offsets.
#[test]
fn test_new_records_reference_offset_for_padded_struct() {
    let mir = r#"
type Packed {
    first: uint8;
    inner: ref<int32, managed, readonly>;
    third: uint8;
}
function allocPacked(): ref<Packed, managed, readonly> {
b0:
    v0: ref<Packed, managed, readonly> = new.zeroed Packed
    return v0
}"#;
    let data_layout = DataLayout { pointer_bytes: 8 };

    let mut machine = create_machine_with_data_layout(mir, data_layout);
    let output = machine
        .run_function_by_name("allocPacked", &[])
        .expect("execution failed");
    let Value::HeapReference(reference) = output else {
        panic!("expected heap reference value");
    };

    assert_eq!(
        machine
            .heap
            .scan(reference, machine.machine.trace_table().as_ref()),
        Ok(TraceMap::Fixed {
            local_offsets: vec![8].into_boxed_slice(),
            shared_offsets: Vec::new().into_boxed_slice(),
        })
    );
}

/// Slices of heap references use pointer-shaped element stride.
#[test]
fn test_new_slice_uses_pointer_stride_for_heap_references() {
    let mir = r#"
function allocArray(): slice<ref<int32, managed, readonly>, managed> {
b0:
    v0: int64 = 2int64
    v1: slice<ref<int32, managed, readonly>, managed> = new.slice.zeroed ref<int32, managed, readonly>, v0
    return v1
}"#;
    let data_layout = DataLayout { pointer_bytes: 8 };

    let mut machine = create_machine_with_data_layout(mir, data_layout);
    let output = machine
        .run_function_by_name("allocArray", &[])
        .expect("execution failed");
    let Value::HeapReference(slice) = output else {
        panic!("expected heap slice value, got {output:?}");
    };
    let bytes = read_heap_bytes(&machine.heap, slice, 2 * HeapReference::BYTE_LEN);
    let reference = decode_heap_reference(&bytes, 0);

    assert_eq!(
        machine
            .heap
            .scan(reference, machine.machine.trace_table().as_ref()),
        Ok(TraceMap::Fixed {
            local_offsets: vec![0, 8].into_boxed_slice(),
            shared_offsets: Vec::new().into_boxed_slice(),
        })
    );
}

/// Managed field addresses load referenced heap values.
#[test]
fn test_field_address_loads_referenced_heap_value() {
    let mir = r#"
type Holder {
    value: ref<int32, managed, readonly>;
}

function comparePaths(): int32 {
b0:
    v0: ref<int32, managed, readonly> = new.zeroed int32
    v1: int32 = 41int32
    store v0, v1
    v2: Holder = struct Holder (v0)
    v3: ref<Holder, managed, readonly> = new.zeroed Holder
    store v3, v2
    v4: ref<ref<int32, managed, readonly>, managed, readonly> = field.address v3, 0
    v5: ref<int32, managed, readonly> = load v4
    v6: int32 = load v5
    return v6
}"#;
    let mut machine = create_machine(mir);
    let output = machine
        .run_function_by_name("comparePaths", &[])
        .expect("execution failed");

    assert_eq!(output, Value::int32(41));
}

/// Frame allocation creates frame-local memory.
#[test]
fn test_frame_allocate() {
    let mir = r#"
function stackAlloc(): int32 {
b0:
    v0: ref<int32, raw, readonly, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 99int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
    run_mir_expect(mir, "stackAlloc", &[], Value::int32(99));
}

/// Frame allocation with field access.
#[test]
fn test_frame_allocate_struct() {
    let mir = r#"
function stackStruct(): int32 {
b0:
    v0: ref<(int32, int32), raw, readonly, space(frame)> = frame.alloc.zeroed (int32, int32)
    v1: int32 = 10int32
    v2: ref<int32, borrowed, readonly, space(frame)> = field.address v0, 0
    store v2, v1
    v3: int32 = load v2
    return v3
}"#;
    run_mir_expect(mir, "stackStruct", &[], Value::int32(10));
}

/// Frame allocation rejects heap-reference fields narrower than the host heap.
#[test]
fn test_frame_allocate_pointer32_heap_reference_field() {
    let mir = r#"
type Packed {
    first: uint8;
    inner: ref<int32, managed, readonly>;
    third: uint8;
}
function stackPacked(): int32 {
b0:
    v0: ref<int32, managed, readonly> = new.zeroed int32
    v1: int32 = 77int32
    store v0, v1
    v2: ref<Packed, raw, readonly, space(frame)> = frame.alloc.zeroed Packed
    v3: ref<ref<int32, managed, readonly>, borrowed, readonly, space(frame)> = field.address v2, 1
    store v3, v0
    v4: ref<int32, managed, readonly> = load v3
    v5: int32 = load v4
    return v5
}"#;
    let data_layout = DataLayout { pointer_bytes: 4 };
    let error = match std::panic::catch_unwind(|| create_machine_with_data_layout(mir, data_layout))
    {
        Ok(_) => panic!("narrow heap reference storage should be rejected loudly"),
        Err(error) => error,
    };
    let message = if let Some(message) = error.downcast_ref::<String>() {
        message.as_str()
    } else if let Some(message) = error.downcast_ref::<&str>() {
        message
    } else {
        panic!("unexpected panic value");
    };

    assert_eq!(
        message,
        "failed to initialize machine: EM040: incompatible pointer width: program 4 bytes, host 8"
    );
}

/// Explicit null checks branch before dereference.
#[test]
fn test_null_pointer_check_branches() {
    let mir = r#"
function nullCheck(v0: ref<int32, raw, readonly>): int32 {
b0(v0: ref<int32, raw, readonly>):
    check null v0 -> b1, b2
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 0int32
    return v2
}"#;
    run_mir_expect(mir, "nullCheck", &[Value::address(0)], Value::int32(0));

    run_mir_expect(mir, "nullCheck", &[Value::address(8)], Value::int32(1));
}
