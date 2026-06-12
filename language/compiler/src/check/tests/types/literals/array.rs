use crate::tests::{DirRows, TestSession};

#[test]
fn test_array_literal_widens_element_literals() {
    let session = TestSession::single(
        r#"
let values = [1, 2];
const first = values[0];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
let values: float64[] = [1, 2];
const first: float64 = values[0];

=== checked ===
let values = [1, 2];
/// @type.symbol symbol=values source=values type=Array<float64>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first source=first type=float64
/// @type.node source=values type=Array<float64>
/// @type.node source=values[0] type=float64
/// @resolution.name source=values target=values
/// @resolution.call source=values[0] parameters=() return=float64 kind=expression
/// @type.node source=0 type=0

/// @check.stats.solve variables=4 types=14 constraints=7 obligations=0 solutions=4 bounds=5 decisions=2
"#,
    );
}

#[test]
fn test_contextual_array_preserves_union_element_type() {
    let session = TestSession::single(
        r#"
const values: (1 | 2)[] = [1, 2];
const first = values[0];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const values: (1 | 2)[] = [1, 2];
const first: 1 | 2 = values[0];

=== checked ===
const values: (1 | 2)[] = [1, 2];
/// @type.symbol symbol=values source=values type=Array<1 | 2>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first source=first type=1 | 2
/// @type.node source=values type=Array<1 | 2>
/// @type.node source=values[0] type=1 | 2
/// @resolution.name source=values target=values
/// @resolution.call source=values[0] parameters=() return=1 | 2 kind=expression
/// @type.node source=0 type=0

/// @check.stats.solve variables=3 types=13 constraints=7 obligations=0 solutions=3 bounds=6 decisions=2
"#,
    );
}
