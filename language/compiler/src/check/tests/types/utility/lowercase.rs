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
/// @type.symbol symbol=Value source="type Value = Lowercase<\"HELLO\">" type=Lowercase<"HELLO"> reduced="hello"
/// @definition.type symbol=Value source="type Value = Lowercase<\"HELLO\">" value=Lowercase<"HELLO"> reduced="hello"
/// @resolution.name source=Lowercase target=types.string.Lowercase

const ok: Value = "hello";
/// @type.symbol symbol=ok source=ok type=Value reduced="hello"
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value

/// @generic.instance id="Lowercase<\"HELLO\">" template=types.string.Lowercase arguments=("HELLO")
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
/// @type.symbol symbol=Method source="type Method = Lowercase<\"GET\" | \"POST\">" type=Lowercase<"GET" | "POST"> reduced="get" | "post"
/// @definition.type symbol=Method source="type Method = Lowercase<\"GET\" | \"POST\">" value=Lowercase<"GET" | "POST"> reduced="get" | "post"
/// @resolution.name source=Lowercase target=types.string.Lowercase

declare const method: Method;
/// @type.symbol symbol=method source=method type=Method reduced="get" | "post"
/// @resolution.pattern source=method kind=binding target=method
/// @resolution.name source=Method target=Method

method satisfies "get" | "post";
/// @resolution.name source=method target=method

/// @generic.instance id="Lowercase<\"GET\" | \"POST\">" template=types.string.Lowercase arguments=("GET" | "POST")
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
/// @type.symbol symbol=Value source="type Value = Lowercase<\"HELLO\">" type=Lowercase<"HELLO"> reduced="hello"
/// @definition.type symbol=Value source="type Value = Lowercase<\"HELLO\">" value=Lowercase<"HELLO"> reduced="hello"
/// @resolution.name source=Lowercase target=types.string.Lowercase

const bad: Value = "HELLO";
/// @type.symbol symbol=bad source=bad type=Value reduced="hello"
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value

/// @generic.instance id="Lowercase<\"HELLO\">" template=types.string.Lowercase arguments=("HELLO")
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"HELLO\"' is not assignable to type 'Value'"
/// @diagnostic.label line=4 column=20 span="\"HELLO\"" line_source="const bad: Value = \"HELLO\";"
/// @diagnostic.related line=4 column=12 span="Value" line_source="const bad: Value = \"HELLO\";" message="expected due to this annotation"
/// @diagnostic.note message="'Value' reduces to '\"hello\"'"
"#,
    );
}
