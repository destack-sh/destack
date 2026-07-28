use crate::tests::{DirRows, TestSession};

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

    session.assert_dir_checked(
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

=== checked ===
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
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point
/// @resolution.name source=Point target=Point
"#,
    );
}

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
struct Box<out T> {
    value: T;
}

function wrap<T>(value: T): Box<T> {
    Box<T> { value }
}

=== checked ===
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
/// @type.symbol symbol=wrap.T source=T type=T#2
/// @type.symbol symbol=wrap.value source="value: T" type=T#2
/// @resolution.name source=T target=wrap.T
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=wrap.T

    Box { value }
    /// @type.node source="Box { value }" type=Box<T#2>
    /// @resolution.name source=Box target=Box
    /// @generic.instance source="Box { value }" id=Box<T#2>
    /// @type.node source=value type=T#2
    /// @resolution.name source=value target=wrap.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=wrap.value

}

/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)

/// @check.stats.solve variables=0 types=11 constraints=0 obligations=2 solutions=0 bounds=0 decisions=6
"#,
    );
}

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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
newtype interface Zero {
/// @type.symbol symbol=Zero type=Zero
/// @definition.interface symbol=Zero nominal=true
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
    /// @resolution.call source=T.zero() parameters=() return=T#2 kind=symbol target=Zero.zero receiver=T#2

}

/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
"#,
        r#""#,
    );
}

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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
import { Float } from "destack:math";

struct Box<T: Float> {
/// @generic.template symbol=Box parameters=(out T#1: math.float.Float)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T#1: math.float.Float)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Box.T source="T: Float" type=T#1
/// @resolution.name source=Float target=math.float.Float

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

}

function doubled<T: Float>(value: T): Box<T> {
/// @generic.template symbol=doubled parameters=(T#2: math.float.Float)
/// @type.symbol symbol=doubled type=<T#2: math.float.Float>(T#2) => Box<T#2>
/// @type.symbol symbol=doubled.T source="T: Float" type=T#2
/// @resolution.name source=Float target=math.float.Float
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

/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
"#,
        r#""#,
    );
}

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

    session.assert_dir_checked(
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

const next: Counter = (Counter { value: 1 }).increment();
next satisfies Counter;

=== checked ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.increment slot=increment type=<Counter.increment.'a>(this: &Counter.increment.'a exclusive this) => Counter

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    increment(): Counter {
    /// @generic.template symbol=Counter.increment parameters=('a)
    /// @type.symbol symbol=Counter.increment type=<Counter.increment.'a>(this: &Counter.increment.'a exclusive this) => Counter
    /// @resolution.name source=Counter target=Counter

        Counter { value: this.value + 1 }
        /// @resolution.name source=Counter target=Counter
        /// @resolution.member source=this.value receiver=&Counter.increment.'a exclusive Counter type=int32 kind=field target_receiver=&Counter.increment.'a exclusive Counter key=value target=Counter.value target_type=int32
        /// @resolution.operator source="this.value + 1" type=int32 operator="+" kind=builtin operands=[this.value as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a exclusive Counter
        /// @resolution.place source=this placement="local" lifetime=Counter.increment.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=Counter.increment.'a access="exclusive"
        /// @resolution.access source=this.value root=this keys=[value]

    }
}

const next = Counter { value: 1 }.increment();
/// @type.symbol symbol=next source=next type=Counter
/// @resolution.pattern source=next kind=binding target=next
/// @resolution.name source=Counter target=Counter
/// @resolution.member source="Counter { value: 1 }.increment" receiver=Counter type=<Counter.increment.'a>(this: &Counter.increment.'a exclusive Counter) => Counter kind=symbol target_receiver=Counter target=Counter.increment
/// @resolution.call source="Counter { value: 1 }.increment()" parameters=() return=Counter kind=symbol target=Counter.increment receiver=Counter adjustments=(borrow(&'frame exclusive Counter))

next satisfies Counter;
/// @resolution.name source=next target=next
/// @resolution.place source=next placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=next root=next
/// @resolution.name source=Counter target=Counter
"#,
    );
}

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1 };

=== checked ===
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point: Point = Point { x: 1, y: 2, z: 3 };

=== checked ===
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

const point = new Point(1, 2);

=== checked ===
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
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error id=not-constructible message="type 'Point' cannot be constructed with 'new'; construct value types with 'T { … }'"
/// @diagnostic.label line=7 column=15 span="new Point(1, 2)" line_source="const point = new Point(1, 2);"
"#,
    );
}

#[test]
fn test_struct_methods_mutate_fields() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;

    increment(): int32 {
        this.value = this.value + 1;
        this.value
    }
}

let counter = Counter { value: 1 };
const next = counter.increment();
next satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Counter {
    value: int32;

    increment(): int32 {
        this.value = this.value + 1;
        this.value
    }
}

let counter: Counter = Counter { value: 1 };
const next: int32 = counter.increment();
next satisfies int32;

