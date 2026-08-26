use crate::tests::{DirRows, TestSession};

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
        "main.ds",
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
        "main.ds",
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
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
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
    /// @resolution.place source=x placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=x root=x
    /// @type.node source=y type=int32
    /// @resolution.name source=y target=y
    /// @resolution.place source=y placement="local" lifetime="frame" access="readonly"
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
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Point = { x: int32; y: int32 };

declare const point: { x: int32; y: int32 };

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
/// @type.symbol symbol=point source=point type={ x: int32; y: int32 }
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

match (point) {
/// @type.node type=int32
/// @resolution.coverage exhaustive=false disjoint=true
/// @type.node source=point type={ x: int32; y: int32 }
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point

    Point { x, y } => x + y
    /// @resolution.name source=Point target=Point
    /// @resolution.pattern source="Point { x, y }" kind=nominal_object target=Point fields={ x, y }
    /// @type.symbol symbol=x source=x type=int32
    /// @type.symbol symbol=y source=y type=int32
    /// @type.node source="x + y" type=int32
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x
    /// @resolution.operator source="x + y" type=int32 operator="+" kind=builtin operands=[x as int32 families=(integer), y as int32 families=(integer)]
    /// @resolution.place source=x placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=x root=x
    /// @type.node source=y type=int32
    /// @resolution.name source=y target=y
    /// @resolution.place source=y placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=y root=y

}
"#,
        r#"
/// @diagnostic.error id=non-exhaustive-pattern message="match is not exhaustive: '{ x: int32; y: int32 }' is not covered"
/// @diagnostic.label line=6 column=1 span="match" line_source="match (point) {"
/// @diagnostic.help message="cover the remaining values or add a wildcard '_' arm"
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
        "main.ds",
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
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string = \"\"" key=name type=string
/// @definition.method symbol=User.displayName slot=displayName type=<User.displayName.P0: Place>(this: Managed<this, User.displayName.P0>) => string

    name: string = "";
    /// @type.symbol symbol=User.name source="name: string = \"\"" type=string
    /// @type.node source="\"\"" type=""

    displayName(): string {
    /// @generic.template symbol=User.displayName parameters=(P0: Place)
    /// @type.symbol symbol=User.displayName type=<User.displayName.P0: Place>(this: Managed<this, User.displayName.P0>) => string
    /// @type.symbol symbol=User.displayName.this type=Managed<User, User.displayName.P0>

        return this.name;
        /// @type.node source=this type=Managed<User, User.displayName.P0>
        /// @type.node source=this.name type=Managed<string, User.displayName.P0>
        /// @resolution.member source=this.name receiver=Managed<User, User.displayName.P0> type=Managed<string, User.displayName.P0> kind=field target_receiver=Managed<User, User.displayName.P0> key=name target=User.name target_type=Managed<string, User.displayName.P0>
        /// @resolution.receiver source=this kind=this declaration=User type=Managed<User, User.displayName.P0>
        /// @resolution.place source=this placement=User.displayName.P0 lifetime="frame" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.name placement=User.displayName.P0 lifetime="frame" access="mutable"
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
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user

    User { displayName } => displayName
    /// @resolution.name source=User target=User
    /// @resolution.pattern source="User { displayName }" kind=nominal_object target=User fields={}
    /// @type.symbol symbol=displayName source=displayName type=<error>
    /// @type.node source=displayName type=<error>
    /// @resolution.name source=displayName target=displayName
    /// @resolution.place source=displayName placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=displayName root=displayName

}
"#,
        r#"
/// @diagnostic.error id=pattern-member-not-field message="member 'displayName' on type 'User' is not a field"
/// @diagnostic.label line=12 column=12 span="displayName" line_source="User { displayName } => displayName"
"#,
    );
}
