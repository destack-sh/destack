use crate::tests::{DirRows, TestSession};

#[test]
fn test_keep_instance_signatures_dependent_on_this() {
    let session = TestSession::single(
        r#"
class Counter {
    value: int32 = 0;

    read(this): int32 {
        return this.value;
    }

    peek(readonly this): int32 {
        return this.value;
    }

    borrow(&this): int32 {
        return this.value;
    }

    inspect(&readonly this): int32 {
        return this.value;
    }

    increment(&exclusive this): void {
        this.value = this.value + 1;
    }

    static zero(): Counter {
        return new Counter();
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    value: int32 = 0;

    read(this): int32 {
        return this.value;
    }

    peek(readonly this): int32 {
        return this.value;
    }

    borrow(&this): int32 {
        return this.value;
    }

    inspect(&readonly this): int32 {
        return this.value;
    }

    increment(&exclusive this): void {
        this.value = this.value + 1;
    }

    static zero(): Counter {
        return new Counter();
    }
}

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32 = 0" key=value type=int32
/// @definition.method symbol=Counter.borrow slot=borrow type=<Counter.borrow.'a>(this: &Counter.borrow.'a Counter) => int32
/// @definition.method symbol=Counter.increment slot=increment type=<Counter.increment.'a>(this: &Counter.increment.'a exclusive Counter) => void
/// @definition.method symbol=Counter.inspect slot=inspect type=<Counter.inspect.'a>(this: &Counter.inspect.'a readonly Counter) => int32
/// @definition.method symbol=Counter.peek slot=peek type=(this: Readonly<Counter>) => int32
/// @definition.method symbol=Counter.read slot=read type=(this: Counter) => int32
/// @definition.method symbol=Counter.zero slot=zero static=true type=() => Counter

    value: int32 = 0;
    /// @type.symbol symbol=Counter.value source="value: int32 = 0" type=int32

    read(this): int32 {
    /// @type.symbol symbol=Counter.read type=(this: Counter) => int32
    /// @type.symbol symbol=Counter.read.this source=this type=this

        return this.value;
        /// @resolution.member source=this.value receiver=Counter type=int32 kind=field target_receiver=Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.value root=this keys=[value]

    }

    peek(readonly this): int32 {
    /// @type.symbol symbol=Counter.peek type=(this: Readonly<Counter>) => int32
    /// @type.symbol symbol=Counter.peek.this source="readonly this" type=Readonly<this>

        return this.value;
        /// @resolution.member source=this.value receiver=Readonly<Counter> type=int32 kind=field target_receiver=Readonly<Counter> key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=Readonly<Counter>
        /// @resolution.place source=this placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }

    borrow(&this): int32 {
    /// @generic.template symbol=Counter.borrow parameters=('a)
    /// @type.symbol symbol=Counter.borrow type=<Counter.borrow.'a>(this: &Counter.borrow.'a Counter) => int32
    /// @type.symbol symbol=Counter.borrow.this source=&this type=&Counter.borrow.'a this

        return this.value;
        /// @resolution.member source=this.value receiver=&Counter.borrow.'a Counter type=int32 kind=field target_receiver=&Counter.borrow.'a Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.borrow.'a Counter
        /// @resolution.place source=this placement="local" lifetime=Counter.borrow.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=Counter.borrow.'a access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]

    }

    inspect(&readonly this): int32 {
    /// @generic.template symbol=Counter.inspect parameters=('a)
    /// @type.symbol symbol=Counter.inspect type=<Counter.inspect.'a>(this: &Counter.inspect.'a readonly Counter) => int32
    /// @type.symbol symbol=Counter.inspect.this source="&readonly this" type=&Counter.inspect.'a readonly this

        return this.value;
        /// @resolution.member source=this.value receiver=&Counter.inspect.'a readonly Counter type=int32 kind=field target_receiver=&Counter.inspect.'a readonly Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.inspect.'a readonly Counter
        /// @resolution.place source=this placement="local" lifetime=Counter.inspect.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=Counter.inspect.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }

    increment(&exclusive this): void {
    /// @generic.template symbol=Counter.increment parameters=('a)
    /// @type.symbol symbol=Counter.increment type=<Counter.increment.'a>(this: &Counter.increment.'a exclusive Counter) => void
    /// @type.symbol symbol=Counter.increment.this source="&exclusive this" type=&Counter.increment.'a exclusive this

        this.value = this.value + 1;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a exclusive Counter
        /// @resolution.place source=this placement="local" lifetime=Counter.increment.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&Counter.increment.'a exclusive Counter, target=field(receiver=&Counter.increment.'a exclusive Counter, target=Counter.value, type=int32), type=int32" type=int32
        /// @resolution.member source=this.value receiver=&Counter.increment.'a exclusive Counter type=int32 kind=field target_receiver=&Counter.increment.'a exclusive Counter key=value target=Counter.value target_type=int32
        /// @resolution.operator source="this.value + 1" type=int32 operator="+" kind=builtin operands=[this.value as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a exclusive Counter
        /// @resolution.place source=this placement="local" lifetime=Counter.increment.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=Counter.increment.'a access="exclusive"
        /// @resolution.access source=this.value root=this keys=[value]

    }

    static zero(): Counter {
    /// @type.symbol symbol=Counter.zero type=() => Counter
    /// @resolution.name source=Counter target=Counter

        return new Counter();
        /// @resolution.construct source="new Counter()" parameters=() return=Counter kind=class target=Counter constructor=default
        /// @resolution.name source=Counter target=Counter

    }
}
"#,
    );
}

#[test]
fn test_resolve_relative_method_parameters_from_receiver() {
    let session = TestSession::single(
        r#"
class Message {}

newtype interface Sink {
    write(&readonly this, value: Message): Message;
}

declare const localSink: local Sink;
declare const sharedSink: shared Sink;
declare const localMessage: local Message;
declare const sharedMessage: shared Message;

localSink.write(localMessage) satisfies local Message;
sharedSink.write(sharedMessage) satisfies shared Message;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Message {}

newtype interface Sink {
    write(&readonly this, value: Message): Message;
}

declare const localSink: local Dynamic<Sink>;
declare const sharedSink: shared Dynamic<Sink>;
declare const localMessage: local Message;
declare const sharedMessage: shared Message;

localSink.write(localMessage) satisfies local Message;
sharedSink.write(sharedMessage) satisfies shared Message;

=== dir ===
class Message {}
/// @type.symbol symbol=Message source="class Message {}" type=Message
/// @definition.class symbol=Message source="class Message {}"

newtype interface Sink {
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink nominal=true
/// @definition.method symbol=Sink.write source="write(&readonly this, value: Message): Message" slot=write type=<Sink.write.'a>(this: &Sink.write.'a readonly Sink, Message) => Message

    write(&readonly this, value: Message): Message;
    /// @generic.template symbol=Sink.write parent=template#0 parameters=('a)
    /// @type.symbol symbol=Sink.write source="write(&readonly this, value: Message): Message" type=<Sink.write.'a>(this: &Sink.write.'a readonly Sink, Message) => Message
    /// @type.symbol symbol=Sink.write.this source="&readonly this" type=&Sink.write.'a readonly this
    /// @type.symbol symbol=Sink.write.value source="value: Message" type=Message
    /// @resolution.name source=Message target=Message
    /// @resolution.name source=Message target=Message

}

declare const localSink: local Sink;
/// @type.symbol symbol=localSink source=localSink type=Placed<Dynamic<Sink>, "local">
/// @resolution.pattern source=localSink kind=binding target=localSink
/// @resolution.name source=Sink target=Sink

declare const sharedSink: shared Sink;
/// @type.symbol symbol=sharedSink source=sharedSink type=Placed<Dynamic<Sink>, "shared">
/// @resolution.pattern source=sharedSink kind=binding target=sharedSink
/// @resolution.name source=Sink target=Sink

declare const localMessage: local Message;
/// @type.symbol symbol=localMessage source=localMessage type=Placed<Message, "local">
/// @resolution.pattern source=localMessage kind=binding target=localMessage
/// @resolution.name source=Message target=Message

declare const sharedMessage: shared Message;
/// @type.symbol symbol=sharedMessage source=sharedMessage type=Placed<Message, "shared">
/// @resolution.pattern source=sharedMessage kind=binding target=sharedMessage
/// @resolution.name source=Message target=Message

localSink.write(localMessage) satisfies local Message;
/// @resolution.name source=localSink target=localSink
/// @resolution.member source=localSink.write receiver=Placed<Dynamic<Sink>, "local"> type=<Sink.write.'a>(this: &Sink.write.'a readonly Sink, Message) => Message kind=symbol target_receiver=Placed<Dynamic<Sink>, "local"> dispatch=dynamic constraint=Sink target=Sink.write
/// @resolution.call source=localSink.write(localMessage) parameters=(Message) arguments=(provided(localMessage) as Message) return=Message kind=dynamic target=Sink.write receiver=Placed<Dynamic<Sink>, "local"> constraint=Sink adjustments=(Placed<Dynamic<Sink>, "local"> => direct -> Dynamic<Sink>, borrow(&'static readonly Dynamic<Sink>))
/// @resolution.place source=localSink placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=localSink root=localSink
/// @resolution.name source=localMessage target=localMessage
/// @resolution.place source=localMessage placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=localMessage root=localMessage
/// @resolution.name source=Message target=Message

sharedSink.write(sharedMessage) satisfies shared Message;
/// @resolution.name source=sharedSink target=sharedSink
/// @resolution.member source=sharedSink.write receiver=Placed<Dynamic<Sink>, "shared"> type=<Sink.write.'a>(this: &Sink.write.'a readonly Sink, Message) => Message kind=symbol target_receiver=Placed<Dynamic<Sink>, "shared"> dispatch=dynamic constraint=Sink target=Sink.write
/// @resolution.call source=sharedSink.write(sharedMessage) parameters=(Placed<Message, "shared">) arguments=(provided(sharedMessage) as Placed<Message, "shared">) return=Placed<Message, "shared"> kind=dynamic target=Sink.write receiver=Placed<Dynamic<Sink>, "shared"> constraint=Sink adjustments=(Placed<Dynamic<Sink>, "shared"> => direct -> Dynamic<Sink>, borrow(&'static readonly Dynamic<Sink>))
/// @resolution.place source=sharedSink placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedSink root=sharedSink
/// @resolution.name source=sharedMessage target=sharedMessage
/// @resolution.place source=sharedMessage placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedMessage root=sharedMessage
/// @resolution.name source=Message target=Message
"#,
    );
}

#[test]
fn test_require_exclusive_receiver_for_exclusive_methods() {
    let session = TestSession::single(
        r#"
class Buffer {
    clear(&exclusive this): void {}
}

declare const localBuffer: local Buffer;
declare const sharedBuffer: shared Buffer;

localBuffer.clear();
sharedBuffer.clear();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Buffer {
    clear(&exclusive this): void {}
}

declare const localBuffer: local Buffer;
declare const sharedBuffer: shared Buffer;

localBuffer.clear();
sharedBuffer.clear();

=== dir ===
class Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.class symbol=Buffer
/// @definition.method symbol=Buffer.clear source="clear(&exclusive this): void {}" slot=clear type=<Buffer.clear.'a>(this: &Buffer.clear.'a exclusive this) => void

    clear(&exclusive this): void {}
    /// @generic.template symbol=Buffer.clear parameters=('a)
    /// @type.symbol symbol=Buffer.clear source="clear(&exclusive this): void {}" type=<Buffer.clear.'a>(this: &Buffer.clear.'a exclusive this) => void
    /// @type.symbol symbol=Buffer.clear.this source="&exclusive this" type=&Buffer.clear.'a exclusive this

}

declare const localBuffer: local Buffer;
/// @type.symbol symbol=localBuffer source=localBuffer type=Placed<Buffer, "local">
/// @resolution.pattern source=localBuffer kind=binding target=localBuffer
/// @resolution.name source=Buffer target=Buffer

declare const sharedBuffer: shared Buffer;
/// @type.symbol symbol=sharedBuffer source=sharedBuffer type=Placed<Buffer, "shared">
/// @resolution.pattern source=sharedBuffer kind=binding target=sharedBuffer
/// @resolution.name source=Buffer target=Buffer

localBuffer.clear();
/// @resolution.name source=localBuffer target=localBuffer
/// @resolution.member source=localBuffer.clear receiver=Placed<Buffer, "local"> type=<Buffer.clear.'a>(this: &Buffer.clear.'a exclusive Buffer) => void kind=symbol target_receiver=Placed<Buffer, "local"> target=Buffer.clear
/// @resolution.call source=localBuffer.clear() parameters=() return=void kind=symbol target=Buffer.clear receiver=Placed<Buffer, "local"> adjustments=(Placed<Buffer, "local"> => direct -> Buffer, borrow(&'static exclusive Buffer))
/// @resolution.place source=localBuffer placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=localBuffer root=localBuffer

sharedBuffer.clear();
/// @resolution.name source=sharedBuffer target=sharedBuffer
/// @resolution.member source=sharedBuffer.clear receiver=Placed<Buffer, "shared"> type=<Buffer.clear.'a>(this: &Buffer.clear.'a exclusive Buffer) => void kind=symbol target_receiver=Placed<Buffer, "shared"> target=Buffer.clear
/// @resolution.call source=sharedBuffer.clear() parameters=() return=void kind=symbol target=Buffer.clear receiver=Placed<Buffer, "shared">
/// @resolution.place source=sharedBuffer placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedBuffer root=sharedBuffer
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type 'shared Buffer' is not assignable to the method's 'this' type '&exclusive Buffer'"
/// @diagnostic.label line=10 column=1 span="sharedBuffer.clear()" line_source="sharedBuffer.clear();"
"#,
    );
}

#[test]
fn test_restrict_borrowed_receivers_of_immutable_direct_storage() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;

    read(&readonly this): int32 {
        return this.value;
    }

    increment(&exclusive this): void {
        this.value = this.value + 1;
    }
}

