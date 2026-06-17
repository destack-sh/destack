use crate::tests::{DirRows, TestSession};

#[test]
fn test_typeof_lifts_local_value_type() {
    let session = TestSession::single(
        r#"
const value = 42;

type ValueType = typeof value;

let ok: ValueType = 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value: 42 = 42;

type ValueType = typeof value;

let ok: ValueType = 42;

=== checked ===
const value = 42;
/// @type.symbol symbol=value type=42

type ValueType = typeof value;
/// @type.symbol symbol=ValueType source="type ValueType = typeof value" type=42
/// @definition.type symbol=ValueType source="type ValueType = typeof value" value=42
/// @resolution.name source=value target=value

let ok: ValueType = 42;
/// @type.symbol symbol=ok source=ok type=42
/// @resolution.name source=ValueType target=ValueType
"#,
    );
}

#[test]
fn test_typeof_rejects_incompatible_local_value() {
    let session = TestSession::single(
        r#"
const value = 42;

type ValueType = typeof value;

let bad: ValueType = "no";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value: 42 = 42;

type ValueType = typeof value;

let bad: ValueType = "no";

=== checked ===
const value = 42;
/// @type.symbol symbol=value type=42

type ValueType = typeof value;
/// @type.symbol symbol=ValueType source="type ValueType = typeof value" type=42
/// @definition.type symbol=ValueType source="type ValueType = typeof value" value=42
/// @resolution.name source=value target=value

let bad: ValueType = "no";
/// @type.symbol symbol=bad source=bad type=42
/// @resolution.name source=ValueType target=ValueType
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '\"no\"' is not assignable to type 'ValueType'"
/// @diagnostic.label line=6 column=5 source="let bad: ValueType = \"no\";"
"#,
    );
}

#[test]
fn test_typeof_class_includes_static_surface() {
    let session = TestSession::single(
        r#"
class Counter {
    static version: int32;
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

type CounterCtor = typeof Counter;

declare function takesCounter(ctor: { new (value: int32): Counter }): void;

takesCounter(Counter);
let version: CounterCtor["version"] = 1;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    static version: int32;
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

type CounterCtor = typeof Counter;

declare function takesCounter(ctor: { new (value: int32): Counter }): void;

takesCounter(Counter);
let version: CounterCtor["version"] = 1;

=== checked ===
class Counter {
/// @type.symbol symbol=Counter type=Counter

    static version: int32;
    /// @type.symbol symbol=Counter.version source="static version: int32" type=int32

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    constructor(value: int32) {
    /// @type.symbol symbol=Counter.constructor type=(int32) => Counter

        this.value = value;
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value
        /// @resolution.name source=value target=value
    }
}

type CounterCtor = typeof Counter;
/// @type.symbol symbol=CounterCtor source="type CounterCtor = typeof Counter" type=typeof Counter
/// @definition.type symbol=CounterCtor source="type CounterCtor = typeof Counter" value=typeof Counter
/// @resolution.name source=Counter target=Counter

declare function takesCounter(ctor: { new (value: int32): Counter }): void;
/// @type.symbol symbol=takesCounter type=({ new (int32) => Counter }) => void
/// @type.symbol symbol=ctor type={ new (int32) => Counter }
/// @resolution.name source=Counter target=Counter

takesCounter(Counter);
/// @resolution.name source=takesCounter target=takesCounter
/// @resolution.name source=Counter target=Counter
/// @resolution.call source=takesCounter(Counter) parameters=(typeof Counter) return=void kind=symbol target=takesCounter

let version: CounterCtor["version"] = 1;
/// @type.symbol symbol=version source=version type=int32
/// @resolution.name source=CounterCtor target=CounterCtor
"#,
    );
}
