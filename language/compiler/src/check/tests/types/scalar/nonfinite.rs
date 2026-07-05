use crate::tests::{DirRows, TestSession};

#[test]
fn test_nonfinite_folds_are_representable_in_every_float_width() {
    let session = TestSession::single(
        r#"
const infinity: float64 = 1.0 / 0.0;
const negative: float64 = -1.0 / 0.0;
const nan: float64 = 0.0 / 0.0;
const half: float32 = 1.0 / 0.0;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const infinity: float64 = 1.0 / 0.0;
const negative: float64 = -1.0 / 0.0;
const nan: float64 = 0.0 / 0.0;
const half: float32 = 1.0 / 0.0;

=== checked ===
const infinity: float64 = 1.0 / 0.0;
/// @type.symbol symbol=infinity source=infinity type=float64
/// @type.node source="1.0 / 0.0" type=inf
/// @type.node source=1.0 type=1
/// @resolution.call source="1.0 / 0.0" parameters=() return=inf kind=builtin builtin=binary.divide
/// @type.node source=0.0 type=0

const negative: float64 = -1.0 / 0.0;
/// @type.symbol symbol=negative source=negative type=float64
/// @type.node source="-1.0 / 0.0" type=-inf
/// @type.node source=-1.0 type=-1
/// @resolution.call source="-1.0 / 0.0" parameters=() return=-inf kind=builtin builtin=binary.divide
/// @resolution.call source=-1.0 parameters=() return=-1 kind=builtin builtin=unary.negate
/// @type.node source=1.0 type=1
/// @type.node source=0.0 type=0

const nan: float64 = 0.0 / 0.0;
/// @type.symbol symbol=nan source=nan type=float64
/// @type.node source="0.0 / 0.0" type=NaN
/// @type.node source=0.0 type=0
/// @resolution.call source="0.0 / 0.0" parameters=() return=NaN kind=builtin builtin=binary.divide
/// @type.node source=0.0 type=0

const half: float32 = 1.0 / 0.0;
/// @type.symbol symbol=half source=half type=float32
/// @type.node source="1.0 / 0.0" type=inf
/// @type.node source=1.0 type=1
/// @resolution.call source="1.0 / 0.0" parameters=() return=inf kind=builtin builtin=binary.divide
/// @type.node source=0.0 type=0
"#,
        r#""#,
    );
}
