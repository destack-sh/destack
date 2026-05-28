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
        DirRows::checked().with_reference_types(),
        r#"
let values = [1, 2];
/// @type.symbol symbol=values type=collections.array.Array<int32>
/// @type.node source=[1, 2] type=collections.array.Array<int32>
/// @type.node source=1 type=int32
/// @type.node source=2 type=int32
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
        DirRows::checked().with_reference_types(),
        r#"
const values = [1, 2];
/// @type.symbol symbol=values type=collections.array.Array<int32>
/// @type.node source=[1, 2] type=collections.array.Array<int32>
/// @type.node source=1 type=int32
/// @type.node source=2 type=int32
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
        DirRows::checked().with_reference_types(),
        r#"
const values = [1, 2] as const;
/// @type.symbol symbol=values type=readonly [1, 2]
/// @type.node source="[1, 2] as const" type=readonly [1, 2]
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first type=1
/// @type.node source=values type=readonly [1, 2]
/// @type.node source=values[0] type=1
/// @resolution.name source=values target=values
/// @type.node source=0 type=0
"#,
    );
}

#[test]
fn test_empty_array_infers_unknown_element_type() {
    let session = TestSession::single(
        r#"
const values = [];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const values = [];
/// @type.symbol symbol=values type=collections.array.Array<unknown>
/// @type.node source=[] type=collections.array.Array<unknown>
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
        DirRows::checked().with_reference_types(),
        r#"
let values = [1, "two", true];
/// @type.symbol symbol=values type=collections.array.Array<int32 | string | boolean>
/// @type.node source="[1, \"two\", true]" type=collections.array.Array<int32 | string | boolean>
/// @type.node source=1 type=int32 | string | boolean
/// @type.node source="\"two\"" type=int32 | string | boolean
/// @type.node source=true type=int32 | string | boolean
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
        DirRows::checked().with_reference_types(),
        r#"
const values: number[] = [1, 2, 3];
/// @type.symbol symbol=values type=collections.array.Array<float64>
/// @type.node source=[1, 2, 3] type=collections.array.Array<float64>
/// @type.node source=1 type=float64
/// @type.node source=2 type=float64
/// @type.node source=3 type=float64
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
        DirRows::checked().with_reference_types(),
        r#"
const values: number[] = [1, "two"];
/// @type.symbol symbol=values type=collections.array.Array<float64>
/// @type.node source="[1, \"two\"]" type=collections.array.Array<float64>
/// @type.node source=1 type=float64
/// @type.node source="\"two\"" type="two"

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=31 source="const values: number[] = [1, \"two\"];"
"#,
    );
}
