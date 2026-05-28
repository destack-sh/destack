use crate::tests::{DirRows, TestSession};

#[test]
fn test_builtin_string_type_functions_transform_literals() {
    let session = TestSession::single(
        r#"
type Upper = Uppercase<"hello">;
type Lower = Lowercase<"HELLO">;
type Title = Capitalize<"hello">;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Upper = Uppercase<"hello">;
/// @type.symbol symbol=Upper type="HELLO"

type Lower = Lowercase<"HELLO">;
/// @type.symbol symbol=Lower type="hello"

type Title = Capitalize<"hello">;
/// @type.symbol symbol=Title type="Hello"
"#,
    );
}
