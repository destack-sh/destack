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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Awaited<string>;

const ok: Value = "ready";
ok satisfies string;

=== dir ===
type Value = Awaited<string>;
/// @type.symbol symbol=Value source="type Value = Awaited<string>" type=string
/// @generic.instance id=Awaited<string> template=Awaited arguments=(string)
/// @definition.type symbol=Value source="type Value = Awaited<string>" value=Awaited<string>
/// @resolution.name source=Awaited target=Awaited

const ok: Value = "ready";
/// @type.symbol symbol=ok source=ok type=Value
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value

ok satisfies string;
/// @resolution.name source=ok target=ok
/// @resolution.place source=ok placement="local" lifetime="static" access="immutable"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Awaited<Promise<Promise<string>>>;

const ok: Value = "ready";
ok satisfies string;

=== dir ===
type Value = Awaited<Promise<Promise<string>>>;
/// @type.symbol symbol=Value source="type Value = Awaited<Promise<Promise<string>>>" type=string
/// @generic.instance id=Awaited<Promise<Promise<string>>> template=Awaited arguments=(Promise<Promise<string>>)
/// @generic.instance id=Clone.clone<Promise<string>> template=Clone.clone arguments=()
/// @generic.instance id=Promise<Promise<string>> template=Promise arguments=(Promise<string>)
/// @generic.instance id=Promise<string> template=Promise arguments=(string)
/// @definition.type symbol=Value source="type Value = Awaited<Promise<Promise<string>>>" value=Awaited<Promise<Promise<string>>>
/// @resolution.name source=Awaited target=Awaited
/// @resolution.name source=Promise target=Promise
/// @resolution.name source=Promise target=Promise

const ok: Value = "ready";
/// @type.symbol symbol=ok source=ok type=Value
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value

ok satisfies string;
/// @resolution.name source=ok target=ok
/// @resolution.place source=ok placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ok root=ok

/// @generic.template symbol=Clone.clone parameters=('a)
/// @type.symbol symbol=Clone.clone type=<Clone.clone.'a>(this: &Clone.clone.'a immutable Promise<string>) => ^Promise<string>
/// @generic.instance id=PromiseForwarded<string> template=PromiseForwarded arguments=(string)
/// @generic.instance id=PromiseFulfilled<string> template=PromiseFulfilled arguments=(string)
/// @generic.instance id=PromisePending<string> template=PromisePending arguments=(string)
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Awaited<null | undefined>;

const first: Value = null as Value;
const second: Value = undefined as Value;

=== dir ===
type Value = Awaited<null | undefined>;
/// @type.symbol symbol=Value source="type Value = Awaited<null | undefined>" type=null | undefined
/// @generic.instance id="Awaited<null | undefined>" template=Awaited arguments=(null | undefined)
/// @definition.type symbol=Value source="type Value = Awaited<null | undefined>" value=Awaited<null | undefined>
/// @resolution.name source=Awaited target=Awaited

const first: Value = null;
/// @type.symbol symbol=first source=first type=Value
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=Value target=Value

const second: Value = undefined;
/// @type.symbol symbol=second source=second type=Value
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Awaited<Promise<string>>;

declare const promise: Promise<string>;
const bad: Value = promise;

=== dir ===
type Value = Awaited<Promise<string>>;
/// @type.symbol symbol=Value source="type Value = Awaited<Promise<string>>" type=string
/// @definition.type symbol=Value source="type Value = Awaited<Promise<string>>" value=Awaited<Promise<string>>
/// @resolution.name source=Awaited target=Awaited
/// @resolution.name source=Promise target=Promise

declare const promise: Promise<string>;
/// @type.symbol symbol=promise source=promise type=Promise<string>
/// @resolution.pattern source=promise kind=binding target=promise
/// @resolution.name source=Promise target=Promise

const bad: Value = promise;
/// @type.symbol symbol=bad source=bad type=Value
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value
/// @resolution.name source=promise target=promise
/// @resolution.place source=promise placement="local" lifetime="static" access="immutable"
/// @resolution.access source=promise root=promise
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Promise<string>' is not assignable to type 'Value'"
/// @diagnostic.label line=5 column=20 span="promise" line_source="const bad: Value = promise;"
/// @diagnostic.related line=5 column=12 span="Value" line_source="const bad: Value = promise;" message="expected due to this annotation"
/// @diagnostic.note message="'Value' reduces to 'string'"
"#,
    );
}
