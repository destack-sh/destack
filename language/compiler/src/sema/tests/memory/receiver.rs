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

    increment(&this): void {
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

    increment(&this): void {
        this.value = this.value + 1;
    }

    static zero(): Counter {
        return new Counter();
    }
}

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32 = 0" key=value type=int32
/// @definition.method symbol=Counter.borrow slot=borrow type=<Counter.borrow.'a>(this: &Counter.borrow.'a Counter) => int32
/// @definition.method symbol=Counter.increment slot=increment type=<Counter.increment.'a>(this: &Counter.increment.'a Counter) => void
/// @definition.method symbol=Counter.inspect slot=inspect type=<Counter.inspect.'a>(this: &Counter.inspect.'a readonly Counter) => int32
/// @definition.method symbol=Counter.peek slot=peek type=(this: readonly Counter) => int32
/// @definition.method symbol=Counter.read slot=read type=(this: Counter) => int32
/// @definition.method symbol=Counter.zero slot=zero static=true type=() => Counter

    value: int32 = 0;
    /// @type.symbol symbol=Counter.value source="value: int32 = 0" type=int32

    read(this): int32 {
    /// @type.symbol symbol=Counter.read type=(this: Counter) => int32
    /// @type.symbol symbol=Counter.read.this source=this type=Counter

        return this.value;
        /// @resolution.member source=this.value receiver=Counter type=int32 kind=field target_receiver=Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]

    }

    peek(readonly this): int32 {
    /// @type.symbol symbol=Counter.peek type=(this: readonly Counter) => int32
    /// @type.symbol symbol=Counter.peek.this source="readonly this" type=readonly Counter

        return this.value;
        /// @resolution.member source=this.value receiver=readonly Counter type=int32 kind=field target_receiver=readonly Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=readonly Counter
        /// @resolution.place source=this placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }

    borrow(&this): int32 {
    /// @generic.template symbol=Counter.borrow parameters=('a)
    /// @type.symbol symbol=Counter.borrow type=<Counter.borrow.'a>(this: &Counter.borrow.'a Counter) => int32
    /// @type.symbol symbol=Counter.borrow.this source=&this type=&Counter.borrow.'a Counter

        return this.value;
        /// @resolution.member source=this.value receiver=&Counter.borrow.'a Counter type=int32 kind=field target_receiver=&Counter.borrow.'a Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.borrow.'a Counter
        /// @resolution.place source=this placement=Counter.borrow.'a lifetime=Counter.borrow.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=Counter.borrow.'a lifetime=Counter.borrow.'a access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]

    }

    inspect(&readonly this): int32 {
    /// @generic.template symbol=Counter.inspect parameters=('a)
    /// @type.symbol symbol=Counter.inspect type=<Counter.inspect.'a>(this: &Counter.inspect.'a readonly Counter) => int32
    /// @type.symbol symbol=Counter.inspect.this source="&readonly this" type=&Counter.inspect.'a readonly Counter

        return this.value;
        /// @resolution.member source=this.value receiver=&Counter.inspect.'a readonly Counter type=int32 kind=field target_receiver=&Counter.inspect.'a readonly Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.inspect.'a readonly Counter
        /// @resolution.place source=this placement=Counter.inspect.'a lifetime=Counter.inspect.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=Counter.inspect.'a lifetime=Counter.inspect.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }

    increment(&this): void {
    /// @generic.template symbol=Counter.increment parameters=('a)
    /// @type.symbol symbol=Counter.increment type=<Counter.increment.'a>(this: &Counter.increment.'a Counter) => void
    /// @type.symbol symbol=Counter.increment.this source=&this type=&Counter.increment.'a Counter

        this.value = this.value + 1;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a Counter
        /// @resolution.place source=this placement=Counter.increment.'a lifetime=Counter.increment.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement=Counter.increment.'a lifetime=Counter.increment.'a access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&Counter.increment.'a Counter, target=field(receiver=&Counter.increment.'a Counter, target=Counter.value, type=int32), type=int32" type=int32
        /// @resolution.member source=this.value receiver=&Counter.increment.'a Counter type=int32 kind=field target_receiver=&Counter.increment.'a Counter key=value target=Counter.value target_type=int32
        /// @resolution.operator source="this.value + 1" type=int32 operator="+" kind=builtin operands=[this.value as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a Counter
        /// @resolution.place source=this placement=Counter.increment.'a lifetime=Counter.increment.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=Counter.increment.'a lifetime=Counter.increment.'a access="mutable"
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
fn test_require_a_mutable_receiver_for_mutable_methods() {
    let session = TestSession::single(
        r#"
class Buffer {
    clear(&this): void {}
}

shared class SharedBuffer {
    clear(&this): void {}
}

declare const localBuffer: Buffer;
declare const sharedBuffer: SharedBuffer;

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
    clear(&this): void {}
}

shared class SharedBuffer {
    clear(&this): void {}
}

declare const localBuffer: Buffer;
declare const sharedBuffer: SharedBuffer;

localBuffer.clear<"managed">();
sharedBuffer.clear<"managed">();

=== dir ===
class Buffer {
/// @type.symbol symbol=Buffer type=typeof Buffer
/// @definition.class symbol=Buffer
/// @definition.method symbol=Buffer.clear source="clear(&this): void {}" slot=clear type=<Buffer.clear.'a>(this: &Buffer.clear.'a Buffer) => void

    clear(&this): void {}
    /// @generic.template symbol=Buffer.clear parameters=('a)
    /// @type.symbol symbol=Buffer.clear source="clear(&this): void {}" type=<Buffer.clear.'a>(this: &Buffer.clear.'a Buffer) => void
    /// @type.symbol symbol=Buffer.clear.this source=&this type=&Buffer.clear.'a Buffer

}

shared class SharedBuffer {
/// @type.symbol symbol=SharedBuffer type=typeof SharedBuffer
/// @definition.class symbol=SharedBuffer
/// @definition.method symbol=SharedBuffer.clear source="clear(&this): void {}" slot=clear type=<SharedBuffer.clear.'a>(this: &SharedBuffer.clear.'a SharedBuffer) => void

    clear(&this): void {}
    /// @generic.template symbol=SharedBuffer.clear parameters=('a)
    /// @type.symbol symbol=SharedBuffer.clear source="clear(&this): void {}" type=<SharedBuffer.clear.'a>(this: &SharedBuffer.clear.'a SharedBuffer) => void
    /// @type.symbol symbol=SharedBuffer.clear.this source=&this type=&SharedBuffer.clear.'a SharedBuffer

}

declare const localBuffer: Buffer;
/// @type.symbol symbol=localBuffer source=localBuffer type=Buffer
/// @resolution.pattern source=localBuffer kind=binding target=localBuffer
/// @resolution.name source=Buffer target=Buffer

declare const sharedBuffer: SharedBuffer;
/// @type.symbol symbol=sharedBuffer source=sharedBuffer type=SharedBuffer
/// @resolution.pattern source=sharedBuffer kind=binding target=sharedBuffer
/// @resolution.name source=SharedBuffer target=SharedBuffer

localBuffer.clear();
/// @resolution.name source=localBuffer target=localBuffer
/// @resolution.member source=localBuffer.clear receiver=Buffer type=<Buffer.clear.'a>(this: &Buffer.clear.'a Buffer) => void kind=symbol target_receiver=Buffer target=Buffer.clear
/// @resolution.call source=localBuffer.clear() parameters=() return=void regions=("managed" & "local") kind=symbol target=Buffer.clear receiver=Buffer adjustments=(borrow(&'managed Buffer)) instance="Buffer.clear<\"managed\" & \"local\">"
/// @resolution.place source=localBuffer placement="local" lifetime="static" access="immutable"
/// @resolution.access source=localBuffer root=localBuffer
/// @generic.instantiation id="Buffer.clear<\"managed\" & \"local\">" template=Buffer.clear arguments=("managed" & "local")

sharedBuffer.clear();
/// @resolution.name source=sharedBuffer target=sharedBuffer
/// @resolution.member source=sharedBuffer.clear receiver=SharedBuffer type=<SharedBuffer.clear.'a>(this: &SharedBuffer.clear.'a SharedBuffer) => void kind=symbol target_receiver=SharedBuffer target=SharedBuffer.clear
/// @resolution.call source=sharedBuffer.clear() parameters=() return=void regions=("managed" & "shared") kind=symbol target=SharedBuffer.clear receiver=SharedBuffer adjustments=(borrow(&'managed SharedBuffer)) instance="SharedBuffer.clear<\"managed\" & \"shared\">"
/// @resolution.place source=sharedBuffer placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=sharedBuffer root=sharedBuffer
/// @generic.instantiation id="SharedBuffer.clear<\"managed\" & \"shared\">" template=SharedBuffer.clear arguments=("managed" & "shared")
"#,
        r#"

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

    increment(&this): void {
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

    increment(&this): void {
        this.value = this.value + 1;
    }
}

declare const counter: Counter;

counter.read<"static">();
counter.increment<"frame">();

=== dir ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.increment slot=increment type=<Counter.increment.'a>(this: &Counter.increment.'a Counter) => void
/// @definition.method symbol=Counter.read slot=read type=<Counter.read.'a>(this: &Counter.read.'a readonly Counter) => int32

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    read(&readonly this): int32 {
    /// @generic.template symbol=Counter.read parameters=('a)
    /// @type.symbol symbol=Counter.read type=<Counter.read.'a>(this: &Counter.read.'a readonly Counter) => int32
    /// @type.symbol symbol=Counter.read.this source="&readonly this" type=&Counter.read.'a readonly Counter

        return this.value;
        /// @resolution.member source=this.value receiver=&Counter.read.'a readonly Counter type=int32 kind=field target_receiver=&Counter.read.'a readonly Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.read.'a readonly Counter
        /// @resolution.place source=this placement=Counter.read.'a lifetime=Counter.read.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=Counter.read.'a lifetime=Counter.read.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }

    increment(&this): void {
    /// @generic.template symbol=Counter.increment parameters=('a)
    /// @type.symbol symbol=Counter.increment type=<Counter.increment.'a>(this: &Counter.increment.'a Counter) => void
    /// @type.symbol symbol=Counter.increment.this source=&this type=&Counter.increment.'a Counter

        this.value = this.value + 1;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a Counter
        /// @resolution.place source=this placement=Counter.increment.'a lifetime=Counter.increment.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement=Counter.increment.'a lifetime=Counter.increment.'a access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&Counter.increment.'a Counter, target=field(receiver=&Counter.increment.'a Counter, target=Counter.value, type=int32), type=int32" type=int32
        /// @resolution.member source=this.value receiver=&Counter.increment.'a Counter type=int32 kind=field target_receiver=&Counter.increment.'a Counter key=value target=Counter.value target_type=int32
        /// @resolution.operator source="this.value + 1" type=int32 operator="+" kind=builtin operands=[this.value as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a Counter
        /// @resolution.place source=this placement=Counter.increment.'a lifetime=Counter.increment.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=Counter.increment.'a lifetime=Counter.increment.'a access="mutable"
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
/// @resolution.call source=counter.read() parameters=() return=int32 regions=("static" & "local") kind=symbol target=Counter.read receiver=Counter adjustments=(borrow(&'static readonly Counter)) instance="Counter.read<\"static\" & \"local\">"
/// @resolution.place source=counter placement="local" lifetime="static" access="immutable"
/// @resolution.access source=counter root=counter
/// @generic.instantiation id="Counter.read<\"static\" & \"local\">" template=Counter.read arguments=("static" & "local")

counter.increment();
/// @resolution.name source=counter target=counter
/// @resolution.member source=counter.increment receiver=Counter type=<Counter.increment.'a>(this: &Counter.increment.'a Counter) => void kind=symbol target_receiver=Counter target=Counter.increment
/// @resolution.call source=counter.increment() parameters=() return=void regions=("frame") kind=symbol target=Counter.increment receiver=Counter instance="Counter.increment<\"frame\">"
/// @resolution.place source=counter placement="local" lifetime="static" access="immutable"
/// @resolution.access source=counter root=counter
/// @generic.instantiation id="Counter.increment<\"frame\">" template=Counter.increment arguments=("frame")
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type 'Counter' is not assignable to the method's 'this' type '&Counter'"
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
    point.scale<"frame">(2);
}

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.method symbol=Point.scale slot=scale type=<Point.scale.'a>(this: &Point.scale.'a readonly Point, int32) => void

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    scale(by: int32): void {
    /// @generic.template symbol=Point.scale parameters=('a)
    /// @type.symbol symbol=Point.scale type=<Point.scale.'a>(this: &Point.scale.'a readonly Point, int32) => void
    /// @type.symbol symbol=Point.scale.this type=&Point.scale.'a readonly Point
    /// @type.symbol symbol=Point.scale.by source="by: int32" type=int32

        this.x = this.x * by;
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.scale.'a readonly Point
        /// @resolution.place source=this placement=Point.scale.'a lifetime=Point.scale.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.x kind=place
        /// @resolution.place source=this.x placement=Point.scale.'a lifetime=Point.scale.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @resolution.assignment source=this.x write="receiver=readonly &Point.scale.'a readonly Point, target=field(receiver=readonly &Point.scale.'a readonly Point, target=Point.x, type=int32), type=int32" type=int32
        /// @resolution.member source=this.x receiver=&Point.scale.'a readonly Point type=int32 kind=field target_receiver=&Point.scale.'a readonly Point key=x target=Point.x target_type=int32
        /// @resolution.operator source="this.x * by" type=int32 operator="*" kind=builtin operands=[this.x as int32 families=(integer), by as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.scale.'a readonly Point
        /// @resolution.place source=this placement=Point.scale.'a lifetime=Point.scale.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=Point.scale.'a lifetime=Point.scale.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @resolution.name source=by target=Point.scale.by
        /// @resolution.place source=by placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=by root=Point.scale.by

    }
}

function freeze(point: readonly Point): void {
/// @type.symbol symbol=freeze type=(readonly Point) => void
/// @type.symbol symbol=freeze.point source="point: readonly Point" type=readonly Point
/// @resolution.name source=Point target=Point

    point.scale(2);
    /// @resolution.name source=point target=freeze.point
    /// @resolution.member source=point.scale receiver=readonly Point type=<Point.scale.'a>(this: &Point.scale.'a readonly Point, int32) => void kind=symbol target_receiver=readonly Point target=Point.scale
    /// @resolution.call source=point.scale(2) parameters=(int32) arguments=(provided(2) as int32) return=void regions=("frame" & "local") kind=symbol target=Point.scale receiver=readonly Point adjustments=(borrow(&'frame readonly Point)) instance="Point.scale<\"frame\" & \"local\">"
    /// @resolution.place source=point placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=point root=freeze.point
    /// @generic.instantiation id="Point.scale<\"frame\" & \"local\">" template=Point.scale arguments=("frame" & "local")

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
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.count source="count: int32 = 0" key=count type=int32
/// @definition.method symbol=Counter.read slot=read type=(this: Counter) => int32

    count: int32 = 0;
    /// @type.symbol symbol=Counter.count source="count: int32 = 0" type=int32

    read(): int32 {
    /// @type.symbol symbol=Counter.read type=(this: Counter) => int32
    /// @type.symbol symbol=Counter.read.this type=Counter

        return this.count;
        /// @resolution.member source=this.count receiver=Counter type=int32 kind=field target_receiver=Counter key=count target=Counter.count target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.count placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.count root=this keys=[count]

    }
}

function inspect(counter: ^Counter): int32 {
/// @type.symbol symbol=inspect type=(^Counter) => int32
/// @type.symbol symbol=inspect.counter source="counter: ^Counter" type=^Counter
/// @resolution.name source=Counter target=Counter

    return counter.read();
    /// @resolution.name source=counter target=inspect.counter
    /// @resolution.member source=counter.read receiver=^Counter type=(this: Counter) => int32 kind=symbol target_receiver=^Counter target=Counter.read
    /// @resolution.call source=counter.read() parameters=() return=int32 kind=symbol target=Counter.read receiver=^Counter
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
    return point.length<"frame">();
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
    /// @type.symbol symbol=Point.length.this source="&readonly this" type=&Point.length.'a readonly Point

        return this.x;
        /// @resolution.member source=this.x receiver=&Point.length.'a readonly Point type=int32 kind=field target_receiver=&Point.length.'a readonly Point key=x target=Point.x target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.length.'a readonly Point
        /// @resolution.place source=this placement=Point.length.'a lifetime=Point.length.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=Point.length.'a lifetime=Point.length.'a access="readonly"
        /// @resolution.access source=this.x root=this keys=[x]

    }
}

function measure(point: readonly Point): int32 {
/// @type.symbol symbol=measure type=(readonly Point) => int32
/// @type.symbol symbol=measure.point source="point: readonly Point" type=readonly Point
/// @resolution.name source=Point target=Point

    return point.length();
    /// @resolution.name source=point target=measure.point
    /// @resolution.member source=point.length receiver=readonly Point type=<Point.length.'a>(this: &Point.length.'a readonly Point) => int32 kind=symbol target_receiver=readonly Point target=Point.length
    /// @resolution.call source=point.length() parameters=() return=int32 regions=("frame" & "local") kind=symbol target=Point.length receiver=readonly Point adjustments=(borrow(&'frame readonly Point)) instance="Point.length<\"frame\" & \"local\">"
    /// @resolution.place source=point placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=point root=measure.point
    /// @generic.instantiation id="Point.length<\"frame\" & \"local\">" template=Point.length arguments=("frame" & "local")
    /// @generic.instance id="Point.length<\"bound0\" & \"local\">" template=Point.length arguments=("bound0" & "local")

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
    /// @type.symbol symbol=Counter.current.this type=&Counter.current.'a readonly Counter

        this.value
        /// @resolution.member source=this.value receiver=&Counter.current.'a readonly Counter type=int32 kind=field target_receiver=&Counter.current.'a readonly Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.current.'a readonly Counter
        /// @resolution.place source=this placement=Counter.current.'a lifetime=Counter.current.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=Counter.current.'a lifetime=Counter.current.'a access="readonly"
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
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32 = 0" key=value type=int32
/// @definition.method symbol=Counter.current slot=current role=getter type=(this: readonly Counter) => int32

    value: int32 = 0;
    /// @type.symbol symbol=Counter.value source="value: int32 = 0" type=int32

    get current(): int32 {
    /// @type.symbol symbol=Counter.current type=(this: readonly Counter) => int32
    /// @type.symbol symbol=Counter.current.this type=readonly Counter

        this.value
        /// @resolution.member source=this.value receiver=readonly Counter type=int32 kind=field target_receiver=readonly Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=readonly Counter
        /// @resolution.place source=this placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}

function read(counter: readonly Counter): int32 {
/// @type.symbol symbol=read type=(readonly Counter) => int32
/// @type.symbol symbol=read.counter source="counter: readonly Counter" type=readonly Counter
/// @resolution.name source=Counter target=Counter

    return counter.current;
    /// @resolution.name source=counter target=read.counter
    /// @resolution.member source=counter.current receiver=readonly Counter type=int32 kind=call target="Counter.current(parameters=(), arguments=(), return=int32)"
    /// @resolution.place source=counter placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=counter root=read.counter

}
"#,
    );
}

/// Reject a mutating method through a readonly borrow while every other receiver form calls it.
#[test]
fn test_reject_a_mutating_method_through_a_readonly_borrow() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32 = 0;

    bump(&this): void {
        this.count += 1;
    }
}

