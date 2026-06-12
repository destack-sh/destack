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
=== annotated ===
let values: float64[] = [1, 2];

=== checked ===
let values = [1, 2];
/// @type.symbol symbol=values source=values type=Array<float64>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=2 types=10 constraints=5 obligations=0 solutions=2 bounds=3 decisions=0
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
=== annotated ===
const values: float64[] = [1, 2];

=== checked ===
const values = [1, 2];
/// @type.symbol symbol=values source=values type=Array<float64>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @check.stats.solve variables=2 types=10 constraints=5 obligations=0 solutions=2 bounds=3 decisions=0
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
=== annotated ===
const values: readonly [1, 2] = [1, 2] as const;
const first: 1 = values[0];

=== checked ===
const values = [1, 2] as const;
/// @type.symbol symbol=values source=values type=Readonly<[1, 2]>
/// @type.node source="[1, 2] as const" type=Readonly<[1, 2]>
/// @type.node source=[1, 2] type=Array<1 | 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first source=first type=1
/// @type.node source=values type=Readonly<[1, 2]>
/// @type.node source=values[0] type=1
/// @resolution.name source=values target=values
/// @resolution.member source=values[0] receiver=[1, 2] kind=builtin builtin=subscript.index
/// @type.node source=0 type=0

/// @check.stats.solve variables=3 types=12 constraints=6 obligations=0 solutions=3 bounds=4 decisions=2
"#,
    );
}

#[test]
fn test_empty_array_requires_element_context() {
    let session = TestSession::single(
        r#"
const values = [];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const values = [];

=== checked ===
const values = [];
/// @type.symbol symbol=values source=values type=<error>
/// @type.node source=[] type=Array<<error>>

/// @check.stats.solve variables=2 types=4 constraints=1 obligations=0 solutions=0 bounds=1 decisions=0
"#,
        r#"
/// @diagnostic.error code=EC101 message="missing type annotation"
/// @diagnostic.label line=2 column=16 source="const values = [];"
/// @diagnostic.error code=EC101 message="missing type annotation"
/// @diagnostic.label line=2 column=7 source="const values = [];"
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
=== annotated ===
const values: int32[] = [];

=== checked ===
const values: int32[] = [];
/// @type.symbol symbol=values source=values type=Array<int32>
/// @type.node source=[] type=Array<int32>

/// @check.stats.solve variables=1 types=5 constraints=2 obligations=0 solutions=1 bounds=2 decisions=0
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
=== annotated ===
let values: (float64 | string | boolean)[] = [1, "two", true];

=== checked ===
let values = [1, "two", true];
/// @type.symbol symbol=values source=values type=Array<float64 | string | boolean>
/// @type.node source=[1, "two", true] type=Array<1 | "two" | true>
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"
/// @type.node source=true type=true

/// @check.stats.solve variables=2 types=13 constraints=5 obligations=0 solutions=2 bounds=4 decisions=0
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
=== annotated ===
const values: number[] = [1 as float64, 2 as float64, 3 as float64];

=== checked ===
const values: number[] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=Array<float64>
/// @type.node source=[1, 2, 3] type=Array<float64>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

/// @check.stats.solve variables=1 types=8 constraints=8 obligations=0 solutions=1 bounds=5 decisions=0
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
=== annotated ===
const values: number[] = [1, "two"];

=== checked ===
const values: number[] = [1, "two"];
/// @type.symbol symbol=values source=values type=Array<float64>
/// @type.node source=[1, "two"] type=Array<"two" | float64>
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"

/// @check.stats.solve variables=1 types=8 constraints=5 obligations=0 solutions=1 bounds=4 decisions=0

"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"two\" | float64' is not assignable to type 'float64'"
/// @diagnostic.label line=2 column=26 source="const values: number[] = [1, \"two\"];"
"#,
    );
}
