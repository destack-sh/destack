use crate::tests::{DirRows, TestSession};

#[test]
fn test_capitalize_converts_first_character() {
    let session = TestSession::single(
        r#"
type Value = Capitalize<"hello">;

const ok: Value = "Hello";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Capitalize<"hello">;

const ok: "Hello" = "Hello";

=== dir ===
type Value = Capitalize<"hello">;
/// @type.symbol symbol=Value source="type Value = Capitalize<\"hello\">" type="Hello"
/// @definition.type symbol=Value source="type Value = Capitalize<\"hello\">" value="Hello"
/// @resolution.name source=Capitalize target=types.string.Capitalize

const ok: Value = "Hello";
/// @type.symbol symbol=ok source=ok type="Hello"
/// @resolution.pattern source=ok kind=binding target=ok
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Capitalize<"yes" | "no">;

declare const value: "Yes" | "No";

value satisfies "Yes" | "No";

=== dir ===
type Value = Capitalize<"yes" | "no">;
/// @type.symbol symbol=Value source="type Value = Capitalize<\"yes\" | \"no\">" type="Yes" | "No"
/// @definition.type symbol=Value source="type Value = Capitalize<\"yes\" | \"no\">" value="Yes" | "No"
/// @resolution.name source=Capitalize target=types.string.Capitalize

declare const value: Value;
/// @type.symbol symbol=value source=value type="Yes" | "No"
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value

value satisfies "Yes" | "No";
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Capitalize<"hello">;

const bad: "Hello" = "hello";

=== dir ===
type Value = Capitalize<"hello">;
/// @type.symbol symbol=Value source="type Value = Capitalize<\"hello\">" type="Hello"
/// @definition.type symbol=Value source="type Value = Capitalize<\"hello\">" value="Hello"
/// @resolution.name source=Capitalize target=types.string.Capitalize

const bad: Value = "hello";
/// @type.symbol symbol=bad source=bad type="Hello"
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"hello\"' is not assignable to type '\"Hello\"'"
/// @diagnostic.label line=4 column=20 span="\"hello\"" line_source="const bad: Value = \"hello\";"
/// @diagnostic.related line=4 column=12 span="Value" line_source="const bad: Value = \"hello\";" message="expected due to this annotation"
"#,
    );
}
