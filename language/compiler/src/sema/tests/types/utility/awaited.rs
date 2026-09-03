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

const ok: string = "ready";
ok satisfies string;

=== dir ===
type Value = Awaited<string>;
/// @type.symbol symbol=Value source="type Value = Awaited<string>" type=string
/// @definition.type symbol=Value source="type Value = Awaited<string>" value=string
/// @resolution.name source=Awaited target=Awaited

const ok: Value = "ready";
/// @type.symbol symbol=ok source=ok type=string
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value

ok satisfies string;
/// @resolution.name source=ok target=ok
/// @resolution.place source=ok placement="local" lifetime="managed" access="exclusive"
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

const ok: string = "ready";
ok satisfies string;

=== dir ===
type Value = Awaited<Promise<Promise<string>>>;
/// @type.symbol symbol=Value source="type Value = Awaited<Promise<Promise<string>>>" type=string
/// @definition.type symbol=Value source="type Value = Awaited<Promise<Promise<string>>>" value=string
/// @resolution.name source=Awaited target=Awaited
/// @resolution.name source=Promise target=Promise
/// @generic.instance id="Promise.symbol12<Promise<string>, \"local\">" template=Promise.symbol12 arguments=(Promise<string>, "local")
/// @generic.instance id="Promise<Promise<string>, \"local\">" template=Promise arguments=(Promise<string>, "local")
/// @generic.instance id="PromiseReaction.symbol173<Promise<string>, \"local\">" template=PromiseReaction.symbol173 arguments=(Promise<string>, "local")
/// @generic.instance id="PromiseReaction<Promise<string>, \"local\">" template=PromiseReaction arguments=(Promise<string>, "local")
/// @generic.instance id=Awaitable<Promise<string>> template=Awaitable arguments=(Promise<string>)
/// @generic.instance id=Promise.addReaction<Promise<string>> template=Promise.addReaction arguments=(Promise<string>)
/// @generic.instance id=Promise.addWaiter<Promise<string>> template=Promise.addWaiter arguments=(Promise<string>)
/// @generic.instance id=Promise.forward<Promise<string>> template=Promise.forward arguments=(Promise<string>)
/// @generic.instance id=Promise.fulfill<Promise<string>> template=Promise.fulfill arguments=(Promise<string>)
/// @generic.instance id=Promise.observe<Promise<string>> template=Promise.observe arguments=(Promise<string>)
/// @generic.instance id=Promise.pending<Promise<string>> template=Promise.pending arguments=(Promise<string>)
/// @generic.instance id=Promise.queueWaiter<Promise<string>> template=Promise.queueWaiter arguments=(Promise<string>)
/// @generic.instance id=Promise.queueWaiters<Promise<string>> template=Promise.queueWaiters arguments=(Promise<string>)
/// @generic.instance id=Promise<Promise<string>> template=Promise arguments=(Promise<string>)
/// @generic.instance id=PromiseAwaiter<Promise<string>> template=PromiseAwaiter arguments=(Promise<string>)
/// @generic.instance id=PromiseForwarded<Promise<string>> template=PromiseForwarded arguments=(Promise<string>)
/// @generic.instance id=PromiseFulfilled<Promise<string>> template=PromiseFulfilled arguments=(Promise<string>)
/// @generic.instance id=PromisePending<Promise<string>> template=PromisePending arguments=(Promise<string>)
/// @generic.instance id=PromiseReaction<Promise<string>> template=PromiseReaction arguments=(Promise<string>)
/// @generic.instance id=PromiseState<Promise<string>> template=PromiseState arguments=(Promise<string>)
/// @generic.instance id=PromiseWaiter<Promise<string>> template=PromiseWaiter arguments=(Promise<string>)
/// @resolution.name source=Promise target=Promise
/// @generic.instance id="Promise.symbol12<string, \"local\">" template=Promise.symbol12 arguments=(string, "local")
/// @generic.instance id="Promise.symbol12<void, \"local\">" template=Promise.symbol12 arguments=(void, "local")
/// @generic.instance id="Promise<string, \"local\">" template=Promise arguments=(string, "local")
/// @generic.instance id="Promise<void, \"local\">" template=Promise arguments=(void, "local")
/// @generic.instance id="PromiseReaction.symbol173<string, \"local\">" template=PromiseReaction.symbol173 arguments=(string, "local")
/// @generic.instance id="PromiseReaction.symbol173<void, \"local\">" template=PromiseReaction.symbol173 arguments=(void, "local")
/// @generic.instance id="PromiseReaction<string, \"local\">" template=PromiseReaction arguments=(string, "local")
/// @generic.instance id="PromiseReaction<void, \"local\">" template=PromiseReaction arguments=(void, "local")
/// @generic.instance id=Awaitable<string> template=Awaitable arguments=(string)
/// @generic.instance id=Awaitable<void> template=Awaitable arguments=(void)
/// @generic.instance id=Promise.addReaction<string> template=Promise.addReaction arguments=(string)
/// @generic.instance id=Promise.addReaction<void> template=Promise.addReaction arguments=(void)
/// @generic.instance id=Promise.addWaiter<string> template=Promise.addWaiter arguments=(string)
/// @generic.instance id=Promise.addWaiter<void> template=Promise.addWaiter arguments=(void)
/// @generic.instance id=Promise.forward<string> template=Promise.forward arguments=(string)
/// @generic.instance id=Promise.forward<void> template=Promise.forward arguments=(void)
/// @generic.instance id=Promise.fulfill<string> template=Promise.fulfill arguments=(string)
/// @generic.instance id=Promise.fulfill<void> template=Promise.fulfill arguments=(void)
/// @generic.instance id=Promise.observe<string> template=Promise.observe arguments=(string)
/// @generic.instance id=Promise.observe<void> template=Promise.observe arguments=(void)
/// @generic.instance id=Promise.pending<string> template=Promise.pending arguments=(string)
/// @generic.instance id=Promise.pending<void> template=Promise.pending arguments=(void)
/// @generic.instance id=Promise.queueWaiter<string> template=Promise.queueWaiter arguments=(string)
/// @generic.instance id=Promise.queueWaiter<void> template=Promise.queueWaiter arguments=(void)
/// @generic.instance id=Promise.queueWaiters<string> template=Promise.queueWaiters arguments=(string)
/// @generic.instance id=Promise.queueWaiters<void> template=Promise.queueWaiters arguments=(void)
/// @generic.instance id=Promise<string> template=Promise arguments=(string)
/// @generic.instance id=Promise<void> template=Promise arguments=(void)
/// @generic.instance id=PromiseAwaiter<string> template=PromiseAwaiter arguments=(string)
/// @generic.instance id=PromiseAwaiter<void> template=PromiseAwaiter arguments=(void)
/// @generic.instance id=PromiseForwarded<string> template=PromiseForwarded arguments=(string)
/// @generic.instance id=PromiseForwarded<void> template=PromiseForwarded arguments=(void)
/// @generic.instance id=PromiseFulfilled<string> template=PromiseFulfilled arguments=(string)
/// @generic.instance id=PromiseFulfilled<void> template=PromiseFulfilled arguments=(void)
/// @generic.instance id=PromisePending<string> template=PromisePending arguments=(string)
/// @generic.instance id=PromisePending<void> template=PromisePending arguments=(void)
/// @generic.instance id=PromiseReaction<string> template=PromiseReaction arguments=(string)
/// @generic.instance id=PromiseReaction<void> template=PromiseReaction arguments=(void)
/// @generic.instance id=PromiseState<string> template=PromiseState arguments=(string)
/// @generic.instance id=PromiseState<void> template=PromiseState arguments=(void)
/// @generic.instance id=PromiseWaiter<string> template=PromiseWaiter arguments=(string)
/// @generic.instance id=PromiseWaiter<void> template=PromiseWaiter arguments=(void)

