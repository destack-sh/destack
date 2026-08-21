use crate::tests::{DirRows, TestSession};

#[test]
fn test_uppercase_a_string_literal() {
    let session = TestSession::single(
        r#"
type Value = Uppercase<"hello">;

const ok: Value = "HELLO";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Uppercase<"hello">;

const ok: "HELLO" = "HELLO";

=== dir ===
type Value = Uppercase<"hello">;
/// @type.symbol symbol=Value source="type Value = Uppercase<\"hello\">" type="HELLO"
/// @definition.type symbol=Value source="type Value = Uppercase<\"hello\">" value="HELLO"
/// @resolution.name source=Uppercase target=types.string.Uppercase

const ok: Value = "HELLO";
/// @type.symbol symbol=ok source=ok type="HELLO"
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_distribute_uppercase_over_a_union() {
    let session = TestSession::single(
        r#"
type Method = Uppercase<"get" | "post">;

declare const method: Method;

method satisfies "GET" | "POST";
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Method = Uppercase<"get" | "post">;

declare const method: "GET" | "POST";

method satisfies "GET" | "POST";

=== dir ===
type Method = Uppercase<"get" | "post">;
/// @type.symbol symbol=Method source="type Method = Uppercase<\"get\" | \"post\">" type="GET" | "POST"
/// @definition.type symbol=Method source="type Method = Uppercase<\"get\" | \"post\">" value="GET" | "POST"
/// @resolution.name source=Uppercase target=types.string.Uppercase

declare const method: Method;
/// @type.symbol symbol=method source=method type="GET" | "POST"
/// @resolution.pattern source=method kind=binding target=method
/// @resolution.name source=Method target=Method

method satisfies "GET" | "POST";
/// @resolution.name source=method target=method
/// @resolution.place source=method placement="local" lifetime="static" access="readonly"
/// @resolution.access source=method root=method
"#,
    );
}

#[test]
fn test_reject_the_original_casing_for_an_uppercase_literal_type() {
    let session = TestSession::single(
        r#"
type Value = Uppercase<"hello">;

const bad: Value = "hello";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Uppercase<"hello">;

const bad: "HELLO" = "hello";

=== dir ===
type Value = Uppercase<"hello">;
/// @type.symbol symbol=Value source="type Value = Uppercase<\"hello\">" type="HELLO"
/// @definition.type symbol=Value source="type Value = Uppercase<\"hello\">" value="HELLO"
/// @resolution.name source=Uppercase target=types.string.Uppercase

const bad: Value = "hello";
/// @type.symbol symbol=bad source=bad type="HELLO"
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"hello\"' is not assignable to type '\"HELLO\"'"
/// @diagnostic.label line=4 column=20 span="\"hello\"" line_source="const bad: Value = \"hello\";"
/// @diagnostic.related line=4 column=12 span="Value" line_source="const bad: Value = \"hello\";" message="expected due to this annotation"
"#,
    );
}

/// Map an uppercase intrinsic through the spans of a template.
#[test]
fn test_map_an_uppercase_intrinsic_through_template_spans() {
    let session = TestSession::single(
        r#"
type Shout = Uppercase<`a${string}`>;

declare const shout: Shout;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Shout = Uppercase<`a${string}`>;

declare const shout: Shout;

=== dir ===
type Shout = Uppercase<`a${string}`>;
/// @type.symbol symbol=Shout source="type Shout = Uppercase<`a${string}`>" type=`A${Uppercase<string>}`
/// @definition.type symbol=Shout source="type Shout = Uppercase<`a${string}`>" value=`A${Uppercase<string>}`
/// @resolution.name source=Uppercase target=types.string.Uppercase

declare const shout: Shout;
/// @type.symbol symbol=shout source=shout type=`A${Uppercase<string>}`
/// @resolution.pattern source=shout kind=binding target=shout
/// @resolution.name source=Shout target=Shout
"#,
        r#"
"#,
    );
}
