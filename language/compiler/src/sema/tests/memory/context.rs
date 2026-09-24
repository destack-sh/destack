use crate::tests::{DirRows, TestSession};

#[test]
fn test_default_managed_bindings_to_local_space() {
    let session = TestSession::single(
        r#"
class User {}

declare function load(): User;

const user = load();
let copy = user;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare function load(): User;

const user: User = load();
let copy: User = user;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function load(): User;
/// @type.symbol symbol=load source="declare function load(): User" type=() => User
/// @resolution.name source=User target=User

const user = load();
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=load target=load
/// @resolution.call source=load() parameters=() return=User kind=symbol target=load

let copy = user;
/// @type.symbol symbol=copy source=copy type=User
/// @resolution.pattern source=copy kind=binding target=copy
/// @resolution.name source=user target=user
/// @resolution.access source=user root=user
"#,
        r#"
"#,
    );
}

#[test]
fn test_preserve_explicit_shared_placement_in_local_binding() {
    let session = TestSession::single(
        r#"
class User {}

shared class SharedUser {}

declare function load(): SharedUser;

const user = load();
user satisfies SharedUser;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared class SharedUser {}

declare function load(): SharedUser;

const user: SharedUser = load();
user satisfies SharedUser;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

shared class SharedUser {}
/// @type.symbol symbol=SharedUser source="shared class SharedUser {}" type=typeof SharedUser
/// @definition.class symbol=SharedUser source="shared class SharedUser {}"

declare function load(): SharedUser;
/// @type.symbol symbol=load source="declare function load(): SharedUser" type=() => SharedUser
/// @resolution.name source=SharedUser target=SharedUser

const user = load();
/// @type.symbol symbol=user source=user type=SharedUser
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=load target=load
/// @resolution.call source=load() parameters=() return=SharedUser kind=symbol target=load

user satisfies SharedUser;
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @resolution.name source=SharedUser target=SharedUser
"#,
        r#"

"#,
    );
}

/// Reject a local class handle in a shared binding.
#[test]
fn test_reject_a_local_class_handle_in_a_shared_binding() {
    let session = TestSession::single(
        r#"
class User {}

shared class SharedUser {}

declare function load(): User;

shared const user = load();
user satisfies SharedUser;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared class SharedUser {}

declare function load(): User;

shared const user: User = load();
user satisfies SharedUser;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

shared class SharedUser {}
/// @type.symbol symbol=SharedUser source="shared class SharedUser {}" type=typeof SharedUser
/// @definition.class symbol=SharedUser source="shared class SharedUser {}"

declare function load(): User;
/// @type.symbol symbol=load source="declare function load(): User" type=() => User
/// @resolution.name source=User target=User

shared const user = load();
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=load target=load
/// @resolution.call source=load() parameters=() return=User kind=symbol target=load

user satisfies SharedUser;
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @resolution.name source=SharedUser target=SharedUser
"#,
        r#"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=8 column=14 span="user" line_source="shared const user = load();"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
/// @diagnostic.error id=constraint-not-satisfied message="type 'User' does not satisfy 'SharedUser'"
/// @diagnostic.label line=9 column=1 span="user" line_source="user satisfies SharedUser;"
"#,
    );
}

#[test]
fn test_place_aggregate_materialization_from_destination() {
    let session = TestSession::single(
        r#"
struct Point { x: int32; }

const localPoint: Point = Point { x: 1 };
shared const sharedPoint: Point = Point { x: 2 };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

const localPoint: Point = Point { x: 1 };
shared const sharedPoint: Point = Point { x: 2 };

=== dir ===
struct Point { x: int32; }
/// @type.symbol symbol=Point source="struct Point { x: int32; }" type=Point
/// @definition.struct symbol=Point source="struct Point { x: int32; }"
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @type.symbol symbol=Point.x source="x: int32" type=int32

const localPoint: Point = Point { x: 1 };
/// @type.symbol symbol=localPoint source=localPoint type=Point
/// @resolution.pattern source=localPoint kind=binding target=localPoint
/// @resolution.name source=Point target=Point
/// @resolution.name source=Point target=Point

shared const sharedPoint: Point = Point { x: 2 };
/// @type.symbol symbol=sharedPoint source=sharedPoint type=Point
/// @resolution.pattern source=sharedPoint kind=binding target=sharedPoint
/// @resolution.name source=Point target=Point
/// @resolution.name source=Point target=Point
"#,
        r#"

"#,
    );
}

