use crate::tests::{DirRows, TestSession};

#[test]
fn test_preserve_source_place_in_explicit_borrow() {
    let session = TestSession::single(
        r#"
class User {}

shared class SharedUser {}

declare const localUser: User;
declare const sharedUser: SharedUser;

const localView = &readonly localUser;
const sharedView = &readonly sharedUser;

localView satisfies &readonly User;
sharedView satisfies &readonly SharedUser;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_node_types().with_coercion(),
        r#"
=== annotated ===
class User {}

shared class SharedUser {}

declare const localUser: User;
declare const sharedUser: SharedUser;

const localView: &'managed readonly User = &readonly localUser;
const sharedView: &'managed readonly SharedUser = &readonly sharedUser;

localView satisfies &readonly User;
sharedView satisfies &readonly SharedUser;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

shared class SharedUser {}
/// @type.symbol symbol=SharedUser source="shared class SharedUser {}" type=typeof SharedUser
/// @definition.class symbol=SharedUser source="shared class SharedUser {}"

declare const localUser: User;
/// @type.symbol symbol=localUser source=localUser type=User
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: SharedUser;
/// @type.symbol symbol=sharedUser source=sharedUser type=SharedUser
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=SharedUser target=SharedUser

const localView = &readonly localUser;
/// @type.symbol symbol=localView source=localView type=&'managed readonly User
/// @resolution.pattern source=localView kind=binding target=localView
/// @type.node source="&readonly localUser" type=&'managed readonly User
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="static" access="immutable"
/// @resolution.access source=localUser root=localUser

const sharedView = &readonly sharedUser;
/// @type.symbol symbol=sharedView source=sharedView type=&'managed readonly SharedUser
/// @resolution.pattern source=sharedView kind=binding target=sharedView
/// @type.node source="&readonly sharedUser" type=&'managed readonly SharedUser
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=sharedUser root=sharedUser

localView satisfies &readonly User;
/// @type.node source="localView satisfies &readonly User" type=&'managed readonly User
/// @resolution.name source=localView target=localView
/// @resolution.place source=localView placement="local" lifetime="managed" access="immutable"
/// @resolution.access source=localView root=localView
/// @resolution.name source=User target=User

sharedView satisfies &readonly SharedUser;
/// @type.node source="sharedView satisfies &readonly SharedUser" type=&'managed readonly SharedUser
/// @resolution.name source=sharedView target=sharedView
/// @resolution.place source=sharedView placement="shared" lifetime="managed" access="immutable"
/// @resolution.access source=sharedView root=sharedView
/// @resolution.name source=SharedUser target=SharedUser
"#,
    );
}

#[test]
fn test_distinguish_borrow_expression_from_explicit_borrow_coercion() {
    let session = TestSession::single(
        r#"
class User {}

declare const user: User;

const projected = &readonly user;
const coerced = user as &readonly User;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_node_types().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const user: User;

const projected: &'managed readonly User = &readonly user;
const coerced: &'managed readonly User = user as &readonly User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const projected = &readonly user;
/// @type.symbol symbol=projected source=projected type=&'managed readonly User
/// @resolution.pattern source=projected kind=binding target=projected
/// @type.node source="&readonly user" type=&'managed readonly User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user

const coerced = user as &readonly User;
/// @type.symbol symbol=coerced source=coerced type=&'managed readonly User
/// @resolution.pattern source=coerced kind=binding target=coerced
/// @type.node source="user as &readonly User" type=&'managed readonly User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
/// @coercion.node source=user from=User adjustments=[{ kind: borrow, target: &'managed readonly User }] origin=explicit
/// @resolution.name source=User target=User
"#,
    );
}

