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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype UserId = int64;

declare const id: UserId;

if (let UserId(value) = id) {
    value satisfies int64;
}

=== checked ===
newtype UserId = int64;
/// @type.symbol symbol=UserId source="newtype UserId = int64" type=UserId
/// @definition.newtype symbol=UserId source="newtype UserId = int64" backing=int64

declare const id: UserId;
/// @type.symbol symbol=id source=id type=UserId
/// @resolution.name source=UserId target=UserId

if (let UserId(value) = id) {
/// @resolution.name source=UserId target=UserId
/// @resolution.pattern source=UserId(value) kind=newtype projection="newtype.payload(UserId, int64)" pattern=pattern
/// @type.symbol symbol=value source=value type=int64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=id type=UserId
/// @resolution.name source=id target=id

    value satisfies int64;
    /// @type.node source="value satisfies int64" type=int64
    /// @type.node source=value type=int64
    /// @resolution.name source=value target=value

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

    session.assert_dir_checked(
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

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.name source=Point target=Point

match (point) {
/// @type.node type=int32
/// @type.node source=point type=Point
/// @resolution.name source=point target=point

    Point { x, y } => x + y
    /// @resolution.name source=Point target=Point
    /// @resolution.pattern source="Point { x, y }" kind=nominal_object target=Point fields={ Point.x, Point.y }
    /// @type.symbol symbol=x source=x type=int32
    /// @type.symbol symbol=y source=y type=int32
    /// @type.node source="x + y" type=int32
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x
    /// @resolution.call source="x + y" parameters=() return=int32 kind=builtin builtin=binary.add
    /// @type.node source=y type=int32
    /// @resolution.name source=y target=y

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Point = { x: int32; y: int32 };

declare const point: Point;

match (point) {
    Point { x, y } => x + y
}

=== checked ===
type Point = { x: int32; y: int32 };
/// @type.symbol symbol=Point source="type Point = { x: int32; y: int32 }" type={ x: int32; y: int32 }
/// @definition.type symbol=Point source="type Point = { x: int32; y: int32 }" value={ x: int32; y: int32 }

declare const point: Point;
/// @type.symbol symbol=point source=point type=Point reduced={ x: int32; y: int32 }
/// @resolution.name source=Point target=Point

match (point) {
/// @type.node type=<error>
/// @type.node source=point type=Point reduced={ x: int32; y: int32 }
/// @resolution.name source=point target=point

    Point { x, y } => x + y
    /// @resolution.name source=Point target=Point
    /// @type.symbol symbol=x source=x type=<error>
    /// @type.symbol symbol=y source=y type=<error>
    /// @type.node source="x + y" type=<error>
    /// @type.node source=x type=<error>
    /// @resolution.name source=x target=x
    /// @type.node source=y type=<error>
    /// @resolution.name source=y target=y

}
"#,
        r#"
/// @diagnostic.error code=EC411 message="pattern tag '{ x: int32; y: int32 }' is not a nominal type"
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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string = \"\"" key=name type=string
/// @definition.method symbol=User.displayName slot=displayName type=(this: this) => string

    name: string = "";
    /// @type.symbol symbol=User.name source="name: string = \"\"" type=string
    /// @type.node source="\"\"" type=""

    displayName(): string {
    /// @type.symbol symbol=User.displayName type=(this: this) => string

        return this.name;
        /// @type.node source=this type=User
        /// @type.node source=this.name type=string
        /// @resolution.member source=this.name receiver=User kind=symbol target=User.name
        /// @resolution.receiver source=this kind=this declaration=User type=User

    }
}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.name source=User target=User

match (user) {
/// @type.node type=<error>
/// @type.node source=user type=User
/// @resolution.name source=user target=user

    User { displayName } => displayName
    /// @resolution.name source=User target=User
    /// @resolution.pattern source="User { displayName }" kind=nominal_object target=User fields={}
    /// @type.symbol symbol=displayName source=displayName type=<error>
    /// @type.node source=displayName type=<error>
    /// @resolution.name source=displayName target=displayName

}
"#,
        r#"
/// @diagnostic.error code=EC427 message="member 'displayName' on type 'User' is not a field"
/// @diagnostic.label line=12 column=12 span="displayName" line_source="User { displayName } => displayName"
"#,
    );
}
