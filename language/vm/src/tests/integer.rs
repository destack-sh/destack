use tspp_program::Word;

use crate::{ErrorReason, Trap};

use super::{TestMachine, TestProgram};

/// Execute exact-width overflow, saturation, bit counts, and byte reversal.
#[test]
fn test_execute_integer_widths() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    add.overflowing.int8 r3, r4, r0, r1
    add.saturating.int8 r5, r0, r1
    countLeadingZeros.int8 r6, r0
    byteSwap.uint16 r7, r2
    return r3:r7
}

"#,
        TestProgram::words(),
    );

    let value = machine.complete(0, &[Word::int8(120), Word::int8(20), Word::uint16(0x1234)]);

    assert_eq!(
        value,
        vec![
            Word::int8(-116),
            Word::boolean(true),
            Word::int8(127),
            Word::uint32(1),
            Word::uint16(0x3412),
        ]
    );
}

/// Execute signed and unsigned 128-bit arithmetic directly across register pairs.
#[test]
fn test_execute_integer128() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    add.overflowing.int128 r7:r8, r9, r0:r1, r2:r3
    rotateLeft.uint128 r10:r11, r4:r5, r6
    countOnes.uint128 r12, r10:r11
    return r7:r12
}
"#,
        TestProgram::words(),
    );
    let maximum = i128::MAX as u128;
    let rotated = 0x8000_0000_0000_0000_0000_0000_0000_0001_u128;

    let value = machine.complete(
        0,
        &[
            Word::from_bits(maximum as u64),
            Word::from_bits((maximum >> u64::BITS) as u64),
            Word::from_bits(1),
            Word::from_bits(0),
            Word::from_bits(rotated as u64),
            Word::from_bits((rotated >> u64::BITS) as u64),
            Word::uint32(1),
        ],
    );

    assert_eq!(
        value,
        vec![
            Word::from_bits(0),
            Word::from_bits(0x8000_0000_0000_0000),
            Word::boolean(true),
            Word::from_bits(3),
            Word::from_bits(0),
            Word::uint32(2),
        ]
    );
}

/// Execute signed integer intrinsics and their edge cases.
#[test]
fn test_execute_integer_intrinsics() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    midpoint.int64 r8, r0, r1
    clamp.int64 r9, r2, r3, r4
    divideCeil.int64 r10, r5, r6
    remainderEuclidean.int64 r11, r5, r6
    isMultipleOf.int64 r12, r5, r7
    isolateLowestOne.int64 r13, r7
    return r8:r13
}

function f1 {
    midpoint.uint64 r2, r0, r1
    isMultipleOf.uint64 r3, r0, r1
    return r2:r3
}

function f2 {
    isMultipleOf.int64 r2, r0, r1
    return r2
}

function f3 {
    absDiff.int32 r2, r0, r1
    return r2
}
"#,
        TestProgram::words(),
    );

    let signed = machine.complete(
        0,
        &[
            Word::int64(i64::MAX),
            Word::int64(i64::MAX - 2),
            Word::int64(20),
            Word::int64(-5),
            Word::int64(10),
            Word::int64(-7),
            Word::int64(3),
            Word::int64(12),
        ],
    );
    assert_eq!(
        signed,
        vec![
            Word::int64(i64::MAX - 1),
            Word::int64(10),
            Word::int64(-2),
            Word::int64(2),
            Word::boolean(false),
            Word::int64(4),
        ]
    );

    let unsigned = machine.complete(1, &[Word::uint64(u64::MAX), Word::uint64(u64::MAX - 2)]);
    assert_eq!(
        unsigned,
        vec![Word::uint64(u64::MAX - 1), Word::boolean(false)]
    );

    let zero = machine.complete(1, &[Word::uint64(0), Word::uint64(0)]);
    assert_eq!(zero, vec![Word::uint64(0), Word::boolean(true)]);

    let signed_boundary = machine.complete(2, &[Word::int64(i64::MIN), Word::int64(-1)]);
    assert_eq!(signed_boundary, vec![Word::boolean(true)]);

    let full_difference = machine.complete(3, &[Word::int32(i32::MIN), Word::int32(i32::MAX)]);
    assert_eq!(full_difference, vec![Word::uint32(u32::MAX)]);
}

/// Execute 128-bit integer intrinsics across register spans.
#[test]
fn test_execute_integer128_intrinsics() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    midpoint.int128 r6:r7, r0:r1, r2:r3
    isolateLowestOne.uint128 r8:r9, r4:r5
    absDiff.int128 r10:r11, r0:r1, r2:r3
    return r6:r11
}
"#,
        TestProgram::words(),
    );
    let left = i128::MAX as u128;
    let right = (i128::MAX - 2) as u128;

    let value = machine.complete(
        0,
        &[
            Word::from_bits(left as u64),
            Word::from_bits((left >> u64::BITS) as u64),
            Word::from_bits(right as u64),
            Word::from_bits((right >> u64::BITS) as u64),
            Word::from_bits(12),
            Word::from_bits(0),
        ],
    );
    let midpoint = (i128::MAX - 1) as u128;

    assert_eq!(
        value,
        vec![
            Word::from_bits(midpoint as u64),
            Word::from_bits((midpoint >> u64::BITS) as u64),
            Word::from_bits(4),
            Word::from_bits(0),
            Word::from_bits(2),
            Word::from_bits(0),
        ]
    );
}

/// Trap on inverted clamp bounds and signed remainder overflow.
#[test]
fn test_trap_invalid_integer_operations() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    clamp.int32 r3, r0, r1, r2
    return r3
}

function f1 {
    rem.int8 r2, r0, r1
    return r2
}
"#,
        TestProgram::words(),
    );

    let clamp = machine
        .run(
            0,
            &[Word::int32(0), Word::int32(10), Word::int32(0)],
            None,
            None,
            None,
        )
        .expect_err("inverted clamp bounds should trap");
    assert_eq!(clamp.reason(), &ErrorReason::Trap(Trap::InvalidArithmetic));

    let remainder = machine
        .run(1, &[Word::int8(i8::MIN), Word::int8(-1)], None, None, None)
        .expect_err("signed remainder overflow should trap");
    assert_eq!(
        remainder.reason(),
        &ErrorReason::Trap(Trap::IntegerOverflow)
    );
}
