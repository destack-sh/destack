use crate::tests::{DirRows, TestSession};

#[test]
fn test_return_type_extracts_function_return() {
    let session = TestSession::single(
        r#"
type Value = ReturnType<() => string>;

const ok: Value = "ready";
ok satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = ReturnType<() => string>;

const ok: string = "ready";
ok satisfies string;

=== checked ===
type Value = ReturnType<() => string>;
/// @type.symbol symbol=Value source="type Value = ReturnType<() => string>" type=string
/// @definition.type symbol=Value source="type Value = ReturnType<() => string>" value=string
/// @resolution.name source=ReturnType target=types.function.ReturnType

const ok: Value = "ready";
/// @type.symbol symbol=ok source=ok type=string
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value

ok satisfies string;
/// @resolution.name source=ok target=ok
/// @resolution.place source=ok placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=ok root=ok
"#,
    );
}

#[test]
fn test_return_type_keeps_union_returns() {
    let session = TestSession::single(
        r#"
type Value = ReturnType<() => "a" | "b">;

const first: Value = "a";
const second: Value = "b";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = ReturnType<() => "a" | "b">;

const first: "a" | "b" = "a" as "a" | "b";
const second: "a" | "b" = "b" as "a" | "b";

=== checked ===
type Value = ReturnType<() => "a" | "b">;
/// @type.symbol symbol=Value source="type Value = ReturnType<() => \"a\" | \"b\">" type="a" | "b"
/// @definition.type symbol=Value source="type Value = ReturnType<() => \"a\" | \"b\">" value="a" | "b"
/// @resolution.name source=ReturnType target=types.function.ReturnType

const first: Value = "a";
/// @type.symbol symbol=first source=first type="a" | "b"
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=Value target=Value

const second: Value = "b";
/// @type.symbol symbol=second source=second type="a" | "b"
/// @resolution.pattern source=second kind=binding target=second
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_return_type_rejects_wrong_values() {
    let session = TestSession::single(
        r#"
type Value = ReturnType<() => string>;

const bad: Value = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = ReturnType<() => string>;

const bad: string = 1;

=== checked ===
type Value = ReturnType<() => string>;
/// @type.symbol symbol=Value source="type Value = ReturnType<() => string>" type=string
/// @definition.type symbol=Value source="type Value = ReturnType<() => string>" value=string
/// @resolution.name source=ReturnType target=types.function.ReturnType

const bad: Value = 1;
/// @type.symbol symbol=bad source=bad type=string
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '1' is not assignable to type 'string'"
/// @diagnostic.label line=4 column=20 span="1" line_source="const bad: Value = 1;"
/// @diagnostic.related line=4 column=12 span="Value" line_source="const bad: Value = 1;" message="expected due to this annotation"
"#,
    );
}
