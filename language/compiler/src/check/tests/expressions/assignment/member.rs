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
/// @resolution.pattern source=state kind=binding target=state
/// @type.node source={ count: 0 } type={ readonly count: int32 }
/// @type.node source=0 type=0

state.count = 1;
/// @type.node source="state.count = 1" type=1
/// @type.node source=state type={ readonly count: int32 }
/// @type.node source=state.count type=int32
/// @resolution.name source=state target=state
/// @resolution.place source=state placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=state root=state
/// @resolution.pattern.assign source=state.count kind=place
/// @resolution.assignment source=state.count write="receiver={ readonly count: int32 }, target=field(receiver={ readonly count: int32 }, target=count, type=int32), type=int32" type=int32
/// @type.node source=1 type=1

/// @check.stats.solve variables=1 types=9 constraints=0 obligations=2 solutions=1 bounds=0 decisions=4
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'count'"
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

declare let counter: Dynamic<Counter>;
counter.current = 2;
counter.current++;

=== checked ===
interface Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.interface symbol=Counter
/// @definition.method symbol=Counter.current#1 source="get current(): int32" slot=current role=getter type=(this: this) => int32
/// @definition.method symbol=Counter.current#2 source="set current(next: int32)" slot=current role=setter type=(this: this, int32) => void

    get current(): int32;
    /// @type.symbol symbol=Counter.current#1 source="get current(): int32" type=(this: this) => int32

    set current(next: int32);
    /// @type.symbol symbol=Counter.current#2 source="set current(next: int32)" type=(this: this, int32) => void
    /// @type.symbol symbol=Counter.current.next source="next: int32" type=int32

}

declare let counter: Counter;
/// @type.symbol symbol=counter source=counter type=Dynamic<Counter>
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=Counter target=Counter

counter.current = 2;
/// @resolution.name source=counter target=counter
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter
/// @resolution.pattern.assign source=counter.current kind=place
/// @resolution.assignment source=counter.current write="receiver=Dynamic<Counter>, target=dynamic(Dynamic<Counter> as Counter, Counter.current#2)(parameters=(int32), arguments=(write as int32), return=void), type=int32" type=int32

counter.current++;
/// @resolution.name source=counter target=counter
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter
/// @resolution.assignment source=counter.current read="receiver=Dynamic<Counter>, target=dynamic(Dynamic<Counter> as Counter, Counter.current#1)(parameters=(), arguments=(), return=int32), type=int32" write="receiver=Dynamic<Counter>, target=dynamic(Dynamic<Counter> as Counter, Counter.current#2)(parameters=(int32), arguments=(write as int32), return=void), type=int32" type=int32
/// @resolution.operator source=counter.current++ type=int32 operator="++" kind=builtin operands=[counter.current as int32 families=(integer)]
"#,
    );
}

#[test]
fn test_static_subscript_records_property_place() {
    let session = TestSession::single(
        r#"
interface Counter {
    get current(): int32;
    set current(next: int32);
}

declare let counter: Counter;
counter["current"] = 2;
counter["current"]++;
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

declare let counter: Dynamic<Counter>;
counter["current"] = 2;
counter["current"]++;

=== checked ===
interface Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.interface symbol=Counter
/// @definition.method symbol=Counter.current#1 source="get current(): int32" slot=current role=getter type=(this: this) => int32
/// @definition.method symbol=Counter.current#2 source="set current(next: int32)" slot=current role=setter type=(this: this, int32) => void

    get current(): int32;
    /// @type.symbol symbol=Counter.current#1 source="get current(): int32" type=(this: this) => int32

    set current(next: int32);
    /// @type.symbol symbol=Counter.current#2 source="set current(next: int32)" type=(this: this, int32) => void
    /// @type.symbol symbol=Counter.current.next source="next: int32" type=int32

}

declare let counter: Counter;
/// @type.symbol symbol=counter source=counter type=Dynamic<Counter>
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=Counter target=Counter

counter["current"] = 2;
/// @resolution.name source=counter target=counter
/// @resolution.pattern.assign source="counter[\"current\"]" kind=place
/// @resolution.assignment source="counter[\"current\"]" write="member(receiver=Dynamic<Counter>, target=dynamic(Dynamic<Counter> as Counter, Counter.current#2)(parameters=(int32), arguments=(write as int32), return=void), type=int32)" type=int32
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter

counter["current"]++;
/// @resolution.name source=counter target=counter
/// @resolution.assignment source="counter[\"current\"]" read="member(receiver=Dynamic<Counter>, target=dynamic(Dynamic<Counter> as Counter, Counter.current#1)(parameters=(), arguments=(), return=int32), type=int32)" write="member(receiver=Dynamic<Counter>, target=dynamic(Dynamic<Counter> as Counter, Counter.current#2)(parameters=(int32), arguments=(write as int32), return=void), type=int32)" type=int32
/// @resolution.operator source="counter[\"current\"]++" type=int32 operator="++" kind=builtin operands=[counter["current"] as int32 families=(integer)]
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter
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

declare const counter: Dynamic<Counter>;
const current: int32 = counter.current;

=== checked ===
interface Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.interface symbol=Counter
/// @definition.method symbol=Counter.current source="get current(): int32" slot=current role=getter type=(this: this) => int32

    get current(): int32;
    /// @type.symbol symbol=Counter.current source="get current(): int32" type=(this: this) => int32

}