#[test]
fn test_place_contextual_object_literal_from_optional_destination() {
    let session = TestSession::single(
        r#"
type Options = { skip?: boolean };

type Argument = Options | (() => void);

declare function register(argument?: Argument): void;

register({ skip: true });
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Options = { skip?: boolean };

type Argument = Options | (() => void);

declare function register(argument?: Argument): void;

register({ skip: true as boolean | undefined } as { skip?: boolean } | (() => void) | undefined);

=== dir ===
type Options = { skip?: boolean };
/// @type.symbol symbol=Options source="type Options = { skip?: boolean }" type={ skip?: boolean }
/// @definition.type symbol=Options source="type Options = { skip?: boolean }" value={ skip?: boolean }
/// @type.symbol symbol=Options.skip source="skip?: boolean" type=boolean

type Argument = Options | (() => void);
/// @type.symbol symbol=Argument source="type Argument = Options | (() => void)" type={ skip?: boolean } | () => void
/// @definition.type symbol=Argument source="type Argument = Options | (() => void)" value=Options | () => void
/// @resolution.name source=Options target=Options

declare function register(argument?: Argument): void;
/// @type.symbol symbol=register source="declare function register(argument?: Argument): void" type=({ skip?: boolean } | () => void | undefined?) => void
/// @resolution.name source=Argument target=Argument

register({ skip: true });
/// @resolution.name source=register target=register
/// @resolution.call source="register({ skip: true })" parameters=({ skip?: boolean } | () => void | undefined) arguments=(provided({ skip: true }) as { skip?: boolean } | () => void | undefined) return=void kind=symbol target=register
"#,
    );
}

#[test]
fn test_project_field_in_receiver_space() {
    let session = TestSession::single(
        r#"
struct Point { x: int32; }

declare const localPoint: Point;
declare shared const sharedPoint: Point;

localPoint.x;
sharedPoint.x;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

declare const localPoint: Point;
declare shared const sharedPoint: Point;

localPoint.x;
sharedPoint.x;

=== dir ===
struct Point { x: int32; }
/// @type.symbol symbol=Point source="struct Point { x: int32; }" type=Point
/// @definition.struct symbol=Point source="struct Point { x: int32; }"
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @type.symbol symbol=Point.x source="x: int32" type=int32

declare const localPoint: Point;
/// @type.symbol symbol=localPoint source=localPoint type=Point
/// @resolution.pattern source=localPoint kind=binding target=localPoint
/// @resolution.name source=Point target=Point

declare shared const sharedPoint: Point;
/// @type.symbol symbol=sharedPoint source=sharedPoint type=Point
/// @resolution.pattern source=sharedPoint kind=binding target=sharedPoint
/// @resolution.name source=Point target=Point

localPoint.x;
/// @resolution.name source=localPoint target=localPoint
/// @resolution.member source=localPoint.x receiver=Point type=int32 kind=field target_receiver=Point key=x target=Point.x target_type=int32
/// @resolution.place source=localPoint placement="local" lifetime="static" access="immutable"
/// @resolution.access source=localPoint root=localPoint
/// @resolution.place source=localPoint.x placement="local" lifetime="static" access="immutable"
/// @resolution.access source=localPoint.x root=localPoint keys=[x]

sharedPoint.x;
/// @resolution.name source=sharedPoint target=sharedPoint
/// @resolution.member source=sharedPoint.x receiver=Point type=int32 kind=field target_receiver=Point key=x target=Point.x target_type=int32
/// @resolution.place source=sharedPoint placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=sharedPoint root=sharedPoint
/// @resolution.place source=sharedPoint.x placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=sharedPoint.x root=sharedPoint keys=[x]
"#,
        r#"
"#,
    );
}

#[test]
fn test_preserve_explicit_shared_field_in_local_aggregate() {
    let session = TestSession::single(
        r#"
class User {}

shared class SharedUser {}

struct State { user: SharedUser; }

declare const state: State;
state.user satisfies SharedUser;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared class SharedUser {}

struct State {
    user: SharedUser;
}

declare const state: State;
state.user satisfies SharedUser;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

shared class SharedUser {}
/// @type.symbol symbol=SharedUser source="shared class SharedUser {}" type=typeof SharedUser
/// @definition.class symbol=SharedUser source="shared class SharedUser {}"

struct State { user: SharedUser; }
/// @type.symbol symbol=State source="struct State { user: SharedUser; }" type=State
/// @definition.struct symbol=State source="struct State { user: SharedUser; }"
/// @definition.field symbol=State.user source="user: SharedUser" key=user type=SharedUser
/// @type.symbol symbol=State.user source="user: SharedUser" type=SharedUser
/// @resolution.name source=SharedUser target=SharedUser

declare const state: State;
/// @type.symbol symbol=state source=state type=State
/// @resolution.pattern source=state kind=binding target=state
/// @resolution.name source=State target=State

state.user satisfies SharedUser;
/// @resolution.name source=state target=state
/// @resolution.member source=state.user receiver=State type=SharedUser kind=field target_receiver=State key=user target=State.user target_type=SharedUser
/// @resolution.place source=state placement="local" lifetime="static" access="immutable"
/// @resolution.access source=state root=state
/// @resolution.place source=state.user placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=state.user root=state keys=[user]
/// @resolution.name source=SharedUser target=SharedUser
"#,
        r#"
"#,
    );
}