const ok: Value = "ready";
/// @type.symbol symbol=ok source=ok type=string
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=Value target=Value

ok satisfies string;
/// @resolution.name source=ok target=ok
/// @resolution.place source=ok placement="local" lifetime="managed" access="exclusive"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Awaited<null | undefined>;

const first: null | undefined = null as null | undefined;
const second: null | undefined = undefined as null | undefined;

=== dir ===
type Value = Awaited<null | undefined>;
/// @type.symbol symbol=Value source="type Value = Awaited<null | undefined>" type=null | undefined
/// @definition.type symbol=Value source="type Value = Awaited<null | undefined>" value=null | undefined
/// @resolution.name source=Awaited target=Awaited

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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Value = Awaited<Promise<string>>;

declare const promise: Promise<string>;
const bad: string = promise;

=== dir ===
type Value = Awaited<Promise<string>>;
/// @type.symbol symbol=Value source="type Value = Awaited<Promise<string>>" type=string
/// @definition.type symbol=Value source="type Value = Awaited<Promise<string>>" value=string
/// @resolution.name source=Awaited target=Awaited
/// @resolution.name source=Promise target=Promise

declare const promise: Promise<string>;
/// @type.symbol symbol=promise source=promise type=Promise<string>
/// @resolution.pattern source=promise kind=binding target=promise
/// @resolution.name source=Promise target=Promise

const bad: Value = promise;
/// @type.symbol symbol=bad source=bad type=string
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Value target=Value
/// @resolution.name source=promise target=promise
/// @resolution.place source=promise placement="local" lifetime="managed" access="exclusive"
/// @resolution.access source=promise root=promise
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Promise<string>' is not assignable to type 'string'"
/// @diagnostic.label line=5 column=20 span="promise" line_source="const bad: Value = promise;"
/// @diagnostic.related line=5 column=12 span="Value" line_source="const bad: Value = promise;" message="expected due to this annotation"
"#,
    );
}
