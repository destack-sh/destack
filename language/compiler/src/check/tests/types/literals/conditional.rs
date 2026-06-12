use crate::tests::{DirRows, TestSession};

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
