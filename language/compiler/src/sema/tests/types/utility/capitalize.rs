use crate::tests::{DirRows, TestSession};

/// Capitalize uppercases the first character of a string literal.
#[test]
fn test_capitalize_converts_first_character() {
    let session = TestSession::single(
        r#"
type Value = Capitalize<"hello">;

const ok: Value = "Hello";
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Capitalize<"hello">;

const ok: Value = "Hello";

=== dir ===
type Value = Capitalize<"hello">;
/// @type.symbol symbol=Value source="type Value = Capitalize<\"hello\">" type="Hello"
/// @generic.instance id="Capitalize<\"hello\">" template=Capitalize arguments=("hello")
/// @definition.type symbol=Value source="type Value = Capitalize<\"hello\">" value=Capitalize<"hello">
/// @resolution.name source=Capitalize target=Capitalize

const ok: Value = "Hello";
/// @type.symbol symbol=ok source=ok type=Value
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value
"#,
    );
}

/// Capitalize distributes over each arm of a union.
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Capitalize<"yes" | "no">;

declare const value: Value;

value satisfies "Yes" | "No";

=== dir ===
type Value = Capitalize<"yes" | "no">;
/// @type.symbol symbol=Value source="type Value = Capitalize<\"yes\" | \"no\">" type="Yes" | "No"
/// @generic.instance id="Capitalize<\"yes\" | \"no\">" template=Capitalize arguments=("yes" | "no")
/// @definition.type symbol=Value source="type Value = Capitalize<\"yes\" | \"no\">" value=Capitalize<"yes" | "no">
/// @resolution.name source=Capitalize target=Capitalize

declare const value: Value;
/// @type.symbol symbol=value source=value type=Value
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Value target=Value

value satisfies "Yes" | "No";
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
    );
}

/// The original casing reports a diagnostic against a Capitalize type.
#[test]
fn test_capitalize_rejects_original_casing() {
    let session = TestSession::single(
        r#"
type Value = Capitalize<"hello">;

const bad: Value = "hello";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Capitalize<"hello">;

const bad: Value = "hello";

=== dir ===
type Value = Capitalize<"hello">;
/// @type.symbol symbol=Value source="type Value = Capitalize<\"hello\">" type="Hello"
/// @definition.type symbol=Value source="type Value = Capitalize<\"hello\">" value=Capitalize<"hello">
/// @resolution.name source=Capitalize target=Capitalize

const bad: Value = "hello";
/// @type.symbol symbol=bad source=bad type=Value
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"hello\"' is not assignable to type 'Value'"
/// @diagnostic.label line=4 column=20 span="\"hello\"" line_source="const bad: Value = \"hello\";"
/// @diagnostic.related line=4 column=12 span="Value" line_source="const bad: Value = \"hello\";" message="expected due to this annotation"
/// @diagnostic.note message="'Value' reduces to '\"Hello\"'"
"#,
    );
}
