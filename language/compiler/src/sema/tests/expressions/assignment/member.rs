use crate::tests::{DirRows, TestSession};

#[test]
fn test_readonly_member_rejects_assignment() {
    let session = TestSession::single(
        r#"
const state: { readonly count: int32 } = { count: 0 };
state.count = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const state: { readonly count: int32 } = { count: 0 };
state.count = 1;

=== dir ===
const state: { readonly count: int32 } = { count: 0 };
/// @type.symbol symbol=state source=state type={ readonly count: int32 }
/// @resolution.pattern source=state kind=binding target=state
/// @type.symbol symbol=count source="readonly count: int32" type=int32
/// @type.node source={ count: 0 } type={ readonly count: int32 }
/// @type.node source=0 type=0

state.count = 1;
/// @type.node source="state.count = 1" type=<error>
/// @type.node source=state type={ readonly count: int32 }
/// @resolution.name source=state target=state
/// @resolution.place source=state placement="local" lifetime="static" access="immutable"
/// @resolution.access source=state root=state
/// @resolution.rejected source=state.count
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

    session.assert_dir(
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

=== dir ===
interface Counter {
/// @generic.template symbol=Counter parameters=(this: Counter)
/// @type.symbol symbol=Counter type=Counter
/// @definition.interface symbol=Counter template=(this: Counter)
/// @definition.where symbol=Counter relation=satisfies left=this right=Counter
/// @definition.method symbol=Counter.current#1 source="get current(): int32" slot=current role=getter type=() => int32
/// @definition.method symbol=Counter.current#2 source="set current(next: int32)" slot=current role=setter type=(int32) => void

    get current(): int32;
    /// @type.symbol symbol=Counter.current#1 source="get current(): int32" type=() => int32

    set current(next: int32);
    /// @type.symbol symbol=Counter.current#2 source="set current(next: int32)" type=(int32) => void
    /// @type.symbol symbol=Counter.current.next source="next: int32" type=int32

}

declare let counter: Counter;
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=Counter target=Counter

counter.current = 2;
/// @resolution.name source=counter target=counter
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter
/// @resolution.pattern.assign source=counter.current kind=place
/// @resolution.assignment source=counter.current write="receiver=Counter, target=dynamic(Counter as Counter, Counter.current#2)(parameters=(int32), arguments=(supplied(0) as int32), return=void), type=int32" type=int32

counter.current++;
/// @resolution.name source=counter target=counter
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter
/// @resolution.assignment source=counter.current read="receiver=Counter, target=dynamic(Counter as Counter, Counter.current#1)(parameters=(), arguments=(), return=int32), type=int32" write="receiver=Counter, target=dynamic(Counter as Counter, Counter.current#2)(parameters=(int32), arguments=(supplied(0) as int32), return=void), type=int32" type=int32
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Counter {
    get current(): int32;
    set current(next: int32);
}

declare let counter: Counter;
counter["current"] = 2;
counter["current"]++;

=== dir ===
interface Counter {
/// @generic.template symbol=Counter parameters=(this: Counter)
/// @type.symbol symbol=Counter type=Counter
/// @definition.interface symbol=Counter template=(this: Counter)
/// @definition.where symbol=Counter relation=satisfies left=this right=Counter
/// @definition.method symbol=Counter.current#1 source="get current(): int32" slot=current role=getter type=() => int32
/// @definition.method symbol=Counter.current#2 source="set current(next: int32)" slot=current role=setter type=(int32) => void

    get current(): int32;
    /// @type.symbol symbol=Counter.current#1 source="get current(): int32" type=() => int32

    set current(next: int32);
    /// @type.symbol symbol=Counter.current#2 source="set current(next: int32)" type=(int32) => void
    /// @type.symbol symbol=Counter.current.next source="next: int32" type=int32

}

declare let counter: Counter;
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=Counter target=Counter

counter["current"] = 2;
/// @resolution.name source=counter target=counter
/// @resolution.pattern.assign source="counter[\"current\"]" kind=place
/// @resolution.assignment source="counter[\"current\"]" write="member(receiver=Counter, target=dynamic(Counter as Counter, Counter.current#2)(parameters=(int32), arguments=(supplied(0) as int32), return=void), type=int32)" type=int32
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter

counter["current"]++;
/// @resolution.name source=counter target=counter
/// @resolution.assignment source="counter[\"current\"]" read="member(receiver=Counter, target=dynamic(Counter as Counter, Counter.current#1)(parameters=(), arguments=(), return=int32), type=int32)" write="member(receiver=Counter, target=dynamic(Counter as Counter, Counter.current#2)(parameters=(int32), arguments=(supplied(0) as int32), return=void), type=int32)" type=int32
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Counter {
    get current(): int32;
}

declare const counter: Counter;
const current: int32 = counter.current;

=== dir ===
interface Counter {
/// @generic.template symbol=Counter parameters=(this: Counter)
/// @type.symbol symbol=Counter type=Counter
/// @definition.interface symbol=Counter template=(this: Counter)
/// @definition.where symbol=Counter relation=satisfies left=this right=Counter
/// @definition.method symbol=Counter.current source="get current(): int32" slot=current role=getter type=() => int32

    get current(): int32;
    /// @type.symbol symbol=Counter.current source="get current(): int32" type=() => int32

}

declare const counter: Counter;
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=Counter target=Counter

const current = counter.current;
/// @type.symbol symbol=current source=current type=int32
/// @resolution.pattern source=current kind=binding target=current
/// @resolution.name source=counter target=counter
/// @resolution.member source=counter.current receiver=Counter type=int32 kind=call target="dynamic(Counter as Counter, Counter.current)(parameters=(), arguments=(), return=int32)"
/// @resolution.place source=counter placement="local" lifetime="static" access="immutable"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Sink {
    set value(next: int32);
}

declare let sink: Sink;
sink.value = 1;

=== dir ===
interface Sink {
/// @generic.template symbol=Sink parameters=(this: Sink)
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink template=(this: Sink)
/// @definition.where symbol=Sink relation=satisfies left=this right=Sink
/// @definition.method symbol=Sink.value source="set value(next: int32)" slot=value role=setter type=(int32) => void

    set value(next: int32);
    /// @type.symbol symbol=Sink.value source="set value(next: int32)" type=(int32) => void
    /// @type.symbol symbol=Sink.value.next source="next: int32" type=int32

}

declare let sink: Sink;
/// @type.symbol symbol=sink source=sink type=Sink
/// @resolution.pattern source=sink kind=binding target=sink
/// @resolution.name source=Sink target=Sink

sink.value = 1;
/// @resolution.name source=sink target=sink
/// @resolution.place source=sink placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=sink root=sink
/// @resolution.pattern.assign source=sink.value kind=place
/// @resolution.assignment source=sink.value write="receiver=Sink, target=dynamic(Sink as Sink, Sink.value)(parameters=(int32), arguments=(supplied(0) as int32), return=void), type=int32" type=int32
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Counter {
    get current(): int32;
}

declare let counter: Counter;
counter.current = 1;

=== dir ===
interface Counter {
/// @generic.template symbol=Counter parameters=(this: Counter)
/// @type.symbol symbol=Counter type=Counter
/// @definition.interface symbol=Counter template=(this: Counter)
/// @definition.where symbol=Counter relation=satisfies left=this right=Counter
/// @definition.method symbol=Counter.current source="get current(): int32" slot=current role=getter type=() => int32

    get current(): int32;
    /// @type.symbol symbol=Counter.current source="get current(): int32" type=() => int32

}

declare let counter: Counter;
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=Counter target=Counter

counter.current = 1;
/// @resolution.name source=counter target=counter
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter
/// @resolution.rejected source=counter.current
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Sink {
    set value(next: int32);
}

declare const sink: Sink;
const value = sink.value;

=== dir ===
interface Sink {
/// @generic.template symbol=Sink parameters=(this: Sink)
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink template=(this: Sink)
/// @definition.where symbol=Sink relation=satisfies left=this right=Sink
/// @definition.method symbol=Sink.value source="set value(next: int32)" slot=value role=setter type=(int32) => void

    set value(next: int32);
    /// @type.symbol symbol=Sink.value source="set value(next: int32)" type=(int32) => void
    /// @type.symbol symbol=Sink.value.next source="next: int32" type=int32

}

declare const sink: Sink;
/// @type.symbol symbol=sink source=sink type=Sink
/// @resolution.pattern source=sink kind=binding target=sink
/// @resolution.name source=Sink target=Sink

const value = sink.value;
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=sink target=sink
/// @resolution.place source=sink placement="local" lifetime="static" access="immutable"
/// @resolution.access source=sink root=sink
/// @resolution.rejected source=sink.value
"#,
        r#"
/// @diagnostic.error id=cannot-read-write-only-member message="member 'value' is write-only"
/// @diagnostic.label line=7 column=20 span="value" line_source="const value = sink.value;"
"#,
    );
}

