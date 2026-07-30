use destack_mir::{Function, Space, Storage};

use crate::tests::TestProgram;

#[test]
fn test_generate_distinct_destructors_for_generic_instances() {
    let mut program = TestProgram::mir(
        r#"
type Box<int32> {
    value: ref<int32, unique, mutable>;
}

type Box<float64> {
    value: ref<float64, unique, mutable>;
}

function test(
    v0: ref<int32, unique, mutable>,
    v1: ref<float64, unique, mutable>,
): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<float64, unique, mutable>):
    v2: Box<int32> = aggregate (v0)
    v3: Box<float64> = aggregate (v1)
    return
}
"#,
    );

    // elaborate both concrete type instances
    let _ = program.run_elaborate();
    let destructors = program
        .lowered
        .tree
        .iter_nodes::<Function>()
        .filter_map(|(_, function)| {
            (program.strings.get(function.name) == "Box.destruct.frame")
                .then_some((&function.arguments, function.symbol))
        })
        .collect::<Vec<_>>();

    // preserve distinct textual and persistent destructor identities
    assert_eq!(destructors.len(), 2);
    assert_ne!(destructors[0].0, destructors[1].0);
    assert_ne!(destructors[0].1, destructors[1].1);
}

#[test]
fn test_generate_destructors_for_reachable_storage() {
    let mut program = TestProgram::mir(
        r#"
type Item {
    value: ref<int32, unique, mutable>;
}

function test(): void {
entry:
    v0: ref<Item, managed, mutable> = new.zeroed Item
    v1: ref<Item, managed, mutable, shared> = new.zeroed Item
    return
}
"#,
    );

    // generate only the heap placements reached by allocation operations
    let _ = program.run_elaborate();
    let item = program.type_by_name("Item");
    let local = program
        .lowered
        .drops
        .destructor(item, Storage::Heap(Space::Local));
    let shared = program
        .lowered
        .drops
        .destructor(item, Storage::Heap(Space::Shared));
    let frame = program.lowered.drops.destructor(item, Storage::Frame);

    assert!(local.is_some());
    assert!(shared.is_some());
    assert_ne!(local, shared);
    assert_eq!(frame, None);
}

#[test]
fn test_insert_drop_calls_destructor_for_hook() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

external function dropBox(ref<Box, borrowed, exclusive, frame>): void

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    return
}
"#,
    );
    program.mark_drop_hook("Box", "dropBox");

    program.assert_drop_mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

external function dropBox(ref<Box, borrowed, exclusive, frame>): void

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    drop v1
    return
}

function Box.destruct.frame(v0: ref<Box, borrowed, exclusive, frame>): void {
entry(v0: ref<Box, borrowed, exclusive, frame>):
    v1: ref<Box, borrowed, exclusive, frame> = cast.bit v0 -> ref<Box, borrowed, exclusive, frame>
    call dropBox(v1): (ref<Box, borrowed, exclusive, frame>) => void
    v2: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 0
    v3: ref<int32, unique, mutable> = load v2
    free v3
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_skips_receiver_inside_hook() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function dropBox(v0: ref<Box, borrowed, exclusive, frame>): void {
entry(v0: ref<Box, borrowed, exclusive, frame>):
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    return
}
"#,
    );
    program.mark_drop_hook("Box", "dropBox");

    program.assert_drop_mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function dropBox(v0: ref<Box, borrowed, exclusive, frame>): void {
entry(v0: ref<Box, borrowed, exclusive, frame>):
    return
}

function test(v0: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>):
    v1: Box = aggregate (v0)
    drop v1
    return
}

function Box.destruct.frame(v0: ref<Box, borrowed, exclusive, frame>): void {
entry(v0: ref<Box, borrowed, exclusive, frame>):
    v1: ref<Box, borrowed, exclusive, frame> = cast.bit v0 -> ref<Box, borrowed, exclusive, frame>
    call dropBox(v1): (ref<Box, borrowed, exclusive, frame>) => void
    v2: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 0
    v3: ref<int32, unique, mutable> = load v2
    free v3
    return
}
"#,
    );
}

