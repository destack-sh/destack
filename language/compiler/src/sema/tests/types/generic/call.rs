use crate::tests::{DirRows, TestSession};

#[test]
fn test_infer_method_type_argument_from_callback_result() {
    let session = TestSession::single(
        r#"
import { Result } from "destack:error";

declare function first(): Result<int32, string>;
declare function second(value: int32): Result<boolean, string>;

const result: Result<boolean, string> = first().andThen((value) => second(value));
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Result } from "destack:error";

declare function first(): Result<int32, string>;
declare function second(value: int32): Result<boolean, string>;

const result: Result<boolean, string> = first().andThen<int32, string, boolean, string>(
    (value: int32): Result<boolean, string> => second(value),
);

=== dir ===
import { Result } from "destack:error";

declare function first(): Result<int32, string>;
/// @type.symbol symbol=first source="declare function first(): Result<int32, string>" type=() => Result<int32, string>
/// @generic.instance id="Result<int32, string>" template=Result arguments=(int32, string)
/// @generic.instance id=Err<string> template=Err arguments=(string)
/// @generic.instance id=Ok<int32> template=Ok arguments=(int32)
/// @resolution.name source=Result target=Result

declare function second(value: int32): Result<boolean, string>;
/// @type.symbol symbol=second source="declare function second(value: int32): Result<boolean, string>" type=(int32) => Result<boolean, string>
/// @generic.instance id="Result<boolean, string>" template=Result arguments=(boolean, string)
/// @generic.instance id=Ok<boolean> template=Ok arguments=(boolean)
/// @resolution.name source=Result target=Result

const result: Result<boolean, string> = first().andThen((value) => second(value));
/// @type.symbol symbol=result source=result type=Result<boolean, string>
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Result target=Result
/// @type.node source="first().andThen((value) => second(value))" type=Result<boolean, string>
/// @type.node source=first type=() => Result<int32, string>
/// @type.node source=first() type=Result<int32, string>
/// @type.node source=first().andThen type=<andThen.U, andThen.F>(this: Result<int32, string>, (int32) => Result<andThen.U, andThen.F>) => Result<andThen.U, string | andThen.F>
/// @resolution.name source=first target=first
/// @resolution.member source=first().andThen receiver=Result<int32, string> type=<andThen.U, andThen.F>(this: Result<int32, string>, (int32) => Result<andThen.U, andThen.F>) => Result<andThen.U, string | andThen.F> kind=symbol target_receiver=Result<int32, string> target=andThen
/// @resolution.call source="first().andThen((value) => second(value))" parameters=((int32) => Result<boolean, string>) arguments=(provided((value) => second(value)) as (int32) => Result<boolean, string>) return=Result<boolean, string> kind=symbol target=andThen receiver=Result<int32, string> instance="Result<int32, string>.<extension#1>.andThen<boolean, string>"
/// @resolution.call source=first() parameters=() return=Result<int32, string> kind=symbol target=first
/// @generic.instantiation id="andThen<int32, string, boolean, string>" template=andThen arguments=(int32, string, boolean, string)
/// @generic.instantiation id="andThen<int32, string>" template=andThen arguments=(int32, string)
/// @generic.instance id="andThen<int32, string, boolean, string>" template=andThen arguments=(int32, string, boolean, string)
/// @generic.instance id="err#1<boolean, string>" template=err#1 arguments=(boolean, string)
/// @generic.instance id="ok#1<boolean, string>" template=ok#1 arguments=(boolean, string)
/// @type.symbol symbol=symbol5 source=(value) => second(value) type=Function<(int32,), Result<boolean, string>, "readonly">
/// @type.node source=(value) => second(value) type=Function<(int32,), Result<boolean, string>, "readonly">
/// @type.symbol symbol=symbol5.value source=value type=int32
/// @type.node source=second type=(int32) => Result<boolean, string>
/// @type.node source=second(value) type=Result<boolean, string>
/// @resolution.name source=second target=second
/// @resolution.call source=second(value) parameters=(int32) arguments=(provided(value) as int32) return=Result<boolean, string> kind=symbol target=second
/// @type.node source=value type=int32
/// @resolution.name source=value target=symbol5.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol5.value
"#,
    );
}

#[test]
fn test_call_selects_value_returning_promise_overload() {
    let session = TestSession::single(
        r#"
import { Promise } from "destack:async";

declare const input: Promise<int32>;
const result: Promise<string> = input.then(() => "done");
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
const result: Promise<string> = input.then<int32, string>((): string => "done");

=== dir ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
/// @type.symbol symbol=input source=input type=Promise<int32>
/// @resolution.pattern source=input kind=binding target=input
/// @generic.instance id=Promise<int32> template=Promise arguments=(int32)
/// @resolution.name source=Promise target=Promise

const result: Promise<string> = input.then(() => "done");
/// @type.symbol symbol=result source=result type=Promise<string>
/// @resolution.pattern source=result kind=binding target=result
/// @generic.instance id=Promise<string> template=Promise arguments=(string)
/// @resolution.name source=Promise target=Promise
/// @type.node source="input.then(() => \"done\")" type=Promise<string>
/// @type.node source=input type=Promise<int32>
/// @type.node source=input.then type=<Promise.then.U#1: Copy>(this: Promise<int32>, (int32) => Promise<Promise.then.U#1>) => Promise<Promise.then.U#1> & <Promise.then.U#2: Copy>(this: Promise<int32>, (int32) => Promise.then.U#2) => Promise<Promise.then.U#2>
/// @resolution.name source=input target=input
/// @resolution.member source=input.then receiver=Promise<int32> type=<Promise.then.U#1: Copy>(this: Promise<int32>, (int32) => Promise<Promise.then.U#1>) => Promise<Promise.then.U#1> & <Promise.then.U#2: Copy>(this: Promise<int32>, (int32) => Promise.then.U#2) => Promise<Promise.then.U#2> kind=overload-set targets=[Promise.then#1, Promise.then#2]
/// @resolution.call source="input.then(() => \"done\")" parameters=((int32) => string) arguments=(provided(() => "done") as (int32) => string) return=Promise<string> kind=symbol target=Promise.then#2 receiver=Promise<int32> instance=Promise<int32>.then#2<string>
/// @resolution.place source=input placement="local" lifetime="static" access="immutable"
/// @resolution.access source=input root=input
/// @generic.instantiation id="Promise.then#2<int32, string>" template=Promise.then#2 arguments=(int32, string)
/// @generic.instantiation id=Promise.then#1<int32> template=Promise.then#1 arguments=(int32)
/// @generic.instantiation id=Promise.then#2<int32> template=Promise.then#2 arguments=(int32)
/// @generic.instance id="Promise.then#2<int32, string>" template=Promise.then#2 arguments=(int32, string)
/// @generic.instance id=Promise.addReaction<int32> template=Promise.addReaction arguments=(int32)
/// @generic.instance id=Promise.addWaiter<int32> template=Promise.addWaiter arguments=(int32)
/// @generic.instance id=Promise.fulfill<string> template=Promise.fulfill arguments=(string)
/// @generic.instance id=Promise.observe<int32> template=Promise.observe arguments=(int32)
/// @generic.instance id=Promise.pending<string> template=Promise.pending arguments=(string)
/// @generic.instance id=Promise.queueWaiter<int32> template=Promise.queueWaiter arguments=(int32)
/// @generic.instance id=Promise.queueWaiter<string> template=Promise.queueWaiter arguments=(string)
/// @generic.instance id=Promise.queueWaiters<string> template=Promise.queueWaiters arguments=(string)
/// @generic.instance id=Promise.symbol12<string> template=Promise.symbol12 arguments=(string)
/// @generic.instance id=PromiseForwarded<int32> template=PromiseForwarded arguments=(int32)
/// @generic.instance id=PromiseFulfilled<int32> template=PromiseFulfilled arguments=(int32)
/// @generic.instance id=PromiseReaction.symbol173<int32> template=PromiseReaction.symbol173 arguments=(int32)
/// @generic.instance id=PromiseReaction<int32> template=PromiseReaction arguments=(int32)
/// @type.symbol symbol=symbol3 source="() => \"done\"" type=Function<(), string, "readonly">
/// @type.node source="() => \"done\"" type=Function<(), string, "readonly">
/// @type.node source="\"done\"" type="done"
"#,
    );
}

#[test]
fn test_call_selects_promise_returning_overload() {
    let session = TestSession::single(
        r#"
import { Promise } from "destack:async";

declare const input: Promise<int32>;
declare const next: Promise<string>;
const result: Promise<string> = input.then(() => next);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
declare const next: Promise<string>;
const result: Promise<string> = input.then<int32, string>((): Promise<string> => next);

=== dir ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
/// @type.symbol symbol=input source=input type=Promise<int32>
/// @resolution.pattern source=input kind=binding target=input
/// @generic.instance id=Promise<int32> template=Promise arguments=(int32)
/// @resolution.name source=Promise target=Promise

declare const next: Promise<string>;
/// @type.symbol symbol=next source=next type=Promise<string>
/// @resolution.pattern source=next kind=binding target=next
/// @generic.instance id=Promise<string> template=Promise arguments=(string)
/// @resolution.name source=Promise target=Promise

const result: Promise<string> = input.then(() => next);
/// @type.symbol symbol=result source=result type=Promise<string>
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Promise target=Promise
/// @type.node source="input.then(() => next)" type=Promise<string>
/// @type.node source=input type=Promise<int32>
/// @type.node source=input.then type=<Promise.then.U#1: Copy>(this: Promise<int32>, (int32) => Promise<Promise.then.U#1>) => Promise<Promise.then.U#1> & <Promise.then.U#2: Copy>(this: Promise<int32>, (int32) => Promise.then.U#2) => Promise<Promise.then.U#2>
/// @resolution.name source=input target=input
/// @resolution.member source=input.then receiver=Promise<int32> type=<Promise.then.U#1: Copy>(this: Promise<int32>, (int32) => Promise<Promise.then.U#1>) => Promise<Promise.then.U#1> & <Promise.then.U#2: Copy>(this: Promise<int32>, (int32) => Promise.then.U#2) => Promise<Promise.then.U#2> kind=overload-set targets=[Promise.then#1, Promise.then#2]
/// @resolution.call source="input.then(() => next)" parameters=((int32) => Promise<string>) arguments=(provided(() => next) as (int32) => Promise<string>) return=Promise<string> kind=symbol target=Promise.then#1 receiver=Promise<int32> instance=Promise<int32>.then#1<string>
/// @resolution.place source=input placement="local" lifetime="static" access="immutable"
/// @resolution.access source=input root=input
/// @generic.instantiation id="Promise.then#1<int32, string>" template=Promise.then#1 arguments=(int32, string)
/// @generic.instantiation id=Promise.then#1<int32> template=Promise.then#1 arguments=(int32)
/// @generic.instantiation id=Promise.then#2<int32> template=Promise.then#2 arguments=(int32)
/// @generic.instance id="Promise.then#1<int32, string>" template=Promise.then#1 arguments=(int32, string)
/// @generic.instance id=Promise.addReaction<int32> template=Promise.addReaction arguments=(int32)
/// @generic.instance id=Promise.addReaction<string> template=Promise.addReaction arguments=(string)
/// @generic.instance id=Promise.addWaiter<int32> template=Promise.addWaiter arguments=(int32)
/// @generic.instance id=Promise.addWaiter<string> template=Promise.addWaiter arguments=(string)
/// @generic.instance id=Promise.forward<string> template=Promise.forward arguments=(string)
/// @generic.instance id=Promise.fulfill<string> template=Promise.fulfill arguments=(string)
/// @generic.instance id=Promise.observe<int32> template=Promise.observe arguments=(int32)
/// @generic.instance id=Promise.pending<string> template=Promise.pending arguments=(string)
/// @generic.instance id=Promise.queueWaiter<int32> template=Promise.queueWaiter arguments=(int32)
/// @generic.instance id=Promise.queueWaiter<string> template=Promise.queueWaiter arguments=(string)
/// @generic.instance id=Promise.queueWaiters<string> template=Promise.queueWaiters arguments=(string)
/// @generic.instance id=Promise.symbol12<string> template=Promise.symbol12 arguments=(string)
/// @generic.instance id=PromiseForwarded<int32> template=PromiseForwarded arguments=(int32)
/// @generic.instance id=PromiseFulfilled<int32> template=PromiseFulfilled arguments=(int32)
/// @generic.instance id=PromiseReaction.symbol173<int32> template=PromiseReaction.symbol173 arguments=(int32)
/// @generic.instance id=PromiseReaction.symbol173<string> template=PromiseReaction.symbol173 arguments=(string)
/// @generic.instance id=PromiseReaction<int32> template=PromiseReaction arguments=(int32)
/// @generic.instance id=PromiseReaction<string> template=PromiseReaction arguments=(string)
/// @type.symbol symbol=symbol4 source="() => next" type=Function<(), Promise<string>, "readonly">
/// @type.node source="() => next" type=Function<(), Promise<string>, "readonly">
/// @type.node source=next type=Promise<string>
/// @resolution.name source=next target=next
/// @resolution.place source=next placement="local" lifetime="static" access="immutable"
/// @resolution.access source=next root=next
"#,
    );
}

#[test]
fn test_call_contextualizes_nested_promise_overloads() {
    let session = TestSession::single(
        r#"
import { Promise } from "destack:async";

declare const input: Promise<int32>;
const result: Promise<string> = input.then((value) => {
    input.then(() => "done")
});
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
const result: Promise<string> = input.then<int32, string>((value: int32): Promise<string> => {
    input.then<int32, string>((): string => "done")
});

=== dir ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
/// @type.symbol symbol=input source=input type=Promise<int32>
/// @resolution.pattern source=input kind=binding target=input
/// @generic.instance id=Promise<int32> template=Promise arguments=(int32)
/// @resolution.name source=Promise target=Promise

const result: Promise<string> = input.then((value) => {
/// @type.symbol symbol=result source=result type=Promise<string>
/// @resolution.pattern source=result kind=binding target=result
/// @generic.instance id=Promise<string> template=Promise arguments=(string)
/// @resolution.name source=Promise target=Promise
/// @type.node source=input type=Promise<int32>
/// @type.node source=input.then type=<Promise.then.U#1: Copy>(this: Promise<int32>, (int32) => Promise<Promise.then.U#1>) => Promise<Promise.then.U#1> & <Promise.then.U#2: Copy>(this: Promise<int32>, (int32) => Promise.then.U#2) => Promise<Promise.then.U#2>
/// @type.node type=Promise<string>
/// @resolution.name source=input target=input
/// @resolution.member source=input.then receiver=Promise<int32> type=<Promise.then.U#1: Copy>(this: Promise<int32>, (int32) => Promise<Promise.then.U#1>) => Promise<Promise.then.U#1> & <Promise.then.U#2: Copy>(this: Promise<int32>, (int32) => Promise.then.U#2) => Promise<Promise.then.U#2> kind=overload-set targets=[Promise.then#1, Promise.then#2]
/// @resolution.call parameters=((int32) => Promise<string>) arguments=(provided(argument) as (int32) => Promise<string>) return=Promise<string> kind=symbol target=Promise.then#1 receiver=Promise<int32> instance=Promise<int32>.then#1<string>
/// @resolution.place source=input placement="local" lifetime="static" access="immutable"
/// @resolution.access source=input root=input
/// @generic.instantiation id="Promise.then#1<int32, string>" template=Promise.then#1 arguments=(int32, string)
/// @generic.instantiation id=Promise.then#1<int32> template=Promise.then#1 arguments=(int32)
/// @generic.instantiation id=Promise.then#2<int32> template=Promise.then#2 arguments=(int32)
/// @generic.instance id="Promise.then#1<int32, string>" template=Promise.then#1 arguments=(int32, string)
/// @generic.instance id=Promise.addReaction<int32> template=Promise.addReaction arguments=(int32)
/// @generic.instance id=Promise.addReaction<string> template=Promise.addReaction arguments=(string)
/// @generic.instance id=Promise.addWaiter<int32> template=Promise.addWaiter arguments=(int32)
/// @generic.instance id=Promise.addWaiter<string> template=Promise.addWaiter arguments=(string)
/// @generic.instance id=Promise.forward<string> template=Promise.forward arguments=(string)
/// @generic.instance id=Promise.observe<int32> template=Promise.observe arguments=(int32)
/// @generic.instance id=Promise.pending<string> template=Promise.pending arguments=(string)
/// @generic.instance id=Promise.queueWaiter<int32> template=Promise.queueWaiter arguments=(int32)
/// @generic.instance id=Promise.queueWaiter<string> template=Promise.queueWaiter arguments=(string)
/// @generic.instance id=Promise.queueWaiters<string> template=Promise.queueWaiters arguments=(string)
/// @generic.instance id=Promise.symbol12<string> template=Promise.symbol12 arguments=(string)
/// @generic.instance id=PromiseForwarded<int32> template=PromiseForwarded arguments=(int32)
/// @generic.instance id=PromiseFulfilled<int32> template=PromiseFulfilled arguments=(int32)
/// @generic.instance id=PromiseReaction.symbol173<int32> template=PromiseReaction.symbol173 arguments=(int32)
/// @generic.instance id=PromiseReaction.symbol173<string> template=PromiseReaction.symbol173 arguments=(string)
/// @generic.instance id=PromiseReaction<int32> template=PromiseReaction arguments=(int32)
/// @generic.instance id=PromiseReaction<string> template=PromiseReaction arguments=(string)
/// @type.symbol symbol=symbol3 type=Function<(int32,), Promise<string>, "readonly">
/// @type.node type=Function<(int32,), Promise<string>, "readonly">
/// @type.symbol symbol=symbol3.value source=value type=int32

    input.then(() => "done")
    /// @type.node source="input.then(() => \"done\")" type=Promise<string>
    /// @type.node source=input type=Promise<int32>
    /// @type.node source=input.then type=<Promise.then.U#1: Copy>(this: Promise<int32>, (int32) => Promise<Promise.then.U#1>) => Promise<Promise.then.U#1> & <Promise.then.U#2: Copy>(this: Promise<int32>, (int32) => Promise.then.U#2) => Promise<Promise.then.U#2>
    /// @resolution.name source=input target=input
    /// @resolution.member source=input.then receiver=Promise<int32> type=<Promise.then.U#1: Copy>(this: Promise<int32>, (int32) => Promise<Promise.then.U#1>) => Promise<Promise.then.U#1> & <Promise.then.U#2: Copy>(this: Promise<int32>, (int32) => Promise.then.U#2) => Promise<Promise.then.U#2> kind=overload-set targets=[Promise.then#1, Promise.then#2]
    /// @resolution.call source="input.then(() => \"done\")" parameters=((int32) => string) arguments=(provided(() => "done") as (int32) => string) return=Promise<string> kind=symbol target=Promise.then#2 receiver=Promise<int32> instance=Promise<int32>.then#2<string>
    /// @resolution.place source=input placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=input root=input
    /// @generic.instantiation id="Promise.then#2<int32, string>" template=Promise.then#2 arguments=(int32, string)
    /// @generic.instance id="Promise.then#2<int32, string>" template=Promise.then#2 arguments=(int32, string)
    /// @generic.instance id=Promise.fulfill<string> template=Promise.fulfill arguments=(string)
    /// @type.symbol symbol=symbol3.symbol5 source="() => \"done\"" type=Function<(), string, "readonly">
    /// @type.node source="() => \"done\"" type=Function<(), string, "readonly">
    /// @type.node source="\"done\"" type="done"

});
"#,
    );
}