declare const counter: Counter;

counter.read();
counter.increment();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Counter {
    value: int32;

    read(&readonly this): int32 {
        return this.value;
    }

    increment(&exclusive this): void {
        this.value = this.value + 1;
    }
}

declare const counter: Counter;

counter.read();
counter.increment();

=== dir ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.increment slot=increment type=<Counter.increment.'a>(this: &Counter.increment.'a exclusive this) => void
/// @definition.method symbol=Counter.read slot=read type=<Counter.read.'a>(this: &Counter.read.'a readonly this) => int32

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    read(&readonly this): int32 {
    /// @generic.template symbol=Counter.read parameters=('a)
    /// @type.symbol symbol=Counter.read type=<Counter.read.'a>(this: &Counter.read.'a readonly this) => int32
    /// @type.symbol symbol=Counter.read.this source="&readonly this" type=&Counter.read.'a readonly this

        return this.value;
        /// @resolution.member source=this.value receiver=&Counter.read.'a readonly Counter type=int32 kind=field target_receiver=&Counter.read.'a readonly Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.read.'a readonly Counter
        /// @resolution.place source=this placement="local" lifetime=Counter.read.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=Counter.read.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }

    increment(&exclusive this): void {
    /// @generic.template symbol=Counter.increment parameters=('a)
    /// @type.symbol symbol=Counter.increment type=<Counter.increment.'a>(this: &Counter.increment.'a exclusive this) => void
    /// @type.symbol symbol=Counter.increment.this source="&exclusive this" type=&Counter.increment.'a exclusive this

        this.value = this.value + 1;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a exclusive Counter
        /// @resolution.place source=this placement="local" lifetime=Counter.increment.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&Counter.increment.'a exclusive Counter, target=field(receiver=&Counter.increment.'a exclusive Counter, target=Counter.value, type=int32), type=int32" type=int32
        /// @resolution.member source=this.value receiver=&Counter.increment.'a exclusive Counter type=int32 kind=field target_receiver=&Counter.increment.'a exclusive Counter key=value target=Counter.value target_type=int32
        /// @resolution.operator source="this.value + 1" type=int32 operator="+" kind=builtin operands=[this.value as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a exclusive Counter
        /// @resolution.place source=this placement="local" lifetime=Counter.increment.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=Counter.increment.'a access="exclusive"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}

declare const counter: Counter;
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=Counter target=Counter

counter.read();
/// @resolution.name source=counter target=counter
/// @resolution.member source=counter.read receiver=Counter type=<Counter.read.'a>(this: &Counter.read.'a readonly Counter) => int32 kind=symbol target_receiver=Counter target=Counter.read
/// @resolution.call source=counter.read() parameters=() return=int32 kind=symbol target=Counter.read receiver=Counter adjustments=(borrow(&'static readonly Counter))
/// @resolution.place source=counter placement="local" lifetime="static" access="readonly"
/// @resolution.access source=counter root=counter

counter.increment();
/// @resolution.name source=counter target=counter
/// @resolution.member source=counter.increment receiver=Counter type=<Counter.increment.'a>(this: &Counter.increment.'a exclusive Counter) => void kind=symbol target_receiver=Counter target=Counter.increment
/// @resolution.call source=counter.increment() parameters=() return=void kind=symbol target=Counter.increment receiver=Counter
/// @resolution.place source=counter placement="local" lifetime="static" access="readonly"
/// @resolution.access source=counter root=counter
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type 'Counter' is not assignable to the method's 'this' type '&exclusive Counter'"
/// @diagnostic.label line=17 column=1 span="counter.increment()" line_source="counter.increment();"
"#,
    );
}

