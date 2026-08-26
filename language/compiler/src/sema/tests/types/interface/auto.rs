use crate::tests::{DirRows, TestSession};

/// A plain struct derives the auto conformance members and satisfies their bounds.
#[test]
fn test_derive_the_auto_conformance_members_on_a_plain_struct() {
    let session = TestSession::single(
        r#"
import { Clone, Default } from "destack:memory";
import { Hash, Equal } from "destack:ops";

struct Point {
    x: int32;
    y: int32;
}

declare function requireClone<T: Clone>(value: T): T;
declare function requireDefault<T: Default>(): T;
declare function requireHash<T: Hash>(value: T): T;
declare function requireEqual<T: Equal<T>>(value: T): T;

const point = Point { x: 1, y: 2 };
const cloned = point.clone();
const defaulted = Point.default();
const same = point.equal(cloned);
const viaClone = requireClone(point);
const viaDefault = requireDefault<Point>();
const viaHash = requireHash(point);
const viaEqual = requireEqual(point);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Clone, Default } from "destack:memory";
import { Equal, Hash } from "destack:ops";

struct Point {
    x: int32;
    y: int32;
}

declare function requireClone<T: Clone>(value: T): T;
declare function requireDefault<T: Default>(): T;
declare function requireHash<T: Hash>(value: T): T;
declare function requireEqual<T: Equal<T>>(value: T): T;

const point: Point = Point { x: 1, y: 2 };
const cloned: Point = point.clone<"constant">();
const defaulted: Point = Point.default();
const same: boolean = point.equal<Point, "constant", "constant">(cloned as &'static readonly Point);
const viaClone: Point = requireClone<Point>(point);
const viaDefault: Point = requireDefault<Point>();
const viaHash: Point = requireHash<Point>(point);
const viaEqual: Point = requireEqual<Point>(point);

=== dir ===
import { Clone, Default } from "destack:memory";
import { Hash, Equal } from "destack:ops";

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

declare function requireClone<T: Clone>(value: T): T;
/// @generic.template symbol=requireClone parameters=(T#1: Clone)
/// @type.symbol symbol=requireClone source="declare function requireClone<T: Clone>(value: T): T" type=<T#1: Clone>(T#1) => T#1
/// @type.symbol symbol=requireClone.T source="T: Clone" type=T#1
/// @resolution.name source=Clone target=Clone
/// @type.symbol symbol=requireClone.value source="value: T" type=T#1
/// @resolution.name source=T target=requireClone.T
/// @resolution.name source=T target=requireClone.T

declare function requireDefault<T: Default>(): T;
/// @generic.template symbol=requireDefault parameters=(T#2: Default)
/// @type.symbol symbol=requireDefault source="declare function requireDefault<T: Default>(): T" type=<T#2: Default>() => T#2
/// @type.symbol symbol=requireDefault.T source="T: Default" type=T#2
/// @resolution.name source=Default target=Default
/// @resolution.name source=T target=requireDefault.T

declare function requireHash<T: Hash>(value: T): T;
/// @generic.template symbol=requireHash parameters=(T#3: Hash)
/// @type.symbol symbol=requireHash source="declare function requireHash<T: Hash>(value: T): T" type=<T#3: Hash>(T#3) => T#3
/// @type.symbol symbol=requireHash.T source="T: Hash" type=T#3
/// @resolution.name source=Hash target=Hash
/// @type.symbol symbol=requireHash.value source="value: T" type=T#3
/// @resolution.name source=T target=requireHash.T
/// @resolution.name source=T target=requireHash.T

declare function requireEqual<T: Equal<T>>(value: T): T;
/// @generic.template symbol=requireEqual parameters=(T#4: Equal<T#4>)
/// @type.symbol symbol=requireEqual source="declare function requireEqual<T: Equal<T>>(value: T): T" type=<T#4: Equal<T#4>>(T#4) => T#4
/// @type.symbol symbol=requireEqual.T source="T: Equal<T>" type=T#4
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=T target=requireEqual.T
/// @type.symbol symbol=requireEqual.value source="value: T" type=T#4
/// @resolution.name source=T target=requireEqual.T
/// @resolution.name source=T target=requireEqual.T

const point = Point { x: 1, y: 2 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

const cloned = point.clone();
/// @type.symbol symbol=cloned source=cloned type=Point
/// @resolution.pattern source=cloned kind=binding target=cloned
/// @resolution.name source=point target=point
/// @resolution.member source=point.clone receiver=Point type=<Clone.clone.'a, Clone.clone.P1: Place>(this: Borrowed<Point, Clone.clone.'a & Clone.clone.P1, "readonly">) => Owned<Point> kind=symbol target_receiver=Point target=Clone.clone
/// @resolution.call source=point.clone() parameters=() return=Owned<Point> kind=symbol target=Clone.clone receiver=Point adjustments=(borrow(&'static readonly constant Point)) instance="Clone.clone<\"constant\">"
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point
/// @generic.instantiation id="Clone.clone<Point, \"constant\">" template=Clone.clone arguments=("constant")

const defaulted = Point.default();
/// @type.symbol symbol=defaulted source=defaulted type=Point
/// @resolution.pattern source=defaulted kind=binding target=defaulted
/// @resolution.name source=Point target=Point
/// @resolution.member source=Point.default receiver=Point type=() => Owned<Point> kind=symbol target_receiver=Point target=Default.default
/// @resolution.call source=Point.default() parameters=() return=Owned<Point> kind=symbol target=Default.default

const same = point.equal(cloned);
/// @type.symbol symbol=same source=same type=boolean
/// @resolution.pattern source=same kind=binding target=same
/// @resolution.name source=point target=point
/// @resolution.member source=point.equal receiver=Point type=<PartialEqual.equal.'a, PartialEqual.equal.P1: Place, PartialEqual.equal.'b, PartialEqual.equal.P3: Place>(this: Borrowed<Point, PartialEqual.equal.'a & PartialEqual.equal.P1, "readonly">, Borrowed<Point, PartialEqual.equal.'b & PartialEqual.equal.P3, "readonly">) => boolean kind=symbol target_receiver=Point target=PartialEqual.equal
/// @resolution.call source=point.equal(cloned) parameters=(&'static readonly constant Point) arguments=(provided(cloned) as &'static readonly constant Point) return=boolean kind=symbol target=PartialEqual.equal receiver=Point adjustments=(borrow(&'static readonly constant Point)) instance="PartialEqual<Point>.equal<\"constant\", \"constant\">"
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point
/// @generic.instantiation id="PartialEqual.equal<Point, Point, \"constant\", \"constant\">" template=PartialEqual.equal arguments=(Point, "constant", "constant")
/// @generic.instantiation id=PartialEqual.equal<Point> template=PartialEqual.equal arguments=(Point)
/// @resolution.name source=cloned target=cloned
/// @resolution.place source=cloned placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=cloned root=cloned

const viaClone = requireClone(point);
/// @type.symbol symbol=viaClone source=viaClone type=Point
/// @resolution.pattern source=viaClone kind=binding target=viaClone
/// @resolution.name source=requireClone target=requireClone
/// @resolution.call source=requireClone(point) parameters=(Point) arguments=(provided(point) as Point) return=Point kind=symbol target=requireClone instance=requireClone<Point>
/// @generic.instantiation id=requireClone<Point> template=requireClone arguments=(Point)
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point

const viaDefault = requireDefault<Point>();
/// @type.symbol symbol=viaDefault source=viaDefault type=Point
/// @resolution.pattern source=viaDefault kind=binding target=viaDefault
/// @resolution.name source=requireDefault target=requireDefault
/// @resolution.call source=requireDefault<Point>() parameters=() return=Point kind=symbol target=requireDefault instance=requireDefault<Point>
/// @generic.instantiation id=requireDefault<Point> template=requireDefault arguments=(Point)
/// @resolution.name source=Point target=Point

const viaHash = requireHash(point);
/// @type.symbol symbol=viaHash source=viaHash type=Point
/// @resolution.pattern source=viaHash kind=binding target=viaHash
/// @resolution.name source=requireHash target=requireHash
/// @resolution.call source=requireHash(point) parameters=(Point) arguments=(provided(point) as Point) return=Point kind=symbol target=requireHash instance=requireHash<Point>
/// @generic.instantiation id=requireHash<Point> template=requireHash arguments=(Point)
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point

const viaEqual = requireEqual(point);
/// @type.symbol symbol=viaEqual source=viaEqual type=Point
/// @resolution.pattern source=viaEqual kind=binding target=viaEqual
/// @resolution.name source=requireEqual target=requireEqual
/// @resolution.call source=requireEqual(point) parameters=(Point) arguments=(provided(point) as Point) return=Point kind=symbol target=requireEqual instance=requireEqual<Point>
/// @generic.instantiation id=requireEqual<Point> template=requireEqual arguments=(Point)
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=point root=point
"#,
        r#"

"#,
    );
}

/// Zeroable and Unpin follow from the fields a struct declares.
#[test]
fn test_decide_zeroable_and_unpin_by_field_shape() {
    let session = TestSession::single(
        r#"
import { Unpin, Zeroable, Pin } from "destack:memory";

struct Scalars {
    x: int32;
    y: float64;
}

struct Holder {
    name: string;
}

struct Pinned {
    inner: Pin<Scalars>;
}

declare function requireZeroable<T: Zeroable>(): T;
declare function requireUnpin<T: Unpin>(value: T): T;

const zeroScalars = requireZeroable<Scalars>();
const zeroHolder = requireZeroable<Holder>();
const unpinScalars = requireUnpin(Scalars { x: 1, y: 2.0 });
const unpinHolder = requireUnpin(Holder { name: "a" });
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Pin, Unpin, Zeroable } from "destack:memory";

struct Scalars {
    x: int32;
    y: float64;
}

struct Holder {
    name: string;
}

struct Pinned {
    inner: Pin<Scalars>;
}

declare function requireZeroable<T: Zeroable>(): T;
declare function requireUnpin<T: Unpin>(value: T): T;

const zeroScalars: Scalars = requireZeroable<Scalars>();
const zeroHolder: Holder = requireZeroable<Holder>();
const unpinScalars: Scalars = requireUnpin<Scalars>(Scalars { x: 1, y: 2.0 });
const unpinHolder: Holder = requireUnpin<Holder>(Holder { name: "a" });

=== dir ===
import { Unpin, Zeroable, Pin } from "destack:memory";

struct Scalars {
/// @type.symbol symbol=Scalars type=Scalars
/// @definition.struct symbol=Scalars
/// @definition.field symbol=Scalars.x source="x: int32" key=x type=int32
/// @definition.field symbol=Scalars.y source="y: float64" key=y type=float64

    x: int32;
    /// @type.symbol symbol=Scalars.x source="x: int32" type=int32

    y: float64;
    /// @type.symbol symbol=Scalars.y source="y: float64" type=float64

}

struct Holder {
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder
/// @definition.field symbol=Holder.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Holder.name source="name: string" type=string

}

struct Pinned {
/// @type.symbol symbol=Pinned type=Pinned
/// @definition.struct symbol=Pinned
/// @definition.field symbol=Pinned.inner source="inner: Pin<Scalars>" key=inner type=Pin<Scalars>

    inner: Pin<Scalars>;
    /// @type.symbol symbol=Pinned.inner source="inner: Pin<Scalars>" type=Pin<Scalars>
    /// @resolution.name source=Pin target=Pin
    /// @resolution.name source=Scalars target=Scalars

}

declare function requireZeroable<T: Zeroable>(): T;
/// @generic.template symbol=requireZeroable parameters=(T#1: Zeroable)
/// @type.symbol symbol=requireZeroable source="declare function requireZeroable<T: Zeroable>(): T" type=<T#1: Zeroable>() => T#1
/// @type.symbol symbol=requireZeroable.T source="T: Zeroable" type=T#1
/// @resolution.name source=Zeroable target=Zeroable
/// @resolution.name source=T target=requireZeroable.T

declare function requireUnpin<T: Unpin>(value: T): T;
/// @generic.template symbol=requireUnpin parameters=(T#2: Unpin)
/// @type.symbol symbol=requireUnpin source="declare function requireUnpin<T: Unpin>(value: T): T" type=<T#2: Unpin>(T#2) => T#2
/// @type.symbol symbol=requireUnpin.T source="T: Unpin" type=T#2
/// @resolution.name source=Unpin target=Unpin
/// @type.symbol symbol=requireUnpin.value source="value: T" type=T#2
/// @resolution.name source=T target=requireUnpin.T
/// @resolution.name source=T target=requireUnpin.T

const zeroScalars = requireZeroable<Scalars>();
/// @type.symbol symbol=zeroScalars source=zeroScalars type=Scalars
/// @resolution.pattern source=zeroScalars kind=binding target=zeroScalars
/// @resolution.name source=requireZeroable target=requireZeroable
/// @resolution.call source=requireZeroable<Scalars>() parameters=() return=Scalars kind=symbol target=requireZeroable instance=requireZeroable<Scalars>
/// @generic.instantiation id=requireZeroable<Scalars> template=requireZeroable arguments=(Scalars)
/// @resolution.name source=Scalars target=Scalars

const zeroHolder = requireZeroable<Holder>();
/// @type.symbol symbol=zeroHolder source=zeroHolder type=Holder
/// @resolution.pattern source=zeroHolder kind=binding target=zeroHolder
/// @resolution.name source=requireZeroable target=requireZeroable
/// @resolution.call source=requireZeroable<Holder>() parameters=() return=Holder kind=symbol target=requireZeroable instance=requireZeroable<Holder>
/// @generic.instantiation id=requireZeroable<Holder> template=requireZeroable arguments=(Holder)
/// @resolution.name source=Holder target=Holder

const unpinScalars = requireUnpin(Scalars { x: 1, y: 2.0 });
/// @type.symbol symbol=unpinScalars source=unpinScalars type=Scalars
/// @resolution.pattern source=unpinScalars kind=binding target=unpinScalars
/// @resolution.name source=requireUnpin target=requireUnpin
/// @resolution.call source="requireUnpin(Scalars { x: 1, y: 2.0 })" parameters=(Scalars) arguments=(provided(Scalars { x: 1, y: 2.0 }) as Scalars) return=Scalars kind=symbol target=requireUnpin instance=requireUnpin<Scalars>
/// @generic.instantiation id=requireUnpin<Scalars> template=requireUnpin arguments=(Scalars)
/// @resolution.name source=Scalars target=Scalars

const unpinHolder = requireUnpin(Holder { name: "a" });
/// @type.symbol symbol=unpinHolder source=unpinHolder type=Holder
/// @resolution.pattern source=unpinHolder kind=binding target=unpinHolder
/// @resolution.name source=requireUnpin target=requireUnpin
/// @resolution.call source="requireUnpin(Holder { name: \"a\" })" parameters=(Holder) arguments=(provided(Holder { name: "a" }) as Holder) return=Holder kind=symbol target=requireUnpin instance=requireUnpin<Holder>
/// @generic.instantiation id=requireUnpin<Holder> template=requireUnpin arguments=(Holder)
/// @resolution.name source=Holder target=Holder
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Holder' does not satisfy 'Zeroable'"
/// @diagnostic.label line=21 column=20 span="requireZeroable<Holder>()" line_source="const zeroHolder = requireZeroable<Holder>();"
/// @diagnostic.related line=17 column=34 span="T" line_source="declare function requireZeroable<T: Zeroable>(): T;" message="required by this bound on 'T'"
"#,
    );
}

