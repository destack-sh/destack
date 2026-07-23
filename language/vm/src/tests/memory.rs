use destack_core::Optional;
use destack_program::{LayoutField, LayoutShapeBuilder, ScalarFormat, TypeId, Word};

use super::{TestMachine, TestProgram};

/// Execute linked global addressing and typed memory access against runtime statics.
#[test]
fn test_execute_global_memory() {
    let mut machine = TestMachine::parse(
        r#"
type State

local global state: State = zero

export function update(r0: int32): int32 {
    slot s0: State = r1[1]

    r1: pointer = global.address state
    store.int32 r1, r0
    r2: int32 = load.int32 r1
    return r2
}

"#,
        TestProgram::new(),
    );

    let value = machine.complete("update", &[Word::int32(41)]);

    assert_eq!(value, vec![Word::int32(41)]);
}

/// Move one packed value through pointer memory using its exact Program layout.
#[test]
fn test_execute_value_memory() {
    let fields = vec![
        LayoutField {
            name: Optional::none(),
            ty: TypeId(1),
            offset: 0,
            size: 8,
            alignment: 8,
        },
        LayoutField {
            name: Optional::none(),
            ty: TypeId(1),
            offset: 8,
            size: 8,
            alignment: 8,
        },
    ];
    let program = TestProgram::new()
        .layout(0, LayoutShapeBuilder::Struct(fields), 16, 8)
        .layout(
            1,
            LayoutShapeBuilder::Scalar(ScalarFormat::int(64, false)),
            8,
            8,
        );
    let mut machine = TestMachine::parse(
        r#"
type Pair
type Uint64

export function retain(r0: uint64, r1: uint64): (uint64, uint64) {
    slot s0: Pair

    r2: words<2> = aggregate Pair (r0, r1)
    r4: pointer = frame.address s0
    store r4, r2, Pair
    r5: words<2> = load r4, Pair
    r7: uint64 = field.get r5, Pair, 0
    r8: uint64 = field.get r5, Pair, 1
    return r7, r8
}
"#,
        program,
    );

    let value = machine.complete(
        "retain",
        &[Word::uint64(0x1122_3344_5566_7788), Word::uint64(89)],
    );

    assert_eq!(
        value,
        vec![Word::uint64(0x1122_3344_5566_7788), Word::uint64(89)]
    );
}

/// Execute byte-range transfer and comparison against linked static storage.
#[test]
fn test_execute_byte_ranges() {
    let mut machine = TestMachine::parse(
        r#"
type State

local global state: State = zero

export function copy(r0: uint32): (int64, uint32, int32) {
    r1: pointer = global.address state
    r2: pointer = pointer.offset r1, 4
    r3: uint64 = 4
    store.uint32 r1, r0
    copy.bytes r1 -> r2, r3
    r4: uint64 = cast.pointerToInt r1 -> uint64
    r5: pointer = cast.intToPointer r4 -> pointer
    r6: uint64 = 1
    r7: pointer = pointer.index r5, r6, stride(4)
    r8: int64 = pointer.distance r2, r7
    r9: uint32 = load.uint32 r2
    r10: int32 = compare.bytes r1, r2, r3
    return r8, r9, r10
}
"#,
        TestProgram::new(),
    );

    let value = machine.complete("copy", &[Word::uint32(0x1122_3344)]);

    assert_eq!(
        value,
        vec![Word::int64(0), Word::uint32(0x1122_3344), Word::int32(0)]
    );
}

/// Execute atomic read-modify-write and load against linked static storage.
#[test]
fn test_execute_atomics() {
    let mut machine = TestMachine::parse(
        r#"
type State

shared global state: State = zero

export function add(r0: uint32): (uint32, uint32) {
    r1: pointer = global.address state
    r2: uint32 = atomic.rmw.add.uint32 r1, r0, sequentiallyConsistent
    r3: uint32 = atomic.load.uint32 r1, acquire
    atomic.fence sequentiallyConsistent, storage(shared)
    return r2, r3
}
"#,
        TestProgram::new(),
    );

    let value = machine.complete("add", &[Word::uint32(17)]);

    assert_eq!(value, vec![Word::uint32(0), Word::uint32(17)]);
}
