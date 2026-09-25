use tspp_program::Word;

use crate::{ErrorReason, Trap};

use super::{TestMachine, TestProgram};

/// Execute floating-point arithmetic and scalar representation conversions.
#[test]
fn test_execute_float_casts() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    add.float32 r3, r0, r1
    truncate.float32.int8 r4, r3
    convert.int64.float64 r5, r2
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

/// Execute cube root and cancellation-safe exponential and logarithm operations.
#[test]
fn test_execute_precise_float_operations() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    cbrt.float64 r3, r0
    expm1.float64 r4, r1
    log1p.float64 r5, r2
    return r3:r5
}
"#,
        TestProgram::words(),
    );
    let offset = 1e-10_f64;

    let value = machine.complete(
        0,
        &[
            Word::float64(27.0),
            Word::float64(offset),
            Word::float64(offset),
        ],
    );
    assert_eq!(
        value,
        vec![
            Word::float64(3.0),
            Word::float64(offset.exp_m1()),
            Word::float64(offset.ln_1p()),
        ]
    );
}

/// Execute midpoint and clamp with their floating-point edge behavior.
#[test]
fn test_execute_float_midpoint_and_clamp() {
    let mut machine = TestMachine::parse(
        r#"
function f0 {
    midpoint.float64 r6, r0, r1
    clamp.float64 r7, r2, r3, r4
    midpoint.float64 r8, r1, r5
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
    isFinite.float64 r4, r0
    isFinite.float64 r5, r1
    isFinite.float64 r6, r2
    isInfinite.float64 r7, r1
    isInfinite.float64 r8, r3
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
    round.float64 r6, r0
    round.float64 r7, r1
    roundTiesEven.float64 r8, r2
    roundTiesEven.float64 r9, r3
    roundTiesAway.float64 r10, r4
    roundTiesAway.float64 r11, r5
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
    min.float64 r4, r0, r1
    max.float64 r5, r0, r1
    min.float64 r6, r2, r0
    max.float64 r7, r3, r0
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
    clamp.float64 r3, r0, r1, r2
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
