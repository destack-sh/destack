use crate::tests::{DirRows, TestSession};

#[test]
fn test_let_array_widens_element_literals() {
    let session = TestSession::single(
        r#"
let values = [1, 2];
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

/// @check.stats.solve variables=0 terms=6 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_const_array_widens_without_const_assertion() {
    let session = TestSession::single(
        r#"
const values = [1, 2];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
const values = [1, 2];
/// @type.symbol symbol=values type=Array<int32>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=0 terms=6 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_const_asserted_array_becomes_readonly_tuple() {
    let session = TestSession::single(
        r#"
const values = [1, 2] as const;
const first = values[0];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
const values = [1, 2] as const;
/// @type.symbol symbol=values type=readonly [1, 2]
/// @type.node source="[1, 2] as const" type=readonly [1, 2]
/// @type.node source=[1, 2] type=readonly [1, 2]
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first type=1
/// @type.node source=values type=readonly [1, 2]
/// @type.node source=values[0] type=1
/// @resolution.name source=values target=values
/// @resolution.member source=values[0] receiver=readonly [1, 2] kind=builtin builtin=subscript.index
/// @type.node source=0 type=0

/// @check.stats.solve variables=0 terms=10 constraints=0 obligations=0 solutions=0 bounds=0 decisions=2
"#,
    );
}

#[test]
fn test_empty_array_has_never_element_type() {
    let session = TestSession::single(
        r#"
const values = [];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
const values = [];
/// @type.symbol symbol=values type=Array<never>
/// @type.node source=[] type=Array<never>

/// @check.stats.solve variables=0 terms=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_contextual_empty_array_uses_element_type() {
    let session = TestSession::single(
        r#"
const values: int32[] = [];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
const values: int32[] = [];
/// @type.symbol symbol=values type=Array<int32>
/// @type.node source=[] type=Array<never>

/// @check.stats.solve variables=0 terms=5 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}
#[test]
fn test_mixed_array_infers_union_element_type() {
    let session = TestSession::single(
        r#"
let values = [1, "two", true];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
let values = [1, "two", true];
/// @type.symbol symbol=values type=Array<int32 | string | boolean>
/// @type.node source=[1, "two", true] type=Array<1 | "two" | true>
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"
/// @type.node source=true type=true

/// @check.stats.solve variables=0 terms=10 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}
#[test]
fn test_contextual_array_literal_uses_element_type() {
    let session = TestSession::single(
        r#"
const values: number[] = [1, 2, 3];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
const values: number[] = [1, 2, 3];
/// @type.symbol symbol=values type=Array<float64>
/// @type.node source=[1, 2, 3] type=Array<1 | 2 | 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

/// @check.stats.solve variables=0 terms=8 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_contextual_array_literal_rejects_element_mismatch() {
    let session = TestSession::single(
        r#"
const values: number[] = [1, "two"];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
const values: number[] = [1, "two"];
/// @type.symbol symbol=values type=Array<float64>
/// @type.node source=[1, "two"] type=Array<1 | "two">
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"

/// @check.stats.solve variables=0 terms=7 constraints=1 obligations=0 solutions=0 bounds=0 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=26 source="const values: number[] = [1, \"two\"];"
"#,
    );
}
