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

const ok: Value = "ready";
ok satisfies string;

=== checked ===
type Value = Awaited<string>;
/// @type.symbol symbol=Value source="type Value = Awaited<string>" type=Awaited<string> reduced=string
/// @definition.type symbol=Value source="type Value = Awaited<string>" value=Awaited<string> reduced=string
/// @resolution.name source=Awaited target=types.object.Awaited

const ok: Value = "ready";
/// @type.symbol symbol=ok source=ok type=Value reduced=string
/// @resolution.name source=Value target=Value

ok satisfies string;
/// @resolution.name source=ok target=ok

/// @generic.instance id=Awaited<string> template=types.object.Awaited arguments=(string)
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

const ok: Value = "ready";
ok satisfies string;

=== checked ===
type Value = Awaited<Promise<Promise<string>>>;
/// @type.symbol symbol=Value source="type Value = Awaited<Promise<Promise<string>>>" type=Awaited<Promise<Promise<string>>> reduced=string
/// @definition.type symbol=Value source="type Value = Awaited<Promise<Promise<string>>>" value=Awaited<Promise<Promise<string>>> reduced=string
/// @resolution.name source=Awaited target=types.object.Awaited
/// @resolution.name source=Promise target=async.promise.Promise
/// @resolution.name source=Promise target=async.promise.Promise

const ok: Value = "ready";
/// @type.symbol symbol=ok source=ok type=Value reduced=string
/// @resolution.name source=Value target=Value

ok satisfies string;
/// @resolution.name source=ok target=ok

/// @generic.instance id=Awaited<Promise<Promise<string>>> template=types.object.Awaited arguments=(Promise<Promise<string>>)
/// @generic.instance id=Promise<Promise<string>> template=async.promise.Promise arguments=(Promise<string>)
/// @generic.instance id=Promise<string> template=async.promise.Promise arguments=(string)
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

const first: Value = null as Value;
const second: Value = undefined as Value;

=== checked ===
type Value = Awaited<null | undefined>;
/// @type.symbol symbol=Value source="type Value = Awaited<null | undefined>" type=Awaited<null | undefined> reduced=null | undefined
/// @definition.type symbol=Value source="type Value = Awaited<null | undefined>" value=Awaited<null | undefined> reduced=null | undefined
/// @resolution.name source=Awaited target=types.object.Awaited

const first: Value = null;
/// @type.symbol symbol=first source=first type=Value reduced=null | undefined
/// @resolution.name source=Value target=Value

const second: Value = undefined;
/// @type.symbol symbol=second source=second type=Value reduced=null | undefined
/// @resolution.name source=Value target=Value

/// @generic.instance id="Awaited<null | undefined>" template=types.object.Awaited arguments=(null | undefined)
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
const bad: Value = promise;

=== checked ===
type Value = Awaited<Promise<string>>;
/// @type.symbol symbol=Value source="type Value = Awaited<Promise<string>>" type=Awaited<Promise<string>> reduced=string
/// @definition.type symbol=Value source="type Value = Awaited<Promise<string>>" value=Awaited<Promise<string>> reduced=string
/// @resolution.name source=Awaited target=types.object.Awaited
/// @resolution.name source=Promise target=async.promise.Promise

declare const promise: Promise<string>;
/// @type.symbol symbol=promise source=promise type=Promise<string>
/// @resolution.name source=Promise target=async.promise.Promise

const bad: Value = promise;
/// @type.symbol symbol=bad source=bad type=Value reduced=string
/// @resolution.name source=Value target=Value
/// @resolution.name source=promise target=promise

/// @generic.instance id=Awaited<Promise<string>> template=types.object.Awaited arguments=(Promise<string>)
/// @generic.instance id=Promise<string> template=async.promise.Promise arguments=(string)
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Promise<string>' is not assignable to type 'Value'"
/// @diagnostic.label line=5 column=20 span="promise" line_source="const bad: Value = promise;"
"#,
    );
}
