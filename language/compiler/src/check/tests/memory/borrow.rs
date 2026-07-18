use crate::tests::{DirRows, TestSession};

#[test]
fn test_preserve_source_place_in_explicit_borrow() {
    let session = TestSession::single(
        r#"
class User {}

declare const localUser: local User;
declare const sharedUser: shared User;

const localView = &readonly localUser;
const sharedView = &readonly sharedUser;

localView satisfies local &readonly User;
sharedView satisfies shared &readonly User;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_node_types().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const localUser: local User;
declare const sharedUser: shared User;

const localView: local Borrowed<User, "static", "readonly"> = &readonly localUser;
const sharedView: shared Borrowed<User, "static", "readonly"> = &readonly sharedUser;

localView satisfies local &readonly User;
sharedView satisfies shared &readonly User;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=Placed<User, "local">
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=Placed<User, "shared">
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

const localView = &readonly localUser;
/// @type.symbol symbol=localView source=localView type=Placed<Borrowed<User, "static", "readonly">, "local">
/// @resolution.pattern source=localView kind=binding target=localView
/// @type.node source="&readonly localUser" type=Placed<Borrowed<User, "static", "readonly">, "local">
/// @resolution.name source=localUser target=localUser

const sharedView = &readonly sharedUser;
/// @type.symbol symbol=sharedView source=sharedView type=Placed<Borrowed<User, "static", "readonly">, "shared">
/// @resolution.pattern source=sharedView kind=binding target=sharedView
/// @type.node source="&readonly sharedUser" type=Placed<Borrowed<User, "static", "readonly">, "shared">
/// @resolution.name source=sharedUser target=sharedUser

localView satisfies local &readonly User;
/// @type.node source="localView satisfies local &readonly User" type=Placed<Borrowed<User, "static", "readonly">, "local">
/// @resolution.name source=localView target=localView
/// @resolution.name source=User target=User

sharedView satisfies shared &readonly User;
/// @type.node source="sharedView satisfies shared &readonly User" type=Placed<Borrowed<User, "static", "readonly">, "shared">
/// @resolution.name source=sharedView target=sharedView
/// @resolution.name source=User target=User
"#,
    );
}

#[test]
fn test_distinguish_borrow_expression_from_explicit_borrow_coercion() {
    let session = TestSession::single(
        r#"
class User {}

declare const user: local User;

const projected = &readonly user;
const coerced = user as local &readonly User;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_node_types().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const user: local User;

const projected: local Borrowed<User, "static", "readonly"> = &readonly user;
const coerced: local Borrowed<User, "static", "readonly"> = user as local &readonly User;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const user: local User;
/// @type.symbol symbol=user source=user type=Placed<User, "local">
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const projected = &readonly user;
/// @type.symbol symbol=projected source=projected type=Placed<Borrowed<User, "static", "readonly">, "local">
/// @resolution.pattern source=projected kind=binding target=projected
/// @type.node source="&readonly user" type=Placed<Borrowed<User, "static", "readonly">, "local">
/// @resolution.name source=user target=user

const coerced = user as local &readonly User;
/// @type.symbol symbol=coerced source=coerced type=Placed<Borrowed<User, "static", "readonly">, "local">
/// @resolution.pattern source=coerced kind=binding target=coerced
/// @type.node source="user as local &readonly User" type=Placed<Borrowed<User, "static", "readonly">, "local">
/// @resolution.name source=user target=user
/// @resolution.name source=User target=User
"#,
    );
}

