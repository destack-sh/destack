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
/// @type.symbol symbol=Value source="type Value = Uppercase<\"hello\">" type=Uppercase<"hello"> reduced="HELLO"
/// @definition.type symbol=Value source="type Value = Uppercase<\"hello\">" value=Uppercase<"hello"> reduced="HELLO"
/// @resolution.name source=Uppercase target=types.string.Uppercase

const ok: Value = "HELLO";
/// @type.symbol symbol=ok source=ok type=Value reduced="HELLO"
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value

/// @generic.instance id="Uppercase<\"hello\">" template=types.string.Uppercase arguments=("hello")
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
/// @type.symbol symbol=Method source="type Method = Uppercase<\"get\" | \"post\">" type=Uppercase<"get" | "post"> reduced="GET" | "POST"
/// @definition.type symbol=Method source="type Method = Uppercase<\"get\" | \"post\">" value=Uppercase<"get" | "post"> reduced="GET" | "POST"
/// @resolution.name source=Uppercase target=types.string.Uppercase

declare const method: Method;
/// @type.symbol symbol=method source=method type=Method reduced="GET" | "POST"
/// @resolution.pattern source=method kind=binding target=method
/// @resolution.name source=Method target=Method

method satisfies "GET" | "POST";
/// @resolution.name source=method target=method

/// @generic.instance id="Uppercase<\"get\" | \"post\">" template=types.string.Uppercase arguments=("get" | "post")
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
/// @type.symbol symbol=Value source="type Value = Uppercase<\"hello\">" type=Uppercase<"hello"> reduced="HELLO"
/// @definition.type symbol=Value source="type Value = Uppercase<\"hello\">" value=Uppercase<"hello"> reduced="HELLO"
/// @resolution.name source=Uppercase target=types.string.Uppercase

const bad: Value = "hello";
/// @type.symbol symbol=bad source=bad type=Value reduced="HELLO"
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value

/// @generic.instance id="Uppercase<\"hello\">" template=types.string.Uppercase arguments=("hello")
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"hello\"' is not assignable to type 'Value'"
/// @diagnostic.label line=4 column=20 span="\"hello\"" line_source="const bad: Value = \"hello\";"
/// @diagnostic.related line=4 column=12 span="Value" line_source="const bad: Value = \"hello\";" message="expected due to this annotation"
/// @diagnostic.note message="'Value' reduces to '\"HELLO\"'"
"#,
    );
}
