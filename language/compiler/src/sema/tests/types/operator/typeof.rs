use crate::tests::{DirRows, TestSession};

/// Specialized borrowed functions retain their parameter and result references.
#[test]
fn test_typeof_specialized_borrowed_function() {
    let session = TestSession::single(
        r#"
declare function identity<T>(value: &readonly T): &readonly T;

declare const queried: typeof identity<int32>;

declare const value: &readonly int32;

const result = queried(value);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare function identity<T, 'a>(value: &readonly T): &readonly T;

declare const queried: typeof identity<int32>;

declare const value: &'static readonly int32;

const result: &'static readonly int32 = queried<"static">(value);

=== dir ===
declare function identity<T>(value: &readonly T): &readonly T;
/// @generic.template symbol=identity parameters=(T, 'a)
/// @type.symbol symbol=identity source="declare function identity<T>(value: &readonly T): &readonly T" type=<T, identity.'a>(&identity.'a readonly T) => &identity.'a readonly T
/// @type.symbol symbol=identity.T source=T type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

declare const queried: typeof identity<int32>;
/// @type.symbol symbol=queried source=queried type=<identity.'a>(&identity.'a readonly int32) => &identity.'a readonly int32
/// @resolution.pattern source=queried kind=binding target=queried
/// @resolution.name source=identity target=identity

declare const value: &readonly int32;
/// @type.symbol symbol=value source=value type=&'static readonly int32
/// @resolution.pattern source=value kind=binding target=value

const result = queried(value);
/// @type.symbol symbol=result source=result type=&'static readonly int32
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=queried target=queried
/// @resolution.call source=queried(value) parameters=(&'static readonly int32) arguments=(provided(value) as &'static readonly int32) return=&'static readonly int32 regions=("static" & "local") kind=expression target=expression generic_arguments=(int32, "static" & "local")
/// @resolution.place source=queried placement="local" lifetime="static" access="immutable"
/// @resolution.access source=queried root=queried
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
    );
}

/// A specialized typeof retains arguments and defaults across a forward class reference.
#[test]
fn test_typeof_specialized_constructor() {
    let session = TestSession::single(
        r#"
declare const create: typeof Box<int32>;

class Box<out T, out U = string> {}

const box = new create();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare const create: new () => Box<int32>;

class Box<out T, out U = string> {}

const box: Box<int32> = new create();

=== dir ===
declare const create: typeof Box<int32>;
/// @type.symbol symbol=create source=create type=Function<(), Box<int32, string>, "readonly">
/// @resolution.pattern source=create kind=binding target=create
/// @generic.instance id="Box<int32, string>" template=Box arguments=(int32, string)
/// @resolution.name source=Box target=Box

class Box<out T, out U = string> {}
/// @generic.template symbol=Box parameters=(out T, out U = string)
/// @type.symbol symbol=Box source="class Box<out T, out U = string> {}" type=typeof Box
/// @definition.class symbol=Box source="class Box<out T, out U = string> {}" template=(out T, out U = string)
/// @type.symbol symbol=Box.T source="out T" type=T
/// @type.symbol symbol=Box.U source="out U = string" type=U

const box = new create();
/// @type.symbol symbol=box source=box type=Box<int32, string>
/// @resolution.pattern source=box kind=binding target=box
/// @resolution.call source="new create()" parameters=() return=Box<int32, string> kind=expression target=expression generic_arguments=(int32, string)
/// @resolution.name source=create target=create
/// @resolution.place source=create placement="local" lifetime="static" access="immutable"
/// @resolution.access source=create root=create
"#,
    );
}

