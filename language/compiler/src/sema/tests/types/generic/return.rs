use crate::tests::{DirRows, TestSession};

#[test]
fn test_construct_a_family_bounded_result_from_a_fitting_literal() {
    let session = TestSession::single(
        r#"
function make<T: int8 | int64>(): T {
    return 1;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function make<T: int8 | int64>(): T {
    return 1;
}

=== checked ===
function make<T: int8 | int64>(): T {
/// @generic.template symbol=make parameters=(T: int8 | int64)
/// @type.symbol symbol=make type=<T: int8 | int64>() => T
/// @type.symbol symbol=make.T source="T: int8 | int64" type=T
/// @resolution.name source=T target=make.T

    return 1;
}
"#,
        r#"
"#,
    );
}

#[test]
fn test_generic_return_contextualizes_empty_array_field() {
    let session = TestSession::single(
        r#"
function capture<T>(value: T): { reactions: T[] } {
    return { reactions: [] };
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function capture<T>(value: T): { reactions: T[] } {
    return { reactions: [] };
}

=== checked ===
function capture<T>(value: T): { reactions: T[] } {
/// @generic.template symbol=capture parameters=(T)
/// @type.symbol symbol=capture type=<T>(T) => { reactions: Array<T> }
/// @type.symbol symbol=capture.T source=T type=T
/// @type.symbol symbol=capture.value source="value: T" type=T
/// @resolution.name source=T target=capture.T
/// @resolution.name source=T target=capture.T

    return { reactions: [] };
    /// @type.node source={ reactions: [] } type={ reactions: Array<T> }
    /// @type.node source=[] type=Array<T>

}
"#,
    );
}

#[test]
fn test_generic_union_return_contextualizes_empty_array_field() {
    let session = TestSession::single(
        r#"
interface Pending<T> {
    kind: "pending";
    reactions: T[];
}

interface Done<T> {
    kind: "done";
    value: T;
}

type State<T> = Pending<T> | Done<T>;

function pending<T>(): State<T> {
    return { kind: "pending", reactions: [] };
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Pending<in out T> {
    kind: "pending";
    reactions: T[];
}

interface Done<in out T> {
    kind: "done";
    value: T;
}

type State<T> = Pending<T> | Done<T>;

function pending<T>(): State<T> {
    return { kind: "pending", reactions: [] } as Dynamic<Pending<T>> | Dynamic<Done<T>>;
}

=== checked ===
interface Pending<T> {
/// @generic.template symbol=Pending parameters=(in out T#1)
/// @type.symbol symbol=Pending type=Pending
/// @definition.interface symbol=Pending template=(in out T#1)
/// @definition.where symbol=Pending relation=satisfies left=this right=Pending<T#1>
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.reactions source="reactions: T[]" key=reactions type=Array<T#1>
/// @type.symbol symbol=Pending.T source=T type=T#1

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    reactions: T[];
    /// @type.symbol symbol=Pending.reactions source="reactions: T[]" type=Array<T#1>
    /// @resolution.name source=T target=Pending.T

}

interface Done<T> {
/// @generic.template symbol=Done parameters=(in out T#2)
/// @type.symbol symbol=Done type=Done
/// @definition.interface symbol=Done template=(in out T#2)
/// @definition.where symbol=Done relation=satisfies left=this right=Done<T#2>
/// @definition.field symbol=Done.kind source="kind: \"done\"" key=kind type="done"
/// @definition.field symbol=Done.value source="value: T" key=value type=T#2
/// @type.symbol symbol=Done.T source=T type=T#2

    kind: "done";
    /// @type.symbol symbol=Done.kind source="kind: \"done\"" type="done"

    value: T;
    /// @type.symbol symbol=Done.value source="value: T" type=T#2
    /// @resolution.name source=T target=Done.T

}

type State<T> = Pending<T> | Done<T>;
/// @generic.template symbol=State parameters=(T#3)
/// @type.symbol symbol=State source="type State<T> = Pending<T> | Done<T>" type=Pending<T#3> | Done<T#3>
/// @definition.type symbol=State source="type State<T> = Pending<T> | Done<T>" template=(T#3) value=Pending<T#3> | Done<T#3>
/// @type.symbol symbol=State.T source=T type=T#3
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=T target=State.T
/// @resolution.name source=Done target=Done
/// @resolution.name source=T target=State.T

function pending<T>(): State<T> {
/// @generic.template symbol=pending parameters=(T#4)
/// @type.symbol symbol=pending type=<T#4>() => State<T#4>
/// @type.symbol symbol=pending.T source=T type=T#4
/// @resolution.name source=State target=State
/// @resolution.name source=T target=pending.T

    return { kind: "pending", reactions: [] };
    /// @type.node source={ kind: "pending", reactions: [] } type={ kind: "pending"; reactions: Array<T#4> }
    /// @type.node source="\"pending\"" type="pending"
    /// @type.node source=[] type=Array<T#4>

}

/// @generic.instance id=Done<T#3> template=Done arguments=(T#3)
/// @generic.instance id=Pending<T#3> template=Pending arguments=(T#3)
/// @generic.instance id=State<T#4> template=State arguments=(T#4)
"#,
    );
}

#[test]
fn test_recursive_return_inference_requires_annotation() {
    let session = TestSession::single(
        r#"
function countdown(n: float64) {
    return n > 0 ? countdown(n - 1) : n;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function countdown(n: float64) {
    return n > 0 ? countdown(n - 1) : n;
}

=== checked ===
function countdown(n: float64) {
/// @type.symbol symbol=countdown type=(float64) => <error>
/// @type.symbol symbol=countdown.n source="n: float64" type=float64

    return n > 0 ? countdown(n - 1) : n;
    /// @resolution.name source=n target=countdown.n
    /// @resolution.operator source="n > 0" type=boolean operator=">" kind=builtin operands=[n as float64 families=(float), 0 as float64 families=(float)]
    /// @resolution.place source=n placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=n root=countdown.n
    /// @resolution.name source=countdown target=countdown
    /// @resolution.call source="countdown(n - 1)" parameters=(float64) arguments=(provided(n - 1) as float64) return=<error> kind=symbol target=countdown
    /// @resolution.name source=n target=countdown.n
    /// @resolution.operator source="n - 1" type=float64 operator="-" kind=builtin operands=[n as float64 families=(float), 1 as float64 families=(float)]
    /// @resolution.place source=n placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=n root=countdown.n
    /// @resolution.name source=n target=countdown.n
    /// @resolution.place source=n placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=n root=countdown.n

}
"#,
        r#"
/// @diagnostic.error id=missing-result-type message="function declaration needs a written result type"
/// @diagnostic.label line=2 column=10 span="countdown" line_source="function countdown(n: float64) {"
/// @diagnostic.help message="state the result type on the declaration"
"#,
    );
}
#[test]
fn test_mutually_recursive_returns_require_annotations() {
    let session = TestSession::single(
        r#"
function ping(n: float64) {
    return n > 0 ? pong(n - 1) : n;
}

function pong(n: float64) {
    return n > 0 ? ping(n - 1) : n;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function ping(n: float64) {
    return n > 0 ? pong(n - 1) : n;
}

function pong(n: float64) {
    return n > 0 ? ping(n - 1) : n;
}

=== checked ===
function ping(n: float64) {
/// @type.symbol symbol=ping type=(float64) => <error>
/// @type.symbol symbol=ping.n source="n: float64" type=float64

    return n > 0 ? pong(n - 1) : n;
    /// @resolution.name source=n target=ping.n
    /// @resolution.operator source="n > 0" type=boolean operator=">" kind=builtin operands=[n as float64 families=(float), 0 as float64 families=(float)]
    /// @resolution.place source=n placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=n root=ping.n
    /// @resolution.name source=pong target=pong
    /// @resolution.call source="pong(n - 1)" parameters=(float64) arguments=(provided(n - 1) as float64) return=<error> kind=symbol target=pong
    /// @resolution.name source=n target=ping.n
    /// @resolution.operator source="n - 1" type=float64 operator="-" kind=builtin operands=[n as float64 families=(float), 1 as float64 families=(float)]
    /// @resolution.place source=n placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=n root=ping.n
    /// @resolution.name source=n target=ping.n
    /// @resolution.place source=n placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=n root=ping.n

}

function pong(n: float64) {
/// @type.symbol symbol=pong type=(float64) => <error>
/// @type.symbol symbol=pong.n source="n: float64" type=float64

    return n > 0 ? ping(n - 1) : n;
    /// @resolution.name source=n target=pong.n
    /// @resolution.operator source="n > 0" type=boolean operator=">" kind=builtin operands=[n as float64 families=(float), 0 as float64 families=(float)]
    /// @resolution.place source=n placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=n root=pong.n
    /// @resolution.name source=ping target=ping
    /// @resolution.call source="ping(n - 1)" parameters=(float64) arguments=(provided(n - 1) as float64) return=<error> kind=symbol target=ping
    /// @resolution.name source=n target=pong.n
    /// @resolution.operator source="n - 1" type=float64 operator="-" kind=builtin operands=[n as float64 families=(float), 1 as float64 families=(float)]
    /// @resolution.place source=n placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=n root=pong.n
    /// @resolution.name source=n target=pong.n
    /// @resolution.place source=n placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=n root=pong.n

}
"#,
        r#"
/// @diagnostic.error id=missing-result-type message="function declaration needs a written result type"
/// @diagnostic.label line=2 column=10 span="ping" line_source="function ping(n: float64) {"
/// @diagnostic.help message="state the result type on the declaration"
/// @diagnostic.error id=missing-result-type message="function declaration needs a written result type"
/// @diagnostic.label line=6 column=10 span="pong" line_source="function pong(n: float64) {"
/// @diagnostic.help message="state the result type on the declaration"
"#,
    );
}

/// Contextualize a nested generic call through an overloaded promise result.
#[test]
fn test_generic_return_contextualizes_nested_call_result() {
    let session = TestSession::single(
        r#"
declare class Promise<in out T> {
    static resolve<T>(value: Promise<T>): Promise<T>;
    static resolve<T>(value: T): Promise<T>;
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

declare function ok<T, E>(value: T): Result<T, E>;

newtype AsyncResult<T, E> = Promise<Result<T, E>>;

function make<T, E>(value: T): AsyncResult<T, E> {
    return AsyncResult(Promise.resolve(ok(value)));
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked().with_reference_types(), r#""#);
}
