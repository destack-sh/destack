use crate::tests::{DirRows, TestSession};

#[test]
fn test_yield_owned_value_from_move_expression() {
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

let point: ^Point = ^Point { x: 1 };

point satisfies ^Point;

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
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source="^Point { x: 1 }" type=Owned<Point> reduced=Point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

point satisfies ^Point;
/// @type.node source="point satisfies ^Point" type=Owned<Point> reduced=Point
/// @type.node source=point type=Owned<Point> reduced=Point
/// @resolution.name source=point target=point
/// @resolution.name source=Point target=Point
"#,
    );
}

#[test]
fn test_materialize_plain_value_into_owned_destination() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

let point: ^Point = Point { x: 1 };
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

let point: ^Point = Point { x: 1 };

=== checked ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

let point: ^Point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Owned<Point> reduced=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_preserve_owned_form_on_fields() {
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
    data: ^Data;
}

const container: Container = Container { data: ^Data { value: 1 } };

container.data satisfies ^Data;

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
    /// @type.symbol symbol=Container.data source="data: ^Data" type=Owned<Data> reduced=Data
    /// @resolution.name source=Data target=Data

}

const container = Container { data: ^Data { value: 1 } };
/// @type.symbol symbol=container source=container type=Container
/// @resolution.pattern source=container kind=binding target=container
/// @type.node source="Container { data: ^Data { value: 1 } }" type=Container
/// @resolution.name source=Container target=Container
/// @type.node source="^Data { value: 1 }" type=Owned<Data> reduced=Data
/// @type.node source="Data { value: 1 }" type=Data
/// @resolution.name source=Data target=Data
/// @type.node source=1 type=1

container.data satisfies ^Data;
/// @type.node source="container.data satisfies ^Data" type=Owned<Data> reduced=Data
/// @type.node source=container type=Container
/// @type.node source=container.data type=Owned<Data> reduced=Data
/// @resolution.name source=container target=container
/// @resolution.member source=container.data receiver=Container kind=symbol target=Container.data
/// @resolution.name source=Data target=Data
"#,
    );
}

#[test]
fn test_reject_nested_mutation_of_readonly_owned_value() {
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

let user: ^readonly User = ^readonly User {
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
/// @type.symbol symbol=user source=user type=Owned<Readonly<User>> reduced=Readonly<User>
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User
/// @type.node type=Owned<Readonly<User>> reduced=Readonly<User>
/// @type.node type=User
/// @resolution.name source=User target=User

    profile: Profile { name: "Ada" },
    /// @type.node source="Profile { name: \"Ada\" }" type=Profile
    /// @resolution.name source=Profile target=Profile
    /// @type.node source="\"Ada\"" type="Ada"

};

user.profile.name = "Grace";
/// @type.node source="user.profile.name = \"Grace\"" type="Grace"
/// @type.node source=user type=Owned<Readonly<User>> reduced=Readonly<User>
/// @type.node source=user.profile type=Profile
/// @type.node source=user.profile.name type=string
/// @resolution.name source=user target=user
/// @resolution.member source=user.profile receiver=Readonly<User> kind=symbol target=User.profile
/// @resolution.pattern.assign source=user.profile.name kind=place place=field(Profile.name) type=string
/// @type.node source="\"Grace\"" type="Grace"
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=14 column=14 span="name" line_source="user.profile.name = \"Grace\";"
"#,
    );
}
