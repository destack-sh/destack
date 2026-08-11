use crate::tests::{DirRows, TestSession};

#[test]
fn test_awaited_keeps_non_promise_values() {
    let session = TestSession::single(
        r#"
type Value = Awaited<string>;

const ok: Value = "ready";
ok satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Awaited<string>;

const ok: string = "ready";
ok satisfies string;

=== checked ===
type Value = Awaited<string>;
/// @type.symbol symbol=Value source="type Value = Awaited<string>" type=string
/// @definition.type symbol=Value source="type Value = Awaited<string>" value=string
/// @resolution.name source=Awaited target=types.object.Awaited

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
fn test_awaited_unwraps_nested_promises() {
    let session = TestSession::single(
        r#"
type Value = Awaited<Promise<Promise<string>>>;

const ok: Value = "ready";
ok satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Awaited<Promise<Promise<string>>>;

const ok: string = "ready";
ok satisfies string;

=== checked ===
type Value = Awaited<Promise<Promise<string>>>;
/// @type.symbol symbol=Value source="type Value = Awaited<Promise<Promise<string>>>" type=string
/// @definition.type symbol=Value source="type Value = Awaited<Promise<Promise<string>>>" value=string
/// @resolution.name source=Awaited target=types.object.Awaited
/// @resolution.name source=Promise target=async.promise.Promise
/// @resolution.name source=Promise target=async.promise.Promise

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
fn test_awaited_preserves_nullish_values() {
    let session = TestSession::single(
        r#"
type Value = Awaited<null | undefined>;

const first: Value = null;
const second: Value = undefined;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Awaited<null | undefined>;

const first: null | undefined = null as null | undefined;
const second: null | undefined = undefined as null | undefined;

=== checked ===
type Value = Awaited<null | undefined>;
/// @type.symbol symbol=Value source="type Value = Awaited<null | undefined>" type=null | undefined
/// @definition.type symbol=Value source="type Value = Awaited<null | undefined>" value=null | undefined
/// @resolution.name source=Awaited target=types.object.Awaited

const first: Value = null;
/// @type.symbol symbol=first source=first type=null | undefined
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=Value target=Value

const second: Value = undefined;
/// @type.symbol symbol=second source=second type=null | undefined
/// @resolution.pattern source=second kind=binding target=second
/// @resolution.name source=Value target=Value
"#,
    );
}

#[test]
fn test_awaited_rejects_unresolved_promise_value() {
    let session = TestSession::single(
        r#"
type Value = Awaited<Promise<string>>;

declare const promise: Promise<string>;
const bad: Value = promise;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Awaited<Promise<string>>;

declare const promise: Promise<string>;
const bad: string = promise;

=== checked ===
type Value = Awaited<Promise<string>>;
/// @type.symbol symbol=Value source="type Value = Awaited<Promise<string>>" type=string
/// @definition.type symbol=Value source="type Value = Awaited<Promise<string>>" value=string
/// @resolution.name source=Awaited target=types.object.Awaited
/// @resolution.name source=Promise target=async.promise.Promise

declare const promise: Promise<string>;
/// @type.symbol symbol=promise source=promise type=Promise<string>
/// @resolution.pattern source=promise kind=binding target=promise
/// @resolution.name source=Promise target=async.promise.Promise

const bad: Value = promise;
/// @type.symbol symbol=bad source=bad type=string
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value
/// @resolution.name source=promise target=promise
/// @resolution.place source=promise placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=promise root=promise

/// @generic.instance id=Promise<string> template=async.promise.Promise arguments=(string)
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Promise<string>' is not assignable to type 'string'"
/// @diagnostic.label line=5 column=20 span="promise" line_source="const bad: Value = promise;"
/// @diagnostic.related line=5 column=12 span="Value" line_source="const bad: Value = promise;" message="expected due to this annotation"
"#,
    );
}
