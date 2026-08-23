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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_node_types().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const localUser: local User;
declare const sharedUser: shared User;

const localView: local &'static readonly User = &readonly localUser;
const sharedView: shared &'static readonly User = &readonly sharedUser;

localView satisfies local &readonly User;
sharedView satisfies shared &readonly User;

=== dir ===
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
/// @type.symbol symbol=localView source=localView type=Placed<&'static readonly User, "local">
/// @resolution.pattern source=localView kind=binding target=localView
/// @type.node source="&readonly localUser" type=Placed<&'static readonly User, "local">
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=localUser root=localUser

const sharedView = &readonly sharedUser;
/// @type.symbol symbol=sharedView source=sharedView type=Placed<&'static readonly User, "shared">
/// @resolution.pattern source=sharedView kind=binding target=sharedView
/// @type.node source="&readonly sharedUser" type=Placed<&'static readonly User, "shared">
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedUser root=sharedUser

localView satisfies local &readonly User;
/// @type.node source="localView satisfies local &readonly User" type=Placed<&'static readonly User, "local">
/// @resolution.name source=localView target=localView
/// @resolution.place source=localView placement="local" lifetime="static" access="readonly"
/// @resolution.access source=localView root=localView
/// @resolution.name source=User target=User

sharedView satisfies shared &readonly User;
/// @type.node source="sharedView satisfies shared &readonly User" type=Placed<&'static readonly User, "shared">
/// @resolution.name source=sharedView target=sharedView
/// @resolution.place source=sharedView placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=sharedView root=sharedView
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_node_types().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const user: local User;

const projected: local &'static readonly User = &readonly user;
const coerced: local &'static readonly User = user as local &readonly User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const user: local User;
/// @type.symbol symbol=user source=user type=Placed<User, "local">
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const projected = &readonly user;
/// @type.symbol symbol=projected source=projected type=Placed<&'static readonly User, "local">
/// @resolution.pattern source=projected kind=binding target=projected
/// @type.node source="&readonly user" type=Placed<&'static readonly User, "local">
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user

const coerced = user as local &readonly User;
/// @type.symbol symbol=coerced source=coerced type=Placed<&'static readonly User, "local">
/// @resolution.pattern source=coerced kind=binding target=coerced
/// @type.node source="user as local &readonly User" type=Placed<&'static readonly User, "local">
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const sharedUser: shared User;
declare const readonlyUser: local readonly User;

const sharedExclusive: shared &'static exclusive User = &exclusive sharedUser;
const readonlyMutable: local &'static readonly User = &readonlyUser;
const readonlyExclusive: local &'static exclusive readonly User = &exclusive readonlyUser;

=== dir ===
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
/// @type.symbol symbol=sharedExclusive source=sharedExclusive type=Placed<&'static exclusive User, "shared">
/// @resolution.pattern source=sharedExclusive kind=binding target=sharedExclusive
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedUser root=sharedUser

const readonlyMutable = &readonlyUser;
/// @type.symbol symbol=readonlyMutable source=readonlyMutable type=Placed<&'static Readonly<User>, "local">
/// @resolution.pattern source=readonlyMutable kind=binding target=readonlyMutable
/// @resolution.name source=readonlyUser target=readonlyUser
/// @resolution.place source=readonlyUser placement="local" lifetime="static" access="readonly"
/// @resolution.access source=readonlyUser root=readonlyUser

