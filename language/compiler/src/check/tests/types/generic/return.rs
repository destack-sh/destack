use crate::tests::{DirRows, TestSession};

#[test]
fn test_literal_cannot_construct_rigid_generic_return() {
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
/// @diagnostic.error id=return-not-assignable message="type '1' is not assignable to the declared result type 'T'"
/// @diagnostic.label line=3 column=12 span="1" line_source="return 1;"
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
fn test_generic_return_contextualizes_nested_call_result() {
    let session = TestSession::single(
        r#"
declare class Promise<in out T> {
    static resolve<T>(value: Promise<T>): Promise<T>;
    static resolve<T>(value: T): Promise<T>;
}

struct Ok<T> {
    kind: "Ok" = "Ok";
    value: T;
}

struct Err<E> {
    kind: "Err" = "Err";
    error: E;
}

@derive(Tagged)
newtype Result<T, E> = Ok<T> | Err<E>;

extension<T, E> of Result<T, E> {
    static ok(value: T): Result<T, E> {
        Result.Ok<T, E>({ value })
    }
}

newtype AsyncResult<T, E> = Promise<Result<T, E>>;

function ok<T, E>(value: T): AsyncResult<T, E> {
    return AsyncResult(Promise.resolve(Result.ok(value)));
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked()
            .with_reference_types()
            ,
        r#"
=== annotated ===
declare class Promise<in out T> {
    static resolve<T>(value: Promise<T>): Promise<T>;
    static resolve<T>(value: T): Promise<T>;
}

struct Ok<out T> {
    kind: "Ok" = "Ok";
    value: T;
}

struct Err<out E> {
    kind: "Err" = "Err";
    error: E;
}

@derive(Tagged)
newtype Result<out T, out E> = Ok<T> | Err<E>;

extension<T, E> of Result<T, E> {
    static ok(value: T): Result<T, E> {
        Result.Ok<T, E>({ value })
    }
}

newtype AsyncResult<in out T, in out E> = Promise<Result<T, E>>;

function ok<T, E>(value: T): AsyncResult<T, E> {
    return AsyncResult(Promise.resolve<Result<T, E>>(Result.ok<T, E>(value)));
}

=== checked ===
declare class Promise<in out T> {
/// @generic.template symbol=Promise parameters=(in out T#1)
/// @type.symbol symbol=Promise type=Promise
/// @definition.class symbol=Promise template=(in out T#1)
/// @definition.method symbol=Promise.resolve#1 source="static resolve<T>(value: Promise<T>): Promise<T>" slot=resolve static=true type=<T#2>(Promise<T#2>) => Promise<T#2>
/// @definition.method symbol=Promise.resolve#2 source="static resolve<T>(value: T): Promise<T>" slot=resolve static=true type=<T#3>(T#3) => Promise<T#3>
/// @type.symbol symbol=Promise.T source="in out T" type=T#1

    static resolve<T>(value: Promise<T>): Promise<T>;
    /// @generic.template symbol=Promise.resolve#1 parent=template#0 parameters=(T#2)
    /// @type.symbol symbol=Promise.resolve#1 source="static resolve<T>(value: Promise<T>): Promise<T>" type=<T#2>(Promise<T#2>) => Promise<T#2>
    /// @type.symbol symbol=Promise.resolve.T#1 source=T type=T#2
    /// @type.symbol symbol=Promise.resolve.value#1 source="value: Promise<T>" type=Promise<T#2>
    /// @resolution.name source=Promise target=Promise
    /// @resolution.name source=T target=Promise.resolve.T#1
    /// @resolution.name source=Promise target=Promise
    /// @resolution.name source=T target=Promise.resolve.T#1

    static resolve<T>(value: T): Promise<T>;
    /// @generic.template symbol=Promise.resolve#2 parent=template#0 parameters=(T#3)
    /// @type.symbol symbol=Promise.resolve#2 source="static resolve<T>(value: T): Promise<T>" type=<T#3>(T#3) => Promise<T#3>
    /// @type.symbol symbol=Promise.resolve.T#2 source=T type=T#3
    /// @type.symbol symbol=Promise.resolve.value#2 source="value: T" type=T#3
    /// @resolution.name source=T target=Promise.resolve.T#2
    /// @resolution.name source=Promise target=Promise
    /// @resolution.name source=T target=Promise.resolve.T#2

}

struct Ok<T> {
/// @generic.template symbol=Ok parameters=(out T#4)
/// @type.symbol symbol=Ok type=Ok
/// @definition.struct symbol=Ok template=(out T#4)
/// @definition.field symbol=Ok.kind source="kind: \"Ok\" = \"Ok\"" key=kind type="Ok"
/// @definition.field symbol=Ok.value source="value: T" key=value type=T#4
/// @type.symbol symbol=Ok.T source=T type=T#4

    kind: "Ok" = "Ok";
    /// @type.symbol symbol=Ok.kind source="kind: \"Ok\" = \"Ok\"" type="Ok"
    /// @type.node source="\"Ok\"" type="Ok"

    value: T;
    /// @type.symbol symbol=Ok.value source="value: T" type=T#4
    /// @resolution.name source=T target=Ok.T

}

struct Err<E> {
/// @generic.template symbol=Err parameters=(out E#1)
/// @type.symbol symbol=Err type=Err
/// @definition.struct symbol=Err template=(out E#1)
/// @definition.field symbol=Err.error source="error: E" key=error type=E#1
/// @definition.field symbol=Err.kind source="kind: \"Err\" = \"Err\"" key=kind type="Err"
/// @type.symbol symbol=Err.E source=E type=E#1

    kind: "Err" = "Err";
    /// @type.symbol symbol=Err.kind source="kind: \"Err\" = \"Err\"" type="Err"
    /// @type.node source="\"Err\"" type="Err"

    error: E;
    /// @type.symbol symbol=Err.error source="error: E" type=E#1
    /// @resolution.name source=E target=Err.E

}

@derive(Tagged)
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Result<T, E> = Ok<T> | Err<E>;
/// @generic.template symbol=Result parameters=(out T#5, out E#2)
/// @type.symbol symbol=Result source="newtype Result<T, E> = Ok<T> | Err<E>" type=Result
/// @type.symbol symbol=Result.Err type=<T#5, E#2>({ error: E#2 }) => Result.Err<T#5, E#2>
/// @type.symbol symbol=Result.Ok type=<T#5, E#2>({ value: T#5 }) => Result.Ok<T#5, E#2>
/// @definition.newtype symbol=Result source="newtype Result<T, E> = Ok<T> | Err<E>" template=(out T#5, out E#2) discriminator=kind backing=Ok<T#5> | Err<E#2>
/// @definition.variant symbol=Result.Err source="newtype Result<T, E> = Ok<T> | Err<E>" key=Err discriminant=Err backing=Err<E#2> argument={ error: E#2 }
/// @definition.variant symbol=Result.Ok source="newtype Result<T, E> = Ok<T> | Err<E>" key=Ok discriminant=Ok backing=Ok<T#5> argument={ value: T#5 }
/// @type.symbol symbol=Result.T source=T type=T#5
/// @type.symbol symbol=Result.E source=E type=E#2
/// @resolution.name source=Ok target=Ok
/// @resolution.name source=T target=Result.T
/// @resolution.name source=Err target=Err
/// @resolution.name source=E target=Result.E

extension<T, E> of Result<T, E> {
/// @generic.template symbol=<module>#2 parameters=(T#6, E#3)
/// @definition.extension symbol=<module>#2 form=local target=Result<T#6, E#3>
/// @definition.method symbol=ok#1 slot=ok static=true type=(T#6) => Result<T#6, E#3>
/// @type.symbol symbol=T source=T type=T#6
/// @type.symbol symbol=E source=E type=E#3
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=T
/// @resolution.name source=E target=E

    static ok(value: T): Result<T, E> {
    /// @type.symbol symbol=ok#1 type=(T#6) => Result<T#6, E#3>
    /// @type.symbol symbol=ok.value#1 source="value: T" type=T#6
    /// @resolution.name source=T target=T
    /// @resolution.name source=Result target=Result
    /// @resolution.name source=T target=T
    /// @resolution.name source=E target=E

        Result.Ok<T, E>({ value })
        /// @type.node source="Result.Ok<T, E>({ value })" type=Result.Ok<T#6, E#3>
        /// @type.node source=Result type=Result
        /// @type.node source=Result.Ok type=<T#5, E#2>({ value: T#5 }) => Result.Ok<T#5, E#2>
        /// @resolution.name source=Result target=Result
        /// @resolution.member source=Result.Ok receiver=Result type=<T#5, E#2>({ value: T#5 }) => Result.Ok<T#5, E#2> kind=symbol target_receiver=Result target=Result.Ok
        /// @resolution.construct source="Result.Ok<T, E>({ value })" parameters=({ value: T#6 }) arguments=(provided({ value }) as { value: T#6 }) return=Result.Ok<T#6, E#3> kind=variant owner=Result variant=Ok instance="Result<T#6, E#3>" backing=Ok<T#6> argument={ value: T#6 } discriminant=Ok
        /// @generic.instance source="Result.Ok<T, E>({ value })" id="Result<T#6, E#3>"
        /// @generic.instance source=Result.Ok id="Result<T#5, E#2>"
        /// @resolution.name source=T target=T
        /// @resolution.name source=E target=E
        /// @type.node source={ value } type={ value: T#6 }
        /// @type.node source=value type=T#6
        /// @resolution.name source=value target=ok.value#1
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=ok.value#1

    }
}

newtype AsyncResult<T, E> = Promise<Result<T, E>>;
/// @generic.template symbol=AsyncResult parameters=(in out T#7, in out E#4)
/// @type.symbol symbol=AsyncResult source="newtype AsyncResult<T, E> = Promise<Result<T, E>>" type=AsyncResult
/// @definition.newtype symbol=AsyncResult source="newtype AsyncResult<T, E> = Promise<Result<T, E>>" template=(in out T#7, in out E#4) backing=Promise<Result<T#7, E#4>> constructors=[<T#7, E#4>(Promise<Result<T#7, E#4>>) => AsyncResult<T#7, E#4>]
/// @type.symbol symbol=AsyncResult.T source=T type=T#7
/// @type.symbol symbol=AsyncResult.E source=E type=E#4
/// @resolution.name source=Promise target=Promise
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=AsyncResult.T
/// @resolution.name source=E target=AsyncResult.E

function ok<T, E>(value: T): AsyncResult<T, E> {
/// @generic.template symbol=ok#2 parameters=(T#8, E#5)
/// @type.symbol symbol=ok#2 type=<T#8, E#5>(T#8) => AsyncResult<T#8, E#5>
/// @type.symbol symbol=ok.T source=T type=T#8
/// @type.symbol symbol=ok.E source=E type=E#5
/// @type.symbol symbol=ok.value#2 source="value: T" type=T#8
/// @resolution.name source=T target=ok.T
/// @resolution.name source=AsyncResult target=AsyncResult
/// @resolution.name source=T target=ok.T
/// @resolution.name source=E target=ok.E

    return AsyncResult(Promise.resolve(Result.ok(value)));
    /// @type.node source=AsyncResult type=AsyncResult
    /// @type.node source=AsyncResult(Promise.resolve(Result.ok(value))) type=AsyncResult<T#8, E#5>
    /// @resolution.name source=AsyncResult target=AsyncResult
    /// @resolution.construct source=AsyncResult(Promise.resolve(Result.ok(value))) parameters=(Promise<Result<T#8, E#5>>) arguments=(provided(Promise.resolve(Result.ok(value))) as Promise<Result<T#8, E#5>>) return=AsyncResult<T#8, E#5> kind=newtype target=AsyncResult backing=Promise<Result<T#8, E#5>> instance="AsyncResult<T#8, E#5>"
    /// @generic.instance source=AsyncResult(Promise.resolve(Result.ok(value))) id="AsyncResult<T#8, E#5>"
    /// @type.node source=Promise type=Promise
    /// @type.node source=Promise.resolve type=<T#2>(Promise<T#2>) => Promise<T#2> & <T#3>(T#3) => Promise<T#3>
    /// @type.node source=Promise.resolve(Result.ok(value)) type=Promise<Result<T#8, E#5>>
    /// @resolution.name source=Promise target=Promise
    /// @resolution.member source=Promise.resolve receiver=Promise type=<T#2>(Promise<T#2>) => Promise<T#2> & <T#3>(T#3) => Promise<T#3> kind=existential targets=[Promise.resolve#1, Promise.resolve#2]
    /// @resolution.call source=Promise.resolve(Result.ok(value)) parameters=(Result<T#8, E#5>) arguments=(provided(Result.ok(value)) as Result<T#8, E#5>) return=Promise<Result<T#8, E#5>> kind=symbol target=Promise.resolve#2 instance="Promise.resolve#2<Result<T#8, E#5>>"
    /// @generic.instance source=Promise.resolve id=Promise<T#2>
    /// @generic.instance source=Promise.resolve id=Promise<T#3>
    /// @generic.instance source=Promise.resolve(Result.ok(value)) id="Promise.resolve#2<Result<T#8, E#5>>"
    /// @generic.instance source=Promise.resolve(Result.ok(value)) id="Promise<Result<T#8, E#5>>"
    /// @generic.instance source=Promise.resolve(Result.ok(value)) id="Result<T#8, E#5>"
    /// @type.node source=Result type=Result
    /// @type.node source=Result.ok type=(T#6) => Result<T#6, E#3>
    /// @type.node source=Result.ok(value) type=Result<T#8, E#5>
    /// @resolution.name source=Result target=Result
    /// @resolution.member source=Result.ok receiver=Result type=(T#6) => Result<T#6, E#3> kind=symbol target_receiver=Result target=ok#1
    /// @resolution.call source=Result.ok(value) parameters=(T#8) arguments=(provided(value) as T#8) return=Result<T#8, E#5> kind=symbol target=ok#1 instance="Result<T#8, E#5>.<extension#1>.ok#1"
    /// @generic.instance source=Result.ok id="Result<T#6, E#3>"
    /// @generic.instance source=Result.ok(value) id="Result<T#8, E#5>"
    /// @generic.instance source=Result.ok(value) id="Result<T#8, E#5>.<extension#1>.ok#1"
    /// @type.node source=value type=T#8
    /// @resolution.name source=value target=ok.value#2
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=ok.value#2

}

/// @generic.instance id="AsyncResult<T#8, E#5>" template=AsyncResult arguments=(T#8, E#5)
/// @generic.instance id="Promise.resolve#2<Result<T#8, E#5>>" template=Promise.resolve#2 arguments=(Result<T#8, E#5>)
/// @generic.instance id="Promise<Result<T#8, E#5>>" template=Promise arguments=(Result<T#8, E#5>)
/// @generic.instance id="Result<T#5, E#2>" template=Result arguments=(T#5, E#2)
/// @generic.instance id="Result<T#6, E#3>" template=Result arguments=(T#6, E#3)
/// @generic.instance id="Result<T#8, E#5>" template=Result arguments=(T#8, E#5)
/// @generic.instance id="Result<T#8, E#5>.<extension#1>.ok#1" template=ok#1 arguments=(T#8, E#5)
/// @generic.instance id=Promise<T#2> template=Promise arguments=(T#2)
/// @generic.instance id=Promise<T#3> template=Promise arguments=(T#3)
"#,
    );
}

#[test]
fn test_generic_return_contextualizes_tagged_variant_result() {
    let session = TestSession::single(
        r#"
struct Ok<T> {
    kind: "Ok" = "Ok";
    value: T;
}

struct Err<E> {
    kind: "Err" = "Err";
    error: E;
}

@derive(Tagged)
newtype Result<T, E> = Ok<T> | Err<E>;

extension<T, E> of Result<T, E> {
    static ok(value: T): Result<T, E> {
        Result.Ok({ value })
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Ok<out T> {
    kind: "Ok" = "Ok";
    value: T;
}

struct Err<out E> {
    kind: "Err" = "Err";
    error: E;
}

@derive(Tagged)
newtype Result<out T, out E> = Ok<T> | Err<E>;

extension<T, E> of Result<T, E> {
    static ok(value: T): Result<T, E> {
        Result.Ok({ value })
    }
}

=== checked ===
struct Ok<T> {
/// @generic.template symbol=Ok parameters=(out T#1)
/// @type.symbol symbol=Ok type=Ok
/// @definition.struct symbol=Ok template=(out T#1)
/// @definition.field symbol=Ok.kind source="kind: \"Ok\" = \"Ok\"" key=kind type="Ok"
/// @definition.field symbol=Ok.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Ok.T source=T type=T#1

    kind: "Ok" = "Ok";
    /// @type.symbol symbol=Ok.kind source="kind: \"Ok\" = \"Ok\"" type="Ok"
    /// @type.node source="\"Ok\"" type="Ok"

    value: T;
    /// @type.symbol symbol=Ok.value source="value: T" type=T#1
    /// @resolution.name source=T target=Ok.T

}

struct Err<E> {
/// @generic.template symbol=Err parameters=(out E#1)
/// @type.symbol symbol=Err type=Err
/// @definition.struct symbol=Err template=(out E#1)
/// @definition.field symbol=Err.error source="error: E" key=error type=E#1
/// @definition.field symbol=Err.kind source="kind: \"Err\" = \"Err\"" key=kind type="Err"
/// @type.symbol symbol=Err.E source=E type=E#1

    kind: "Err" = "Err";
    /// @type.symbol symbol=Err.kind source="kind: \"Err\" = \"Err\"" type="Err"
    /// @type.node source="\"Err\"" type="Err"

    error: E;
    /// @type.symbol symbol=Err.error source="error: E" type=E#1
    /// @resolution.name source=E target=Err.E

}

@derive(Tagged)
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Result<T, E> = Ok<T> | Err<E>;
/// @generic.template symbol=Result parameters=(out T#2, out E#2)
/// @type.symbol symbol=Result source="newtype Result<T, E> = Ok<T> | Err<E>" type=Result
/// @type.symbol symbol=Result.Err type=<T#2, E#2>({ error: E#2 }) => Result.Err<T#2, E#2>
/// @type.symbol symbol=Result.Ok type=<T#2, E#2>({ value: T#2 }) => Result.Ok<T#2, E#2>
/// @definition.newtype symbol=Result source="newtype Result<T, E> = Ok<T> | Err<E>" template=(out T#2, out E#2) discriminator=kind backing=Ok<T#2> | Err<E#2>
/// @definition.variant symbol=Result.Err source="newtype Result<T, E> = Ok<T> | Err<E>" key=Err discriminant=Err backing=Err<E#2> argument={ error: E#2 }
/// @definition.variant symbol=Result.Ok source="newtype Result<T, E> = Ok<T> | Err<E>" key=Ok discriminant=Ok backing=Ok<T#2> argument={ value: T#2 }
/// @type.symbol symbol=Result.T source=T type=T#2
/// @type.symbol symbol=Result.E source=E type=E#2
/// @resolution.name source=Ok target=Ok
/// @resolution.name source=T target=Result.T
/// @resolution.name source=Err target=Err
/// @resolution.name source=E target=Result.E

extension<T, E> of Result<T, E> {
/// @generic.template symbol=<module>#2 parameters=(T#3, E#3)
/// @definition.extension symbol=<module>#2 form=local target=Result<T#3, E#3>
/// @definition.method symbol=ok slot=ok static=true type=(T#3) => Result<T#3, E#3>
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=E source=E type=E#3
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=T
/// @resolution.name source=E target=E

    static ok(value: T): Result<T, E> {
    /// @type.symbol symbol=ok type=(T#3) => Result<T#3, E#3>
    /// @type.symbol symbol=ok.value source="value: T" type=T#3
    /// @resolution.name source=T target=T
    /// @resolution.name source=Result target=Result
    /// @resolution.name source=T target=T
    /// @resolution.name source=E target=E

        Result.Ok({ value })
        /// @type.node source="Result.Ok({ value })" type=Result.Ok<T#3, E#3>
        /// @type.node source=Result type=Result
        /// @type.node source=Result.Ok type=<T#2, E#2>({ value: T#2 }) => Result.Ok<T#2, E#2>
        /// @resolution.name source=Result target=Result
        /// @resolution.member source=Result.Ok receiver=Result type=<T#2, E#2>({ value: T#2 }) => Result.Ok<T#2, E#2> kind=symbol target_receiver=Result target=Result.Ok
        /// @resolution.construct source="Result.Ok({ value })" parameters=({ value: T#3 }) arguments=(provided({ value }) as { value: T#3 }) return=Result.Ok<T#3, E#3> kind=variant owner=Result variant=Ok instance="Result<T#3, E#3>" backing=Ok<T#3> argument={ value: T#3 } discriminant=Ok
        /// @generic.instance source="Result.Ok({ value })" id="Result<T#3, E#3>"
        /// @generic.instance source=Result.Ok id="Result<T#2, E#2>"
        /// @type.node source={ value } type={ value: T#3 }
        /// @type.node source=value type=T#3
        /// @resolution.name source=value target=ok.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=ok.value

    }
}

/// @generic.instance id="Result<T#2, E#2>" template=Result arguments=(T#2, E#2)
/// @generic.instance id="Result<T#3, E#3>" template=Result arguments=(T#3, E#3)
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