function mutate(owned: ^Counter, managed: Counter, borrowed: &Counter, view: &readonly Counter): void {
    owned.bump();
    managed.bump();
    borrowed.bump();
    view.bump();
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

    bump(&this): void {
        this.count += 1;
    }
}

function mutate<'a, 'b>(
    owned: ^Counter,
    managed: Counter,
    borrowed: &'a Counter,
    view: &'b readonly Counter,
): void {
    owned.bump<"frame">();
    managed.bump<"managed">();
    borrowed.bump<'a>();
    view.bump<'b>();
}

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.count source="count: int32 = 0" key=count type=int32
/// @definition.method symbol=Counter.bump slot=bump type=<Counter.bump.'a>(this: &Counter.bump.'a Counter) => void

    count: int32 = 0;
    /// @type.symbol symbol=Counter.count source="count: int32 = 0" type=int32

    bump(&this): void {
    /// @generic.template symbol=Counter.bump parameters=('a)
    /// @type.symbol symbol=Counter.bump type=<Counter.bump.'a>(this: &Counter.bump.'a Counter) => void
    /// @type.symbol symbol=Counter.bump.this source=&this type=&Counter.bump.'a Counter

        this.count += 1;
        /// @resolution.operator source="this.count += 1" type=int32 operator="+" kind=builtin operands=[this.count as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.bump.'a Counter
        /// @resolution.place source=this placement=Counter.bump.'a lifetime=Counter.bump.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.count kind=place
        /// @resolution.place source=this.count placement=Counter.bump.'a lifetime=Counter.bump.'a access="mutable"
        /// @resolution.assignment source=this.count read="receiver=&Counter.bump.'a Counter, target=field(receiver=&Counter.bump.'a Counter, target=Counter.count, type=int32), type=int32" write="receiver=&Counter.bump.'a Counter, target=field(receiver=&Counter.bump.'a Counter, target=Counter.count, type=int32), type=int32" type=int32
        /// @resolution.access source=this.count root=this keys=[count]

    }
}

