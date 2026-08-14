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
    /// @resolution.subscript source=values[0] type=T kind=call target="collections.array.index#1(parameters=(usize), arguments=(provided(0) as usize), return=memory.type.WithAccess<&'frame T, \"exclusive\">)"
    /// @generic.instance source=values[0] id="Array<T>.<extension#4>.index#1<\"exclusive\">"
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

/// @generic.instance id="Array<T>.<extension#4>.index#1<\"exclusive\">" template=collections.array.index#1 arguments=(T, "exclusive")
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
    /// @resolution.subscript source=value[0] type=T#2 kind=call target="collections.array.index#1(parameters=(usize), arguments=(provided(0) as usize), return=memory.type.WithAccess<&'frame T#2, \"exclusive\">)"
    /// @generic.instance source=value[0] id="Array<T#2>.<extension#4>.index#1<\"exclusive\">"
    /// @type.node source=0 type=0

}

const parser = parse<int32>;
/// @type.symbol symbol=parser source=parser type=<error>
/// @resolution.pattern source=parser kind=binding target=parser
/// @type.node source=parse<int32> type=<error>
/// @resolution.name source=parse target=[parse#1, parse#2]
/// @resolution.rejected source=parse<int32>

/// @generic.instance id="Array<T#2>.<extension#4>.index#1<\"exclusive\">" template=collections.array.index#1 arguments=(T#2, "exclusive")
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
/// @resolution.member receiver=Owned<Array<int32>> type=(this: Owned<Array<int32>>, Function<(&type_expression.'a readonly int32, usize), boolean>) => Owned<Array<int32>> & (this: Owned<Array<int32>>, Function<(int32, usize), boolean>) => Owned<Array<int32>> kind=existential targets=[collections.array.filter#1, collections.array.filter#2]
/// @resolution.member receiver=Owned<Array<int32>> type=<collections.array.map.U#1>(this: Owned<Array<int32>>, Function<(int32, usize), collections.array.map.U#1>) => Owned<Array<collections.array.map.U#1>> & <collections.array.map.U#2>(this: Owned<Array<int32>>, Function<(int32, usize), collections.array.map.U#2>) => Owned<Array<collections.array.map.U#2>> kind=existential targets=[collections.array.map#1, collections.array.map#2]
/// @resolution.call parameters=(Function<(&type_expression.'a readonly int32, usize), boolean>) arguments=(provided((value) => value !== undefined) as Function<(&type_expression.'a readonly int32, usize), boolean>) return=Owned<Array<int32>> kind=symbol target=collections.array.filter#1 receiver=Owned<Array<int32>> instance=Owned<Array<collections.array.T#1>>.<extension#1>.filter#1
/// @resolution.call parameters=(Function<(int32, usize), int32>) arguments=(provided((value) => value) as Function<(int32, usize), int32>) return=Owned<Array<int32>> kind=symbol target=collections.array.map#1 receiver=Owned<Array<int32>> instance=Owned<Array<collections.array.T#1>>.<extension#1>.map#1<int32>
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
/// @generic.instance id=Owned<Array<collections.array.T#1>>.<extension#1>.filter#1 template=collections.array.filter#1 arguments=(int32)
/// @generic.instance id=Owned<Array<collections.array.T#1>>.<extension#1>.map#1<int32> template=collections.array.map#1 arguments=(int32, int32)
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

    session.assert_dir_checked("main.ds", DirRows::checked().with_reference_types(), r#""#);
}
