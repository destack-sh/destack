use crate::tests::{DirRows, TestSession};

#[test]
fn test_yield_owned_value_from_move_expression() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

let point: ^Point = Point { x: 1 };

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

let point: ^Point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Owned<Point>
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point
/// @type.node source="Point { x: 1 }" type=Owned<Point>
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

point satisfies ^Point;
/// @type.node source="point satisfies ^Point" type=Owned<Point>
/// @type.node source=point type=Owned<Point>
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point
/// @resolution.name source=Point target=Point
"#,
    );
}

#[test]
fn test_materialize_struct_call_result_into_owned_destination() {
    let session = TestSession::single(
        r#"
struct Buffer {
    values: int32[];
}

declare function makeBuffer(): Buffer;

let buffer: ^Buffer = makeBuffer();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Buffer {
    values: int32[];
}

declare function makeBuffer(): Buffer;

let buffer: ^Buffer = makeBuffer();

=== checked ===
struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.values source="values: int32[]" key=values type=Array<int32>

    values: int32[];
    /// @type.symbol symbol=Buffer.values source="values: int32[]" type=Array<int32>

}

declare function makeBuffer(): Buffer;
/// @type.symbol symbol=makeBuffer source="declare function makeBuffer(): Buffer" type=() => Buffer
/// @resolution.name source=Buffer target=Buffer

let buffer: ^Buffer = makeBuffer();
/// @type.symbol symbol=buffer source=buffer type=Owned<Buffer>
/// @resolution.pattern source=buffer kind=binding target=buffer
/// @resolution.name source=Buffer target=Buffer
/// @type.node source=makeBuffer type=() => Buffer
/// @type.node source=makeBuffer() type=Buffer
/// @resolution.name source=makeBuffer target=makeBuffer
/// @resolution.call source=makeBuffer() parameters=() return=Buffer kind=symbol target=makeBuffer
"#,
    );
}

#[test]
fn test_reject_managed_call_result_as_owned() {
    let session = TestSession::single(
        r#"
class User {}

declare function makeUser(): User;

let user: ^User = makeUser();
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}

declare function makeUser(): User;

let user: ^User = makeUser();

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function makeUser(): User;
/// @type.symbol symbol=makeUser source="declare function makeUser(): User" type=() => User
/// @resolution.name source=User target=User

let user: ^User = makeUser();
/// @type.symbol symbol=user source=user type=Owned<User>
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User
/// @type.node source=makeUser type=() => User
/// @type.node source=makeUser() type=User
/// @resolution.name source=makeUser target=makeUser
/// @resolution.call source=makeUser() parameters=() return=User kind=symbol target=makeUser
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'User' is not assignable to type '^User'"
/// @diagnostic.label line=6 column=19 span="makeUser()" line_source="let user: ^User = makeUser();"
/// @diagnostic.related line=6 column=11 span="^" line_source="let user: ^User = makeUser();" message="expected due to this annotation"
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

const container = Container { data: Data { value: 1 } };

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
    /// @type.symbol symbol=Container.data source="data: ^Data" type=Owned<Data>
    /// @resolution.name source=Data target=Data

}

const container = Container { data: Data { value: 1 } };
/// @type.symbol symbol=container source=container type=Container
/// @resolution.pattern source=container kind=binding target=container
/// @type.node source="Container { data: Data { value: 1 } }" type=Container
/// @resolution.name source=Container target=Container
/// @type.node source="Data { value: 1 }" type=Owned<Data>
/// @resolution.name source=Data target=Data
/// @type.node source=1 type=1

container.data satisfies ^Data;
/// @type.node source="container.data satisfies ^Data" type=Owned<Data>
/// @type.node source=container type=Container
/// @type.node source=container.data type=Owned<Data>
/// @resolution.name source=container target=container
/// @resolution.member source=container.data receiver=Container type=Owned<Data> kind=field target_receiver=Container key=data target=Container.data target_type=Owned<Data>
/// @resolution.place source=container placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=container root=container
/// @resolution.place source=container.data placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=container.data root=container keys=[data]
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

let user: ^readonly User = User {
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

let user: ^readonly User = User {
/// @type.symbol symbol=user source=user type=Owned<Readonly<User>>
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User
/// @type.node type=Owned<Readonly<User>>
/// @resolution.name source=User target=User

    profile: Profile { name: "Ada" },
    /// @type.node source="Profile { name: \"Ada\" }" type=Profile
    /// @resolution.name source=Profile target=Profile
    /// @type.node source="\"Ada\"" type="Ada"

};

user.profile.name = "Grace";
/// @type.node source="user.profile.name = \"Grace\"" type="Grace"
/// @type.node source=user type=Owned<Readonly<User>>
/// @type.node source=user.profile type=Profile
/// @type.node source=user.profile.name type=Readonly<string>
/// @resolution.name source=user target=user
/// @resolution.member source=user.profile receiver=Owned<Readonly<User>> type=Profile kind=field target_receiver=Owned<Readonly<User>> key=profile target=User.profile target_type=Profile
/// @resolution.place source=user placement="local" lifetime="static" access="readonly"
/// @resolution.access source=user root=user
/// @resolution.place source=user.profile placement="local" lifetime="static" access="readonly"
/// @resolution.access source=user.profile root=user keys=[profile]
/// @resolution.pattern.assign source=user.profile.name kind=place
/// @resolution.access source=user.profile.name root=user keys=[profile, name]
/// @resolution.assignment source=user.profile.name write="receiver=Readonly<Profile>, target=field(receiver=Readonly<Profile>, target=Profile.name, type=Readonly<string>), type=Readonly<string>" type=Readonly<string>
/// @type.node source="\"Grace\"" type="Grace"
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=14 column=14 span="name" line_source="user.profile.name = \"Grace\";"
"#,
    );
}
