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
struct Label {
    name: string;
}

let label = ^Label { name: "origin" };
let borrow = &label;
let owned: ^Label = borrow;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Label {
    name: string;
}

let label: ^Label = ^Label { name: "origin" };
let borrow: Borrowed<Label, "static", "mutable"> = &label;
let owned: ^Label = borrow;

=== checked ===
struct Label {
/// @type.symbol symbol=Label type=Label
/// @definition.struct symbol=Label
/// @definition.field symbol=Label.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Label.name source="name: string" type=string

}

let label = ^Label { name: "origin" };
/// @type.symbol symbol=label source=label type=Owned<Label> reduced=Label
/// @type.node source="^Label { name: \"origin\" }" type=Owned<Label> reduced=Label
/// @type.node source="Label { name: \"origin\" }" type=Label
/// @resolution.name source=Label target=Label
/// @type.node source="\"origin\"" type="origin"

let borrow = &label;
/// @type.symbol symbol=borrow source=borrow type=Borrowed<Label, "static", "mutable">
/// @type.node source=&label type=Borrowed<Label, "static", "mutable">
/// @type.node source=label type=Owned<Label> reduced=Label
/// @resolution.name source=label target=label

let owned: ^Label = borrow;
/// @type.symbol symbol=owned source=owned type=Owned<Label> reduced=Label
/// @resolution.name source=Label target=Label
/// @type.node source=borrow type=Borrowed<Label, "static", "mutable">
/// @resolution.name source=borrow target=borrow
"#,
        r#"
/// @diagnostic.error code=EC200 message="type '&Label' is not assignable to type '^Label'"
/// @diagnostic.label line=8 column=21 span="borrow" line_source="let owned: ^Label = borrow;"
"#,
    );
}

#[test]
fn test_borrow_copies_a_copyable_value_out() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };
let borrow = &point;
let copied: Point = borrow;
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
let copied: Point = borrow as Point;
let owned: ^Point = borrow as Point;

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

let copied: Point = borrow;
/// @type.symbol symbol=copied source=copied type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=borrow type=Borrowed<Point, "static", "mutable">
/// @resolution.name source=borrow target=borrow

let owned: ^Point = borrow;
/// @type.symbol symbol=owned source=owned type=Owned<Point> reduced=Point
/// @resolution.name source=Point target=Point
/// @type.node source=borrow type=Borrowed<Point, "static", "mutable">
/// @resolution.name source=borrow target=borrow
"#,
        r#""#,
    );
}

#[test]
fn test_construction_materializes_at_an_owned_target() {
    let session = TestSession::single(
        r#"
class User {}

let owned: ^User = new User();
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}

let owned: ^User = new User();

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

let owned: ^User = new User();
/// @type.symbol symbol=owned source=owned type=Owned<User>
/// @resolution.name source=User target=User
/// @type.node source="new User()" type=Owned<User>
/// @resolution.construct source="new User()" parameters=() return=Owned<User> kind=class target=User constructor=default
/// @resolution.name source=User target=User
"#,
        r#""#,
    );
}

#[test]
fn test_copyable_value_copies_into_an_owned_target() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

let point = Point { x: 1 };
let owned: ^Point = point;
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

let point: Point = Point { x: 1 };
let owned: ^Point = point;

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

let point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

let owned: ^Point = point;
/// @type.symbol symbol=owned source=owned type=Owned<Point> reduced=Point
/// @resolution.name source=Point target=Point
/// @type.node source=point type=Point
/// @resolution.name source=point target=point

"#,
        r#""#,
    );
}

#[test]
fn test_class_reference_coerces_to_readonly_borrow() {
    let session = TestSession::single(
        r#"
class User {
    name!: string;
}

declare const user: User;
declare function inspect(view: &readonly User): string;

const name = inspect(user);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    name!: string;
}

declare const user: User;
declare function inspect<comptime L0: Lifetime>(view: Borrowed<User, L0, "readonly">): string;

const name: string = inspect(user as Borrowed<User, L0, "readonly">);

=== checked ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name!: string" key=name type=string

    name!: string;
    /// @type.symbol symbol=User.name source="name!: string" type=string

}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.name source=User target=User

declare function inspect(view: &readonly User): string;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(view: &readonly User): string" type=<comptime inspect.L0: Lifetime>(Borrowed<User, inspect.L0, "readonly">) => string
/// @type.symbol symbol=inspect.view source="view: &readonly User" type=Borrowed<User, inspect.L0, "readonly">
/// @resolution.name source=User target=User

const name = inspect(user);
/// @type.symbol symbol=name source=name type=string
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(user) parameters=(Borrowed<User, inspect.L0, "readonly">) arguments=(provided(user) as Borrowed<User, inspect.L0, "readonly">) return=string kind=symbol target=inspect
/// @resolution.name source=user target=user
"#,
        r#"

"#,
    );
}

#[test]
fn test_class_reference_rejects_exclusive_borrow_coercion() {
    let session = TestSession::single(
        r#"
class User {
    name!: string;
}

declare const user: User;
declare function rename(view: &exclusive User): void;

rename(user);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    name!: string;
}

declare const user: User;
declare function rename<comptime L0: Lifetime>(view: Borrowed<User, L0, "exclusive">): void;

rename(user);

=== checked ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name!: string" key=name type=string

    name!: string;
    /// @type.symbol symbol=User.name source="name!: string" type=string

}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.name source=User target=User

declare function rename(view: &exclusive User): void;
/// @generic.template symbol=rename parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=rename source="declare function rename(view: &exclusive User): void" type=<comptime rename.L0: Lifetime>(Borrowed<User, rename.L0, "exclusive">) => void
/// @type.symbol symbol=rename.view source="view: &exclusive User" type=Borrowed<User, rename.L0, "exclusive">
/// @resolution.name source=User target=User

rename(user);
/// @resolution.name source=rename target=rename
/// @resolution.call source=rename(user) parameters=(Borrowed<User, rename.L0, "exclusive">) arguments=(provided(user) as Borrowed<User, rename.L0, "exclusive">) return=void kind=symbol target=rename
/// @resolution.name source=user target=user
"#,
        r#"
/// @diagnostic.error code=EC209 message="argument of type 'User' is not assignable to parameter of type '&exclusive User'"
/// @diagnostic.label line=9 column=8 span="user" line_source="rename(user);"
"#,
    );
}
