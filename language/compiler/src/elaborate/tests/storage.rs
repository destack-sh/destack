use destack_mir::{Space, Storage};

use crate::tests::TestProgram;

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
    program.elaborate();
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

    program.assert_elaborated(
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

function drop.local<Item>(v0: ref<Item, borrowed, exclusive>): void {
entry(v0: ref<Item, borrowed, exclusive>):
    call dropItem(v0): (ref<Item, borrowed, exclusive>) => void
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, unique, mutable> = load v1
    free v2
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

    program.assert_elaborated(
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

function drop.local<Item>(v0: ref<Item, borrowed, exclusive>): void {
entry(v0: ref<Item, borrowed, exclusive>):
    call dropItem(v0): (ref<Item, borrowed, exclusive>) => void
    v1: ref<ref<int32, unique, mutable>, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, unique, mutable> = load v1
    free v2
    return
}
"#,
    );
}