/// A copyable value clones through the blanket Clone conformance.
#[test]
fn test_clone_a_copy_value_through_the_blanket_clone() {
    let session = TestSession::single(
        r#"
import { Clone } from "destack:memory";

declare function requireClone<T: Clone>(value: T): T;

const number = 1;
const clonedNumber = number.clone();
const viaBound = requireClone(2);
const text: string = "a";
const clonedText = text.clone();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Clone } from "destack:memory";

declare function requireClone<T: Clone>(value: T): T;

const number: 1 = 1;
const clonedNumber: int64 = number.clone<"constant">();
const viaBound: int64 = requireClone<int64>(2);
const text: string = "a";
const clonedText: string = text.clone<"local">() as string;

=== dir ===
import { Clone } from "destack:memory";

declare function requireClone<T: Clone>(value: T): T;
/// @generic.template symbol=requireClone parameters=(T: Clone)
/// @type.symbol symbol=requireClone source="declare function requireClone<T: Clone>(value: T): T" type=<T: Clone>(T) => T
/// @type.symbol symbol=requireClone.T source="T: Clone" type=T
/// @resolution.name source=Clone target=Clone
/// @type.symbol symbol=requireClone.value source="value: T" type=T
/// @resolution.name source=T target=requireClone.T
/// @resolution.name source=T target=requireClone.T

const number = 1;
/// @type.symbol symbol=number source=number type=1
/// @resolution.pattern source=number kind=binding target=number

const clonedNumber = number.clone();
/// @type.symbol symbol=clonedNumber source=clonedNumber type=int64
/// @resolution.pattern source=clonedNumber kind=binding target=clonedNumber
/// @resolution.name source=number target=number
/// @resolution.member source=number.clone receiver=1 type=<Clone.clone.'a, Clone.clone.P1: Place>(this: Borrowed<int64, Clone.clone.'a & Clone.clone.P1, "readonly">) => Owned<int64> kind=symbol target_receiver=1 target=Clone.clone
/// @resolution.call source=number.clone() parameters=() return=Owned<int64> kind=symbol target=Clone.clone receiver=1 adjustments=(borrow(&'static readonly constant 1)) instance="Clone.clone<\"constant\">"
/// @resolution.place source=number placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=number root=number
/// @generic.instantiation id="Clone.clone<int64, \"constant\">" template=Clone.clone arguments=("constant")

const viaBound = requireClone(2);
/// @type.symbol symbol=viaBound source=viaBound type=int64
/// @resolution.pattern source=viaBound kind=binding target=viaBound
/// @resolution.name source=requireClone target=requireClone
/// @resolution.call source=requireClone(2) parameters=(int64) arguments=(provided(2) as int64) return=int64 kind=symbol target=requireClone instance=requireClone<int64>
/// @generic.instantiation id=requireClone<int64> template=requireClone arguments=(int64)

const text: string = "a";
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text

const clonedText = text.clone();
/// @type.symbol symbol=clonedText source=clonedText type=string
/// @resolution.pattern source=clonedText kind=binding target=clonedText
/// @resolution.name source=text target=text
/// @resolution.member source=text.clone receiver=string type=<Clone.clone.'a, Clone.clone.P1: Place>(this: Borrowed<string, Clone.clone.'a & Clone.clone.P1, "readonly">) => Owned<string> kind=symbol target_receiver=string target=Clone.clone
/// @resolution.call source=text.clone() parameters=() return=Owned<string> kind=symbol target=Clone.clone receiver=string adjustments=(borrow(&'static readonly string)) instance="Clone.clone<\"local\">"
/// @resolution.place source=text placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=text root=text
/// @generic.instantiation id="Clone.clone<string, \"local\">" template=Clone.clone arguments=("local")
"#,
        r#"

"#,
    );
}

