use destack_native::FrameSource;

use crate::tests::TestProgram;

/// Preserve one live MIR value in the physical frame map at a runtime poll.
#[test]
fn test_emit_native_frame_map() {
    let program = TestProgram::mir(
        r#"
export function advance(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.add v0, v0
    poll
    return v1
}
"#,
    );
    let object = program.assert_native(
        r#"
function u0:0(i64, i32) -> i32 native {
    ss0 = explicit_slot 1, key = 0
    ss1 = explicit_slot 4, align = 4, key = 1
    gv0 = symbol colocated userextname0
    sig0 = (i64, i32, i64) native

block0(v0: i64, v1: i32):
    v2 = iadd v1, v1
    v3 = load.i64 notrap aligned v0+120
    v4 = atomic_load.i32 notrap aligned v3
    brif v4, block1, block2

block1:
    v5 = stack_addr.i64 ss0
    v6 = stack_addr.i64 ss1
    store.i32 notrap aligned v2, v6
    v7 = symbol_value.i64 gv0
    v8 = load.i32 notrap aligned v7
    v9 = load.i64 notrap aligned v0+8
    v10 = load.i64 notrap aligned v9+48
    call_indirect sig0, v10(v0, v8, v5), stack_map=[i8 @ ss0+0, i8 @ ss1+0]
    trap user4

block2:
    return v2
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );
    let sections = object.sections();
    let map = object.map();
    let [frame] = map.frames(sections) else {
        panic!("native poll should emit one frame map");
    };
    let [value] = map.values(sections, *frame) else {
        panic!("native frame map should retain one live value");
    };
    let [location] = map.locations(sections, *value) else {
        panic!("native value should occupy one stack location");
    };
    let definition = object.definitions()[0]
        .get()
        .expect("native function should retain one definition");

    // map the canonical four-byte value through the function body stack frame
    assert_eq!(frame.block, definition.body);
    assert_eq!(frame.state, 0);
    assert_eq!(location.source, FrameSource::Stack);
    assert_eq!(location.value_offset, 0);
    assert_eq!(location.byte_len, 4);
}
