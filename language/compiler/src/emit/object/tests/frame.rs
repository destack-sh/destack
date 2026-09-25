use crate::tests::TestProgram;
use destack_bytecode::{RegisterId, RegisterSpan};
use destack_native::FrameSource;

/// Map one runtime poll onto the exact live bytecode registers at its resume point.
#[test]
fn test_emit_frame_map() {
    let program = TestProgram::mir(
        r#"
export function advance(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = add v0, v0
    poll
    return v1
}
"#,
    );
    let object = program.assert_bytecode(
        r#"
function advance {
    add.int32 r1, r0, r0
    poll
    return r1
}
"#,
    );
    let bytecode = object
        .bytecode()
        .expect("bytecode emission should attach bytecode");
    let maps = bytecode.frames();
    let registers = bytecode.registers();

    // retain only the result live when execution resumes after the poll
    assert_eq!(maps.len(), 1);
    assert_eq!(
        maps[0].registers(registers),
        &[RegisterSpan::new(RegisterId(1), 1)]
    );

    let object = program.assert_native(
        r#"
function u0:0(i64 vmctx, i32) -> i32 native {
    ss0 = explicit_slot 1, key = 0
    ss1 = explicit_slot 4, align = 4, key = 1
    region0 = 0 "activation"
    region1 = 1 "world"
    gv0 = vmctx
    gv1 = load.i64 notrap aligned gv0+48
    gv2 = symbol colocated userextname0
    sig0 = (i64, i32, i64) native
    stack_limit = gv1

block0(v0: i64, v1: i32):
    v2 = iadd v1, v1
    v3 = load.i64 notrap aligned region0 v0+112
    v4 = atomic_load.i32 notrap aligned v3
    brif v4, block1, block2

block1:
    v5 = stack_addr.i64 ss0
    v6 = stack_addr.i64 ss1
    store.i32 notrap aligned region1 v2, v6
    v7 = symbol_value.i64 gv2
    v8 = load.i32 notrap aligned v7
    v9 = load.i64 notrap aligned region0 v0+8
    v10 = load.i64 notrap aligned v9+40
    call_indirect sig0, v10(v0, v8, v5), stack_map=[i8 @ ss0+0, i8 @ ss1+0]
    trap user4

block2:
    return v2
}

function u1:0(i64, i64, i64) native {
    sig0 = (i64 vmctx, i32) -> i32 native
    fn0 = colocated u0:0 sig0

block0(v0: i64, v1: i64, v2: i64):
    v3 = load.i32 notrap aligned v1
    v4 = call fn0(v0, v3)
    store notrap aligned v4, v2
    return
}
"#,
    );

    // walk the emitted frame map down to the single live stack location
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

    // recover the emitted body block the frame should point at
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
