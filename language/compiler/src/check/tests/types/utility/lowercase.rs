use crate::tests::{DirRows, TestSession};

#[test]
fn test_lowercase_converts_literal() {
    let session = TestSession::single(
        r#"
type Value = Lowercase<"HELLO">;

const ok: Value = "hello";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Lowercase<"HELLO">;

const ok: Value = "hello";

=== checked ===
type Value = Lowercase<"HELLO">;
/// @type.symbol symbol=Value source="type Value = Lowercase<\"HELLO\">" type="hello"
/// @definition.type symbol=Value source="type Value = Lowercase<\"HELLO\">" value="hello"
/// @resolution.name source=Lowercase target=types.string.Lowercase

const ok: Value = "hello";
/// @type.symbol symbol=ok source=ok type="hello"
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_lowercase_distributes_over_union() {
    let session = TestSession::single(
        r#"
type Method = Lowercase<"GET" | "POST">;

declare const method: Method;

method satisfies "get" | "post";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Method = Lowercase<"GET" | "POST">;

declare const method: Method;

method satisfies "get" | "post";

=== checked ===
type Method = Lowercase<"GET" | "POST">;
/// @type.symbol symbol=Method source="type Method = Lowercase<\"GET\" | \"POST\">" type="get" | "post"
/// @definition.type symbol=Method source="type Method = Lowercase<\"GET\" | \"POST\">" value="get" | "post"
/// @resolution.name source=Lowercase target=types.string.Lowercase

declare const method: Method;
/// @type.symbol symbol=method source=method type="get" | "post"
/// @resolution.name source=Method target=Method

method satisfies "get" | "post";
"#,
    );
}

#[test]
fn test_lowercase_rejects_original_casing() {
    let session = TestSession::single(
        r#"
type Value = Lowercase<"HELLO">;

const bad: Value = "HELLO";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Lowercase<"HELLO">;

const bad: Value = "HELLO";

=== checked ===
type Value = Lowercase<"HELLO">;
/// @type.symbol symbol=Value source="type Value = Lowercase<\"HELLO\">" type="hello"
/// @definition.type symbol=Value source="type Value = Lowercase<\"HELLO\">" value="hello"
/// @resolution.name source=Lowercase target=types.string.Lowercase

const bad: Value = "HELLO";
/// @type.symbol symbol=bad source=bad type="hello"
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"HELLO\"' is not assignable to type 'Value'"
/// @diagnostic.label line=4 column=7 source="const bad: Value = \"HELLO\";"
"#,
    );
}
