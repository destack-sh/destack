use crate::tests::{DirRows, TestSession};

/// Nominal patterns name types even when instance bindings have those types.
#[test]
fn test_match_instance_bindings_as_nominal_patterns() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

newtype UserId = int64;

declare const point: Point;

declare const id: UserId;

if (let point {} = point) {}

if (let id(_) = id) {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

newtype UserId = int64;

declare const point: Point;

declare const id: UserId;

if (let point {} = point) {
}

if (let id(_) = id) {
}

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64 constructors=[(int64) => UserId]

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

declare const id: UserId;
/// @type.symbol symbol=id source=id type=UserId
/// @resolution.pattern source=id kind=binding target=id
/// @resolution.name source=UserId target=UserId

if (let point {} = point) {}
/// @resolution.name source=point target=point
/// @resolution.rejected source="point {}"
/// @resolution.name source=point target=point
/// @resolution.access source=point root=point

if (let id(_) = id) {}
/// @resolution.name source=id target=id
/// @resolution.rejected source=id(_)
/// @resolution.name source=id target=id
/// @resolution.access source=id root=id
"#,
        r#"
/// @diagnostic.error id=value-used-as-type message="expected a type, found value 'point'"
/// @diagnostic.label line=12 column=9 span="point" line_source="if (let point {} = point) {}"
/// @diagnostic.error id=value-used-as-type message="expected a type, found value 'id'"
/// @diagnostic.label line=14 column=9 span="id" line_source="if (let id(_) = id) {}"
"#,
    );
}

#[test]
fn test_newtype_pattern_unwraps_backing_value() {
    let session = TestSession::single(
        r#"
newtype UserId = int64;

declare const id: UserId;

if (let UserId(value) = id) {
    value satisfies int64;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype UserId = int64;

declare const id: UserId;

if (let UserId(value) = id) {
    value satisfies int64;
}

=== dir ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64 constructors=[(int64) => UserId]

declare const id: UserId;
/// @type.symbol symbol=id source=id type=UserId
/// @resolution.pattern source=id kind=binding target=id
/// @resolution.name source=UserId target=UserId

if (let UserId(value) = id) {
/// @resolution.name source=UserId target=UserId
/// @resolution.pattern source=UserId(value) kind=newtype projection="newtype.payload(UserId, int64)" pattern=pattern
/// @type.symbol symbol=value source=value type=int64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=id type=UserId
/// @resolution.name source=id target=id
/// @resolution.access source=id root=id

    value satisfies int64;
    /// @type.node source="value satisfies int64" type=int64
    /// @type.node source=value type=int64
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=value

}
"#,
    );
}

#[test]
fn test_nominal_object_pattern_binds_struct_fields() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

declare const point: Point;

match (point) {
    Point { x, y } => x + y
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

declare const point: Point;

match (point) {
    Point { x, y } => x + y
}

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

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

match (point) {
/// @type.node type=int32
/// @resolution.coverage exhaustive=true disjoint=true
/// @type.node source=point type=Point
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point

    Point { x, y } => x + y
    /// @resolution.name source=Point target=Point
    /// @resolution.pattern source="Point { x, y }" kind=nominal_object target=Point fields={ Point.x, Point.y }
    /// @type.symbol symbol=x source=x type=int32
    /// @type.symbol symbol=y source=y type=int32
    /// @type.node source="x + y" type=int32
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x
    /// @resolution.operator source="x + y" type=int32 operator="+" kind=builtin operands=[x as int32 families=(integer), y as int32 families=(integer)]
    /// @resolution.place source=x placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=x root=x
    /// @type.node source=y type=int32
    /// @resolution.name source=y target=y
    /// @resolution.place source=y placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=y root=y

}
"#,
    );
}

