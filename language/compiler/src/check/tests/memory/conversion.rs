use crate::tests::{DirRows, TestSession};

#[test]
fn test_managed_value_does_not_become_owned() {
    let session = TestSession::single(
        r#"
class User {}

let user: User = new User();
let owned: ^User = user;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}

let user: User = new User();
let owned: Owned<User> = user;

=== checked ===
class User {
/// @type.symbol symbol=User type=User
}

let user: User = new User();
/// @type.symbol symbol=user source=user type=User
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User
/// @resolution.construct source="new User()" parameters=() return=User kind=class target=User

let owned: ^User = user;
/// @type.symbol symbol=owned source=owned type=Owned<User>
/// @resolution.name source=User target=User
/// @resolution.name source=user target=user
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'User' is not assignable to type 'Owned<User>'"
/// @diagnostic.label line=5 column=5 source="let owned: ^User = user;"
"#,
    );
}

#[test]
fn test_borrow_does_not_become_owned() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let borrow = &point;
let owned: ^Point = borrow;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

let point: Owned<Point> = ^Point { x: 1 };
let borrow: Borrowed<Point, L0, "mutable"> = &point;
let owned: Owned<Point> = borrow;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32
}

let point = ^Point { x: 1 };
/// @type.symbol symbol=point type=Owned<Point>
/// @resolution.name source=Point target=Point

let borrow = &point;
/// @type.symbol symbol=borrow type=Borrowed<Point, borrow.L0, "mutable">
/// @resolution.name source=point target=point
/// @borrow.source source="&point" place=point lifetime=borrow.L0 access=mutable

let owned: ^Point = borrow;
/// @type.symbol symbol=owned source=owned type=Owned<Point>
/// @resolution.name source=Point target=Point
/// @resolution.name source=borrow target=borrow
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Borrowed<Point, borrow.L0, \"mutable\">' is not assignable to type 'Owned<Point>'"
/// @diagnostic.label line=8 column=5 source="let owned: ^Point = borrow;"
"#,
    );
}