#[test]
fn test_reject_readonly_view_calling_an_implicit_struct_method() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;

    scale(by: int32): void {
        this.x = this.x * by;
    }
}

function freeze(point: readonly Point): void {
    point.scale(2);
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

    scale(by: int32): void {
        this.x = this.x * by;
    }
}

function freeze(point: readonly Point): void {
    point.scale(2);
}

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.method symbol=Point.scale slot=scale type=<Point.scale.'a>(this: &Point.scale.'a readonly this, int32) => void

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    scale(by: int32): void {
    /// @generic.template symbol=Point.scale parameters=('a)
    /// @type.symbol symbol=Point.scale type=<Point.scale.'a>(this: &Point.scale.'a readonly this, int32) => void
    /// @type.symbol symbol=Point.scale.by source="by: int32" type=int32

        this.x = this.x * by;
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.scale.'a readonly Point
        /// @resolution.place source=this placement="local" lifetime=Point.scale.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.x kind=place
        /// @resolution.access source=this.x root=this keys=[x]
        /// @resolution.assignment source=this.x write="receiver=Readonly<&Point.scale.'a readonly Point>, target=field(receiver=Readonly<&Point.scale.'a readonly Point>, target=Point.x, type=int32), type=int32" type=int32
        /// @resolution.member source=this.x receiver=&Point.scale.'a readonly Point type=int32 kind=field target_receiver=&Point.scale.'a readonly Point key=x target=Point.x target_type=int32
        /// @resolution.operator source="this.x * by" type=int32 operator="*" kind=builtin operands=[this.x as int32 families=(integer), by as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.scale.'a readonly Point
        /// @resolution.place source=this placement="local" lifetime=Point.scale.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement="local" lifetime=Point.scale.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @resolution.name source=by target=Point.scale.by
        /// @resolution.place source=by placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=by root=Point.scale.by

    }
}

