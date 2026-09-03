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
/// @resolution.call parameters=(^Function<(), Promise.create.T, "once">) arguments=(supplied as ^Function<(), Promise.create.T, "once">) return=Promise<Promise.create.T> kind=symbol target=Promise.create instance=Promise.create<int32>
/// @generic.instantiation id=Promise.create<int32> template=Promise.create arguments=(int32)
/// @generic.instance id="Function<(), int32, \"once\">" template=Function arguments=((), int32, "once")
/// @generic.instance id="Promise.symbol12<int32, \"local\">" template=Promise.symbol12 arguments=(int32, "local")
/// @generic.instance id="Promise.symbol12<void, \"local\">" template=Promise.symbol12 arguments=(void, "local")
/// @generic.instance id="Promise<int32, \"local\">" template=Promise arguments=(int32, "local")
/// @generic.instance id="Promise<void, \"local\">" template=Promise arguments=(void, "local")
/// @generic.instance id="PromiseReaction.symbol173<int32, \"local\">" template=PromiseReaction.symbol173 arguments=(int32, "local")
/// @generic.instance id="PromiseReaction.symbol173<void, \"local\">" template=PromiseReaction.symbol173 arguments=(void, "local")
/// @generic.instance id="PromiseReaction<int32, \"local\">" template=PromiseReaction arguments=(int32, "local")
/// @generic.instance id="PromiseReaction<void, \"local\">" template=PromiseReaction arguments=(void, "local")
/// @generic.instance id=Awaitable<int32> template=Awaitable arguments=(int32)
/// @generic.instance id=Awaitable<void> template=Awaitable arguments=(void)
/// @generic.instance id=Promise.addReaction<int32> template=Promise.addReaction arguments=(int32)
/// @generic.instance id=Promise.addReaction<void> template=Promise.addReaction arguments=(void)
/// @generic.instance id=Promise.addWaiter<int32> template=Promise.addWaiter arguments=(int32)
/// @generic.instance id=Promise.addWaiter<void> template=Promise.addWaiter arguments=(void)
/// @generic.instance id=Promise.create<int32> template=Promise.create arguments=(int32)
/// @generic.instance id=Promise.forward<int32> template=Promise.forward arguments=(int32)
/// @generic.instance id=Promise.forward<void> template=Promise.forward arguments=(void)
/// @generic.instance id=Promise.fulfill<int32> template=Promise.fulfill arguments=(int32)
/// @generic.instance id=Promise.fulfill<void> template=Promise.fulfill arguments=(void)
/// @generic.instance id=Promise.observe<int32> template=Promise.observe arguments=(int32)
/// @generic.instance id=Promise.observe<void> template=Promise.observe arguments=(void)
/// @generic.instance id=Promise.pending<int32> template=Promise.pending arguments=(int32)
/// @generic.instance id=Promise.pending<void> template=Promise.pending arguments=(void)
/// @generic.instance id=Promise.queueWaiter<int32> template=Promise.queueWaiter arguments=(int32)
/// @generic.instance id=Promise.queueWaiter<void> template=Promise.queueWaiter arguments=(void)
/// @generic.instance id=Promise.queueWaiters<int32> template=Promise.queueWaiters arguments=(int32)
/// @generic.instance id=Promise.queueWaiters<void> template=Promise.queueWaiters arguments=(void)
/// @generic.instance id=Promise<int32> template=Promise arguments=(int32)
/// @generic.instance id=Promise<void> template=Promise arguments=(void)
/// @generic.instance id=PromiseAwaiter<int32> template=PromiseAwaiter arguments=(int32)
/// @generic.instance id=PromiseAwaiter<void> template=PromiseAwaiter arguments=(void)
/// @generic.instance id=PromiseForwarded<int32> template=PromiseForwarded arguments=(int32)
/// @generic.instance id=PromiseForwarded<void> template=PromiseForwarded arguments=(void)
/// @generic.instance id=PromiseFulfilled<int32> template=PromiseFulfilled arguments=(int32)
/// @generic.instance id=PromiseFulfilled<void> template=PromiseFulfilled arguments=(void)
/// @generic.instance id=PromisePending<int32> template=PromisePending arguments=(int32)
/// @generic.instance id=PromisePending<void> template=PromisePending arguments=(void)
/// @generic.instance id=PromiseReaction<int32> template=PromiseReaction arguments=(int32)
/// @generic.instance id=PromiseReaction<void> template=PromiseReaction arguments=(void)
/// @generic.instance id=PromiseState<int32> template=PromiseState arguments=(int32)
/// @generic.instance id=PromiseState<void> template=PromiseState arguments=(void)
/// @generic.instance id=PromiseWaiter<int32> template=PromiseWaiter arguments=(int32)
/// @generic.instance id=PromiseWaiter<void> template=PromiseWaiter arguments=(void)
/// @resolution.name source=Promise target=Promise

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
/// @resolution.call parameters=(^Function<(), Promise.create.T, "once">) arguments=(supplied as ^Function<(), Promise.create.T, "once">) return=Promise<Promise.create.T> kind=symbol target=Promise.create instance=Promise.create<int32>
/// @generic.instantiation id=Promise.create<int32> template=Promise.create arguments=(int32)
/// @generic.instance id="Function<(), int32, \"once\">" template=Function arguments=((), int32, "once")
/// @generic.instance id="Promise.symbol12<int32, \"local\">" template=Promise.symbol12 arguments=(int32, "local")
/// @generic.instance id="Promise.symbol12<void, \"local\">" template=Promise.symbol12 arguments=(void, "local")
/// @generic.instance id="Promise<int32, \"local\">" template=Promise arguments=(int32, "local")
/// @generic.instance id="Promise<void, \"local\">" template=Promise arguments=(void, "local")
/// @generic.instance id="PromiseReaction.symbol173<int32, \"local\">" template=PromiseReaction.symbol173 arguments=(int32, "local")
/// @generic.instance id="PromiseReaction.symbol173<void, \"local\">" template=PromiseReaction.symbol173 arguments=(void, "local")
/// @generic.instance id="PromiseReaction<int32, \"local\">" template=PromiseReaction arguments=(int32, "local")
/// @generic.instance id="PromiseReaction<void, \"local\">" template=PromiseReaction arguments=(void, "local")
/// @generic.instance id=Awaitable<int32> template=Awaitable arguments=(int32)
/// @generic.instance id=Awaitable<void> template=Awaitable arguments=(void)
/// @generic.instance id=Promise.addReaction<int32> template=Promise.addReaction arguments=(int32)
/// @generic.instance id=Promise.addReaction<void> template=Promise.addReaction arguments=(void)
/// @generic.instance id=Promise.addWaiter<int32> template=Promise.addWaiter arguments=(int32)
/// @generic.instance id=Promise.addWaiter<void> template=Promise.addWaiter arguments=(void)
/// @generic.instance id=Promise.create<int32> template=Promise.create arguments=(int32)
/// @generic.instance id=Promise.forward<int32> template=Promise.forward arguments=(int32)
/// @generic.instance id=Promise.forward<void> template=Promise.forward arguments=(void)
/// @generic.instance id=Promise.fulfill<int32> template=Promise.fulfill arguments=(int32)
/// @generic.instance id=Promise.fulfill<void> template=Promise.fulfill arguments=(void)
/// @generic.instance id=Promise.observe<int32> template=Promise.observe arguments=(int32)
/// @generic.instance id=Promise.observe<void> template=Promise.observe arguments=(void)
/// @generic.instance id=Promise.pending<int32> template=Promise.pending arguments=(int32)
/// @generic.instance id=Promise.pending<void> template=Promise.pending arguments=(void)
/// @generic.instance id=Promise.queueWaiter<int32> template=Promise.queueWaiter arguments=(int32)
/// @generic.instance id=Promise.queueWaiter<void> template=Promise.queueWaiter arguments=(void)
/// @generic.instance id=Promise.queueWaiters<int32> template=Promise.queueWaiters arguments=(int32)
/// @generic.instance id=Promise.queueWaiters<void> template=Promise.queueWaiters arguments=(void)
/// @generic.instance id=Promise<int32> template=Promise arguments=(int32)
/// @generic.instance id=Promise<void> template=Promise arguments=(void)
/// @generic.instance id=PromiseAwaiter<void> template=PromiseAwaiter arguments=(void)
/// @generic.instance id=PromiseForwarded<int32> template=PromiseForwarded arguments=(int32)
/// @generic.instance id=PromiseForwarded<void> template=PromiseForwarded arguments=(void)
/// @generic.instance id=PromiseFulfilled<int32> template=PromiseFulfilled arguments=(int32)
/// @generic.instance id=PromiseFulfilled<void> template=PromiseFulfilled arguments=(void)
/// @generic.instance id=PromisePending<int32> template=PromisePending arguments=(int32)
/// @generic.instance id=PromisePending<void> template=PromisePending arguments=(void)
/// @generic.instance id=PromiseReaction<int32> template=PromiseReaction arguments=(int32)
/// @generic.instance id=PromiseReaction<void> template=PromiseReaction arguments=(void)
/// @generic.instance id=PromiseState<int32> template=PromiseState arguments=(int32)
/// @generic.instance id=PromiseState<void> template=PromiseState arguments=(void)
/// @generic.instance id=PromiseWaiter<int32> template=PromiseWaiter arguments=(int32)
/// @generic.instance id=PromiseWaiter<void> template=PromiseWaiter arguments=(void)
/// @resolution.name source=Promise target=Promise

    return 1;
}

