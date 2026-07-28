use destack_program::Word;

use super::{TestMachine, TestProgram};

/// Execute exact-width overflow, saturation, bit counts, and byte reversal.
#[test]
fn test_execute_integer_widths() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    int.add.overflowing.int8 r3, r4, r0, r1
    int.add.saturating.int8 r5, r0, r1
    int.countLeadingZeros.int8 r6, r0
    int.byteSwap.uint16 r7, r2
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
    int.add.overflowing.int128 r7:r8, r9, r0:r1, r2:r3
    int.rotateLeft.uint128 r10:r11, r4:r5, r6
    int.countOnes.uint128 r12, r10:r11
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
