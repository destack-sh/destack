use crate::tests::{DirRows, TestSession};

#[test]
fn test_scalar_newtype_call_constructs_nominal_value() {
    let session = TestSession::single(
        r#"
newtype UserId = int64;

const id = UserId(42);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype UserId = int64;

const id: UserId = UserId(42);

=== checked ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64 constructors=[(int64) => UserId]

const id = UserId(42);
/// @type.symbol symbol=id source=id type=UserId
/// @resolution.pattern source=id kind=binding target=id
/// @type.node source=UserId type=UserId
/// @type.node source=UserId(42) type=UserId
/// @resolution.name source=UserId target=UserId
/// @resolution.construct source=UserId(42) parameters=(int64) arguments=(provided(42) as int64) return=UserId kind=newtype target=UserId backing=int64
/// @type.node source=42 type=42
"#,
    );
}

#[test]
fn test_tuple_newtype_call_constructs_nominal_value() {
    let session = TestSession::single(
        r#"
newtype Pair = (int32, string);

const pair = Pair(1, "x");
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Pair = (int32, string);

const pair: Pair = Pair(1, "x");

=== checked ===
newtype Pair = (int32, string);
/// @type.symbol symbol=Pair source="newtype Pair = (int32, string)" type=Pair
/// @definition.newtype symbol=Pair source="newtype Pair = (int32, string)" backing=(int32, string) constructors=[(int32, string) => Pair]

const pair = Pair(1, "x");
/// @type.symbol symbol=pair source=pair type=Pair
/// @resolution.pattern source=pair kind=binding target=pair
/// @type.node source="Pair(1, \"x\")" type=Pair
/// @type.node source=Pair type=Pair
/// @resolution.name source=Pair target=Pair
/// @resolution.construct source="Pair(1, \"x\")" parameters=(int32, string) arguments=(provided(1) as int32, provided("x") as string) return=Pair kind=newtype target=Pair backing=(int32, string)
/// @type.node source=1 type=1
/// @type.node source="\"x\"" type="x"
"#,
    );
}