#[test]
fn test_project_index_in_receiver_space() {
    let session = TestSession::single(
        r#"
declare shared const values: [int32; 2];
values[0];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare shared const values: [int32; 2];
values[0];

=== dir ===
declare shared const values: [int32; 2];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 2>
/// @resolution.pattern source=values kind=binding target=values

values[0];
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[0] type=int32 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=int32, regions=(\"static\" & \"shared\"))"
/// @generic.instantiation id="index#2<int32, 2, \"static\" & \"shared\">" template=index#2 arguments=(int32, 2, "static" & "shared")
"#,
        r#"

"#,
    );
}

#[test]
fn test_copy_shared_scalar_into_local_binding() {
    let session = TestSession::single(
        r#"
declare shared const source: int32;
const value = source;
value satisfies int32;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare shared const source: int32;
const value: int32 = source;
value satisfies int32;

=== dir ===
declare shared const source: int32;
/// @type.symbol symbol=source source=source type=int32
/// @resolution.pattern source=source kind=binding target=source

const value = source;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=source target=source
/// @resolution.access source=source root=source

value satisfies int32;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
        r#"
"#,
    );
}

#[test]
fn test_reject_relocating_local_owned_value_into_shared_binding() {
    let session = TestSession::single(
        r#"
class World {}
declare const world: ^World;

shared const sharedWorld: ^World = world;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class World {}
declare const world: ^World;

shared const sharedWorld: ^World = world;

=== dir ===
class World {}
/// @type.symbol symbol=World source="class World {}" type=typeof World
/// @definition.class symbol=World source="class World {}"

declare const world: ^World;
/// @type.symbol symbol=world source=world type=^World
/// @resolution.pattern source=world kind=binding target=world
/// @resolution.name source=World target=World

shared const sharedWorld: ^World = world;
/// @type.symbol symbol=sharedWorld source=sharedWorld type=^World
/// @resolution.pattern source=sharedWorld kind=binding target=sharedWorld
/// @resolution.name source=World target=World
/// @resolution.name source=world target=world
/// @resolution.place source=world placement="local" lifetime="static" access="immutable"
/// @resolution.access source=world root=world
"#,
        r#"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=5 column=14 span="sharedWorld" line_source="shared const sharedWorld: ^World = world;"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
"#,
    );
}

#[test]
fn test_default_nested_callable_signature_and_closure_to_local() {
    let session = TestSession::single(
        r#"
class User {}

type Transform = (value: User) => User;

const transform = (value: User): User => value;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

type Transform = (value: User) => User;

const transform: (value: User) => User = (value: User): User => value;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

type Transform = (value: User) => User;
/// @type.symbol symbol=Transform source="type Transform = (value: User) => User" type=(User) => User
/// @definition.type symbol=Transform source="type Transform = (value: User) => User" value=(User) => User
/// @type.symbol symbol=Transform.value source="value: User" type=User
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

const transform = (value: User): User => value;
/// @type.symbol symbol=transform source=transform type=Function<(User,), User, "readonly">
/// @resolution.pattern source=transform kind=binding target=transform
/// @type.symbol symbol=symbol4 source="(value: User): User => value" type=Function<(User,), User, "readonly">
/// @type.symbol symbol=symbol4.value source="value: User" type=User
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User
/// @resolution.name source=value target=symbol4.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol4.value
"#,
    );
}

#[test]
fn test_default_static_member_to_local() {
    let session = TestSession::single(
        r#"
class User {}

class Registry {
    static current: User = new User();
}

Registry.current satisfies User;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

class Registry {
    static current: User = new User();
}

Registry.current satisfies User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

class Registry {
/// @type.symbol symbol=Registry type=typeof Registry
/// @definition.class symbol=Registry
/// @definition.field symbol=Registry.current source="static current: User = new User()" key=current static=true type=User

    static current: User = new User();
    /// @type.symbol symbol=Registry.current source="static current: User = new User()" type=User
    /// @resolution.name source=User target=User
    /// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
    /// @resolution.name source=User target=User

}

Registry.current satisfies User;
/// @resolution.name source=Registry target=Registry
/// @resolution.member source=Registry.current receiver=typeof Registry type=User kind=field target_receiver=typeof Registry key=current target=Registry.current target_type=User
/// @resolution.place source=Registry.current placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=Registry.current root=Registry keys=[current]
/// @resolution.name source=User target=User
"#,
    );
}
