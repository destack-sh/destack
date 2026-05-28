use crate::tests::{DirRows, TestSession};

#[test]
fn test_let_tuple_widens_element_literals() {
    let session = TestSession::single(
        r#"
let value = (1, "two", true);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
let value = (1, "two", true);
/// @type.symbol symbol=value type=(int32, string, boolean)
/// @type.node source="(1, \"two\", true)" type=(int32, string, boolean)
/// @type.node source=1 type=int32
/// @type.node source="\"two\"" type=string
/// @type.node source=true type=boolean
"#,
    );
}

#[test]
fn test_const_asserted_tuple_preserves_literal_elements() {
    let session = TestSession::single(
        r#"
const value = (1, "two", true) as const;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = (1, "two", true) as const;
/// @type.symbol symbol=value type=readonly (1, "two", true)
/// @type.node source="(1, \"two\", true) as const" type=readonly (1, "two", true)
"#,
    );
}
