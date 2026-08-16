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
/// @resolution.name source=Awaited target=types.object.Awaited
/// @resolution.name source=Promise target=async.promise.Promise
/// @generic.instance id=Promise<Promise<string>> template=async.promise.Promise arguments=(Promise<string>)
/// @generic.instance id=async.awaitable.Awaitable<Promise<string>> template=async.awaitable.Awaitable arguments=(Promise<string>)
/// @generic.instance id=async.fiber.Fiber.wake<Promise<string>> template=async.fiber.Fiber.wake arguments=(Promise<string>)
/// @generic.instance id=async.fiber.wakeFiber<Promise<string>> template=async.fiber.wakeFiber arguments=(Promise<string>)
/// @generic.instance id=async.promise.Promise.addReaction<Promise<string>> template=async.promise.Promise.addReaction arguments=(Promise<string>)
/// @generic.instance id=async.promise.Promise.addWaiter<Promise<string>> template=async.promise.Promise.addWaiter arguments=(Promise<string>)
/// @generic.instance id=async.promise.Promise.cancel<Promise<string>> template=async.promise.Promise.cancel arguments=(Promise<string>)
/// @generic.instance id=async.promise.Promise.cancelWaiter<Promise<string>> template=async.promise.Promise.cancelWaiter arguments=(Promise<string>)
/// @generic.instance id=async.promise.Promise.cancelWaiters<Promise<string>> template=async.promise.Promise.cancelWaiters arguments=(Promise<string>)
/// @generic.instance id=async.promise.Promise.forward<Promise<string>> template=async.promise.Promise.forward arguments=(Promise<string>)
/// @generic.instance id=async.promise.Promise.fulfill<Promise<string>> template=async.promise.Promise.fulfill arguments=(Promise<string>)
/// @generic.instance id=async.promise.Promise.observe<Promise<string>> template=async.promise.Promise.observe arguments=(Promise<string>)
/// @generic.instance id=async.promise.Promise.pending<Promise<string>> template=async.promise.Promise.pending arguments=(Promise<string>)
/// @generic.instance id=async.promise.Promise.queueWaiter<Promise<string>> template=async.promise.Promise.queueWaiter arguments=(Promise<string>)
/// @generic.instance id=async.promise.Promise.queueWaiters<Promise<string>> template=async.promise.Promise.queueWaiters arguments=(Promise<string>)
/// @generic.instance id=async.promise.Promise.symbol12<Promise<string>> template=async.promise.Promise.symbol12 arguments=(Promise<string>)
/// @generic.instance id=async.promise.PromiseAwaiter<Promise<string>> template=async.promise.PromiseAwaiter arguments=(Promise<string>)
/// @generic.instance id=async.promise.PromiseForwarded<Promise<string>> template=async.promise.PromiseForwarded arguments=(Promise<string>)
/// @generic.instance id=async.promise.PromiseFulfilled<Promise<string>> template=async.promise.PromiseFulfilled arguments=(Promise<string>)
/// @generic.instance id=async.promise.PromisePending<Promise<string>> template=async.promise.PromisePending arguments=(Promise<string>)
/// @generic.instance id=async.promise.PromiseReaction.symbol194<Promise<string>> template=async.promise.PromiseReaction.symbol194 arguments=(Promise<string>)
/// @generic.instance id=async.promise.PromiseReaction<Promise<string>> template=async.promise.PromiseReaction arguments=(Promise<string>)
/// @generic.instance id=async.promise.PromiseState<Promise<string>> template=async.promise.PromiseState arguments=(Promise<string>)
/// @generic.instance id=async.promise.PromiseWaiter<Promise<string>> template=async.promise.PromiseWaiter arguments=(Promise<string>)
/// @resolution.name source=Promise target=async.promise.Promise
/// @generic.instance id=Promise<string> template=async.promise.Promise arguments=(string)
/// @generic.instance id=Promise<void> template=async.promise.Promise arguments=(void)
/// @generic.instance id=async.awaitable.Awaitable<string> template=async.awaitable.Awaitable arguments=(string)
/// @generic.instance id=async.awaitable.Awaitable<void> template=async.awaitable.Awaitable arguments=(void)
/// @generic.instance id=async.fiber.Fiber.wake<string> template=async.fiber.Fiber.wake arguments=(string)
/// @generic.instance id=async.fiber.Fiber.wake<void> template=async.fiber.Fiber.wake arguments=(void)
/// @generic.instance id=async.fiber.wakeFiber<string> template=async.fiber.wakeFiber arguments=(string)
/// @generic.instance id=async.fiber.wakeFiber<void> template=async.fiber.wakeFiber arguments=(void)
/// @generic.instance id=async.promise.Promise.addReaction<string> template=async.promise.Promise.addReaction arguments=(string)
/// @generic.instance id=async.promise.Promise.addReaction<void> template=async.promise.Promise.addReaction arguments=(void)
/// @generic.instance id=async.promise.Promise.addWaiter<string> template=async.promise.Promise.addWaiter arguments=(string)
/// @generic.instance id=async.promise.Promise.addWaiter<void> template=async.promise.Promise.addWaiter arguments=(void)
/// @generic.instance id=async.promise.Promise.cancel<string> template=async.promise.Promise.cancel arguments=(string)
/// @generic.instance id=async.promise.Promise.cancel<void> template=async.promise.Promise.cancel arguments=(void)
/// @generic.instance id=async.promise.Promise.cancelWaiter<string> template=async.promise.Promise.cancelWaiter arguments=(string)
/// @generic.instance id=async.promise.Promise.cancelWaiter<void> template=async.promise.Promise.cancelWaiter arguments=(void)
/// @generic.instance id=async.promise.Promise.cancelWaiters<string> template=async.promise.Promise.cancelWaiters arguments=(string)
/// @generic.instance id=async.promise.Promise.cancelWaiters<void> template=async.promise.Promise.cancelWaiters arguments=(void)
/// @generic.instance id=async.promise.Promise.forward<string> template=async.promise.Promise.forward arguments=(string)
/// @generic.instance id=async.promise.Promise.forward<void> template=async.promise.Promise.forward arguments=(void)
/// @generic.instance id=async.promise.Promise.fulfill<string> template=async.promise.Promise.fulfill arguments=(string)
/// @generic.instance id=async.promise.Promise.fulfill<void> template=async.promise.Promise.fulfill arguments=(void)
/// @generic.instance id=async.promise.Promise.observe<string> template=async.promise.Promise.observe arguments=(string)
/// @generic.instance id=async.promise.Promise.observe<void> template=async.promise.Promise.observe arguments=(void)
/// @generic.instance id=async.promise.Promise.pending<string> template=async.promise.Promise.pending arguments=(string)
/// @generic.instance id=async.promise.Promise.pending<void> template=async.promise.Promise.pending arguments=(void)
/// @generic.instance id=async.promise.Promise.queueWaiter<string> template=async.promise.Promise.queueWaiter arguments=(string)
/// @generic.instance id=async.promise.Promise.queueWaiter<void> template=async.promise.Promise.queueWaiter arguments=(void)
/// @generic.instance id=async.promise.Promise.queueWaiters<string> template=async.promise.Promise.queueWaiters arguments=(string)
/// @generic.instance id=async.promise.Promise.queueWaiters<void> template=async.promise.Promise.queueWaiters arguments=(void)
/// @generic.instance id=async.promise.Promise.symbol12<string> template=async.promise.Promise.symbol12 arguments=(string)
/// @generic.instance id=async.promise.Promise.symbol12<void> template=async.promise.Promise.symbol12 arguments=(void)
/// @generic.instance id=async.promise.PromiseAwaiter<string> template=async.promise.PromiseAwaiter arguments=(string)
/// @generic.instance id=async.promise.PromiseAwaiter<void> template=async.promise.PromiseAwaiter arguments=(void)
/// @generic.instance id=async.promise.PromiseForwarded<string> template=async.promise.PromiseForwarded arguments=(string)
/// @generic.instance id=async.promise.PromiseForwarded<void> template=async.promise.PromiseForwarded arguments=(void)
/// @generic.instance id=async.promise.PromiseFulfilled<string> template=async.promise.PromiseFulfilled arguments=(string)
/// @generic.instance id=async.promise.PromiseFulfilled<void> template=async.promise.PromiseFulfilled arguments=(void)
/// @generic.instance id=async.promise.PromisePending<string> template=async.promise.PromisePending arguments=(string)
/// @generic.instance id=async.promise.PromisePending<void> template=async.promise.PromisePending arguments=(void)
/// @generic.instance id=async.promise.PromiseReaction.symbol194<string> template=async.promise.PromiseReaction.symbol194 arguments=(string)
/// @generic.instance id=async.promise.PromiseReaction.symbol194<void> template=async.promise.PromiseReaction.symbol194 arguments=(void)
/// @generic.instance id=async.promise.PromiseReaction<string> template=async.promise.PromiseReaction arguments=(string)
/// @generic.instance id=async.promise.PromiseReaction<void> template=async.promise.PromiseReaction arguments=(void)
/// @generic.instance id=async.promise.PromiseState<string> template=async.promise.PromiseState arguments=(string)
/// @generic.instance id=async.promise.PromiseState<void> template=async.promise.PromiseState arguments=(void)
/// @generic.instance id=async.promise.PromiseWaiter<string> template=async.promise.PromiseWaiter arguments=(string)
/// @generic.instance id=async.promise.PromiseWaiter<void> template=async.promise.PromiseWaiter arguments=(void)

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
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Promise<string>' is not assignable to type 'string'"
/// @diagnostic.label line=5 column=20 span="promise" line_source="const bad: Value = promise;"
/// @diagnostic.related line=5 column=12 span="Value" line_source="const bad: Value = promise;" message="expected due to this annotation"
"#,
    );
}
