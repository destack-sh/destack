use destack_program::Word;

use super::{TestMachine, TestProgram};

/// Execute floating-point arithmetic and scalar representation conversions.
#[test]
fn test_execute_float_casts() {
    let mut machine = TestMachine::parse(
        r#"
export function convert(
    r0: float32,
    r1: float32,
    r2: int64,
): (float32, int8, float64) {
    r3: float32 = float.add r0, r1
    r4: int8 = cast.floatToInt.s r3 -> int8
    r5: float64 = cast.intToFloat.s r2 -> float64
    return r3, r4, r5
}
"#,
        TestProgram::new(),
    );

    let value = machine.complete(
        "convert",
        &[Word::float32(1.25), Word::float32(2.5), Word::int64(-9)],
    );

    assert_eq!(
        value,
        vec![Word::float32(3.75), Word::int8(3), Word::float64(-9.0)]
    );
}