declare const counter: Counter;
/// @type.symbol symbol=counter source=counter type=Dynamic<Counter>
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=Counter target=Counter

const current = counter.current;
/// @type.symbol symbol=current source=current type=int32
/// @resolution.pattern source=current kind=binding target=current
/// @resolution.name source=counter target=counter
/// @resolution.member source=counter.current receiver=Dynamic<Counter> type=int32 kind=call target="dynamic(Dynamic<Counter> as Counter, Counter.current)(parameters=(), arguments=(), return=int32)"
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter
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

declare let sink: Dynamic<Sink>;
sink.value = 1;

=== checked ===
interface Sink {
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink
/// @definition.method symbol=Sink.value source="set value(next: int32)" slot=value role=setter type=(this: this, int32) => void

    set value(next: int32);
    /// @type.symbol symbol=Sink.value source="set value(next: int32)" type=(this: this, int32) => void
    /// @type.symbol symbol=Sink.value.next source="next: int32" type=int32

}

declare let sink: Sink;
/// @type.symbol symbol=sink source=sink type=Dynamic<Sink>
/// @resolution.pattern source=sink kind=binding target=sink
/// @resolution.name source=Sink target=Sink

sink.value = 1;
/// @resolution.name source=sink target=sink
/// @resolution.place source=sink placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=sink root=sink
/// @resolution.pattern.assign source=sink.value kind=place
/// @resolution.assignment source=sink.value write="receiver=Dynamic<Sink>, target=dynamic(Dynamic<Sink> as Sink, Sink.value)(parameters=(int32), arguments=(write as int32), return=void), type=int32" type=int32
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

declare let counter: Dynamic<Counter>;
counter.current = 1;

=== checked ===
interface Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.interface symbol=Counter
/// @definition.method symbol=Counter.current source="get current(): int32" slot=current role=getter type=(this: this) => int32

    get current(): int32;
    /// @type.symbol symbol=Counter.current source="get current(): int32" type=(this: this) => int32

}

declare let counter: Counter;
/// @type.symbol symbol=counter source=counter type=Dynamic<Counter>
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=Counter target=Counter

counter.current = 1;
/// @resolution.name source=counter target=counter
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'current'"
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

declare const sink: Dynamic<Sink>;
const value = sink.value;

=== checked ===
interface Sink {
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink
/// @definition.method symbol=Sink.value source="set value(next: int32)" slot=value role=setter type=(this: this, int32) => void

    set value(next: int32);
    /// @type.symbol symbol=Sink.value source="set value(next: int32)" type=(this: this, int32) => void
    /// @type.symbol symbol=Sink.value.next source="next: int32" type=int32

}

declare const sink: Sink;
/// @type.symbol symbol=sink source=sink type=Dynamic<Sink>
/// @resolution.pattern source=sink kind=binding target=sink
/// @resolution.name source=Sink target=Sink

const value = sink.value;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=sink target=sink
/// @resolution.place source=sink placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=sink root=sink
"#,
        r#"
/// @diagnostic.error id=cannot-read-write-only-member message="member 'value' is write-only"
/// @diagnostic.label line=7 column=20 span="value" line_source="const value = sink.value;"
"#,
    );
}
