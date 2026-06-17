use crate::tests::{DirRows, TestSession};

#[test]
fn test_owned_expression_yields_owned_value() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

let point = ^Point { x: 1 };

point satisfies ^Point;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

let point: Owned<Point> = ^Point { x: 1 };

point satisfies Owned<Point>;

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

point satisfies ^Point;
/// @resolution.name source=point target=point
/// @resolution.name source=Point target=Point
"#,
    );
}

#[test]
fn test_plain_value_does_not_satisfy_owned_destination() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

let point: ^Point = Point { x: 1 };
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

let point: Owned<Point> = Point { x: 1 };

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32
}

let point: ^Point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Owned<Point>
/// @resolution.name source=Point target=Point
/// @resolution.name source=Point target=Point
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'Point' is not assignable to type 'Owned<Point>'"
/// @diagnostic.label line=6 column=5 source="let point: ^Point = Point { x: 1 };"
"#,
    );
}

#[test]
fn test_owned_fields_keep_ownership_form() {
    let session = TestSession::single(
        r#"
struct Data {
    value: int32;
}

struct Container {
    data: ^Data;
}

const container = Container { data: ^Data { value: 1 } };

container.data satisfies ^Data;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Data {
    value: int32;
}

struct Container {
    data: Owned<Data>;
}

const container: Container = Container { data: ^Data { value: 1 } };

container.data satisfies Owned<Data>;

=== checked ===
struct Data {
/// @type.symbol symbol=Data type=Data
/// @definition.struct symbol=Data
/// @definition.field symbol=Data.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Data.value source="value: int32" type=int32
}

struct Container {
/// @type.symbol symbol=Container type=Container
/// @definition.struct symbol=Container
/// @definition.field symbol=Container.data source="data: ^Data" key=data type=Owned<Data>

    data: ^Data;
    /// @type.symbol symbol=Container.data source="data: ^Data" type=Owned<Data>
    /// @resolution.name source=Data target=Data
}

const container = Container { data: ^Data { value: 1 } };
/// @type.symbol symbol=container type=Container
/// @resolution.name source=Container target=Container
/// @resolution.name source=Data target=Data

container.data satisfies ^Data;
/// @resolution.name source=container target=container
/// @resolution.member source=container.data receiver=Container kind=symbol target=Container.data
/// @resolution.name source=Data target=Data
"#,
    );
}

#[test]
fn test_readonly_owned_value_rejects_nested_mutation() {
    let session = TestSession::single(
        r#"
struct Profile {
    name: string;
}

struct User {
    profile: Profile;
}

let user: ^readonly User = ^readonly User {
    profile: Profile { name: "Ada" },
};

user.profile.name = "Grace";
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Profile {
    name: string;
}

struct User {
    profile: Profile;
}

let user: Owned<readonly User> = ^readonly User {
    profile: Profile { name: "Ada" },
};

user.profile.name = "Grace";

=== checked ===
struct Profile {
/// @type.symbol symbol=Profile type=Profile
/// @definition.struct symbol=Profile
/// @definition.field symbol=Profile.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Profile.name source="name: string" type=string
}

struct User {
/// @type.symbol symbol=User type=User
/// @definition.struct symbol=User
/// @definition.field symbol=User.profile source="profile: Profile" key=profile type=Profile

    profile: Profile;
    /// @type.symbol symbol=User.profile source="profile: Profile" type=Profile
    /// @resolution.name source=Profile target=Profile
}

let user: ^readonly User = ^readonly User {
    profile: Profile { name: "Ada" },
};
/// @type.symbol symbol=user source=user type=Owned<readonly User>
/// @resolution.name source=User target=User
/// @resolution.name source=Profile target=Profile

user.profile.name = "Grace";
/// @resolution.name source=user target=user
/// @resolution.member source=user.profile receiver=Owned<readonly User> kind=symbol target=User.profile
/// @resolution.member source=user.profile.name receiver=readonly Profile kind=symbol target=Profile.name
"#,
        r#"
/// @diagnostic.error code=EC214 message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=13 column=1 source="user.profile.name = \"Grace\";"
"#,
    );
}