function mutate(owned: ^Counter, managed: Counter, borrowed: &Counter, view: &readonly Counter): void {
/// @generic.template symbol=mutate parameters=('a, 'b)
/// @type.symbol symbol=mutate type=<mutate.'a, mutate.'b>(^Counter, Counter, &mutate.'a Counter, &mutate.'b readonly Counter) => void
/// @type.symbol symbol=mutate.owned source="owned: ^Counter" type=^Counter
/// @resolution.name source=Counter target=Counter
/// @type.symbol symbol=mutate.managed source="managed: Counter" type=Counter
/// @resolution.name source=Counter target=Counter
/// @type.symbol symbol=mutate.borrowed source="borrowed: &Counter" type=&mutate.'a Counter
/// @resolution.name source=Counter target=Counter
/// @type.symbol symbol=mutate.view source="view: &readonly Counter" type=&mutate.'b readonly Counter
/// @resolution.name source=Counter target=Counter

    owned.bump();
    /// @resolution.name source=owned target=mutate.owned
    /// @resolution.member source=owned.bump receiver=^Counter type=<Counter.bump.'a>(this: &Counter.bump.'a Counter) => void kind=symbol target_receiver=^Counter target=Counter.bump
    /// @resolution.call source=owned.bump() parameters=() return=void regions=("frame" & "local") kind=symbol target=Counter.bump receiver=^Counter adjustments=(borrow(&'frame Counter)) instance="Counter.bump<\"frame\" & \"local\">"
    /// @resolution.place source=owned placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=owned root=mutate.owned
    /// @generic.instantiation id="Counter.bump<\"frame\" & \"local\">" template=Counter.bump arguments=("frame" & "local")

    managed.bump();
    /// @resolution.name source=managed target=mutate.managed
    /// @resolution.member source=managed.bump receiver=Counter type=<Counter.bump.'a>(this: &Counter.bump.'a Counter) => void kind=symbol target_receiver=Counter target=Counter.bump
    /// @resolution.call source=managed.bump() parameters=() return=void regions=("managed" & "local") kind=symbol target=Counter.bump receiver=Counter adjustments=(borrow(&'managed Counter)) instance="Counter.bump<\"managed\" & \"local\">"
    /// @resolution.place source=managed placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=managed root=mutate.managed
    /// @generic.instantiation id="Counter.bump<\"managed\" & \"local\">" template=Counter.bump arguments=("managed" & "local")

    borrowed.bump();
    /// @resolution.name source=borrowed target=mutate.borrowed
    /// @resolution.member source=borrowed.bump receiver=&mutate.'a Counter type=<Counter.bump.'a>(this: &Counter.bump.'a Counter) => void kind=symbol target_receiver=&mutate.'a Counter target=Counter.bump
    /// @resolution.call source=borrowed.bump() parameters=() return=void regions=(mutate.'a) kind=symbol target=Counter.bump receiver=&mutate.'a Counter instance=Counter.bump<mutate.'a>
    /// @resolution.place source=borrowed placement=mutate.'a lifetime=mutate.'a access="mutable"
    /// @resolution.access source=borrowed root=mutate.borrowed
    /// @generic.instantiation id=Counter.bump<mutate.'a> template=Counter.bump arguments=(mutate.'a)

    view.bump();
    /// @resolution.name source=view target=mutate.view
    /// @resolution.member source=view.bump receiver=&mutate.'b readonly Counter type=<Counter.bump.'a>(this: &Counter.bump.'a Counter) => void kind=symbol target_receiver=&mutate.'b readonly Counter target=Counter.bump
    /// @resolution.call source=view.bump() parameters=() return=void regions=(mutate.'b) kind=symbol target=Counter.bump receiver=&mutate.'b readonly Counter instance=Counter.bump<mutate.'b>
    /// @resolution.place source=view placement=mutate.'b lifetime=mutate.'b access="readonly"
    /// @resolution.access source=view root=mutate.view
    /// @generic.instantiation id=Counter.bump<mutate.'b> template=Counter.bump arguments=(mutate.'b)

}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '&'b readonly Counter' is not assignable to the method's 'this' type '&'b Counter'"
/// @diagnostic.label line=14 column=5 span="view.bump()" line_source="view.bump();"
"#,
    );
}