#[test]
fn test_object_newtype_call_constructs_nominal_value() {
    let session = TestSession::single(
        r#"
newtype Config = { debug: boolean };

const config = Config({ debug: true });
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Config = { debug: boolean };

const config: Config = Config({ debug: true });

=== checked ===
newtype Config = { debug: boolean };
/// @type.symbol symbol=Config source="newtype Config = { debug: boolean }" type=Config
/// @definition.newtype symbol=Config source="newtype Config = { debug: boolean }" backing={ debug: boolean } constructors=[({ debug: boolean }) => Config]

const config = Config({ debug: true });
/// @type.symbol symbol=config source=config type=Config
/// @resolution.pattern source=config kind=binding target=config
/// @type.node source="Config({ debug: true })" type=Config
/// @type.node source=Config type=Config
/// @resolution.name source=Config target=Config
/// @resolution.construct source="Config({ debug: true })" parameters=({ debug: boolean }) arguments=(provided({ debug: true }) as { debug: boolean }) return=Config kind=newtype target=Config backing={ debug: boolean }
/// @type.node source={ debug: true } type={ debug: boolean }
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_union_newtype_constructor_accepts_optional_object_literal() {
    let session = TestSession::single(
        r#"
newtype Annotation = () | (string, { reason?: string });

const annotation = Annotation("lint", { reason: "intentional" });
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Annotation = () | (string, { reason?: string });

const annotation: Annotation = Annotation("lint", { reason: "intentional" });

=== checked ===
newtype Annotation = () | (string, { reason?: string });
/// @type.symbol symbol=Annotation source="newtype Annotation = () | (string, { reason?: string })" type=Annotation
/// @definition.newtype symbol=Annotation source="newtype Annotation = () | (string, { reason?: string })" backing=() | (string, { reason?: string }) constructors=[() => Annotation, (string, { reason?: string }) => Annotation, (() | (string, { reason?: string })) => Annotation]

const annotation = Annotation("lint", { reason: "intentional" });
/// @type.symbol symbol=annotation source=annotation type=Annotation
/// @resolution.pattern source=annotation kind=binding target=annotation
/// @type.node source="Annotation(\"lint\", { reason: \"intentional\" })" type=Annotation
/// @type.node source=Annotation type=Annotation
/// @resolution.name source=Annotation target=Annotation
/// @resolution.construct source="Annotation(\"lint\", { reason: \"intentional\" })" parameters=(string, { reason?: string }) arguments=(provided("lint") as string, provided({ reason: "intentional" }) as { reason?: string }) return=Annotation kind=newtype target=Annotation backing=(string, { reason?: string })
/// @type.node source="\"lint\"" type="lint"
/// @type.node source={ reason: "intentional" } type={ reason?: string }
/// @type.node source="\"intentional\"" type="intentional"
"#,
    );
}

#[test]
fn test_scalar_newtype_inferred_call_constructs_nominal_value() {
    let session = TestSession::single(
        r#"
newtype UserId = int64;

const id: UserId = _(42);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype UserId = int64;

const id: UserId = UserId(42);

=== checked ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64 constructors=[(int64) => UserId]

const id: UserId = _(42);
/// @type.symbol symbol=id source=id type=UserId
/// @resolution.pattern source=id kind=binding target=id
/// @resolution.name source=UserId target=UserId
/// @type.node source=_ type=UserId
/// @type.node source=_(42) type=UserId
/// @resolution.construct source=_(42) parameters=(int64) arguments=(provided(42) as int64) return=UserId kind=newtype target=UserId backing=int64
/// @type.node source=42 type=42
"#,
    );
}

#[test]
fn test_tuple_newtype_inferred_call_constructs_nominal_value() {
    let session = TestSession::single(
        r#"
newtype Point = (int32, int32);

const point: Point = _(1, 2);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Point = (int32, int32);

const point: Point = Point(1, 2);

=== checked ===
newtype Point = (int32, int32);
/// @type.symbol symbol=Point source="newtype Point = (int32, int32)" type=Point
/// @definition.newtype symbol=Point source="newtype Point = (int32, int32)" backing=(int32, int32) constructors=[(int32, int32) => Point]

const point: Point = _(1, 2);
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point
/// @type.node source="_(1, 2)" type=Point
/// @type.node source=_ type=Point
/// @resolution.construct source="_(1, 2)" parameters=(int32, int32) arguments=(provided(1) as int32, provided(2) as int32) return=Point kind=newtype target=Point backing=(int32, int32)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_object_newtype_inferred_call_constructs_nominal_value() {
    let session = TestSession::single(
        r#"
newtype Config = { debug: boolean };

const config: Config = _({ debug: true });
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Config = { debug: boolean };

const config: Config = Config({ debug: true });

=== checked ===
newtype Config = { debug: boolean };
/// @type.symbol symbol=Config source="newtype Config = { debug: boolean }" type=Config
/// @definition.newtype symbol=Config source="newtype Config = { debug: boolean }" backing={ debug: boolean } constructors=[({ debug: boolean }) => Config]

const config: Config = _({ debug: true });
/// @type.symbol symbol=config source=config type=Config
/// @resolution.pattern source=config kind=binding target=config
/// @resolution.name source=Config target=Config
/// @type.node source="_({ debug: true })" type=Config
/// @type.node source=_ type=Config
/// @resolution.construct source="_({ debug: true })" parameters=({ debug: boolean }) arguments=(provided({ debug: true }) as { debug: boolean }) return=Config kind=newtype target=Config backing={ debug: boolean }
/// @type.node source={ debug: true } type={ debug: boolean }
/// @type.node source=true type=true
"#,
    );
}

#[test]
fn test_generic_newtype_inferred_call_uses_expected_arguments() {
    let session = TestSession::single(
        r#"
newtype Box<T> = T;

const value: Box<int32> = _(1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Box<out T> = T;

const value: Box<int32> = Box(1);

=== checked ===
newtype Box<T> = T;
/// @generic.template symbol=Box parameters=(out T)
/// @type.symbol symbol=Box source="newtype Box<T> = T" type=Box
/// @definition.newtype symbol=Box source="newtype Box<T> = T" template=(out T) backing=T constructors=[<T>(T) => Box<T>]
/// @type.symbol symbol=Box.T source=T type=T
/// @resolution.name source=T target=Box.T

const value: Box<int32> = _(1);
/// @type.symbol symbol=value source=value type=Box<int32>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Box target=Box
/// @type.node source=_ type=Box
/// @type.node source=_(1) type=Box<int32>
/// @resolution.construct source=_(1) parameters=(int32) arguments=(provided(1) as int32) return=Box<int32> kind=newtype target=Box backing=int32 instance=Box<int32>
/// @generic.instance source=_(1) id=Box<int32>
/// @type.node source=1 type=1

/// @generic.instance id=Box<int32> template=Box arguments=(int32)
"#,
    );
}

#[test]
fn test_generic_newtype_union_constructor_uses_expected_result() {
    let session = TestSession::single(
        r#"
newtype Result<T, E> = T | E;

function from<T, E>(value: E): Result<T, E> {
    Result(value)
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Result<out T, out E> = T | E;

function from<T, E>(value: E): Result<T, E> {
    Result(value)
}

=== checked ===
newtype Result<T, E> = T | E;
/// @generic.template symbol=Result parameters=(out T#1, out E#1)
/// @type.symbol symbol=Result source="newtype Result<T, E> = T | E" type=Result
/// @definition.newtype symbol=Result source="newtype Result<T, E> = T | E" template=(out T#1, out E#1) backing=T#1 | E#1 constructors=[<T#1, E#1>(T#1) => Result<T#1, E#1>, <T#1, E#1>(E#1) => Result<T#1, E#1>, <T#1, E#1>(T#1 | E#1) => Result<T#1, E#1>]
/// @type.symbol symbol=Result.T source=T type=T#1
/// @type.symbol symbol=Result.E source=E type=E#1
/// @resolution.name source=T target=Result.T
/// @resolution.name source=E target=Result.E

function from<T, E>(value: E): Result<T, E> {
/// @generic.template symbol=from parameters=(T#2, E#2)
/// @type.symbol symbol=from type=<T#2, E#2>(E#2) => Result<T#2, E#2>
/// @type.symbol symbol=from.T source=T type=T#2
/// @type.symbol symbol=from.E source=E type=E#2
/// @type.symbol symbol=from.value source="value: E" type=E#2
/// @resolution.name source=E target=from.E
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=from.T
/// @resolution.name source=E target=from.E

    Result(value)
    /// @type.node source=Result type=Result
    /// @type.node source=Result(value) type=Result<T#2, E#2>
    /// @resolution.name source=Result target=Result
    /// @resolution.construct source=Result(value) parameters=(E#2) arguments=(provided(value) as E#2) return=Result<T#2, E#2> kind=newtype target=Result backing=E#2 instance="Result<T#2, E#2>"
    /// @generic.instance source=Result(value) id="Result<T#2, E#2>"
    /// @type.node source=value type=E#2
    /// @resolution.name source=value target=from.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=from.value

}

/// @generic.instance id="Result<T#2, E#2>" template=Result arguments=(T#2, E#2)
"#,
    );
}

#[test]
fn test_inferred_call_without_expected_newtype_reports_error() {
    let session = TestSession::single(
        r#"
const value = _(1);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value = _(1);

=== checked ===
const value = _(1);
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.rejected source=_(1)
"#,
        r#"
/// @diagnostic.error id=cannot-infer-type message="cannot infer a type here"
/// @diagnostic.label line=2 column=15 span="_(1)" line_source="const value = _(1);"
/// @diagnostic.help message="annotate the type explicitly"
"#,
    );
}

#[test]
fn test_inferred_call_with_non_newtype_target_reports_error() {
    let session = TestSession::single(
        r#"
const value: string = _(1);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value: string = _(1);

=== checked ===
const value: string = _(1);
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.rejected source=_(1)
"#,
        r#"
/// @diagnostic.error id=invalid-inferred-construct-target message="type 'string' cannot be constructed with '_(...)'"
/// @diagnostic.label line=2 column=23 span="_(1)" line_source="const value: string = _(1);"
"#,
    );
}

#[test]
fn test_variant_call_infers_payload_type_argument() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Wrap<T> =
    | { kind: "some"; value: T }
    | { kind: "none" };

const wrapped = Wrap.Some({ value: 1 });
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
@derive(Tagged)
newtype Wrap<in out T> = { kind: "some"; value: T } | { kind: "none" };

const wrapped = Wrap.Some({ value: 1 });

=== checked ===
@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Wrap<T> =
/// @generic.template symbol=Wrap parameters=(in out T)
/// @type.symbol symbol=Wrap type=Wrap
/// @type.symbol symbol=Wrap.None type=Wrap.None<T>
/// @type.symbol symbol=Wrap.Some type=<T>({ value: T }) => Wrap.Some<T>
/// @definition.newtype symbol=Wrap template=(in out T) discriminator=kind backing={ kind: "some"; value: T } | { kind: "none" }
/// @definition.variant symbol=Wrap.None key=None discriminant=none backing={ kind: "none" }
/// @definition.variant symbol=Wrap.Some key=Some discriminant=some backing={ kind: "some"; value: T } argument={ value: T }
/// @type.symbol symbol=Wrap.T source=T type=T

    | { kind: "some"; value: T }
    /// @resolution.name source=T target=Wrap.T

    | { kind: "none" };

const wrapped = Wrap.Some({ value: 1 });
/// @type.symbol symbol=wrapped source=wrapped type=Wrap.Some<float64>
/// @resolution.pattern source=wrapped kind=binding target=wrapped
/// @resolution.name source=Wrap target=Wrap
/// @resolution.member source=Wrap.Some receiver=Wrap type=<T>({ value: T }) => Wrap.Some<T> kind=symbol target_receiver=Wrap target=Wrap.Some
/// @resolution.construct source="Wrap.Some({ value: 1 })" parameters=({ value: float64 }) arguments=(provided({ value: 1 }) as { value: float64 }) return=Wrap.Some<float64> kind=variant owner=Wrap variant=Some instance=Wrap<float64> backing={ kind: "some"; value: float64 } argument={ value: float64 } discriminant=some
/// @generic.instance source="Wrap.Some({ value: 1 })" id=Wrap<float64>

/// @generic.instance id=Wrap<T> template=Wrap arguments=(T)
/// @generic.instance id=Wrap<float64> template=Wrap arguments=(float64)
"#);
}

#[test]
fn test_imported_variant_call_infers_payload_type_argument() {
    let session = TestSession::builder()
        .module(
            "wrap.ds",
            r#"
@derive(Tagged)
export newtype Wrap<T> =
    | { kind: "some"; value: T }
    | { kind: "none" };
"#,
        )
        .module(
            "main.ds",
            r#"
import { Wrap } from "./wrap.ds";

const wrapped = Wrap.Some({ value: 1 });
"#,
        )
        .build();

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Wrap } from "./wrap.ds";

const wrapped = Wrap.Some({ value: 1 });

=== checked ===
import { Wrap } from "./wrap.ds";

const wrapped = Wrap.Some({ value: 1 });
/// @type.symbol symbol=wrapped source=wrapped type=wrap.Wrap.Some<float64>
/// @resolution.pattern source=wrapped kind=binding target=wrapped
/// @resolution.name source=Wrap target=wrap.Wrap
/// @resolution.member source=Wrap.Some receiver=wrap.Wrap type=<wrap.Wrap.T>({ value: wrap.Wrap.T }) => wrap.Wrap.Some<wrap.Wrap.T> kind=symbol target_receiver=wrap.Wrap target=wrap.symbol9
/// @resolution.construct source="Wrap.Some({ value: 1 })" parameters=({ value: float64 }) arguments=(provided({ value: 1 }) as { value: float64 }) return=wrap.Wrap.Some<float64> kind=variant owner=wrap.Wrap variant=Some instance=wrap.Wrap<float64> backing={ kind: "some"; value: float64 } argument={ value: float64 } discriminant=some
/// @generic.instance source="Wrap.Some({ value: 1 })" id=wrap.Wrap<float64>

/// @generic.instance id=wrap.Wrap<float64> template=wrap.Wrap arguments=(float64)
"#);
}

#[test]
fn test_imported_variant_call_infers_borrowed_payload() {
    let session = TestSession::builder()
        .module(
            "wrap.ds",
            r#"
@derive(Tagged)
export newtype Wrap<T> =
    | { kind: "some"; value: T }
    | { kind: "none" };

export newtype interface Peekable<T> {
    peek(&readonly this): Wrap<&readonly T>;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Peekable, Wrap } from "./wrap.ds";

export class Holder<T> {
    start: T;

    constructor(start: T) {
        this.start = start;
    }
}

export extension<T> of Holder<T> implements Peekable<T> {
    peek(&readonly this): Wrap<&readonly T> {
        Wrap.Some({ value: &readonly this.start })
    }
}
"#,
        )
        .build();

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
import { Peekable, Wrap } from "./wrap.ds";

export class Holder<in out T> {
    start: T;

    constructor(start: T): this {
        this.start = start;
    }
}

export extension<T> of Holder<T> implements Peekable<T> {
    peek(&readonly this): Wrap<&'a readonly T> {
        Wrap.Some({ value: &readonly this.start })
    }
}

=== checked ===
import { Peekable, Wrap } from "./wrap.ds";

export class Holder<T> {
/// @generic.template symbol=Holder parameters=(in out T#1)
/// @type.symbol symbol=Holder type=Holder
/// @definition.class symbol=Holder template=(in out T#1)
/// @definition.field symbol=Holder.start source="start: T" key=start type=T#1
/// @definition.method symbol=Holder.constructor slot=constructor role=constructor type=(T#1) => this
/// @type.symbol symbol=Holder.T source=T type=T#1

    start: T;
    /// @type.symbol symbol=Holder.start source="start: T" type=T#1
    /// @resolution.name source=T target=Holder.T

    constructor(start: T) {
    /// @type.symbol symbol=Holder.constructor type=(T#1) => this
    /// @type.symbol symbol=Holder.constructor.start source="start: T" type=T#1
    /// @resolution.name source=T target=Holder.T

        this.start = start;
        /// @resolution.receiver source=this kind=this declaration=Holder type=Holder<T#1>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.start kind=place
        /// @resolution.access source=this.start root=this keys=[start]
        /// @resolution.assignment source=this.start write="receiver=Holder<T#1>, target=field(receiver=Holder<T#1>, target=Holder.start, type=T#1), type=T#1" type=T#1
        /// @resolution.name source=start target=Holder.constructor.start
        /// @resolution.place source=start placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=start root=Holder.constructor.start

    }
}

export extension<T> of Holder<T> implements Peekable<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=exported target=Holder<T#2>
/// @definition.implements symbol=<module>#2 source=Peekable<T> target=wrap.Peekable<T#2>
/// @definition.method symbol=peek slot=peek type=<peek.'a>(this: &peek.'a readonly this) => wrap.Wrap<&peek.'a readonly T#2>
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=T target=T
/// @resolution.name source=Peekable target=wrap.Peekable
/// @resolution.name source=T target=T

    peek(&readonly this): Wrap<&readonly T> {
    /// @generic.template symbol=peek parent=template#1 parameters=('a)
    /// @type.symbol symbol=peek type=<peek.'a>(this: &peek.'a readonly this) => wrap.Wrap<&peek.'a readonly T#2>
    /// @type.symbol symbol=peek.this source="&readonly this" type=&peek.'a readonly this
    /// @resolution.name source=Wrap target=wrap.Wrap
    /// @resolution.name source=T target=T

        Wrap.Some({ value: &readonly this.start })
        /// @resolution.name source=Wrap target=wrap.Wrap
        /// @resolution.member source=Wrap.Some receiver=wrap.Wrap type=<wrap.Wrap.T>({ value: wrap.Wrap.T }) => wrap.Wrap.Some<wrap.Wrap.T> kind=symbol target_receiver=wrap.Wrap target=wrap.symbol14
        /// @resolution.construct source="Wrap.Some({ value: &readonly this.start })" parameters=({ value: &peek.'a readonly T#2 }) arguments=(provided({ value: &readonly this.start }) as { value: &peek.'a readonly T#2 }) return=wrap.Wrap.Some<&peek.'a readonly T#2> kind=variant owner=wrap.Wrap variant=Some instance="wrap.Wrap<&peek.'a readonly T#2>" backing={ kind: "some"; value: &peek.'a readonly T#2 } argument={ value: &peek.'a readonly T#2 } discriminant=some
        /// @generic.instance source="Wrap.Some({ value: &readonly this.start })" id="wrap.Wrap<&peek.'a readonly T#2>"
        /// @resolution.member source=this.start receiver=&peek.'a readonly Holder<T#2> type=Readonly<T#2> kind=field target_receiver=&peek.'a readonly Holder<T#2> key=start target=Holder.start target_type=Readonly<T#2>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&peek.'a readonly Holder<T#2>
        /// @resolution.place source=this placement="local" lifetime=peek.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.start placement="local" lifetime=peek.'a access="readonly"
        /// @resolution.access source=this.start root=this keys=[start]

    }
}

/// @generic.instance id="wrap.Wrap<&peek.'a readonly T#2>" template=wrap.Wrap arguments=(&peek.'a readonly T#2)
"#);
}
