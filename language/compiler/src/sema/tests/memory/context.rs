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
/// @type.symbol symbol=User source="class User {}" type=User
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

declare function load(): shared User;

const user = load();
user satisfies shared User;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare function load(): shared User;

const user: shared User = load();
user satisfies shared User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function load(): shared User;
/// @type.symbol symbol=load source="declare function load(): shared User" type=() => shared User
/// @resolution.name source=User target=User

const user = load();
/// @type.symbol symbol=user source=user type=shared User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=load target=load
/// @resolution.call source=load() parameters=() return=shared User kind=symbol target=load

user satisfies shared User;
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=user root=user
/// @resolution.name source=User target=User
"#,
        r#"

"#,
    );
}

#[test]
fn test_infer_shared_binding_placement() {
    let session = TestSession::single(
        r#"
class User {}

declare function load(): User;

shared const user = load();
user satisfies shared User;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare function load(): User;

shared const user: shared User = load();
user satisfies shared User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function load(): User;
/// @type.symbol symbol=load source="declare function load(): User" type=() => User
/// @resolution.name source=User target=User

shared const user = load();
/// @type.symbol symbol=user source=user type=shared User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=load target=load
/// @resolution.call source=load() parameters=() return=User kind=symbol target=load

user satisfies shared User;
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=user root=user
/// @resolution.name source=User target=User
"#,
        r#"

"#,
    );
}

#[test]
fn test_place_aggregate_materialization_from_destination() {
    let session = TestSession::single(
        r#"
struct Point { x: int32; }

const localPoint: local Point = Point { x: 1 };
const sharedPoint: shared Point = Point { x: 2 };
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
const sharedPoint: Point = Point { x: 2 };

=== dir ===
struct Point { x: int32; }
/// @type.symbol symbol=Point source="struct Point { x: int32; }" type=Point
/// @definition.struct symbol=Point source="struct Point { x: int32; }"
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @type.symbol symbol=Point.x source="x: int32" type=int32

const localPoint: local Point = Point { x: 1 };
/// @type.symbol symbol=localPoint source=localPoint type=Point
/// @resolution.pattern source=localPoint kind=binding target=localPoint
/// @resolution.name source=Point target=Point
/// @resolution.name source=Point target=Point

const sharedPoint: shared Point = Point { x: 2 };
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

register({ skip: true } as Argument | undefined);

=== dir ===
type Options = { skip?: boolean };
/// @type.symbol symbol=Options source="type Options = { skip?: boolean }" type={ skip?: boolean }
/// @definition.type symbol=Options source="type Options = { skip?: boolean }" value={ skip?: boolean }
/// @type.symbol symbol=Options.skip source="skip?: boolean" type=boolean

type Argument = Options | (() => void);
/// @type.symbol symbol=Argument source="type Argument = Options | (() => void)" type=Options | Function<(), void>
/// @definition.type symbol=Argument source="type Argument = Options | (() => void)" value=Options | Function<(), void>
/// @resolution.name source=Options target=Options

declare function register(argument?: Argument): void;
/// @type.symbol symbol=register source="declare function register(argument?: Argument): void" type=(Argument | undefined?) => void
/// @type.symbol symbol=register.argument source="argument?: Argument" type=Argument | undefined
/// @resolution.name source=Argument target=Argument

register({ skip: true });
/// @resolution.name source=register target=register
/// @resolution.call source="register({ skip: true })" parameters=(Argument | undefined) arguments=(provided({ skip: true }) as Argument | undefined) return=void kind=symbol target=register
"#,
    );
}