/// A primitive's extension implements an operator interface through a readonly receiver.
#[test]
fn test_implement_an_operator_interface_for_a_primitive_through_a_readonly_receiver() {
    let session = TestSession::single(
        r#"
newtype interface Add<T = this> {
    type Output;

    add(this, other: T): this.Output;
}

export extension StringAdd of string implements Add<string> {
    type Output = ^string;

    add(&readonly this, other: string): ^string {
        todo()
    }
}

declare function todo(): never;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Add<in T = this> {
    type Output;

    add(this, other: T): this.Output;
}

export extension StringAdd of string implements Add<string> {
    type Output = ^string;

    add(&readonly this, other: string): ^string {
        todo()
    }
}

declare function todo(): never;

=== dir ===
newtype interface Add<T = this> {
/// @generic.template symbol=Add parameters=(in T = this, this: Add<T>)
/// @type.symbol symbol=Add type=Add
/// @definition.interface symbol=Add template=(in T = this, this: Add<T>) nominal=true
/// @definition.where symbol=Add relation=satisfies left=this right=Add<T>
/// @definition.associated.type symbol=Add.Output source="type Output" key=Output
/// @definition.method symbol=Add.add source="add(this, other: T): this.Output" slot=add type=(this: this, T) => this.Output
/// @type.symbol symbol=Add.T source="T = this" type=T

    type Output;

    add(this, other: T): this.Output;
    /// @type.symbol symbol=Add.add source="add(this, other: T): this.Output" type=(this: this, T) => this.Output
    /// @type.symbol symbol=Add.add.this source=this type=this
    /// @type.symbol symbol=Add.add.other source="other: T" type=T
    /// @resolution.name source=T target=Add.T
    /// @resolution.name source=this.Output target=Add.Output

}

export extension StringAdd of string implements Add<string> {
/// @definition.extension symbol=StringAdd form=exported target=string
/// @definition.implements symbol=StringAdd source=Add<string> target=Add<string>
/// @definition.associated.type symbol=StringAdd.Output source="type Output = ^string" key=Output value=^string
/// @definition.method symbol=StringAdd.add slot=add type=<StringAdd.add.'a>(this: &StringAdd.add.'a readonly string, string) => ^string
/// @definition.conformance symbol=StringAdd member=StringAdd.Output requirement=Add.Output
/// @definition.conformance symbol=StringAdd member=StringAdd.add requirement=Add.add
/// @resolution.name source=Add target=Add

    type Output = ^string;
    /// @type.symbol symbol=StringAdd.Output source="type Output = ^string" type=^string

    add(&readonly this, other: string): ^string {
    /// @generic.template symbol=StringAdd.add parent=template#1 parameters=('a)
    /// @type.symbol symbol=StringAdd.add type=<StringAdd.add.'a>(this: &StringAdd.add.'a readonly string, string) => ^string
    /// @type.symbol symbol=StringAdd.add.this source="&readonly this" type=&StringAdd.add.'a readonly string
    /// @type.symbol symbol=StringAdd.add.other source="other: string" type=string

        todo()
        /// @resolution.name source=todo target=todo
        /// @resolution.call source=todo() parameters=() return=never kind=symbol target=todo

    }
}

declare function todo(): never;
/// @type.symbol symbol=todo source="declare function todo(): never" type=() => never
"#,
        r#"
"#,
    );
}