#[test]
fn test_nominal_object_pattern_rejects_structural_tag() {
    let session = TestSession::single(
        r#"
type Point = { x: int32; y: int32 };

declare const point: Point;

match (point) {
    Point { x, y } => x + y
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Point = { x: int32; y: int32 };

declare const point: Point;

match (point) {
    Point { x, y } => x + y
}

=== dir ===
type Point = { x: int32; y: int32 };
/// @type.symbol symbol=Point source="type Point = { x: int32; y: int32 }" type={ x: int32; y: int32 }
/// @definition.type symbol=Point source="type Point = { x: int32; y: int32 }" value={ x: int32; y: int32 }
/// @type.symbol symbol=Point.x source="x: int32" type=int32
/// @type.symbol symbol=Point.y source="y: int32" type=int32

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

match (point) {
/// @type.node type=<error>
/// @type.node source=point type=Point
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point

    Point { x, y } => x + y
    /// @resolution.name source=Point target=Point
    /// @resolution.rejected source="Point { x, y }"
    /// @type.symbol symbol=x source=x type=<error>
    /// @type.symbol symbol=y source=y type=<error>
    /// @type.node source="x + y" type=<error>
    /// @type.node source=x type=<error>
    /// @resolution.name source=x target=x
    /// @resolution.poisoned source="x + y"
    /// @resolution.place source=x placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=x root=x
    /// @type.node source=y type=<error>
    /// @resolution.name source=y target=y
    /// @resolution.place source=y placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=y root=y

}
"#,
        r#"
/// @diagnostic.error id=invalid-pattern-tag message="pattern tag '{ x: int32; y: int32 }' is not a nominal type"
/// @diagnostic.label line=7 column=5 span="Point { x, y }" line_source="Point { x, y } => x + y"
"#,
    );
}

#[test]
fn test_nominal_object_pattern_rejects_non_field_member() {
    let session = TestSession::single(
        r#"
class User {
    name: string = "";
    displayName(): string {
        return this.name;
    }
}

declare const user: User;

match (user) {
    User { displayName } => displayName
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string = "";
    displayName(): string {
        return this.name;
    }
}

declare const user: User;

match (user) {
    User { displayName } => displayName
}

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string = \"\"" key=name type=string
/// @definition.method symbol=User.displayName slot=displayName type=(this: User) => string

    name: string = "";
    /// @type.symbol symbol=User.name source="name: string = \"\"" type=string
    /// @type.node source="\"\"" type=""

    displayName(): string {
    /// @type.symbol symbol=User.displayName type=(this: User) => string
    /// @type.symbol symbol=User.displayName.this type=User

        return this.name;
        /// @type.node source=this type=User
        /// @type.node source=this.name type=string
        /// @resolution.member source=this.name receiver=User type=string kind=field target_receiver=User key=name target=User.name target_type=string
        /// @resolution.receiver source=this kind=this declaration=User type=User
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]

    }
}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

match (user) {
/// @type.node type=<error>
/// @resolution.coverage exhaustive=true disjoint=true
/// @type.node source=user type=User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user

    User { displayName } => displayName
    /// @resolution.name source=User target=User
    /// @resolution.pattern source="User { displayName }" kind=nominal_object target=User fields={}
    /// @type.symbol symbol=displayName source=displayName type=<error>
    /// @type.node source=displayName type=<error>
    /// @resolution.name source=displayName target=displayName
    /// @resolution.place source=displayName placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=displayName root=displayName

}
"#,
        r#"
/// @diagnostic.error id=pattern-member-not-field message="member 'displayName' on type 'User' is not a field"
/// @diagnostic.label line=12 column=12 span="displayName" line_source="User { displayName } => displayName"
"#,
    );
}

