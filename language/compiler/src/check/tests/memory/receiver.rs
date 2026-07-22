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

    session.assert_dir_checked(
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

=== checked ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32 = 0" key=value type=int32
/// @definition.method symbol=Counter.borrow slot=borrow type=<Counter.borrow.'l0>(this: &Counter.borrow.'l0 this) => int32
/// @definition.method symbol=Counter.increment slot=increment type=<Counter.increment.'l0>(this: &Counter.increment.'l0 exclusive this) => void
/// @definition.method symbol=Counter.inspect slot=inspect type=<Counter.inspect.'l0>(this: &Counter.inspect.'l0 readonly this) => int32
/// @definition.method symbol=Counter.peek slot=peek type=(this: Readonly<this>) => int32
/// @definition.method symbol=Counter.read slot=read type=(this: this) => int32
/// @definition.method symbol=Counter.zero slot=zero static=true type=() => Counter

    value: int32 = 0;
    /// @type.symbol symbol=Counter.value source="value: int32 = 0" type=int32

    read(this): int32 {
    /// @type.symbol symbol=Counter.read type=(this: this) => int32
    /// @type.symbol symbol=Counter.read.this source=this type=this

        return this.value;
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter

    }

    peek(readonly this): int32 {
    /// @type.symbol symbol=Counter.peek type=(this: Readonly<this>) => int32
    /// @type.symbol symbol=Counter.peek.this source="readonly this" type=Readonly<this>

        return this.value;
        /// @resolution.member source=this.value receiver=Readonly<Counter> kind=symbol target=Counter.value
        /// @resolution.receiver source=this kind=this declaration=Counter type=Readonly<Counter>

    }

    borrow(&this): int32 {
    /// @generic.template symbol=Counter.borrow parameters=('l0)
    /// @type.symbol symbol=Counter.borrow type=<Counter.borrow.'l0>(this: &Counter.borrow.'l0 this) => int32
    /// @type.symbol symbol=Counter.borrow.this source=&this type=&Counter.borrow.'l0 this

        return this.value;
        /// @resolution.member source=this.value receiver=&Counter.borrow.'l0 Counter kind=symbol target=Counter.value
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.borrow.'l0 Counter

    }

    inspect(&readonly this): int32 {
    /// @generic.template symbol=Counter.inspect parameters=('l0)
    /// @type.symbol symbol=Counter.inspect type=<Counter.inspect.'l0>(this: &Counter.inspect.'l0 readonly this) => int32
    /// @type.symbol symbol=Counter.inspect.this source="&readonly this" type=&Counter.inspect.'l0 readonly this

        return this.value;
        /// @resolution.member source=this.value receiver=&Counter.inspect.'l0 readonly Counter kind=symbol target=Counter.value
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.inspect.'l0 readonly Counter

    }

    increment(&exclusive this): void {
    /// @generic.template symbol=Counter.increment parameters=('l0)
    /// @type.symbol symbol=Counter.increment type=<Counter.increment.'l0>(this: &Counter.increment.'l0 exclusive this) => void
    /// @type.symbol symbol=Counter.increment.this source="&exclusive this" type=&Counter.increment.'l0 exclusive this

        this.value = this.value + 1;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'l0 exclusive Counter
        /// @resolution.pattern.assign source=this.value kind=place place=field(Counter.value) type=int32
        /// @resolution.member source=this.value receiver=&Counter.increment.'l0 exclusive Counter kind=symbol target=Counter.value
        /// @resolution.operator source="this.value + 1" kind=builtin
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'l0 exclusive Counter

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
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

=== checked ===
class Message {}
/// @type.symbol symbol=Message source="class Message {}" type=Message
/// @definition.class symbol=Message source="class Message {}"

newtype interface Sink {
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink nominal=true
/// @definition.method symbol=Sink.write source="write(&readonly this, value: Message): Message" slot=write type=<Sink.write.'l0>(this: &Sink.write.'l0 readonly this, Message) => Message

    write(&readonly this, value: Message): Message;
    /// @generic.template symbol=Sink.write parent=template#0 parameters=('l0)
    /// @type.symbol symbol=Sink.write source="write(&readonly this, value: Message): Message" type=<Sink.write.'l0>(this: &Sink.write.'l0 readonly this, Message) => Message
    /// @type.symbol symbol=Sink.write.this source="&readonly this" type=&Sink.write.'l0 readonly this
    /// @type.symbol symbol=Sink.write.value source="value: Message" type=Message
    /// @resolution.name source=Message target=Message
    /// @resolution.name source=Message target=Message

}

declare const localSink: local Sink;
/// @type.symbol symbol=localSink source=localSink type=Placed<Sink, "local">
/// @resolution.pattern source=localSink kind=binding target=localSink
/// @resolution.name source=Sink target=Sink

declare const sharedSink: shared Sink;
/// @type.symbol symbol=sharedSink source=sharedSink type=Placed<Sink, "shared">
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
/// @resolution.member source=localSink.write receiver=Placed<Sink, "local"> kind=symbol target=Sink.write
/// @resolution.call source=localSink.write(localMessage) parameters=(Message) arguments=(provided(localMessage) as Message) return=Message kind=symbol target=Sink.write receiver=Placed<Sink, "local"> adjustments=(borrow)
/// @resolution.name source=localMessage target=localMessage
/// @resolution.name source=Message target=Message

sharedSink.write(sharedMessage) satisfies shared Message;
/// @resolution.name source=sharedSink target=sharedSink
/// @resolution.member source=sharedSink.write receiver=Placed<Sink, "shared"> kind=symbol target=Sink.write
/// @resolution.call source=sharedSink.write(sharedMessage) parameters=(Placed<Message, "shared">) arguments=(provided(sharedMessage) as Placed<Message, "shared">) return=Placed<Message, "shared"> kind=symbol target=Sink.write receiver=Placed<Sink, "shared"> adjustments=(borrow)
/// @resolution.name source=sharedMessage target=sharedMessage
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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
class Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.class symbol=Buffer
/// @definition.method symbol=Buffer.clear source="clear(&exclusive this): void {}" slot=clear type=<Buffer.clear.'l0>(this: &Buffer.clear.'l0 exclusive this) => void

    clear(&exclusive this): void {}
    /// @generic.template symbol=Buffer.clear parameters=('l0)
    /// @type.symbol symbol=Buffer.clear source="clear(&exclusive this): void {}" type=<Buffer.clear.'l0>(this: &Buffer.clear.'l0 exclusive this) => void
    /// @type.symbol symbol=Buffer.clear.this source="&exclusive this" type=&Buffer.clear.'l0 exclusive this

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
/// @resolution.member source=localBuffer.clear receiver=Placed<Buffer, "local"> kind=symbol target=Buffer.clear
/// @resolution.call source=localBuffer.clear() parameters=() return=void kind=symbol target=Buffer.clear receiver=Placed<Buffer, "local"> adjustments=(borrow)

sharedBuffer.clear();
/// @resolution.name source=sharedBuffer target=sharedBuffer
/// @resolution.member source=sharedBuffer.clear receiver=Placed<Buffer, "shared"> kind=symbol target=Buffer.clear
/// @resolution.call source=sharedBuffer.clear() parameters=() return=void kind=symbol target=Buffer.clear receiver=Placed<Buffer, "shared">
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type 'shared Buffer' is not assignable to the method's 'this' type 'shared &exclusive Buffer'"
/// @diagnostic.label line=10 column=1 span="sharedBuffer.clear()" line_source="sharedBuffer.clear();"
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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.method symbol=Point.scale slot=scale type=<Point.scale.'l0>(this: &Point.scale.'l0 exclusive this, int32) => void

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    scale(by: int32): void {
    /// @generic.template symbol=Point.scale parameters=('l0)
    /// @type.symbol symbol=Point.scale type=<Point.scale.'l0>(this: &Point.scale.'l0 exclusive this, int32) => void
    /// @type.symbol symbol=Point.scale.by source="by: int32" type=int32

        this.x = this.x * by;
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.scale.'l0 exclusive Point
        /// @resolution.pattern.assign source=this.x kind=place place=field(Point.x) type=int32
        /// @resolution.member source=this.x receiver=&Point.scale.'l0 exclusive Point kind=symbol target=Point.x
        /// @resolution.operator source="this.x * by" kind=builtin
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.scale.'l0 exclusive Point
        /// @resolution.name source=by target=Point.scale.by

    }
}

