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
    v0: ref<Item, managed, mutable, local> = new.zeroed Item, local
    v1: ref<Item, managed, mutable, shared> = new.zeroed Item, shared
    return
}
"#,
    );

    // generate only the heap placements reached by allocation operations
    program.optimize();
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

external function dropItem<'a>(ref<Item, borrowed, 'a, mutable>): void

function test(): ref<Item, managed, mutable, local> {
entry:
    v0: ref<Item, managed, mutable, local> = new.zeroed Item, local
    return v0
}
"#,
    );

    program.mark_drop_hook("Item", "dropItem");

    program.assert_optimized(
        r#"
type Item {
    value: ref<int32, unique, mutable>;
}

external function dropItem<'a>(ref<Item, borrowed, 'a, mutable>): void

function test(): ref<Item, managed, mutable, local> {
entry:
    v0: ref<Item, managed, mutable, local> = new.zeroed Item, local
    return v0
}

function drop.local<Item, 'a>(v0: ref<Item, borrowed, 'a, exclusive>): void {
entry(v0: ref<Item, borrowed, 'a, exclusive>):
    v1: ref<Item, borrowed, 'a, mutable> = address (*v0)
    call dropItem(v1): <'a_1>(ref<Item, borrowed, 'a_1, mutable>) => void
    v2: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v3: ref<int32, unique, mutable> = load (*v2)
    release v3
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

external function dropItem<'a>(ref<Item, borrowed, 'a, mutable>): void

function test(): uninit<ref<Item, managed, mutable, local>> {
entry:
    v0: uninit<ref<Item, managed, mutable, local>> = new.uninit Item, local
    return v0
}
"#,
    );

    program.mark_drop_hook("Item", "dropItem");

    program.assert_optimized(
        r#"
type Item {
    value: ref<int32, unique, mutable>;
}

external function dropItem<'a>(ref<Item, borrowed, 'a, mutable>): void

function test(): uninit<ref<Item, managed, mutable, local>> {
entry:
    v0: uninit<ref<Item, managed, mutable, local>> = new.uninit Item, local
    return v0
}

function drop.local<Item, 'a>(v0: ref<Item, borrowed, 'a, exclusive>): void {
entry(v0: ref<Item, borrowed, 'a, exclusive>):
    v1: ref<Item, borrowed, 'a, mutable> = address (*v0)
    call dropItem(v1): <'a_1>(ref<Item, borrowed, 'a_1, mutable>) => void
    v2: ref<ref<int32, unique, mutable>, borrowed, 'a, exclusive> = address (*v0).0
    v3: ref<int32, unique, mutable> = load (*v2)
    release v3
    return
}
"#,
    );
}
