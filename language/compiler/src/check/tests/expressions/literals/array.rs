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

const first = values[0];
/// @type.symbol symbol=first type=1
/// @type.node source=values type=readonly [1, 2]
/// @type.node source=values[0] type=1
/// @resolution.name source=values target=values
/// @type.node source=0 type=0
"#,
    );
}
