use destack_mir::Space;
use destack_program::{MemoryAccess, StopReason, WatchSet, Word};

use super::{TestMachine, TestProgram};

/// Execute packed vector arithmetic, masks, projections, and conversions.
#[test]
fn test_execute_vectors() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    vector.add.int32x4 r6:r7, r0:r1, r2:r3
    vector.compare.gt.int32x4 r8, r6:r7, r0:r1
    vector.select.int32x4 r9:r10, r8, r6:r7, r0:r1
    vector.insert.int32x4 r11:r12, r9:r10, r4, r5
    vector.shuffle.int32x4 r13:r14, r11:r12, r2:r3, [0, 5, 2, 7]
    vector.reduce.add.int32x4 r15, r9:r10
    vector.extract.int32x4 r16, r9:r10, r4
    return r13:r16
}

function f1 {
    vector.convert.roundFloor.float32x4.int16x4 r3, r0:r1
    vector.extract.int16x4 r4, r3, r2
    return r3:r4
}
"#,
        TestProgram::words(),
    );

    // execute one multiword integer vector pipeline
    let value = machine.complete(
        0,
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
        1,
        &[pack_f32(1.75, -2.25), pack_f32(3.0, -4.75), Word::uint32(2)],
    );
    assert_eq!(value, vec![pack_i16(1, -3, 3, -5), Word::int16(3)]);
}

/// Execute vector memory through the observed loop and stop after the write.
#[test]
fn test_watch_vector_memory() {
    let site = TestProgram::memory_site(0, 0, MemoryAccess::Write, Space::Local);
    let watch = TestProgram::watchpoint(0, 0, 17, MemoryAccess::Write);
    let watchpoint_id = watch.watchpoint_id;
    let watches = WatchSet::new(vec![watch]);
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    vector.store.uint32x4 r0, r1:r2
    vector.load.uint32x4 r3:r4, r0
    return r3:r4
}
"#,
        TestProgram::words().memory([site]),
    );
    let mut storage = [0_u32; 4];
    let pointer = Word::from_bits(storage.as_mut_ptr() as u64);
    let vector = [pack_u32(3, 5), pack_u32(7, 11)];

    // stop only after the vector bytes have been written
    let reason = machine.run_to_stop(0, &[pointer, vector[0], vector[1]], None, Some(&watches));
    assert_eq!(storage, [3, 5, 7, 11]);
    assert_eq!(
        reason,
        StopReason::Watchpoint {
            watchpoint_id,
            point: TestProgram::point(0, 0),
        }
    );

    // continue after the retained store and read the same packed bytes
    let value = machine.continue_to_completion(None, Some(&watches), None);
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