/// A value application and its typeof query have the same callable parameter and result types.
#[test]
fn test_typeof_specialized_function() {
    let session = TestSession::single(
        r#"
declare function identity<T>(value: T): T;

const direct = identity<int32>;

declare const queried: typeof identity<int32>;

const assigned: (value: int32) => int32 = direct;

const first = direct(1);

const second = queried(2);

const third = assigned(3);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare function identity<T>(value: T): T;

const direct: (value: int32) => int32 = identity<int32>;

declare const queried: (value: int32) => int32;

const assigned: (value: int32) => int32 = direct;

const first: int32 = direct(1);

const second: int32 = queried(2);

const third: int32 = assigned(3);

=== dir ===
declare function identity<T>(value: T): T;
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity source="declare function identity<T>(value: T): T" type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

const direct = identity<int32>;
/// @type.symbol symbol=direct source=direct type=Function<(int32,), int32, "readonly">
/// @resolution.pattern source=direct kind=binding target=direct
/// @resolution.name source=identity target=identity
/// @resolution.function source=identity type=Function<(T,), T, "readonly"> target=identity
/// @resolution.function source=identity<int32> type=Function<(int32,), int32, "readonly"> target=identity instance=identity<int32>
/// @generic.instantiation id=identity<int32> template=identity arguments=(int32)
/// @generic.instance id=identity<int32> template=identity arguments=(int32)

declare const queried: typeof identity<int32>;
/// @type.symbol symbol=queried source=queried type=(int32) => int32
/// @resolution.pattern source=queried kind=binding target=queried
/// @resolution.name source=identity target=identity

const assigned: (value: int32) => int32 = direct;
/// @type.symbol symbol=assigned source=assigned type=(int32) => int32
/// @resolution.pattern source=assigned kind=binding target=assigned
/// @type.symbol symbol=value source="value: int32" type=int32
/// @resolution.name source=direct target=direct
/// @resolution.place source=direct placement="local" lifetime="static" access="immutable"
/// @resolution.access source=direct root=direct

const first = direct(1);
/// @type.symbol symbol=first source=first type=int32
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.name source=direct target=direct
/// @resolution.call source=direct(1) parameters=(int32) arguments=(provided(1) as int32) return=int32 kind=expression target=expression generic_arguments=(int32)
/// @resolution.place source=direct placement="local" lifetime="static" access="immutable"
/// @resolution.access source=direct root=direct
/// @coercion.node source=1 from=1 adjustments=[{ kind: materialize, target: int32 }] origin=implicit

const second = queried(2);
/// @type.symbol symbol=second source=second type=int32
/// @resolution.pattern source=second kind=binding target=second
/// @resolution.name source=queried target=queried
/// @resolution.call source=queried(2) parameters=(int32) arguments=(provided(2) as int32) return=int32 kind=expression target=expression generic_arguments=(int32)
/// @resolution.place source=queried placement="local" lifetime="static" access="immutable"
/// @resolution.access source=queried root=queried
/// @coercion.node source=2 from=2 adjustments=[{ kind: materialize, target: int32 }] origin=implicit

const third = assigned(3);
/// @type.symbol symbol=third source=third type=int32
/// @resolution.pattern source=third kind=binding target=third
/// @resolution.name source=assigned target=assigned
/// @resolution.call source=assigned(3) parameters=(int32) arguments=(provided(3) as int32) return=int32 kind=expression target=expression
/// @resolution.place source=assigned placement="local" lifetime="static" access="immutable"
/// @resolution.access source=assigned root=assigned
/// @coercion.node source=3 from=3 adjustments=[{ kind: materialize, target: int32 }] origin=implicit
"#,
    );
}

/// Imported fields retain the type selected by a typeof annotation.
#[test]
fn test_read_imported_field_with_typeof() {
    let session = TestSession::builder()
        .module(
            "box.ds",
            r#"
declare const seed: int32;

export struct Box {
    value: typeof seed;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Box } from "./box.ds";

declare const box: Box;

const value = box.value;
"#,
        )
        .build();

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Box } from "./box.ds";

declare const box: Box;

const value: int32 = box.value;

=== dir ===
import { Box } from "./box.ds";

declare const box: Box;
/// @type.symbol symbol=box source=box type=box.Box
/// @resolution.pattern source=box kind=binding target=box
/// @resolution.name source=Box target=box.Box

const value = box.value;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=box target=box
/// @resolution.member source=box.value receiver=box.Box type=int32 kind=field target_receiver=box.Box key=value target=box.Box.value target_type=int32
/// @resolution.place source=box placement="local" lifetime="static" access="immutable"
/// @resolution.access source=box root=box
/// @resolution.access source=box.value root=box keys=[value]
"#,
    );
}

/// A typeof operand must name a value, even when its declaration is a type alias.
#[test]
fn test_query_type_alias_with_typeof() {
    let session = TestSession::single(
        r#"
type Count = int32;

type Counter = typeof Count;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Count = int32;

type Counter = typeof Count;

=== dir ===
type Count = int32;
/// @type.symbol symbol=Count source="type Count = int32" type=int32
/// @definition.type symbol=Count source="type Count = int32" value=int32

type Counter = typeof Count;
/// @type.symbol symbol=Counter source="type Counter = typeof Count" type=<error>
/// @definition.type symbol=Counter source="type Counter = typeof Count" value=<error>
/// @resolution.name source=Count target=Count
"#,
        r#"
/// @diagnostic.error id=invalid-value-reference message="'Count' is not a value"
/// @diagnostic.label line=4 column=23 span="Count" line_source="type Counter = typeof Count;"
"#,
    );
}