/// A struct holding a raw pointer derives Copy only when it asks for it.
#[test]
fn test_derive_copy_for_a_raw_pointer_field_only_on_request() {
    let session = TestSession::single(
        r#"
import { Copy } from "destack:memory";

struct Handle {
    pointer: *int32;
}

@derive(Copy, Clone)
struct Cursor {
    pointer: *int32;
}

function witness<T: Copy>(value: T): T {
    return value;
}

declare const handle: Handle;
declare const cursor: Cursor;

witness(handle);
witness(cursor);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import { Copy } from "destack:memory";

struct Handle {
    pointer: *int32;
}

@derive(Copy, Clone)
struct Cursor {
    pointer: *int32;
}

function witness<T: Copy>(value: T): T {
    return value;
}

declare const handle: Handle;
declare const cursor: Cursor;

witness(handle);
witness<Cursor>(cursor);

=== dir ===
import { Copy } from "destack:memory";

struct Handle {
    pointer: *int32;
}

@derive(Copy, Clone)
struct Cursor {
    pointer: *int32;
}

function witness<T: Copy>(value: T): T {
    return value;
}

declare const handle: Handle;
declare const cursor: Cursor;

witness(handle);
witness(cursor);
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Handle' does not satisfy 'Copy'"
/// @diagnostic.label line=20 column=1 span="witness(handle)" line_source="witness(handle);"
/// @diagnostic.related line=13 column=18 span="T" line_source="function witness<T: Copy>(value: T): T {" message="required by this bound on 'T'"
"#,
    );
}