#[test]
fn test_generate_struct_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    return
}
"#,
    );

    program.assert_elaborated_mir(
        r#"
type Pair {
    left: ref<int32, unique, mutable>;
    right: ref<int32, unique, mutable>;
}

function test(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>): void {
entry(v0: ref<int32, unique, mutable>, v1: ref<int32, unique, mutable>):
    v2: Pair = aggregate (v0, v1)
    drop v2
    return
}

function Pair.destruct.frame(v0: ref<Pair, borrowed, exclusive, frame>): void {
entry(v0: ref<Pair, borrowed, exclusive, frame>):
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 1
    v2: ref<int32, unique, mutable> = load v1
    free v2
    v3: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 0
    v4: ref<int32, unique, mutable> = load v3
    free v4
    return
}
"#,
    );
}

#[test]
fn test_generate_managed_allocation_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Item {
    value: ref<int32, unique, mutable>;
}

external function dropItem(ref<Item, borrowed, exclusive, local>): void

function test(): ref<Item, managed, mutable> {
entry:
    v0: ref<Item, managed, mutable> = new.zeroed Item
    return v0
}
"#,
    );
    program.mark_drop_hook("Item", "dropItem");

    program.assert_drop_mir(
        r#"
type Item {
    value: ref<int32, unique, mutable>;
}

external function dropItem(ref<Item, borrowed, exclusive>): void

function test(): ref<Item, managed, mutable> {
entry:
    v0: ref<Item, managed, mutable> = new.zeroed Item
    return v0
}

function Item.destruct.local(v0: ref<Item, borrowed, exclusive>): void {
entry(v0: ref<Item, borrowed, exclusive>):
    v1: ref<Item, borrowed, exclusive> = cast.bit v0 -> ref<Item, borrowed, exclusive>
    call dropItem(v1): (ref<Item, borrowed, exclusive>) => void
    v2: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v0, 0
    v3: ref<int32, unique, mutable> = load v2
    free v3
    return
}
"#,
    );
}

#[test]
fn test_generate_uninit_managed_allocation_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Item {
    value: ref<int32, unique, mutable>;
}

external function dropItem(ref<Item, borrowed, exclusive, local>): void

function test(): uninit<ref<Item, managed, mutable>> {
entry:
    v0: uninit<ref<Item, managed, mutable>> = new.uninit Item
    return v0
}
"#,
    );
    program.mark_drop_hook("Item", "dropItem");

    program.assert_elaborated_mir(
        r#"
type Item {
    value: ref<int32, unique, mutable>;
}

external function dropItem(ref<Item, borrowed, exclusive>): void

function test(): uninit<ref<Item, managed, mutable>> {
entry:
    v0: uninit<ref<Item, managed, mutable>> = new.uninit Item
    return v0
}

function Item.destruct.local(v0: ref<Item, borrowed, exclusive>): void {
entry(v0: ref<Item, borrowed, exclusive>):
    v1: ref<Item, borrowed, exclusive> = cast.bit v0 -> ref<Item, borrowed, exclusive>
    call dropItem(v1): (ref<Item, borrowed, exclusive>) => void
    v2: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v0, 0
    v3: ref<int32, unique, mutable> = load v2
    free v3
    return
}
"#,
    );
}

#[test]
fn test_skip_destructor_for_dynamic_field() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

type Entry {
    writer: dynamic<Writer>;
}

function test(v0: dynamic<Writer>): void {
entry(v0: dynamic<Writer>):
    v1: Entry = aggregate (v0)
    return
}
"#,
    );

    program.assert_elaborated_mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

type Entry {
    writer: dynamic<Writer>;
}

function test(v0: dynamic<Writer>): void {
entry(v0: dynamic<Writer>):
    v1: Entry = aggregate (v0)
    return
}
"#,
    );
}

#[test]
fn test_generate_struct_destructor_with_unique_slice_field() {
    let mut program = TestProgram::mir(
        r#"
type Buffer {
    items: slice<int32, unique, mutable>;
}

function test(v0: slice<int32, unique, mutable>): void {
entry(v0: slice<int32, unique, mutable>):
    v1: Buffer = aggregate (v0)
    return
}
"#,
    );

    program.assert_elaborated_mir(
        r#"
type Buffer {
    items: slice<int32, unique, mutable>;
}

function test(v0: slice<int32, unique, mutable>): void {
entry(v0: slice<int32, unique, mutable>):
    v1: Buffer = aggregate (v0)
    drop v1
    return
}

function Buffer.destruct.frame(v0: ref<Buffer, borrowed, exclusive, frame>): void {
entry(v0: ref<Buffer, borrowed, exclusive, frame>):
    v1: ref<slice<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 0
    v2: slice<int32, unique, mutable> = load v1
    free v2
    return
}
"#,
    );
}