async function double(): Promise<int32> {
/// @type.symbol symbol=double type=async () => Promise<int32>
/// @resolution.call parameters=(^Function<(), Promise.create.T, "once">) arguments=(supplied as ^Function<(), Promise.create.T, "once">) return=Promise<Promise.create.T> kind=symbol target=Promise.create instance=Promise.create<int32>
/// @resolution.name source=Promise target=Promise

    const count = await fetchCount();
    /// @type.symbol symbol=double.count source=count type=int32
    /// @resolution.pattern source=count kind=binding target=double.count
    /// @resolution.call source="await fetchCount()" parameters=(Promise<int32>) arguments=(provided(fetchCount()) as Promise<int32>) return=int32 kind=symbol target=Promise.park receiver=Promise<int32> instance=Promise<int32>.park<int32>
    /// @generic.instantiation id="Promise.park<int32, int32>" template=Promise.park arguments=(int32, int32)
    /// @generic.instance id="Promise.park<int32, int32>" template=Promise.park arguments=(int32, int32)
    /// @generic.instance id="PromiseAwaiter.symbol161<int32, \"local\">" template=PromiseAwaiter.symbol161 arguments=(int32, "local")
    /// @generic.instance id="PromiseAwaiter<int32, \"local\">" template=PromiseAwaiter arguments=(int32, "local")
    /// @generic.instance id=PromiseAwaiter<int32> template=PromiseAwaiter arguments=(int32)
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
/// @resolution.call parameters=(^Function<(AsyncGeneratorProducer<AsyncGenerator.create.Y, AsyncGenerator.create.R, AsyncGenerator.create.N>,), AsyncGenerator.create.R, "once">) arguments=(supplied as ^Function<(AsyncGeneratorProducer<AsyncGenerator.create.Y, AsyncGenerator.create.R, AsyncGenerator.create.N>,), AsyncGenerator.create.R, "once">) return=AsyncGenerator<AsyncGenerator.create.Y, AsyncGenerator.create.R, AsyncGenerator.create.N> kind=symbol target=AsyncGenerator.create instance="AsyncGenerator.create<int32, void, void>"
/// @generic.instantiation id="AsyncGenerator.create<int32, void, void>" template=AsyncGenerator.create arguments=(int32, void, void)
/// @resolution.name source=AsyncGenerator target=AsyncGenerator

    yield 1;
    /// @resolution.call source="yield 1" parameters=(int32) arguments=(provided(1) as int32) return=GeneratorRequest<void, void> kind=symbol target=AsyncGeneratorProducer.yield instance="AsyncGeneratorProducer.yield<\"local\">"
    /// @generic.instantiation id="AsyncGeneratorProducer.yield<\"local\", int32, void, void>" template=AsyncGeneratorProducer.yield arguments=("local", int32, void, void)

}