const readonlyExclusive = &exclusive readonlyUser;
/// @type.symbol symbol=readonlyExclusive source=readonlyExclusive type=Placed<&'static exclusive Readonly<User>, "local">
/// @resolution.pattern source=readonlyExclusive kind=binding target=readonlyExclusive
/// @resolution.name source=readonlyUser target=readonlyUser
/// @resolution.place source=readonlyUser placement="local" lifetime="static" access="readonly"
/// @resolution.access source=readonlyUser root=readonlyUser
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
fn test_restrict_explicit_borrows_of_immutable_direct_storage() {
    let session = TestSession::single(
        r#"
declare const value: int32;

const readonlyValue = &readonly value;
const mutableValue = &value;
const exclusiveValue = &exclusive value;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const value: int32;

const readonlyValue: &'static readonly int32 = &readonly value;
const mutableValue: &'static int32 = &value;
const exclusiveValue: &'static exclusive int32 = &exclusive value;

=== dir ===
declare const value: int32;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value

const readonlyValue = &readonly value;
/// @type.symbol symbol=readonlyValue source=readonlyValue type=&'static readonly int32
/// @resolution.pattern source=readonlyValue kind=binding target=readonlyValue
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="readonly"
/// @resolution.access source=value root=value

const mutableValue = &value;
/// @type.symbol symbol=mutableValue source=mutableValue type=&'static int32
/// @resolution.pattern source=mutableValue kind=binding target=mutableValue
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="readonly"
/// @resolution.access source=value root=value

const exclusiveValue = &exclusive value;
/// @type.symbol symbol=exclusiveValue source=exclusiveValue type=&'static exclusive int32
/// @resolution.pattern source=exclusiveValue kind=binding target=exclusiveValue
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="readonly"
/// @resolution.access source=value root=value
"#,
        r#"
/// @diagnostic.error id=borrow-access-not-granted message="'mutable' access is not granted by a value of type 'int32'"
/// @diagnostic.label line=5 column=22 span="&" line_source="const mutableValue = &value;"
/// @diagnostic.note message="the source grants at most 'readonly' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
/// @diagnostic.error id=borrow-access-not-granted message="'exclusive' access is not granted by a value of type 'int32'"
/// @diagnostic.label line=6 column=24 span="&" line_source="const exclusiveValue = &exclusive value;"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_node_types().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const user: local User;
declare function inspect<'a>(value: local &'a readonly User): void;
declare function modify<'a>(value: local &'a User): void;

inspect(&user);
modify(&user);
inspect(&exclusive user);
modify(&exclusive user);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const user: local User;
/// @type.symbol symbol=user source=user type=Placed<User, "local">
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

declare function inspect(value: local &readonly User): void;
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect source="declare function inspect(value: local &readonly User): void" type=<inspect.'a>(Placed<&inspect.'a readonly User, "local">) => void
/// @type.symbol symbol=inspect.value source="value: local &readonly User" type=Placed<&inspect.'a readonly User, "local">
/// @resolution.name source=User target=User

declare function modify(value: local &User): void;
/// @generic.template symbol=modify parameters=('a)
/// @type.symbol symbol=modify source="declare function modify(value: local &User): void" type=<modify.'a>(Placed<&modify.'a User, "local">) => void
/// @type.symbol symbol=modify.value source="value: local &User" type=Placed<&modify.'a User, "local">
/// @resolution.name source=User target=User

inspect(&user);
/// @type.node source=inspect(&user) type=void
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(&user) parameters=(Placed<&'static readonly User, "local">) arguments=(provided(&user) as Placed<&'static readonly User, "local">) return=void kind=symbol target=inspect
/// @type.node source=&user type=Placed<&'static User, "local">
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user

modify(&user);
/// @type.node source=modify(&user) type=void
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(&user) parameters=(Placed<&'static User, "local">) arguments=(provided(&user) as Placed<&'static User, "local">) return=void kind=symbol target=modify
/// @type.node source=&user type=Placed<&'static User, "local">
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user

inspect(&exclusive user);
/// @type.node source="inspect(&exclusive user)" type=void
/// @resolution.name source=inspect target=inspect
/// @resolution.call source="inspect(&exclusive user)" parameters=(Placed<&'static readonly User, "local">) arguments=(provided(&exclusive user) as Placed<&'static readonly User, "local">) return=void kind=symbol target=inspect
/// @type.node source="&exclusive user" type=Placed<&'static exclusive User, "local">
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user

modify(&exclusive user);
/// @type.node source="modify(&exclusive user)" type=void
/// @resolution.name source=modify target=modify
/// @resolution.call source="modify(&exclusive user)" parameters=(Placed<&'static User, "local">) arguments=(provided(&exclusive user) as Placed<&'static User, "local">) return=void kind=symbol target=modify
/// @type.node source="&exclusive user" type=Placed<&'static exclusive User, "local">
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const readonlyView: local &'static readonly User;
declare const mutableView: local &'static User;
declare function modify<'a>(value: local &'a User): void;
declare function replace<'a>(value: local &'a exclusive User): void;

modify(readonlyView);
replace(readonlyView);
replace(mutableView);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const readonlyView: local &readonly User;
/// @type.symbol symbol=readonlyView source=readonlyView type=Placed<&'static readonly User, "local">
/// @resolution.pattern source=readonlyView kind=binding target=readonlyView
/// @resolution.name source=User target=User