function freeze(point: readonly Point): void {
/// @type.symbol symbol=freeze type=(Readonly<Point>) => void
/// @type.symbol symbol=freeze.point source="point: readonly Point" type=Readonly<Point>
/// @resolution.name source=Point target=Point

    point.scale(2);
    /// @resolution.name source=point target=freeze.point
    /// @resolution.member source=point.scale receiver=Readonly<Point> kind=symbol target=Point.scale
    /// @resolution.call source=point.scale(2) parameters=(int32) arguments=(provided(2) as int32) return=void kind=symbol target=Point.scale receiver=Readonly<Point> adjustments=(borrow)

}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type 'readonly Point' is not assignable to the method's 'this' type 'readonly Point'"
/// @diagnostic.label line=11 column=5 span="point.scale(2)" line_source="point.scale(2);"
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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
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
        /// @resolution.member source=this.count receiver=Counter kind=symbol target=Counter.count
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter

    }
}

function inspect(counter: ^Counter): int32 {
/// @type.symbol symbol=inspect type=(Owned<Counter>) => int32
/// @type.symbol symbol=inspect.counter source="counter: ^Counter" type=Owned<Counter>
/// @resolution.name source=Counter target=Counter

    return counter.read();
    /// @resolution.name source=counter target=inspect.counter
    /// @resolution.member source=counter.read receiver=Owned<Counter> kind=symbol target=Counter.read
    /// @resolution.call source=counter.read() parameters=() return=int32 kind=symbol target=Counter.read receiver=Owned<Counter>

}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '^Counter' is not assignable to the method's 'this' type '^Counter'"
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

    session.assert_dir_checked(
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

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.method symbol=Point.length slot=length type=<Point.length.'l0>(this: &Point.length.'l0 readonly this) => int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    length(&readonly this): int32 {
    /// @generic.template symbol=Point.length parameters=('l0)
    /// @type.symbol symbol=Point.length type=<Point.length.'l0>(this: &Point.length.'l0 readonly this) => int32
    /// @type.symbol symbol=Point.length.this source="&readonly this" type=&Point.length.'l0 readonly this

        return this.x;
        /// @resolution.member source=this.x receiver=&Point.length.'l0 readonly Point kind=symbol target=Point.x
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.length.'l0 readonly Point

    }
}

function measure(point: readonly Point): int32 {
/// @type.symbol symbol=measure type=(Readonly<Point>) => int32
/// @type.symbol symbol=measure.point source="point: readonly Point" type=Readonly<Point>
/// @resolution.name source=Point target=Point

    return point.length();
    /// @resolution.name source=point target=measure.point
    /// @resolution.member source=point.length receiver=Readonly<Point> kind=symbol target=Point.length
    /// @resolution.call source=point.length() parameters=() return=int32 kind=symbol target=Point.length receiver=Readonly<Point> adjustments=(borrow)

}
"#,
    );
}