#[test]
fn test_reject_explicit_borrow_access_not_granted_by_managed_source() {
    let session = TestSession::single(
        r#"
class User {}

shared class SharedUser {}

declare const sharedUser: SharedUser;
declare const readonlyUser: readonly User;

const sharedExclusive = &sharedUser;
const readonlyMutable = &readonlyUser;
const readonlyExclusive = &readonlyUser;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared class SharedUser {}

declare const sharedUser: SharedUser;
declare const readonlyUser: readonly User;

const sharedExclusive: &'managed SharedUser = &sharedUser;
const readonlyMutable: &'managed readonly User = &readonlyUser;
const readonlyExclusive: &'managed readonly User = &readonlyUser;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

shared class SharedUser {}
/// @type.symbol symbol=SharedUser source="shared class SharedUser {}" type=typeof SharedUser
/// @definition.class symbol=SharedUser source="shared class SharedUser {}"

declare const sharedUser: SharedUser;
/// @type.symbol symbol=sharedUser source=sharedUser type=SharedUser
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=SharedUser target=SharedUser

declare const readonlyUser: readonly User;
/// @type.symbol symbol=readonlyUser source=readonlyUser type=readonly User
/// @resolution.pattern source=readonlyUser kind=binding target=readonlyUser
/// @resolution.name source=User target=User

const sharedExclusive = &sharedUser;
/// @type.symbol symbol=sharedExclusive source=sharedExclusive type=&'managed SharedUser
/// @resolution.pattern source=sharedExclusive kind=binding target=sharedExclusive
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=sharedUser root=sharedUser

const readonlyMutable = &readonlyUser;
/// @type.symbol symbol=readonlyMutable source=readonlyMutable type=&'managed readonly User
/// @resolution.pattern source=readonlyMutable kind=binding target=readonlyMutable
/// @resolution.name source=readonlyUser target=readonlyUser
/// @resolution.place source=readonlyUser placement="local" lifetime="static" access="immutable"
/// @resolution.access source=readonlyUser root=readonlyUser

const readonlyExclusive = &readonlyUser;
/// @type.symbol symbol=readonlyExclusive source=readonlyExclusive type=&'managed readonly User
/// @resolution.pattern source=readonlyExclusive kind=binding target=readonlyExclusive
/// @resolution.name source=readonlyUser target=readonlyUser
/// @resolution.place source=readonlyUser placement="local" lifetime="static" access="immutable"
/// @resolution.access source=readonlyUser root=readonlyUser
"#,
        r#"
/// @diagnostic.error id=borrow-access-not-granted message="'mutable' access is not granted by a value of type 'readonly User'"
/// @diagnostic.label line=10 column=25 span="&" line_source="const readonlyMutable = &readonlyUser;"
/// @diagnostic.note message="the source grants at most 'readonly' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
/// @diagnostic.error id=borrow-access-not-granted message="'mutable' access is not granted by a value of type 'readonly User'"
/// @diagnostic.label line=11 column=27 span="&" line_source="const readonlyExclusive = &readonlyUser;"
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
const exclusiveValue = &value;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare const value: int32;

const readonlyValue: &'static readonly int32 = &readonly value;
const mutableValue: &'static int32 = &value;
const exclusiveValue: &'static int32 = &value;

=== dir ===
declare const value: int32;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value

const readonlyValue = &readonly value;
/// @type.symbol symbol=readonlyValue source=readonlyValue type=&'static readonly int32
/// @resolution.pattern source=readonlyValue kind=binding target=readonlyValue
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

const mutableValue = &value;
/// @type.symbol symbol=mutableValue source=mutableValue type=&'static int32
/// @resolution.pattern source=mutableValue kind=binding target=mutableValue
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value

const exclusiveValue = &value;
/// @type.symbol symbol=exclusiveValue source=exclusiveValue type=&'static int32
/// @resolution.pattern source=exclusiveValue kind=binding target=exclusiveValue
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#,
        r#"
/// @diagnostic.error id=borrow-access-not-granted message="'mutable' access is not granted by a value of type 'int32'"
/// @diagnostic.label line=5 column=22 span="&" line_source="const mutableValue = &value;"
/// @diagnostic.note message="the source grants at most 'immutable' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
/// @diagnostic.error id=borrow-access-not-granted message="'mutable' access is not granted by a value of type 'int32'"
/// @diagnostic.label line=6 column=24 span="&" line_source="const exclusiveValue = &value;"
/// @diagnostic.note message="the source grants at most 'immutable' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
"#,
    );
}

