use destack_program::Word;

use crate::{ErrorReason, Trap};

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

/// Execute midpoint and clamp with their floating-point edge behavior.
#[test]
fn test_execute_float_midpoint_and_clamp() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    float.midpoint r6, r0, r1: float64
    float.clamp r7, r2, r3, r4: float64
    float.midpoint r8, r1, r5: float64
    return r6:r8
}
"#,
        TestProgram::words(),
    );

    let value = machine.complete(
        0,
        &[
            Word::float64(f64::MAX),
            Word::float64(f64::MAX),
            Word::float64(20.0),
            Word::float64(-5.0),
            Word::float64(10.0),
            Word::float64(-f64::MAX),
        ],
    );
    assert_eq!(
        value,
        vec![
            Word::float64(f64::MAX),
            Word::float64(10.0),
            Word::float64(0.0),
        ]
    );
}

/// Classify finite, infinite, and NaN values.
#[test]
fn test_execute_float_predicates() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    float.isFinite r4, r0: float64
    float.isFinite r5, r1: float64
    float.isFinite r6, r2: float64
    float.isInfinite r7, r1: float64
    float.isInfinite r8, r3: float64
    return r4:r8
}

"#,
        TestProgram::words(),
    );

    let value = machine.complete(
        0,
        &[
            Word::float64(1.0),
            Word::float64(f64::INFINITY),
            Word::float64(f64::NAN),
            Word::float64(f64::NEG_INFINITY),
        ],
    );
    assert_eq!(
        value,
        vec![
            Word::boolean(true),
            Word::boolean(false),
            Word::boolean(false),
            Word::boolean(true),
            Word::boolean(true),
        ]
    );
}

/// Execute the ECMAScript, ties-to-even, and ties-away rounding rules.
#[test]
fn test_execute_float_rounding() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    float.round r6, r0: float64
    float.round r7, r1: float64
    float.roundTiesEven r8, r2: float64
    float.roundTiesEven r9, r3: float64
    float.roundTiesAway r10, r4: float64
    float.roundTiesAway r11, r5: float64
    return r6:r11
}
"#,
        TestProgram::words(),
    );

    let value = machine.complete(
        0,
        &[
            Word::float64(-1.5),
            Word::float64(-0.5),
            Word::float64(2.5),
            Word::float64(3.5),
            Word::float64(-1.5),
            Word::float64(1.5),
        ],
    );
    assert_eq!(
        value,
        vec![
            Word::float64(-1.0),
            Word::float64(-0.0),
            Word::float64(2.0),
            Word::float64(4.0),
            Word::float64(-2.0),
            Word::float64(2.0),
        ]
    );
}

/// Propagate NaN and select the required signed zero for minimum and maximum.
#[test]
fn test_execute_float_extrema() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    float.min r4, r0, r1: float64
    float.max r5, r0, r1: float64
    float.min r6, r2, r0: float64
    float.max r7, r3, r0: float64
    return r4:r7
}
"#,
        TestProgram::words(),
    );

    let value = machine.complete(
        0,
        &[
            Word::float64(0.0),
            Word::float64(-0.0),
            Word::float64(f64::NAN),
            Word::float64(f64::NAN),
        ],
    );
    assert_eq!(
        value,
        vec![
            Word::float64(-0.0),
            Word::float64(0.0),
            Word::float64(f64::NAN),
            Word::float64(f64::NAN),
        ]
    );
}

/// Trap when floating-point clamp bounds are inverted.
#[test]
fn test_trap_inverted_float_clamp() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    float.clamp r3, r0, r1, r2: float64
    return r3
}
"#,
        TestProgram::words(),
    );

    let error = machine
        .run(
            0,
            &[Word::float64(0.0), Word::float64(1.0), Word::float64(-1.0)],
            None,
            None,
            None,
        )
        .expect_err("inverted clamp bounds should trap");
    assert_eq!(error.reason(), &ErrorReason::Trap(Trap::InvalidArithmetic));
}