=== checked ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.increment slot=increment type=<Counter.increment.'a>(this: &Counter.increment.'a exclusive this) => int32

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    increment(): int32 {
    /// @generic.template symbol=Counter.increment parameters=('a)
    /// @type.symbol symbol=Counter.increment type=<Counter.increment.'a>(this: &Counter.increment.'a exclusive this) => int32

        this.value = this.value + 1;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a exclusive Counter
        /// @resolution.place source=this placement="local" lifetime=Counter.increment.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.assignment source=this.value write="receiver=&Counter.increment.'a exclusive Counter, target=field(receiver=&Counter.increment.'a exclusive Counter, target=Counter.value, type=int32), type=int32" type=int32
        /// @resolution.member source=this.value receiver=&Counter.increment.'a exclusive Counter type=int32 kind=field target_receiver=&Counter.increment.'a exclusive Counter key=value target=Counter.value target_type=int32
        /// @resolution.operator source="this.value + 1" type=int32 operator="+" kind=builtin operands=[this.value as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a exclusive Counter
        /// @resolution.place source=this placement="local" lifetime=Counter.increment.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=Counter.increment.'a access="exclusive"
        /// @resolution.access source=this.value root=this keys=[value]

        this.value
        /// @resolution.member source=this.value receiver=&Counter.increment.'a exclusive Counter type=int32 kind=field target_receiver=&Counter.increment.'a exclusive Counter key=value target=Counter.value target_type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.increment.'a exclusive Counter
        /// @resolution.place source=this placement="local" lifetime=Counter.increment.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement="local" lifetime=Counter.increment.'a access="exclusive"
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
/// @resolution.member source=counter.increment receiver=Counter type=<Counter.increment.'a>(this: &Counter.increment.'a exclusive Counter) => int32 kind=symbol target_receiver=Counter target=Counter.increment
/// @resolution.call source=counter.increment() parameters=() return=int32 kind=symbol target=Counter.increment receiver=Counter adjustments=(borrow(&'static exclusive Counter))
/// @resolution.place source=counter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=counter root=counter

next satisfies int32;
/// @resolution.name source=next target=next
/// @resolution.place source=next placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=next root=next
"#,
    );
}

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Counter {
    value: int32;
}

const counter: Counter = { value: 1 };

=== checked ===
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
/// @diagnostic.error id=not-assignable message="type '{ value: 1 }' is not assignable to type 'Counter'"
/// @diagnostic.label line=6 column=26 span="{ value: 1 }" line_source="const counter: Counter = { value: 1 };"
/// @diagnostic.related line=6 column=16 span="Counter" line_source="const counter: Counter = { value: 1 };" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_struct_literal_infers_omitted_borrow_lifetime_from_return() {
    let session = TestSession::single(
        r#"
type Options = {
    message?: string;
};

struct Entry {
    logger?: &readonly string;
    message?: string | undefined;
}

function make(options?: Options): Entry {
    const entry = Entry { message: options?.message };

    return entry;
}
"#,
    );

    session.assert_dir_checked(
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

function make(options?: Options): Entry<"static"> {
    const entry: Entry<"static"> = Entry<"static"> { message: options?.message };

    return entry;
}

=== checked ===
type Options = {
/// @type.symbol symbol=Options type={ message?: string }
/// @definition.type symbol=Options value={ message?: string }

    message?: string;
};

struct Entry {
/// @generic.template symbol=Entry parameters=('a)
/// @type.symbol symbol=Entry type=Entry
/// @definition.struct symbol=Entry template=('a)
/// @definition.field symbol=Entry.logger source="logger?: &readonly string" key=logger type=&Entry.'a readonly string
/// @definition.field symbol=Entry.message source="message?: string | undefined" key=message type=string | undefined

    logger?: &readonly string;
    /// @type.symbol symbol=Entry.logger source="logger?: &readonly string" type=&Entry.'a readonly string

    message?: string | undefined;
    /// @type.symbol symbol=Entry.message source="message?: string | undefined" type=string | undefined

}

function make(options?: Options): Entry {
/// @type.symbol symbol=make type=(Options | undefined?) => Entry<"static">
/// @type.symbol symbol=make.options source="options?: Options" type=Options | undefined
/// @resolution.name source=Options target=Options
/// @resolution.name source=Entry target=Entry

    const entry = Entry { message: options?.message };
    /// @type.symbol symbol=make.entry source=entry type=Entry<"static">
    /// @resolution.pattern source=entry kind=binding target=make.entry
    /// @resolution.name source=Entry target=Entry
    /// @resolution.name source=options target=make.options
    /// @resolution.member source=options?.message receiver={ message?: string } type=string | undefined kind=field target_receiver={ message?: string } key=message target_type=string | undefined
    /// @resolution.place source=options placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=options root=make.options
    /// @resolution.place source=options?.message placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=options?.message root=make.options keys=[message]
    /// @resolution.access source=options?.message root=make.options keys=[message]

    return entry;
    /// @resolution.name source=entry target=make.entry
    /// @resolution.place source=entry placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=entry root=make.entry

}

/// @generic.instance id="Entry<\"static\">" template=Entry arguments=("static")
"#,
    );
}
