use crate::tests::{DirRows, TestSession};

/// A tagged struct literal constructs a value of that struct.
#[test]
fn test_struct_tagged_literal_constructs_value() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
point satisfies Point;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1, y: 2 };
point satisfies Point;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

const point = Point { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

point satisfies Point;
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point
/// @resolution.name source=Point target=Point
"#,
    );
}

/// Reject accessors because struct literals initialize stored fields.
#[test]
fn test_struct_literal_rejects_accessors() {
    let session = TestSession::single(
        r#"
struct Store {
    read: () => string;
    write: (value: string) => void;
}

const store = Store {
    get read(): string { return "ready"; },
    set write(value: string): void {},
};
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Store {
    read: () => string;
    write: (value: string) => void;
}

const store: Store = Store {
    get read(): string {
        return "ready";
    },
    set write(value: string): void {},
};

=== dir ===
struct Store {
/// @type.symbol symbol=Store type=Store
/// @definition.struct symbol=Store
/// @definition.field symbol=Store.read source="read: () => string" key=read type=() => string
/// @definition.field symbol=Store.write source="write: (value: string) => void" key=write type=(string) => void

    read: () => string;
    /// @type.symbol symbol=Store.read source="read: () => string" type=() => string

    write: (value: string) => void;
    /// @type.symbol symbol=Store.write source="write: (value: string) => void" type=(string) => void
    /// @type.symbol symbol=Store.value source="value: string" type=string

}

const store = Store {
/// @type.symbol symbol=store source=store type=Store
/// @resolution.pattern source=store kind=binding target=store
/// @resolution.name source=Store target=Store

    get read(): string { return "ready"; },
    /// @type.symbol symbol=symbol7 source="get read(): string { return \"ready\"; }" type=() => string

    set write(value: string): void {},
    /// @type.symbol symbol=symbol8 source="set write(value: string): void {}" type=(string) => void

};
"#,
        r#"
/// @diagnostic.error id=invalid-struct-accessor message="accessors are not valid in struct literals"
/// @diagnostic.label line=8 column=9 span="read" line_source="get read(): string { return \"ready\"; },"
/// @diagnostic.error id=invalid-struct-accessor message="accessors are not valid in struct literals"
/// @diagnostic.label line=9 column=9 span="write" line_source="set write(value: string): void {},"
"#,
    );
}

