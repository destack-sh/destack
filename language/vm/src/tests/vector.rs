use destack_mir::Space;
use destack_program::{MemoryAccess, StopReason, WatchSet, Word};

use super::{TestMachine, TestProgram};

/// Execute packed vector arithmetic, masks, projections, and conversions.
#[test]
fn test_execute_vectors() {
    let mut machine = TestMachine::parse(
        r#"
export function vectors(
    r0: vector<int32, 4>,
    r2: vector<int32, 4>,
    r4: uint32,
    r5: int32,
): (vector<int32, 4>, int32, int32) {
    r6: vector<int32, 4> = int.add r0, r2
    r8: vector<boolean, 4> = vector.compare int.gt, r6, r0
    r9: vector<int32, 4> = select r8, r6, r0
    r11: vector<int32, 4> = vector.insert r9, r4, r5
    r13: vector<int32, 4> = vector.shuffle r11, r2, [0, 5, 2, 7]
    r15: int32 = vector.reduce int.add, r9
    r16: int32 = vector.extract r9, r4
    return r13, r15, r16
}

export function convert(
    r0: vector<float32, 4>,
    r2: uint32,
): (vector<int16, 4>, int16) {
    r3: vector<int16, 4> = vector.convert roundFloor, r0
    r4: int16 = vector.extract r3, r2
    return r3, r4
}
"#,
        TestProgram::new(),
    );

    // execute one multiword integer vector pipeline
    let value = machine.complete(
        "vectors",
        &[
            pack_i32(1, 2),
            pack_i32(3, 4),
            pack_i32(10, 20),
            pack_i32(30, 40),
            Word::uint32(2),
            Word::int32(-7),
        ],
    );
    assert_eq!(
        value,
        vec![
            pack_i32(11, 20),
            pack_i32(-7, 40),
            Word::int32(110),
            Word::int32(33),
        ]
    );

    // convert every floating lane under one explicit rounding mode
    let value = machine.complete(
        "convert",
        &[pack_f32(1.75, -2.25), pack_f32(3.0, -4.75), Word::uint32(2)],
    );
    assert_eq!(value, vec![pack_i16(1, -3, 3, -5), Word::int16(3)]);
}

/// Execute vector memory through the observed loop and stop after the write.
#[test]
fn test_watch_vector_memory() {
    let site = TestMachine::memory(0, 0, MemoryAccess::Write, Space::Local);
    let watch = TestMachine::watchpoint(0, 0, 17, MemoryAccess::Write);
    let watchpoint_id = watch.watchpoint_id;
    let watches = WatchSet::new(vec![watch]);
    let mut machine = TestMachine::parse(
        r#"
type State

export function copy(
    r0: pointer,
    r1: vector<uint32, 4>,
): vector<uint32, 4> {
    slot s0: State = r0[1]

    store r0, r1
    r3: vector<uint32, 4> = load r0
    return r3
}
"#,
        TestProgram::new().memory([site]),
    );
    let mut storage = [0_u32; 4];
    let pointer = Word::from_bits(storage.as_mut_ptr() as u64);
    let vector = [pack_u32(3, 5), pack_u32(7, 11)];

    // stop only after the vector bytes have been written
    let (continuation, reason) = machine.run_to_stop(
        "copy",
        &[pointer, vector[0], vector[1]],
        None,
        Some(&watches),
    );
    assert_eq!(storage, [3, 5, 7, 11]);
    assert_eq!(
        reason,
        StopReason::Watchpoint {
            watchpoint_id,
            point: TestMachine::point(0, 0),
        }
    );

    // continue after the retained store and read the same packed bytes
    let value = machine.continue_to_completion(continuation, None, Some(&watches), None);
    assert_eq!(value, vector);
}

/// Pack two 32-bit signed lanes into one register word.
const fn pack_i32(low: i32, high: i32) -> Word {
    let bits = low as u32 as u64 | ((high as u32 as u64) << u32::BITS);

    Word::from_bits(bits)
}

/// Pack four 16-bit signed lanes into one register word.
const fn pack_i16(first: i16, second: i16, third: i16, fourth: i16) -> Word {
    let bits = first as u16 as u64
        | ((second as u16 as u64) << u16::BITS)
        | ((third as u16 as u64) << (u16::BITS * 2))
        | ((fourth as u16 as u64) << (u16::BITS * 3));

    Word::from_bits(bits)
}

/// Pack two 32-bit unsigned lanes into one register word.
const fn pack_u32(low: u32, high: u32) -> Word {
    Word::from_bits(low as u64 | ((high as u64) << u32::BITS))
}

/// Pack two 32-bit floating-point lanes into one register word.
fn pack_f32(low: f32, high: f32) -> Word {
    pack_u32(low.to_bits(), high.to_bits())
}