function freeze(point: readonly Point): void {
/// @type.symbol symbol=freeze type=(Readonly<Point>) => void
/// @type.symbol symbol=freeze.point source="point: readonly Point" type=Readonly<Point>
/// @resolution.name source=Point target=Point

    point.scale(2);
    /// @resolution.name source=point target=freeze.point
    /// @resolution.member source=point.scale receiver=Readonly<Point> type=<Point.scale.'a>(this: &Point.scale.'a readonly Point, int32) => void kind=symbol target_receiver=Readonly<Point> target=Point.scale
    /// @resolution.call source=point.scale(2) parameters=(int32) arguments=(provided(2) as int32) return=void kind=symbol target=Point.scale receiver=Readonly<Point> adjustments=(Readonly<Point> => direct -> Point, borrow(&'frame readonly Point))
    /// @resolution.place source=point placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=point root=freeze.point

}
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'x'"
/// @diagnostic.label line=6 column=14 span="x" line_source="this.x = this.x * by;"
"#,
    );
}

#[test]
fn test_reject_owned_class_calling_an_implicit_managed_method() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32 = 0;

    read(): int32 {
        return this.count;
    }
}

function inspect(counter: ^Counter): int32 {
    return counter.read();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    count: int32 = 0;

    read(): int32 {
        return this.count;
    }
}