declare const mutableView: local &User;
/// @type.symbol symbol=mutableView source=mutableView type=Placed<&'static User, "local">
/// @resolution.pattern source=mutableView kind=binding target=mutableView
/// @resolution.name source=User target=User

declare function modify(value: local &User): void;
/// @generic.template symbol=modify parameters=('a)
/// @type.symbol symbol=modify source="declare function modify(value: local &User): void" type=<modify.'a>(Placed<&modify.'a User, "local">) => void
/// @type.symbol symbol=modify.value source="value: local &User" type=Placed<&modify.'a User, "local">
/// @resolution.name source=User target=User

declare function replace(value: local &exclusive User): void;
/// @generic.template symbol=replace parameters=('a)
/// @type.symbol symbol=replace source="declare function replace(value: local &exclusive User): void" type=<replace.'a>(Placed<&replace.'a exclusive User, "local">) => void
/// @type.symbol symbol=replace.value source="value: local &exclusive User" type=Placed<&replace.'a exclusive User, "local">
/// @resolution.name source=User target=User

modify(readonlyView);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(readonlyView) parameters=(Placed<&'static User, "local">) arguments=(provided(readonlyView) as Placed<&'static User, "local">) return=void kind=symbol target=modify
/// @resolution.name source=readonlyView target=readonlyView
/// @resolution.place source=readonlyView placement="local" lifetime="static" access="readonly"
/// @resolution.access source=readonlyView root=readonlyView

replace(readonlyView);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(readonlyView) parameters=(Placed<&'static exclusive User, "local">) arguments=(provided(readonlyView) as Placed<&'static exclusive User, "local">) return=void kind=symbol target=replace
/// @resolution.name source=readonlyView target=readonlyView
/// @resolution.place source=readonlyView placement="local" lifetime="static" access="readonly"
/// @resolution.access source=readonlyView root=readonlyView

replace(mutableView);
/// @resolution.name source=replace target=replace
/// @resolution.call source=replace(mutableView) parameters=(Placed<&'static exclusive User, "local">) arguments=(provided(mutableView) as Placed<&'static exclusive User, "local">) return=void kind=symbol target=replace
/// @resolution.name source=mutableView target=mutableView
/// @resolution.place source=mutableView placement="local" lifetime="static" access="mutable"
/// @resolution.access source=mutableView root=mutableView
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'local &'static readonly User' is not assignable to parameter of type 'local &'static User'"
/// @diagnostic.label line=9 column=8 span="readonlyView" line_source="modify(readonlyView);"
/// @diagnostic.related line=9 column=1 span="modify(readonlyView)" line_source="modify(readonlyView);" message="in this call"
/// @diagnostic.note message="expected '&'static User', found '&'static readonly User'"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'local &'static readonly User' is not assignable to parameter of type 'local &'static exclusive User'"
/// @diagnostic.label line=10 column=9 span="readonlyView" line_source="replace(readonlyView);"
/// @diagnostic.related line=10 column=1 span="replace(readonlyView)" line_source="replace(readonlyView);" message="in this call"
/// @diagnostic.note message="expected '&'static exclusive User', found '&'static readonly User'"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'local &'static User' is not assignable to parameter of type 'local &'static exclusive User'"
/// @diagnostic.label line=11 column=9 span="mutableView" line_source="replace(mutableView);"
/// @diagnostic.related line=11 column=1 span="replace(mutableView)" line_source="replace(mutableView);" message="in this call"
/// @diagnostic.note message="expected '&'static exclusive User', found '&'static User'"
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

function access<'a, 'b, 'c>(
    read: &'a readonly Node,
    write: &'b Node,
    exclusive: &'c exclusive Node,
): void {
    read.id;
    write.id;
    exclusive.id;
}

=== dir ===
struct Node {
/// @type.symbol symbol=Node type=Node
/// @definition.struct symbol=Node
/// @definition.field symbol=Node.id source="id: int32" key=id type=int32

    id: int32;
    /// @type.symbol symbol=Node.id source="id: int32" type=int32

}