/// A generic struct literal takes its arguments from the expected result type.
#[test]
fn test_generic_struct_literal_uses_expected_result_arguments() {
    let session = TestSession::single(
        r#"
struct Box<T> {
    value: T;
}

function wrap<T>(value: T): Box<T> {
    Box { value }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Box<out T> {
    value: T;
}

function wrap<T>(value: T): Box<T> {
    Box<T> { value }
}

=== dir ===
struct Box<T> {
/// @generic.template symbol=Box parameters=(out T#1)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T#1)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Box.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

}

function wrap<T>(value: T): Box<T> {
/// @generic.template symbol=wrap parameters=(T#2)
/// @type.symbol symbol=wrap type=<T#2>(T#2) => Box<T#2>
/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
/// @type.symbol symbol=wrap.T source=T type=T#2
/// @type.symbol symbol=wrap.value source="value: T" type=T#2
/// @resolution.name source=T target=wrap.T
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=wrap.T

    Box { value }
    /// @type.node source="Box { value }" type=Box<T#2>
    /// @resolution.name source=Box target=Box
    /// @type.node source=value type=T#2
    /// @resolution.name source=value target=wrap.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=wrap.value

}
"#,
    );
}

/// A generic struct literal checks a field initialized by a static bound member.
#[test]
fn test_generic_struct_literal_checks_field_with_static_bound_member() {
    let session = TestSession::single(
        r#"
newtype interface Zero {
    static zero(): this;
}

struct Box<T: Zero> {
    value: T;
}

function make<T: Zero>(): Box<T> {
    Box { value: T.zero() }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Zero {
    static zero(): this;
}

struct Box<out T: Zero> {
    value: T;
}

function make<T: Zero>(): Box<T> {
    Box<T> { value: T.zero() }
}

=== dir ===
newtype interface Zero {
/// @generic.template symbol=Zero parameters=(this: Zero)
/// @type.symbol symbol=Zero type=Zero
/// @definition.interface symbol=Zero template=(this: Zero) nominal=true
/// @definition.where symbol=Zero relation=satisfies left=this right=Zero
/// @definition.method symbol=Zero.zero source="static zero(): this" slot=zero static=true type=() => this

    static zero(): this;
    /// @type.symbol symbol=Zero.zero source="static zero(): this" type=() => this

}

struct Box<T: Zero> {
/// @generic.template symbol=Box parameters=(out T#1: Zero)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T#1: Zero)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Box.T source="T: Zero" type=T#1
/// @resolution.name source=Zero target=Zero

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

}

function make<T: Zero>(): Box<T> {
/// @generic.template symbol=make parameters=(T#2: Zero)
/// @type.symbol symbol=make type=<T#2: Zero>() => Box<T#2>
/// @type.symbol symbol=make.T source="T: Zero" type=T#2
/// @resolution.name source=Zero target=Zero
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=make.T

    Box { value: T.zero() }
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=T target=make.T
    /// @resolution.member source=T.zero receiver=T#2 type=() => T#2 kind=symbol target_receiver=T#2 target=Zero.zero
    /// @resolution.call source=T.zero() parameters=() return=T#2 kind=symbol target=Zero.zero
    /// @generic.instantiation id=Zero.zero<T#2> template=Zero.zero arguments=() owner=make

}
"#,
        r#"
"#,
    );
}

/// A generic struct literal checks a field initialized by a scalar operator.
#[test]
fn test_generic_struct_literal_checks_field_with_scalar_operator() {
    let session = TestSession::single(
        r#"
import { Float } from "destack:math";

struct Box<T: Float> {
    value: T;
}

function doubled<T: Float>(value: T): Box<T> {
    Box { value: value + value }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Float } from "destack:math";

struct Box<out T: Float> {
    value: T;
}

function doubled<T: Float>(value: T): Box<T> {
    Box<T> { value: value + value }
}

=== dir ===
import { Float } from "destack:math";

struct Box<T: Float> {
/// @generic.template symbol=Box parameters=(out T#1: Float)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T#1: Float)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Box.T source="T: Float" type=T#1
/// @resolution.name source=Float target=Float

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

}

function doubled<T: Float>(value: T): Box<T> {
/// @generic.template symbol=doubled parameters=(T#2: Float)
/// @type.symbol symbol=doubled type=<T#2: Float>(T#2) => Box<T#2>
/// @type.symbol symbol=doubled.T source="T: Float" type=T#2
/// @resolution.name source=Float target=Float
/// @type.symbol symbol=doubled.value source="value: T" type=T#2
/// @resolution.name source=T target=doubled.T
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=doubled.T

    Box { value: value + value }
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=value target=doubled.value
    /// @resolution.operator source="value + value" type=T#2 operator="+" kind=builtin operands=[value as T#2 families=(float), value as T#2 families=(float)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=doubled.value
    /// @resolution.name source=value target=doubled.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=doubled.value

}
"#,
        r#"
"#,
    );
}

/// A tagged struct literal exposes the methods the struct declares.
#[test]
fn test_struct_tagged_literal_exposes_methods() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;

    increment(): Counter {
        Counter { value: this.value + 1 }
    }
}

const next = Counter { value: 1 }.increment();
next satisfies Counter;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Counter {
    value: int32;

    increment(): Counter {
        Counter { value: this.value + 1 }
    }
}

const next: Counter = Counter { value: 1 }.increment<"frame">();
next satisfies Counter;

=== dir ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.increment slot=increment type=<Counter.increment.'a>(this: &Counter.increment.'a readonly Counter) => Counter

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    increment(): Counter {
    /// @generic.template symbol=Counter.increment parameters=('a)
    /// @type.symbol symbol=Counter.increment type=<Counter.increment.'a>(this: &Counter.increment.'a readonly Counter) => Counter
    /// @type.symbol symbol=Counter.increment.this type=&Counter.increment.'a readonly Counter
    /// @resolution.name source=Counter target=Counter

        Counter { value: this.value + 1 }
        /// @resolution.name source=Counter target=Counter
        /// @resolution.member source=this.value receiver=&Counter.increment.'a readonly Counter type=int32 kind=field target_receiver=&Counter.increment.'a readonly Counter key=value target=Counter.value target_type=int32
        /// @resolution.operator source="this.value + 1" type=int32 operator="+" kind=builtin operands=[this.value as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a readonly Counter
        /// @resolution.place source=this placement=Counter.increment.'a lifetime=Counter.increment.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=Counter.increment.'a lifetime=Counter.increment.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}

const next = Counter { value: 1 }.increment();
/// @type.symbol symbol=next source=next type=Counter
/// @resolution.pattern source=next kind=binding target=next
/// @resolution.name source=Counter target=Counter
/// @resolution.member source="Counter { value: 1 }.increment" receiver=Counter type=<Counter.increment.'a>(this: &Counter.increment.'a readonly Counter) => Counter kind=symbol target_receiver=Counter target=Counter.increment
/// @resolution.call source="Counter { value: 1 }.increment()" parameters=() return=Counter regions=("frame" & "local") kind=symbol target=Counter.increment receiver=Counter adjustments=(borrow(&'frame readonly Counter)) instance="Counter.increment<\"frame\" & \"local\">"
/// @generic.instantiation id="Counter.increment<\"frame\" & \"local\">" template=Counter.increment arguments=("frame" & "local")
/// @generic.instance id="Counter.increment<\"bound0\" & \"local\">" template=Counter.increment arguments=("bound0" & "local")

next satisfies Counter;
/// @resolution.name source=next target=next
/// @resolution.place source=next placement="local" lifetime="static" access="immutable"
/// @resolution.access source=next root=next
/// @resolution.name source=Counter target=Counter
"#,
    );
}