#[test]
fn test_assign_stronger_borrow_access_to_weaker_access() {
    let session = TestSession::single(
        r#"
class User {}

declare const user: User;
declare function inspect(value: &readonly User): void;
declare function modify(value: &User): void;

inspect(&user);
modify(&user);
inspect(&user);
modify(&user);
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_node_types().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const user: User;
declare function inspect<'a>(value: &readonly User): void;
declare function modify<'a>(value: &User): void;

inspect<"managed">(&user);
modify<"managed">(&user);
inspect<"managed">(&user);
modify<"managed">(&user);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

declare function inspect(value: &readonly User): void;
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect source="declare function inspect(value: &readonly User): void" type=<inspect.'a>(&inspect.'a readonly User) => void
/// @resolution.name source=User target=User

declare function modify(value: &User): void;
/// @generic.template symbol=modify parameters=('a)
/// @type.symbol symbol=modify source="declare function modify(value: &User): void" type=<modify.'a>(&modify.'a User) => void
/// @resolution.name source=User target=User

inspect(&user);
/// @type.node source=inspect(&user) type=void
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(&user) parameters=(&'managed readonly User) arguments=(provided(&user) as &'managed readonly User) return=void regions=("managed" & "local") kind=symbol target=inspect instance="inspect<\"managed\" & \"local\">"
/// @generic.instantiation id="inspect<\"managed\" & \"local\">" template=inspect arguments=("managed" & "local")
/// @generic.instance id="inspect<\"bound0\" & \"local\">" template=inspect arguments=("bound0" & "local")
/// @type.node source=&user type=&'managed User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user

modify(&user);
/// @type.node source=modify(&user) type=void
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(&user) parameters=(&'managed User) arguments=(provided(&user) as &'managed User) return=void regions=("managed" & "local") kind=symbol target=modify instance="modify<\"managed\" & \"local\">"
/// @generic.instantiation id="modify<\"managed\" & \"local\">" template=modify arguments=("managed" & "local")
/// @generic.instance id="modify<\"bound0\" & \"local\">" template=modify arguments=("bound0" & "local")
/// @type.node source=&user type=&'managed User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user

inspect(&user);
/// @type.node source=inspect(&user) type=void
/// @resolution.name source=inspect target=inspect
/// @resolution.call source=inspect(&user) parameters=(&'managed readonly User) arguments=(provided(&user) as &'managed readonly User) return=void regions=("managed" & "local") kind=symbol target=inspect instance="inspect<\"managed\" & \"local\">"
/// @type.node source=&user type=&'managed User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user

modify(&user);
/// @type.node source=modify(&user) type=void
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(&user) parameters=(&'managed User) arguments=(provided(&user) as &'managed User) return=void regions=("managed" & "local") kind=symbol target=modify instance="modify<\"managed\" & \"local\">"
/// @type.node source=&user type=&'managed User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
"#,
    );
}

