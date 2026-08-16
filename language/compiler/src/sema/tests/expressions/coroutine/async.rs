use crate::tests::{DirRows, TestSession};

#[test]
fn test_type_async_function_results_as_promises() {
    let session = TestSession::single(
        r#"
async function fetchCount(): Promise<int32> {
    return 1;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
async function fetchCount(): Promise<int32> {
    return 1;
}

=== dir ===
async function fetchCount(): Promise<int32> {
/// @type.symbol symbol=fetchCount type=async () => Promise<int32>
/// @generic.instance id=Promise<int32> template=async.promise.Promise arguments=(int32)
/// @generic.instance id=Promise<void> template=async.promise.Promise arguments=(void)
/// @generic.instance id=async.awaitable.Awaitable<int32> template=async.awaitable.Awaitable arguments=(int32)
/// @generic.instance id=async.awaitable.Awaitable<void> template=async.awaitable.Awaitable arguments=(void)
/// @generic.instance id=async.fiber.Fiber.wake<int32> template=async.fiber.Fiber.wake arguments=(int32)
/// @generic.instance id=async.fiber.Fiber.wake<void> template=async.fiber.Fiber.wake arguments=(void)
/// @generic.instance id=async.fiber.wakeFiber<int32> template=async.fiber.wakeFiber arguments=(int32)
/// @generic.instance id=async.fiber.wakeFiber<void> template=async.fiber.wakeFiber arguments=(void)
/// @generic.instance id=async.promise.Promise.addReaction<int32> template=async.promise.Promise.addReaction arguments=(int32)
/// @generic.instance id=async.promise.Promise.addReaction<void> template=async.promise.Promise.addReaction arguments=(void)
/// @generic.instance id=async.promise.Promise.addWaiter<int32> template=async.promise.Promise.addWaiter arguments=(int32)
/// @generic.instance id=async.promise.Promise.addWaiter<void> template=async.promise.Promise.addWaiter arguments=(void)
/// @generic.instance id=async.promise.Promise.cancel<int32> template=async.promise.Promise.cancel arguments=(int32)
/// @generic.instance id=async.promise.Promise.cancel<void> template=async.promise.Promise.cancel arguments=(void)
/// @generic.instance id=async.promise.Promise.cancelWaiter<int32> template=async.promise.Promise.cancelWaiter arguments=(int32)
/// @generic.instance id=async.promise.Promise.cancelWaiter<void> template=async.promise.Promise.cancelWaiter arguments=(void)
/// @generic.instance id=async.promise.Promise.cancelWaiters<int32> template=async.promise.Promise.cancelWaiters arguments=(int32)
/// @generic.instance id=async.promise.Promise.cancelWaiters<void> template=async.promise.Promise.cancelWaiters arguments=(void)
/// @generic.instance id=async.promise.Promise.forward<int32> template=async.promise.Promise.forward arguments=(int32)
/// @generic.instance id=async.promise.Promise.forward<void> template=async.promise.Promise.forward arguments=(void)
/// @generic.instance id=async.promise.Promise.fulfill<int32> template=async.promise.Promise.fulfill arguments=(int32)
/// @generic.instance id=async.promise.Promise.fulfill<void> template=async.promise.Promise.fulfill arguments=(void)
/// @generic.instance id=async.promise.Promise.observe<int32> template=async.promise.Promise.observe arguments=(int32)
/// @generic.instance id=async.promise.Promise.observe<void> template=async.promise.Promise.observe arguments=(void)
/// @generic.instance id=async.promise.Promise.pending<int32> template=async.promise.Promise.pending arguments=(int32)
/// @generic.instance id=async.promise.Promise.pending<void> template=async.promise.Promise.pending arguments=(void)
/// @generic.instance id=async.promise.Promise.queueWaiter<int32> template=async.promise.Promise.queueWaiter arguments=(int32)
/// @generic.instance id=async.promise.Promise.queueWaiter<void> template=async.promise.Promise.queueWaiter arguments=(void)
/// @generic.instance id=async.promise.Promise.queueWaiters<int32> template=async.promise.Promise.queueWaiters arguments=(int32)
/// @generic.instance id=async.promise.Promise.queueWaiters<void> template=async.promise.Promise.queueWaiters arguments=(void)
/// @generic.instance id=async.promise.Promise.symbol12<int32> template=async.promise.Promise.symbol12 arguments=(int32)
/// @generic.instance id=async.promise.Promise.symbol12<void> template=async.promise.Promise.symbol12 arguments=(void)
/// @generic.instance id=async.promise.PromiseAwaiter<int32> template=async.promise.PromiseAwaiter arguments=(int32)
/// @generic.instance id=async.promise.PromiseAwaiter<void> template=async.promise.PromiseAwaiter arguments=(void)
/// @generic.instance id=async.promise.PromiseForwarded<int32> template=async.promise.PromiseForwarded arguments=(int32)
/// @generic.instance id=async.promise.PromiseForwarded<void> template=async.promise.PromiseForwarded arguments=(void)
/// @generic.instance id=async.promise.PromiseFulfilled<int32> template=async.promise.PromiseFulfilled arguments=(int32)
/// @generic.instance id=async.promise.PromiseFulfilled<void> template=async.promise.PromiseFulfilled arguments=(void)
/// @generic.instance id=async.promise.PromisePending<int32> template=async.promise.PromisePending arguments=(int32)
/// @generic.instance id=async.promise.PromisePending<void> template=async.promise.PromisePending arguments=(void)
/// @generic.instance id=async.promise.PromiseReaction.symbol194<int32> template=async.promise.PromiseReaction.symbol194 arguments=(int32)
/// @generic.instance id=async.promise.PromiseReaction.symbol194<void> template=async.promise.PromiseReaction.symbol194 arguments=(void)
/// @generic.instance id=async.promise.PromiseReaction<int32> template=async.promise.PromiseReaction arguments=(int32)
/// @generic.instance id=async.promise.PromiseReaction<void> template=async.promise.PromiseReaction arguments=(void)
/// @generic.instance id=async.promise.PromiseState<int32> template=async.promise.PromiseState arguments=(int32)
/// @generic.instance id=async.promise.PromiseState<void> template=async.promise.PromiseState arguments=(void)
/// @generic.instance id=async.promise.PromiseWaiter<int32> template=async.promise.PromiseWaiter arguments=(int32)
/// @generic.instance id=async.promise.PromiseWaiter<void> template=async.promise.PromiseWaiter arguments=(void)
/// @resolution.name source=Promise target=async.promise.Promise

    return 1;
}
"#,
    );
}

#[test]
fn test_unwrap_awaited_promise_values() {
    let session = TestSession::single(
        r#"
async function fetchCount(): Promise<int32> {
    return 1;
}

async function double(): Promise<int32> {
    const count = await fetchCount();

    return count + count;
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
async function fetchCount(): Promise<int32> {
    return 1;
}

async function double(): Promise<int32> {
    const count: int32 = await fetchCount();

    return count + count;
}

=== dir ===
async function fetchCount(): Promise<int32> {
/// @type.symbol symbol=fetchCount type=async () => Promise<int32>
/// @generic.instance id=Promise<int32> template=async.promise.Promise arguments=(int32)
/// @generic.instance id=Promise<void> template=async.promise.Promise arguments=(void)
/// @generic.instance id=async.awaitable.Awaitable<int32> template=async.awaitable.Awaitable arguments=(int32)
/// @generic.instance id=async.awaitable.Awaitable<void> template=async.awaitable.Awaitable arguments=(void)
/// @generic.instance id=async.fiber.Fiber.wake<int32> template=async.fiber.Fiber.wake arguments=(int32)
/// @generic.instance id=async.fiber.Fiber.wake<void> template=async.fiber.Fiber.wake arguments=(void)
/// @generic.instance id=async.fiber.wakeFiber<int32> template=async.fiber.wakeFiber arguments=(int32)
/// @generic.instance id=async.fiber.wakeFiber<void> template=async.fiber.wakeFiber arguments=(void)
/// @generic.instance id=async.promise.Promise.addReaction<int32> template=async.promise.Promise.addReaction arguments=(int32)
/// @generic.instance id=async.promise.Promise.addReaction<void> template=async.promise.Promise.addReaction arguments=(void)
/// @generic.instance id=async.promise.Promise.addWaiter<int32> template=async.promise.Promise.addWaiter arguments=(int32)
/// @generic.instance id=async.promise.Promise.addWaiter<void> template=async.promise.Promise.addWaiter arguments=(void)
/// @generic.instance id=async.promise.Promise.cancel<int32> template=async.promise.Promise.cancel arguments=(int32)
/// @generic.instance id=async.promise.Promise.cancel<void> template=async.promise.Promise.cancel arguments=(void)
/// @generic.instance id=async.promise.Promise.cancelWaiter<int32> template=async.promise.Promise.cancelWaiter arguments=(int32)
/// @generic.instance id=async.promise.Promise.cancelWaiter<void> template=async.promise.Promise.cancelWaiter arguments=(void)
/// @generic.instance id=async.promise.Promise.cancelWaiters<int32> template=async.promise.Promise.cancelWaiters arguments=(int32)
/// @generic.instance id=async.promise.Promise.cancelWaiters<void> template=async.promise.Promise.cancelWaiters arguments=(void)
/// @generic.instance id=async.promise.Promise.forward<int32> template=async.promise.Promise.forward arguments=(int32)
/// @generic.instance id=async.promise.Promise.forward<void> template=async.promise.Promise.forward arguments=(void)
/// @generic.instance id=async.promise.Promise.fulfill<int32> template=async.promise.Promise.fulfill arguments=(int32)
/// @generic.instance id=async.promise.Promise.fulfill<void> template=async.promise.Promise.fulfill arguments=(void)
/// @generic.instance id=async.promise.Promise.observe<int32> template=async.promise.Promise.observe arguments=(int32)
/// @generic.instance id=async.promise.Promise.observe<void> template=async.promise.Promise.observe arguments=(void)
/// @generic.instance id=async.promise.Promise.pending<int32> template=async.promise.Promise.pending arguments=(int32)
/// @generic.instance id=async.promise.Promise.pending<void> template=async.promise.Promise.pending arguments=(void)
/// @generic.instance id=async.promise.Promise.queueWaiter<int32> template=async.promise.Promise.queueWaiter arguments=(int32)
/// @generic.instance id=async.promise.Promise.queueWaiter<void> template=async.promise.Promise.queueWaiter arguments=(void)
/// @generic.instance id=async.promise.Promise.queueWaiters<int32> template=async.promise.Promise.queueWaiters arguments=(int32)
/// @generic.instance id=async.promise.Promise.queueWaiters<void> template=async.promise.Promise.queueWaiters arguments=(void)
/// @generic.instance id=async.promise.Promise.symbol12<int32> template=async.promise.Promise.symbol12 arguments=(int32)
/// @generic.instance id=async.promise.Promise.symbol12<void> template=async.promise.Promise.symbol12 arguments=(void)
/// @generic.instance id=async.promise.PromiseAwaiter<int32> template=async.promise.PromiseAwaiter arguments=(int32)
/// @generic.instance id=async.promise.PromiseAwaiter<void> template=async.promise.PromiseAwaiter arguments=(void)
/// @generic.instance id=async.promise.PromiseForwarded<int32> template=async.promise.PromiseForwarded arguments=(int32)
/// @generic.instance id=async.promise.PromiseForwarded<void> template=async.promise.PromiseForwarded arguments=(void)
/// @generic.instance id=async.promise.PromiseFulfilled<int32> template=async.promise.PromiseFulfilled arguments=(int32)
/// @generic.instance id=async.promise.PromiseFulfilled<void> template=async.promise.PromiseFulfilled arguments=(void)
/// @generic.instance id=async.promise.PromisePending<int32> template=async.promise.PromisePending arguments=(int32)
/// @generic.instance id=async.promise.PromisePending<void> template=async.promise.PromisePending arguments=(void)
/// @generic.instance id=async.promise.PromiseReaction.symbol194<int32> template=async.promise.PromiseReaction.symbol194 arguments=(int32)
/// @generic.instance id=async.promise.PromiseReaction.symbol194<void> template=async.promise.PromiseReaction.symbol194 arguments=(void)
/// @generic.instance id=async.promise.PromiseReaction<int32> template=async.promise.PromiseReaction arguments=(int32)
/// @generic.instance id=async.promise.PromiseReaction<void> template=async.promise.PromiseReaction arguments=(void)
/// @generic.instance id=async.promise.PromiseState<int32> template=async.promise.PromiseState arguments=(int32)
/// @generic.instance id=async.promise.PromiseState<void> template=async.promise.PromiseState arguments=(void)
/// @generic.instance id=async.promise.PromiseWaiter<int32> template=async.promise.PromiseWaiter arguments=(int32)
/// @generic.instance id=async.promise.PromiseWaiter<void> template=async.promise.PromiseWaiter arguments=(void)
/// @resolution.name source=Promise target=async.promise.Promise

    return 1;
}

async function double(): Promise<int32> {
/// @type.symbol symbol=double type=async () => Promise<int32>
/// @resolution.name source=Promise target=async.promise.Promise

    const count = await fetchCount();
    /// @type.symbol symbol=double.count source=count type=int32
    /// @resolution.pattern source=count kind=binding target=double.count
    /// @resolution.name source=fetchCount target=fetchCount
    /// @resolution.call source=fetchCount() parameters=() return=Promise<int32> kind=symbol target=fetchCount

    return count + count;
    /// @resolution.name source=count target=double.count
    /// @resolution.operator source="count + count" type=int32 operator="+" kind=builtin operands=[count as int32 families=(integer), count as int32 families=(integer)]
    /// @resolution.place source=count placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=count root=double.count
    /// @resolution.name source=count target=double.count
    /// @resolution.place source=count placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=count root=double.count

}
"#);
}

#[test]
fn test_iterate_async_sequences_with_for_await() {
    let session = TestSession::single(
        r#"
async function* stream(): AsyncGenerator<int32, void, void> {
    yield 1;
}

async function sum(): Promise<int32> {
    let total: int32 = 0;
    for await (const value of stream()) {
        total += value;
    }

    return total;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
async function* stream(): AsyncGenerator<int32, void, void> {
    yield 1;
}

async function sum(): Promise<int32> {
    let total: int32 = 0;
    for await (const value of stream()) {
        total += value;
    }

    return total;
}

=== dir ===
async function* stream(): AsyncGenerator<int32, void, void> {
/// @type.symbol symbol=stream type=async () => *AsyncGenerator<int32, void, void>
/// @resolution.name source=AsyncGenerator target=async.generator.AsyncGenerator

    yield 1;
}

async function sum(): Promise<int32> {
/// @type.symbol symbol=sum type=async () => Promise<int32>
/// @resolution.name source=Promise target=async.promise.Promise

    let total: int32 = 0;
    /// @type.symbol symbol=sum.total source=total type=int32
    /// @resolution.pattern source=total kind=binding target=sum.total

    for await (const value of stream()) {
    /// @type.symbol symbol=sum.value source=value type=<error>
    /// @resolution.pattern source=value kind=binding target=sum.value
    /// @resolution.name source=stream target=stream
    /// @resolution.call source=stream() parameters=() return=AsyncGenerator<int32, void, void> kind=symbol target=stream

        total += value;
        /// @resolution.name source=total target=sum.total
        /// @resolution.poisoned source="total += value"
        /// @resolution.pattern.assign source=total kind=place
        /// @resolution.assignment source=total read=binding(sum.total) write=binding(sum.total) type=int32
        /// @resolution.access source=total root=sum.total
        /// @resolution.name source=value target=sum.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=value root=sum.value

    }

    return total;
    /// @resolution.name source=total target=sum.total
    /// @resolution.place source=total placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=total root=sum.total

}
"#, r#"
/// @diagnostic.error id=for-of-source-not-iterable message="for-of source must be iterable"
/// @diagnostic.label line=8 column=5 span="for" line_source="for await (const value of stream()) {"
"#);
}
