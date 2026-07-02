use crate::tests::{DirRows, TestSession};

#[test]
fn test_shift_keeps_left_operand_type() {
    let session = TestSession::single(
        r#"
declare const flags: int32;
const shifted = flags << 5;
const literal = 1 << 5;
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
declare const flags: int32;
const shifted: int32 = flags << 5;
const literal: int = 1 << 5;

=== checked ===
declare const flags: int32;
/// @type.symbol symbol=flags source=flags type=int32

const shifted = flags << 5;
/// @type.symbol symbol=shifted source=shifted type=int32
/// @resolution.name source=flags target=flags
/// @resolution.call source="flags << 5" parameters=() return=int32 kind=builtin builtin=binary.shift_left

const literal = 1 << 5;
/// @type.symbol symbol=literal source=literal type=int
/// @resolution.call source="1 << 5" parameters=() return=int kind=builtin builtin=binary.shift_left
"#);
}

#[test]
fn test_bitwise_and_joins_integer_operands() {
    let session = TestSession::single(
        r#"
declare const mask: int32;
declare const bits: int32;
const masked = mask & bits;
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
declare const mask: int32;
declare const bits: int32;
const masked: int32 = mask & bits;

=== checked ===
declare const mask: int32;
/// @type.symbol symbol=mask source=mask type=int32

declare const bits: int32;
/// @type.symbol symbol=bits source=bits type=int32

const masked = mask & bits;
/// @type.symbol symbol=masked source=masked type=int32
/// @resolution.name source=mask target=mask
/// @resolution.call source="mask & bits" parameters=() return=int32 kind=builtin builtin=binary.elementwise_and
/// @resolution.name source=bits target=bits
"#);
}

#[test]
fn test_bitwise_rejects_float_operands() {
    let session = TestSession::single(
        r#"
declare const scale: float64;
const bad = scale & 2;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const scale: float64;
const bad = scale & 2;

=== checked ===
declare const scale: float64;
/// @type.symbol symbol=scale source=scale type=float64

const bad = scale & 2;
/// @type.symbol symbol=bad source=bad type=<error>
/// @resolution.name source=scale target=scale
"#,
        r#"
/// @diagnostic.error code=EC306 message="operator '&' is not defined for 'float64' and '2'"
/// @diagnostic.label line=3 column=19 span="&" line_source="const bad = scale & 2;"
"#,
    );
}