/// A typeof lifts the type of a local binding.
#[test]
fn test_typeof_lifts_local_value_type() {
    let session = TestSession::single(
        r#"
const value = 42;

type ValueType = typeof value;

let ok: ValueType = 42;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value: 42 = 42;

type ValueType = typeof value;

let ok: ValueType = 42;

=== dir ===
const value = 42;
/// @type.symbol symbol=value source=value type=42
/// @resolution.pattern source=value kind=binding target=value

type ValueType = typeof value;
/// @type.symbol symbol=ValueType source="type ValueType = typeof value" type=42
/// @definition.type symbol=ValueType source="type ValueType = typeof value" value=typeof value
/// @resolution.name source=value target=value

let ok: ValueType = 42;
/// @type.symbol symbol=ok source=ok type=ValueType
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=ValueType target=ValueType
"#,
    );
}

/// A value of another type reports a diagnostic against a typeof alias.
#[test]
fn test_typeof_rejects_incompatible_local_value() {
    let session = TestSession::single(
        r#"
const value = 42;

type ValueType = typeof value;

let bad: ValueType = "no";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value: 42 = 42;

type ValueType = typeof value;

let bad: ValueType = "no";

=== dir ===
const value = 42;
/// @type.symbol symbol=value source=value type=42
/// @resolution.pattern source=value kind=binding target=value

type ValueType = typeof value;
/// @type.symbol symbol=ValueType source="type ValueType = typeof value" type=42
/// @definition.type symbol=ValueType source="type ValueType = typeof value" value=typeof value
/// @resolution.name source=value target=value

let bad: ValueType = "no";
/// @type.symbol symbol=bad source=bad type=ValueType
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=ValueType target=ValueType
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"no\"' is not assignable to type 'ValueType'"
/// @diagnostic.label line=6 column=22 span="\"no\"" line_source="let bad: ValueType = \"no\";"
/// @diagnostic.related line=6 column=10 span="ValueType" line_source="let bad: ValueType = \"no\";" message="expected due to this annotation"
/// @diagnostic.note message="'ValueType' reduces to '42'"
"#,
    );
}

/// Class names provide constructor values and direct static member access.
#[test]
fn test_reference_class_constructor_and_static_members() {
    let session = TestSession::single(
        r#"
class Counter {
    static version: int32 = 0;
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

type CounterCtor = typeof Counter;

declare function takesCounter(ctor: { new (value: int32): Counter }): void;

takesCounter(Counter);
let version: int32 = Counter.version;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    static version: int32 = 0;
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

type CounterCtor = typeof Counter;

declare function takesCounter(ctor: { new (value: int32): Counter }): void;

takesCounter(Counter);
let version: int32 = Counter.version;

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.field symbol=Counter.version source="static version: int32 = 0" key=version static=true type=int32
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(this: &'managed Counter, int32) => Counter

    static version: int32 = 0;
    /// @type.symbol symbol=Counter.version source="static version: int32 = 0" type=int32

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    constructor(value: int32) {
    /// @type.symbol symbol=Counter.constructor type=(this: &'managed Counter, int32) => Counter
    /// @type.symbol symbol=Counter.constructor.this type=&'managed Counter
    /// @type.symbol symbol=Counter.constructor.value source="value: int32" type=int32

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&'managed Counter
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Counter, target=field(receiver=&'managed Counter, target=Counter.value, type=int32), type=int32" type=int32
        /// @resolution.name source=value target=Counter.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Counter.constructor.value

    }
}

type CounterCtor = typeof Counter;
/// @type.symbol symbol=CounterCtor source="type CounterCtor = typeof Counter" type=Function<(int32,), Counter, "readonly">
/// @definition.type symbol=CounterCtor source="type CounterCtor = typeof Counter" value=typeof Counter
/// @resolution.name source=Counter target=Counter

declare function takesCounter(ctor: { new (value: int32): Counter }): void;
/// @type.symbol symbol=takesCounter source="declare function takesCounter(ctor: { new (value: int32): Counter }): void" type=(new (int32) => Counter) => void
/// @type.symbol symbol=takesCounter.value source="value: int32" type=int32
/// @resolution.name source=Counter target=Counter

takesCounter(Counter);
/// @resolution.name source=takesCounter target=takesCounter
/// @resolution.call source=takesCounter(Counter) parameters=(new (int32) => Counter) arguments=(provided(Counter) as new (int32) => Counter) return=void kind=symbol target=takesCounter
/// @resolution.name source=Counter target=Counter
/// @resolution.function source=Counter type=Function<(int32,), Counter, "readonly"> target=Counter

let version: int32 = Counter.version;
/// @type.symbol symbol=version source=version type=int32
/// @resolution.pattern source=version kind=binding target=version
/// @resolution.name source=Counter target=Counter
/// @resolution.member source=Counter.version receiver=typeof Counter type=int32 kind=field target_receiver=typeof Counter key=version target=Counter.version target_type=int32
/// @resolution.place source=Counter.version placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=Counter.version root=Counter keys=[version]
"#,
    );
}
