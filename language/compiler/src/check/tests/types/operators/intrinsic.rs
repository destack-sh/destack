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
=== annotated ===
type Upper = Uppercase<"hello">;
type Lower = Lowercase<"HELLO">;
type Title = Capitalize<"hello">;

=== checked ===
type Upper = Uppercase<"hello">;
/// @type.symbol symbol=Upper source="type Upper = Uppercase<\"hello\">" type="HELLO"
/// @definition.type symbol=Upper source="type Upper = Uppercase<\"hello\">" value="HELLO"
/// @resolution.name source=Uppercase target=types.string.Uppercase

type Lower = Lowercase<"HELLO">;
/// @type.symbol symbol=Lower source="type Lower = Lowercase<\"HELLO\">" type="hello"
/// @definition.type symbol=Lower source="type Lower = Lowercase<\"HELLO\">" value="hello"
/// @resolution.name source=Lowercase target=types.string.Lowercase

type Title = Capitalize<"hello">;
/// @type.symbol symbol=Title source="type Title = Capitalize<\"hello\">" type="Hello"
/// @definition.type symbol=Title source="type Title = Capitalize<\"hello\">" value="Hello"
/// @resolution.name source=Capitalize target=types.string.Capitalize
"#,
    );
}