#[test]
fn test_reject_a_readonly_borrow_where_a_mutable_one_is_required() {
    let session = TestSession::single(
        r#"
class User {}

declare const readonlyView: &readonly User;
declare function modify(value: &User): void;

modify(readonlyView);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare const readonlyView: &'static readonly User;
declare function modify<'a>(value: &User): void;

modify<"static">(readonlyView);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare const readonlyView: &readonly User;
/// @type.symbol symbol=readonlyView source=readonlyView type=&'static readonly User
/// @resolution.pattern source=readonlyView kind=binding target=readonlyView
/// @resolution.name source=User target=User

declare function modify(value: &User): void;
/// @generic.template symbol=modify parameters=('a)
/// @type.symbol symbol=modify source="declare function modify(value: &User): void" type=<modify.'a>(&modify.'a User) => void
/// @resolution.name source=User target=User

modify(readonlyView);
/// @resolution.name source=modify target=modify
/// @resolution.call source=modify(readonlyView) parameters=(&'static User) arguments=(provided(readonlyView) as &'static User) return=void regions=("static" & "local") kind=symbol target=modify instance="modify<\"static\" & \"local\">"
/// @generic.instantiation id="modify<\"static\" & \"local\">" template=modify arguments=("static" & "local")
/// @resolution.name source=readonlyView target=readonlyView
/// @resolution.place source=readonlyView placement="local" lifetime="static" access="immutable"
/// @resolution.access source=readonlyView root=readonlyView
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '&'static readonly User' is not assignable to parameter of type '&'static User'"
/// @diagnostic.label line=7 column=8 span="readonlyView" line_source="modify(readonlyView);"
/// @diagnostic.related line=7 column=1 span="modify(readonlyView)" line_source="modify(readonlyView);" message="in this call"
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

function access(read: &readonly Node, write: &Node, exclusive: &Node): void {
    read.id;
    write.id;
    exclusive.id;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Node {
    id: int32;
}

function access<'a, 'b, 'c>(read: &'a readonly Node, write: &'b Node, exclusive: &'c Node): void {
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

function access(read: &readonly Node, write: &Node, exclusive: &Node): void {
/// @generic.template symbol=access parameters=('a, 'b, 'c)
/// @type.symbol symbol=access type=<access.'a, access.'b, access.'c>(&access.'a readonly Node, &access.'b Node, &access.'c Node) => void
/// @type.symbol symbol=access.read source="read: &readonly Node" type=&access.'a readonly Node
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=access.write source="write: &Node" type=&access.'b Node
/// @resolution.name source=Node target=Node
/// @type.symbol symbol=access.exclusive source="exclusive: &Node" type=&access.'c Node
/// @resolution.name source=Node target=Node

    read.id;
    /// @resolution.name source=read target=access.read
    /// @resolution.member source=read.id receiver=&access.'a readonly Node type=int32 kind=field target_receiver=&access.'a readonly Node key=id target=Node.id target_type=int32
    /// @resolution.place source=read placement=access.'a lifetime=access.'a access="readonly"
    /// @resolution.access source=read root=access.read
    /// @resolution.place source=read.id placement=access.'a lifetime=access.'a access="readonly"
    /// @resolution.access source=read.id root=access.read keys=[id]

    write.id;
    /// @resolution.name source=write target=access.write
    /// @resolution.member source=write.id receiver=&access.'b Node type=int32 kind=field target_receiver=&access.'b Node key=id target=Node.id target_type=int32
    /// @resolution.place source=write placement=access.'b lifetime=access.'b access="mutable"
    /// @resolution.access source=write root=access.write
    /// @resolution.place source=write.id placement=access.'b lifetime=access.'b access="mutable"
    /// @resolution.access source=write.id root=access.write keys=[id]

    exclusive.id;
    /// @resolution.name source=exclusive target=access.exclusive
    /// @resolution.member source=exclusive.id receiver=&access.'c Node type=int32 kind=field target_receiver=&access.'c Node key=id target=Node.id target_type=int32
    /// @resolution.place source=exclusive placement=access.'c lifetime=access.'c access="mutable"
    /// @resolution.access source=exclusive root=access.exclusive
    /// @resolution.place source=exclusive.id placement=access.'c lifetime=access.'c access="mutable"
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

x satisfies Borrowed<int32, 'static, "readonly">;
point.x satisfies int32;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}

let point: Point = Point { x: 1 };
let x: &'static readonly int32 = &readonly point.x;

x satisfies Borrowed<int32, 'static, "readonly">;
point.x satisfies int32;

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

x satisfies Borrowed<int32, 'static, "readonly">;
/// @type.node source="x satisfies Borrowed<int32, 'static, \"readonly\">" type=&'static readonly int32
/// @type.node source=x type=&'static readonly int32
/// @resolution.name source=x target=x
/// @resolution.place source=x placement="local" lifetime="static" access="readonly"
/// @resolution.access source=x root=x
/// @resolution.name source=Borrowed target=Borrowed

point.x satisfies int32;
/// @type.node source="point.x satisfies int32" type=int32
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
function view(): void {
    let values: [int32; 3] = [1, 2, 3];
    let borrow = &readonly values;

    borrow satisfies &readonly [int32; 3];
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function view(): void {
    let values: [int32; 3] = [1, 2, 3];
    let borrow: &'frame readonly [int32; 3] = &readonly values;

    borrow satisfies &readonly [int32; 3];
}

=== dir ===
function view(): void {
/// @type.symbol symbol=view type=() => void

    let values: [int32; 3] = [1, 2, 3];
    /// @type.symbol symbol=view.values source=values type=FixedArray<int32, 3>
    /// @resolution.pattern source=values kind=binding target=view.values
    /// @type.node source=[1, 2, 3] type=FixedArray<int32, 3>
    /// @type.node source=1 type=1
    /// @type.node source=2 type=2
    /// @type.node source=3 type=3

    let borrow = &readonly values;
    /// @type.symbol symbol=view.borrow source=borrow type=&'frame readonly FixedArray<int32, 3>
    /// @resolution.pattern source=borrow kind=binding target=view.borrow
    /// @type.node source="&readonly values" type=&'frame readonly FixedArray<int32, 3>
    /// @type.node source=values type=FixedArray<int32, 3>
    /// @resolution.name source=values target=view.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=view.values

    borrow satisfies &readonly [int32; 3];
    /// @type.node source="borrow satisfies &readonly [int32; 3]" type=&'frame readonly FixedArray<int32, 3>
    /// @type.node source=borrow type=&'frame readonly FixedArray<int32, 3>
    /// @resolution.name source=borrow target=view.borrow
    /// @resolution.place source=borrow placement="local" lifetime="frame" access="readonly"
    /// @resolution.access source=borrow root=view.borrow

}
"#,
    );
}

/// A readonly view refuses a readonly borrow of a handle until an explicit dereference.
#[test]
fn test_refuse_a_borrowed_array_into_a_readonly_view_without_a_dereference() {
    let session = TestSession::single(
        r#"
function view(values: &readonly int32[]): readonly int32[] {
    return values;
}

function copied(values: &readonly int32[]): readonly int32[] {
    return *values;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function view<'a>(values: &'a readonly int32[]): readonly int32[] {
    return values;
}

function copied<'a>(values: &'a readonly int32[]): readonly int32[] {
    return *values as readonly int32[];
}

=== dir ===
function view(values: &readonly int32[]): readonly int32[] {
/// @generic.template symbol=view parameters=('a)
/// @type.symbol symbol=view type=<view.'a>(&view.'a readonly int32[]) => readonly int32[]
/// @type.symbol symbol=view.values source="values: &readonly int32[]" type=&view.'a readonly int32[]

    return values;
    /// @resolution.name source=values target=view.values
    /// @resolution.place source=values placement=view.'a lifetime=view.'a access="readonly"
    /// @resolution.access source=values root=view.values

}

function copied(values: &readonly int32[]): readonly int32[] {
/// @generic.template symbol=copied parameters=('a)
/// @type.symbol symbol=copied type=<copied.'a>(&copied.'a readonly int32[]) => readonly int32[]
/// @type.symbol symbol=copied.values source="values: &readonly int32[]" type=&copied.'a readonly int32[]

    return *values;
    /// @resolution.place source=*values placement=copied.'a lifetime=copied.'a access="readonly"
    /// @resolution.operator source=*values type=^int32[] operator="*" kind=builtin operands=[values as &copied.'a readonly int32[]]
    /// @resolution.name source=values target=copied.values
    /// @resolution.place source=values placement=copied.'a lifetime=copied.'a access="readonly"
    /// @resolution.access source=values root=copied.values

}
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type '&'a readonly int32[]' is not assignable to the declared result type 'readonly int32[]'"
/// @diagnostic.label line=3 column=12 span="values" line_source="return values;"
"#,
    );
}
