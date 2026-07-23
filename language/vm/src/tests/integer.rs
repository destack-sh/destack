use destack_program::Word;

use super::{TestMachine, TestProgram};

/// Execute exact-width overflow, saturation, bit counts, and byte reversal.
#[test]
fn test_execute_integer_widths() {
    let mut machine = TestMachine::parse(
        r#"
export function integers(
    r0: int8,
    r1: int8,
    r2: uint16,
): (int8, boolean, int8, uint32, uint16) {
    r3: int8, r4: boolean = int.add.overflowing r0, r1
    r5: int8 = int.add.saturating r0, r1
    r6: uint32 = int.countLeadingZeros r0
    r7: uint16 = int.byteSwap r2
    return r3, r4, r5, r6, r7
}

"#,
        TestProgram::new(),
    );

    let value = machine.complete(
        "integers",
        &[Word::int8(120), Word::int8(20), Word::uint16(0x1234)],
    );

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
export function integers(
    r0: int128,
    r2: int128,
    r4: uint128,
    r6: uint32,
): (int128, boolean, uint128, uint32) {
    r7: int128, r9: boolean = int.add.overflowing r0, r2
    r10: uint128 = int.rotateLeft r4, r6
    r12: uint32 = int.countOnes r10
    return r7, r9, r10, r12
}
"#,
        TestProgram::new(),
    );
    let maximum = i128::MAX as u128;
    let rotated = 0x8000_0000_0000_0000_0000_0000_0000_0001_u128;

    let value = machine.complete(
        "integers",
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