function access(read: &readonly Node, write: &Node, exclusive: &exclusive Node): void {
/// @generic.template symbol=access parameters=('a, 'b, 'c)
/// @type.symbol symbol=access type=<access.'a, access.'b, access.'c>(&access.'a readonly Node, &access.'b Node, &access.'c exclusive Node) => void
/// @type.symbol symbol=access.read source="read: &readonly Node" type=&access.'a readonly Node
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=access.write source="write: &Node" type=&access.'b Node
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=access.exclusive source="exclusive: &exclusive Node" type=&access.'c exclusive Node
/// @resolution.name source=Node target=Node

    read.id;
    /// @resolution.name source=read target=access.read
    /// @resolution.member source=read.id receiver=&access.'a readonly Node type=int32 kind=field target_receiver=&access.'a readonly Node key=id target=Node.id target_type=int32
    /// @resolution.place source=read placement="local" lifetime=access.'a access="readonly"
    /// @resolution.access source=read root=access.read
    /// @resolution.place source=read.id placement="local" lifetime=access.'a access="readonly"
    /// @resolution.access source=read.id root=access.read keys=[id]

    write.id;
    /// @resolution.name source=write target=access.write
    /// @resolution.member source=write.id receiver=&access.'b Node type=int32 kind=field target_receiver=&access.'b Node key=id target=Node.id target_type=int32
    /// @resolution.place source=write placement="local" lifetime=access.'b access="mutable"
    /// @resolution.access source=write root=access.write
    /// @resolution.place source=write.id placement="local" lifetime=access.'b access="mutable"
    /// @resolution.access source=write.id root=access.write keys=[id]

    exclusive.id;
    /// @resolution.name source=exclusive target=access.exclusive
    /// @resolution.member source=exclusive.id receiver=&access.'c exclusive Node type=int32 kind=field target_receiver=&access.'c exclusive Node key=id target=Node.id target_type=int32
    /// @resolution.place source=exclusive placement="local" lifetime=access.'c access="exclusive"
    /// @resolution.access source=exclusive root=access.exclusive
    /// @resolution.place source=exclusive.id placement="local" lifetime=access.'c access="exclusive"
    /// @resolution.access source=exclusive.id root=access.exclusive keys=[id]

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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

let point: Point = Point { x: 1 };
let x: &'static readonly int32 = &readonly point.x;

x satisfies &readonly int32;
point.x satisfies local int32;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

}

let point: ^Point = Point { x: 1 };
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point
/// @type.node source="Point { x: 1 }" type=Point
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

let x = &readonly point.x;
/// @type.symbol symbol=x source=x type=&'static readonly int32
/// @resolution.pattern source=x kind=binding target=x
/// @type.node source="&readonly point.x" type=&'static readonly int32
/// @type.node source=point type=Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point type=int32 kind=field target_receiver=Point key=x target=Point.x target_type=int32
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point
/// @resolution.place source=point.x placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point.x root=point keys=[x]

x satisfies &readonly int32;
/// @type.node source="x satisfies &readonly int32" type=&'static readonly int32
/// @type.node source=x type=&'static readonly int32
/// @resolution.name source=x target=x
/// @resolution.place source=x placement="local" lifetime="static" access="readonly"
/// @resolution.access source=x root=x

point.x satisfies local int32;
/// @type.node source="point.x satisfies local int32" type=int32
/// @type.node source=point type=Point
/// @type.node source=point.x type=int32
/// @resolution.name source=point target=point
/// @resolution.member source=point.x receiver=Point type=int32 kind=field target_receiver=Point key=x target=Point.x target_type=int32
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point
/// @resolution.place source=point.x placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point.x root=point keys=[x]
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: [int32; 3] = [1, 2, 3];
let borrow: &'static readonly [int32; 3] = &readonly values;

borrow satisfies &readonly [int32; 3];

=== dir ===
let values: [int32; 3] = [1, 2, 3];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 3>
/// @resolution.pattern source=values kind=binding target=values
/// @type.node source=[1, 2, 3] type=FixedArray<int32, 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3

let borrow = &readonly values;
/// @type.symbol symbol=borrow source=borrow type=&'static readonly FixedArray<int32, 3>
/// @resolution.pattern source=borrow kind=binding target=borrow
/// @type.node source="&readonly values" type=&'static readonly FixedArray<int32, 3>
/// @type.node source=values type=FixedArray<int32, 3>
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values

borrow satisfies &readonly [int32; 3];
/// @type.node source="borrow satisfies &readonly [int32; 3]" type=&'static readonly FixedArray<int32, 3>
/// @type.node source=borrow type=&'static readonly FixedArray<int32, 3>
/// @resolution.name source=borrow target=borrow
/// @resolution.place source=borrow placement="local" lifetime="static" access="readonly"
/// @resolution.access source=borrow root=borrow
"#,
    );
}
