use destack_program::Word;

use super::{TestMachine, TestProgram};

/// Execute floating-point arithmetic and scalar representation conversions.
#[test]
fn test_execute_float_casts() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    float.add r3, r0, r1: float32
    cast.floatToInt.s r4, r3: float32 -> int8
    cast.intToFloat.s r5, r2: int64 -> float64
    return r3:r5
}
"#,
        TestProgram::words(),
    );

    let value = machine.complete(
        0,
        &[Word::float32(1.25), Word::float32(2.5), Word::int64(-9)],
    );

    assert_eq!(
        value,
        vec![Word::float32(3.75), Word::int8(3), Word::float64(-9.0)]
    );
}