#[test]
fn test_project_field_in_receiver_space() {
    let session = TestSession::single(
        r#"
struct Point { x: int32; }

declare const localPoint: local Point;
declare const sharedPoint: shared Point;

localPoint.x satisfies local int32;
sharedPoint.x satisfies shared int32;
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
declare const sharedPoint: Point;

localPoint.x satisfies local int32;
sharedPoint.x satisfies shared int32;

=== dir ===
struct Point { x: int32; }
/// @type.symbol symbol=Point source="struct Point { x: int32; }" type=Point
/// @definition.struct symbol=Point source="struct Point { x: int32; }"
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @type.symbol symbol=Point.x source="x: int32" type=int32

declare const localPoint: local Point;
/// @type.symbol symbol=localPoint source=localPoint type=Point
/// @resolution.pattern source=localPoint kind=binding target=localPoint
/// @resolution.name source=Point target=Point

declare const sharedPoint: shared Point;
/// @type.symbol symbol=sharedPoint source=sharedPoint type=Point
/// @resolution.pattern source=sharedPoint kind=binding target=sharedPoint
/// @resolution.name source=Point target=Point

localPoint.x satisfies local int32;
/// @resolution.name source=localPoint target=localPoint
/// @resolution.member source=localPoint.x receiver=Point type=int32 kind=field target_receiver=Point key=x target=Point.x target_type=int32
/// @resolution.place source=localPoint placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=localPoint root=localPoint
/// @resolution.place source=localPoint.x placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=localPoint.x root=localPoint keys=[x]

sharedPoint.x satisfies shared int32;
/// @resolution.name source=sharedPoint target=sharedPoint
/// @resolution.member source=sharedPoint.x receiver=Point type=int32 kind=field target_receiver=Point key=x target=Point.x target_type=int32
/// @resolution.place source=sharedPoint placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=sharedPoint root=sharedPoint
/// @resolution.place source=sharedPoint.x placement="constant" lifetime="static" access="readonly"
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

struct State { user: shared User; }

declare const state: local State;
state.user satisfies shared User;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct State {
    user: shared User;
}

declare const state: State;
state.user satisfies shared User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

struct State { user: shared User; }
/// @type.symbol symbol=State source="struct State { user: shared User; }" type=State
/// @definition.struct symbol=State source="struct State { user: shared User; }"
/// @definition.field symbol=State.user source="user: shared User" key=user type=shared User
/// @type.symbol symbol=State.user source="user: shared User" type=shared User
/// @resolution.name source=User target=User

declare const state: local State;
/// @type.symbol symbol=state source=state type=State
/// @resolution.pattern source=state kind=binding target=state
/// @resolution.name source=State target=State

state.user satisfies shared User;
/// @resolution.name source=state target=state
/// @resolution.member source=state.user receiver=State type=shared User kind=field target_receiver=State key=user target=State.user target_type=shared User
/// @resolution.place source=state placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=state root=state
/// @resolution.place source=state.user placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=state.user root=state keys=[user]
/// @resolution.name source=User target=User
"#,
        r#"
"#,
    );
}

#[test]
fn test_project_index_in_receiver_space() {
    let session = TestSession::single(
        r#"
declare const values: shared [int32; 2];
values[0] satisfies shared int32;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const values: [int32; 2];
values[0] satisfies shared int32;

=== dir ===
declare const values: shared [int32; 2];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 2>
/// @resolution.pattern source=values kind=binding target=values

values[0] satisfies shared int32;
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=values root=values
/// @resolution.place source=values[0] placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=values[0] root=values keys=[0]
/// @resolution.subscript source=values[0] type=int32 kind=call target="index#1(parameters=(isize), arguments=(provided(0) as isize), return=WithAccess<&'static constant int32, \"readonly\">)"
/// @generic.instantiation id="index#1<int32, 2, \"readonly\">" template=index#1 arguments=(int32, 2, "readonly")
"#,
        r#"

"#,
    );
}

#[test]
fn test_copy_shared_scalar_into_local_binding() {
    let session = TestSession::single(
        r#"
declare const source: shared int32;
const value = source;
value satisfies local int32;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const source: int32;
const value: int32 = source;
value satisfies local int32;

=== dir ===
declare const source: shared int32;
/// @type.symbol symbol=source source=source type=int32
/// @resolution.pattern source=source kind=binding target=source

const value = source;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=source target=source
/// @resolution.access source=source root=source

value satisfies local int32;
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="constant" lifetime="static" access="readonly"
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
/// @type.symbol symbol=World source="class World {}" type=World
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
/// @resolution.place source=world placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=world root=world
"#,
        r#"

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

const transform: (arg0: User) => User = (value: User): User => value;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

type Transform = (value: User) => User;
/// @type.symbol symbol=Transform source="type Transform = (value: User) => User" type=Function<(User,), User>
/// @definition.type symbol=Transform source="type Transform = (value: User) => User" value=Function<(User,), User>
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

Registry.current satisfies local User;
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

Registry.current satisfies local User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

class Registry {
/// @type.symbol symbol=Registry type=Registry
/// @definition.class symbol=Registry
/// @definition.field symbol=Registry.current source="static current: User = new User()" key=current static=true type=User

    static current: User = new User();
    /// @type.symbol symbol=Registry.current source="static current: User = new User()" type=User
    /// @resolution.name source=User target=User
    /// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
    /// @resolution.name source=User target=User

}

Registry.current satisfies local User;
/// @resolution.name source=Registry target=Registry
/// @resolution.member source=Registry.current receiver=Registry type=User kind=field target_receiver=Registry key=current target=Registry.current target_type=User
/// @resolution.place source=Registry.current placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=Registry.current root=Registry keys=[current]
/// @resolution.name source=User target=User
"#,
    );
}