async function sum(): Promise<int32> {
/// @type.symbol symbol=sum type=async () => Promise<int32>
/// @resolution.call parameters=(^Function<(), Promise.create.T, "once">) arguments=(supplied as ^Function<(), Promise.create.T, "once">) return=Promise<Promise.create.T> kind=symbol target=Promise.create instance=Promise.create<int32>
/// @generic.instantiation id=Promise.create<int32> template=Promise.create arguments=(int32)
/// @resolution.name source=Promise target=Promise

    let total: int32 = 0;
    /// @type.symbol symbol=sum.total source=total type=int32
    /// @resolution.pattern source=total kind=binding target=sum.total

    for await (const value of stream()) {
    /// @resolution.iteration iterator="asyncIterator(parameters=(), arguments=(), return=AsyncGenerator<int32, void, void>)" next="next#2(parameters=(), arguments=(), return=Promise<IteratorResult<int32, void>>)" await="Promise.park(parameters=(Promise<IteratorResult<int32, void>>), arguments=(supplied as Promise<IteratorResult<int32, void>>), return=IteratorResult<int32, void>)" awaits=result
    /// @generic.instantiation id="Promise.park<IteratorResult<int32, void>, IteratorResult<int32, void>>" template=Promise.park arguments=(IteratorResult<int32, void>, IteratorResult<int32, void>)
    /// @generic.instantiation id="asyncIterator<int32, AsyncGenerator<int32, void, void>>" template=asyncIterator arguments=(int32, AsyncGenerator<int32, void, void>)
    /// @generic.instantiation id="next#2<int32, void, void, \"local\">" template=next#2 arguments=(int32, void, void, "local")
    /// @type.symbol symbol=sum.value source=value type=int32
    /// @resolution.pattern source=value kind=binding target=sum.value
    /// @resolution.name source=stream target=stream
    /// @resolution.call source=stream() parameters=() return=AsyncGenerator<int32, void, void> kind=symbol target=stream

        total += value;
        /// @resolution.name source=total target=sum.total
        /// @resolution.operator source="total += value" type=int32 operator="+" kind=builtin operands=[total as int32 families=(integer), value as int32 families=(integer)]
        /// @resolution.pattern.assign source=total kind=place
        /// @resolution.place source=total placement="local" lifetime="frame" access="exclusive"
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

"#);
}
