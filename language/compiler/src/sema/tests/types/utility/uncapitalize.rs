use crate::tests::{DirRows, TestSession};

#[test]
fn test_uncapitalize_converts_first_character() {
    let session = TestSession::single(
        r#"
type Value = Uncapitalize<"Hello">;

const ok: Value = "hello";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Uncapitalize<"Hello">;

const ok: "hello" = "hello";

=== dir ===
type Value = Uncapitalize<"Hello">;
/// @type.symbol symbol=Value source="type Value = Uncapitalize<\"Hello\">" type="hello"
/// @definition.type symbol=Value source="type Value = Uncapitalize<\"Hello\">" value="hello"
/// @resolution.name source=Uncapitalize target=types.string.Uncapitalize

const ok: Value = "hello";
/// @type.symbol symbol=ok source=ok type="hello"
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_uncapitalize_distributes_over_union() {
    let session = TestSession::single(
        r#"
type Value = Uncapitalize<"Yes" | "No">;

declare const value: Value;

value satisfies "yes" | "no";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Uncapitalize<"Yes" | "No">;

declare const value: "yes" | "no";

value satisfies "yes" | "no";

=== dir ===
type Value = Uncapitalize<"Yes" | "No">;
/// @type.symbol symbol=Value source="type Value = Uncapitalize<\"Yes\" | \"No\">" type="yes" | "no"
/// @definition.type symbol=Value source="type Value = Uncapitalize<\"Yes\" | \"No\">" value="yes" | "no"
/// @resolution.name source=Uncapitalize target=types.string.Uncapitalize

declare const value: Value;
/// @type.symbol symbol=value source=value type="yes" | "no"
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value

value satisfies "yes" | "no";
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
"#,
    );
}

#[test]
fn test_uncapitalize_rejects_original_casing() {
    let session = TestSession::single(
        r#"
type Value = Uncapitalize<"Hello">;

const bad: Value = "Hello";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Uncapitalize<"Hello">;

const bad: "hello" = "Hello";

=== dir ===
type Value = Uncapitalize<"Hello">;
/// @type.symbol symbol=Value source="type Value = Uncapitalize<\"Hello\">" type="hello"
/// @definition.type symbol=Value source="type Value = Uncapitalize<\"Hello\">" value="hello"
/// @resolution.name source=Uncapitalize target=types.string.Uncapitalize

const bad: Value = "Hello";
/// @type.symbol symbol=bad source=bad type="hello"
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"Hello\"' is not assignable to type '\"hello\"'"
/// @diagnostic.label line=4 column=20 span="\"Hello\"" line_source="const bad: Value = \"Hello\";"
/// @diagnostic.related line=4 column=12 span="Value" line_source="const bad: Value = \"Hello\";" message="expected due to this annotation"
"#,
    );
}
