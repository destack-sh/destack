use crate::tests::{DirRows, TestSession};

#[test]
fn test_uppercase_converts_literal() {
    let session = TestSession::single(
        r#"
type Value = Uppercase<"hello">;

const ok: Value = "HELLO";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Uppercase<"hello">;

const ok: Value = "HELLO";

=== checked ===
type Value = Uppercase<"hello">;
/// @type.symbol symbol=Value source="type Value = Uppercase<\"hello\">" type="HELLO"
/// @definition.type symbol=Value source="type Value = Uppercase<\"hello\">" value="HELLO"
/// @resolution.name source=Uppercase target=types.string.Uppercase

const ok: Value = "HELLO";
/// @type.symbol symbol=ok source=ok type="HELLO"
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_uppercase_distributes_over_union() {
    let session = TestSession::single(
        r#"
type Method = Uppercase<"get" | "post">;

declare const method: Method;

method satisfies "GET" | "POST";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Method = Uppercase<"get" | "post">;

declare const method: Method;

method satisfies "GET" | "POST";

=== checked ===
type Method = Uppercase<"get" | "post">;
/// @type.symbol symbol=Method source="type Method = Uppercase<\"get\" | \"post\">" type="GET" | "POST"
/// @definition.type symbol=Method source="type Method = Uppercase<\"get\" | \"post\">" value="GET" | "POST"
/// @resolution.name source=Uppercase target=types.string.Uppercase

declare const method: Method;
/// @type.symbol symbol=method source=method type="GET" | "POST"
/// @resolution.name source=Method target=Method

method satisfies "GET" | "POST";
"#,
    );
}

#[test]
fn test_uppercase_rejects_original_casing() {
    let session = TestSession::single(
        r#"
type Value = Uppercase<"hello">;

const bad: Value = "hello";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Uppercase<"hello">;

const bad: Value = "hello";

=== checked ===
type Value = Uppercase<"hello">;
/// @type.symbol symbol=Value source="type Value = Uppercase<\"hello\">" type="HELLO"
/// @definition.type symbol=Value source="type Value = Uppercase<\"hello\">" value="HELLO"
/// @resolution.name source=Uppercase target=types.string.Uppercase

const bad: Value = "hello";
/// @type.symbol symbol=bad source=bad type="HELLO"
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"hello\"' is not assignable to type 'Value'"
/// @diagnostic.label line=4 column=7 source="const bad: Value = \"hello\";"
"#,
    );
}
