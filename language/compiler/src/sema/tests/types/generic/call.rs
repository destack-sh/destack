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

    session.assert_dir_checked(
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

=== checked ===
import { Result } from "destack:error";

declare function first(): Result<int32, string>;
/// @type.symbol symbol=first source="declare function first(): Result<int32, string>" type=() => error.result.Result<int32, string>
/// @resolution.name source=Result target=error.result.Result

declare function second(value: int32): Result<boolean, string>;
/// @type.symbol symbol=second source="declare function second(value: int32): Result<boolean, string>" type=(int32) => error.result.Result<boolean, string>
/// @type.symbol symbol=second.value source="value: int32" type=int32
/// @resolution.name source=Result target=error.result.Result

const result: Result<boolean, string> = first().andThen((value) => second(value));
/// @type.symbol symbol=result source=result type=error.result.Result<boolean, string>
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Result target=error.result.Result
/// @type.node source="first().andThen((value) => second(value))" type=error.result.Result<boolean, string>
/// @type.node source=first type=() => error.result.Result<int32, string>
/// @type.node source=first() type=error.result.Result<int32, string>
/// @type.node source=first().andThen type=<error.result.andThen.U, error.result.andThen.F>(this: error.result.Result<int32, string>, Function<(int32,), error.result.Result<error.result.andThen.U, error.result.andThen.F>>) => error.result.Result<error.result.andThen.U, string | error.result.andThen.F>
/// @resolution.name source=first target=first
/// @resolution.member source=first().andThen receiver=error.result.Result<int32, string> type=<error.result.andThen.U, error.result.andThen.F>(this: error.result.Result<int32, string>, Function<(int32,), error.result.Result<error.result.andThen.U, error.result.andThen.F>>) => error.result.Result<error.result.andThen.U, string | error.result.andThen.F> kind=symbol target_receiver=error.result.Result<int32, string> target=error.result.andThen
/// @resolution.call source="first().andThen((value) => second(value))" parameters=(Function<(int32,), error.result.Result<boolean, string>>) arguments=(provided((value) => second(value)) as Function<(int32,), error.result.Result<boolean, string>>) return=error.result.Result<boolean, string> kind=symbol target=error.result.andThen receiver=error.result.Result<int32, string> instance="error.result.Result<int32, string>.<extension#1>.andThen<boolean, string>"
/// @resolution.call source=first() parameters=() return=error.result.Result<int32, string> kind=symbol target=first
/// @generic.instance source="first().andThen((value) => second(value))" id="error.result.Result<boolean, string>"
/// @generic.instance source="first().andThen((value) => second(value))" id="error.result.Result<int32, string>.<extension#1>.andThen<boolean, string>"
/// @generic.instance source=first id="error.result.Result<int32, string>"
/// @generic.instance source=first() id="error.result.Result<int32, string>"
/// @generic.instance source=first().andThen id="error.result.Result<error.result.andThen.U, error.result.andThen.F>"
/// @generic.instance source=first().andThen id="error.result.Result<error.result.andThen.U, string | error.result.andThen.F>"
/// @generic.instance source=first().andThen id="error.result.Result<int32, string>"
/// @type.symbol symbol=symbol5 source=(value) => second(value) type=Function<(int32,), error.result.Result<boolean, string>>
/// @type.node source=(value) => second(value) type=Function<(int32,), error.result.Result<boolean, string>>
/// @generic.instance source=(value) => second(value) id="error.result.Result<boolean, string>"
/// @type.symbol symbol=symbol5.value source=value type=int32
/// @type.node source=second type=(int32) => error.result.Result<boolean, string>
/// @type.node source=second(value) type=error.result.Result<boolean, string>
/// @resolution.name source=second target=second
/// @resolution.call source=second(value) parameters=(int32) arguments=(provided(value) as int32) return=error.result.Result<boolean, string> kind=symbol target=second
/// @generic.instance source=second id="error.result.Result<boolean, string>"
/// @generic.instance source=second(value) id="error.result.Result<boolean, string>"
/// @type.node source=value type=int32
/// @resolution.name source=value target=symbol5.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol5.value

/// @generic.instance id="error.result.Result<boolean, string>" template=error.result.Result arguments=(boolean, string)
/// @generic.instance id="error.result.Result<error.result.andThen.U, error.result.andThen.F>" template=error.result.Result arguments=(error.result.andThen.U, error.result.andThen.F)
/// @generic.instance id="error.result.Result<error.result.andThen.U, string | error.result.andThen.F>" template=error.result.Result arguments=(error.result.andThen.U, string | error.result.andThen.F)
/// @generic.instance id="error.result.Result<int32, string>" template=error.result.Result arguments=(int32, string)
/// @generic.instance id="error.result.Result<int32, string>.<extension#1>.andThen<boolean, string>" template=error.result.andThen arguments=(int32, string, boolean, string)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
const result: Promise<string> = input.then<int32, string>((): string => "done");

=== checked ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
/// @type.symbol symbol=input source=input type=async.promise.Promise<int32>
/// @resolution.pattern source=input kind=binding target=input
/// @resolution.name source=Promise target=async.promise.Promise

const result: Promise<string> = input.then(() => "done");
/// @type.symbol symbol=result source=result type=async.promise.Promise<string>
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Promise target=async.promise.Promise
/// @type.node source="input.then(() => \"done\")" type=async.promise.Promise<string>
/// @type.node source=input type=async.promise.Promise<int32>
/// @type.node source=input.then type=<async.promise.Promise.then.U#1: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise<async.promise.Promise.then.U#1>>) => async.promise.Promise<async.promise.Promise.then.U#1> & <async.promise.Promise.then.U#2: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise.then.U#2>) => async.promise.Promise<async.promise.Promise.then.U#2>
/// @resolution.name source=input target=input
/// @resolution.member source=input.then receiver=async.promise.Promise<int32> type=<async.promise.Promise.then.U#1: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise<async.promise.Promise.then.U#1>>) => async.promise.Promise<async.promise.Promise.then.U#1> & <async.promise.Promise.then.U#2: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise.then.U#2>) => async.promise.Promise<async.promise.Promise.then.U#2> kind=existential targets=[async.promise.Promise.then#1, async.promise.Promise.then#2]
/// @resolution.call source="input.then(() => \"done\")" parameters=(Function<(int32,), string>) arguments=(provided(() => "done") as Function<(int32,), string>) return=async.promise.Promise<string> kind=symbol target=async.promise.Promise.then#2 receiver=async.promise.Promise<int32> instance=async.promise.Promise<int32>.then#2<string>
/// @resolution.place source=input placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=input root=input
/// @generic.instance source="input.then(() => \"done\")" id=async.promise.Promise<int32>.then#2<string>
/// @generic.instance source="input.then(() => \"done\")" id=async.promise.Promise<string>
/// @generic.instance source=input id=async.promise.Promise<int32>
/// @generic.instance source=input.then id=async.promise.Promise<async.promise.Promise.then.U#1>
/// @generic.instance source=input.then id=async.promise.Promise<async.promise.Promise.then.U#2>
/// @generic.instance source=input.then id=async.promise.Promise<int32>
/// @type.symbol symbol=symbol3 source="() => \"done\"" type=Function<(), string>
/// @type.node source="() => \"done\"" type=Function<(), string>
/// @type.node source="\"done\"" type="done"

/// @generic.instance id=async.promise.Promise<async.promise.Promise.then.U#1> template=async.promise.Promise arguments=(async.promise.Promise.then.U#1)
/// @generic.instance id=async.promise.Promise<async.promise.Promise.then.U#2> template=async.promise.Promise arguments=(async.promise.Promise.then.U#2)
/// @generic.instance id=async.promise.Promise<int32> template=async.promise.Promise arguments=(int32)
/// @generic.instance id=async.promise.Promise<int32>.then#2<string> template=async.promise.Promise.then#2 arguments=(int32, string)
/// @generic.instance id=async.promise.Promise<string> template=async.promise.Promise arguments=(string)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
declare const next: Promise<string>;
const result: Promise<string> = input.then<int32, string>((): Promise<string> => next);

=== checked ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
/// @type.symbol symbol=input source=input type=async.promise.Promise<int32>
/// @resolution.pattern source=input kind=binding target=input
/// @resolution.name source=Promise target=async.promise.Promise

declare const next: Promise<string>;
/// @type.symbol symbol=next source=next type=async.promise.Promise<string>
/// @resolution.pattern source=next kind=binding target=next
/// @resolution.name source=Promise target=async.promise.Promise

const result: Promise<string> = input.then(() => next);
/// @type.symbol symbol=result source=result type=async.promise.Promise<string>
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Promise target=async.promise.Promise
/// @type.node source="input.then(() => next)" type=async.promise.Promise<string>
/// @type.node source=input type=async.promise.Promise<int32>
/// @type.node source=input.then type=<async.promise.Promise.then.U#1: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise<async.promise.Promise.then.U#1>>) => async.promise.Promise<async.promise.Promise.then.U#1> & <async.promise.Promise.then.U#2: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise.then.U#2>) => async.promise.Promise<async.promise.Promise.then.U#2>
/// @resolution.name source=input target=input
/// @resolution.member source=input.then receiver=async.promise.Promise<int32> type=<async.promise.Promise.then.U#1: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise<async.promise.Promise.then.U#1>>) => async.promise.Promise<async.promise.Promise.then.U#1> & <async.promise.Promise.then.U#2: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise.then.U#2>) => async.promise.Promise<async.promise.Promise.then.U#2> kind=existential targets=[async.promise.Promise.then#1, async.promise.Promise.then#2]
/// @resolution.call source="input.then(() => next)" parameters=(Function<(int32,), async.promise.Promise<string>>) arguments=(provided(() => next) as Function<(int32,), async.promise.Promise<string>>) return=async.promise.Promise<string> kind=symbol target=async.promise.Promise.then#1 receiver=async.promise.Promise<int32> instance=async.promise.Promise<int32>.then#1<string>
/// @resolution.place source=input placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=input root=input
/// @generic.instance source="input.then(() => next)" id=async.promise.Promise<int32>.then#1<string>
/// @generic.instance source="input.then(() => next)" id=async.promise.Promise<string>
/// @generic.instance source=input id=async.promise.Promise<int32>
/// @generic.instance source=input.then id=async.promise.Promise<async.promise.Promise.then.U#1>
/// @generic.instance source=input.then id=async.promise.Promise<async.promise.Promise.then.U#2>
/// @generic.instance source=input.then id=async.promise.Promise<int32>
/// @type.symbol symbol=symbol4 source="() => next" type=Function<(), async.promise.Promise<string>>
/// @type.node source="() => next" type=Function<(), async.promise.Promise<string>>
/// @generic.instance source="() => next" id=async.promise.Promise<string>
/// @type.node source=next type=async.promise.Promise<string>
/// @resolution.name source=next target=next
/// @resolution.place source=next placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=next root=next
/// @generic.instance source=next id=async.promise.Promise<string>

/// @generic.instance id=async.promise.Promise<async.promise.Promise.then.U#1> template=async.promise.Promise arguments=(async.promise.Promise.then.U#1)
/// @generic.instance id=async.promise.Promise<async.promise.Promise.then.U#2> template=async.promise.Promise arguments=(async.promise.Promise.then.U#2)
/// @generic.instance id=async.promise.Promise<int32> template=async.promise.Promise arguments=(int32)
/// @generic.instance id=async.promise.Promise<int32>.then#1<string> template=async.promise.Promise.then#1 arguments=(int32, string)
/// @generic.instance id=async.promise.Promise<string> template=async.promise.Promise arguments=(string)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
const result: Promise<string> = input.then<int32, string>((value: int32): Promise<string> => {
    input.then<int32, string>((): string => "done")
});

=== checked ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
/// @type.symbol symbol=input source=input type=async.promise.Promise<int32>
/// @resolution.pattern source=input kind=binding target=input
/// @resolution.name source=Promise target=async.promise.Promise

const result: Promise<string> = input.then((value) => {
/// @type.symbol symbol=result source=result type=async.promise.Promise<string>
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Promise target=async.promise.Promise
/// @type.node source=input type=async.promise.Promise<int32>
/// @type.node source=input.then type=<async.promise.Promise.then.U#1: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise<async.promise.Promise.then.U#1>>) => async.promise.Promise<async.promise.Promise.then.U#1> & <async.promise.Promise.then.U#2: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise.then.U#2>) => async.promise.Promise<async.promise.Promise.then.U#2>
/// @type.node type=async.promise.Promise<string>
/// @resolution.name source=input target=input
/// @resolution.member source=input.then receiver=async.promise.Promise<int32> type=<async.promise.Promise.then.U#1: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise<async.promise.Promise.then.U#1>>) => async.promise.Promise<async.promise.Promise.then.U#1> & <async.promise.Promise.then.U#2: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise.then.U#2>) => async.promise.Promise<async.promise.Promise.then.U#2> kind=existential targets=[async.promise.Promise.then#1, async.promise.Promise.then#2]
/// @resolution.call parameters=(Function<(int32,), async.promise.Promise<string>>) arguments=(provided(argument) as Function<(int32,), async.promise.Promise<string>>) return=async.promise.Promise<string> kind=symbol target=async.promise.Promise.then#1 receiver=async.promise.Promise<int32> instance=async.promise.Promise<int32>.then#1<string>
/// @resolution.place source=input placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=input root=input
/// @generic.instance source=input id=async.promise.Promise<int32>
/// @generic.instance source=input.then id=async.promise.Promise<async.promise.Promise.then.U#1>
/// @generic.instance source=input.then id=async.promise.Promise<async.promise.Promise.then.U#2>
/// @generic.instance source=input.then id=async.promise.Promise<int32>
/// @type.symbol symbol=symbol3 type=Function<(int32,), async.promise.Promise<string>>
/// @type.node type=Function<(int32,), async.promise.Promise<string>>
/// @type.symbol symbol=symbol3.value source=value type=int32

    input.then(() => "done")
    /// @type.node source="input.then(() => \"done\")" type=async.promise.Promise<string>
    /// @type.node source=input type=async.promise.Promise<int32>
    /// @type.node source=input.then type=<async.promise.Promise.then.U#1: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise<async.promise.Promise.then.U#1>>) => async.promise.Promise<async.promise.Promise.then.U#1> & <async.promise.Promise.then.U#2: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise.then.U#2>) => async.promise.Promise<async.promise.Promise.then.U#2>
    /// @resolution.name source=input target=input
    /// @resolution.member source=input.then receiver=async.promise.Promise<int32> type=<async.promise.Promise.then.U#1: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise<async.promise.Promise.then.U#1>>) => async.promise.Promise<async.promise.Promise.then.U#1> & <async.promise.Promise.then.U#2: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise.then.U#2>) => async.promise.Promise<async.promise.Promise.then.U#2> kind=existential targets=[async.promise.Promise.then#1, async.promise.Promise.then#2]
    /// @resolution.call source="input.then(() => \"done\")" parameters=(Function<(int32,), string>) arguments=(provided(() => "done") as Function<(int32,), string>) return=async.promise.Promise<string> kind=symbol target=async.promise.Promise.then#2 receiver=async.promise.Promise<int32> instance=async.promise.Promise<int32>.then#2<string>
    /// @resolution.place source=input placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=input root=input
    /// @generic.instance source="input.then(() => \"done\")" id=async.promise.Promise<int32>.then#2<string>
    /// @generic.instance source="input.then(() => \"done\")" id=async.promise.Promise<string>
    /// @generic.instance source=input id=async.promise.Promise<int32>
    /// @generic.instance source=input.then id=async.promise.Promise<async.promise.Promise.then.U#1>
    /// @generic.instance source=input.then id=async.promise.Promise<async.promise.Promise.then.U#2>
    /// @generic.instance source=input.then id=async.promise.Promise<int32>
    /// @type.symbol symbol=symbol3.symbol5 source="() => \"done\"" type=Function<(), string>
    /// @type.node source="() => \"done\"" type=Function<(), string>
    /// @type.node source="\"done\"" type="done"

});

/// @generic.instance id=async.promise.Promise<async.promise.Promise.then.U#1> template=async.promise.Promise arguments=(async.promise.Promise.then.U#1)
/// @generic.instance id=async.promise.Promise<async.promise.Promise.then.U#2> template=async.promise.Promise arguments=(async.promise.Promise.then.U#2)
/// @generic.instance id=async.promise.Promise<int32> template=async.promise.Promise arguments=(int32)
/// @generic.instance id=async.promise.Promise<int32>.then#1<string> template=async.promise.Promise.then#1 arguments=(int32, string)
/// @generic.instance id=async.promise.Promise<int32>.then#2<string> template=async.promise.Promise.then#2 arguments=(int32, string)
/// @generic.instance id=async.promise.Promise<string> template=async.promise.Promise arguments=(string)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
declare const next: Promise<string>;
declare const usePromise: boolean;
const result: Promise<string | Promise<string>> = input.then<int32, string | Promise<string>>(
    (): Promise<string> | string => {
        usePromise ? (next as Promise<string> | string) : ("done" as Promise<string> | string)
    },
);

=== checked ===
import { Promise } from "destack:async";

declare const input: Promise<int32>;
/// @type.symbol symbol=input source=input type=async.promise.Promise<int32>
/// @resolution.pattern source=input kind=binding target=input
/// @resolution.name source=Promise target=async.promise.Promise

declare const next: Promise<string>;
/// @type.symbol symbol=next source=next type=async.promise.Promise<string>
/// @resolution.pattern source=next kind=binding target=next
/// @resolution.name source=Promise target=async.promise.Promise

declare const usePromise: boolean;
/// @type.symbol symbol=usePromise source=usePromise type=boolean
/// @resolution.pattern source=usePromise kind=binding target=usePromise

const result: Promise<string | Promise<string>> = input.then(() => {
/// @type.symbol symbol=result source=result type=async.promise.Promise<string | async.promise.Promise<string>>
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Promise target=async.promise.Promise
/// @resolution.name source=Promise target=async.promise.Promise
/// @type.node source=input type=async.promise.Promise<int32>
/// @type.node source=input.then type=<async.promise.Promise.then.U#1: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise<async.promise.Promise.then.U#1>>) => async.promise.Promise<async.promise.Promise.then.U#1> & <async.promise.Promise.then.U#2: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise.then.U#2>) => async.promise.Promise<async.promise.Promise.then.U#2>
/// @type.node type=async.promise.Promise<string | async.promise.Promise<string>>
/// @resolution.name source=input target=input
/// @resolution.member source=input.then receiver=async.promise.Promise<int32> type=<async.promise.Promise.then.U#1: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise<async.promise.Promise.then.U#1>>) => async.promise.Promise<async.promise.Promise.then.U#1> & <async.promise.Promise.then.U#2: memory.capability.Copy>(this: async.promise.Promise<int32>, Function<(int32,), async.promise.Promise.then.U#2>) => async.promise.Promise<async.promise.Promise.then.U#2> kind=existential targets=[async.promise.Promise.then#1, async.promise.Promise.then#2]
/// @resolution.call parameters=(Function<(int32,), string | async.promise.Promise<string>>) arguments=(provided(argument) as Function<(int32,), string | async.promise.Promise<string>>) return=async.promise.Promise<string | async.promise.Promise<string>> kind=symbol target=async.promise.Promise.then#2 receiver=async.promise.Promise<int32> instance="async.promise.Promise<int32>.then#2<string | async.promise.Promise<string>>"
/// @resolution.place source=input placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=input root=input
/// @generic.instance source=input id=async.promise.Promise<int32>
/// @generic.instance source=input.then id=async.promise.Promise<async.promise.Promise.then.U#1>
/// @generic.instance source=input.then id=async.promise.Promise<async.promise.Promise.then.U#2>
/// @generic.instance source=input.then id=async.promise.Promise<int32>
/// @type.symbol symbol=symbol5 type=Function<(), async.promise.Promise<string> | string>
/// @type.node type=Function<(), async.promise.Promise<string> | string>

    usePromise ? next : "done"
    /// @type.node source="usePromise ? next : \"done\"" type=async.promise.Promise<string> | string
    /// @type.node source=usePromise type=boolean
    /// @resolution.name source=usePromise target=usePromise
    /// @resolution.place source=usePromise placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=usePromise root=usePromise
    /// @generic.instance source="usePromise ? next : \"done\"" id=async.promise.Promise<string>
    /// @type.node source=next type=async.promise.Promise<string>
    /// @resolution.name source=next target=next
    /// @resolution.place source=next placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=next root=next
    /// @generic.instance source=next id=async.promise.Promise<string>
    /// @type.node source="\"done\"" type="done"

});

/// @generic.instance id="async.promise.Promise<int32>.then#2<string | async.promise.Promise<string>>" template=async.promise.Promise.then#2 arguments=(int32, string | async.promise.Promise<string>)
/// @generic.instance id="async.promise.Promise<string | async.promise.Promise<string>>" template=async.promise.Promise arguments=(string | async.promise.Promise<string>)
/// @generic.instance id=async.promise.Promise<async.promise.Promise.then.U#1> template=async.promise.Promise arguments=(async.promise.Promise.then.U#1)
/// @generic.instance id=async.promise.Promise<async.promise.Promise.then.U#2> template=async.promise.Promise arguments=(async.promise.Promise.then.U#2)
/// @generic.instance id=async.promise.Promise<int32> template=async.promise.Promise arguments=(int32)
/// @generic.instance id=async.promise.Promise<string> template=async.promise.Promise arguments=(string)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

declare const input: Promise<string>;
const result: Promise<string> = Promise.resolve<string>(input);

=== checked ===
import { Promise } from "destack:async";

declare const input: Promise<string>;
/// @type.symbol symbol=input source=input type=async.promise.Promise<string>
/// @resolution.pattern source=input kind=binding target=input
/// @resolution.name source=Promise target=async.promise.Promise

const result: Promise<string> = Promise.resolve(input);
/// @type.symbol symbol=result source=result type=async.promise.Promise<string>
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=Promise target=async.promise.Promise
/// @type.node source=Promise type=async.promise.Promise
/// @type.node source=Promise.resolve type=<async.promise.Promise.resolve.T#1: memory.capability.Copy>(async.promise.Promise<async.promise.Promise.resolve.T#1>) => async.promise.Promise<async.promise.Promise.resolve.T#1> & <async.promise.Promise.resolve.T#2: memory.capability.Copy>(async.promise.Promise.resolve.T#2) => async.promise.Promise<async.promise.Promise.resolve.T#2>
/// @type.node source=Promise.resolve(input) type=async.promise.Promise<string>
/// @resolution.name source=Promise target=async.promise.Promise
/// @resolution.member source=Promise.resolve receiver=async.promise.Promise type=<async.promise.Promise.resolve.T#1: memory.capability.Copy>(async.promise.Promise<async.promise.Promise.resolve.T#1>) => async.promise.Promise<async.promise.Promise.resolve.T#1> & <async.promise.Promise.resolve.T#2: memory.capability.Copy>(async.promise.Promise.resolve.T#2) => async.promise.Promise<async.promise.Promise.resolve.T#2> kind=existential targets=[async.promise.Promise.resolve#1, async.promise.Promise.resolve#2]
/// @resolution.call source=Promise.resolve(input) parameters=(async.promise.Promise<string>) arguments=(provided(input) as async.promise.Promise<string>) return=async.promise.Promise<string> kind=symbol target=async.promise.Promise.resolve#1 instance=async.promise.Promise.resolve#1<string>
/// @generic.instance source=Promise.resolve id=async.promise.Promise<async.promise.Promise.resolve.T#1>
/// @generic.instance source=Promise.resolve id=async.promise.Promise<async.promise.Promise.resolve.T#2>
/// @generic.instance source=Promise.resolve(input) id=async.promise.Promise.resolve#1<string>
/// @generic.instance source=Promise.resolve(input) id=async.promise.Promise<string>
/// @type.node source=input type=async.promise.Promise<string>
/// @resolution.name source=input target=input
/// @resolution.place source=input placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=input root=input
/// @generic.instance source=input id=async.promise.Promise<string>

/// @generic.instance id=async.promise.Promise.resolve#1<string> template=async.promise.Promise.resolve#1 arguments=(string)
/// @generic.instance id=async.promise.Promise<async.promise.Promise.resolve.T#1> template=async.promise.Promise arguments=(async.promise.Promise.resolve.T#1)
/// @generic.instance id=async.promise.Promise<async.promise.Promise.resolve.T#2> template=async.promise.Promise arguments=(async.promise.Promise.resolve.T#2)
/// @generic.instance id=async.promise.Promise<string> template=async.promise.Promise arguments=(string)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            ,
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const number: 1 = identity<1>(1);
const text: "x" = identity<"x">("x");

=== checked ===
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
/// @type.symbol symbol=number source=number type=1
/// @resolution.pattern source=number kind=binding target=number
/// @type.node source=identity type=(1) => 1
/// @type.node source=identity(1) type=1
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(1) parameters=(1) arguments=(provided(1) as 1) return=1 kind=symbol target=identity instance=identity<1>
/// @generic.instance source=identity(1) id=identity<1>
/// @type.node source=1 type=1

const text = identity("x");
/// @type.symbol symbol=text source=text type="x"
/// @resolution.pattern source=text kind=binding target=text
/// @type.node source="identity(\"x\")" type="x"
/// @type.node source=identity type=("x") => "x"
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity(\"x\")" parameters=("x") arguments=(provided("x") as "x") return="x" kind=symbol target=identity instance="identity<\"x\">"
/// @generic.instance source="identity(\"x\")" id="identity<\"x\">"
/// @type.node source="\"x\"" type="x"

/// @generic.instance id="identity<\"x\">" template=identity arguments=("x")
/// @generic.instance id=identity<1> template=identity arguments=(1)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const values: float64[] = identity<float64[]>([1, 2]);

=== checked ===
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
/// @type.symbol symbol=values source=values type=Array<float64>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source="identity([1, 2])" type=Array<float64>
/// @type.node source=identity type=(Array<float64>) => Array<float64>
/// @resolution.name source=identity target=identity
/// @resolution.call source="identity([1, 2])" parameters=(Array<float64>) arguments=(provided([1, 2]) as Array<float64>) return=Array<float64> kind=symbol target=identity instance=identity<Array<float64>>
/// @generic.instance source="identity([1, 2])" id=identity<Array<float64>>
/// @type.node source=[1, 2] type=Array<float64>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @generic.instance id=identity<Array<float64>> template=identity arguments=(Array<float64>)
"#,
    );
}