/// A tagged struct literal missing a field reports a diagnostic.
#[test]
fn test_struct_tagged_literal_requires_fields() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1 };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1 };

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

const point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error id=missing-required-property message="missing required property 'y' for type 'Point'"
/// @diagnostic.label line=7 column=15 span="Point { x: 1 }" line_source="const point = Point { x: 1 };"
"#,
    );
}

/// A tagged struct literal with an extra field reports a diagnostic.
#[test]
fn test_struct_tagged_literal_rejects_extra_fields() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2, z: 3 };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1, y: 2, z: 3 };

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

const point = Point { x: 1, y: 2, z: 3 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error id=excess-property message="unknown property 'z' in object literal for type 'Point'"
/// @diagnostic.label line=7 column=15 span="Point { x: 1, y: 2, z: 3 }" line_source="const point = Point { x: 1, y: 2, z: 3 };"
/// @diagnostic.note message="object literals may only specify known properties"
"#,
    );
}

/// Constructing a struct with new reports a diagnostic.
#[test]
fn test_struct_rejects_new_constructor_syntax() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

const point = new Point(1, 2);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point = new Point(1, 2);

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

const point = new Point(1, 2);
/// @type.symbol symbol=point source=point type=<error>
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.rejected source="new Point(1, 2)"
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error id=invalid-value-reference message="'Point' is not a value"
/// @diagnostic.label line=7 column=19 span="Point" line_source="const point = new Point(1, 2);"
/// @diagnostic.help message="construct structs with 'T { … }'"
"#,
    );
}

/// An exclusive struct method writes the fields of its receiver.
#[test]
fn test_struct_methods_mutate_fields() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;

    increment(&this): int32 {
        this.value = this.value + 1;
        this.value
    }
}

let counter = Counter { value: 1 };
const next = counter.increment();
next satisfies int32;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Counter {
    value: int32;

    increment(&this): int32 {
        this.value = this.value + 1;
        this.value
    }
}

let counter: Counter = Counter { value: 1 };
const next: int32 = counter.increment<"static">();
next satisfies int32;

