use crate::tests::{DirRows, TestSession};

/// Lowercase lowercases a string literal.
#[test]
fn test_lowercase_converts_literal() {
    let session = TestSession::single(
        r#"
type Value = Lowercase<"HELLO">;

const ok: Value = "hello";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Lowercase<"HELLO">;

const ok: Value = "hello";

=== dir ===
type Value = Lowercase<"HELLO">;
/// @type.symbol symbol=Value source="type Value = Lowercase<\"HELLO\">" type="hello"
/// @generic.instance id="Lowercase<\"HELLO\">" template=Lowercase arguments=("HELLO")
/// @definition.type symbol=Value source="type Value = Lowercase<\"HELLO\">" value=Lowercase<"HELLO">
/// @resolution.name source=Lowercase target=Lowercase

const ok: Value = "hello";
/// @type.symbol symbol=ok source=ok type=Value
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value
"#,
    );
}

/// Lowercase distributes over each arm of a union.
#[test]
fn test_lowercase_distributes_over_union() {
    let session = TestSession::single(
        r#"
type Method = Lowercase<"GET" | "POST">;

declare const method: Method;

method satisfies "get" | "post";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Method = Lowercase<"GET" | "POST">;

declare const method: Method;

method satisfies "get" | "post";

=== dir ===
type Method = Lowercase<"GET" | "POST">;
/// @type.symbol symbol=Method source="type Method = Lowercase<\"GET\" | \"POST\">" type="get" | "post"
/// @generic.instance id="Lowercase<\"GET\" | \"POST\">" template=Lowercase arguments=("GET" | "POST")
/// @definition.type symbol=Method source="type Method = Lowercase<\"GET\" | \"POST\">" value=Lowercase<"GET" | "POST">
/// @resolution.name source=Lowercase target=Lowercase

declare const method: Method;
/// @type.symbol symbol=method source=method type=Method
/// @resolution.pattern source=method kind=binding target=method
/// @resolution.name source=Method target=Method

method satisfies "get" | "post";
/// @resolution.name source=method target=method
/// @resolution.place source=method placement="local" lifetime="static" access="immutable"
/// @resolution.access source=method root=method
"#,
    );
}

/// The original casing reports a diagnostic against a Lowercase type.
#[test]
fn test_lowercase_rejects_original_casing() {
    let session = TestSession::single(
        r#"
type Value = Lowercase<"HELLO">;

const bad: Value = "HELLO";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Lowercase<"HELLO">;

const bad: Value = "HELLO";

=== dir ===
type Value = Lowercase<"HELLO">;
/// @type.symbol symbol=Value source="type Value = Lowercase<\"HELLO\">" type="hello"
/// @definition.type symbol=Value source="type Value = Lowercase<\"HELLO\">" value=Lowercase<"HELLO">
/// @resolution.name source=Lowercase target=Lowercase

const bad: Value = "HELLO";
/// @type.symbol symbol=bad source=bad type=Value
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"HELLO\"' is not assignable to type 'Value'"
/// @diagnostic.label line=4 column=20 span="\"HELLO\"" line_source="const bad: Value = \"HELLO\";"
/// @diagnostic.related line=4 column=12 span="Value" line_source="const bad: Value = \"HELLO\";" message="expected due to this annotation"
/// @diagnostic.note message="'Value' reduces to '\"hello\"'"
"#,
    );
}