/// A primitive's extension implements an imported operator interface through a readonly receiver.
#[test]
fn test_implement_an_imported_operator_interface_for_a_primitive() {
    let session = TestSession::builder()
        .module(
            "ops.ds",
            r#"
export newtype interface Add<T = this> {
    type Output;

    add(this, other: T): this.Output;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Add } from "./ops.ds";

export extension StringAdd of string implements Add<string> {
    type Output = ^string;

    add(&readonly this, other: string): ^string {
        todo()
    }
}

declare function todo(): never;
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Add } from "./ops.ds";

export extension StringAdd of string implements Add<string> {
    type Output = ^string;

    add(&readonly this, other: string): ^string {
        todo()
    }
}

declare function todo(): never;

=== dir ===
import { Add } from "./ops.ds";

export extension StringAdd of string implements Add<string> {
/// @definition.extension symbol=StringAdd form=exported target=string
/// @definition.implements symbol=StringAdd source=Add<string> target=ops.Add<string>
/// @definition.associated.type symbol=StringAdd.Output source="type Output = ^string" key=Output value=^string
/// @definition.method symbol=StringAdd.add slot=add type=<StringAdd.add.'a>(this: &StringAdd.add.'a readonly string, string) => ^string
/// @definition.conformance symbol=StringAdd member=StringAdd.Output requirement=ops.Add.Output
/// @definition.conformance symbol=StringAdd member=StringAdd.add requirement=ops.Add.add
/// @resolution.name source=Add target=ops.Add

    type Output = ^string;
    /// @type.symbol symbol=StringAdd.Output source="type Output = ^string" type=^string

    add(&readonly this, other: string): ^string {
    /// @generic.template symbol=StringAdd.add parent=template#0 parameters=('a)
    /// @type.symbol symbol=StringAdd.add type=<StringAdd.add.'a>(this: &StringAdd.add.'a readonly string, string) => ^string
    /// @type.symbol symbol=StringAdd.add.this source="&readonly this" type=&StringAdd.add.'a readonly string
    /// @type.symbol symbol=StringAdd.add.other source="other: string" type=string

        todo()
        /// @resolution.name source=todo target=todo
        /// @resolution.call source=todo() parameters=() return=never kind=symbol target=todo

    }
}

declare function todo(): never;
/// @type.symbol symbol=todo source="declare function todo(): never" type=() => never
"#,
        r#"
"#,
    );
}