#[test]
fn test_reject_explicit_borrow_access_not_granted_by_managed_source() {
    let session = TestSession::single(
        r#"
class User {}

declare const sharedUser: shared User;
declare const readonlyUser: local readonly User;

const sharedExclusive = &exclusive sharedUser;
const readonlyMutable = &readonlyUser;
const readonlyExclusive = &exclusive readonlyUser;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const sharedUser: shared User;
declare const readonlyUser: local readonly User;

const sharedExclusive: shared Borrowed<User, "static", "exclusive"> = &exclusive sharedUser;
const readonlyMutable: local Borrowed<User, "static", "readonly"> = &readonlyUser;
const readonlyExclusive: local Borrowed<User, "static", "readonly"> = &exclusive readonlyUser;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=Placed<User, "shared">
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

declare const readonlyUser: local readonly User;
/// @type.symbol symbol=readonlyUser source=readonlyUser type=Placed<Readonly<User>, "local">
/// @resolution.pattern source=readonlyUser kind=binding target=readonlyUser
/// @resolution.name source=User target=User

const sharedExclusive = &exclusive sharedUser;
/// @type.symbol symbol=sharedExclusive source=sharedExclusive type=Placed<Borrowed<User, "static", "exclusive">, "shared">
/// @resolution.pattern source=sharedExclusive kind=binding target=sharedExclusive
/// @resolution.name source=sharedUser target=sharedUser

const readonlyMutable = &readonlyUser;
/// @type.symbol symbol=readonlyMutable source=readonlyMutable type=Placed<Borrowed<User, "static", "readonly">, "local">
/// @resolution.pattern source=readonlyMutable kind=binding target=readonlyMutable
/// @resolution.name source=readonlyUser target=readonlyUser

const readonlyExclusive = &exclusive readonlyUser;
/// @type.symbol symbol=readonlyExclusive source=readonlyExclusive type=Placed<Borrowed<User, "static", "readonly">, "local">
/// @resolution.pattern source=readonlyExclusive kind=binding target=readonlyExclusive
/// @resolution.name source=readonlyUser target=readonlyUser
"#,
        r#"
/// @diagnostic.error id=borrow-access-not-granted message="'exclusive' access is not granted by a value of type 'shared User'"
/// @diagnostic.label line=7 column=25 span="&" line_source="const sharedExclusive = &exclusive sharedUser;"
/// @diagnostic.note message="the source grants at most 'mutable' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
/// @diagnostic.error id=borrow-access-not-granted message="'mutable' access is not granted by a value of type 'local readonly User'"
/// @diagnostic.label line=8 column=25 span="&" line_source="const readonlyMutable = &readonlyUser;"
/// @diagnostic.note message="the source grants at most 'readonly' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
/// @diagnostic.error id=borrow-access-not-granted message="'exclusive' access is not granted by a value of type 'local readonly User'"
/// @diagnostic.label line=9 column=27 span="&" line_source="const readonlyExclusive = &exclusive readonlyUser;"
/// @diagnostic.note message="the source grants at most 'readonly' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
"#,
    );
}

#[test]
fn test_assign_stronger_borrow_access_to_weaker_access() {
    let session = TestSession::single(
        r#"
class User {}

declare const user: local User;
declare function inspect(value: local &readonly User): void;
declare function modify(value: local &User): void;

inspect(&user);
modify(&user);
inspect(&exclusive user);
modify(&exclusive user);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_node_types().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const user: local User;
declare function inspect<comptime L0: Lifetime>(value: local Borrowed<User, L0, "readonly">): void;
declare function modify<comptime L0: Lifetime>(value: local Borrowed<User, L0, "mutable">): void;

inspect(&user);
modify(&user);
inspect(&exclusive user);
modify(&exclusive user);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const user: local User;
/// @type.symbol symbol=user source=user type=Placed<User, "local">
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

declare function inspect(value: local &readonly User): void;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(value: local &readonly User): void" type=<comptime inspect.L0: Lifetime>(Placed<Borrowed<User, inspect.L0, "readonly">, "local">) => void
/// @type.symbol symbol=inspect.value source="value: local &readonly User" type=Placed<Borrowed<User, inspect.L0, "readonly">, "local">
/// @resolution.name source=User target=User

declare function modify(value: local &User): void;
/// @generic.template symbol=modify parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=modify source="declare function modify(value: local &User): void" type=<comptime modify.L0: Lifetime>(Placed<Borrowed<User, modify.L0, "mutable">, "local">) => void
/// @type.symbol symbol=modify.value source="value: local &User" type=Placed<Borrowed<User, modify.L0, "mutable">, "local">
/// @resolution.name source=User target=User

inspect(&user);
/// @type.node source=inspect(&user) type=void
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(&user) parameters=(Placed<Borrowed<User, "static", "readonly">, "local">) arguments=(provided(&user) as Placed<Borrowed<User, "static", "readonly">, "local">) return=void kind=symbol target=inspect
/// @type.node source=&user type=Placed<Borrowed<User, "static", "mutable">, "local">
/// @resolution.name source=user target=user

modify(&user);
/// @type.node source=modify(&user) type=void
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(&user) parameters=(Placed<Borrowed<User, "static", "mutable">, "local">) arguments=(provided(&user) as Placed<Borrowed<User, "static", "mutable">, "local">) return=void kind=symbol target=modify
/// @type.node source=&user type=Placed<Borrowed<User, "static", "mutable">, "local">
/// @resolution.name source=user target=user

inspect(&exclusive user);
/// @type.node source="inspect(&exclusive user)" type=void
/// @resolution.name source=inspect target=inspect
/// @resolution.call source="inspect(&exclusive user)" parameters=(Placed<Borrowed<User, "static", "readonly">, "local">) arguments=(provided(&exclusive user) as Placed<Borrowed<User, "static", "readonly">, "local">) return=void kind=symbol target=inspect
/// @type.node source="&exclusive user" type=Placed<Borrowed<User, "static", "exclusive">, "local">
/// @resolution.name source=user target=user

modify(&exclusive user);
/// @type.node source="modify(&exclusive user)" type=void
/// @resolution.name source=modify target=modify
/// @resolution.call source="modify(&exclusive user)" parameters=(Placed<Borrowed<User, "static", "mutable">, "local">) arguments=(provided(&exclusive user) as Placed<Borrowed<User, "static", "mutable">, "local">) return=void kind=symbol target=modify
/// @type.node source="&exclusive user" type=Placed<Borrowed<User, "static", "exclusive">, "local">
/// @resolution.name source=user target=user
"#,
    );
}

