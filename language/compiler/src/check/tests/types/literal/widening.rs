use crate::tests::{DirRows, TestSession};

#[test]
fn test_let_array_widens_element_literals() {
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
/// @type.node source=[1, 2] type=Array<float64>
/// @type.node source=1 type=float64
/// @type.node source=2 type=float64

const first = values[0];
/// @type.symbol symbol=first source=first type=float64
/// @type.node source=values type=Array<float64>
/// @type.node source=values[0] type=float64
/// @resolution.name source=values target=values
/// @resolution.call source=values[0] parameters=(usize) return=float64 kind=symbol target=collections.array.index#8 receiver=Array<float64>
/// @type.node source=0 type=usize

/// @check.stats.solve variables=8 types=39 constraints=7 obligations=0 solutions=8 bounds=10 decisions=2
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
/// @resolution.call source=values[0] parameters=(usize) return=1 | 2 kind=symbol target=collections.array.index#8 receiver=Array<1 | 2>
/// @type.node source=0 type=usize

/// @check.stats.solve variables=7 types=38 constraints=7 obligations=0 solutions=7 bounds=11 decisions=2
"#,
    );
}

#[test]
fn test_const_conditional_preserves_literal_union() {
    let session = TestSession::single(
        r#"
const value = true ? 1 : 2;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: 1 | 2 = true ? 1 : 2;

=== checked ===
const value = true ? 1 : 2;
/// @type.symbol symbol=value source=value type=1 | 2
/// @type.node source="true ? 1 : 2" type=1 | 2
/// @type.node source=true type=true
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 types=6 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_let_conditional_widens_literal_union() {
    let session = TestSession::single(
        r#"
let value = true ? 1 : 2;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
let value: float64 = true ? 1 : 2;

=== checked ===
let value = true ? 1 : 2;
/// @type.symbol symbol=value source=value type=float64
/// @type.node source="true ? 1 : 2" type=1 | 2
/// @type.node source=true type=true
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 types=8 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}