/// A class implements an iteration interface with a defaulted associated iterator type.
#[test]
fn test_implement_an_iterable_interface_with_a_defaulted_iterator_type() {
    let session = TestSession::single(
        r#"
newtype interface Iterator<T> {
    next(&this): T | undefined;
}

newtype interface Iterable<T> {
    type Iterator: Iterator<T> = Iterator<T>;

    iterator(this): this.Iterator;
}

struct Key {
    index: uint32;
}

class Slab<T> {
    entries: T[] = [];
}

export extension<T> of Slab<T> implements Iterable<(Key, T)> {
    iterator(this): this.Iterator {
        todo()
    }
}

export extension<T, 'a> of &'a Slab<T> implements Iterable<(Key, &'a T)> {
    iterator(this): this.Iterator {
        todo()
    }
}

declare function todo(): never;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Iterator<out T> {
    next(&this): T | undefined;
}

newtype interface Iterable<out T> {
    type Iterator: Iterator<T> = Iterator<T>;

    iterator(this): this.Iterator;
}

struct Key {
    index: uint32;
}

class Slab<in out T> {
    entries: T[] = [];
}

export extension<T> of Slab<T> implements Iterable<(Key, T)> {
    iterator(this): Iterator<(Key, T)> {
        todo()
    }
}

export extension<T, 'a> of &'a Slab<T> implements Iterable<(Key, &'a T)> {
    iterator(this): Iterator<(Key, &'a T)> {
        todo()
    }
}

declare function todo(): never;

=== dir ===
newtype interface Iterator<T> {
/// @generic.template symbol=Iterator parameters=(out T#1, this: Iterator<T#1>)
/// @type.symbol symbol=Iterator type=Iterator
/// @definition.interface symbol=Iterator template=(out T#1, this: Iterator<T#1>) nominal=true
/// @definition.where symbol=Iterator relation=satisfies left=this right=Iterator<T#1>
/// @definition.method symbol=Iterator.next source="next(&this): T | undefined" slot=next type=<Iterator.next.'a>(this: &Iterator.next.'a this) => T#1 | undefined
/// @type.symbol symbol=Iterator.T source=T type=T#1

    next(&this): T | undefined;
    /// @generic.template symbol=Iterator.next parent=template#0 parameters=('a)
    /// @type.symbol symbol=Iterator.next source="next(&this): T | undefined" type=<Iterator.next.'a>(this: &Iterator.next.'a this) => T#1 | undefined
    /// @type.symbol symbol=Iterator.next.this source=&this type=&Iterator.next.'a this
    /// @resolution.name source=T target=Iterator.T

}

newtype interface Iterable<T> {
/// @generic.template symbol=Iterable parameters=(out T#2, this: Iterable<T#2>)
/// @type.symbol symbol=Iterable type=Iterable
/// @definition.interface symbol=Iterable template=(out T#2, this: Iterable<T#2>) nominal=true
/// @definition.where symbol=Iterable relation=satisfies left=this right=Iterable<T#2>
/// @definition.associated.type symbol=Iterable.Iterator source="type Iterator: Iterator<T> = Iterator<T>" key=Iterator constraint=Iterator<T#2> value=Iterator<T#2>
/// @definition.method symbol=Iterable.iterator source="iterator(this): this.Iterator" slot=iterator type=(this: this) => this.Iterator
/// @type.symbol symbol=Iterable.T source=T type=T#2

    type Iterator: Iterator<T> = Iterator<T>;
    /// @type.symbol symbol=Iterable.Iterator source="type Iterator: Iterator<T> = Iterator<T>" type=Iterator<T#2>
    /// @resolution.name source=Iterator target=Iterator
    /// @resolution.name source=T target=Iterable.T
    /// @resolution.name source=Iterator target=Iterator
    /// @resolution.name source=T target=Iterable.T

    iterator(this): this.Iterator;
    /// @type.symbol symbol=Iterable.iterator source="iterator(this): this.Iterator" type=(this: this) => this.Iterator
    /// @type.symbol symbol=Iterable.iterator.this source=this type=this
    /// @resolution.name source=this.Iterator target=Iterable.Iterator

}

struct Key {
/// @type.symbol symbol=Key type=Key
/// @definition.struct symbol=Key
/// @definition.field symbol=Key.index source="index: uint32" key=index type=uint32

    index: uint32;
    /// @type.symbol symbol=Key.index source="index: uint32" type=uint32

}

class Slab<T> {
/// @generic.template symbol=Slab parameters=(in out T#3)
/// @type.symbol symbol=Slab type=typeof Slab
/// @definition.class symbol=Slab template=(in out T#3)
/// @definition.field symbol=Slab.entries source="entries: T[] = []" key=entries type=T#3[]
/// @type.symbol symbol=Slab.T source=T type=T#3

    entries: T[] = [];
    /// @type.symbol symbol=Slab.entries source="entries: T[] = []" type=T#3[]
    /// @resolution.name source=T target=Slab.T
    /// @resolution.call source=[] parameters=(^Slice<T#3>) arguments=(rest() as T#3) return=T#3[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<T#3>
    /// @generic.instantiation id=arrayFromOwnedSlice<T#3> template=arrayFromOwnedSlice arguments=(T#3) owner=Slab

}

export extension<T> of Slab<T> implements Iterable<(Key, T)> {
/// @generic.template symbol=<module>#2 parameters=(T#4)
/// @definition.extension symbol=<module>#2 form=exported target=Slab<T#4>
/// @definition.implements symbol=<module>#2 source="Iterable<(Key, T)>" target="Iterable<(Key, T#4)>"
/// @definition.method symbol=iterator#1 slot=iterator type=(this: Slab<T#4>) => Iterator<(Key, T#4)>
/// @definition.conformance symbol=<module>#2 member=Iterable.Iterator requirement=Iterable.Iterator
/// @definition.conformance symbol=<module>#2 member=iterator#1 requirement=Iterable.iterator
/// @type.symbol symbol=T#1 source=T type=T#4
/// @resolution.name source=Slab target=Slab
/// @resolution.name source=T target=T#1
/// @resolution.name source=Iterable target=Iterable
/// @resolution.name source=Key target=Key
/// @resolution.name source=T target=T#1

    iterator(this): this.Iterator {
    /// @type.symbol symbol=iterator#1 type=(this: Slab<T#4>) => Iterator<(Key, T#4)>
    /// @type.symbol symbol=iterator.this#1 source=this type=Slab<T#4>
    /// @resolution.name source=this.Iterator target=Iterable.Iterator

        todo()
        /// @resolution.name source=todo target=todo
        /// @resolution.call source=todo() parameters=() return=never kind=symbol target=todo

    }
}

export extension<T, 'a> of &'a Slab<T> implements Iterable<(Key, &'a T)> {
/// @generic.template symbol=<module>#3 parameters=(T#5, 'a)
/// @definition.extension symbol=<module>#3 form=exported target=&'a Slab<T#5>
/// @definition.implements symbol=<module>#3 source="Iterable<(Key, &'a T)>" target="Iterable<(Key, &'a T#5)>"
/// @definition.method symbol=iterator#2 slot=iterator type=(this: &'a Slab<T#5>) => Iterator<(Key, &'a T#5)>
/// @definition.conformance symbol=<module>#3 member=Iterable.Iterator requirement=Iterable.Iterator
/// @definition.conformance symbol=<module>#3 member=iterator#2 requirement=Iterable.iterator
/// @type.symbol symbol=T#2 source=T type=T#5
/// @type.symbol symbol='a source='a type='a
/// @resolution.name source='a target='a
/// @resolution.name source=Slab target=Slab
/// @resolution.name source=T target=T#2
/// @resolution.name source=Iterable target=Iterable
/// @resolution.name source=Key target=Key
/// @resolution.name source='a target='a
/// @resolution.name source=T target=T#2

    iterator(this): this.Iterator {
    /// @type.symbol symbol=iterator#2 type=(this: &'a Slab<T#5>) => Iterator<(Key, &'a T#5)>
    /// @type.symbol symbol=iterator.this#2 source=this type=&'a Slab<T#5>
    /// @resolution.name source=this.Iterator target=Iterable.Iterator

        todo()
        /// @resolution.name source=todo target=todo
        /// @resolution.call source=todo() parameters=() return=never kind=symbol target=todo

    }
}

declare function todo(): never;
/// @type.symbol symbol=todo source="declare function todo(): never" type=() => never
"#,
        r#"
"#,
    );
}
