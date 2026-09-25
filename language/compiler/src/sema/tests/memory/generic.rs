use crate::tests::{DirRows, TestSession};

#[test]
fn test_default_free_managed_parameters_and_results_to_local() {
    let session = TestSession::single(
        r#"
class User {}

function identity(value: User): User {
    return value;
}

declare function create(): User;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

function identity(value: User): User {
    return value;
}

declare function create(): User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

function identity(value: User): User {
/// @type.symbol symbol=identity type=(User) => User
/// @type.symbol symbol=identity.value source="value: User" type=User
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

    return value;
    /// @resolution.name source=value target=identity.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=identity.value

}

declare function create(): User;
/// @type.symbol symbol=create source="declare function create(): User" type=() => User
/// @resolution.name source=User target=User
"#,
    );
}

#[test]
fn test_acquire_a_mutable_borrow_from_a_default_local_parameter() {
    let session = TestSession::single(
        r#"
class User {}

declare function consume(value: &User): void;

function replace(user: User): void {
    consume(user);
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare function consume<'a>(value: &User): void;

function replace(user: User): void {
    consume<"managed">(user as &'managed User);
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function consume(value: &User): void;
/// @generic.template symbol=consume parameters=('a)
/// @type.symbol symbol=consume source="declare function consume(value: &User): void" type=<consume.'a>(&consume.'a User) => void
/// @resolution.name source=User target=User

function replace(user: User): void {
/// @type.symbol symbol=replace type=(User) => void
/// @type.symbol symbol=replace.user source="user: User" type=User
/// @resolution.name source=User target=User

    consume(user);
    /// @resolution.name source=consume target=consume
    /// @resolution.call source=consume(user) parameters=(&'managed User) arguments=(provided(user) as &'managed User) return=void regions=("managed" & "local") kind=symbol target=consume instance="consume<\"managed\" & \"local\">"
    /// @generic.instantiation id="consume<\"managed\" & \"local\">" template=consume arguments=("managed" & "local")
    /// @generic.instance id="consume<\"bound0\" & \"local\">" template=consume arguments=("bound0" & "local")
    /// @resolution.name source=user target=replace.user
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=replace.user

}
"#,
    );
}

#[test]
fn test_preserve_placement_inside_ordinary_type_parameter() {
    let session = TestSession::single(
        r#"
class User {}

shared class SharedUser {}

function identity<T>(value: T): T {
    return value;
}

declare const localUser: User;
declare const sharedUser: SharedUser;

identity(localUser) satisfies User;
identity(sharedUser) satisfies SharedUser;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared class SharedUser {}

function identity<T>(value: T): T {
    return value;
}

declare const localUser: User;
declare const sharedUser: SharedUser;

identity<User>(localUser) satisfies User;
identity<SharedUser>(sharedUser) satisfies SharedUser;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

shared class SharedUser {}
/// @type.symbol symbol=SharedUser source="shared class SharedUser {}" type=typeof SharedUser
/// @definition.class symbol=SharedUser source="shared class SharedUser {}"

function identity<T>(value: T): T {
/// @generic.template symbol=identity parameters=(T)
/// @type.symbol symbol=identity type=<T>(T) => T
/// @type.symbol symbol=identity.T source=T type=T
/// @type.symbol symbol=identity.value source="value: T" type=T
/// @resolution.name source=T target=identity.T
/// @resolution.name source=T target=identity.T

    return value;
    /// @resolution.name source=value target=identity.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=identity.value

}

declare const localUser: User;
/// @type.symbol symbol=localUser source=localUser type=User
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: SharedUser;
/// @type.symbol symbol=sharedUser source=sharedUser type=SharedUser
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=SharedUser target=SharedUser

identity(localUser) satisfies User;
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(localUser) parameters=(User) arguments=(provided(localUser) as User) return=User kind=symbol target=identity instance=identity<User>
/// @generic.instantiation id=identity<User> template=identity arguments=(User)
/// @generic.instance id=identity<User> template=identity arguments=(User)
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="static" access="immutable"
/// @resolution.access source=localUser root=localUser
/// @resolution.name source=User target=User

identity(sharedUser) satisfies SharedUser;
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(sharedUser) parameters=(SharedUser) arguments=(provided(sharedUser) as SharedUser) return=SharedUser kind=symbol target=identity instance=identity<SharedUser>
/// @generic.instantiation id=identity<SharedUser> template=identity arguments=(SharedUser)
/// @generic.instance id=identity<SharedUser> template=identity arguments=(SharedUser)
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=sharedUser root=sharedUser
/// @resolution.name source=SharedUser target=SharedUser
"#,
    );
}

#[test]
fn test_omit_placement_from_immediate_callable_values() {
    let session = TestSession::single(
        r#"
function negate(value: boolean): boolean {
    return !value;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function negate(value: boolean): boolean {
    return !value;
}

=== dir ===
function negate(value: boolean): boolean {
/// @type.symbol symbol=negate type=(boolean) => boolean
/// @type.symbol symbol=negate.value source="value: boolean" type=boolean

    return !value;
    /// @resolution.operator source=!value type=boolean operator="!" kind=builtin operands=[value as boolean families=(boolean)]
    /// @resolution.name source=value target=negate.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=negate.value

}
"#,
    );
}

#[test]
fn test_borrow_immediate_parameter_without_placing_it() {
    let session = TestSession::single(
        r#"
function inspect(value: int32): void {
    const borrow = &readonly value;
    borrow satisfies &readonly int32;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function inspect(value: int32): void {
    const borrow: &'frame readonly int32 = &readonly value;
    borrow satisfies &readonly int32;
}

=== dir ===
function inspect(value: int32): void {
/// @type.symbol symbol=inspect type=(int32) => void
/// @type.symbol symbol=inspect.value source="value: int32" type=int32

    const borrow = &readonly value;
    /// @type.symbol symbol=inspect.borrow source=borrow type=&'frame readonly int32
    /// @resolution.pattern source=borrow kind=binding target=inspect.borrow
    /// @resolution.name source=value target=inspect.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=inspect.value

    borrow satisfies &readonly int32;
    /// @resolution.name source=borrow target=inspect.borrow
    /// @resolution.place source=borrow placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=borrow root=inspect.borrow

}
"#,
    );
}

#[test]
fn test_default_free_borrow_parameter_and_result_to_local() {
    let session = TestSession::single(
        r#"
class User {}

function inspect(value: &readonly User): &readonly User {
    return value;
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

function inspect<'a>(value: &'a readonly User): &'a readonly User {
    return value;
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

function inspect(value: &readonly User): &readonly User {
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect type=<inspect.'a>(&inspect.'a readonly User) => &inspect.'a readonly User
/// @type.symbol symbol=inspect.value source="value: &readonly User" type=&inspect.'a readonly User
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

    return value;
    /// @resolution.name source=value target=inspect.value
    /// @resolution.place source=value placement=inspect.'a lifetime=inspect.'a access="readonly"
    /// @resolution.access source=value root=inspect.value

}
"#,
    );
}

#[test]
fn test_close_optional_and_nested_borrows_locally() {
    let session = TestSession::single(
        r#"
class User {}

struct Holder<T> {
    value: T;
}

declare function maybe(value: &readonly User | undefined): &readonly User | undefined;
declare function inspect(holder: Holder<&readonly User>): void;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct Holder<out T> {
    value: T;
}

declare function maybe<'a>(value: &readonly User | undefined): &readonly User | undefined;
declare function inspect<'a>(holder: Holder<&readonly User>): void;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

struct Holder<T> {
/// @generic.template symbol=Holder parameters=(out T)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(out T)
/// @definition.field symbol=Holder.value source="value: T" key=value type=T
/// @type.symbol symbol=Holder.T source=T type=T

    value: T;
    /// @type.symbol symbol=Holder.value source="value: T" type=T
    /// @resolution.name source=T target=Holder.T

}

declare function maybe(value: &readonly User | undefined): &readonly User | undefined;
/// @generic.template symbol=maybe parameters=('a)
/// @type.symbol symbol=maybe type=<maybe.'a>(&maybe.'a readonly User | undefined) => &maybe.'a readonly User | undefined
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

declare function inspect(holder: Holder<&readonly User>): void;
/// @generic.template symbol=inspect parameters=('a)
/// @type.symbol symbol=inspect source="declare function inspect(holder: Holder<&readonly User>): void" type=<inspect.'a>(Holder<&inspect.'a readonly User>) => void
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=User target=User
"#,
        r#"

"#,
    );
}

#[test]
fn test_project_field_place_through_implicit_receiver() {
    let session = TestSession::single(
        r#"
struct Pair<'a, T> {
    value: &'a readonly T;

    read(this): void {
        let v = this.value;
    }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Pair<'a, out T> {
    value: &'a readonly T;

    read(this): void {
        let v: &'a readonly T = this.value;
    }
}

=== dir ===
struct Pair<'a, T> {
/// @generic.template symbol=Pair parameters=('a, out T)
/// @type.symbol symbol=Pair type=Pair
/// @definition.struct symbol=Pair template=('a, out T)
/// @definition.field symbol=Pair.value source="value: &'a readonly T" key=value type=&'a readonly T
/// @definition.method symbol=Pair.read slot=read type=(this: Pair<'a, T>) => void
/// @type.symbol symbol=Pair.'a source='a type='a
/// @type.symbol symbol=Pair.T source=T type=T

    value: &'a readonly T;
    /// @type.symbol symbol=Pair.value source="value: &'a readonly T" type=&'a readonly T
    /// @resolution.name source='a target=Pair.'a
    /// @resolution.name source=T target=Pair.T

    read(this): void {
    /// @type.symbol symbol=Pair.read type=(this: Pair<'a, T>) => void
    /// @type.symbol symbol=Pair.read.this source=this type=Pair<'a, T>

        let v = this.value;
        /// @type.symbol symbol=Pair.read.v source=v type=&'a readonly T
        /// @resolution.pattern source=v kind=binding target=Pair.read.v
        /// @resolution.member source=this.value receiver=Pair<'a, T> type=&'a readonly T kind=field target_receiver=Pair<'a, T> key=value target=Pair.value target_type=&'a readonly T
        /// @resolution.receiver source=this kind=this declaration=Pair type=Pair<'a, T>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.access source=this.value root=this keys=[value]

    }
}
"#,
    );
}
