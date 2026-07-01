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
let owned: ^User = user;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

let user: User = new User();
/// @type.symbol symbol=user source=user type=User
/// @resolution.name source=User target=User
/// @type.node source="new User()" type=User
/// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
/// @resolution.name source=User target=User

let owned: ^User = user;
/// @type.symbol symbol=owned source=owned type=Owned<User>
/// @resolution.name source=User target=User
/// @type.node source=user type=User
/// @resolution.name source=user target=user
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'User' is not assignable to type '^User'"
/// @diagnostic.label line=5 column=20 span="user" line_source="let owned: ^User = user;"
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

let point: ^Point = ^Point { x: 1 };
let borrow: Borrowed<Point, "static", "mutable"> = &point;
let owned: ^Point = borrow;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

let point = ^Point { x: 1 };
/// @type.symbol symbol=point source=point type=Owned<Point> reduced=Point
/// @type.node source="^Point { x: 1 }" type=Owned<Point> reduced=Point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

let borrow = &point;
/// @type.symbol symbol=borrow source=borrow type=Borrowed<Point, "static", "mutable">
/// @type.node source=&point type=Borrowed<Point, "static", "mutable">
/// @type.node source=point type=Owned<Point> reduced=Point
/// @resolution.name source=point target=point

let owned: ^Point = borrow;
/// @type.symbol symbol=owned source=owned type=Owned<Point> reduced=Point
/// @resolution.name source=Point target=Point
/// @type.node source=borrow type=Borrowed<Point, "static", "mutable">
/// @resolution.name source=borrow target=borrow
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '&Point' is not assignable to type '^Point'"
/// @diagnostic.label line=8 column=21 span="borrow" line_source="let owned: ^Point = borrow;"
"#,
    );
}