/// A repeatable function value satisfies Copy, a once function does not.
#[test]
fn test_copy_a_function_value_by_its_invocation_count() {
    let session = TestSession::single(
        r#"
function witness<T: Copy>(value: T): T {
    return value;
}

declare const repeatable: () => void;
declare const once: Function<(), void, "once">;

witness(repeatable);
witness(once);
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
function witness<T: Copy>(value: T): T {
    return value;
}

declare const repeatable: () => void;
declare const once: () => void;

witness<() => void>(repeatable);
witness(once);

=== dir ===
function witness<T: Copy>(value: T): T {
/// @generic.template symbol=witness parameters=(T: Copy)
/// @type.symbol symbol=witness type=<T: Copy>(T) => T
/// @type.symbol symbol=witness.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy
/// @type.symbol symbol=witness.value source="value: T" type=T
/// @resolution.name source=T target=witness.T
/// @resolution.name source=T target=witness.T

    return value;
    /// @resolution.name source=value target=witness.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=witness.value

}

declare const repeatable: () => void;
/// @type.symbol symbol=repeatable source=repeatable type=Function<(), void>
/// @resolution.pattern source=repeatable kind=binding target=repeatable

declare const once: Function<(), void, "once">;
/// @type.symbol symbol=once source=once type=Function<(), void>
/// @resolution.pattern source=once kind=binding target=once
/// @resolution.name source=Function target=Function

witness(repeatable);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(repeatable) parameters=(Function<(), void>) arguments=(provided(repeatable) as Function<(), void>) return=Function<(), void> kind=symbol target=witness instance="witness<Function<(), void>>"
/// @generic.instantiation id="witness<Function<(), void>>" template=witness arguments=(Function<(), void>)
/// @resolution.name source=repeatable target=repeatable
/// @resolution.place source=repeatable placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=repeatable root=repeatable

witness(once);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(once) parameters=(<error>) arguments=(provided(once) as <error>) return=<error> kind=symbol target=witness instance=witness<<error>>
/// @resolution.name source=once target=once
/// @resolution.place source=once placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=once root=once
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Function<(), void, \"once\">' does not satisfy 'Copy'"
/// @diagnostic.label line=10 column=1 span="witness(once)" line_source="witness(once);"
/// @diagnostic.related line=2 column=18 span="T" line_source="function witness<T: Copy>(value: T): T {" message="required by this bound on 'T'"
"#,
    );
}