=== dir ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.increment slot=increment type=<Counter.increment.'a>(this: &Counter.increment.'a Counter) => int32

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    increment(&this): int32 {
    /// @generic.template symbol=Counter.increment parameters=('a)
    /// @type.symbol symbol=Counter.increment type=<Counter.increment.'a>(this: &Counter.increment.'a Counter) => int32
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

        this.value
        /// @resolution.member source=this.value receiver=&Counter.increment.'a Counter type=int32 kind=field target_receiver=&Counter.increment.'a Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a Counter
        /// @resolution.place source=this placement=Counter.increment.'a lifetime=Counter.increment.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=Counter.increment.'a lifetime=Counter.increment.'a access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}

let counter = Counter { value: 1 };
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=Counter target=Counter

const next = counter.increment();
/// @type.symbol symbol=next source=next type=int32
/// @resolution.pattern source=next kind=binding target=next
/// @resolution.name source=counter target=counter
/// @resolution.member source=counter.increment receiver=Counter type=<Counter.increment.'a>(this: &Counter.increment.'a Counter) => int32 kind=symbol target_receiver=Counter target=Counter.increment
/// @resolution.call source=counter.increment() parameters=() return=int32 regions=("static" & "local") kind=symbol target=Counter.increment receiver=Counter adjustments=(borrow(&'static Counter)) instance="Counter.increment<\"static\" & \"local\">"
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter
/// @generic.instantiation id="Counter.increment<\"static\" & \"local\">" template=Counter.increment arguments=("static" & "local")
/// @generic.instance id="Counter.increment<\"bound0\" & \"local\">" template=Counter.increment arguments=("bound0" & "local")

next satisfies int32;
/// @resolution.name source=next target=next
/// @resolution.place source=next placement="local" lifetime="static" access="immutable"
/// @resolution.access source=next root=next
"#,
    );
}

/// Assigning a bare object literal to a struct reports a diagnostic.
#[test]
fn test_object_literal_does_not_construct_struct() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;
}

const counter: Counter = { value: 1 };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Counter {
    value: int32;
}

const counter: Counter = { value: 1 };

=== dir ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

}

const counter: Counter = { value: 1 };
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.pattern source=counter kind=binding target=counter
/// @resolution.name source=Counter target=Counter
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ value: int32 }' is not assignable to type 'Counter'"
/// @diagnostic.label line=6 column=26 span="{ value: 1 }" line_source="const counter: Counter = { value: 1 };"
/// @diagnostic.related line=6 column=16 span="Counter" line_source="const counter: Counter = { value: 1 };" message="expected due to this annotation"
"#,
    );
}

/// A struct literal infers the borrow lifetime it omits from the return type.
#[test]
fn test_struct_literal_infers_omitted_borrow_lifetime_from_return() {
    let session = TestSession::single(
        r#"
type Options = {
    message?: string;
};

struct Entry<'a> {
    logger?: &'a readonly string;
    message?: string | undefined;
}

function make(options?: Options): Entry {
    const entry = Entry { message: options?.message };

    return entry;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Options = {
    message?: string;
};

struct Entry<'a> {
    logger?: &'a readonly string;
    message?: string | undefined;
}

function make(options?: { message?: string }): Entry<"managed"> {
    const entry: Entry<"managed"> = Entry<"managed"> { message: options?.message };

    return entry;
}

=== dir ===
type Options = {
/// @type.symbol symbol=Options type={ message?: string }
/// @definition.type symbol=Options value={ message?: string }

    message?: string;
    /// @type.symbol symbol=Options.message source="message?: string" type=string

};

struct Entry<'a> {
/// @generic.template symbol=Entry parameters=('a)
/// @type.symbol symbol=Entry type=Entry
/// @definition.struct symbol=Entry template=('a)
/// @definition.field symbol=Entry.logger source="logger?: &'a readonly string" key=logger type=&'a readonly string
/// @definition.field symbol=Entry.message source="message?: string | undefined" key=message type=string | undefined
/// @type.symbol symbol=Entry.'a source='a type='a

    logger?: &'a readonly string;
    /// @type.symbol symbol=Entry.logger source="logger?: &'a readonly string" type=&'a readonly string
    /// @resolution.name source='a target=Entry.'a

    message?: string | undefined;
    /// @type.symbol symbol=Entry.message source="message?: string | undefined" type=string | undefined

}

function make(options?: Options): Entry {
/// @type.symbol symbol=make type=({ message?: string } | undefined?) => Entry<"managed" & "local">
/// @generic.instance id="Entry<\"bound0\" & \"local\">" template=Entry arguments=("bound0" & "local")
/// @type.symbol symbol=make.options source="options?: Options" type={ message?: string } | undefined
/// @resolution.name source=Options target=Options
/// @resolution.name source=Entry target=Entry
/// @generic.instance id="Entry<\"frame\">" template=Entry arguments=("frame")

    const entry = Entry { message: options?.message };
    /// @type.symbol symbol=make.entry source=entry type=Entry<"managed" & "local">
    /// @resolution.pattern source=entry kind=binding target=make.entry
    /// @resolution.name source=Entry target=Entry
    /// @resolution.name source=options target=make.options
    /// @resolution.member source=options?.message receiver={ message?: string } | undefined type=string | undefined kind=field target_receiver={ message?: string } | undefined adjustments=(union.payload({ message?: string } | undefined, { message?: string }, { message?: string })) key=message target_type=string | undefined
    /// @resolution.place source=options placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=options root=make.options
    /// @resolution.place source=options?.message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=options?.message root=make.options keys=[message]

    return entry;
    /// @resolution.name source=entry target=make.entry
    /// @resolution.place source=entry placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=entry root=make.entry

}
"#,
    );
}