function inspect(counter: ^Counter): int32 {
    return counter.read();
}

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.count source="count: int32 = 0" key=count type=int32
/// @definition.method symbol=Counter.read slot=read type=(this: this) => int32

    count: int32 = 0;
    /// @type.symbol symbol=Counter.count source="count: int32 = 0" type=int32

    read(): int32 {
    /// @type.symbol symbol=Counter.read type=(this: this) => int32

        return this.count;
        /// @resolution.member source=this.count receiver=Counter type=int32 kind=field target_receiver=Counter key=count target=Counter.count target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.count placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.count root=this keys=[count]

    }
}

function inspect(counter: ^Counter): int32 {
/// @type.symbol symbol=inspect type=(Owned<Counter>) => int32
/// @type.symbol symbol=inspect.counter source="counter: ^Counter" type=Owned<Counter>
/// @resolution.name source=Counter target=Counter

    return counter.read();
    /// @resolution.name source=counter target=inspect.counter
    /// @resolution.member source=counter.read receiver=Owned<Counter> type=(this: Counter) => int32 kind=symbol target_receiver=Owned<Counter> target=Counter.read
    /// @resolution.call source=counter.read() parameters=() return=int32 kind=symbol target=Counter.read receiver=Owned<Counter>
    /// @resolution.place source=counter placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=counter root=inspect.counter

}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '^Counter' is not assignable to the method's 'this' type 'Counter'"
/// @diagnostic.label line=11 column=12 span="counter.read()" line_source="return counter.read();"
"#,
    );
}