#[test]
fn test_call_keeps_mixed_promise_result_nested() {
    let session = TestSession::single(
        r#"
import { Promise } from "destack:async";

declare const input: Promise<int32>;
declare const next: Promise<string>;
declare const usePromise: boolean;
const result: Promise<string | Promise<string>> = input.then(() => {
    usePromise ? next : "done"
});
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
declare const next: Promise<string>;
declare const usePromise: boolean;
const result: Promise<string | Promise<string>> = input.then<int32, Promise<string> | string>(
    (): Promise<string> | string => {
        usePromise ? (next as Promise<string> | string) : ("done" as Promise<string> | string)
    },
);

=== dir ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
/// @type.symbol symbol=input source=input type=Promise<int32>
/// @resolution.pattern source=input kind=binding target=input
/// @generic.instance id=Promise<int32> template=Promise arguments=(int32)
/// @resolution.name source=Promise target=Promise

declare const next: Promise<string>;
/// @type.symbol symbol=next source=next type=Promise<string>
/// @resolution.pattern source=next kind=binding target=next
/// @generic.instance id=Promise<string> template=Promise arguments=(string)
/// @resolution.name source=Promise target=Promise

declare const usePromise: boolean;
/// @type.symbol symbol=usePromise source=usePromise type=boolean
/// @resolution.pattern source=usePromise kind=binding target=usePromise

const result: Promise<string | Promise<string>> = input.then(() => {
/// @type.symbol symbol=result source=result type=Promise<string | Promise<string>>
/// @resolution.pattern source=result kind=binding target=result
/// @generic.instance id="Promise<string | Promise<string>>" template=Promise arguments=(string | Promise<string>)
/// @resolution.name source=Promise target=Promise
/// @resolution.name source=Promise target=Promise
/// @type.node source=input type=Promise<int32>
/// @type.node source=input.then type=<Promise.then.U#1: Copy>(this: Promise<int32>, (int32) => Promise<Promise.then.U#1>) => Promise<Promise.then.U#1> & <Promise.then.U#2: Copy>(this: Promise<int32>, (int32) => Promise.then.U#2) => Promise<Promise.then.U#2>
/// @type.node type=Promise<Promise<string> | string>
/// @resolution.name source=input target=input
/// @resolution.member source=input.then receiver=Promise<int32> type=<Promise.then.U#1: Copy>(this: Promise<int32>, (int32) => Promise<Promise.then.U#1>) => Promise<Promise.then.U#1> & <Promise.then.U#2: Copy>(this: Promise<int32>, (int32) => Promise.then.U#2) => Promise<Promise.then.U#2> kind=overload-set targets=[Promise.then#1, Promise.then#2]
/// @resolution.call parameters=((int32) => Promise<string> | string) arguments=(provided(argument) as (int32) => Promise<string> | string) return=Promise<Promise<string> | string> kind=symbol target=Promise.then#2 receiver=Promise<int32> instance="Promise<int32>.then#2<Promise<string> | string>"
/// @resolution.place source=input placement="local" lifetime="static" access="immutable"
/// @resolution.access source=input root=input
/// @generic.instantiation id="Promise.then#2<int32, Promise<string> | string>" template=Promise.then#2 arguments=(int32, Promise<string> | string)
/// @generic.instantiation id=Promise.then#1<int32> template=Promise.then#1 arguments=(int32)
/// @generic.instantiation id=Promise.then#2<int32> template=Promise.then#2 arguments=(int32)
/// @generic.instance id="Promise.fulfill<Promise<string> | string>" template=Promise.fulfill arguments=(Promise<string> | string)
/// @generic.instance id="Promise.pending<Promise<string> | string>" template=Promise.pending arguments=(Promise<string> | string)
/// @generic.instance id="Promise.queueWaiter<Promise<string> | string>" template=Promise.queueWaiter arguments=(Promise<string> | string)
/// @generic.instance id="Promise.queueWaiters<Promise<string> | string>" template=Promise.queueWaiters arguments=(Promise<string> | string)
/// @generic.instance id="Promise.symbol12<Promise<string> | string>" template=Promise.symbol12 arguments=(Promise<string> | string)
/// @generic.instance id="Promise.then#2<int32, Promise<string> | string>" template=Promise.then#2 arguments=(int32, Promise<string> | string)
/// @generic.instance id="Promise<Promise<string> | string>" template=Promise arguments=(Promise<string> | string)
/// @generic.instance id=Promise.addReaction<int32> template=Promise.addReaction arguments=(int32)
/// @generic.instance id=Promise.addWaiter<int32> template=Promise.addWaiter arguments=(int32)
/// @generic.instance id=Promise.observe<int32> template=Promise.observe arguments=(int32)
/// @generic.instance id=Promise.queueWaiter<int32> template=Promise.queueWaiter arguments=(int32)
/// @generic.instance id=PromiseForwarded<int32> template=PromiseForwarded arguments=(int32)
/// @generic.instance id=PromiseFulfilled<int32> template=PromiseFulfilled arguments=(int32)
/// @generic.instance id=PromiseReaction.symbol173<int32> template=PromiseReaction.symbol173 arguments=(int32)
/// @generic.instance id=PromiseReaction<int32> template=PromiseReaction arguments=(int32)
/// @type.symbol symbol=symbol5 type=Function<(), Promise<string> | string, "readonly">
/// @type.node type=Function<(), Promise<string> | string, "readonly">

    usePromise ? next : "done"
    /// @type.node source="usePromise ? next : \"done\"" type=Promise<string> | string
    /// @type.node source=usePromise type=boolean
    /// @resolution.name source=usePromise target=usePromise
    /// @resolution.place source=usePromise placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=usePromise root=usePromise
    /// @type.node source=next type=Promise<string>
    /// @resolution.name source=next target=next
    /// @resolution.place source=next placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=next root=next
    /// @type.node source="\"done\"" type="done"

});
"#,
    );
}

#[test]
fn test_call_selects_promise_resolve_identity_overload() {
    let session = TestSession::single(
        r#"
import { Promise } from "destack:async";

declare const input: Promise<string>;
const result: Promise<string> = Promise.resolve(input);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

declare const input: Promise<string>;
const result: Promise<string> = Promise.resolve<string>(input);

=== dir ===
import { Promise } from "destack:async";

declare const input: Promise<string>;
/// @type.symbol symbol=input source=input type=Promise<string>
/// @resolution.pattern source=input kind=binding target=input
/// @generic.instance id=Promise<string> template=Promise arguments=(string)
/// @resolution.name source=Promise target=Promise

const result: Promise<string> = Promise.resolve(input);
/// @type.symbol symbol=result source=result type=Promise<string>
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Promise target=Promise
/// @type.node source=Promise type=typeof Promise
/// @type.node source=Promise.resolve type=<Promise.resolve.T#1: Copy>(Promise<Promise.resolve.T#1>) => Promise<Promise.resolve.T#1> & <Promise.resolve.T#2: Copy>(Promise.resolve.T#2) => Promise<Promise.resolve.T#2>
/// @type.node source=Promise.resolve(input) type=Promise<string>
/// @resolution.name source=Promise target=Promise
/// @resolution.member source=Promise.resolve receiver=typeof Promise type=<Promise.resolve.T#1: Copy>(Promise<Promise.resolve.T#1>) => Promise<Promise.resolve.T#1> & <Promise.resolve.T#2: Copy>(Promise.resolve.T#2) => Promise<Promise.resolve.T#2> kind=overload-set targets=[Promise.resolve#1, Promise.resolve#2]
/// @resolution.call source=Promise.resolve(input) parameters=(Promise<string>) arguments=(provided(input) as Promise<string>) return=Promise<string> kind=symbol target=Promise.resolve#1 instance=Promise.resolve#1<string>
/// @generic.instantiation id=Promise.resolve#1<string> template=Promise.resolve#1 arguments=(string)
/// @generic.instance id=Promise.resolve#1<string> template=Promise.resolve#1 arguments=(string)
/// @type.node source=input type=Promise<string>
/// @resolution.name source=input target=input
/// @resolution.place source=input placement="local" lifetime="static" access="immutable"
/// @resolution.access source=input root=input
"#);
}

#[test]
fn test_call_infers_type_argument_from_parameter() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const number = identity(1);
const text = identity("x");
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const number: int64 = identity<int64>(1);
const text: "x" = identity<"x">("x");

=== dir ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=identity.value

}

const number = identity(1);
/// @type.symbol symbol=number source=number type=int64
/// @resolution.pattern source=number kind=binding target=number
/// @type.node source=identity type=(int64) => int64
/// @type.node source=identity(1) type=int64
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(1) parameters=(int64) arguments=(provided(1) as int64) return=int64 kind=symbol target=identity instance=identity<int64>
/// @generic.instantiation id=identity<int64> template=identity arguments=(int64)
/// @generic.instance id=identity<int64> template=identity arguments=(int64)
/// @type.node source=1 type=1

const text = identity("x");
/// @type.symbol symbol=text source=text type="x"
/// @resolution.pattern source=text kind=binding target=text
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=("x") => "x"
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(\"x\")" parameters=("x") arguments=(provided("x") as "x") return="x" kind=symbol target=identity instance="identity<\"x\">"
/// @generic.instantiation id="identity<\"x\">" template=identity arguments=("x")
/// @generic.instance id="identity<\"x\">" template=identity arguments=("x")
/// @type.node source="\"x\"" type="x"
"#);
}

#[test]
fn test_array_literal_argument_widens_generic_container() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const values = identity([1, 2]);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const values: int64[] = identity<int64[]>([1, 2]);

=== dir ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=identity.value

}

const values = identity([1, 2]);
/// @type.symbol symbol=values source=values type=int64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @type.node source="identity([1, 2])" type=int64[]
/// @type.node source=identity type=(int64[]) => int64[]
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity([1, 2])" parameters=(int64[]) arguments=(provided([1, 2]) as int64[]) return=int64[] kind=symbol target=identity instance=identity<int64[]>
/// @generic.instantiation id=identity<int64[]> template=identity arguments=(int64[])
/// @generic.instance id=identity<int64[]> template=identity arguments=(int64[])
/// @type.node source=[1, 2] type=int64[]
/// @resolution.call source=[1, 2] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_array_parameter_infers_widened_element_type() {
    let session = TestSession::single(
        r#"
function first<T: Copy>(values: T[]): T {
    return values[0];
}

const value = first([1, 2]);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function first<T: Copy>(values: T[]): T {
    return values[0];
}

const value: int64 = first<int64>([1, 2]);

=== dir ===
function first<T: Copy>(values: T[]): T {
/// @generic.template symbol=first parameters=(T: Copy)
/// @type.symbol symbol=first type=<T: Copy>(T[]) => T
/// @generic.instance id=Array<T> template=Array arguments=(T)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<T>> template=sliceAssumeInit arguments=(MaybeUninit<T>)
/// @generic.instance id=sliceUninit<MaybeUninit<T>> template=sliceUninit arguments=(MaybeUninit<T>)
/// @type.symbol symbol=first.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy
/// @type.symbol symbol=first.values source="values: T[]" type=T[]
/// @resolution.name source=T target=first.T
/// @resolution.name source=T target=first.T

    return values[0];
    /// @type.node source=values type=T[]
    /// @type.node source=values[0] type=T
    /// @resolution.name source=values target=first.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=first.values
    /// @resolution.subscript source=values[0] type=T kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=T, regions=(\"managed\" & \"local\"))"
    /// @generic.instantiation id="index#2<T, \"managed\" & \"local\">" template=index#2 arguments=(T, "managed" & "local") owner=first
    /// @generic.instance id="index#2<T, \"bound0\" & \"local\">" template=index#2 arguments=(T, "bound0" & "local")
    /// @type.node source=0 type=0

}

const value = first([1, 2]);
/// @type.symbol symbol=value source=value type=int64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="first([1, 2])" type=int64
/// @type.node source=first type=(int64[]) => int64
/// @resolution.name source=first target=first
/// @resolution.call source="first([1, 2])" parameters=(int64[]) arguments=(provided([1, 2]) as int64[]) return=int64 kind=symbol target=first instance=first<int64>
/// @generic.instantiation id=first<int64> template=first arguments=(int64)
/// @generic.instance id="index#2<int64, \"bound0\" & \"local\">" template=index#2 arguments=(int64, "bound0" & "local")
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=first<int64> template=first arguments=(int64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @type.node source=[1, 2] type=int64[]
/// @resolution.call source=[1, 2] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_explicit_literal_type_arguments_create_distinct_generic_instances() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const first = identity<1>(1);
const second = identity<2>(2);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const first: 1 = identity<1>(1);
const second: 2 = identity<2>(2);

=== dir ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=identity.value

}

const first = identity<1>(1);
/// @type.symbol symbol=first source=first type=1
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=identity type=(1) => 1
/// @type.node source=identity<1>(1) type=1
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity<1>(1) parameters=(1) arguments=(provided(1) as 1) return=1 kind=symbol target=identity instance=identity<1>
/// @generic.instantiation id=identity<1> template=identity arguments=(1)
/// @generic.instance id=identity<1> template=identity arguments=(1)
/// @type.node source=1 type=1

const second = identity<2>(2);
/// @type.symbol symbol=second source=second type=2
/// @resolution.pattern source=second kind=binding target=second
/// @type.node source=identity type=(2) => 2
/// @type.node source=identity<2>(2) type=2
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity<2>(2) parameters=(2) arguments=(provided(2) as 2) return=2 kind=symbol target=identity instance=identity<2>
/// @generic.instantiation id=identity<2> template=identity arguments=(2)
/// @generic.instance id=identity<2> template=identity arguments=(2)
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_explicit_call_type_argument_selects_instance() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const text = identity<string>("x");
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const text: string = identity<string>("x");

=== dir ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=identity.value

}

const text = identity<string>("x");
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text
/// @type.node source="identity<string>(\"x\")" type=string
/// @type.node source=identity type=(string) => string
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity<string>(\"x\")" parameters=(string) arguments=(provided("x") as string) return=string kind=symbol target=identity instance=identity<string>
/// @generic.instantiation id=identity<string> template=identity arguments=(string)
/// @generic.instance id=identity<string> template=identity arguments=(string)
/// @type.node source="\"x\"" type="x"
"#,
    );
}

#[test]
fn test_explicit_call_type_argument_mismatch_reports_assignability_error() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

identity<int32>("x");
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

identity<int32>("x");

=== dir ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=identity.value

}

identity<int32>("x");
/// @type.node source="identity<int32>(\"x\")" type=int32
/// @type.node source=identity type=(int32) => int32
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity<int32>(\"x\")" parameters=(int32) arguments=(provided("x") as int32) return=int32 kind=symbol target=identity instance=identity<int32>
/// @generic.instantiation id=identity<int32> template=identity arguments=(int32)
/// @type.node source="\"x\"" type="x"
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '\"x\"' is not assignable to parameter of type 'int32'"
/// @diagnostic.label line=6 column=17 span="\"x\"" line_source="identity<int32>(\"x\");"
/// @diagnostic.related line=6 column=1 span="identity<int32>(\"x\")" line_source="identity<int32>(\"x\");" message="in this call"
"#,
    );
}

#[test]
fn test_explicit_function_type_argument_specializes_reference() {
    let session = TestSession::single(
        r#"
function identity<T>(value: T): T {
    return value;
}

const asInt = identity<int32>;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const asInt: (value: int32) => int32 = identity<int32>;

=== dir ===
function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=identity.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=identity.value

}

const asInt = identity<int32>;
/// @type.symbol symbol=asInt source=asInt type=Function<(int32,), int32, "readonly">
/// @resolution.pattern source=asInt kind=binding target=asInt
/// @type.node source=identity type=Function<(T,), T, "readonly">
/// @type.node source=identity<int32> type=Function<(int32,), int32, "readonly">
/// @resolution.name source=identity target=identity
/// @resolution.function source=identity type=Function<(T,), T, "readonly"> target=identity
/// @resolution.function source=identity<int32> type=Function<(int32,), int32, "readonly"> target=identity instance=identity<int32>
/// @generic.instantiation id=identity<int32> template=identity arguments=(int32)
/// @generic.instance id=identity<int32> template=identity arguments=(int32)
"#,
    );
}

