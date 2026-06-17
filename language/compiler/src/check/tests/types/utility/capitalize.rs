use crate::tests::{DirRows, TestSession};

#[test]
fn test_capitalize_converts_first_character() {
    let session = TestSession::single(
        r#"
type Value = Capitalize<"hello">;

const ok: Value = "Hello";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Capitalize<"hello">;

const ok: Value = "Hello";

=== checked ===
type Value = Capitalize<"hello">;
/// @type.symbol symbol=Value source="type Value = Capitalize<\"hello\">" type="Hello"
/// @definition.type symbol=Value source="type Value = Capitalize<\"hello\">" value="Hello"
/// @resolution.name source=Capitalize target=types.string.Capitalize

const ok: Value = "Hello";
/// @type.symbol symbol=ok source=ok type="Hello"
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_capitalize_distributes_over_union() {
    let session = TestSession::single(
        r#"
type Value = Capitalize<"yes" | "no">;

declare const value: Value;

value satisfies "Yes" | "No";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Capitalize<"yes" | "no">;

declare const value: Value;

value satisfies "Yes" | "No";

=== checked ===
type Value = Capitalize<"yes" | "no">;
/// @type.symbol symbol=Value source="type Value = Capitalize<\"yes\" | \"no\">" type="Yes" | "No"
/// @definition.type symbol=Value source="type Value = Capitalize<\"yes\" | \"no\">" value="Yes" | "No"
/// @resolution.name source=Capitalize target=types.string.Capitalize

declare const value: Value;
/// @type.symbol symbol=value source=value type="Yes" | "No"
/// @resolution.name source=Value target=Value

value satisfies "Yes" | "No";
"#,
    );
}

#[test]
fn test_capitalize_rejects_original_casing() {
    let session = TestSession::single(
        r#"
type Value = Capitalize<"hello">;

const bad: Value = "hello";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Capitalize<"hello">;

const bad: Value = "hello";

=== checked ===
type Value = Capitalize<"hello">;
/// @type.symbol symbol=Value source="type Value = Capitalize<\"hello\">" type="Hello"
/// @definition.type symbol=Value source="type Value = Capitalize<\"hello\">" value="Hello"
/// @resolution.name source=Capitalize target=types.string.Capitalize

const bad: Value = "hello";
/// @type.symbol symbol=bad source=bad type="Hello"
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"hello\"' is not assignable to type 'Value'"
/// @diagnostic.label line=4 column=7 source="const bad: Value = \"hello\";"
"#,
    );
}
