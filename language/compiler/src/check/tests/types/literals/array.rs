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
let values = [1, 2];
/// @type.symbol symbol=values type=Array<int32>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first type=int32
/// @type.node source=values type=Array<int32>
/// @type.node source=values[0] type=int32
/// @resolution.name source=values target=values
/// @resolution.member source=values[0] receiver=Array<int32> kind=builtin builtin=subscript.index
/// @type.node source=0 type=0

/// @check.stats.solve variables=0 terms=10 constraints=0 obligations=0 solutions=0 bounds=0 decisions=2
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
const values: (1 | 2)[] = [1, 2];
/// @type.symbol symbol=values type=Array<1 | 2>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first type=1 | 2
/// @type.node source=values type=Array<1 | 2>
/// @type.node source=values[0] type=1 | 2
/// @resolution.name source=values target=values
/// @resolution.member source=values[0] receiver=Array<1 | 2> kind=builtin builtin=subscript.index
/// @type.node source=0 type=0

/// @check.stats.solve variables=0 terms=13 constraints=1 obligations=0 solutions=0 bounds=0 decisions=2
"#,
    );
}