#[test]
fn test_explicit_function_type_argument_rejects_overload_set() {
    let session = TestSession::single(
        r#"
function parse<T>(value: T): T {
    return value;
}

function parse<T: Copy>(value: T[]): T {
    return value[0];
}

const parser = parse<int32>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function parse<T>(value: T): T {
    return value;
}

function parse<T: Copy>(value: T[]): T {
    return value[0];
}

const parser = parse<int32>;

=== dir ===
function parse<T>(value: T): T {
/// @generic.template symbol=parse#1 parameters=(T#1)
/// @type.symbol symbol=parse#1 type=<T#1>(T#1) => T#1
/// @type.symbol symbol=parse.T#1 source=T type=T#1
/// @type.symbol symbol=parse.value#1 source="value: T" type=T#1
/// @resolution.name source=T target=parse.T#1
/// @resolution.name source=T target=parse.T#1

    return value;
    /// @type.node source=value type=T#1
    /// @resolution.name source=value target=parse.value#1
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=parse.value#1

}

function parse<T: Copy>(value: T[]): T {
/// @generic.template symbol=parse#2 parameters=(T#2: Copy)
/// @type.symbol symbol=parse#2 type=<T#2: Copy>(T#2[]) => T#2
/// @type.symbol symbol=parse.T#2 source="T: Copy" type=T#2
/// @resolution.name source=Copy target=Copy
/// @type.symbol symbol=parse.value#2 source="value: T[]" type=T#2[]
/// @resolution.name source=T target=parse.T#2
/// @resolution.name source=T target=parse.T#2

    return value[0];
    /// @type.node source=value type=T#2[]
    /// @type.node source=value[0] type=T#2
    /// @resolution.name source=value target=parse.value#2
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=parse.value#2
    /// @resolution.subscript source=value[0] type=T#2 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=T#2, regions=(\"managed\" & \"local\"))"
    /// @generic.instantiation id="index#2<T#2, \"managed\" & \"local\">" template=index#2 arguments=(T#2, "managed" & "local") owner=parse#2
    /// @type.node source=0 type=0

}

const parser = parse<int32>;
/// @type.symbol symbol=parser source=parser type=<error>
/// @resolution.pattern source=parser kind=binding target=parser
/// @type.node source=parse type=<error>
/// @type.node source=parse<int32> type=<error>
/// @resolution.name source=parse target=[parse#1, parse#2]
/// @resolution.rejected source=parse<int32>
"#,
        r#"
/// @diagnostic.error id=ambiguous-overload message="overload 'parse' is ambiguous without a call"
/// @diagnostic.label line=10 column=16 span="parse" line_source="const parser = parse<int32>;"
/// @diagnostic.related line=2 column=10 span="parse" line_source="function parse<T>(value: T): T {" message="one candidate is declared here"
/// @diagnostic.related line=6 column=10 span="parse" line_source="function parse<T: Copy>(value: T[]): T {" message="one candidate is declared here"
"#,
    );
}

#[test]
fn test_defaulted_type_argument_uses_prior_type_parameter() {
    let session = TestSession::single(
        r#"
declare function pair<T, U = T>(left: T, right?: U): (T, U);

const defaulted = pair(1);
const overridden = pair(1, "x");
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function pair<T, U = T>(left: T, right?: U): (T, U);

const defaulted: (int64, int64) = pair<int64, int64>(1);
const overridden: (int64, string) = pair<int64, string>(1, "x" as string | undefined);

=== dir ===
declare function pair<T, U = T>(left: T, right?: U): (T, U);
/// @generic.template symbol=pair parameters=(T, U = T)
/// @type.symbol symbol=pair source="declare function pair<T, U = T>(left: T, right?: U): (T, U)" type=<T, U = T>(T, U | undefined?) => (T, U)
/// @type.symbol symbol=pair.T source=T type=T
/// @type.symbol symbol=pair.U source="U = T" type=U
/// @resolution.name source=T target=pair.T
/// @resolution.name source=T target=pair.T
/// @resolution.name source=U target=pair.U
/// @resolution.name source=T target=pair.T
/// @resolution.name source=U target=pair.U

const defaulted = pair(1);
/// @type.symbol symbol=defaulted source=defaulted type=(int64, int64)
/// @resolution.pattern source=defaulted kind=binding target=defaulted
/// @type.node source=pair type=(int64, int64 | undefined?) => (int64, int64)
/// @type.node source=pair(1) type=(int64, int64)
/// @resolution.name source=pair target=pair
/// @resolution.call source=pair(1) parameters=(int64, int64 | undefined) arguments=(provided(1) as int64, omitted as int64 | undefined) return=(int64, int64) kind=symbol target=pair instance="pair<int64, int64>"
/// @generic.instantiation id="pair<int64, int64>" template=pair arguments=(int64, int64)
/// @generic.instance id="pair<int64, int64>" template=pair arguments=(int64, int64)
/// @type.node source=1 type=1

const overridden = pair(1, "x");
/// @type.symbol symbol=overridden source=overridden type=(int64, string)
/// @resolution.pattern source=overridden kind=binding target=overridden
/// @type.node source="pair(1, \"x\")" type=(int64, string)
/// @type.node source=pair type=(int64, string | undefined?) => (int64, string)
/// @resolution.name source=pair target=pair
/// @resolution.call source="pair(1, \"x\")" parameters=(int64, string | undefined) arguments=(provided(1) as int64, provided("x") as string | undefined) return=(int64, string) kind=symbol target=pair instance="pair<int64, string>"
/// @generic.instantiation id="pair<int64, string>" template=pair arguments=(int64, string)
/// @generic.instance id="pair<int64, string>" template=pair arguments=(int64, string)
/// @type.node source=1 type=1
/// @type.node source="\"x\"" type="x"
"#,
    );
}