#[test]
fn test_reject_borrow_access_upgrades() {
    let session = TestSession::single(
        r#"
class User {}

declare const readonlyView: local &readonly User;
declare const mutableView: local &User;
declare function modify(value: local &User): void;
declare function replace(value: local &exclusive User): void;

modify(readonlyView);
replace(readonlyView);
replace(mutableView);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const readonlyView: local Borrowed<User, "static", "readonly">;
declare const mutableView: local Borrowed<User, "static", "mutable">;
declare function modify<comptime L0: Lifetime>(value: local Borrowed<User, L0, "mutable">): void;
declare function replace<comptime L0: Lifetime>(value: local Borrowed<User, L0, "exclusive">): void;

modify(readonlyView);
replace(readonlyView);
replace(mutableView);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const readonlyView: local &readonly User;
/// @type.symbol symbol=readonlyView source=readonlyView type=Placed<Borrowed<User, "static", "readonly">, "local">
/// @resolution.pattern source=readonlyView kind=binding target=readonlyView
/// @resolution.name source=User target=User

declare const mutableView: local &User;
/// @type.symbol symbol=mutableView source=mutableView type=Placed<Borrowed<User, "static", "mutable">, "local">
/// @resolution.pattern source=mutableView kind=binding target=mutableView
/// @resolution.name source=User target=User

declare function modify(value: local &User): void;
/// @generic.template symbol=modify parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=modify source="declare function modify(value: local &User): void" type=<comptime modify.L0: Lifetime>(Placed<Borrowed<User, modify.L0, "mutable">, "local">) => void
/// @type.symbol symbol=modify.value source="value: local &User" type=Placed<Borrowed<User, modify.L0, "mutable">, "local">
/// @resolution.name source=User target=User

declare function replace(value: local &exclusive User): void;
/// @generic.template symbol=replace parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=replace source="declare function replace(value: local &exclusive User): void" type=<comptime replace.L0: Lifetime>(Placed<Borrowed<User, replace.L0, "exclusive">, "local">) => void
/// @type.symbol symbol=replace.value source="value: local &exclusive User" type=Placed<Borrowed<User, replace.L0, "exclusive">, "local">
/// @resolution.name source=User target=User

modify(readonlyView);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(readonlyView) parameters=(Placed<Borrowed<User, "static", "mutable">, "local">) arguments=(provided(readonlyView) as Placed<Borrowed<User, "static", "mutable">, "local">) return=void kind=symbol target=modify
/// @resolution.name source=readonlyView target=readonlyView

replace(readonlyView);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(readonlyView) parameters=(Placed<Borrowed<User, "static", "exclusive">, "local">) arguments=(provided(readonlyView) as Placed<Borrowed<User, "static", "exclusive">, "local">) return=void kind=symbol target=replace
/// @resolution.name source=readonlyView target=readonlyView

replace(mutableView);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(mutableView) parameters=(Placed<Borrowed<User, "static", "exclusive">, "local">) arguments=(provided(mutableView) as Placed<Borrowed<User, "static", "exclusive">, "local">) return=void kind=symbol target=replace
/// @resolution.name source=mutableView target=mutableView
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'local &readonly User' is not assignable to parameter of type 'local &User'"
/// @diagnostic.label line=9 column=8 span="readonlyView" line_source="modify(readonlyView);"
/// @diagnostic.related line=9 column=1 span="modify(readonlyView)" line_source="modify(readonlyView);" message="in this call"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'local &readonly User' is not assignable to parameter of type 'local &exclusive User'"
/// @diagnostic.label line=10 column=9 span="readonlyView" line_source="replace(readonlyView);"
/// @diagnostic.related line=10 column=1 span="replace(readonlyView)" line_source="replace(readonlyView);" message="in this call"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'local &User' is not assignable to parameter of type 'local &exclusive User'"
/// @diagnostic.label line=11 column=9 span="mutableView" line_source="replace(mutableView);"
/// @diagnostic.related line=11 column=1 span="replace(mutableView)" line_source="replace(mutableView);" message="in this call"
"#,
    );
}

