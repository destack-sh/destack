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
/// @resolution.pattern source=infinity kind=binding target=infinity
/// @type.node source="1.0 / 0.0" type=inf
/// @type.node source=1.0 type=1
/// @resolution.operator source="1.0 / 0.0" kind=builtin
/// @type.node source=0.0 type=0

const negative: float64 = -1.0 / 0.0;
/// @type.symbol symbol=negative source=negative type=float64
/// @resolution.pattern source=negative kind=binding target=negative
/// @type.node source="-1.0 / 0.0" type=-inf
/// @type.node source=-1.0 type=-1
/// @resolution.operator source="-1.0 / 0.0" kind=builtin
/// @resolution.operator source=-1.0 kind=builtin
/// @type.node source=1.0 type=1
/// @type.node source=0.0 type=0

const nan: float64 = 0.0 / 0.0;
/// @type.symbol symbol=nan source=nan type=float64
/// @resolution.pattern source=nan kind=binding target=nan
/// @type.node source="0.0 / 0.0" type=NaN
/// @type.node source=0.0 type=0
/// @resolution.operator source="0.0 / 0.0" kind=builtin
/// @type.node source=0.0 type=0

const half: float32 = 1.0 / 0.0;
/// @type.symbol symbol=half source=half type=float32
/// @resolution.pattern source=half kind=binding target=half
/// @type.node source="1.0 / 0.0" type=inf
/// @type.node source=1.0 type=1
/// @resolution.operator source="1.0 / 0.0" kind=builtin
/// @type.node source=0.0 type=0
"#,
        r#""#,
    );
}