#[test]
fn test_constrained_type_parameter_preserves_competing_literals() {
    let session = TestSession::single(
        r#"
declare function choose<T: 1 | 2>(left: T, right: T): T;

const value = choose(1, 2);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function choose<T: 1 | 2>(left: T, right: T): T;

const value: 1 | 2 = choose<1 | 2>(1, 2);

=== dir ===
declare function choose<T: 1 | 2>(left: T, right: T): T;
/// @generic.template symbol=choose parameters=(T: 1 | 2)
/// @type.symbol symbol=choose source="declare function choose<T: 1 | 2>(left: T, right: T): T" type=<T: 1 | 2>(T, T) => T
/// @type.symbol symbol=choose.T source="T: 1 | 2" type=T
/// @resolution.name source=T target=choose.T
/// @resolution.name source=T target=choose.T
/// @resolution.name source=T target=choose.T

const value = choose(1, 2);
/// @type.symbol symbol=value source=value type=1 | 2
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="choose(1, 2)" type=1 | 2
/// @type.node source=choose type=(1 | 2, 1 | 2) => 1 | 2
/// @resolution.name source=choose target=choose
/// @resolution.call source="choose(1, 2)" parameters=(1 | 2, 1 | 2) arguments=(provided(1) as 1 | 2, provided(2) as 1 | 2) return=1 | 2 kind=symbol target=choose instance="choose<1 | 2>"
/// @generic.instantiation id="choose<1 | 2>" template=choose arguments=(1 | 2)
/// @generic.instance id="choose<1 | 2>" template=choose arguments=(1 | 2)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_where_constraint_preserves_competing_literals() {
    let session = TestSession::single(
        r#"
declare function choose<T>(left: T, right: T): T where T: 1 | 2;

const value = choose(1, 2);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function choose<T>(left: T, right: T): T where T: 1 | 2;

const value: 1 | 2 = choose<1 | 2>(1, 2);

=== dir ===
declare function choose<T>(left: T, right: T): T where T: 1 | 2;
/// @generic.template symbol=choose parameters=(T)
/// @type.symbol symbol=choose source="declare function choose<T>(left: T, right: T): T where T: 1 | 2" type=<T>(T, T) => T
/// @type.symbol symbol=choose.T source=T type=T
/// @resolution.name source=T target=choose.T
/// @resolution.name source=T target=choose.T
/// @resolution.name source=T target=choose.T
/// @resolution.name source=T target=choose.T

const value = choose(1, 2);
/// @type.symbol symbol=value source=value type=1 | 2
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="choose(1, 2)" type=1 | 2
/// @type.node source=choose type=(1 | 2, 1 | 2) => 1 | 2
/// @resolution.name source=choose target=choose
/// @resolution.call source="choose(1, 2)" parameters=(1 | 2, 1 | 2) arguments=(provided(1) as 1 | 2, provided(2) as 1 | 2) return=1 | 2 kind=symbol target=choose instance="choose<1 | 2>"
/// @generic.instantiation id="choose<1 | 2>" template=choose arguments=(1 | 2)
/// @generic.instance id="choose<1 | 2>" template=choose arguments=(1 | 2)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_explicit_function_argument_checks_where_clause() {
    let session = TestSession::single(
        r#"
newtype interface Marker {}

struct Good {}

extension of Good implements Marker {}

function accept<T>(value: T): T where T: Marker {
    return value;
}

const accepted = accept<Good>;
const rejected = accept<int32>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype interface Marker {}

struct Good {}

extension of Good implements Marker {}

function accept<T>(value: T): T where T: Marker {
    return value;
}

const accepted: (value: Good) => Good = accept<Good>;
const rejected: (value: int32) => int32 = accept<int32>;

=== dir ===
newtype interface Marker {}
/// @generic.template symbol=Marker parameters=(this: Marker)
/// @type.symbol symbol=Marker source="newtype interface Marker {}" type=Marker
/// @definition.interface symbol=Marker source="newtype interface Marker {}" template=(this: Marker) nominal=true
/// @definition.where symbol=Marker source="newtype interface Marker {}" relation=satisfies left=this right=Marker

struct Good {}
/// @type.symbol symbol=Good source="struct Good {}" type=Good
/// @definition.struct symbol=Good source="struct Good {}"

extension of Good implements Marker {}
/// @definition.extension symbol=<module>#2 source="extension of Good implements Marker {}" form=local target=Good
/// @definition.implements symbol=<module>#2 source=Marker target=Marker
/// @resolution.name source=Good target=Good
/// @resolution.name source=Marker target=Marker

function accept<T>(value: T): T where T: Marker {
/// @generic.template symbol=accept parameters=(T)
/// @type.symbol symbol=accept type=<T>(T) => T
/// @type.symbol symbol=accept.T source=T type=T
/// @type.symbol symbol=accept.value source="value: T" type=T
/// @resolution.name source=T target=accept.T
/// @resolution.name source=T target=accept.T
/// @resolution.name source=T target=accept.T
/// @resolution.name source=Marker target=Marker

    return value;
    /// @type.node source=value type=T
    /// @resolution.name source=value target=accept.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=accept.value

}

const accepted = accept<Good>;
/// @type.symbol symbol=accepted source=accepted type=Function<(Good,), Good, "readonly">
/// @resolution.pattern source=accepted kind=binding target=accepted
/// @type.node source=accept type=Function<(T,), T, "readonly">
/// @type.node source=accept<Good> type=Function<(Good,), Good, "readonly">
/// @resolution.name source=accept target=accept
/// @resolution.function source=accept type=Function<(T,), T, "readonly"> target=accept
/// @resolution.function source=accept<Good> type=Function<(Good,), Good, "readonly"> target=accept instance=accept<Good>
/// @generic.instantiation id=accept<Good> template=accept arguments=(Good)
/// @resolution.name source=Good target=Good

const rejected = accept<int32>;
/// @type.symbol symbol=rejected source=rejected type=Function<(int32,), int32, "readonly">
/// @resolution.pattern source=rejected kind=binding target=rejected
/// @type.node source=accept type=Function<(T,), T, "readonly">
/// @type.node source=accept<int32> type=Function<(int32,), int32, "readonly">
/// @resolution.name source=accept target=accept
/// @resolution.function source=accept type=Function<(T,), T, "readonly"> target=accept
/// @resolution.function source=accept<int32> type=Function<(int32,), int32, "readonly"> target=accept instance=accept<int32>
/// @generic.instantiation id=accept<int32> template=accept arguments=(int32)
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'int32' does not satisfy 'Marker'"
/// @diagnostic.label line=13 column=18 span="accept<int32>" line_source="const rejected = accept<int32>;"
"#,
    );
}

#[test]
fn test_reject_a_generic_call_whose_argument_contradicts_the_contextual_result() {
    let session = TestSession::single(
        r#"
struct Box<T> {
    value: T;
}

extension<T> of Box<T> {
    static of(value: T): Box<T> {
        return Box<T> { value };
    }
}

function build(value: float64): Box<int32> {
    return Box.of(value);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Box<out T> {
    value: T;
}

extension<T> of Box<T> {
    static of(value: T): Box<T> {
        return Box<T> { value };
    }
}

function build(value: float64): Box<int32> {
    return Box.of<int32>(value);
}

=== dir ===
struct Box<T> {
/// @generic.template symbol=Box parameters=(out T#1)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T#1)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Box.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

}

extension<T> of Box<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<T#2>
/// @definition.method symbol=of slot=of static=true type=(T#2) => Box<T#2>
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T

    static of(value: T): Box<T> {
    /// @type.symbol symbol=of type=(T#2) => Box<T#2>
    /// @type.symbol symbol=of.value source="value: T" type=T#2
    /// @resolution.name source=T target=T
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=T target=T

        return Box<T> { value };
        /// @resolution.name source=Box target=Box
        /// @resolution.name source=T target=T
        /// @resolution.name source=value target=of.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=of.value

    }
}

function build(value: float64): Box<int32> {
/// @type.symbol symbol=build type=(float64) => Box<int32>
/// @type.symbol symbol=build.value source="value: float64" type=float64
/// @resolution.name source=Box target=Box

    return Box.of(value);
    /// @resolution.name source=Box target=Box
    /// @resolution.member source=Box.of receiver=Box type=(T#2) => Box<T#2> kind=symbol target_receiver=Box target=of
    /// @resolution.call source=Box.of(value) parameters=(int32) arguments=(provided(value) as int32) return=Box<int32> kind=symbol target=of instance=Box<int32>.<extension#1>.of
    /// @generic.instantiation id=of<int32> template=of arguments=(int32)
    /// @resolution.name source=value target=build.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=build.value

}
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type 'float64' is not assignable to the declared result type 'int32'"
/// @diagnostic.label line=13 column=12 span="Box.of(value)" line_source="return Box.of(value);"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Box'"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'float64' is not assignable to parameter of type 'int32'"
/// @diagnostic.label line=13 column=19 span="value" line_source="return Box.of(value);"
/// @diagnostic.related line=13 column=12 span="Box.of(value)" line_source="return Box.of(value);" message="in this call"
"#,
    );
}

#[test]
fn test_infer_a_chained_generic_method_call_through_its_contextual_callbacks() {
    let session = TestSession::single(
        r#"
declare const values: ^int32[];

const kept = values
    .map((value) => value)
    .filter((value) => value !== undefined);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: ^int32[];

const kept: ^int32[] = values.map<int32, int32>((value: int32): int32 => value).filter<int32>(
    (value: &immutable int32): boolean => value !== undefined,
);

=== dir ===
declare const values: ^int32[];
/// @type.symbol symbol=values source=values type=^int32[]
/// @resolution.pattern source=values kind=binding target=values

const kept = values
/// @type.symbol symbol=kept source=kept type=^int32[]
/// @resolution.pattern source=kept kind=binding target=kept
/// @resolution.name source=values target=values
/// @resolution.member receiver=^int32[] type=(this: ^int32[], <type_expression.'a>(&type_expression.'a immutable int32, isize) => boolean) => ^int32[] kind=symbol target_receiver=^int32[] target=filter#1
/// @resolution.member receiver=^int32[] type=<map.U#1>(this: ^int32[], (int32, isize) => map.U#1) => ^map.U#1[] kind=symbol target_receiver=^int32[] target=map#1
/// @resolution.call parameters=((int32, isize) => int32) arguments=(provided((value) => value) as (int32, isize) => int32) return=^int32[] kind=symbol target=map#1 receiver=^int32[] instance=^T#3[].<extension#3>.map#1<int32>
/// @resolution.call parameters=(<type_expression.'a>(&type_expression.'a immutable int32, isize) => boolean) arguments=(provided((value) => value !== undefined) as <type_expression.'a>(&type_expression.'a immutable int32, isize) => boolean) return=^int32[] kind=symbol target=filter#1 receiver=^int32[] instance=^T#3[].<extension#3>.filter#1
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @generic.instantiation id="map#1<int32, int32>" template=map#1 arguments=(int32, int32)
/// @generic.instantiation id=filter#1<int32> template=filter#1 arguments=(int32)
/// @generic.instantiation id=map#1<int32> template=map#1 arguments=(int32)

    .map((value) => value)
    /// @type.symbol symbol=symbol2 source="(value) => value" type=Function<(int32,), int32, "readonly">
    /// @type.symbol symbol=symbol2.value source=value type=int32
    /// @resolution.name source=value target=symbol2.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=symbol2.value

    .filter((value) => value !== undefined);
    /// @type.symbol symbol=symbol4 source="(value) => value !== undefined" type=Function<(&type_expression.'a immutable int32,), boolean, "readonly">
    /// @type.symbol symbol=symbol4.value source=value type=&type_expression.'a immutable int32
    /// @resolution.name source=value target=symbol4.value
    /// @resolution.operator source="value !== undefined" type=boolean operator="!==" kind=builtin operands=[value as int32 families=(integer), undefined as undefined families=(undefined)]
    /// @resolution.place source=value placement=type_expression.'a lifetime=type_expression.'a access="immutable"
    /// @resolution.access source=value root=symbol4.value
"#,
        r#"
/// @diagnostic.error id=invalid-strict-equality message="this comparison is unintentional: types '&'a immutable int32' and 'undefined' have no overlap"
/// @diagnostic.label line=6 column=30 span="!==" line_source=".filter((value) => value !== undefined);"
"#,
    );
}

#[test]
fn test_adapt_a_literal_argument_to_a_family_bounded_parameter() {
    let session = TestSession::single(
        r#"
import { Arithmetic, Integer } from "destack:math";

function bump<T: Integer>(value: T): T | undefined {
    return value.checkedAdd(1);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import { Arithmetic, Integer } from "destack:math";

function bump<T: Integer>(value: T): T | undefined {
    return value.checkedAdd<T>(1);
}

=== dir ===
import { Arithmetic, Integer } from "destack:math";

function bump<T: Integer>(value: T): T | undefined {
    return value.checkedAdd(1);
}
"#,
        r#"
"#,
    );
}

#[test]
fn test_adopt_the_declared_result_for_numeric_literal_arguments() {
    let session = TestSession::single(
        r#"
function pick<T>(a: T, b: T, flag: boolean): T {
    return flag ? a : b;
}

function halves(flag: boolean): float64 {
    return pick(1, 2, flag);
}

function fractions(flag: boolean): float32 {
    return pick(1.5, 2.5, flag);
}

const widened = pick(1.5, 2.5, true);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
function pick<T>(a: T, b: T, flag: boolean): T {
    return flag ? a : b;
}

function halves(flag: boolean): float64 {
    return pick<float64>(1, 2, flag);
}

function fractions(flag: boolean): float32 {
    return pick<float32>(1.5, 2.5, flag);
}

const widened: float64 = pick<float64>(1.5, 2.5, true);

=== dir ===
function pick<T>(a: T, b: T, flag: boolean): T {
    return flag ? a : b;
}

function halves(flag: boolean): float64 {
    return pick(1, 2, flag);
}

function fractions(flag: boolean): float32 {
    return pick(1.5, 2.5, flag);
}

const widened = pick(1.5, 2.5, true);
"#,
        r#"
"#,
    );
}

#[test]
fn test_adapt_a_literal_argument_inside_a_blanket_extension_method() {
    let session = TestSession::single(
        r#"
import { Arithmetic, Integer } from "destack:math";

extension<T: Integer> of T {
    bump(this): T | undefined {
        this.checkedAdd(1)
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import { Arithmetic, Integer } from "destack:math";

extension<T: Integer> of T {
    bump(this): T | undefined {
        this.checkedAdd<T>(1)
    }
}

=== dir ===
import { Arithmetic, Integer } from "destack:math";

extension<T: Integer> of T {
    bump(this): T | undefined {
        this.checkedAdd(1)
    }
}
"#,
        r#"
"#,
    );
}

/// Infer chained callback parameters from the receiver's element type.
#[test]
fn test_chained_filter_callback_infers_its_parameter() {
    let session = TestSession::single(
        r#"
declare const values: (int32 | undefined)[];

const defined = values.map((value) => value).filter((value) => value !== undefined);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: (int32 | undefined)[];

const defined: ^(int32 | undefined)[] = values.map<int32 | undefined, int32 | undefined, "managed">(
    (value: int32 | undefined): int32 | undefined => value,
).filter<int32 | undefined>(
    (value: &immutable (int32 | undefined)): boolean => value !== (undefined as int32 | undefined),
);

=== dir ===
declare const values: (int32 | undefined)[];
/// @type.symbol symbol=values source=values type=int32 | undefined[]
/// @resolution.pattern source=values kind=binding target=values

const defined = values.map((value) => value).filter((value) => value !== undefined);
/// @type.symbol symbol=defined source=defined type=^int32 | undefined[]
/// @resolution.pattern source=defined kind=binding target=defined
/// @resolution.name source=values target=values
/// @resolution.member source="values.map((value) => value).filter" receiver=^int32 | undefined[] type=(this: ^int32 | undefined[], <type_expression.'a>(&type_expression.'a immutable (int32 | undefined), isize) => boolean) => ^int32 | undefined[] kind=symbol target_receiver=^int32 | undefined[] target=filter#1
/// @resolution.member source=values.map receiver=int32 | undefined[] type=<map.U#2, map#2.'a>(this: &map#2.'a readonly int32 | undefined[], (int32 | undefined, isize) => map.U#2) => ^map.U#2[] kind=symbol target_receiver=int32 | undefined[] target=map#2
/// @resolution.call source="values.map((value) => value)" parameters=((int32 | undefined, isize) => int32 | undefined) arguments=(provided((value) => value) as (int32 | undefined, isize) => int32 | undefined) return=^int32 | undefined[] regions=("managed" & "local") kind=symbol target=map#2 receiver=int32 | undefined[] adjustments=(borrow(&'managed readonly int32 | undefined[])) instance="Array<int32 | undefined>.<extension#4>.map#2<int32 | undefined, \"managed\" & \"local\">"
/// @resolution.call source="values.map((value) => value).filter((value) => value !== undefined)" parameters=(<type_expression.'a>(&type_expression.'a immutable (int32 | undefined), isize) => boolean) arguments=(provided((value) => value !== undefined) as <type_expression.'a>(&type_expression.'a immutable (int32 | undefined), isize) => boolean) return=^int32 | undefined[] kind=symbol target=filter#1 receiver=^int32 | undefined[] instance=^T#3[].<extension#3>.filter#1
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @generic.instantiation id="filter#1<int32 | undefined>" template=filter#1 arguments=(int32 | undefined)
/// @generic.instantiation id="map#2<int32 | undefined, int32 | undefined, \"managed\" & \"local\">" template=map#2 arguments=(int32 | undefined, int32 | undefined, "managed" & "local")
/// @generic.instantiation id="map#2<int32 | undefined>" template=map#2 arguments=(int32 | undefined)
/// @type.symbol symbol=symbol2 source="(value) => value" type=Function<(int32 | undefined,), int32 | undefined, "readonly">
/// @type.symbol symbol=symbol2.value source=value type=int32 | undefined
/// @resolution.name source=value target=symbol2.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol2.value
/// @type.symbol symbol=symbol4 source="(value) => value !== undefined" type=Function<(&type_expression.'a immutable (int32 | undefined),), boolean, "readonly">
/// @type.symbol symbol=symbol4.value source=value type=&type_expression.'a immutable (int32 | undefined)
/// @resolution.name source=value target=symbol4.value
/// @resolution.operator source="value !== undefined" type=boolean operator="!==" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), undefined as int32 | undefined families=(integer | undefined)]
/// @resolution.place source=value placement=type_expression.'a lifetime=type_expression.'a access="immutable"
/// @resolution.access source=value root=symbol4.value
"#,
        r#"

"#,
    );
}

/// Infer a receiver-owned const parameter from a literal argument.
#[test]
fn test_method_literal_argument_infers_a_const_parameter() {
    let session = TestSession::single(
        r#"
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];

class Values<T> {
    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[]
        where T: Clone {
        todo("flat")
    }
}

declare const values: Values<int32>;

const once = values.flat();
const twice = values.flat(2);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];

class Values<T> {
    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[]
        where T: Clone {
        todo("flat" as string | undefined)
    }
}

declare const values: Values<int32>;

const once: Element<int32, 1>[] = values.flat<int32, 1, "managed">();
const twice: Element<int32, 2>[] = values.flat<int32, 2, "managed">(2 as 2 | undefined);

=== dir ===
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];
/// @generic.template symbol=Element parameters=(T#1, const Depth#1: usize)
/// @type.symbol symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" type=Depth#1 extends 0 ? T#1 : T#1[]
/// @definition.type symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" template=(T#1, const Depth#1: usize) value=Depth#1 extends 0 ? T#1 : T#1[]
/// @type.symbol symbol=Element.T source=T type=T#1
/// @type.symbol symbol=Element.Depth source="const Depth: usize" type=Depth#1
/// @resolution.name source=Depth target=Element.Depth
/// @resolution.name source=T target=Element.T
/// @resolution.name source=T target=Element.T

class Values<T> {
/// @generic.template symbol=Values parameters=(T#2)
/// @type.symbol symbol=Values type=typeof Values
/// @definition.class symbol=Values template=(T#2)
/// @definition.method symbol=Values.flat slot=flat type=<const Depth#2: usize = 1, Values.flat.'a>(this: &Values.flat.'a readonly Values<T#2>, Depth#2 | undefined?) => Element<T#2, Depth#2>[]
/// @type.symbol symbol=Values.T source=T type=T#2

    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[]
    /// @generic.template symbol=Values.flat parent=template#1 parameters=(const Depth#2: usize = 1, 'a)
    /// @type.symbol symbol=Values.flat type=<const Depth#2: usize = 1, Values.flat.'a>(this: &Values.flat.'a readonly Values<T#2>, Depth#2 | undefined?) => Element<T#2, Depth#2>[]
    /// @type.symbol symbol=Values.flat.Depth source="const Depth: usize = 1" type=Depth#2
    /// @type.symbol symbol=Values.flat.this source="&readonly this" type=&Values.flat.'a readonly Values<T#2>
    /// @type.symbol symbol=Values.flat.depth source="depth?: Depth" type=Depth#2 | undefined
    /// @resolution.name source=Depth target=Values.flat.Depth
    /// @resolution.name source=Element target=Element
    /// @resolution.name source=T target=Values.T
    /// @resolution.name source=Depth target=Values.flat.Depth

        where T: Clone {
        /// @resolution.name source=T target=Values.T
        /// @resolution.name source=Clone target=Clone

        todo("flat")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"flat\")" parameters=(string | undefined) arguments=(provided("flat") as string | undefined) return=never kind=symbol target=todo

    }
}

declare const values: Values<int32>;
/// @type.symbol symbol=values source=values type=Values<int32>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=Values target=Values

const once = values.flat();
/// @type.symbol symbol=once source=once type=Element<int32, 1>[]
/// @resolution.pattern source=once kind=binding target=once
/// @resolution.name source=values target=values
/// @resolution.member source=values.flat receiver=Values<int32> type=<const Depth#2: usize = 1, Values.flat.'a>(this: &Values.flat.'a readonly Values<int32>, Depth#2 | undefined?) => Element<int32, Depth#2>[] kind=symbol target_receiver=Values<int32> target=Values.flat
/// @resolution.call source=values.flat() parameters=(1 | undefined) arguments=(omitted as 1 | undefined) return=Element<int32, 1>[] regions=("managed" & "local") kind=symbol target=Values.flat receiver=Values<int32> adjustments=(borrow(&'managed readonly Values<int32>)) instance="Values<int32>.flat<1, \"managed\" & \"local\">"
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @generic.instantiation id="Values.flat<int32, 1, \"managed\" & \"local\">" template=Values.flat arguments=(int32, 1, "managed" & "local")
/// @generic.instantiation id=Values.flat<int32> template=Values.flat arguments=(int32)

const twice = values.flat(2);
/// @type.symbol symbol=twice source=twice type=Element<int32, 2>[]
/// @resolution.pattern source=twice kind=binding target=twice
/// @resolution.name source=values target=values
/// @resolution.member source=values.flat receiver=Values<int32> type=<const Depth#2: usize = 1, Values.flat.'a>(this: &Values.flat.'a readonly Values<int32>, Depth#2 | undefined?) => Element<int32, Depth#2>[] kind=symbol target_receiver=Values<int32> target=Values.flat
/// @resolution.call source=values.flat(2) parameters=(2 | undefined) arguments=(provided(2) as 2 | undefined) return=Element<int32, 2>[] regions=("managed" & "local") kind=symbol target=Values.flat receiver=Values<int32> adjustments=(borrow(&'managed readonly Values<int32>)) instance="Values<int32>.flat<2, \"managed\" & \"local\">"
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @generic.instantiation id="Values.flat<int32, 2, \"managed\" & \"local\">" template=Values.flat arguments=(int32, 2, "managed" & "local")
"#,
        r#"
"#,
    );
}

/// Infer an extension method's const parameter from a literal argument.
#[test]
fn test_extension_literal_argument_infers_a_const_parameter() {
    let session = TestSession::single(
        r#"
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];

declare class Values<out T> {}

extension<T> of Values<T> {
    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[]
        where T: Clone {
        todo("flat")
    }
}

declare const values: Values<int32>;

const twice: Element<int32, 2>[] = values.flat(2);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];

declare class Values<out T> {}

extension<T> of Values<T> {
    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[]
        where T: Clone {
        todo("flat" as string | undefined)
    }
}

declare const values: Values<int32>;

const twice: Element<int32, 2>[] = values.flat<int32, 2, "managed">(2 as 2 | undefined);

=== dir ===
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];
/// @generic.template symbol=Element parameters=(T#1, const Depth#1: usize)
/// @type.symbol symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" type=Depth#1 extends 0 ? T#1 : T#1[]
/// @definition.type symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" template=(T#1, const Depth#1: usize) value=Depth#1 extends 0 ? T#1 : T#1[]
/// @type.symbol symbol=Element.T source=T type=T#1
/// @type.symbol symbol=Element.Depth source="const Depth: usize" type=Depth#1
/// @resolution.name source=Depth target=Element.Depth
/// @resolution.name source=T target=Element.T
/// @resolution.name source=T target=Element.T

declare class Values<out T> {}
/// @generic.template symbol=Values parameters=(out T#2)
/// @type.symbol symbol=Values source="declare class Values<out T> {}" type=typeof Values
/// @definition.class symbol=Values source="declare class Values<out T> {}" template=(out T#2)
/// @type.symbol symbol=Values.T source="out T" type=T#2

extension<T> of Values<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Values<T#3>
/// @definition.method symbol=flat slot=flat type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly Values<T#3>, Depth#2 | undefined?) => Element<T#3, Depth#2>[]
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Values target=Values
/// @resolution.name source=T target=T

    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[]
    /// @generic.template symbol=flat parent=template#2 parameters=(const Depth#2: usize = 1, 'a)
    /// @type.symbol symbol=flat type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly Values<T#3>, Depth#2 | undefined?) => Element<T#3, Depth#2>[]
    /// @type.symbol symbol=flat.Depth source="const Depth: usize = 1" type=Depth#2
    /// @type.symbol symbol=flat.this source="&readonly this" type=&flat.'a readonly Values<T#3>
    /// @type.symbol symbol=flat.depth source="depth?: Depth" type=Depth#2 | undefined
    /// @resolution.name source=Depth target=flat.Depth
    /// @resolution.name source=Element target=Element
    /// @resolution.name source=T target=T
    /// @resolution.name source=Depth target=flat.Depth

        where T: Clone {
        /// @resolution.name source=T target=T
        /// @resolution.name source=Clone target=Clone

        todo("flat")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"flat\")" parameters=(string | undefined) arguments=(provided("flat") as string | undefined) return=never kind=symbol target=todo

    }
}

declare const values: Values<int32>;
/// @type.symbol symbol=values source=values type=Values<int32>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=Values target=Values

const twice: Element<int32, 2>[] = values.flat(2);
/// @type.symbol symbol=twice source=twice type=Element<int32, 2>[]
/// @resolution.pattern source=twice kind=binding target=twice
/// @resolution.name source=Element target=Element
/// @resolution.name source=values target=values
/// @resolution.member source=values.flat receiver=Values<int32> type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly Values<int32>, Depth#2 | undefined?) => Element<int32, Depth#2>[] kind=symbol target_receiver=Values<int32> target=flat
/// @resolution.call source=values.flat(2) parameters=(2 | undefined) arguments=(provided(2) as 2 | undefined) return=Element<int32, 2>[] regions=("managed" & "local") kind=symbol target=flat receiver=Values<int32> adjustments=(borrow(&'managed readonly Values<int32>)) instance="Values<int32>.<extension#1>.flat<2, \"managed\" & \"local\">"
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @generic.instantiation id="flat<int32, 2, \"managed\" & \"local\">" template=flat arguments=(int32, 2, "managed" & "local")
/// @generic.instantiation id=flat<int32> template=flat arguments=(int32)
"#,
        r#"

"#,
    );
}

/// Keep an expected optional result open across a callback's return statements.
#[test]
fn test_block_callback_returns_solve_through_an_optional_result() {
    let session = TestSession::single(
        r#"
declare function filterMap<T, U>(values: T[], callback: (value: T) => U | undefined): U[];

declare const values: (int32 | undefined)[];

const defined = filterMap(values, (value) => {
    if (value !== undefined) {
        return value;
    }
    return undefined;
});
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function filterMap<T, U>(values: T[], callback: (value: T) => U | undefined): U[];

declare const values: (int32 | undefined)[];

const defined: int32[] = filterMap<int32 | undefined, int32>(
    values,
    (value: int32 | undefined): int32 | undefined => {
        if (value !== (undefined as int32 | undefined)) {
            return value as int32 | undefined;
        }
        return undefined as int32 | undefined;
    },
);

=== dir ===
declare function filterMap<T, U>(values: T[], callback: (value: T) => U | undefined): U[];
/// @generic.template symbol=filterMap parameters=(T, U)
/// @type.symbol symbol=filterMap type=<T, U>(T[], (T) => U | undefined) => U[]
/// @type.symbol symbol=filterMap.T source=T type=T
/// @type.symbol symbol=filterMap.U source=U type=U
/// @resolution.name source=T target=filterMap.T
/// @type.symbol symbol=filterMap.value source="value: T" type=T
/// @resolution.name source=T target=filterMap.T
/// @resolution.name source=U target=filterMap.U
/// @resolution.name source=U target=filterMap.U

declare const values: (int32 | undefined)[];
/// @type.symbol symbol=values source=values type=int32 | undefined[]
/// @resolution.pattern source=values kind=binding target=values

const defined = filterMap(values, (value) => {
/// @type.symbol symbol=defined source=defined type=int32[]
/// @resolution.pattern source=defined kind=binding target=defined
/// @resolution.name source=filterMap target=filterMap
/// @resolution.call parameters=(int32 | undefined[], (int32 | undefined) => int32 | undefined) arguments=(provided(values) as int32 | undefined[], provided(argument) as (int32 | undefined) => int32 | undefined) return=int32[] kind=symbol target=filterMap instance="filterMap<int32 | undefined, int32>"
/// @generic.instantiation id="filterMap<int32 | undefined, int32>" template=filterMap arguments=(int32 | undefined, int32)
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @type.symbol symbol=symbol8 type=Function<(int32 | undefined,), int32 | undefined, "readonly">
/// @type.symbol symbol=symbol8.value source=value type=int32 | undefined

    if (value !== undefined) {
    /// @resolution.name source=value target=symbol8.value
    /// @resolution.operator source="value !== undefined" type=boolean operator="!==" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), undefined as int32 | undefined families=(integer | undefined)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=symbol8.value

        return value;
        /// @resolution.name source=value target=symbol8.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=symbol8.value
        /// @resolution.narrowing source=value union=int32 | undefined arms=int32

    }
    return undefined;
});
"#,
        r#"
"#,
    );
}

/// Infer a const parameter from a literal argument beside a union receiver.
#[test]
fn test_union_receiver_literal_argument_infers_a_const_parameter() {
    let session = TestSession::single(
        r#"
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];

declare class Values<out T> {}

extension<T> of Values<T> {
    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[] {
        todo("flat")
    }
}

declare const values: Values<int32 | int32[]>;

const once: Element<int32 | int32[], 1>[] = values.flat(1);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];

