use crate::tests::{DirRows, TestSession};

#[test]
fn test_readonly_member_rejects_assignment() {
    let session = TestSession::single(
        r#"
const state: { readonly count: int32 } = { count: 0 };
state.count = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const state: { readonly count: int32 } = { count: 0 };
state.count = 1;

=== checked ===
const state: { readonly count: int32 } = { count: 0 };
/// @type.symbol symbol=state source=state type={ readonly count: int32 }
/// @type.node source={ count: 0 } type={ count: 0 }
/// @type.node source=0 type=0

state.count = 1;
/// @type.node source="state.count = 1" type=1
/// @type.node source=state type={ readonly count: int32 }
/// @type.node source=state.count type=int32
/// @resolution.name source=state target=state
/// @resolution.pattern.assign source=state.count kind=place place=field(count) type=int32
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=6 constraints=0 obligations=1 solutions=0 bounds=0 decisions=2
"#,
        r#"
/// @diagnostic.error code=EC214 message="cannot assign to readonly member 'count'"
/// @diagnostic.label line=3 column=7 span="count" line_source="state.count = 1;"
"#,
    );
}

#[test]
fn test_accessor_member_records_property_place() {
    let session = TestSession::single(
        r#"
interface Counter {
    get current(): int32;
    set current(next: int32);
}

declare let counter: Counter;
counter.current = 2;
counter.current++;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Counter {
    get current(): int32;
    set current(next: int32);
}

declare let counter: Counter;
counter.current = 2;
counter.current++;

=== checked ===
interface Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.interface symbol=Counter
/// @definition.method symbol=Counter.current#1 source="get current(): int32" slot=current role=getter type=(this: Counter) => int32
/// @definition.method symbol=Counter.current#2 source="set current(next: int32)" slot=current role=setter type=(this: Counter, int32) => void

    get current(): int32;
    /// @type.symbol symbol=Counter.current#1 source="get current(): int32" type=(this: Counter) => int32

    set current(next: int32);
    /// @type.symbol symbol=Counter.current#2 source="set current(next: int32)" type=(this: Counter, int32) => void
    /// @type.symbol symbol=Counter.current.next source="next: int32" type=int32

}

declare let counter: Counter;
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.name source=Counter target=Counter

counter.current = 2;
/// @resolution.name source=counter target=counter
/// @resolution.pattern.assign source=counter.current kind=place place=property(setter(Counter.current#2)) type=int32

counter.current++;
/// @resolution.name source=counter target=counter
/// @resolution.place source=counter.current place="property(getter(Counter.current#1), setter(Counter.current#2))" type=int32
"#,
    );
}

#[test]
fn test_getter_member_records_property_read() {
    let session = TestSession::single(
        r#"
interface Counter {
    get current(): int32;
}

declare const counter: Counter;
const current = counter.current;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Counter {
    get current(): int32;
}

declare const counter: Counter;
const current: int32 = counter.current;

=== checked ===
interface Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.interface symbol=Counter
/// @definition.method symbol=Counter.current source="get current(): int32" slot=current role=getter type=(this: Counter) => int32

    get current(): int32;
    /// @type.symbol symbol=Counter.current source="get current(): int32" type=(this: Counter) => int32

}

declare const counter: Counter;
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.name source=Counter target=Counter

const current = counter.current;
/// @type.symbol symbol=current source=current type=int32
/// @resolution.name source=counter target=counter
/// @resolution.member source=counter.current receiver=Counter kind=symbol target=Counter.current
"#,
    );
}

#[test]
fn test_setter_member_records_property_write() {
    let session = TestSession::single(
        r#"
interface Sink {
    set value(next: int32);
}

declare let sink: Sink;
sink.value = 1;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Sink {
    set value(next: int32);
}

declare let sink: Sink;
sink.value = 1;

=== checked ===
interface Sink {
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink
/// @definition.method symbol=Sink.value source="set value(next: int32)" slot=value role=setter type=(this: Sink, int32) => void

    set value(next: int32);
    /// @type.symbol symbol=Sink.value source="set value(next: int32)" type=(this: Sink, int32) => void
    /// @type.symbol symbol=Sink.value.next source="next: int32" type=int32

}

declare let sink: Sink;
/// @type.symbol symbol=sink source=sink type=Sink
/// @resolution.name source=Sink target=Sink

sink.value = 1;
/// @resolution.name source=sink target=sink
/// @resolution.pattern.assign source=sink.value kind=place place=property(setter(Sink.value)) type=int32
"#,
    );
}

#[test]
fn test_getter_member_rejects_write() {
    let session = TestSession::single(
        r#"
interface Counter {
    get current(): int32;
}

declare let counter: Counter;
counter.current = 1;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Counter {
    get current(): int32;
}

declare let counter: Counter;
counter.current = 1;

=== checked ===
interface Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.interface symbol=Counter
/// @definition.method symbol=Counter.current source="get current(): int32" slot=current role=getter type=(this: Counter) => int32

    get current(): int32;
    /// @type.symbol symbol=Counter.current source="get current(): int32" type=(this: Counter) => int32

}

declare let counter: Counter;
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.name source=Counter target=Counter

counter.current = 1;
/// @resolution.name source=counter target=counter
"#,
        r#"
/// @diagnostic.error code=EC214 message="cannot assign to readonly member 'current'"
/// @diagnostic.label line=7 column=9 span="current" line_source="counter.current = 1;"
"#,
    );
}

#[test]
fn test_setter_member_rejects_read() {
    let session = TestSession::single(
        r#"
interface Sink {
    set value(next: int32);
}

declare const sink: Sink;
const value = sink.value;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Sink {
    set value(next: int32);
}

declare const sink: Sink;
const value = sink.value;

=== checked ===
interface Sink {
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink
/// @definition.method symbol=Sink.value source="set value(next: int32)" slot=value role=setter type=(this: Sink, int32) => void

    set value(next: int32);
    /// @type.symbol symbol=Sink.value source="set value(next: int32)" type=(this: Sink, int32) => void
    /// @type.symbol symbol=Sink.value.next source="next: int32" type=int32

}

declare const sink: Sink;
/// @type.symbol symbol=sink source=sink type=Sink
/// @resolution.name source=Sink target=Sink

const value = sink.value;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.name source=sink target=sink
"#,
        r#"
/// @diagnostic.error code=EC321 message="member 'value' is write-only"
/// @diagnostic.label line=7 column=20 span="value" line_source="const value = sink.value;"
"#,
    );
}