/// Commit conformances on definitions, instances, and bounded parameters.
#[test]
fn test_commit_conformances_on_declarations_instances_and_parameters() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

struct Holder<Value> {
    value: Value;
}

declare function bound<T: Compare>(value: T): T;

declare const held: Holder<Point>;

function read(): int32 {
    const holder = Holder { value: Point { x: 1, y: 2 } };

    return holder.value.x;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::none().with_conformances(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

struct Holder<out Value> {
    value: Value;
}

declare function bound<T: Compare>(value: T): T;

declare const held: Holder<Point>;

function read(): int32 {
    const holder: Holder<Point> = Holder<Point> { value: Point { x: 1, y: 2 } };

    return holder.value.x;
}

=== dir ===
struct Point {
/// @conformance.definition symbol=Point conformances=(SuspendSafe, Concrete, Copy, Clone, Debug, Display, Default, DynamicSafe, Equal, Hash, OverwriteStable, PartialEqual, SharedSafe, Unpin, Zeroable)

    x: int32;
    y: int32;
}

struct Holder<Value> {
    value: Value;
}

declare function bound<T: Compare>(value: T): T;
/// @conformance.parameter parameter=T conformances=(Compare)

declare const held: Holder<Point>;
/// @conformance.instance id=Holder<Point> conformances=(SuspendSafe, Concrete, Copy, Clone, Debug, Display, Default, DynamicSafe, Equal, Hash, OverwriteStable, PartialEqual, SharedSafe, Unpin, Zeroable)

function read(): int32 {
    const holder = Holder { value: Point { x: 1, y: 2 } };

    return holder.value.x;
}
"#,
    );
}

/// A scalar receiver clones through the builtin Clone conformance.
#[test]
fn test_clone_a_scalar_receiver() {
    let session = TestSession::single(
        r#"
function duplicate(value: int32): int32 {
    return value.clone();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function duplicate(value: int32): int32 {
    return value.clone<"local">();
}

=== dir ===
function duplicate(value: int32): int32 {
/// @type.symbol symbol=duplicate type=(int32) => int32
/// @type.symbol symbol=duplicate.value source="value: int32" type=int32

    return value.clone();
    /// @resolution.name source=value target=duplicate.value
    /// @resolution.member source=value.clone receiver=int32 type=<Clone.clone.'a, Clone.clone.P1: Place>(this: Borrowed<int32, Clone.clone.'a & Clone.clone.P1, "readonly">) => Owned<int32> kind=symbol target_receiver=int32 target=Clone.clone
    /// @resolution.call source=value.clone() parameters=() return=Owned<int32> kind=symbol target=Clone.clone receiver=int32 adjustments=(borrow(&'frame readonly int32)) instance="Clone.clone<\"local\">"
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=duplicate.value
    /// @generic.instantiation id="Clone.clone<int32, \"local\">" template=Clone.clone arguments=("local")

}
"#,
        r"",
    );
}