#[test]
fn test_expand_borrow_parameter_shorthands_to_borrowed_forms() {
    let session = TestSession::single(
        r#"
struct Node {
    id: int32;
}

function access(read: &readonly Node, write: &Node, exclusive: &exclusive Node): void {
    read.id;
    write.id;
    exclusive.id;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

function access<comptime L0: Lifetime, comptime L1: Lifetime, comptime L2: Lifetime>(
    read: Borrowed<Node, L0, "readonly">,
    write: Borrowed<Node, L1, "mutable">,
    exclusive: Borrowed<Node, L2, "exclusive">,
): void {
    read.id;
    write.id;
    exclusive.id;
}

=== checked ===
struct Node {
/// @type.symbol symbol=Node type=Node
/// @definition.struct symbol=Node
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Node.id source="id: int32" type=int32

}

function access(read: &readonly Node, write: &Node, exclusive: &exclusive Node): void {
/// @generic.template symbol=access parameters=(comptime L0: Lifetime, comptime L1: Lifetime, comptime L2: Lifetime)
/// @type.symbol symbol=access type=<comptime access.L0: Lifetime, comptime access.L1: Lifetime, comptime access.L2: Lifetime>(Borrowed<Node, access.L0, "readonly">, Borrowed<Node, access.L1, "mutable">, Borrowed<Node, access.L2, "exclusive">) => void
/// @type.symbol symbol=access.read source="read: &readonly Node" type=Borrowed<Node, access.L0, "readonly">
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=access.write source="write: &Node" type=Borrowed<Node, access.L1, "mutable">
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=access.exclusive source="exclusive: &exclusive Node" type=Borrowed<Node, access.L2, "exclusive">
/// @resolution.name source=Node target=Node

    read.id;
    /// @resolution.name source=read target=access.read
    /// @resolution.member source=read.id receiver=Borrowed<Node, access.L0, "readonly"> kind=symbol target=Node.id

    write.id;
    /// @resolution.name source=write target=access.write
    /// @resolution.member source=write.id receiver=Borrowed<Node, access.L1, "mutable"> kind=symbol target=Node.id

    exclusive.id;
    /// @resolution.name source=exclusive target=access.exclusive
    /// @resolution.member source=exclusive.id receiver=Borrowed<Node, access.L2, "exclusive"> kind=symbol target=Node.id

}
"#,
    );
}

#[test]
fn test_preserve_field_path_in_borrow_expression() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

let point: ^Point = Point { x: 1 };
let x = &readonly point.x;

x satisfies &readonly int32;
point.x satisfies local int32;
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
let x: Borrowed<int32, "static", "readonly"> = &readonly point.x;

x satisfies &readonly int32;
point.x satisfies local int32;

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

let x = &readonly point.x;
/// @type.symbol symbol=x source=x type=Borrowed<int32, "static", "readonly">
/// @resolution.pattern source=x kind=binding target=x
/// @type.node source="&readonly point.x" type=Borrowed<int32, "static", "readonly">
/// @type.node source=point type=Owned<Point> reduced=Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point kind=symbol target=Point.x

x satisfies &readonly int32;
/// @type.node source="x satisfies &readonly int32" type=Borrowed<int32, "static", "readonly">
/// @type.node source=x type=Borrowed<int32, "static", "readonly">
/// @resolution.name source=x target=x

point.x satisfies local int32;
/// @type.node source="point.x satisfies local int32" type=int32
/// @type.node source=point type=Owned<Point> reduced=Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point kind=symbol target=Point.x
"#,
    );
}

#[test]
fn test_preserve_fixed_array_length_under_borrow() {
    let session = TestSession::single(
        r#"
let values: [int32; 3] = [1, 2, 3];
let borrow = &readonly values;

borrow satisfies &readonly [int32; 3];
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: [int32; 3] = [1, 2, 3];
let borrow: Borrowed<[int32; 3], "static", "readonly"> = &readonly values;

borrow satisfies &readonly [int32; 3];

=== checked ===
let values: [int32; 3] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 3>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2, 3] type=FixedArray<int32, 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

let borrow = &readonly values;
/// @type.symbol symbol=borrow source=borrow type=Borrowed<FixedArray<int32, 3>, "static", "readonly">
/// @resolution.pattern source=borrow kind=binding target=borrow
/// @type.node source="&readonly values" type=Borrowed<FixedArray<int32, 3>, "static", "readonly">
/// @type.node source=values type=FixedArray<int32, 3>
/// @resolution.name source=values target=values

borrow satisfies &readonly [int32; 3];
/// @type.node source="borrow satisfies &readonly [int32; 3]" type=Borrowed<FixedArray<int32, 3>, "static", "readonly">
/// @type.node source=borrow type=Borrowed<FixedArray<int32, 3>, "static", "readonly">
/// @resolution.name source=borrow target=borrow
"#,
    );
}
