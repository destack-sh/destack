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
/// @type.node source=1 type=1
/// @type.node source="\"two\"" type="two"
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_const_tuple_widens_without_const_assertion() {
    let session = TestSession::single(
        r#"
const value = (1, "two", true);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = (1, "two", true);
/// @type.symbol symbol=value type=(int32, string, boolean)
/// @type.node source="(1, \"two\", true)" type=(int32, string, boolean)
/// @type.node source=1 type=int32
/// @type.node source="\"two\"" type=string
/// @type.node source=true type=boolean
"#,
    );
}

#[test]
fn test_nested_tuple_preserves_nested_shape() {
    let session = TestSession::single(
        r#"
const value = (1, (2, 3));
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value = (1, (2, 3));
/// @type.symbol symbol=value type=(int32, (int32, int32))
/// @type.node source="(1, (2, 3))" type=(int32, (int32, int32))
/// @type.node source=1 type=int32
/// @type.node source="(2, 3)" type=(int32, int32)
/// @type.node source=2 type=int32
/// @type.node source=3 type=int32
"#,
    );
}

#[test]
fn test_contextual_tuple_literal_preserves_union_elements() {
    let session = TestSession::single(
        r#"
const value: (1 | 2, "a" | "b") = (1, "a");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: (1 | 2, "a" | "b") = (1, "a");
/// @type.symbol symbol=value type=(1 | 2, "a" | "b")
/// @type.node source="(1, \"a\")" type=(1 | 2, "a" | "b")
/// @type.node source=1 type=1 | 2
/// @type.node source="\"a\"" type="a" | "b"
"#,
    );
}

#[test]
fn test_contextual_tuple_literal_rejects_element_mismatch() {
    let session = TestSession::single(
        r#"
const value: (number, string) = (1, 2);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
const value: (number, string) = (1, 2);
/// @type.symbol symbol=value type=(float64, string)
/// @type.node source="(1, 2)" type=(float64, 2)
/// @type.node source=1 type=float64
/// @type.node source=2 type=2

"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=2 column=33 source="const value: (number, string) = (1, 2);"
"#,
    );
}
