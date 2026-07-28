use destack_core::Optional;
use destack_program::{LayoutField, LayoutShapeBuilder, ScalarFormat, TypeId, Word};

use super::{TestMachine, TestProgram};

/// Execute linked global addressing and typed memory access against runtime statics.
#[test]
fn test_execute_global_memory() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    global.address r3, g0
    pointer.global r1, r3
    store.int32 r1, r0
    load.int32 r2, r1
    return r2
}

"#,
        TestProgram::words().local_global(),
    );

    let value = machine.complete(0, &[Word::int32(41)]);

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
    let program = TestProgram::words()
        .layout(0, LayoutShapeBuilder::Struct(fields), 16, 8)
        .layout(
            1,
            LayoutShapeBuilder::Scalar(ScalarFormat::int(64, false)),
            8,
            8,
        );
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    aggregate r2:r3, ([r0, 0, 8], [r1, 8, 8])
    constant.zeroed r4:r5
    frame.address r11, r4:r5
    pointer.frame r6, r11
    store r6, r2:r3, 16
    load r7:r8, r6, 16
    extract r9, r7:r8, 0, 8
    extract r10, r7:r8, 8, 8
    return r9:r10
}
"#,
        program,
    );

    let value = machine.complete(0, &[Word::uint64(0x1122_3344_5566_7788), Word::uint64(89)]);

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
function f0 {
    global.address r11, g0
    pointer.global r1, r11
    pointer.add r2, r1, 4
    constant.uint64 r3, 4
    store.uint32 r1, r0
    copy.bytes r1 -> r2, r3
    cast.pointerToInt.pointer.uint64 r4, r1
    cast.intToPointer.uint64.pointer r5, r4
    constant.int64 r6, 2
    pointer.add r7, r5, r6, 4
    pointer.byteOffsetFrom r8, r2, r7
    load.uint32 r9, r2
    compare.bytes r10, r1, r2, r3
    return r8:r10
}
"#,
        TestProgram::words().local_global(),
    );

    let value = machine.complete(0, &[Word::uint32(0x1122_3344)]);

    assert_eq!(
        value,
        vec![Word::int64(-4), Word::uint32(0x1122_3344), Word::int32(0)]
    );
}

/// Execute atomic read-modify-write and load against linked static storage.
#[test]
fn test_execute_atomics() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    global.address r4, g0
    pointer.global r1, r4
    atomic.rmw.add.uint32 r2, r1, r0, sequentiallyConsistent
    atomic.load.uint32 r3, r1, acquire
    atomic.fence sequentiallyConsistent, storage(shared)
    return r2:r3
}
"#,
        TestProgram::words().shared_global(),
    );

    let value = machine.complete(0, &[Word::uint32(17)]);

    assert_eq!(value, vec![Word::uint32(0), Word::uint32(17)]);
}