/// Fields destructured beneath a borrowed scrutinee bind through its borrow.
#[test]
fn test_bind_destructured_fields_through_a_borrowed_scrutinee() {
    let session = TestSession::single(
        r#"
import { Equal } from "tspp:ops";

struct Ok<T> {
    value: T;
}

struct Err<E> {
    error: E;
}

newtype Outcome<T, E> = Ok<T> | Err<E>;

function same<T: Equal<T>, E: Equal<E>>(left: &immutable Outcome<T, E>, right: &immutable Outcome<T, E>): boolean {
    match (left) {
        Ok { value: a } => {
            match (right) {
                Ok { value: b } => a.equal(b)
                Err { error: _ } => false
            }
        }
        Err { error: a } => {
            match (right) {
                Err { error: b } => a.equal(b)
                Ok { value: _ } => false
            }
        }
    }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Equal } from "tspp:ops";

struct Ok<out T> {
    value: T;
}

struct Err<out E> {
    error: E;
}

newtype Outcome<out T, out E> = Ok<T> | Err<E>;

function same<T: Equal<T>, E: Equal<E>, 'a, 'b>(
    left: &'a immutable Outcome<T, E>,
    right: &'b immutable Outcome<T, E>,
): boolean {
    match (left) {
        Ok { value: a } => {
            match (right) {
                Ok { value: b } => a.equal<T, 'a, 'b>(b)
                Err { error: _ } => false
            }
        }
        Err { error: a } => {
            match (right) {
                Err { error: b } => a.equal<E, 'a, 'b>(b)
                Ok { value: _ } => false
            }
        }
    }
}

=== dir ===
import { Equal } from "tspp:ops";

struct Ok<T> {
/// @generic.template symbol=Ok parameters=(out T#1)
/// @type.symbol symbol=Ok type=Ok
/// @definition.struct symbol=Ok template=(out T#1)
/// @definition.field symbol=Ok.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Ok.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Ok.value source="value: T" type=T#1
    /// @resolution.name source=T target=Ok.T

}

struct Err<E> {
/// @generic.template symbol=Err parameters=(out E#1)
/// @type.symbol symbol=Err type=Err
/// @definition.struct symbol=Err template=(out E#1)
/// @definition.field symbol=Err.error source="error: E" key=error type=E#1
/// @type.symbol symbol=Err.E source=E type=E#1

    error: E;
    /// @type.symbol symbol=Err.error source="error: E" type=E#1
    /// @resolution.name source=E target=Err.E

}

newtype Outcome<T, E> = Ok<T> | Err<E>;
/// @generic.template symbol=Outcome parameters=(out T#2, out E#2)
/// @type.symbol symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" type=Outcome
/// @generic.instance id=Err<E#2> template=Err arguments=(E#2)
/// @generic.instance id=Ok<T#2> template=Ok arguments=(T#2)
/// @definition.newtype symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" template=(out T#2, out E#2) backing=Ok<T#2> | Err<E#2> constructors=[<T#2, E#2>(Ok<T#2>) => Outcome<T#2, E#2>, <T#2, E#2>(Err<E#2>) => Outcome<T#2, E#2>, <T#2, E#2>(Ok<T#2> | Err<E#2>) => Outcome<T#2, E#2>]
/// @type.symbol symbol=Outcome.T source=T type=T#2
/// @type.symbol symbol=Outcome.E source=E type=E#2
/// @resolution.name source=Ok target=Ok
/// @resolution.name source=T target=Outcome.T
/// @resolution.name source=Err target=Err
/// @resolution.name source=E target=Outcome.E

function same<T: Equal<T>, E: Equal<E>>(left: &immutable Outcome<T, E>, right: &immutable Outcome<T, E>): boolean {
/// @generic.template symbol=same parameters=(T#3: Equal<T#3>, E#3: Equal<E#3>, 'a, 'b)
/// @type.symbol symbol=same type=<T#3: Equal<T#3>, E#3: Equal<E#3>, same.'a, same.'b>(&same.'a immutable Outcome<T#3, E#3>, &same.'b immutable Outcome<T#3, E#3>) => boolean
/// @generic.instance id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3)
/// @generic.instance id=Err<E#3> template=Err arguments=(E#3)
/// @generic.instance id=Ok<T#3> template=Ok arguments=(T#3)
/// @type.symbol symbol=same.T source="T: Equal<T>" type=T#3
/// @resolution.name source=Equal target=Equal
/// @generic.instance id=Equal<T#3> template=Equal arguments=(T#3)
/// @resolution.name source=T target=same.T
/// @type.symbol symbol=same.E source="E: Equal<E>" type=E#3
/// @resolution.name source=Equal target=Equal
/// @generic.instance id=Equal<E#3> template=Equal arguments=(E#3)
/// @resolution.name source=E target=same.E
/// @type.symbol symbol=same.left source="left: &immutable Outcome<T, E>" type=&same.'a immutable Outcome<T#3, E#3>
/// @resolution.name source=Outcome target=Outcome
/// @resolution.name source=T target=same.T
/// @resolution.name source=E target=same.E
/// @type.symbol symbol=same.right source="right: &immutable Outcome<T, E>" type=&same.'b immutable Outcome<T#3, E#3>
/// @resolution.name source=Outcome target=Outcome
/// @resolution.name source=T target=same.T
/// @resolution.name source=E target=same.E

    match (left) {
    /// @resolution.coverage exhaustive=true disjoint=true
    /// @resolution.name source=left target=same.left
    /// @resolution.place source=left placement=same.'a lifetime=same.'a access="immutable"
    /// @resolution.access source=left root=same.left

        Ok { value: a } => {
        /// @resolution.name source=Ok target=Ok
        /// @resolution.pattern source="Ok { value: a }" kind=nominal_object adjustments=(newtype.payload(Outcome, &same.'a immutable (Ok<T#3> | Err<E#3>)), union.payload(Ok<T#3> | Err<E#3>, Ok<T#3>, &same.'a immutable Ok<T#3>)) target=Ok instance=Ok<T#3> fields={ Ok.value: same.a#1 }
        /// @generic.instantiation id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3) owner=same
        /// @generic.instantiation id=Ok<T#3> template=Ok arguments=(T#3) owner=same
        /// @type.symbol symbol=same.a#1 source=a type=&same.'a immutable T#3
        /// @resolution.pattern source=a kind=binding target=same.a#1

            match (right) {
            /// @resolution.coverage exhaustive=true disjoint=true
            /// @resolution.name source=right target=same.right
            /// @resolution.place source=right placement=same.'b lifetime=same.'b access="immutable"
            /// @resolution.access source=right root=same.right

                Ok { value: b } => a.equal(b)
                /// @resolution.name source=Ok target=Ok
                /// @resolution.pattern source="Ok { value: b }" kind=nominal_object adjustments=(newtype.payload(Outcome, &same.'b immutable (Ok<T#3> | Err<E#3>)), union.payload(Ok<T#3> | Err<E#3>, Ok<T#3>, &same.'b immutable Ok<T#3>)) target=Ok instance=Ok<T#3> fields={ Ok.value: same.b#1 }
                /// @type.symbol symbol=same.b#1 source=b type=&same.'b immutable T#3
                /// @resolution.pattern source=b kind=binding target=same.b#1
                /// @resolution.name source=a target=same.a#1
                /// @resolution.member source=a.equal receiver=&same.'a immutable T#3 type=<PartialEqual.equal.'a, PartialEqual.equal.'b>(this: &PartialEqual.equal.'a immutable T#3, &PartialEqual.equal.'b immutable T#3) => boolean kind=symbol target_receiver=&same.'a immutable T#3 target=PartialEqual.equal
                /// @resolution.call source=a.equal(b) parameters=(&same.'b immutable T#3) arguments=(provided(b) as &same.'b immutable T#3) return=boolean regions=(same.'a, same.'b) kind=symbol target=PartialEqual.equal receiver=&same.'a immutable T#3 instance="PartialEqual<T#3>.equal<same.'a, same.'b>"
                /// @resolution.place source=a placement=same.'a lifetime=same.'a access="immutable"
                /// @resolution.access source=a root=same.a#1
                /// @generic.instantiation id="PartialEqual.equal<T#3, T#3, same.'a, same.'b>" template=PartialEqual.equal arguments=(T#3, same.'a, same.'b) owner=same
                /// @generic.instantiation id=PartialEqual.equal<T#3> template=PartialEqual.equal arguments=(T#3) owner=same
                /// @generic.instance id="PartialEqual.equal<T#3, T#3, same.'a, same.'b>" template=PartialEqual.equal arguments=(T#3, same.'a, same.'b)
                /// @resolution.name source=b target=same.b#1
                /// @resolution.place source=b placement=same.'b lifetime=same.'b access="immutable"
                /// @resolution.access source=b root=same.b#1

                Err { error: _ } => false
                /// @resolution.name source=Err target=Err
                /// @resolution.pattern source="Err { error: _ }" kind=nominal_object adjustments=(newtype.payload(Outcome, &same.'b immutable (Ok<T#3> | Err<E#3>)), union.payload(Ok<T#3> | Err<E#3>, Err<E#3>, &same.'b immutable Err<E#3>)) target=Err instance=Err<E#3> fields={ Err.error: _ }
                /// @generic.instantiation id=Err<E#3> template=Err arguments=(E#3) owner=same
                /// @resolution.pattern source=_ kind=wildcard

            }
        }
        Err { error: a } => {
        /// @resolution.name source=Err target=Err
        /// @resolution.pattern source="Err { error: a }" kind=nominal_object adjustments=(newtype.payload(Outcome, &same.'a immutable (Ok<T#3> | Err<E#3>)), union.payload(Ok<T#3> | Err<E#3>, Err<E#3>, &same.'a immutable Err<E#3>)) target=Err instance=Err<E#3> fields={ Err.error: same.a#2 }
        /// @type.symbol symbol=same.a#2 source=a type=&same.'a immutable E#3
        /// @resolution.pattern source=a kind=binding target=same.a#2

            match (right) {
            /// @resolution.coverage exhaustive=true disjoint=true
            /// @resolution.name source=right target=same.right
            /// @resolution.place source=right placement=same.'b lifetime=same.'b access="immutable"
            /// @resolution.access source=right root=same.right

                Err { error: b } => a.equal(b)
                /// @resolution.name source=Err target=Err
                /// @resolution.pattern source="Err { error: b }" kind=nominal_object adjustments=(newtype.payload(Outcome, &same.'b immutable (Ok<T#3> | Err<E#3>)), union.payload(Ok<T#3> | Err<E#3>, Err<E#3>, &same.'b immutable Err<E#3>)) target=Err instance=Err<E#3> fields={ Err.error: same.b#2 }
                /// @type.symbol symbol=same.b#2 source=b type=&same.'b immutable E#3
                /// @resolution.pattern source=b kind=binding target=same.b#2
                /// @resolution.name source=a target=same.a#2
                /// @resolution.member source=a.equal receiver=&same.'a immutable E#3 type=<PartialEqual.equal.'a, PartialEqual.equal.'b>(this: &PartialEqual.equal.'a immutable E#3, &PartialEqual.equal.'b immutable E#3) => boolean kind=symbol target_receiver=&same.'a immutable E#3 target=PartialEqual.equal
                /// @resolution.call source=a.equal(b) parameters=(&same.'b immutable E#3) arguments=(provided(b) as &same.'b immutable E#3) return=boolean regions=(same.'a, same.'b) kind=symbol target=PartialEqual.equal receiver=&same.'a immutable E#3 instance="PartialEqual<E#3>.equal<same.'a, same.'b>"
                /// @resolution.place source=a placement=same.'a lifetime=same.'a access="immutable"
                /// @resolution.access source=a root=same.a#2
                /// @generic.instantiation id="PartialEqual.equal<E#3, E#3, same.'a, same.'b>" template=PartialEqual.equal arguments=(E#3, same.'a, same.'b) owner=same
                /// @generic.instantiation id=PartialEqual.equal<E#3> template=PartialEqual.equal arguments=(E#3) owner=same
                /// @generic.instance id="PartialEqual.equal<E#3, E#3, same.'a, same.'b>" template=PartialEqual.equal arguments=(E#3, same.'a, same.'b)
                /// @resolution.name source=b target=same.b#2
                /// @resolution.place source=b placement=same.'b lifetime=same.'b access="immutable"
                /// @resolution.access source=b root=same.b#2

                Ok { value: _ } => false
                /// @resolution.name source=Ok target=Ok
                /// @resolution.pattern source="Ok { value: _ }" kind=nominal_object adjustments=(newtype.payload(Outcome, &same.'b immutable (Ok<T#3> | Err<E#3>)), union.payload(Ok<T#3> | Err<E#3>, Ok<T#3>, &same.'b immutable Ok<T#3>)) target=Ok instance=Ok<T#3> fields={ Ok.value: _ }
                /// @resolution.pattern source=_ kind=wildcard

            }
        }
    }
}
"#,
    );
}