#[test]
fn test_generate_struct_destructor_with_unique_tensor_view_field() {
    let mut program = TestProgram::mir(
        r#"
type Buffer {
    items: tensorView<int32, unique, mutable, (2, 2)>;
}

function test(v0: tensorView<int32, unique, mutable, (2, 2)>): void {
entry(v0: tensorView<int32, unique, mutable, (2, 2)>):
    v1: Buffer = aggregate (v0)
    return
}
"#,
    );

    program.assert_elaborated_mir(
        r#"
type Buffer {
    items: tensorView<int32, unique, mutable, (2, 2)>;
}

function test(v0: tensorView<int32, unique, mutable, (2, 2)>): void {
entry(v0: tensorView<int32, unique, mutable, (2, 2)>):
    v1: Buffer = aggregate (v0)
    drop v1
    return
}

function Buffer.destruct.frame(v0: ref<Buffer, borrowed, exclusive, frame>): void {
entry(v0: ref<Buffer, borrowed, exclusive, frame>):
    v1: ref<tensorView<int32, unique, mutable, (2, 2)>, borrowed, exclusive, frame> = field.address v0, 0
    v2: tensorView<int32, unique, mutable, (2, 2)> = load v1
    free v2
    return
}
"#,
    );
}

#[test]
fn test_generate_variant_destructor() {
    let mut program = TestProgram::mir(
        r#"
@copy
type ValueStorage = [usize; 1];

type Value = variant<uint8, ValueStorage> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    return
}
"#,
    );

    program.assert_elaborated_mir(
        r#"
@copy
type ValueStorage = [usize; 1];

type Value = variant<uint8, ValueStorage> { 0uint8 = ref<int32, unique, mutable>; 1uint8 = int32; };

function test(v0: Value): void {
entry(v0: Value):
    drop v0
    return
}

function Value.destruct.frame(v0: ref<Value, borrowed, exclusive, frame>): void {
entry(v0: ref<Value, borrowed, exclusive, frame>):
    v1: ref<uint8, borrowed, exclusive, frame> = field.address v0, 0
    v2: uint8 = load v1
    v3: ref<ValueStorage, borrowed, exclusive, frame> = field.address v0, 1
    v4: uint8 = 0
    v5: boolean = int.eq v2, v4
    branch v5, b1, b2

b1:
    v6: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = cast.bit v3 -> ref<ref<int32, unique, mutable>, borrowed, exclusive, frame>
    v7: ref<int32, unique, mutable> = load v6
    free v7
    jump b2

b2:
    return
}
"#,
    );
}

#[test]
fn test_generate_unique_slice_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: slice<Box, unique, mutable>): void {
entry(v0: slice<Box, unique, mutable>):
    return
}
"#,
    );

    program.assert_elaborated_mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: slice<Box, unique, mutable>): void {
entry(v0: slice<Box, unique, mutable>):
    drop v0
    return
}

function slice.Box.destruct.frame(v0: ref<slice<Box, unique, mutable>, borrowed, exclusive, frame>): void {
entry(v0: ref<slice<Box, unique, mutable>, borrowed, exclusive, frame>):
    v1: slice<Box, unique, mutable> = load v0
    v2: usize = slice.length v1
    v3: usize = 0
    jump b1(v2)

b1(v4: usize):
    v5: boolean = int.ne v4, v3
    branch v5, b2, b3

b2:
    v6: usize = 1
    v7: usize = int.sub v4, v6
    v8: ref<Box, unique, mutable> = element.address v1, v7
    v9: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v8, 0
    v10: ref<int32, unique, mutable> = load v9
    free v10
    jump b1(v7)

b3:
    v11: slice<Box, unique, mutable> = load v0
    free v11
    return
}

function Box.destruct.local(v0: ref<Box, borrowed, exclusive>): void {
entry(v0: ref<Box, borrowed, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, unique, mutable> = load v1
    free v2
    return
}
"#,
    );
}