declare class Values<out T> {}

extension<T> of Values<T> {
    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[] {
        todo("flat" as string | undefined)
    }
}

declare const values: Values<int32 | int32[]>;

const once: Element<int32 | int32[], 1>[] = values.flat<int32 | int32[], 1, "managed">(
    1 as 1 | undefined,
);

=== dir ===
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];
/// @generic.template symbol=Element parameters=(T#1, const Depth#1: usize)
/// @type.symbol symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" type=Depth#1 extends 0 ? T#1 : T#1[]
/// @definition.type symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" template=(T#1, const Depth#1: usize) value=Depth#1 extends 0 ? T#1 : T#1[]
/// @type.symbol symbol=Element.T source=T type=T#1
/// @type.symbol symbol=Element.Depth source="const Depth: usize" type=Depth#1
/// @resolution.name source=Depth target=Element.Depth
/// @resolution.name source=T target=Element.T
/// @resolution.name source=T target=Element.T

declare class Values<out T> {}
/// @generic.template symbol=Values parameters=(out T#2)
/// @type.symbol symbol=Values source="declare class Values<out T> {}" type=typeof Values
/// @definition.class symbol=Values source="declare class Values<out T> {}" template=(out T#2)
/// @type.symbol symbol=Values.T source="out T" type=T#2

extension<T> of Values<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Values<T#3>
/// @definition.method symbol=flat slot=flat type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly Values<T#3>, Depth#2 | undefined?) => Element<T#3, Depth#2>[]
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Values target=Values
/// @resolution.name source=T target=T

    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[] {
    /// @generic.template symbol=flat parent=template#2 parameters=(const Depth#2: usize = 1, 'a)
    /// @type.symbol symbol=flat type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly Values<T#3>, Depth#2 | undefined?) => Element<T#3, Depth#2>[]
    /// @type.symbol symbol=flat.Depth source="const Depth: usize = 1" type=Depth#2
    /// @type.symbol symbol=flat.this source="&readonly this" type=&flat.'a readonly Values<T#3>
    /// @type.symbol symbol=flat.depth source="depth?: Depth" type=Depth#2 | undefined
    /// @resolution.name source=Depth target=flat.Depth
    /// @resolution.name source=Element target=Element
    /// @resolution.name source=T target=T
    /// @resolution.name source=Depth target=flat.Depth

        todo("flat")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"flat\")" parameters=(string | undefined) arguments=(provided("flat") as string | undefined) return=never kind=symbol target=todo

    }
}

declare const values: Values<int32 | int32[]>;
/// @type.symbol symbol=values source=values type=Values<int32 | int32[]>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=Values target=Values

const once: Element<int32 | int32[], 1>[] = values.flat(1);
/// @type.symbol symbol=once source=once type=Element<int32 | int32[], 1>[]
/// @resolution.pattern source=once kind=binding target=once
/// @resolution.name source=Element target=Element
/// @resolution.name source=values target=values
/// @resolution.member source=values.flat receiver=Values<int32 | int32[]> type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly Values<int32 | int32[]>, Depth#2 | undefined?) => Element<int32 | int32[], Depth#2>[] kind=symbol target_receiver=Values<int32 | int32[]> target=flat
/// @resolution.call source=values.flat(1) parameters=(1 | undefined) arguments=(provided(1) as 1 | undefined) return=Element<int32 | int32[], 1>[] regions=("managed" & "local") kind=symbol target=flat receiver=Values<int32 | int32[]> adjustments=(borrow(&'managed readonly Values<int32 | int32[]>)) instance="Values<int32 | int32[]>.<extension#1>.flat<1, \"managed\" & \"local\">"
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @generic.instantiation id="flat<int32 | int32[], 1, \"managed\" & \"local\">" template=flat arguments=(int32 | int32[], 1, "managed" & "local")
/// @generic.instantiation id="flat<int32 | int32[]>" template=flat arguments=(int32 | int32[])
"#,
        r#"

"#,
    );
}

/// Satisfy a Copy bound with a union of copyable arms.
#[test]
fn test_union_of_copyable_arms_satisfies_copy() {
    let session = TestSession::single(
        r#"
import { Copy } from "destack:memory";

declare function requireCopy<T: Copy>(value: T): void;

declare const value: int32 | int32[];

requireCopy(value);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Copy } from "destack:memory";

declare function requireCopy<T: Copy>(value: T): void;

declare const value: int32 | int32[];

requireCopy<int32 | int32[]>(value);

=== dir ===
import { Copy } from "destack:memory";

declare function requireCopy<T: Copy>(value: T): void;
/// @generic.template symbol=requireCopy parameters=(T: Copy)
/// @type.symbol symbol=requireCopy source="declare function requireCopy<T: Copy>(value: T): void" type=<T: Copy>(T) => void
/// @type.symbol symbol=requireCopy.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=T target=requireCopy.T

declare const value: int32 | int32[];
/// @type.symbol symbol=value source=value type=int32 | int32[]
/// @resolution.pattern source=value kind=binding target=value

requireCopy(value);
/// @resolution.name source=requireCopy target=requireCopy
/// @resolution.call source=requireCopy(value) parameters=(int32 | int32[]) arguments=(provided(value) as int32 | int32[]) return=void kind=symbol target=requireCopy instance="requireCopy<int32 | int32[]>"
/// @generic.instantiation id="requireCopy<int32 | int32[]>" template=requireCopy arguments=(int32 | int32[])
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
        r#"
"#,
    );
}

/// Flatten a union-element array through the library flat.
#[test]
fn test_flatten_a_union_element_array_by_one_level() {
    let session = TestSession::single(
        r#"
function flatten(values: (int32 | int32[])[]): int32[] {
    return values.flat(1);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function flatten(values: (int32 | int32[])[]): int32[] {
    return values.flat<int32 | int32[], 1, "managed">(1 as 1 | undefined) as int32[];
}

=== dir ===
function flatten(values: (int32 | int32[])[]): int32[] {
/// @type.symbol symbol=flatten type=(int32 | int32[][]) => int32[]
/// @type.symbol symbol=flatten.values source="values: (int32 | int32[])[]" type=int32 | int32[][]

    return values.flat(1);
    /// @resolution.name source=values target=flatten.values
    /// @resolution.member source=values.flat receiver=int32 | int32[][] type=<const flat.Depth: usize = 1, flat.'a>(this: &flat.'a readonly int32 | int32[][], flat.Depth | undefined?) => ^FlattenedElement<int32 | int32[], flat.Depth>[] kind=symbol target_receiver=int32 | int32[][] target=flat
    /// @resolution.call source=values.flat(1) parameters=(1 | undefined) arguments=(provided(1) as 1 | undefined) return=^FlattenedElement<int32 | int32[], 1>[] regions=("managed" & "local") kind=symbol target=flat receiver=int32 | int32[][] adjustments=(borrow(&'managed readonly int32 | int32[][])) instance="Array<int32 | int32[]>.<extension#6>.flat<1, \"managed\" & \"local\">"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=flatten.values
    /// @generic.instantiation id="flat<int32 | int32[], 1, \"managed\" & \"local\">" template=flat arguments=(int32 | int32[], 1, "managed" & "local")
    /// @generic.instantiation id="flat<int32 | int32[]>" template=flat arguments=(int32 | int32[])

}
"#,
        r#"

"#,
    );
}

/// Infer a const parameter under a return expectation through a conditional alias.
#[test]
fn test_return_expectation_keeps_a_const_parameter_exact() {
    let session = TestSession::single(
        r#"
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];

declare class Values<out T> {}

extension<T> of Values<T> {
    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[] {
        todo("flat")
    }
}

declare const values: Values<int32 | int32[]>;

function flatten(): (int32 | int32[])[][] {
    return values.flat(1);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];

declare class Values<out T> {}

extension<T> of Values<T> {
    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[] {
        todo("flat" as string | undefined)
    }
}

declare const values: Values<int32 | int32[]>;

function flatten(): (int32 | int32[])[][] {
    return values.flat<int32 | int32[], 1, "managed">(1 as 1 | undefined);
}

=== dir ===
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];
/// @generic.template symbol=Element parameters=(T#1, const Depth#1: usize)
/// @type.symbol symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" type=Depth#1 extends 0 ? T#1 : T#1[]
/// @definition.type symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" template=(T#1, const Depth#1: usize) value=Depth#1 extends 0 ? T#1 : T#1[]
/// @type.symbol symbol=Element.T source=T type=T#1
/// @type.symbol symbol=Element.Depth source="const Depth: usize" type=Depth#1
/// @resolution.name source=Depth target=Element.Depth
/// @resolution.name source=T target=Element.T
/// @resolution.name source=T target=Element.T

declare class Values<out T> {}
/// @generic.template symbol=Values parameters=(out T#2)
/// @type.symbol symbol=Values source="declare class Values<out T> {}" type=typeof Values
/// @definition.class symbol=Values source="declare class Values<out T> {}" template=(out T#2)
/// @type.symbol symbol=Values.T source="out T" type=T#2

extension<T> of Values<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Values<T#3>
/// @definition.method symbol=flat slot=flat type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly Values<T#3>, Depth#2 | undefined?) => Element<T#3, Depth#2>[]
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Values target=Values
/// @resolution.name source=T target=T

    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[] {
    /// @generic.template symbol=flat parent=template#2 parameters=(const Depth#2: usize = 1, 'a)
    /// @type.symbol symbol=flat type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly Values<T#3>, Depth#2 | undefined?) => Element<T#3, Depth#2>[]
    /// @type.symbol symbol=flat.Depth source="const Depth: usize = 1" type=Depth#2
    /// @type.symbol symbol=flat.this source="&readonly this" type=&flat.'a readonly Values<T#3>
    /// @type.symbol symbol=flat.depth source="depth?: Depth" type=Depth#2 | undefined
    /// @resolution.name source=Depth target=flat.Depth
    /// @resolution.name source=Element target=Element
    /// @resolution.name source=T target=T
    /// @resolution.name source=Depth target=flat.Depth

        todo("flat")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"flat\")" parameters=(string | undefined) arguments=(provided("flat") as string | undefined) return=never kind=symbol target=todo

    }
}

declare const values: Values<int32 | int32[]>;
/// @type.symbol symbol=values source=values type=Values<int32 | int32[]>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=Values target=Values

function flatten(): (int32 | int32[])[][] {
/// @type.symbol symbol=flatten type=() => int32 | int32[][][]

    return values.flat(1);
    /// @resolution.name source=values target=values
    /// @resolution.member source=values.flat receiver=Values<int32 | int32[]> type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly Values<int32 | int32[]>, Depth#2 | undefined?) => Element<int32 | int32[], Depth#2>[] kind=symbol target_receiver=Values<int32 | int32[]> target=flat
    /// @resolution.call source=values.flat(1) parameters=(1 | undefined) arguments=(provided(1) as 1 | undefined) return=Element<int32 | int32[], 1>[] regions=("managed" & "local") kind=symbol target=flat receiver=Values<int32 | int32[]> adjustments=(borrow(&'managed readonly Values<int32 | int32[]>)) instance="Values<int32 | int32[]>.<extension#1>.flat<1, \"managed\" & \"local\">"
    /// @resolution.place source=values placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=values root=values
    /// @generic.instantiation id="flat<int32 | int32[], 1, \"managed\" & \"local\">" template=flat arguments=(int32 | int32[], 1, "managed" & "local")
    /// @generic.instantiation id="flat<int32 | int32[]>" template=flat arguments=(int32 | int32[])

}
"#,
        r#"

"#,
    );
}

/// Narrow a union operand through loose nullish equality.
#[test]
fn test_loose_nullish_equality_narrows_a_union_operand() {
    let session = TestSession::single(
        r#"
declare const value: int32 | null | undefined;

if (value != null) {
    value satisfies int32;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: int32 | null | undefined;

if (value != null) {
    value satisfies int32;
}

=== dir ===
declare const value: int32 | null | undefined;
/// @type.symbol symbol=value source=value type=int32 | null | undefined
/// @resolution.pattern source=value kind=binding target=value

if (value != null) {
/// @type.node source="value != null" type=boolean
/// @type.node source=value type=int32 | null | undefined
/// @resolution.name source=value target=value
/// @resolution.operator source="value != null" type=boolean operator="!=" kind=builtin operands=[value as int32 | null | undefined families=(integer | null | undefined), null as null families=(null)]
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @type.node source=null type=null

    value satisfies int32;
    /// @type.node source="value satisfies int32" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=value root=value
    /// @resolution.narrowing source=value union=int32 | null | undefined arms=int32

}
"#,
        r#"
"#,
    );
}

/// Compare a borrowed scalar with an intrinsic operator.
#[test]
fn test_borrowed_scalar_compares_with_an_intrinsic_operator() {
    let session = TestSession::single(
        r#"
declare const values: ^int32[];

const positive = values.filter((value) => value > 0);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: ^int32[];

const positive: ^int32[] = values.filter<int32>(
    (value: &immutable int32): boolean => (value as int32) > 0,
);

=== dir ===
declare const values: ^int32[];
/// @type.symbol symbol=values source=values type=^int32[]
/// @resolution.pattern source=values kind=binding target=values

const positive = values.filter((value) => value > 0);
/// @type.symbol symbol=positive source=positive type=^int32[]
/// @resolution.pattern source=positive kind=binding target=positive
/// @resolution.name source=values target=values
/// @resolution.member source=values.filter receiver=^int32[] type=(this: ^int32[], <type_expression.'a>(&type_expression.'a immutable int32, isize) => boolean) => ^int32[] kind=symbol target_receiver=^int32[] target=filter#1
/// @resolution.call source="values.filter((value) => value > 0)" parameters=(<type_expression.'a>(&type_expression.'a immutable int32, isize) => boolean) arguments=(provided((value) => value > 0) as <type_expression.'a>(&type_expression.'a immutable int32, isize) => boolean) return=^int32[] kind=symbol target=filter#1 receiver=^int32[] instance=^T#3[].<extension#3>.filter#1
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @generic.instantiation id=filter#1<int32> template=filter#1 arguments=(int32)
/// @type.symbol symbol=symbol2 source="(value) => value > 0" type=Function<(&type_expression.'a immutable int32,), boolean, "readonly">
/// @type.symbol symbol=symbol2.value source=value type=&type_expression.'a immutable int32
/// @resolution.name source=value target=symbol2.value
/// @resolution.operator source="value > 0" type=boolean operator=">" kind=builtin operands=[value as int32 families=(integer), 0 as int32 families=(integer)]
/// @resolution.place source=value placement=type_expression.'a lifetime=type_expression.'a access="immutable"
/// @resolution.access source=value root=symbol2.value
"#,
        r#"

"#,
    );
}

/// Instantiate an extension through explicit receiver type arguments.
#[test]
fn test_extension_selection_substitutes_explicit_receiver_arguments() {
    let session = TestSession::single(
        r#"
declare class Holder<out T> {}

extension<T> of Holder<T> {
    static wrap(value: T): Holder<T> {
        todo("wrap")
    }
}

const held: Holder<int32> = Holder<int32>.wrap(42);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare class Holder<out T> {}

extension<T> of Holder<T> {
    static wrap(value: T): Holder<T> {
        todo("wrap" as string | undefined)
    }
}

const held: Holder<int32> = Holder<int32>.wrap<int32>(42);

=== dir ===
declare class Holder<out T> {}
/// @generic.template symbol=Holder parameters=(out T#1)
/// @type.symbol symbol=Holder source="declare class Holder<out T> {}" type=typeof Holder
/// @definition.class symbol=Holder source="declare class Holder<out T> {}" template=(out T#1)
/// @type.symbol symbol=Holder.T source="out T" type=T#1

extension<T> of Holder<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Holder<T#2>
/// @definition.method symbol=wrap slot=wrap static=true type=(T#2) => Holder<T#2>
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=T

    static wrap(value: T): Holder<T> {
    /// @type.symbol symbol=wrap type=(T#2) => Holder<T#2>
    /// @type.symbol symbol=wrap.value source="value: T" type=T#2
    /// @resolution.name source=T target=T
    /// @resolution.name source=Holder target=Holder
    /// @resolution.name source=T target=T

        todo("wrap")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"wrap\")" parameters=(string | undefined) arguments=(provided("wrap") as string | undefined) return=never kind=symbol target=todo

    }
}

const held: Holder<int32> = Holder<int32>.wrap(42);
/// @type.symbol symbol=held source=held type=Holder<int32>
/// @resolution.pattern source=held kind=binding target=held
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Holder<int32> target=Holder
/// @resolution.member source=Holder<int32>.wrap receiver=typeof Holder<int32> type=(int32) => Holder<int32> kind=symbol target_receiver=typeof Holder<int32> target=wrap
/// @resolution.call source=Holder<int32>.wrap(42) parameters=(int32) arguments=(provided(42) as int32) return=Holder<int32> kind=symbol target=wrap instance=Holder<int32>.<extension#1>.wrap
/// @generic.instantiation id=wrap<int32> template=wrap arguments=(int32)
"#,
        r#"
"#,
    );
}

