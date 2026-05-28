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
        DirRows::checked().with_reference_types(),
        r#"
let values = [1, 2];
/// @type.symbol symbol=values type=collections.array.Array<int32>
/// @type.node source=[1, 2] type=collections.array.Array<int32>
/// @type.node source=1 type=int32
/// @type.node source=2 type=int32

const first = values[0];
/// @type.symbol symbol=first type=int32
/// @type.node source=values type=collections.array.Array<int32>
/// @type.node source=values[0] type=int32
/// @resolution.name source=values target=values
/// @type.node source=0 type=0
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
        DirRows::checked().with_reference_types(),
        r#"
const values: (1 | 2)[] = [1, 2];
/// @type.symbol symbol=values type=collections.array.Array<1 | 2>
/// @type.node source=[1, 2] type=collections.array.Array<1 | 2>
/// @type.node source=1 type=1 | 2
/// @type.node source=2 type=1 | 2

const first = values[0];
/// @type.symbol symbol=first type=1 | 2
/// @type.node source=values type=collections.array.Array<1 | 2>
/// @type.node source=values[0] type=1 | 2
/// @resolution.name source=values target=values
/// @type.node source=0 type=0
"#,
    );
}