#[test]
fn test_array_parameter_infers_widened_element_type() {
    let session = TestSession::single(
        r#"
function first<T>(values: T[]): T {
    return values[0];
}

const value = first([1, 2]);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function first<T>(values: T[]): T {
    return values[0];
}

const value: float64 = first<float64>([1, 2]);

=== checked ===
function first<T>(values: T[]): T {
/// @generic.template symbol=first parameters=(T)
/// @type.symbol symbol=first type=<T>(Array<T>) => T
/// @type.symbol symbol=first.T source=T type=T
/// @type.symbol symbol=first.values source="values: T[]" type=Array<T>
/// @resolution.name source=T target=first.T
/// @resolution.name source=T target=first.T

    return values[0];
    /// @type.node source=values type=Array<T>
    /// @type.node source=values[0] type=T
    /// @resolution.name source=values target=first.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=first.values
    /// @resolution.place source=values[0] placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values[0] root=first.values keys=[0]
    /// @resolution.subscript source=values[0] type=T kind=call target="collections.array.index#1(parameters=(isize), arguments=(provided(0) as isize), return=memory.type.WithAccess<&'frame T, \"exclusive\">)"
    /// @generic.instance source=values[0] id="Array<T>.<extension#5>.index#1<\"exclusive\">"
    /// @type.node source=0 type=0

}

const value = first([1, 2]);
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="first([1, 2])" type=float64
/// @type.node source=first type=(Array<float64>) => float64
/// @resolution.name source=first target=first
/// @resolution.call source="first([1, 2])" parameters=(Array<float64>) arguments=(provided([1, 2]) as Array<float64>) return=float64 kind=symbol target=first instance=first<float64>
/// @generic.instance source="first([1, 2])" id=first<float64>
/// @type.node source=[1, 2] type=Array<float64>
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @generic.instance id="Array<T>.<extension#5>.index#1<\"exclusive\">" template=collections.array.index#1 arguments=(T, "exclusive")
/// @generic.instance id=first<float64> template=first arguments=(float64)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const first: 1 = identity<1>(1);
const second: 2 = identity<2>(2);

=== checked ===
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
/// @generic.instance source=identity<1>(1) id=identity<1>
/// @type.node source=1 type=1

const second = identity<2>(2);
/// @type.symbol symbol=second source=second type=2
/// @resolution.pattern source=second kind=binding target=second
/// @type.node source=identity type=(2) => 2
/// @type.node source=identity<2>(2) type=2
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity<2>(2) parameters=(2) arguments=(provided(2) as 2) return=2 kind=symbol target=identity instance=identity<2>
/// @generic.instance source=identity<2>(2) id=identity<2>
/// @type.node source=2 type=2

/// @generic.instance id=identity<1> template=identity arguments=(1)
/// @generic.instance id=identity<2> template=identity arguments=(2)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const text: string = identity<string>("x");

=== checked ===
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
/// @generic.instance source="identity<string>(\"x\")" id=identity<string>
/// @type.node source="\"x\"" type="x"

/// @generic.instance id=identity<string> template=identity arguments=(string)
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

identity<int32>("x");

=== checked ===
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
/// @generic.instance source="identity<int32>(\"x\")" id=identity<int32>
/// @type.node source="\"x\"" type="x"

/// @generic.instance id=identity<int32> template=identity arguments=(int32)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function identity<T>(value: T): T {
    return value;
}

const asInt = identity<int32>;

=== checked ===
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
/// @type.symbol symbol=asInt source=asInt type=<T>(int32) => int32
/// @resolution.pattern source=asInt kind=binding target=asInt
/// @type.node source=identity<int32> type=<T>(int32) => int32
/// @resolution.name source=identity target=identity
/// @resolution.instantiation source=identity<int32> target=identity instance=identity<int32>
/// @generic.instance source=identity<int32> id=identity<int32>

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

function parse<T>(value: T[]): T {
    return value[0];
}

const parser = parse<int32>;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function parse<T>(value: T): T {
    return value;
}

function parse<T>(value: T[]): T {
    return value[0];
}

const parser = parse<int32>;

=== checked ===
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

function parse<T>(value: T[]): T {
/// @generic.template symbol=parse#2 parameters=(T#2)
/// @type.symbol symbol=parse#2 type=<T#2>(Array<T#2>) => T#2
/// @type.symbol symbol=parse.T#2 source=T type=T#2
/// @type.symbol symbol=parse.value#2 source="value: T[]" type=Array<T#2>
/// @resolution.name source=T target=parse.T#2
/// @resolution.name source=T target=parse.T#2

    return value[0];
    /// @type.node source=value type=Array<T#2>
    /// @type.node source=value[0] type=T#2
    /// @resolution.name source=value target=parse.value#2
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=parse.value#2
    /// @resolution.place source=value[0] placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value[0] root=parse.value#2 keys=[0]
    /// @resolution.subscript source=value[0] type=T#2 kind=call target="collections.array.index#1(parameters=(isize), arguments=(provided(0) as isize), return=memory.type.WithAccess<&'frame T#2, \"exclusive\">)"
    /// @generic.instance source=value[0] id="Array<T#2>.<extension#5>.index#1<\"exclusive\">"
    /// @type.node source=0 type=0

}

const parser = parse<int32>;
/// @type.symbol symbol=parser source=parser type=<error>
/// @resolution.pattern source=parser kind=binding target=parser
/// @type.node source=parse<int32> type=<error>
/// @resolution.name source=parse target=[parse#1, parse#2]
/// @resolution.rejected source=parse<int32>

/// @generic.instance id="Array<T#2>.<extension#5>.index#1<\"exclusive\">" template=collections.array.index#1 arguments=(T#2, "exclusive")
"#,
        r#"
/// @diagnostic.error id=ambiguous-reference message="ambiguous reference 'parse'"
/// @diagnostic.label line=10 column=16 span="parse" line_source="const parser = parse<int32>;"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function pair<T, U = T>(left: T, right?: U): (T, U);

const defaulted: (float64, float64) = pair<float64, float64>(1);
const overridden: (float64, string) = pair<float64, string>(1, "x" as string | undefined);

=== checked ===
declare function pair<T, U = T>(left: T, right?: U): (T, U);
/// @generic.template symbol=pair parameters=(T, U = T)
/// @type.symbol symbol=pair source="declare function pair<T, U = T>(left: T, right?: U): (T, U)" type=<T, U = T>(T, U | undefined?) => (T, U)
/// @type.symbol symbol=pair.T source=T type=T
/// @type.symbol symbol=pair.U source="U = T" type=U
/// @resolution.name source=T target=pair.T
/// @type.symbol symbol=pair.left source="left: T" type=T
/// @resolution.name source=T target=pair.T
/// @type.symbol symbol=pair.right source="right?: U" type=U | undefined
/// @resolution.name source=U target=pair.U
/// @resolution.name source=T target=pair.T
/// @resolution.name source=U target=pair.U

const defaulted = pair(1);
/// @type.symbol symbol=defaulted source=defaulted type=(float64, float64)
/// @resolution.pattern source=defaulted kind=binding target=defaulted
/// @type.node source=pair type=(float64, float64 | undefined?) => (float64, float64)
/// @type.node source=pair(1) type=(float64, float64)
/// @resolution.name source=pair target=pair
/// @resolution.call source=pair(1) parameters=(float64, float64 | undefined) arguments=(provided(1) as float64, omitted as float64 | undefined) return=(float64, float64) kind=symbol target=pair instance="pair<float64, float64>"
/// @generic.instance source=pair(1) id="pair<float64, float64>"
/// @type.node source=1 type=1

const overridden = pair(1, "x");
/// @type.symbol symbol=overridden source=overridden type=(float64, string)
/// @resolution.pattern source=overridden kind=binding target=overridden
/// @type.node source="pair(1, \"x\")" type=(float64, string)
/// @type.node source=pair type=(float64, string | undefined?) => (float64, string)
/// @resolution.name source=pair target=pair
/// @resolution.call source="pair(1, \"x\")" parameters=(float64, string | undefined) arguments=(provided(1) as float64, provided("x") as string | undefined) return=(float64, string) kind=symbol target=pair instance="pair<float64, string>"
/// @generic.instance source="pair(1, \"x\")" id="pair<float64, string>"
/// @type.node source=1 type=1
/// @type.node source="\"x\"" type="x"

/// @generic.instance id="pair<float64, float64>" template=pair arguments=(float64, float64)
/// @generic.instance id="pair<float64, string>" template=pair arguments=(float64, string)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function choose<T: 1 | 2>(left: T, right: T): T;

const value: 1 | 2 = choose<1 | 2>(1 as 1 | 2, 2 as 1 | 2);

=== checked ===
declare function choose<T: 1 | 2>(left: T, right: T): T;
/// @generic.template symbol=choose parameters=(T: 1 | 2)
/// @type.symbol symbol=choose source="declare function choose<T: 1 | 2>(left: T, right: T): T" type=<T: 1 | 2>(T, T) => T
/// @type.symbol symbol=choose.T source="T: 1 | 2" type=T
/// @type.symbol symbol=choose.left source="left: T" type=T
/// @resolution.name source=T target=choose.T
/// @type.symbol symbol=choose.right source="right: T" type=T
/// @resolution.name source=T target=choose.T
/// @resolution.name source=T target=choose.T

const value = choose(1, 2);
/// @type.symbol symbol=value source=value type=1 | 2
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="choose(1, 2)" type=1 | 2
/// @type.node source=choose type=(1 | 2, 1 | 2) => 1 | 2
/// @resolution.name source=choose target=choose
/// @resolution.call source="choose(1, 2)" parameters=(1 | 2, 1 | 2) arguments=(provided(1) as 1 | 2, provided(2) as 1 | 2) return=1 | 2 kind=symbol target=choose instance="choose<1 | 2>"
/// @generic.instance source="choose(1, 2)" id="choose<1 | 2>"
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @generic.instance id="choose<1 | 2>" template=choose arguments=(1 | 2)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function choose<T>(left: T, right: T): T where T: 1 | 2;

const value: 1 | 2 = choose<1 | 2>(1 as 1 | 2, 2 as 1 | 2);

=== checked ===
declare function choose<T>(left: T, right: T): T where T: 1 | 2;
/// @generic.template symbol=choose parameters=(T)
/// @type.symbol symbol=choose source="declare function choose<T>(left: T, right: T): T where T: 1 | 2" type=<T>(T, T) => T
/// @type.symbol symbol=choose.T source=T type=T
/// @type.symbol symbol=choose.left source="left: T" type=T
/// @resolution.name source=T target=choose.T
/// @type.symbol symbol=choose.right source="right: T" type=T
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
/// @generic.instance source="choose(1, 2)" id="choose<1 | 2>"
/// @type.node source=1 type=1
/// @type.node source=2 type=2

/// @generic.instance id="choose<1 | 2>" template=choose arguments=(1 | 2)
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

    session.assert_dir_checked_and_diagnostics(
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

const accepted = accept<Good>;
const rejected = accept<int32>;

=== checked ===
newtype interface Marker {}
/// @type.symbol symbol=Marker source="newtype interface Marker {}" type=Marker
/// @definition.interface symbol=Marker source="newtype interface Marker {}" nominal=true

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
/// @type.symbol symbol=accepted source=accepted type=<T>(Good) => Good
/// @resolution.pattern source=accepted kind=binding target=accepted
/// @type.node source=accept<Good> type=<T>(Good) => Good
/// @resolution.name source=accept target=accept
/// @resolution.instantiation source=accept<Good> target=accept instance=accept<Good>
/// @generic.instance source=accept<Good> id=accept<Good>
/// @resolution.name source=Good target=Good

const rejected = accept<int32>;
/// @type.symbol symbol=rejected source=rejected type=<T>(int32) => int32
/// @resolution.pattern source=rejected kind=binding target=rejected
/// @type.node source=accept<int32> type=<T>(int32) => int32
/// @resolution.name source=accept target=accept
/// @resolution.instantiation source=accept<int32> target=accept instance=accept<int32>
/// @generic.instance source=accept<int32> id=accept<int32>

/// @generic.instance id=accept<Good> template=accept arguments=(Good)
/// @generic.instance id=accept<int32> template=accept arguments=(int32)
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

    session.assert_dir_checked_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=not-assignable message="type 'float64' is not assignable to type 'int32'"
/// @diagnostic.label line=13 column=12 span="Box.of(value)" line_source="return Box.of(value);"
/// @diagnostic.note message="the mismatch is in type argument 0 of 'Box'"
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: ^int32[];

const kept: ^Array<int32> = values.map<int32, int32>((value: int32): int32 => value).filter<int32>(
    (value: &'a readonly int32): boolean => value !== undefined,
);

=== checked ===
declare const values: ^int32[];
/// @type.symbol symbol=values source=values type=Owned<Array<int32>>
/// @resolution.pattern source=values kind=binding target=values

const kept = values
/// @type.symbol symbol=kept source=kept type=Owned<Array<int32>>
/// @resolution.pattern source=kept kind=binding target=kept
/// @resolution.name source=values target=values
/// @resolution.member receiver=Owned<Array<int32>> type=(this: Owned<Array<int32>>, Function<(&type_expression.'a readonly int32, isize), boolean>) => Owned<Array<int32>> & (this: Owned<Array<int32>>, Function<(int32, isize), boolean>) => Owned<Array<int32>> kind=existential targets=[collections.array.filter#1, collections.array.filter#2]
/// @resolution.member receiver=Owned<Array<int32>> type=<collections.array.map.U#1>(this: Owned<Array<int32>>, Function<(int32, isize), collections.array.map.U#1>) => Owned<Array<collections.array.map.U#1>> & <collections.array.map.U#2>(this: Owned<Array<int32>>, Function<(int32, isize), collections.array.map.U#2>) => Owned<Array<collections.array.map.U#2>> kind=existential targets=[collections.array.map#1, collections.array.map#2]
/// @resolution.call parameters=(Function<(&type_expression.'a readonly int32, isize), boolean>) arguments=(provided((value) => value !== undefined) as Function<(&type_expression.'a readonly int32, isize), boolean>) return=Owned<Array<int32>> kind=symbol target=collections.array.filter#1 receiver=Owned<Array<int32>> instance=Owned<Array<collections.array.T#2>>.<extension#2>.filter#1
/// @resolution.call parameters=(Function<(int32, isize), int32>) arguments=(provided((value) => value) as Function<(int32, isize), int32>) return=Owned<Array<int32>> kind=symbol target=collections.array.map#1 receiver=Owned<Array<int32>> instance=Owned<Array<collections.array.T#2>>.<extension#2>.map#1<int32>
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values

    .map((value) => value)
    /// @type.symbol symbol=symbol2 source="(value) => value" type=Function<(int32,), int32>
    /// @type.symbol symbol=symbol2.value source=value type=int32
    /// @resolution.name source=value target=symbol2.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=symbol2.value

    .filter((value) => value !== undefined);
    /// @type.symbol symbol=symbol4 source="(value) => value !== undefined" type=Function<(&type_expression.'a readonly int32,), boolean>
    /// @type.symbol symbol=symbol4.value source=value type=&type_expression.'a readonly int32
    /// @resolution.name source=value target=symbol4.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=symbol4.value

/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
/// @generic.instance id=Owned<Array<collections.array.T#2>>.<extension#2>.filter#1 template=collections.array.filter#1 arguments=(int32)
/// @generic.instance id=Owned<Array<collections.array.T#2>>.<extension#2>.map#1<int32> template=collections.array.map#1 arguments=(int32, int32)
"#,
        r#"
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import { Arithmetic, Integer } from "destack:math";

function bump<T: Integer>(value: T): T | undefined {
    return value.checkedAdd<T>(1);
}

=== checked ===
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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: (int32 | undefined)[];

const defined: ^Array<int32 | undefined> = values.map<int32 | undefined, int32 | undefined>(
    (value: int32 | undefined): int32 | undefined => value,
).filter<int32 | undefined>(
    (value: &'a readonly (int32 | undefined)): boolean => value !== undefined,
);

=== checked ===
declare const values: (int32 | undefined)[];
/// @type.symbol symbol=values source=values type=Array<int32 | undefined>
/// @resolution.pattern source=values kind=binding target=values

const defined = values.map((value) => value).filter((value) => value !== undefined);
/// @type.symbol symbol=defined source=defined type=Owned<Array<int32 | undefined>>
/// @resolution.pattern source=defined kind=binding target=defined
/// @resolution.name source=values target=values
/// @resolution.member source="values.map((value) => value).filter" receiver=Owned<Array<int32 | undefined>> type=(this: Owned<Array<int32 | undefined>>, Function<(&type_expression.'a readonly int32 | undefined, isize), boolean>) => Owned<Array<int32 | undefined>> & (this: Owned<Array<int32 | undefined>>, Function<(int32 | undefined, isize), boolean>) => Owned<Array<int32 | undefined>> kind=existential targets=[collections.array.filter#1, collections.array.filter#2]
/// @resolution.member source=values.map receiver=Array<int32 | undefined> type=<collections.array.map.U#2>(this: Array<int32 | undefined>, Function<(int32 | undefined, isize), collections.array.map.U#2>) => Owned<Array<collections.array.map.U#2>> kind=symbol target_receiver=Array<int32 | undefined> target=collections.array.map#2
/// @resolution.call source="values.map((value) => value)" parameters=(Function<(int32 | undefined, isize), int32 | undefined>) arguments=(provided((value) => value) as Function<(int32 | undefined, isize), int32 | undefined>) return=Owned<Array<int32 | undefined>> kind=symbol target=collections.array.map#2 receiver=Array<int32 | undefined> instance="Array<int32 | undefined>.<extension#3>.map#2<int32 | undefined>"
/// @resolution.call source="values.map((value) => value).filter((value) => value !== undefined)" parameters=(Function<(&type_expression.'a readonly int32 | undefined, isize), boolean>) arguments=(provided((value) => value !== undefined) as Function<(&type_expression.'a readonly int32 | undefined, isize), boolean>) return=Owned<Array<int32 | undefined>> kind=symbol target=collections.array.filter#1 receiver=Owned<Array<int32 | undefined>> instance=Owned<Array<collections.array.T#2>>.<extension#2>.filter#1
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instance source="values.map((value) => value)" id="Array<int32 | undefined>.<extension#3>.map#2<int32 | undefined>"
/// @generic.instance source="values.map((value) => value).filter((value) => value !== undefined)" id=Owned<Array<collections.array.T#2>>.<extension#2>.filter#1
/// @type.symbol symbol=symbol2 source="(value) => value" type=Function<(int32 | undefined,), int32 | undefined>
/// @type.symbol symbol=symbol2.value source=value type=int32 | undefined
/// @resolution.name source=value target=symbol2.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol2.value
/// @type.symbol symbol=symbol4 source="(value) => value !== undefined" type=Function<(&type_expression.'a readonly int32 | undefined,), boolean>
/// @type.symbol symbol=symbol4.value source=value type=&type_expression.'a readonly int32 | undefined
/// @resolution.name source=value target=symbol4.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol4.value

/// @generic.instance id="Array<int32 | undefined>" template=collections.array.Array arguments=(int32 | undefined)
/// @generic.instance id="Array<int32 | undefined>.<extension#3>.map#2<int32 | undefined>" template=collections.array.map#2 arguments=(int32 | undefined, int32 | undefined)
/// @generic.instance id=Owned<Array<collections.array.T#2>>.<extension#2>.filter#1 template=collections.array.filter#1 arguments=(int32 | undefined)
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

    session.assert_dir_checked_and_diagnostics(
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

const once: Element<int32, 1>[] = values.flat<int32, 1>();
const twice: Element<int32, 2>[] = values.flat<int32, 2>(2 as 2 | undefined);

=== checked ===
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];
/// @generic.template symbol=Element parameters=(T#1, const Depth#1: usize)
/// @type.symbol symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" type=Depth#1 extends 0 ? T#1 : Array<T#1>
/// @definition.type symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" template=(T#1, const Depth#1: usize) value=Depth#1 extends 0 ? T#1 : Array<T#1>
/// @type.symbol symbol=Element.T source=T type=T#1
/// @type.symbol symbol=Element.Depth source="const Depth: usize" type=Depth#1
/// @resolution.name source=Depth target=Element.Depth
/// @resolution.name source=T target=Element.T
/// @resolution.name source=T target=Element.T

class Values<T> {
/// @generic.template symbol=Values parameters=(T#2)
/// @type.symbol symbol=Values type=Values
/// @definition.class symbol=Values template=(T#2)
/// @definition.method symbol=Values.flat slot=flat type=<const Depth#2: usize = 1, Values.flat.'a>(this: &Values.flat.'a readonly this, Depth#2 | undefined?) => Array<Element<T#2, Depth#2>>
/// @type.symbol symbol=Values.T source=T type=T#2

    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[]
    /// @generic.template symbol=Values.flat parent=template#1 parameters=(const Depth#2: usize = 1, 'a)
    /// @type.symbol symbol=Values.flat type=<const Depth#2: usize = 1, Values.flat.'a>(this: &Values.flat.'a readonly this, Depth#2 | undefined?) => Array<Element<T#2, Depth#2>>
    /// @type.symbol symbol=Values.flat.Depth source="const Depth: usize = 1" type=Depth#2
    /// @type.symbol symbol=Values.flat.this source="&readonly this" type=&Values.flat.'a readonly this
    /// @type.symbol symbol=Values.flat.depth source="depth?: Depth" type=Depth#2 | undefined
    /// @resolution.name source=Depth target=Values.flat.Depth
    /// @resolution.name source=Element target=Element
    /// @resolution.name source=T target=Values.T
    /// @resolution.name source=Depth target=Values.flat.Depth

        where T: Clone {
        /// @resolution.name source=T target=Values.T
        /// @resolution.name source=Clone target=memory.capability.Clone

        todo("flat")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"flat\")" parameters=(string | undefined) arguments=(provided("flat") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}

declare const values: Values<int32>;
/// @type.symbol symbol=values source=values type=Values<int32>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=Values target=Values

const once = values.flat();
/// @type.symbol symbol=once source=once type=Array<Array<int32>>
/// @resolution.pattern source=once kind=binding target=once
/// @resolution.name source=values target=values
/// @resolution.member source=values.flat receiver=Values<int32> type=<const Depth#2: usize = 1, Values.flat.'a>(this: &Values.flat.'a readonly Values<int32>, Depth#2 | undefined?) => Array<Element<int32, Depth#2>> kind=symbol target_receiver=Values<int32> target=Values.flat
/// @resolution.call source=values.flat() parameters=(1 | undefined) arguments=(omitted as 1 | undefined) return=Array<Array<int32>> kind=symbol target=Values.flat receiver=Values<int32> adjustments=(borrow(&'static readonly Values<int32>)) instance=Values<int32>.flat<1>
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instance source=values.flat() id=Values<int32>.flat<1>

const twice = values.flat(2);
/// @type.symbol symbol=twice source=twice type=Array<Array<int32>>
/// @resolution.pattern source=twice kind=binding target=twice
/// @resolution.name source=values target=values
/// @resolution.member source=values.flat receiver=Values<int32> type=<const Depth#2: usize = 1, Values.flat.'a>(this: &Values.flat.'a readonly Values<int32>, Depth#2 | undefined?) => Array<Element<int32, Depth#2>> kind=symbol target_receiver=Values<int32> target=Values.flat
/// @resolution.call source=values.flat(2) parameters=(2 | undefined) arguments=(provided(2) as 2 | undefined) return=Array<Array<int32>> kind=symbol target=Values.flat receiver=Values<int32> adjustments=(borrow(&'static readonly Values<int32>)) instance=Values<int32>.flat<2>
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instance source=values.flat(2) id=Values<int32>.flat<2>

/// @generic.instance id="Element<T#2, Depth#2>" template=Element arguments=(T#2, Depth#2)
/// @generic.instance id=Values<int32> template=Values arguments=(int32)
/// @generic.instance id=Values<int32>.flat<1> template=Values.flat arguments=(int32, 1)
/// @generic.instance id=Values<int32>.flat<2> template=Values.flat arguments=(int32, 2)
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

    session.assert_dir_checked_and_diagnostics(
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

const twice: Element<int32, 2>[] = values.flat<int32, 2>(2 as 2 | undefined);

=== checked ===
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];
/// @generic.template symbol=Element parameters=(T#1, const Depth#1: usize)
/// @type.symbol symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" type=Depth#1 extends 0 ? T#1 : Array<T#1>
/// @definition.type symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" template=(T#1, const Depth#1: usize) value=Depth#1 extends 0 ? T#1 : Array<T#1>
/// @type.symbol symbol=Element.T source=T type=T#1
/// @type.symbol symbol=Element.Depth source="const Depth: usize" type=Depth#1
/// @resolution.name source=Depth target=Element.Depth
/// @resolution.name source=T target=Element.T
/// @resolution.name source=T target=Element.T

declare class Values<out T> {}
/// @generic.template symbol=Values parameters=(out T#2)
/// @type.symbol symbol=Values source="declare class Values<out T> {}" type=Values
/// @definition.class symbol=Values source="declare class Values<out T> {}" template=(out T#2)
/// @type.symbol symbol=Values.T source="out T" type=T#2

extension<T> of Values<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Values<T#3>
/// @definition.method symbol=flat slot=flat type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly this, Depth#2 | undefined?) => Array<Element<T#3, Depth#2>>
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Values target=Values
/// @resolution.name source=T target=T

    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[]
    /// @generic.template symbol=flat parent=template#2 parameters=(const Depth#2: usize = 1, 'a)
    /// @type.symbol symbol=flat type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly this, Depth#2 | undefined?) => Array<Element<T#3, Depth#2>>
    /// @type.symbol symbol=flat.Depth source="const Depth: usize = 1" type=Depth#2
    /// @type.symbol symbol=flat.this source="&readonly this" type=&flat.'a readonly this
    /// @type.symbol symbol=flat.depth source="depth?: Depth" type=Depth#2 | undefined
    /// @resolution.name source=Depth target=flat.Depth
    /// @resolution.name source=Element target=Element
    /// @resolution.name source=T target=T
    /// @resolution.name source=Depth target=flat.Depth

        where T: Clone {
        /// @resolution.name source=T target=T
        /// @resolution.name source=Clone target=memory.capability.Clone

        todo("flat")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"flat\")" parameters=(string | undefined) arguments=(provided("flat") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}

declare const values: Values<int32>;
/// @type.symbol symbol=values source=values type=Values<int32>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=Values target=Values

const twice: Element<int32, 2>[] = values.flat(2);
/// @type.symbol symbol=twice source=twice type=Array<Array<int32>>
/// @resolution.pattern source=twice kind=binding target=twice
/// @resolution.name source=Element target=Element
/// @resolution.name source=values target=values
/// @resolution.member source=values.flat receiver=Values<int32> type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly Values<int32>, Depth#2 | undefined?) => Array<Element<int32, Depth#2>> kind=symbol target_receiver=Values<int32> target=flat
/// @resolution.call source=values.flat(2) parameters=(2 | undefined) arguments=(provided(2) as 2 | undefined) return=Array<Array<int32>> kind=symbol target=flat receiver=Values<int32> adjustments=(borrow(&'static readonly Values<int32>)) instance=Values<int32>.<extension#1>.flat<2>
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instance source=values.flat(2) id=Values<int32>.<extension#1>.flat<2>

/// @generic.instance id="Element<T#3, Depth#2>" template=Element arguments=(T#3, Depth#2)
/// @generic.instance id=Values<int32> template=Values arguments=(int32)
/// @generic.instance id=Values<int32>.<extension#1>.flat<2> template=flat arguments=(int32, 2)
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function filterMap<T, U>(values: T[], callback: (arg0: T) => U | undefined): U[];

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

=== checked ===
declare function filterMap<T, U>(values: T[], callback: (value: T) => U | undefined): U[];
/// @generic.template symbol=filterMap parameters=(T, U)
/// @type.symbol symbol=filterMap type=<T, U>(Array<T>, Function<(T,), U | undefined>) => Array<U>
/// @type.symbol symbol=filterMap.T source=T type=T
/// @type.symbol symbol=filterMap.U source=U type=U
/// @type.symbol symbol=filterMap.values source="values: T[]" type=Array<T>
/// @resolution.name source=T target=filterMap.T
/// @type.symbol symbol=filterMap.callback source="callback: (value: T) => U | undefined" type=Function<(T,), U | undefined>
/// @type.symbol symbol=filterMap.value source="value: T" type=T
/// @resolution.name source=T target=filterMap.T
/// @resolution.name source=U target=filterMap.U
/// @resolution.name source=U target=filterMap.U

declare const values: (int32 | undefined)[];
/// @type.symbol symbol=values source=values type=Array<int32 | undefined>
/// @resolution.pattern source=values kind=binding target=values

const defined = filterMap(values, (value) => {
/// @type.symbol symbol=defined source=defined type=Array<int32>
/// @resolution.pattern source=defined kind=binding target=defined
/// @resolution.name source=filterMap target=filterMap
/// @resolution.call parameters=(Array<int32 | undefined>, Function<(int32 | undefined,), int32 | undefined>) arguments=(provided(values) as Array<int32 | undefined>, provided(argument) as Function<(int32 | undefined,), int32 | undefined>) return=Array<int32> kind=symbol target=filterMap instance="filterMap<int32 | undefined, int32>"
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @type.symbol symbol=symbol8 type=Function<(int32 | undefined,), int32 | undefined>
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

    }
    return undefined;
});

/// @generic.instance id="filterMap<int32 | undefined, int32>" template=filterMap arguments=(int32 | undefined, int32)
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

    session.assert_dir_checked_and_diagnostics(
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

const once: Element<int32 | int32[], 1>[] = values.flat<int32 | int32[], 1>(1 as 1 | undefined);

=== checked ===
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];
/// @generic.template symbol=Element parameters=(T#1, const Depth#1: usize)
/// @type.symbol symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" type=Depth#1 extends 0 ? T#1 : Array<T#1>
/// @definition.type symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" template=(T#1, const Depth#1: usize) value=Depth#1 extends 0 ? T#1 : Array<T#1>
/// @type.symbol symbol=Element.T source=T type=T#1
/// @type.symbol symbol=Element.Depth source="const Depth: usize" type=Depth#1
/// @resolution.name source=Depth target=Element.Depth
/// @resolution.name source=T target=Element.T
/// @resolution.name source=T target=Element.T

declare class Values<out T> {}
/// @generic.template symbol=Values parameters=(out T#2)
/// @type.symbol symbol=Values source="declare class Values<out T> {}" type=Values
/// @definition.class symbol=Values source="declare class Values<out T> {}" template=(out T#2)
/// @type.symbol symbol=Values.T source="out T" type=T#2

extension<T> of Values<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Values<T#3>
/// @definition.method symbol=flat slot=flat type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly this, Depth#2 | undefined?) => Array<Element<T#3, Depth#2>>
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Values target=Values
/// @resolution.name source=T target=T

    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[] {
    /// @generic.template symbol=flat parent=template#2 parameters=(const Depth#2: usize = 1, 'a)
    /// @type.symbol symbol=flat type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly this, Depth#2 | undefined?) => Array<Element<T#3, Depth#2>>
    /// @type.symbol symbol=flat.Depth source="const Depth: usize = 1" type=Depth#2
    /// @type.symbol symbol=flat.this source="&readonly this" type=&flat.'a readonly this
    /// @type.symbol symbol=flat.depth source="depth?: Depth" type=Depth#2 | undefined
    /// @resolution.name source=Depth target=flat.Depth
    /// @resolution.name source=Element target=Element
    /// @resolution.name source=T target=T
    /// @resolution.name source=Depth target=flat.Depth

        todo("flat")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"flat\")" parameters=(string | undefined) arguments=(provided("flat") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}

declare const values: Values<int32 | int32[]>;
/// @type.symbol symbol=values source=values type=Values<int32 | Array<int32>>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=Values target=Values

const once: Element<int32 | int32[], 1>[] = values.flat(1);
/// @type.symbol symbol=once source=once type=Array<Array<int32 | Array<int32>>>
/// @resolution.pattern source=once kind=binding target=once
/// @resolution.name source=Element target=Element
/// @resolution.name source=values target=values
/// @resolution.member source=values.flat receiver=Values<int32 | Array<int32>> type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly Values<int32 | Array<int32>>, Depth#2 | undefined?) => Array<Element<int32 | Array<int32>, Depth#2>> kind=symbol target_receiver=Values<int32 | Array<int32>> target=flat
/// @resolution.call source=values.flat(1) parameters=(1 | undefined) arguments=(provided(1) as 1 | undefined) return=Array<Array<int32 | Array<int32>>> kind=symbol target=flat receiver=Values<int32 | Array<int32>> adjustments=(borrow(&'static readonly Values<int32 | Array<int32>>)) instance="Values<int32 | Array<int32>>.<extension#1>.flat<1>"
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instance source=values.flat(1) id="Values<int32 | Array<int32>>.<extension#1>.flat<1>"

/// @generic.instance id="Element<T#3, Depth#2>" template=Element arguments=(T#3, Depth#2)
/// @generic.instance id="Values<int32 | Array<int32>>" template=Values arguments=(int32 | Array<int32>)
/// @generic.instance id="Values<int32 | Array<int32>>.<extension#1>.flat<1>" template=flat arguments=(int32 | Array<int32>, 1)
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Copy } from "destack:memory";

declare function requireCopy<T: Copy>(value: T): void;

declare const value: int32 | int32[];

requireCopy<int32 | int32[]>(value);

=== checked ===
import { Copy } from "destack:memory";

declare function requireCopy<T: Copy>(value: T): void;
/// @generic.template symbol=requireCopy parameters=(T: memory.capability.Copy)
/// @type.symbol symbol=requireCopy source="declare function requireCopy<T: Copy>(value: T): void" type=<T: memory.capability.Copy>(T) => void
/// @type.symbol symbol=requireCopy.T source="T: Copy" type=T
/// @resolution.name source=Copy target=memory.capability.Copy
/// @type.symbol symbol=requireCopy.value source="value: T" type=T
/// @resolution.name source=T target=requireCopy.T

declare const value: int32 | int32[];
/// @type.symbol symbol=value source=value type=int32 | Array<int32>
/// @resolution.pattern source=value kind=binding target=value

requireCopy(value);
/// @resolution.name source=requireCopy target=requireCopy
/// @resolution.call source=requireCopy(value) parameters=(int32 | Array<int32>) arguments=(provided(value) as int32 | Array<int32>) return=void kind=symbol target=requireCopy instance="requireCopy<int32 | Array<int32>>"
/// @generic.instance source=requireCopy(value) id="requireCopy<int32 | Array<int32>>"
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value

/// @generic.instance id="requireCopy<int32 | Array<int32>>" template=requireCopy arguments=(int32 | Array<int32>)
"#,
        r#"
"#,
    );
}

/// Flatten a union-element array through the library flat.
#[test]
fn test_library_flat_reduces_a_union_element_array() {
    let session = TestSession::single(
        r#"
function flatten(values: (int32 | int32[])[]): int32[] {
    return values.flat(1);
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function flatten(values: (int32 | int32[])[]): int32[] {
    return values.flat<int32 | int32[], 1>(1 as 1 | undefined) as int32[];
}

=== checked ===
function flatten(values: (int32 | int32[])[]): int32[] {
/// @type.symbol symbol=flatten type=(Array<int32 | Array<int32>>) => Array<int32>
/// @type.symbol symbol=flatten.values source="values: (int32 | int32[])[]" type=Array<int32 | Array<int32>>

    return values.flat(1);
    /// @resolution.name source=values target=flatten.values
    /// @resolution.member source=values.flat receiver=Array<int32 | Array<int32>> type=<const collections.array.flat.Depth: usize = 1, collections.array.flat.'a>(this: &collections.array.flat.'a readonly Array<int32 | Array<int32>>, collections.array.flat.Depth | undefined?) => Owned<Array<collections.array.FlattenedElement<int32 | Array<int32>, collections.array.flat.Depth>>> kind=symbol target_receiver=Array<int32 | Array<int32>> target=collections.array.flat
    /// @resolution.call source=values.flat(1) parameters=(1 | undefined) arguments=(provided(1) as 1 | undefined) return=Owned<Array<int32>> kind=symbol target=collections.array.flat receiver=Array<int32 | Array<int32>> adjustments=(borrow(&'frame readonly Array<int32 | Array<int32>>)) instance="Array<int32 | Array<int32>>.<extension#5>.flat<1>"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=flatten.values
    /// @generic.instance source=values.flat(1) id="Array<int32 | Array<int32>>.<extension#5>.flat<1>"

}

/// @generic.instance id="Array<int32 | Array<int32>>.<extension#5>.flat<1>" template=collections.array.flat arguments=(int32 | Array<int32>, 1)
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

    session.assert_dir_checked_and_diagnostics(
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
    return values.flat<int32 | int32[], 1>(1 as 1 | undefined);
}

=== checked ===
type Element<T, const Depth: usize> = Depth extends 0 ? T : T[];
/// @generic.template symbol=Element parameters=(T#1, const Depth#1: usize)
/// @type.symbol symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" type=Depth#1 extends 0 ? T#1 : Array<T#1>
/// @definition.type symbol=Element source="type Element<T, const Depth: usize> = Depth extends 0 ? T : T[]" template=(T#1, const Depth#1: usize) value=Depth#1 extends 0 ? T#1 : Array<T#1>
/// @type.symbol symbol=Element.T source=T type=T#1
/// @type.symbol symbol=Element.Depth source="const Depth: usize" type=Depth#1
/// @resolution.name source=Depth target=Element.Depth
/// @resolution.name source=T target=Element.T
/// @resolution.name source=T target=Element.T

declare class Values<out T> {}
/// @generic.template symbol=Values parameters=(out T#2)
/// @type.symbol symbol=Values source="declare class Values<out T> {}" type=Values
/// @definition.class symbol=Values source="declare class Values<out T> {}" template=(out T#2)
/// @type.symbol symbol=Values.T source="out T" type=T#2

extension<T> of Values<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=Values<T#3>
/// @definition.method symbol=flat slot=flat type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly this, Depth#2 | undefined?) => Array<Element<T#3, Depth#2>>
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Values target=Values
/// @resolution.name source=T target=T

    flat<const Depth: usize = 1>(&readonly this, depth?: Depth): Element<T, Depth>[] {
    /// @generic.template symbol=flat parent=template#2 parameters=(const Depth#2: usize = 1, 'a)
    /// @type.symbol symbol=flat type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly this, Depth#2 | undefined?) => Array<Element<T#3, Depth#2>>
    /// @type.symbol symbol=flat.Depth source="const Depth: usize = 1" type=Depth#2
    /// @type.symbol symbol=flat.this source="&readonly this" type=&flat.'a readonly this
    /// @type.symbol symbol=flat.depth source="depth?: Depth" type=Depth#2 | undefined
    /// @resolution.name source=Depth target=flat.Depth
    /// @resolution.name source=Element target=Element
    /// @resolution.name source=T target=T
    /// @resolution.name source=Depth target=flat.Depth

        todo("flat")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"flat\")" parameters=(string | undefined) arguments=(provided("flat") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}

declare const values: Values<int32 | int32[]>;
/// @type.symbol symbol=values source=values type=Values<int32 | Array<int32>>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=Values target=Values

function flatten(): (int32 | int32[])[][] {
/// @type.symbol symbol=flatten type=() => Array<Array<int32 | Array<int32>>>

    return values.flat(1);
    /// @resolution.name source=values target=values
    /// @resolution.member source=values.flat receiver=Values<int32 | Array<int32>> type=<const Depth#2: usize = 1, flat.'a>(this: &flat.'a readonly Values<int32 | Array<int32>>, Depth#2 | undefined?) => Array<Element<int32 | Array<int32>, Depth#2>> kind=symbol target_receiver=Values<int32 | Array<int32>> target=flat
    /// @resolution.call source=values.flat(1) parameters=(1 | undefined) arguments=(provided(1) as 1 | undefined) return=Array<Array<int32 | Array<int32>>> kind=symbol target=flat receiver=Values<int32 | Array<int32>> adjustments=(borrow(&'static readonly Values<int32 | Array<int32>>)) instance="Values<int32 | Array<int32>>.<extension#1>.flat<1>"
    /// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=values root=values
    /// @generic.instance source=values.flat(1) id="Values<int32 | Array<int32>>.<extension#1>.flat<1>"

}

/// @generic.instance id="Element<T#3, Depth#2>" template=Element arguments=(T#3, Depth#2)
/// @generic.instance id="Values<int32 | Array<int32>>" template=Values arguments=(int32 | Array<int32>)
/// @generic.instance id="Values<int32 | Array<int32>>.<extension#1>.flat<1>" template=flat arguments=(int32 | Array<int32>, 1)
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: int32 | null | undefined;

if (value != null) {
    value satisfies int32;
}

=== checked ===
declare const value: int32 | null | undefined;
/// @type.symbol symbol=value source=value type=int32 | null | undefined
/// @resolution.pattern source=value kind=binding target=value

if (value != null) {
/// @type.node source="value != null" type=boolean
/// @type.node source=value type=int32 | null | undefined
/// @resolution.name source=value target=value
/// @resolution.operator source="value != null" type=boolean operator="!=" kind=builtin operands=[value as int32 | null | undefined families=(integer | null | undefined), null as null families=(null)]
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @type.node source=null type=null

    value satisfies int32;
    /// @type.node source="value satisfies int32" type=int32
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=value root=value

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: ^int32[];

const positive: ^Array<int32> = values.filter<int32>(
    (value: &'a readonly int32): boolean => (value as int32) > 0,
);

=== checked ===
declare const values: ^int32[];
/// @type.symbol symbol=values source=values type=Owned<Array<int32>>
/// @resolution.pattern source=values kind=binding target=values

const positive = values.filter((value) => value > 0);
/// @type.symbol symbol=positive source=positive type=Owned<Array<int32>>
/// @resolution.pattern source=positive kind=binding target=positive
/// @resolution.name source=values target=values
/// @resolution.member source=values.filter receiver=Owned<Array<int32>> type=(this: Owned<Array<int32>>, Function<(&type_expression.'a readonly int32, isize), boolean>) => Owned<Array<int32>> & (this: Owned<Array<int32>>, Function<(int32, isize), boolean>) => Owned<Array<int32>> kind=existential targets=[collections.array.filter#1, collections.array.filter#2]
/// @resolution.call source="values.filter((value) => value > 0)" parameters=(Function<(&type_expression.'a readonly int32, isize), boolean>) arguments=(provided((value) => value > 0) as Function<(&type_expression.'a readonly int32, isize), boolean>) return=Owned<Array<int32>> kind=symbol target=collections.array.filter#1 receiver=Owned<Array<int32>> instance=Owned<Array<collections.array.T#2>>.<extension#2>.filter#1
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @generic.instance source="values.filter((value) => value > 0)" id=Owned<Array<collections.array.T#2>>.<extension#2>.filter#1
/// @type.symbol symbol=symbol2 source="(value) => value > 0" type=Function<(&type_expression.'a readonly int32,), boolean>
/// @type.symbol symbol=symbol2.value source=value type=&type_expression.'a readonly int32
/// @resolution.name source=value target=symbol2.value
/// @resolution.operator source="value > 0" type=boolean operator=">" kind=builtin operands=[value as int32 families=(integer), 0 as int32 families=(integer)]
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol2.value

/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
/// @generic.instance id=Owned<Array<collections.array.T#2>>.<extension#2>.filter#1 template=collections.array.filter#1 arguments=(int32)
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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
declare class Holder<out T> {}
/// @generic.template symbol=Holder parameters=(out T#1)
/// @type.symbol symbol=Holder source="declare class Holder<out T> {}" type=Holder
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
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"wrap\")" parameters=(string | undefined) arguments=(provided("wrap") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}

const held: Holder<int32> = Holder<int32>.wrap(42);
/// @type.symbol symbol=held source=held type=Holder<int32>
/// @resolution.pattern source=held kind=binding target=held
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=Holder target=Holder
/// @resolution.member source=Holder<int32>.wrap receiver=Holder<int32> type=(int32) => Holder<int32> kind=symbol target_receiver=Holder<int32> target=wrap
/// @resolution.call source=Holder<int32>.wrap(42) parameters=(int32) arguments=(provided(42) as int32) return=Holder<int32> kind=symbol target=wrap instance=Holder<int32>.<extension#1>.wrap
/// @resolution.instantiation source=Holder<int32> target=Holder instance=Holder<int32>
/// @generic.instance source=Holder<int32> id=Holder<int32>
/// @generic.instance source=Holder<int32>.wrap(42) id=Holder<int32>.<extension#1>.wrap

/// @generic.instance id=Holder<T#2> template=Holder arguments=(T#2)
/// @generic.instance id=Holder<int32> template=Holder arguments=(int32)
/// @generic.instance id=Holder<int32>.<extension#1>.wrap template=wrap arguments=(int32)
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
for (const value of 0..10) {
    value;
}

=== checked ===
for (const value of 0..10) {
/// @type.symbol symbol=value source=value type=int64
/// @resolution.pattern source=value kind=binding target=value

    value;
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=value

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_argument_matching_both_union_arms_requires_annotation() {
    // A value inhabiting both cases of a union parameter cannot pick the type argument.
    let session = TestSession::single(
        r#"
declare class Box<in out T> {}
declare function pick<T>(value: Box<T> | Box<Box<T>>): T;
declare const boxed: Box<Box<int32>>;

const picked = pick(boxed);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<in out T> {}
declare function pick<T>(value: Box<T> | Box<Box<T>>): T;
declare const boxed: Box<Box<int32>>;

const picked = pick(boxed);

=== checked ===
declare class Box<in out T> {}
/// @generic.template symbol=Box parameters=(in out T#1)
/// @type.symbol symbol=Box source="declare class Box<in out T> {}" type=Box
/// @definition.class symbol=Box source="declare class Box<in out T> {}" template=(in out T#1)
/// @type.symbol symbol=Box.T source="in out T" type=T#1

declare function pick<T>(value: Box<T> | Box<Box<T>>): T;
/// @generic.template symbol=pick parameters=(T#2)
/// @type.symbol symbol=pick source="declare function pick<T>(value: Box<T> | Box<Box<T>>): T" type=<T#2>(Box<T#2> | Box<Box<T#2>>) => T#2
/// @type.symbol symbol=pick.T source=T type=T#2
/// @type.symbol symbol=pick.value source="value: Box<T> | Box<Box<T>>" type=Box<T#2> | Box<Box<T#2>>
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
/// @generic.instance source=pick id=Box<<error>>
/// @generic.instance source=pick id=Box<Box<<error>>>
/// @generic.instance source=pick(boxed) id=pick<<error>>
/// @type.node source=boxed type=Box<Box<int32>>
/// @resolution.name source=boxed target=boxed
/// @resolution.place source=boxed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=boxed root=boxed
/// @generic.instance source=boxed id=Box<Box<int32>>
/// @generic.instance source=boxed id=Box<int32>

/// @generic.instance id=Box<<error>> template=Box arguments=(<error>)
/// @generic.instance id=Box<Box<<error>>> template=Box arguments=(Box<<error>>)
/// @generic.instance id=Box<Box<T#2>> template=Box arguments=(Box<T#2>)
/// @generic.instance id=Box<Box<int32>> template=Box arguments=(Box<int32>)
/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
/// @generic.instance id=Box<int32> template=Box arguments=(int32)
/// @generic.instance id=pick<<error>> template=pick arguments=(<error>)
"#,
        r#"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=6 column=16 span="pick(boxed)" line_source="const picked = pick(boxed);"
/// @diagnostic.help message="annotate the type explicitly"
"#,
    );
}

#[test]
fn test_nested_block_callbacks_solve_through_optional_results() {
    // Nested optional-result callbacks with block bodies solve both type arguments.
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function collect<T, U>(values: T[], step: (arg0: T) => U | undefined): U[];
declare const starts: (int32 | undefined)[];

const doubled: int32[] = collect<int32, int32>(
    collect<int32 | undefined, int32>(starts, (start: int32 | undefined): int32 | undefined => {
        if (start !== (undefined as int32 | undefined)) {
            return start;
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

=== checked ===
declare function collect<T, U>(values: T[], step: (value: T) => U | undefined): U[];
/// @generic.template symbol=collect parameters=(T, U)
/// @type.symbol symbol=collect type=<T, U>(Array<T>, Function<(T,), U | undefined>) => Array<U>
/// @type.symbol symbol=collect.T source=T type=T
/// @type.symbol symbol=collect.U source=U type=U
/// @type.symbol symbol=collect.values source="values: T[]" type=Array<T>
/// @resolution.name source=T target=collect.T
/// @type.symbol symbol=collect.step source="step: (value: T) => U | undefined" type=Function<(T,), U | undefined>
/// @type.symbol symbol=collect.value source="value: T" type=T
/// @resolution.name source=T target=collect.T
/// @resolution.name source=U target=collect.U
/// @resolution.name source=U target=collect.U

declare const starts: (int32 | undefined)[];
/// @type.symbol symbol=starts source=starts type=Array<int32 | undefined>
/// @resolution.pattern source=starts kind=binding target=starts

const doubled = collect(collect(starts, (start) => {
/// @type.symbol symbol=doubled source=doubled type=Array<int32>
/// @resolution.pattern source=doubled kind=binding target=doubled
/// @type.node source=collect type=(Array<int32>, Function<(int32,), int32 | undefined>) => Array<int32>
/// @type.node type=Array<int32>
/// @resolution.name source=collect target=collect
/// @resolution.call parameters=(Array<int32>, Function<(int32,), int32 | undefined>) arguments=(provided(argument) as Array<int32>, provided(argument) as Function<(int32,), int32 | undefined>) return=Array<int32> kind=symbol target=collect instance="collect<int32, int32>"
/// @type.node source=collect type=(Array<int32 | undefined>, Function<(int32 | undefined,), int32 | undefined>) => Array<int32>
/// @type.node type=Array<int32>
/// @resolution.name source=collect target=collect
/// @resolution.call parameters=(Array<int32 | undefined>, Function<(int32 | undefined,), int32 | undefined>) arguments=(provided(starts) as Array<int32 | undefined>, provided(argument) as Function<(int32 | undefined,), int32 | undefined>) return=Array<int32> kind=symbol target=collect instance="collect<int32 | undefined, int32>"
/// @type.node source=starts type=Array<int32 | undefined>
/// @resolution.name source=starts target=starts
/// @resolution.place source=starts placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=starts root=starts
/// @type.symbol symbol=symbol8 type=Function<(int32 | undefined,), int32 | undefined>
/// @type.node type=Function<(int32 | undefined,), int32 | undefined>
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
        /// @type.node source=start type=int32 | undefined
        /// @resolution.name source=start target=symbol8.start
        /// @resolution.place source=start placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=start root=symbol8.start

    }
    return undefined;
    /// @type.node source=undefined type=undefined

}), (value) => {
/// @type.symbol symbol=symbol10 type=Function<(int32,), int32 | undefined>
/// @type.node type=Function<(int32,), int32 | undefined>
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

/// @generic.instance id="collect<int32 | undefined, int32>" template=collect arguments=(int32 | undefined, int32)
/// @generic.instance id="collect<int32, int32>" template=collect arguments=(int32, int32)
"#,
        r#""#,
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

    session.assert_dir_checked("main.ds", DirRows::checked().with_reference_types(), r#"
=== annotated ===
declare class Promise<out T> {
    then<U>(onFulfilled: (arg0: T) => U | Promise<U>): Promise<U>;
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
    andThen<U, F>(f: (arg0: T) => Result<U, F>): Result<U, E | F> {
        result<U, E | F>()
    }
}

newtype AsyncResult<out T, out E> = Promise<Result<T, E>>;

extension<T, E> of AsyncResult<T, E> {
    andThenSync<U, F>(f: (arg0: T) => Result<U, F>): AsyncResult<U, E | F> {
        AsyncResult(
            this.then<Result<T, E>, Result<U, E | F>>(
                (result: Result<T, E>): Result<U, E | F> => result.andThen<T, E, U, F>(f),
            ),
        )
    }
}

=== checked ===
declare class Promise<T> {
/// @generic.template symbol=Promise parameters=(out T#1)
/// @type.symbol symbol=Promise type=Promise
/// @definition.class symbol=Promise template=(out T#1)
/// @definition.method symbol=Promise.then source="then<U>(onFulfilled: (value: T) => U | Promise<U>): Promise<U>" slot=then type=<U#1>(this: this, Function<(T#1,), U#1 | Promise<U#1>>) => Promise<U#1>
/// @type.symbol symbol=Promise.T source=T type=T#1

    then<U>(onFulfilled: (value: T) => U | Promise<U>): Promise<U>;
    /// @generic.template symbol=Promise.then parent=template#0 parameters=(U#1)
    /// @type.symbol symbol=Promise.then source="then<U>(onFulfilled: (value: T) => U | Promise<U>): Promise<U>" type=<U#1>(this: this, Function<(T#1,), U#1 | Promise<U#1>>) => Promise<U#1>
    /// @type.symbol symbol=Promise.then.U source=U type=U#1
    /// @type.symbol symbol=Promise.then.onFulfilled source="onFulfilled: (value: T) => U | Promise<U>" type=Function<(T#1,), U#1 | Promise<U#1>>
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
/// @type.symbol symbol=result.T source=T type=T#4
/// @type.symbol symbol=result.E source=E type=E#3
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=result.T
/// @resolution.name source=E target=result.E

extension<T, E> of Result<T, E> {
/// @generic.template symbol=<module>#2 parameters=(T#5, E#4)
/// @definition.extension symbol=<module>#2 form=local target=Result<T#5, E#4>
/// @definition.method symbol=andThen slot=andThen type=<U#2, F#1>(this: this, Function<(T#5,), Result<U#2, F#1>>) => Result<U#2, E#4 | F#1>
/// @type.symbol symbol=T#1 source=T type=T#5
/// @type.symbol symbol=E#1 source=E type=E#4
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=T#1
/// @resolution.name source=E target=E#1

    andThen<U, F>(f: (value: T) => Result<U, F>): Result<U, E | F> {
    /// @generic.template symbol=andThen parent=template#5 parameters=(U#2, F#1)
    /// @type.symbol symbol=andThen type=<U#2, F#1>(this: this, Function<(T#5,), Result<U#2, F#1>>) => Result<U#2, E#4 | F#1>
    /// @type.symbol symbol=andThen.U source=U type=U#2
    /// @type.symbol symbol=andThen.F source=F type=F#1
    /// @type.symbol symbol=andThen.f source="f: (value: T) => Result<U, F>" type=Function<(T#5,), Result<U#2, F#1>>
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
        /// @generic.instance source="result<U, E | F>()" id="Result<U#2, E#4 | F#1>"
        /// @generic.instance source="result<U, E | F>()" id="result<U#2, E#4 | F#1>"
        /// @generic.instance source=result id="Result<U#2, E#4 | F#1>"
        /// @resolution.name source=U target=andThen.U
        /// @resolution.name source=E target=E#1
        /// @resolution.name source=F target=andThen.F

    }
}

newtype AsyncResult<T, E> = Promise<Result<T, E>>;
/// @generic.template symbol=AsyncResult parameters=(out T#6, out E#5)
/// @type.symbol symbol=AsyncResult source="newtype AsyncResult<T, E> = Promise<Result<T, E>>" type=AsyncResult
/// @definition.newtype symbol=AsyncResult source="newtype AsyncResult<T, E> = Promise<Result<T, E>>" template=(out T#6, out E#5) backing=Promise<Result<T#6, E#5>> constructors=[<T#6, E#5>(Promise<Result<T#6, E#5>>) => AsyncResult<T#6, E#5>]
/// @type.symbol symbol=AsyncResult.T source=T type=T#6
/// @type.symbol symbol=AsyncResult.E source=E type=E#5
/// @resolution.name source=Promise target=Promise
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=AsyncResult.T
/// @resolution.name source=E target=AsyncResult.E

extension<T, E> of AsyncResult<T, E> {
/// @generic.template symbol=<module>#3 parameters=(T#7, E#6)
/// @definition.extension symbol=<module>#3 form=local target=AsyncResult<T#7, E#6>
/// @definition.method symbol=andThenSync slot=andThenSync type=<U#3, F#2>(this: this, Function<(T#7,), Result<U#3, F#2>>) => AsyncResult<U#3, E#6 | F#2>
/// @type.symbol symbol=T#2 source=T type=T#7
/// @type.symbol symbol=E#2 source=E type=E#6
/// @resolution.name source=AsyncResult target=AsyncResult
/// @resolution.name source=T target=T#2
/// @resolution.name source=E target=E#2

    andThenSync<U, F>(f: (value: T) => Result<U, F>): AsyncResult<U, E | F> {
    /// @generic.template symbol=andThenSync parent=template#7 parameters=(U#3, F#2)
    /// @type.symbol symbol=andThenSync type=<U#3, F#2>(this: this, Function<(T#7,), Result<U#3, F#2>>) => AsyncResult<U#3, E#6 | F#2>
    /// @type.symbol symbol=andThenSync.U source=U type=U#3
    /// @type.symbol symbol=andThenSync.F source=F type=F#2
    /// @type.symbol symbol=andThenSync.f source="f: (value: T) => Result<U, F>" type=Function<(T#7,), Result<U#3, F#2>>
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
        /// @generic.instance source="AsyncResult(this.then((result) => result.andThen(f)))" id="AsyncResult<U#3, E#6 | F#2>"
        /// @type.node source="this.then((result) => result.andThen(f))" type=Promise<Result<U#3, E#6 | F#2>>
        /// @type.node source=this type=AsyncResult<T#7, E#6>
        /// @type.node source=this.then type=<U#1>(this: Promise<Result<T#7, E#6>>, Function<(Result<T#7, E#6>,), U#1 | Promise<U#1>>) => Promise<U#1>
        /// @resolution.member source=this.then receiver=AsyncResult<T#7, E#6> type=<U#1>(this: Promise<Result<T#7, E#6>>, Function<(Result<T#7, E#6>,), U#1 | Promise<U#1>>) => Promise<U#1> kind=symbol target_receiver=AsyncResult<T#7, E#6> adjustments=(newtype.payload(AsyncResult, Promise<Result<T#7, E#6>>)) target=Promise.then
        /// @resolution.call source="this.then((result) => result.andThen(f))" parameters=(Function<(Result<T#7, E#6>,), Result<U#3, E#6 | F#2> | Promise<Result<U#3, E#6 | F#2>>>) arguments=(provided((result) => result.andThen(f)) as Function<(Result<T#7, E#6>,), Result<U#3, E#6 | F#2> | Promise<Result<U#3, E#6 | F#2>>>) return=Promise<Result<U#3, E#6 | F#2>> kind=symbol target=Promise.then receiver=AsyncResult<T#7, E#6> adjustments=(newtype.payload(AsyncResult, Promise<Result<T#7, E#6>>)) instance="Promise<Result<T#7, E#6>>.then<Result<U#3, E#6 | F#2>>"
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=AsyncResult<T#7, E#6>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instance source="this.then((result) => result.andThen(f))" id="Promise<Result<T#7, E#6>>.then<Result<U#3, E#6 | F#2>>"
        /// @generic.instance source="this.then((result) => result.andThen(f))" id="Promise<Result<U#3, E#6 | F#2>>"
        /// @generic.instance source="this.then((result) => result.andThen(f))" id="Result<U#3, E#6 | F#2>"
        /// @generic.instance source=this id="AsyncResult<T#7, E#6>"
        /// @generic.instance source=this.then id="Promise<Result<T#7, E#6>>"
        /// @generic.instance source=this.then id="Result<T#7, E#6>"
        /// @generic.instance source=this.then id=Promise<U#1>
        /// @type.symbol symbol=andThenSync.symbol47 source=(result) => result.andThen(f) type=Function<(Result<T#7, E#6>,), Result<U#3, E#6 | F#2>>
        /// @type.node source=(result) => result.andThen(f) type=Function<(Result<T#7, E#6>,), Result<U#3, E#6 | F#2>>
        /// @generic.instance source=(result) => result.andThen(f) id="Result<T#7, E#6>"
        /// @generic.instance source=(result) => result.andThen(f) id="Result<U#3, E#6 | F#2>"
        /// @type.symbol symbol=andThenSync.symbol47.result source=result type=Result<T#7, E#6>
        /// @type.node source=result type=Result<T#7, E#6>
        /// @type.node source=result.andThen type=<U#2, F#1>(this: Result<T#7, E#6>, Function<(T#7,), Result<U#2, F#1>>) => Result<U#2, E#6 | F#1>
        /// @type.node source=result.andThen(f) type=Result<U#3, E#6 | F#2>
        /// @resolution.name source=result target=andThenSync.symbol47.result
        /// @resolution.member source=result.andThen receiver=Result<T#7, E#6> type=<U#2, F#1>(this: Result<T#7, E#6>, Function<(T#7,), Result<U#2, F#1>>) => Result<U#2, E#6 | F#1> kind=symbol target_receiver=Result<T#7, E#6> target=andThen
        /// @resolution.call source=result.andThen(f) parameters=(Function<(T#7,), Result<U#3, F#2>>) arguments=(provided(f) as Function<(T#7,), Result<U#3, F#2>>) return=Result<U#3, E#6 | F#2> kind=symbol target=andThen receiver=Result<T#7, E#6> instance="Result<T#7, E#6>.<extension#1>.andThen<U#3, F#2>"
        /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result root=andThenSync.symbol47.result
        /// @generic.instance source=result id="Result<T#7, E#6>"
        /// @generic.instance source=result.andThen id="Result<T#7, E#6>"
        /// @generic.instance source=result.andThen id="Result<U#2, E#6 | F#1>"
        /// @generic.instance source=result.andThen id="Result<U#2, F#1>"
        /// @generic.instance source=result.andThen(f) id="Result<T#7, E#6>.<extension#1>.andThen<U#3, F#2>"
        /// @generic.instance source=result.andThen(f) id="Result<U#3, E#6 | F#2>"
        /// @type.node source=f type=Function<(T#7,), Result<U#3, F#2>>
        /// @resolution.name source=f target=andThenSync.f
        /// @resolution.place source=f placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=f root=andThenSync.f
        /// @generic.instance source=f id="Result<U#3, F#2>"

    }
}

/// @generic.instance id="AsyncResult<T#7, E#6>" template=AsyncResult arguments=(T#7, E#6)
/// @generic.instance id="AsyncResult<U#3, E#6 | F#2>" template=AsyncResult arguments=(U#3, E#6 | F#2)
/// @generic.instance id="Promise<Result<T#7, E#6>>" template=Promise arguments=(Result<T#7, E#6>)
/// @generic.instance id="Promise<Result<T#7, E#6>>.then<Result<U#3, E#6 | F#2>>" template=Promise.then arguments=(Result<T#7, E#6>, Result<U#3, E#6 | F#2>)
/// @generic.instance id="Promise<Result<U#3, E#6 | F#2>>" template=Promise arguments=(Result<U#3, E#6 | F#2>)
/// @generic.instance id="Result<T#4, E#3>" template=Result arguments=(T#4, E#3)
/// @generic.instance id="Result<T#7, E#6>" template=Result arguments=(T#7, E#6)
/// @generic.instance id="Result<T#7, E#6>.<extension#1>.andThen<U#3, F#2>" template=andThen arguments=(T#7, E#6, U#3, F#2)
/// @generic.instance id="Result<U#2, E#4 | F#1>" template=Result arguments=(U#2, E#4 | F#1)
/// @generic.instance id="Result<U#2, E#6 | F#1>" template=Result arguments=(U#2, E#6 | F#1)
/// @generic.instance id="Result<U#2, F#1>" template=Result arguments=(U#2, F#1)
/// @generic.instance id="Result<U#3, E#6 | F#2>" template=Result arguments=(U#3, E#6 | F#2)
/// @generic.instance id="Result<U#3, F#2>" template=Result arguments=(U#3, F#2)
/// @generic.instance id="result<U#2, E#4 | F#1>" template=result arguments=(U#2, E#4 | F#1)
/// @generic.instance id=Promise<U#1> template=Promise arguments=(U#1)
"#);
}