/// Destructured assignment fields record the projected source access.
#[test]
fn test_destructured_field_records_source_access() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

function retain(point: Point): void {
    ({ x: point.x } = point);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

function retain(point: Point): void {
    ({ x: point.x } = point);
}

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

function retain(point: Point): void {
/// @type.symbol symbol=retain type=(Point) => void
/// @type.symbol symbol=retain.point source="point: Point" type=Point
/// @resolution.name source=Point target=Point

    ({ x: point.x } = point);
    /// @resolution.pattern.assign source={ x: point.x } kind=object fields={ Point.x: point.x }
    /// @resolution.access source={ x: point.x } root=retain.point
    /// @resolution.access source="x: point.x" root=retain.point keys=[x]
    /// @resolution.name source=point target=retain.point
    /// @resolution.place source=point placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=point root=retain.point
    /// @resolution.pattern.assign source=point.x kind=place
    /// @resolution.place source=point.x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=point.x root=retain.point keys=[x]
    /// @resolution.assignment source=point.x write="receiver=Point, target=field(receiver=Point, target=Point.x, type=int32), type=int32" type=int32
    /// @resolution.name source=point target=retain.point
    /// @resolution.place source=point placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=point root=retain.point

}
"#,
        r#"
"#,
    );
}

/// A compound assignment reads its own target as the operand inside a loop.
#[test]
fn test_grow_a_string_by_itself_inside_a_range_loop() {
    let session = TestSession::single(
        r#"
function double(depth: isize): string {
    let output = "x";
    for (const _ of 0..depth) {
        output += output;
    }

    return output;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function double(depth: isize): string {
    let output: string = "x";
    for (const _ of 0..depth) {
        (output += output) as string;
    }

    return output;
}

=== dir ===
function double(depth: isize): string {
/// @type.symbol symbol=double type=(isize) => string
/// @type.symbol symbol=double.depth source="depth: isize" type=isize

    let output = "x";
    /// @type.symbol symbol=double.output source=output type=string
    /// @resolution.pattern source=output kind=binding target=double.output

    for (const _ of 0..depth) {
    /// @resolution.iteration iterator="iterator#1(parameters=(), arguments=(), return=RangeIterator<isize>)" next="next(parameters=(), arguments=(), return=IteratorResult<isize, void>, regions=(\"frame\" & \"local\"))"
    /// @generic.instantiation id="next<isize, \"frame\" & \"local\">" template=next arguments=(isize, "frame" & "local")
    /// @generic.instantiation id=iterator#1<isize> template=iterator#1 arguments=(isize)
    /// @resolution.pattern source=_ kind=wildcard
    /// @resolution.name source=depth target=double.depth
    /// @resolution.place source=depth placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=depth root=double.depth

        output += output;
        /// @resolution.name source=output target=double.output
        /// @resolution.operator source="output += output" type=^string operator="+" kind=call parameters=(string) arguments=(provided(output) as string) return=^string regions=("managed" & "local") kind=symbol target=add receiver=string adjustments=(borrow(&'managed readonly string)) instance="string.<extension#2>.add<\"managed\" & \"local\">"
        /// @resolution.pattern.assign source=output kind=place
        /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
        /// @resolution.assignment source=output read=binding(double.output) write=binding(double.output) type=string
        /// @resolution.access source=output root=double.output
        /// @generic.instantiation id="add<\"managed\" & \"local\">" template=add arguments=("managed" & "local")
        /// @resolution.name source=output target=double.output
        /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=output root=double.output

    }

    return output;
    /// @resolution.name source=output target=double.output
    /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=output root=double.output

}
"#,
        r#"
"#,
    );
}