/// Report a mismatched struct field once, without a second report at the enclosing return.
#[test]
fn test_report_a_mismatched_struct_field_once() {
    let session = TestSession::single(
        r#"
struct Expectation {
    message?: ^string;

    get not(this): Expectation {
        Expectation { message: this.message }
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Expectation {
    message?: ^string;

    get not(this): Expectation {
        Expectation { message: this.message }
    }
}

=== dir ===
struct Expectation {
/// @type.symbol symbol=Expectation type=Expectation
/// @definition.struct symbol=Expectation
/// @definition.field symbol=Expectation.message source="message?: ^string" key=message type=^string
/// @definition.method symbol=Expectation.not slot=not role=getter type=(this: Expectation) => Expectation

    message?: ^string;
    /// @type.symbol symbol=Expectation.message source="message?: ^string" type=^string

    get not(this): Expectation {
    /// @type.symbol symbol=Expectation.not type=(this: Expectation) => Expectation
    /// @type.symbol symbol=Expectation.not.this source=this type=Expectation
    /// @resolution.name source=Expectation target=Expectation

        Expectation { message: this.message }
        /// @resolution.name source=Expectation target=Expectation
        /// @resolution.member source=this.message receiver=Expectation type=^string | undefined kind=field target_receiver=Expectation key=message target=Expectation.message target_type=^string | undefined
        /// @resolution.receiver source=this kind=this declaration=Expectation type=Expectation
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.message placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.message root=this keys=[message]

    }
}
"#, r#"
/// @diagnostic.error id=not-assignable message="type '^string | undefined' is not assignable to type '^string'"
/// @diagnostic.label line=6 column=32 span="this.message" line_source="Expectation { message: this.message }"
/// @diagnostic.related line=6 column=9 span="Expectation { message: this.message }" line_source="Expectation { message: this.message }" message="expected due to the type of this target"
/// @diagnostic.note message="the mismatch is in field 'message': expected '^string', found 'undefined'"
"#);
}

/// Report a missing struct field alongside a mismatched one.
#[test]
fn test_report_a_missing_field_alongside_a_mismatched_field() {
    let session = TestSession::single(
        r#"
struct Pair {
    left: int32;
    right: int32;
}

function pair(): Pair {
    return Pair { left: "one" };
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Pair {
    left: int32;
    right: int32;
}

function pair(): Pair {
    return Pair { left: "one" };
}

=== dir ===
struct Pair {
/// @type.symbol symbol=Pair type=Pair
/// @definition.struct symbol=Pair
/// @definition.field symbol=Pair.left source="left: int32" key=left type=int32
/// @definition.field symbol=Pair.right source="right: int32" key=right type=int32

    left: int32;
    /// @type.symbol symbol=Pair.left source="left: int32" type=int32

    right: int32;
    /// @type.symbol symbol=Pair.right source="right: int32" type=int32

}

function pair(): Pair {
/// @type.symbol symbol=pair type=() => Pair
/// @resolution.name source=Pair target=Pair

    return Pair { left: "one" };
    /// @resolution.name source=Pair target=Pair

}
"#, r#"
/// @diagnostic.error id=missing-required-property message="missing required property 'right' for type 'Pair'"
/// @diagnostic.label line=8 column=12 span="Pair { left: \"one\" }" line_source="return Pair { left: \"one\" };"
/// @diagnostic.error id=not-assignable message="type '\"one\"' is not assignable to type 'int32'"
/// @diagnostic.label line=8 column=25 span="\"one\"" line_source="return Pair { left: \"one\" };"
/// @diagnostic.related line=8 column=12 span="Pair { left: \"one\" }" line_source="return Pair { left: \"one\" };" message="expected due to the type of this target"
/// @diagnostic.note message="the mismatch is in field 'left'"
"#);
}