#[test]
fn test_generate_nested_unique_slice_destructor() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: slice<slice<Box, unique, mutable>, unique, mutable>): void {
entry(v0: slice<slice<Box, unique, mutable>, unique, mutable>):
    return
}
"#,
    );

    program.assert_elaborated_mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: slice<slice<Box, unique, mutable>, unique, mutable>): void {
entry(v0: slice<slice<Box, unique, mutable>, unique, mutable>):
    drop v0
    return
}

function slice.slice.Box.unique.destruct.frame(v0: ref<slice<slice<Box, unique, mutable>, unique, mutable>, borrowed, exclusive, frame>): void {
entry(v0: ref<slice<slice<Box, unique, mutable>, unique, mutable>, borrowed, exclusive, frame>):
    v1: slice<slice<Box, unique, mutable>, unique, mutable> = load v0
    v2: usize = slice.length v1
    v3: usize = 0
    jump b1(v2)

b1(v4: usize):
    v5: boolean = int.ne v4, v3
    branch v5, b2, b3

b2:
    v6: usize = 1
    v7: usize = int.sub v4, v6
    v8: ref<slice<Box, unique, mutable>, unique, mutable> = element.address v1, v7
    v9: slice<Box, unique, mutable> = load v8
    v10: usize = slice.length v9
    v11: usize = 0
    jump b4(v10)

b3:
    v20: slice<slice<Box, unique, mutable>, unique, mutable> = load v0
    free v20
    return

b4(v12: usize):
    v13: boolean = int.ne v12, v11
    branch v13, b5, b6

b5:
    v14: usize = 1
    v15: usize = int.sub v12, v14
    v16: ref<Box, unique, mutable> = element.address v9, v15
    v17: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v16, 0
    v18: ref<int32, unique, mutable> = load v17
    free v18
    jump b4(v15)

b6:
    v19: slice<Box, unique, mutable> = load v8
    free v19
    jump b1(v7)
}

function slice.Box.destruct.local(v0: ref<slice<Box, unique, mutable>, borrowed, exclusive>): void {
entry(v0: ref<slice<Box, unique, mutable>, borrowed, exclusive>):
    v1: slice<Box, unique, mutable> = load v0
    v2: usize = slice.length v1
    v3: usize = 0
    jump b1(v2)

b1(v4: usize):
    v5: boolean = int.ne v4, v3
    branch v5, b2, b3

b2:
    v6: usize = 1
    v7: usize = int.sub v4, v6
    v8: ref<Box, unique, mutable> = element.address v1, v7
    v9: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v8, 0
    v10: ref<int32, unique, mutable> = load v9
    free v10
    jump b1(v7)

b3:
    v11: slice<Box, unique, mutable> = load v0
    free v11
    return
}

function Box.destruct.local(v0: ref<Box, borrowed, exclusive>): void {
entry(v0: ref<Box, borrowed, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, unique, mutable> = load v1
    free v2
    return
}
"#,
    );
}

#[test]
fn test_free_unique_slice_with_dynamic_elements() {
    let mut program = TestProgram::mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

function test(v0: slice<dynamic<Writer>, unique, mutable>): void {
entry(v0: slice<dynamic<Writer>, unique, mutable>):
    return
}
"#,
    );

    program.assert_elaborated_mir(
        r#"
@copy
type Writer {
    write: fn() => uint32;
}

function test(v0: slice<dynamic<Writer>, unique, mutable>): void {
entry(v0: slice<dynamic<Writer>, unique, mutable>):
    free v0
    return
}
"#,
    );
}

#[test]
fn test_insert_drop_drops_unique_pointee_before_free() {
    let mut program = TestProgram::mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    v1: Box = load v0
    return
}
"#,
    );

    program.assert_elaborated_mir(
        r#"
type Box {
    value: ref<int32, unique, mutable>;
}

function test(v0: ref<Box, unique, mutable>): void {
entry(v0: ref<Box, unique, mutable>):
    v1: Box = load v0
    v2: Box = load v0
    drop v2
    free v0
    drop v1
    return
}

function Box.destruct.frame(v0: ref<Box, borrowed, exclusive, frame>): void {
entry(v0: ref<Box, borrowed, exclusive, frame>):
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive, frame> = field.address v0, 0
    v2: ref<int32, unique, mutable> = load v1
    free v2
    return
}

function Box.destruct.local(v0: ref<Box, borrowed, exclusive>): void {
entry(v0: ref<Box, borrowed, exclusive>):
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, unique, mutable> = load v1
    free v2
    return
}
"#,
    );
}