/// Iterate an integer range without a float default.
#[test]
fn test_integer_range_iterates_with_an_integer_element() {
    let session = TestSession::single(
        r#"
for (const value of 0..10) {
    value;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
for (const value of 0..10) {
    value;
}

=== dir ===
for (const value of 0..10) {
/// @resolution.iteration iterator="iterator#1(parameters=(), arguments=(), return=RangeIterator<int64>)" next="next(parameters=(), arguments=(), return=IteratorResult<int64, void>, regions=(\"frame\" & \"local\"))"
/// @generic.instantiation id="next<int64, \"frame\" & \"local\">" template=next arguments=(int64, "frame" & "local")
/// @generic.instantiation id=iterator#1<int64> template=iterator#1 arguments=(int64)
/// @type.symbol symbol=value source=value type=int64
/// @resolution.pattern source=value kind=binding target=value

    value;
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=value root=value

}
"#,
        r#"
"#,
    );
}

/// A value inhabiting both cases of a union parameter cannot pick the type argument.
#[test]
fn test_argument_matching_both_union_arms_requires_annotation() {
    let session = TestSession::single(
        r#"
declare class Box<in out T> {}
declare function pick<T>(value: Box<T> | Box<Box<T>>): T;
declare const boxed: Box<Box<int32>>;

const picked = pick(boxed);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<in out T> {}
declare function pick<T>(value: Box<T> | Box<Box<T>>): T;
declare const boxed: Box<Box<int32>>;

const picked = pick(boxed);

=== dir ===
declare class Box<in out T> {}
/// @generic.template symbol=Box parameters=(in out T#1)
/// @type.symbol symbol=Box source="declare class Box<in out T> {}" type=typeof Box
/// @definition.class symbol=Box source="declare class Box<in out T> {}" template=(in out T#1)
/// @type.symbol symbol=Box.T source="in out T" type=T#1

declare function pick<T>(value: Box<T> | Box<Box<T>>): T;
/// @generic.template symbol=pick parameters=(T#2)
/// @type.symbol symbol=pick source="declare function pick<T>(value: Box<T> | Box<Box<T>>): T" type=<T#2>(Box<T#2> | Box<Box<T#2>>) => T#2
/// @type.symbol symbol=pick.T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=pick.T
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=pick.T
/// @resolution.name source=T target=pick.T

declare const boxed: Box<Box<int32>>;
/// @type.symbol symbol=boxed source=boxed type=Box<Box<int32>>
/// @resolution.pattern source=boxed kind=binding target=boxed
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box target=Box

const picked = pick(boxed);
/// @type.symbol symbol=picked source=picked type=<error>
/// @resolution.pattern source=picked kind=binding target=picked
/// @type.node source=pick type=(Box<<error>> | Box<Box<<error>>>) => <error>
/// @type.node source=pick(boxed) type=<error>
/// @resolution.name source=pick target=pick
/// @resolution.call source=pick(boxed) parameters=(Box<<error>> | Box<Box<<error>>>) arguments=(provided(boxed) as Box<<error>> | Box<Box<<error>>>) return=<error> kind=symbol target=pick instance=pick<<error>>
/// @type.node source=boxed type=Box<Box<int32>>
/// @resolution.name source=boxed target=boxed
/// @resolution.place source=boxed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=boxed root=boxed
"#,
        r#"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=6 column=21 span="boxed" line_source="const picked = pick(boxed);"
/// @diagnostic.help message="annotate the type explicitly"
"#,
    );
}

/// Nested optional-result callbacks with block bodies solve both type arguments.
#[test]
fn test_nested_block_callbacks_solve_through_optional_results() {
    let session = TestSession::single(
        r#"
declare function collect<T, U>(values: T[], step: (value: T) => U | undefined): U[];
declare const starts: (int32 | undefined)[];

const doubled = collect(collect(starts, (start) => {
    if (start !== (undefined as int32 | undefined)) {
        return start;
    }
    return undefined;
}), (value) => {
    if (value > 3) {
        return value * 2;
    }
    return undefined;
});
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function collect<T, U>(values: T[], step: (value: T) => U | undefined): U[];
declare const starts: (int32 | undefined)[];

const doubled: int32[] = collect<int32, int32>(
    collect<int32 | undefined, int32>(starts, (start: int32 | undefined): int32 | undefined => {
        if (start !== (undefined as int32 | undefined)) {
            return start as int32 | undefined;
        }
        return undefined as int32 | undefined;
    }),
    (value: int32): int32 | undefined => {
        if (value > 3) {
            return (value * 2) as int32 | undefined;
        }
        return undefined as int32 | undefined;
    },
);

=== dir ===
declare function collect<T, U>(values: T[], step: (value: T) => U | undefined): U[];
/// @generic.template symbol=collect parameters=(T, U)
/// @type.symbol symbol=collect type=<T, U>(T[], (T) => U | undefined) => U[]
/// @type.symbol symbol=collect.T source=T type=T
/// @type.symbol symbol=collect.U source=U type=U
/// @resolution.name source=T target=collect.T
/// @type.symbol symbol=collect.value source="value: T" type=T
/// @resolution.name source=T target=collect.T
/// @resolution.name source=U target=collect.U
/// @resolution.name source=U target=collect.U

declare const starts: (int32 | undefined)[];
/// @type.symbol symbol=starts source=starts type=int32 | undefined[]
/// @resolution.pattern source=starts kind=binding target=starts

const doubled = collect(collect(starts, (start) => {
/// @type.symbol symbol=doubled source=doubled type=int32[]
/// @resolution.pattern source=doubled kind=binding target=doubled
/// @type.node source=collect type=(int32[], (int32) => int32 | undefined) => int32[]
/// @type.node type=int32[]
/// @resolution.name source=collect target=collect
/// @resolution.call parameters=(int32[], (int32) => int32 | undefined) arguments=(provided(argument) as int32[], provided(argument) as (int32) => int32 | undefined) return=int32[] kind=symbol target=collect instance="collect<int32, int32>"
/// @generic.instantiation id="collect<int32, int32>" template=collect arguments=(int32, int32)
/// @type.node source=collect type=(int32 | undefined[], (int32 | undefined) => int32 | undefined) => int32[]
/// @type.node type=int32[]
/// @resolution.name source=collect target=collect
/// @resolution.call parameters=(int32 | undefined[], (int32 | undefined) => int32 | undefined) arguments=(provided(starts) as int32 | undefined[], provided(argument) as (int32 | undefined) => int32 | undefined) return=int32[] kind=symbol target=collect instance="collect<int32 | undefined, int32>"
/// @generic.instantiation id="collect<int32 | undefined, int32>" template=collect arguments=(int32 | undefined, int32)
/// @type.node source=starts type=int32 | undefined[]
/// @resolution.name source=starts target=starts
/// @resolution.place source=starts placement="local" lifetime="static" access="immutable"
/// @resolution.access source=starts root=starts
/// @type.symbol symbol=symbol8 type=Function<(int32 | undefined,), int32 | undefined, "readonly">
/// @type.node type=Function<(int32 | undefined,), int32 | undefined, "readonly">
/// @type.symbol symbol=symbol8.start source=start type=int32 | undefined

    if (start !== (undefined as int32 | undefined)) {
    /// @type.node source="start !== (undefined as int32 | undefined)" type=boolean
    /// @type.node source=start type=int32 | undefined
    /// @resolution.name source=start target=symbol8.start
    /// @resolution.operator source="start !== (undefined as int32 | undefined)" type=boolean operator="!==" kind=builtin operands=[start as int32 | undefined families=(integer | undefined), undefined as int32 | undefined as int32 | undefined families=(integer | undefined)]
    /// @resolution.place source=start placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=start root=symbol8.start
    /// @type.node source="undefined as int32 | undefined" type=int32 | undefined
    /// @type.node source=undefined type=undefined

        return start;
        /// @type.node source=start type=int32
        /// @resolution.name source=start target=symbol8.start
        /// @resolution.place source=start placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=start root=symbol8.start
        /// @resolution.narrowing source=start union=int32 | undefined arms=int32

    }
    return undefined;
    /// @type.node source=undefined type=undefined

}), (value) => {
/// @type.symbol symbol=symbol10 type=Function<(int32,), int32 | undefined, "readonly">
/// @type.node type=Function<(int32,), int32 | undefined, "readonly">
/// @type.symbol symbol=symbol10.value source=value type=int32

    if (value > 3) {
    /// @type.node source="value > 3" type=boolean
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=symbol10.value
    /// @resolution.operator source="value > 3" type=boolean operator=">" kind=builtin operands=[value as int32 families=(integer), 3 as int32 families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=symbol10.value
    /// @type.node source=3 type=3

        return value * 2;
        /// @type.node source="value * 2" type=int32
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=symbol10.value
        /// @resolution.operator source="value * 2" type=int32 operator="*" kind=builtin operands=[value as int32 families=(integer), 2 as int32 families=(integer)]
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=symbol10.value
        /// @type.node source=2 type=2

    }
    return undefined;
    /// @type.node source=undefined type=undefined

});
"#,
        r#"
"#,
    );
}

/// Infer a nested result through an overloaded promise callback.
#[test]
fn test_overloaded_promise_callback_infers_nested_result() {
    let session = TestSession::single(
        r#"
declare class Promise<T> {
    then<U>(onFulfilled: (value: T) => U | Promise<U>): Promise<U>;
}

struct Ok<T> {
    kind: "ok" = "ok";
    value: T;
}

struct Err<E> {
    kind: "err" = "err";
    error: E;
}

newtype Result<T, E> = Ok<T> | Err<E>;

declare function result<T, E>(): Result<T, E>;

extension<T, E> of Result<T, E> {
    andThen<U, F>(f: (value: T) => Result<U, F>): Result<U, E | F> {
        result<U, E | F>()
    }
}

newtype AsyncResult<T, E> = Promise<Result<T, E>>;

extension<T, E> of AsyncResult<T, E> {
    andThenSync<U, F>(f: (value: T) => Result<U, F>): AsyncResult<U, E | F> {
        AsyncResult(this.then((result) => result.andThen(f)))
    }
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
declare class Promise<out T> {
    then<U>(onFulfilled: (value: T) => U | Promise<U>): Promise<U>;
}

struct Ok<out T> {
    kind: "ok" = "ok";
    value: T;
}

struct Err<out E> {
    kind: "err" = "err";
    error: E;
}

newtype Result<out T, out E> = Ok<T> | Err<E>;

declare function result<T, E>(): Result<T, E>;

extension<T, E> of Result<T, E> {
    andThen<U, F>(f: (value: T) => Result<U, F>): Result<U, E | F> {
        result<U, E | F>()
    }
}

newtype AsyncResult<out T, out E> = Promise<Result<T, E>>;

extension<T, E> of AsyncResult<T, E> {
    andThenSync<U, F>(f: (value: T) => Result<U, F>): AsyncResult<U, E | F> {
        AsyncResult(
            this.then<Result<T, E>, Result<U, E | F>>(
                (result: Result<T, E>): Result<U, E | F> | Promise<Result<U, E | F>> =>
                    result.andThen<T, E, U, F>(f) as Result<U, E | F> | Promise<Result<U, E | F>>,
            ),
        )
    }
}

=== dir ===
declare class Promise<T> {
/// @generic.template symbol=Promise parameters=(out T#1)
/// @type.symbol symbol=Promise type=typeof Promise
/// @definition.class symbol=Promise template=(out T#1)
/// @definition.method symbol=Promise.then source="then<U>(onFulfilled: (value: T) => U | Promise<U>): Promise<U>" slot=then type=<U#1>(this: Promise<T#1>, (T#1) => U#1 | Promise<U#1>) => Promise<U#1>
/// @type.symbol symbol=Promise.T source=T type=T#1

    then<U>(onFulfilled: (value: T) => U | Promise<U>): Promise<U>;
    /// @generic.template symbol=Promise.then parent=template#0 parameters=(U#1)
    /// @type.symbol symbol=Promise.then source="then<U>(onFulfilled: (value: T) => U | Promise<U>): Promise<U>" type=<U#1>(this: Promise<T#1>, (T#1) => U#1 | Promise<U#1>) => Promise<U#1>
    /// @generic.instance id=Promise<U#1> template=Promise arguments=(U#1)
    /// @type.symbol symbol=Promise.then.U source=U type=U#1
    /// @type.symbol symbol=Promise.then.onFulfilled source="onFulfilled: (value: T) => U | Promise<U>" type=(T#1) => U#1 | Promise<U#1>
    /// @type.symbol symbol=Promise.then.value source="value: T" type=T#1
    /// @resolution.name source=T target=Promise.T
    /// @resolution.name source=U target=Promise.then.U
    /// @resolution.name source=Promise target=Promise
    /// @resolution.name source=U target=Promise.then.U
    /// @resolution.name source=Promise target=Promise
    /// @resolution.name source=U target=Promise.then.U

}

struct Ok<T> {
/// @generic.template symbol=Ok parameters=(out T#2)
/// @type.symbol symbol=Ok type=Ok
/// @definition.struct symbol=Ok template=(out T#2)
/// @definition.field symbol=Ok.kind source="kind: \"ok\" = \"ok\"" key=kind type="ok"
/// @definition.field symbol=Ok.value source="value: T" key=value type=T#2
/// @type.symbol symbol=Ok.T source=T type=T#2

    kind: "ok" = "ok";
    /// @type.symbol symbol=Ok.kind source="kind: \"ok\" = \"ok\"" type="ok"
    /// @type.node source="\"ok\"" type="ok"

    value: T;
    /// @type.symbol symbol=Ok.value source="value: T" type=T#2
    /// @resolution.name source=T target=Ok.T

}

struct Err<E> {
/// @generic.template symbol=Err parameters=(out E#1)
/// @type.symbol symbol=Err type=Err
/// @definition.struct symbol=Err template=(out E#1)
/// @definition.field symbol=Err.error source="error: E" key=error type=E#1
/// @definition.field symbol=Err.kind source="kind: \"err\" = \"err\"" key=kind type="err"
/// @type.symbol symbol=Err.E source=E type=E#1

    kind: "err" = "err";
    /// @type.symbol symbol=Err.kind source="kind: \"err\" = \"err\"" type="err"
    /// @type.node source="\"err\"" type="err"

    error: E;
    /// @type.symbol symbol=Err.error source="error: E" type=E#1
    /// @resolution.name source=E target=Err.E

}

newtype Result<T, E> = Ok<T> | Err<E>;
/// @generic.template symbol=Result parameters=(out T#3, out E#2)
/// @type.symbol symbol=Result source="newtype Result<T, E> = Ok<T> | Err<E>" type=Result
/// @generic.instance id=Err<E#2> template=Err arguments=(E#2)
/// @generic.instance id=Ok<T#3> template=Ok arguments=(T#3)
/// @definition.newtype symbol=Result source="newtype Result<T, E> = Ok<T> | Err<E>" template=(out T#3, out E#2) backing=Ok<T#3> | Err<E#2> constructors=[<T#3, E#2>(Ok<T#3>) => Result<T#3, E#2>, <T#3, E#2>(Err<E#2>) => Result<T#3, E#2>, <T#3, E#2>(Ok<T#3> | Err<E#2>) => Result<T#3, E#2>]
/// @type.symbol symbol=Result.T source=T type=T#3
/// @type.symbol symbol=Result.E source=E type=E#2
/// @resolution.name source=Ok target=Ok
/// @resolution.name source=T target=Result.T
/// @resolution.name source=Err target=Err
/// @resolution.name source=E target=Result.E

declare function result<T, E>(): Result<T, E>;
/// @generic.template symbol=result parameters=(T#4, E#3)
/// @type.symbol symbol=result source="declare function result<T, E>(): Result<T, E>" type=<T#4, E#3>() => Result<T#4, E#3>
/// @generic.instance id="Result<T#4, E#3>" template=Result arguments=(T#4, E#3)
/// @generic.instance id=Err<E#3> template=Err arguments=(E#3)
/// @generic.instance id=Ok<T#4> template=Ok arguments=(T#4)
/// @type.symbol symbol=result.T source=T type=T#4
/// @type.symbol symbol=result.E source=E type=E#3
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=result.T
/// @resolution.name source=E target=result.E

extension<T, E> of Result<T, E> {
/// @generic.template symbol=<module>#2 parameters=(T#5, E#4)
/// @generic.instance id="Result<T#5, E#4>" template=Result arguments=(T#5, E#4)
/// @generic.instance id=Err<E#4> template=Err arguments=(E#4)
/// @generic.instance id=Ok<T#5> template=Ok arguments=(T#5)
/// @definition.extension symbol=<module>#2 form=local target=Result<T#5, E#4>
/// @definition.method symbol=andThen slot=andThen type=<U#2, F#1>(this: Result<T#5, E#4>, (T#5) => Result<U#2, F#1>) => Result<U#2, E#4 | F#1>
/// @type.symbol symbol=T#1 source=T type=T#5
/// @type.symbol symbol=E#1 source=E type=E#4
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=T#1
/// @resolution.name source=E target=E#1

    andThen<U, F>(f: (value: T) => Result<U, F>): Result<U, E | F> {
    /// @generic.template symbol=andThen parent=template#5 parameters=(U#2, F#1)
    /// @type.symbol symbol=andThen type=<U#2, F#1>(this: Result<T#5, E#4>, (T#5) => Result<U#2, F#1>) => Result<U#2, E#4 | F#1>
    /// @type.symbol symbol=andThen.this type=Result<T#5, E#4>
    /// @generic.instance id="Err<E#4 | F#1>" template=Err arguments=(E#4 | F#1)
    /// @generic.instance id="Result<U#2, E#4 | F#1>" template=Result arguments=(U#2, E#4 | F#1)
    /// @generic.instance id="Result<U#2, F#1>" template=Result arguments=(U#2, F#1)
    /// @generic.instance id=Err<F#1> template=Err arguments=(F#1)
    /// @generic.instance id=Ok<U#2> template=Ok arguments=(U#2)
    /// @type.symbol symbol=andThen.U source=U type=U#2
    /// @type.symbol symbol=andThen.F source=F type=F#1
    /// @type.symbol symbol=andThen.f source="f: (value: T) => Result<U, F>" type=(T#5) => Result<U#2, F#1>
    /// @type.symbol symbol=andThen.value source="value: T" type=T#5
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=Result target=Result
    /// @resolution.name source=U target=andThen.U
    /// @resolution.name source=F target=andThen.F
    /// @resolution.name source=Result target=Result
    /// @resolution.name source=U target=andThen.U
    /// @resolution.name source=E target=E#1
    /// @resolution.name source=F target=andThen.F

        result<U, E | F>()
        /// @type.node source="result<U, E | F>()" type=Result<U#2, E#4 | F#1>
        /// @type.node source=result type=() => Result<U#2, E#4 | F#1>
        /// @resolution.name source=result target=result
        /// @resolution.call source="result<U, E | F>()" parameters=() return=Result<U#2, E#4 | F#1> kind=symbol target=result instance="result<U#2, E#4 | F#1>"
        /// @generic.instantiation id="result<U#2, E#4 | F#1>" template=result arguments=(U#2, E#4 | F#1) owner=andThen
        /// @generic.instance id="result<U#2, E#4 | F#1>" template=result arguments=(U#2, E#4 | F#1)
        /// @resolution.name source=U target=andThen.U
        /// @resolution.name source=E target=E#1
        /// @resolution.name source=F target=andThen.F

    }
}

newtype AsyncResult<T, E> = Promise<Result<T, E>>;
/// @generic.template symbol=AsyncResult parameters=(out T#6, out E#5)
/// @type.symbol symbol=AsyncResult source="newtype AsyncResult<T, E> = Promise<Result<T, E>>" type=AsyncResult
/// @generic.instance id="Promise<Result<T#6, E#5>>" template=Promise arguments=(Result<T#6, E#5>)
/// @generic.instance id="Result<T#6, E#5>" template=Result arguments=(T#6, E#5)
/// @generic.instance id=Err<E#5> template=Err arguments=(E#5)
/// @generic.instance id=Ok<T#6> template=Ok arguments=(T#6)
/// @definition.newtype symbol=AsyncResult source="newtype AsyncResult<T, E> = Promise<Result<T, E>>" template=(out T#6, out E#5) backing=Promise<Result<T#6, E#5>> constructors=[<T#6, E#5>(Promise<Result<T#6, E#5>>) => AsyncResult<T#6, E#5>]
/// @type.symbol symbol=AsyncResult.T source=T type=T#6
/// @type.symbol symbol=AsyncResult.E source=E type=E#5
/// @resolution.name source=Promise target=Promise
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=AsyncResult.T
/// @resolution.name source=E target=AsyncResult.E

extension<T, E> of AsyncResult<T, E> {
/// @generic.template symbol=<module>#3 parameters=(T#7, E#6)
/// @generic.instance id="AsyncResult<T#7, E#6>" template=AsyncResult arguments=(T#7, E#6)
/// @definition.extension symbol=<module>#3 form=local target=AsyncResult<T#7, E#6>
/// @definition.method symbol=andThenSync slot=andThenSync type=<U#3, F#2>(this: AsyncResult<T#7, E#6>, (T#7) => Result<U#3, F#2>) => AsyncResult<U#3, E#6 | F#2>
/// @type.symbol symbol=T#2 source=T type=T#7
/// @type.symbol symbol=E#2 source=E type=E#6
/// @resolution.name source=AsyncResult target=AsyncResult
/// @resolution.name source=T target=T#2
/// @resolution.name source=E target=E#2

    andThenSync<U, F>(f: (value: T) => Result<U, F>): AsyncResult<U, E | F> {
    /// @generic.template symbol=andThenSync parent=template#7 parameters=(U#3, F#2)
    /// @type.symbol symbol=andThenSync type=<U#3, F#2>(this: AsyncResult<T#7, E#6>, (T#7) => Result<U#3, F#2>) => AsyncResult<U#3, E#6 | F#2>
    /// @type.symbol symbol=andThenSync.this type=AsyncResult<T#7, E#6>
    /// @generic.instance id="AsyncResult<U#3, E#6 | F#2>" template=AsyncResult arguments=(U#3, E#6 | F#2)
    /// @generic.instance id="Result<U#3, F#2>" template=Result arguments=(U#3, F#2)
    /// @generic.instance id=Err<F#2> template=Err arguments=(F#2)
    /// @generic.instance id=Ok<U#3> template=Ok arguments=(U#3)
    /// @type.symbol symbol=andThenSync.U source=U type=U#3
    /// @type.symbol symbol=andThenSync.F source=F type=F#2
    /// @type.symbol symbol=andThenSync.f source="f: (value: T) => Result<U, F>" type=(T#7) => Result<U#3, F#2>
    /// @type.symbol symbol=andThenSync.value source="value: T" type=T#7
    /// @resolution.name source=T target=T#2
    /// @resolution.name source=Result target=Result
    /// @resolution.name source=U target=andThenSync.U
    /// @resolution.name source=F target=andThenSync.F
    /// @resolution.name source=AsyncResult target=AsyncResult
    /// @resolution.name source=U target=andThenSync.U
    /// @resolution.name source=E target=E#2
    /// @resolution.name source=F target=andThenSync.F

        AsyncResult(this.then((result) => result.andThen(f)))
        /// @type.node source="AsyncResult(this.then((result) => result.andThen(f)))" type=AsyncResult<U#3, E#6 | F#2>
        /// @type.node source=AsyncResult type=AsyncResult
        /// @resolution.name source=AsyncResult target=AsyncResult
        /// @resolution.construct source="AsyncResult(this.then((result) => result.andThen(f)))" parameters=(Promise<Result<U#3, E#6 | F#2>>) arguments=(provided(this.then((result) => result.andThen(f))) as Promise<Result<U#3, E#6 | F#2>>) return=AsyncResult<U#3, E#6 | F#2> kind=newtype target=AsyncResult backing=Promise<Result<U#3, E#6 | F#2>> instance="AsyncResult<U#3, E#6 | F#2>"
        /// @generic.instantiation id="AsyncResult<U#3, E#6 | F#2>" template=AsyncResult arguments=(U#3, E#6 | F#2) owner=andThenSync
        /// @type.node source="this.then((result) => result.andThen(f))" type=Promise<Result<U#3, E#6 | F#2>>
        /// @type.node source=this type=AsyncResult<T#7, E#6>
        /// @type.node source=this.then type=<U#1>(this: Promise<Result<T#7, E#6>>, (Result<T#7, E#6>) => U#1 | Promise<U#1>) => Promise<U#1>
        /// @resolution.member source=this.then receiver=AsyncResult<T#7, E#6> type=<U#1>(this: Promise<Result<T#7, E#6>>, (Result<T#7, E#6>) => U#1 | Promise<U#1>) => Promise<U#1> kind=symbol target_receiver=AsyncResult<T#7, E#6> adjustments=(newtype.payload(AsyncResult, Promise<Result<T#7, E#6>>)) target=Promise.then
        /// @resolution.call source="this.then((result) => result.andThen(f))" parameters=((Result<T#7, E#6>) => Result<U#3, E#6 | F#2> | Promise<Result<U#3, E#6 | F#2>>) arguments=(provided((result) => result.andThen(f)) as (Result<T#7, E#6>) => Result<U#3, E#6 | F#2> | Promise<Result<U#3, E#6 | F#2>>) return=Promise<Result<U#3, E#6 | F#2>> kind=symbol target=Promise.then receiver=AsyncResult<T#7, E#6> adjustments=(newtype.payload(AsyncResult, Promise<Result<T#7, E#6>>)) instance="Promise<Result<T#7, E#6>>.then<Result<U#3, E#6 | F#2>>"
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=AsyncResult<T#7, E#6>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="AsyncResult<T#7, E#6>" template=AsyncResult arguments=(T#7, E#6) owner=andThenSync
        /// @generic.instantiation id="Promise.then<Result<T#7, E#6>, Result<U#3, E#6 | F#2>>" template=Promise.then arguments=(Result<T#7, E#6>, Result<U#3, E#6 | F#2>) owner=andThenSync
        /// @generic.instantiation id="Promise.then<Result<T#7, E#6>>" template=Promise.then arguments=(Result<T#7, E#6>) owner=andThenSync
        /// @generic.instance id="Promise.then<Result<T#7, E#6>, Result<U#3, E#6 | F#2>>" template=Promise.then arguments=(Result<T#7, E#6>, Result<U#3, E#6 | F#2>)
        /// @generic.instance id="Promise<Result<T#7, E#6>>" template=Promise arguments=(Result<T#7, E#6>)
        /// @type.symbol symbol=andThenSync.symbol47 source=(result) => result.andThen(f) type=Function<(Result<T#7, E#6>,), Result<U#3, E#6 | F#2> | Promise<Result<U#3, E#6 | F#2>>, "readonly">
        /// @type.node source=(result) => result.andThen(f) type=Function<(Result<T#7, E#6>,), Result<U#3, E#6 | F#2> | Promise<Result<U#3, E#6 | F#2>>, "readonly">
        /// @generic.instance id="Err<E#6 | F#2>" template=Err arguments=(E#6 | F#2)
        /// @generic.instance id="Promise<Result<U#3, E#6 | F#2>>" template=Promise arguments=(Result<U#3, E#6 | F#2>)
        /// @generic.instance id="Result<T#7, E#6>" template=Result arguments=(T#7, E#6)
        /// @generic.instance id="Result<U#3, E#6 | F#2>" template=Result arguments=(U#3, E#6 | F#2)
        /// @generic.instance id=Err<E#6> template=Err arguments=(E#6)
        /// @generic.instance id=Ok<T#7> template=Ok arguments=(T#7)
        /// @type.symbol symbol=andThenSync.symbol47.result source=result type=Result<T#7, E#6>
        /// @type.node source=result type=Result<T#7, E#6>
        /// @type.node source=result.andThen type=<U#2, F#1>(this: Result<T#7, E#6>, (T#7) => Result<U#2, F#1>) => Result<U#2, E#6 | F#1>
        /// @type.node source=result.andThen(f) type=Result<U#3, E#6 | F#2>
        /// @resolution.name source=result target=andThenSync.symbol47.result
        /// @resolution.member source=result.andThen receiver=Result<T#7, E#6> type=<U#2, F#1>(this: Result<T#7, E#6>, (T#7) => Result<U#2, F#1>) => Result<U#2, E#6 | F#1> kind=symbol target_receiver=Result<T#7, E#6> target=andThen
        /// @resolution.call source=result.andThen(f) parameters=((T#7) => Result<U#3, F#2>) arguments=(provided(f) as (T#7) => Result<U#3, F#2>) return=Result<U#3, E#6 | F#2> kind=symbol target=andThen receiver=Result<T#7, E#6> instance="Result<T#7, E#6>.<extension#1>.andThen<U#3, F#2>"
        /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result root=andThenSync.symbol47.result
        /// @generic.instantiation id="andThen<T#7, E#6, U#3, F#2>" template=andThen arguments=(T#7, E#6, U#3, F#2) owner=andThenSync
        /// @generic.instantiation id="andThen<T#7, E#6>" template=andThen arguments=(T#7, E#6) owner=andThenSync
        /// @generic.instance id="andThen<T#7, E#6, U#3, F#2>" template=andThen arguments=(T#7, E#6, U#3, F#2)
        /// @generic.instance id="result<U#3, E#6 | F#2>" template=result arguments=(U#3, E#6 | F#2)
        /// @type.node source=f type=(T#7) => Result<U#3, F#2>
        /// @resolution.name source=f target=andThenSync.f
        /// @resolution.place source=f placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=f root=andThenSync.f

    }
}
"#);
}

/// Infer the parameters of both callbacks in a chained map and filter call.
#[test]
fn test_infer_a_map_and_filter_chain_with_arithmetic_callbacks() {
    let session = TestSession::single(
        r#"
function positive(values: int32[]): int32[] {
    return values.map((value) => value + 1).filter((value) => value > 0);
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
function positive(values: int32[]): int32[] {
    return values.map<int32, int32, "managed">((value: int32): int32 => value + 1).filter<int32>(
        (value: &immutable int32): boolean => (value as int32) > 0,
    ) as int32[];
}

=== dir ===
function positive(values: int32[]): int32[] {
/// @type.symbol symbol=positive type=(int32[]) => int32[]
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @type.symbol symbol=positive.values source="values: int32[]" type=int32[]

    return values.map((value) => value + 1).filter((value) => value > 0);
    /// @type.node source="values.map((value) => value + 1)" type=^int32[]
    /// @type.node source="values.map((value) => value + 1).filter" type=(this: ^int32[], <type_expression.'a>(&type_expression.'a immutable int32, isize) => boolean) => ^int32[]
    /// @type.node source="values.map((value) => value + 1).filter((value) => value > 0)" type=^int32[]
    /// @type.node source=values type=int32[]
    /// @type.node source=values.map type=<map.U#2, map#2.'a>(this: &map#2.'a readonly int32[], (int32, isize) => map.U#2) => ^map.U#2[]
    /// @resolution.name source=values target=positive.values
    /// @resolution.member source="values.map((value) => value + 1).filter" receiver=^int32[] type=(this: ^int32[], <type_expression.'a>(&type_expression.'a immutable int32, isize) => boolean) => ^int32[] kind=symbol target_receiver=^int32[] target=filter#1
    /// @resolution.member source=values.map receiver=int32[] type=<map.U#2, map#2.'a>(this: &map#2.'a readonly int32[], (int32, isize) => map.U#2) => ^map.U#2[] kind=symbol target_receiver=int32[] target=map#2
    /// @resolution.call source="values.map((value) => value + 1)" parameters=((int32, isize) => int32) arguments=(provided((value) => value + 1) as (int32, isize) => int32) return=^int32[] regions=("managed" & "local") kind=symbol target=map#2 receiver=int32[] adjustments=(borrow(&'managed readonly int32[])) instance="Array<int32>.<extension#4>.map#2<int32, \"managed\" & \"local\">"
    /// @resolution.call source="values.map((value) => value + 1).filter((value) => value > 0)" parameters=(<type_expression.'a>(&type_expression.'a immutable int32, isize) => boolean) arguments=(provided((value) => value > 0) as <type_expression.'a>(&type_expression.'a immutable int32, isize) => boolean) return=^int32[] kind=symbol target=filter#1 receiver=^int32[] instance=^T#3[].<extension#3>.filter#1
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=positive.values
    /// @generic.instantiation id="map#2<int32, int32, \"managed\" & \"local\">" template=map#2 arguments=(int32, int32, "managed" & "local")
    /// @generic.instantiation id=filter#1<int32> template=filter#1 arguments=(int32)
    /// @generic.instantiation id=map#2<int32> template=map#2 arguments=(int32)
    /// @generic.instance id="map#2<int32, int32, \"bound0\" & \"local\">" template=map#2 arguments=(int32, int32, "bound0" & "local")
    /// @generic.instance id=filter#1<int32> template=filter#1 arguments=(int32)
    /// @type.symbol symbol=positive.symbol3 source="(value) => value + 1" type=Function<(int32,), int32, "readonly">
    /// @type.node source="(value) => value + 1" type=Function<(int32,), int32, "readonly">
    /// @type.symbol symbol=positive.symbol3.value source=value type=int32
    /// @type.node source="value + 1" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=positive.symbol3.value
    /// @resolution.operator source="value + 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=positive.symbol3.value
    /// @type.node source=1 type=1
    /// @type.symbol symbol=positive.symbol5 source="(value) => value > 0" type=Function<(&type_expression.'a immutable int32,), boolean, "readonly">
    /// @type.node source="(value) => value > 0" type=Function<(&type_expression.'a immutable int32,), boolean, "readonly">
    /// @type.symbol symbol=positive.symbol5.value source=value type=&type_expression.'a immutable int32
    /// @type.node source="value > 0" type=boolean
    /// @type.node source=value type=&type_expression.'a immutable int32
    /// @resolution.name source=value target=positive.symbol5.value
    /// @resolution.operator source="value > 0" type=boolean operator=">" kind=builtin operands=[value as int32 families=(integer), 0 as int32 families=(integer)]
    /// @resolution.place source=value placement=type_expression.'a lifetime=type_expression.'a access="immutable"
    /// @resolution.access source=value root=positive.symbol5.value
    /// @type.node source=0 type=0

}
"#);
}

/// Infer the filter callback that compares its parameter against undefined.
#[test]
fn test_infer_a_map_and_filter_chain_with_a_nullish_comparison() {
    let session = TestSession::single(
        r#"
function defined(values: (int32 | undefined)[]): (int32 | undefined)[] {
    return values.map((value) => value).filter((value) => value !== undefined);
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
function defined(values: (int32 | undefined)[]): (int32 | undefined)[] {
    return values.map<int32 | undefined, int32 | undefined, "managed">(
        (value: int32 | undefined): int32 | undefined => value,
    ).filter<int32 | undefined>(
        (value: &immutable (int32 | undefined)): boolean =>
            value !== (undefined as int32 | undefined),
    ) as (int32 | undefined)[];
}

=== dir ===
function defined(values: (int32 | undefined)[]): (int32 | undefined)[] {
/// @type.symbol symbol=defined type=(int32 | undefined[]) => int32 | undefined[]
/// @generic.instance id="Array<int32 | undefined>" template=Array arguments=(int32 | undefined)
/// @generic.instance id="sliceAssumeInit<MaybeUninit<int32 | undefined>>" template=sliceAssumeInit arguments=(MaybeUninit<int32 | undefined>)
/// @generic.instance id="sliceUninit<MaybeUninit<int32 | undefined>>" template=sliceUninit arguments=(MaybeUninit<int32 | undefined>)
/// @type.symbol symbol=defined.values source="values: (int32 | undefined)[]" type=int32 | undefined[]

    return values.map((value) => value).filter((value) => value !== undefined);
    /// @type.node source="values.map((value) => value)" type=^int32 | undefined[]
    /// @type.node source="values.map((value) => value).filter" type=(this: ^int32 | undefined[], <type_expression.'a>(&type_expression.'a immutable (int32 | undefined), isize) => boolean) => ^int32 | undefined[]
    /// @type.node source="values.map((value) => value).filter((value) => value !== undefined)" type=^int32 | undefined[]
    /// @type.node source=values type=int32 | undefined[]
    /// @type.node source=values.map type=<map.U#2, map#2.'a>(this: &map#2.'a readonly int32 | undefined[], (int32 | undefined, isize) => map.U#2) => ^map.U#2[]
    /// @resolution.name source=values target=defined.values
    /// @resolution.member source="values.map((value) => value).filter" receiver=^int32 | undefined[] type=(this: ^int32 | undefined[], <type_expression.'a>(&type_expression.'a immutable (int32 | undefined), isize) => boolean) => ^int32 | undefined[] kind=symbol target_receiver=^int32 | undefined[] target=filter#1
    /// @resolution.member source=values.map receiver=int32 | undefined[] type=<map.U#2, map#2.'a>(this: &map#2.'a readonly int32 | undefined[], (int32 | undefined, isize) => map.U#2) => ^map.U#2[] kind=symbol target_receiver=int32 | undefined[] target=map#2
    /// @resolution.call source="values.map((value) => value)" parameters=((int32 | undefined, isize) => int32 | undefined) arguments=(provided((value) => value) as (int32 | undefined, isize) => int32 | undefined) return=^int32 | undefined[] regions=("managed" & "local") kind=symbol target=map#2 receiver=int32 | undefined[] adjustments=(borrow(&'managed readonly int32 | undefined[])) instance="Array<int32 | undefined>.<extension#4>.map#2<int32 | undefined, \"managed\" & \"local\">"
    /// @resolution.call source="values.map((value) => value).filter((value) => value !== undefined)" parameters=(<type_expression.'a>(&type_expression.'a immutable (int32 | undefined), isize) => boolean) arguments=(provided((value) => value !== undefined) as <type_expression.'a>(&type_expression.'a immutable (int32 | undefined), isize) => boolean) return=^int32 | undefined[] kind=symbol target=filter#1 receiver=^int32 | undefined[] instance=^T#3[].<extension#3>.filter#1
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=defined.values
    /// @generic.instantiation id="filter#1<int32 | undefined>" template=filter#1 arguments=(int32 | undefined)
    /// @generic.instantiation id="map#2<int32 | undefined, int32 | undefined, \"managed\" & \"local\">" template=map#2 arguments=(int32 | undefined, int32 | undefined, "managed" & "local")
    /// @generic.instantiation id="map#2<int32 | undefined>" template=map#2 arguments=(int32 | undefined)
    /// @generic.instance id="filter#1<int32 | undefined>" template=filter#1 arguments=(int32 | undefined)
    /// @generic.instance id="map#2<int32 | undefined, int32 | undefined, \"bound0\" & \"local\">" template=map#2 arguments=(int32 | undefined, int32 | undefined, "bound0" & "local")
    /// @type.symbol symbol=defined.symbol3 source="(value) => value" type=Function<(int32 | undefined,), int32 | undefined, "readonly">
    /// @type.node source="(value) => value" type=Function<(int32 | undefined,), int32 | undefined, "readonly">
    /// @type.symbol symbol=defined.symbol3.value source=value type=int32 | undefined
    /// @type.node source=value type=int32 | undefined
    /// @resolution.name source=value target=defined.symbol3.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=defined.symbol3.value
    /// @type.symbol symbol=defined.symbol5 source="(value) => value !== undefined" type=Function<(&type_expression.'a immutable (int32 | undefined),), boolean, "readonly">
    /// @type.node source="(value) => value !== undefined" type=Function<(&type_expression.'a immutable (int32 | undefined),), boolean, "readonly">
    /// @type.symbol symbol=defined.symbol5.value source=value type=&type_expression.'a immutable (int32 | undefined)
    /// @type.node source="value !== undefined" type=boolean
    /// @type.node source=value type=&type_expression.'a immutable (int32 | undefined)
    /// @resolution.name source=value target=defined.symbol5.value
    /// @resolution.operator source="value !== undefined" type=boolean operator="!==" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), undefined as int32 | undefined families=(integer | undefined)]
    /// @resolution.place source=value placement=type_expression.'a lifetime=type_expression.'a access="immutable"
    /// @resolution.access source=value root=defined.symbol5.value
    /// @type.node source=undefined type=undefined

}
"#);
}

#[test]
fn test_widen_the_literal_initial_value_in_a_reduce_call() {
    let session = TestSession::single(
        r#"
function containsPositive(values: int32[]): boolean {
    return values.reduce(
        (found, value) => found || value > 0,
        false,
    );
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_node_types(), r#"
=== annotated ===
function containsPositive(values: int32[]): boolean {
    return values.reduce<int32, boolean, "managed">(
        (found: boolean, value: int32): boolean => found || value > 0,
        false,
    );
}

=== dir ===
function containsPositive(values: int32[]): boolean {
/// @type.symbol symbol=containsPositive type=(int32[]) => boolean
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @type.symbol symbol=containsPositive.values source="values: int32[]" type=int32[]

    return values.reduce(
    /// @type.node source=values.reduce type=<reduce.U#2, reduce#2.'a>(this: &reduce#2.'a readonly int32[], (reduce.U#2, int32, isize) => reduce.U#2, reduce.U#2) => reduce.U#2
    /// @type.node type=boolean
    /// @resolution.name source=values target=containsPositive.values
    /// @resolution.member source=values.reduce receiver=int32[] type=<reduce.U#2, reduce#2.'a>(this: &reduce#2.'a readonly int32[], (reduce.U#2, int32, isize) => reduce.U#2, reduce.U#2) => reduce.U#2 kind=symbol target_receiver=int32[] target=reduce#2
    /// @resolution.call parameters=((boolean, int32, isize) => boolean, boolean) arguments=(provided((found, value) => found || value > 0) as (boolean, int32, isize) => boolean, provided(false) as boolean) return=boolean regions=("managed" & "local") kind=symbol target=reduce#2 receiver=int32[] adjustments=(borrow(&'managed readonly int32[])) instance="Array<int32>.<extension#4>.reduce#2<boolean, \"managed\" & \"local\">"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=containsPositive.values
    /// @generic.instantiation id="reduce#2<int32, boolean, \"managed\" & \"local\">" template=reduce#2 arguments=(int32, boolean, "managed" & "local")
    /// @generic.instantiation id=reduce#2<int32> template=reduce#2 arguments=(int32)
    /// @generic.instance id="reduce#2<int32, boolean, \"bound0\" & \"local\">" template=reduce#2 arguments=(int32, boolean, "bound0" & "local")

        (found, value) => found || value > 0,
        /// @type.symbol symbol=containsPositive.symbol3 source="(found, value) => found || value > 0" type=Function<(boolean, int32), boolean, "readonly">
        /// @type.node source="(found, value) => found || value > 0" type=Function<(boolean, int32), boolean, "readonly">
        /// @type.symbol symbol=containsPositive.symbol3.found source=found type=boolean
        /// @type.symbol symbol=containsPositive.symbol3.value source=value type=int32
        /// @type.node source="found || value > 0" type=boolean
        /// @resolution.name source=found target=containsPositive.symbol3.found
        /// @resolution.operator source="found || value > 0" type=boolean operator="||" kind=builtin operands=[found as boolean families=(boolean), value > 0 as boolean families=(boolean)]
        /// @resolution.place source=found placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=found root=containsPositive.symbol3.found
        /// @type.node source="value > 0" type=boolean
        /// @resolution.name source=value target=containsPositive.symbol3.value
        /// @resolution.operator source="value > 0" type=boolean operator=">" kind=builtin operands=[value as int32 families=(integer), 0 as int32 families=(integer)]
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=containsPositive.symbol3.value
        /// @type.node source=0 type=0

        false,
        /// @type.node source=false type=false

    );
}
"#);
}

#[test]
fn test_fill_a_defaulted_interface_argument_at_a_partial_annotation() {
    let session = TestSession::single(
        r#"
newtype interface It<T, R = void> {
    next(this): R {
        todo("next")
    }

    first(this): T | undefined {
        todo("first")
    }

    count(this): isize {
        todo("count")
    }
}

function length(values: It<int32>): isize {
    return values.count();
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked().with_node_types(), r#"
=== annotated ===
newtype interface It<out T, out R = void> {
    next(this): R {
        todo("next" as string | undefined)
    }

    first(this): T | undefined {
        todo("first" as string | undefined)
    }

    count(this): isize {
        todo("count" as string | undefined)
    }
}

function length(values: It<int32>): isize {
    return values.count<int32, void>();
}

=== dir ===
newtype interface It<T, R = void> {
/// @generic.template symbol=It parameters=(out T, out R = void, this: It<T, R>)
/// @type.symbol symbol=It type=It
/// @definition.interface symbol=It template=(out T, out R = void, this: It<T, R>) nominal=true
/// @definition.where symbol=It relation=satisfies left=this right=It<T, R>
/// @definition.method symbol=It.count slot=count type=(this: this) => isize
/// @definition.method symbol=It.first slot=first type=(this: this) => T | undefined
/// @definition.method symbol=It.next slot=next type=(this: this) => R
/// @type.symbol symbol=It.T source=T type=T
/// @type.symbol symbol=It.R source="R = void" type=R

    next(this): R {
    /// @type.symbol symbol=It.next type=(this: this) => R
    /// @type.symbol symbol=It.next.this source=this type=this
    /// @resolution.name source=R target=It.R

        todo("next")
        /// @type.node source="todo(\"next\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"next\")" parameters=(string | undefined) arguments=(provided("next") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"next\"" type="next"

    }

    first(this): T | undefined {
    /// @type.symbol symbol=It.first type=(this: this) => T | undefined
    /// @type.symbol symbol=It.first.this source=this type=this
    /// @resolution.name source=T target=It.T

        todo("first")
        /// @type.node source="todo(\"first\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"first\")" parameters=(string | undefined) arguments=(provided("first") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"first\"" type="first"

    }

    count(this): isize {
    /// @type.symbol symbol=It.count type=(this: this) => isize
    /// @type.symbol symbol=It.count.this source=this type=this

        todo("count")
        /// @type.node source="todo(\"count\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"count\")" parameters=(string | undefined) arguments=(provided("count") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"count\"" type="count"

    }
}

function length(values: It<int32>): isize {
/// @type.symbol symbol=length type=(It<int32, void>) => isize
/// @generic.instance id="It<int32, void>" template=It arguments=(int32, void)
/// @type.symbol symbol=length.values source="values: It<int32>" type=It<int32, void>
/// @resolution.name source=It target=It

    return values.count();
    /// @type.node source=values.count type=(this: It<int32, void>) => isize
    /// @type.node source=values.count() type=isize
    /// @resolution.name source=values target=length.values
    /// @resolution.member source=values.count receiver=It<int32, void> type=(this: It<int32, void>) => isize kind=symbol target_receiver=It<int32, void> dispatch=dynamic constraint=It<int32, void> target=It.count
    /// @resolution.call source=values.count() parameters=() return=isize kind=dynamic target=It.count receiver=It<int32, void> constraint=It<int32, void> generic_arguments=(int32, void)
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=length.values
    /// @generic.instantiation id="It.count<int32, void>" template=It.count arguments=(int32, void)

}
"#);
}

#[test]
fn test_reject_a_cyclic_type_argument() {
    let session = TestSession::single(
        r#"
declare function fix<T>(step: (value: T) => T): T;

const result = fix((value) => [value]);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function fix<T>(step: (value: T) => T): T;

const result = fix((value) => [value]);

=== dir ===
declare function fix<T>(step: (value: T) => T): T;
/// @generic.template symbol=fix parameters=(T)
/// @type.symbol symbol=fix source="declare function fix<T>(step: (value: T) => T): T" type=<T>((T) => T) => T
/// @type.symbol symbol=fix.T source=T type=T
/// @type.symbol symbol=fix.value source="value: T" type=T
/// @resolution.name source=T target=fix.T
/// @resolution.name source=T target=fix.T
/// @resolution.name source=T target=fix.T

const result = fix((value) => [value]);
/// @type.symbol symbol=result source=result type=<error>[]
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=fix target=fix
/// @resolution.call source="fix((value) => [value])" parameters=((<error>[]) => <error>[]) arguments=(provided((value) => [value]) as (<error>[]) => <error>[]) return=<error>[] kind=symbol target=fix instance=fix<<error>[]>
/// @type.symbol symbol=symbol5 source="(value) => [value]" type=Function<(<error>[],), <error>[], "readonly">
/// @type.symbol symbol=symbol5.value source=value type=<error>[]
/// @resolution.call source=[value] parameters=(^Slice<<error>>) arguments=(rest(provided(value) as <error>) as <error>) return=<error>[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<<error>>
/// @resolution.name source=value target=symbol5.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol5.value
"#,
        r#"
/// @diagnostic.error id=circular-type message="type is circular"
/// @diagnostic.label line=4 column=31 span="[value]" line_source="const result = fix((value) => [value]);"
"#,
    );
}

/// Infer a type argument through an intersection argument without a return expectation.
#[test]
fn test_infer_a_type_argument_through_an_intersection_argument() {
    let session = TestSession::single(
        r#"
interface Safe {}

declare function run<T>(body: ^Function<(), T, "once"> & Safe): T;

struct Runner<R> {
    body: ^Function<(), R, "once"> & Safe;

    go(this): void {
        const result = run(this.body);
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Safe {}

declare function run<T>(body: ^Function<(), T, "once"> & Safe): T;

struct Runner<R> {
    body: ^Function<(), R, "once"> & Safe;

    go(this): void {
        const result: R = run<R>(this.body);
    }
}

=== dir ===
interface Safe {}
/// @generic.template symbol=Safe parameters=(this: Safe)
/// @type.symbol symbol=Safe source="interface Safe {}" type=Safe
/// @definition.interface symbol=Safe source="interface Safe {}" template=(this: Safe)
/// @definition.where symbol=Safe source="interface Safe {}" relation=satisfies left=this right=Safe

declare function run<T>(body: ^Function<(), T, "once"> & Safe): T;
/// @generic.template symbol=run parameters=(T)
/// @type.symbol symbol=run source="declare function run<T>(body: ^Function<(), T, \"once\"> & Safe): T" type=<T>(^Function<(), T, "once"> & Safe) => T
/// @type.symbol symbol=run.T source=T type=T
/// @resolution.name source=Function target=Function
/// @resolution.name source=T target=run.T
/// @resolution.name source=Safe target=Safe
/// @resolution.name source=T target=run.T

struct Runner<R> {
/// @generic.template symbol=Runner parameters=(R)
/// @type.symbol symbol=Runner type=Runner
/// @definition.struct symbol=Runner template=(R)
/// @definition.field symbol=Runner.body source="body: ^Function<(), R, \"once\"> & Safe" key=body type=^Function<(), R, "once"> & Safe
/// @definition.method symbol=Runner.go slot=go type=(this: Runner<R>) => void
/// @type.symbol symbol=Runner.R source=R type=R

    body: ^Function<(), R, "once"> & Safe;
    /// @type.symbol symbol=Runner.body source="body: ^Function<(), R, \"once\"> & Safe" type=^Function<(), R, "once"> & Safe
    /// @resolution.name source=Function target=Function
    /// @resolution.name source=R target=Runner.R
    /// @resolution.name source=Safe target=Safe

    go(this): void {
    /// @type.symbol symbol=Runner.go type=(this: Runner<R>) => void
    /// @type.symbol symbol=Runner.go.this source=this type=Runner<R>

        const result = run(this.body);
        /// @type.symbol symbol=Runner.go.result source=result type=R
        /// @resolution.pattern source=result kind=binding target=Runner.go.result
        /// @resolution.name source=run target=run
        /// @resolution.call source=run(this.body) parameters=(^Function<(), R, "once"> & Safe) arguments=(provided(this.body) as ^Function<(), R, "once"> & Safe) return=R kind=symbol target=run instance=run<R>
        /// @generic.instantiation id=run<R> template=run arguments=(R) owner=Runner.go
        /// @resolution.member source=this.body receiver=Runner<R> type=^Function<(), R, "once"> & Safe kind=field target_receiver=Runner<R> key=body target=Runner.body target_type=^Function<(), R, "once"> & Safe
        /// @resolution.receiver source=this kind=this declaration=Runner type=Runner<R>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.body placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.body root=this keys=[body]

    }
}
"#, r#"

"#);
}

/// A cast settles its operand's call by the arguments before the target judges the result.
#[test]
fn test_settle_a_generic_call_by_its_arguments_before_a_cast() {
    let session = TestSession::single(
        r#"
newtype Box<T> = T;

declare function address<T>(value: &readonly T): *T;

export extension<T> of Box<T> {
    backing(&readonly this): *T {
        return address(this) as *T;
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype Box<out T> = T;

declare function address<T, 'a>(value: &readonly T): *T;

export extension<T> of Box<T> {
    backing(&readonly this): *T {
        return address<Box<T>, 'a>(this) as *T;
    }
}

=== dir ===
newtype Box<T> = T;
/// @generic.template symbol=Box parameters=(out T#1)
/// @type.symbol symbol=Box source="newtype Box<T> = T" type=Box
/// @definition.newtype symbol=Box source="newtype Box<T> = T" template=(out T#1) backing=T#1 constructors=[<T#1>(T#1) => Box<T#1>]
/// @type.symbol symbol=Box.T source=T type=T#1
/// @resolution.name source=T target=Box.T

declare function address<T>(value: &readonly T): *T;
/// @generic.template symbol=address parameters=(T#2, 'a)
/// @type.symbol symbol=address source="declare function address<T>(value: &readonly T): *T" type=<T#2, address.'a>(&address.'a readonly T#2) => *T#2
/// @type.symbol symbol=address.T source=T type=T#2
/// @resolution.name source=T target=address.T
/// @resolution.name source=T target=address.T

export extension<T> of Box<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @generic.instance id=Box<T#3> template=Box arguments=(T#3)
/// @definition.extension symbol=<module>#2 form=exported target=Box<T#3>
/// @definition.method symbol=backing slot=backing type=<backing.'a>(this: &backing.'a readonly Box<T#3>) => *T#3
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T

    backing(&readonly this): *T {
    /// @generic.template symbol=backing parent=template#2 parameters=('a)
    /// @type.symbol symbol=backing type=<backing.'a>(this: &backing.'a readonly Box<T#3>) => *T#3
    /// @type.symbol symbol=backing.this source="&readonly this" type=&backing.'a readonly Box<T#3>
    /// @resolution.name source=T target=T

        return address(this) as *T;
        /// @resolution.name source=address target=address
        /// @resolution.call source=address(this) parameters=(&backing.'a readonly Box<T#3>) arguments=(provided(this) as &backing.'a readonly Box<T#3>) return=*Box<T#3> regions=(backing.'a) kind=symbol target=address instance="address<Box<T#3>, backing.'a>"
        /// @generic.instantiation id="address<Box<T#3>, backing.'a>" template=address arguments=(Box<T#3>, backing.'a) owner=backing
        /// @generic.instance id="address<Box<T#3>, backing.'a>" template=address arguments=(Box<T#3>, backing.'a)
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&backing.'a readonly Box<T#3>
        /// @resolution.place source=this placement=backing.'a lifetime=backing.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.name source=T target=T

    }
}
"#,
    );
}