#[test]
fn test_accept_explicit_borrowed_this_through_readonly_views() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;

    length(&readonly this): int32 {
        return this.x;
    }
}

function measure(point: readonly Point): int32 {
    return point.length();
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;

    length(&readonly this): int32 {
        return this.x;
    }
}

function measure(point: readonly Point): int32 {
    return point.length();
}

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.method symbol=Point.length slot=length type=<Point.length.'a>(this: &Point.length.'a readonly Point) => int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    length(&readonly this): int32 {
    /// @generic.template symbol=Point.length parameters=('a)
    /// @type.symbol symbol=Point.length type=<Point.length.'a>(this: &Point.length.'a readonly Point) => int32
    /// @type.symbol symbol=Point.length.this source="&readonly this" type=&Point.length.'a readonly this

        return this.x;
        /// @resolution.member source=this.x receiver=&Point.length.'a readonly Point type=int32 kind=field target_receiver=&Point.length.'a readonly Point key=x target=Point.x target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.length.'a readonly Point
        /// @resolution.place source=this placement="local" lifetime=Point.length.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement="local" lifetime=Point.length.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]

    }
}

function measure(point: readonly Point): int32 {
/// @type.symbol symbol=measure type=(Readonly<Point>) => int32
/// @type.symbol symbol=measure.point source="point: readonly Point" type=Readonly<Point>
/// @resolution.name source=Point target=Point

    return point.length();
    /// @resolution.name source=point target=measure.point
    /// @resolution.member source=point.length receiver=Readonly<Point> type=<Point.length.'a>(this: &Point.length.'a readonly Point) => int32 kind=symbol target_receiver=Readonly<Point> target=Point.length
    /// @resolution.call source=point.length() parameters=() return=int32 kind=symbol target=Point.length receiver=Readonly<Point> adjustments=(Readonly<Point> => direct -> Point, borrow(&'frame readonly Point))
    /// @resolution.place source=point placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=point root=measure.point

}
"#,
    );
}

#[test]
fn test_default_value_getter_borrows_readonly() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;

    get current(): int32 {
        this.value
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Counter {
    value: int32;

    get current(): int32 {
        this.value
    }
}

=== dir ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.current slot=current role=getter type=<Counter.current.'a>(this: &Counter.current.'a readonly Counter) => int32

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    get current(): int32 {
    /// @generic.template symbol=Counter.current parameters=('a)
    /// @type.symbol symbol=Counter.current type=<Counter.current.'a>(this: &Counter.current.'a readonly Counter) => int32

        this.value
        /// @resolution.member source=this.value receiver=&Counter.current.'a readonly Counter type=int32 kind=field target_receiver=&Counter.current.'a readonly Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.current.'a readonly Counter
        /// @resolution.place source=this placement="local" lifetime=Counter.current.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=Counter.current.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}
"#,
    );
}

#[test]
fn test_default_managed_getter_reads_through_readonly_view() {
    let session = TestSession::single(
        r#"
class Counter {
    value: int32 = 0;

    get current(): int32 {
        this.value
    }
}

function read(counter: readonly Counter): int32 {
    return counter.current;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    value: int32 = 0;

    get current(): int32 {
        this.value
    }
}

function read(counter: readonly Counter): int32 {
    return counter.current;
}

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32 = 0" key=value type=int32
/// @definition.method symbol=Counter.current slot=current role=getter type=(this: Readonly<Counter>) => int32

    value: int32 = 0;
    /// @type.symbol symbol=Counter.value source="value: int32 = 0" type=int32

    get current(): int32 {
    /// @type.symbol symbol=Counter.current type=(this: Readonly<Counter>) => int32

        this.value
        /// @resolution.member source=this.value receiver=Readonly<Counter> type=int32 kind=field target_receiver=Readonly<Counter> key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=Readonly<Counter>
        /// @resolution.place source=this placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}

function read(counter: readonly Counter): int32 {
/// @type.symbol symbol=read type=(Readonly<Counter>) => int32
/// @type.symbol symbol=read.counter source="counter: readonly Counter" type=Readonly<Counter>
/// @resolution.name source=Counter target=Counter

    return counter.current;
    /// @resolution.name source=counter target=read.counter
    /// @resolution.member source=counter.current receiver=Readonly<Counter> type=int32 kind=call target="Counter.current(parameters=(), arguments=(), return=int32)"
    /// @resolution.place source=counter placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=counter root=read.counter

}
"#,
    );
}
