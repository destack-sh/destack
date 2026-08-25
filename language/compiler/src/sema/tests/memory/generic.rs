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
        "main.ds",
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
/// @type.symbol symbol=User source="class User {}" type=User
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
fn test_reject_shared_managed_argument_for_default_local_parameter() {
    let session = TestSession::single(
        r#"
class User {}

declare function consume(user: User): void;
declare const user: shared User;

consume(user);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare function consume(user: User): void;
declare const user: shared User;

consume(user);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function consume(user: User): void;
/// @type.symbol symbol=consume source="declare function consume(user: User): void" type=(User) => void
/// @type.symbol symbol=consume.user source="user: User" type=User
/// @resolution.name source=User target=User

declare const user: shared User;
/// @type.symbol symbol=user source=user type=shared User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

consume(user);
/// @resolution.name source=consume target=consume
/// @resolution.call source=consume(user) parameters=(User) arguments=(provided(user) as User) return=void kind=symbol target=consume
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=user root=user
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'shared User' is not assignable to parameter of type 'User'"
/// @diagnostic.label line=7 column=9 span="user" line_source="consume(user);"
/// @diagnostic.related line=7 column=1 span="consume(user)" line_source="consume(user);" message="in this call"
"#,
    );
}

#[test]
fn test_acquire_exclusive_borrow_from_default_local_parameter() {
    let session = TestSession::single(
        r#"
class User {}

declare function consume(value: &exclusive User): void;

function replace(user: User): void {
    consume(user);
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare function consume<'a, P1: Place>(value: Borrowed<User, 'a & P1, "exclusive">): void;

function replace(user: User): void {
    consume<"local">(user as &'frame exclusive User);
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function consume(value: &exclusive User): void;
/// @generic.template symbol=consume parameters=('a, P1: Place)
/// @type.symbol symbol=consume source="declare function consume(value: &exclusive User): void" type=<consume.'a, consume.P1: Place>(Borrowed<User, consume.'a & consume.P1, "exclusive">) => void
/// @type.symbol symbol=consume.value source="value: &exclusive User" type=Borrowed<User, consume.'a & consume.P1, "exclusive">
/// @resolution.name source=User target=User

function replace(user: User): void {
/// @type.symbol symbol=replace type=(User) => void
/// @type.symbol symbol=replace.user source="user: User" type=User
/// @resolution.name source=User target=User

    consume(user);
    /// @resolution.name source=consume target=consume
    /// @resolution.call source=consume(user) parameters=(&'frame exclusive User) arguments=(provided(user) as &'frame exclusive User) return=void kind=symbol target=consume instance="consume<\"local\">"
    /// @generic.instantiation id="consume<\"local\">" template=consume arguments=("local")
    /// @generic.instance id="consume<\"local\">" template=consume arguments=("local")
    /// @resolution.name source=user target=replace.user
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=replace.user

}
"#,
    );
}

#[test]
fn test_preserve_explicit_space_parameter() {
    let session = TestSession::single(
        r#"
class User {}

function identity<const S: Space>(value: Placed<User, S>): Placed<User, S> {
    return value;
}

declare const localUser: local User;
declare const sharedUser: shared User;

identity(localUser) satisfies local User;
identity(sharedUser) satisfies shared User;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

function identity<const S: Space>(value: Placed<User, S>): Placed<User, S> {
    return value;
}

declare const localUser: local User;
declare const sharedUser: shared User;

identity<"local">(localUser) satisfies local User;
identity<"shared">(sharedUser) satisfies shared User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

function identity<const S: Space>(value: Placed<User, S>): Placed<User, S> {
/// @generic.template symbol=identity parameters=(const S: Space)
/// @type.symbol symbol=identity type=<const S: Space>(Managed<User, S>) => Managed<User, S>
/// @type.symbol symbol=identity.S source="const S: Space" type=S
/// @resolution.name source=Space target=Space
/// @type.symbol symbol=identity.value source="value: Placed<User, S>" type=Managed<User, S>
/// @resolution.name source=Placed target=Placed
/// @resolution.name source=User target=User
/// @resolution.name source=S target=identity.S
/// @resolution.name source=Placed target=Placed
/// @resolution.name source=User target=User
/// @resolution.name source=S target=identity.S

    return value;
    /// @resolution.name source=value target=identity.value
    /// @resolution.place source=value placement=S lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=identity.value

}

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=local User
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=shared User
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

identity(localUser) satisfies local User;
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(localUser) parameters=(local User) arguments=(provided(localUser) as local User) return=local User kind=symbol target=identity instance="identity<\"local\">"
/// @generic.instantiation id="identity<\"local\">" template=identity arguments=("local")
/// @generic.instance id="identity<\"local\">" template=identity arguments=("local")
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=localUser root=localUser
/// @resolution.name source=User target=User

identity(sharedUser) satisfies shared User;
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(sharedUser) parameters=(shared User) arguments=(provided(sharedUser) as shared User) return=shared User kind=symbol target=identity instance="identity<\"shared\">"
/// @generic.instantiation id="identity<\"shared\">" template=identity arguments=("shared")
/// @generic.instance id="identity<\"shared\">" template=identity arguments=("shared")
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedUser root=sharedUser
/// @resolution.name source=User target=User
"#,
    );
}

#[test]
fn test_preserve_placement_inside_ordinary_type_parameter() {
    let session = TestSession::single(
        r#"
class User {}

function identity<T>(value: T): T {
    return value;
}

declare const localUser: local User;
declare const sharedUser: shared User;

identity(localUser) satisfies local User;
identity(sharedUser) satisfies shared User;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

function identity<T>(value: T): T {
    return value;
}

declare const localUser: local User;
declare const sharedUser: shared User;

identity<local User>(localUser) satisfies local User;
identity<shared User>(sharedUser) satisfies shared User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

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

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=local User
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=shared User
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

identity(localUser) satisfies local User;
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(localUser) parameters=(local User) arguments=(provided(localUser) as local User) return=local User kind=symbol target=identity instance="identity<local User>"
/// @generic.instantiation id="identity<local User>" template=identity arguments=(local User)
/// @generic.instance id="identity<local User>" template=identity arguments=(local User)
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=localUser root=localUser
/// @resolution.name source=User target=User

identity(sharedUser) satisfies shared User;
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(sharedUser) parameters=(shared User) arguments=(provided(sharedUser) as shared User) return=shared User kind=symbol target=identity instance="identity<shared User>"
/// @generic.instantiation id="identity<shared User>" template=identity arguments=(shared User)
/// @generic.instance id="identity<shared User>" template=identity arguments=(shared User)
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedUser root=sharedUser
/// @resolution.name source=User target=User
"#,
    );
}

#[test]
fn test_resolve_relative_generic_field_from_receiver() {
    let session = TestSession::single(
        r#"
class User {}

struct Box<T> {
    value: T;
}

declare local const localBox: Box<User>;
declare shared const sharedBox: Box<User>;
declare local const mixedBox: Box<shared User>;

localBox.value satisfies local User;
sharedBox.value satisfies shared User;
mixedBox.value satisfies shared User;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct Box<out T> {
    value: T;
}

declare local const localBox: Box<User>;
declare shared const sharedBox: Box<User>;
declare local const mixedBox: Box<shared User>;

localBox.value satisfies local User;
sharedBox.value satisfies shared User;
mixedBox.value satisfies shared User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

struct Box<T> {
/// @generic.template symbol=Box parameters=(out T)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T)
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @type.symbol symbol=Box.T source=T type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

}

declare local const localBox: Box<User>;
/// @type.symbol symbol=localBox source=localBox type=Box<User>
/// @resolution.pattern source=localBox kind=binding target=localBox
/// @generic.instance id=Box<User> template=Box arguments=(User)
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User

declare shared const sharedBox: Box<User>;
/// @type.symbol symbol=sharedBox source=sharedBox type=Box<User>
/// @resolution.pattern source=sharedBox kind=binding target=sharedBox
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User

declare local const mixedBox: Box<shared User>;
/// @type.symbol symbol=mixedBox source=mixedBox type=Box<shared User>
/// @resolution.pattern source=mixedBox kind=binding target=mixedBox
/// @generic.instance id="Box<shared User>" template=Box arguments=(shared User)
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User

localBox.value satisfies local User;
/// @resolution.name source=localBox target=localBox
/// @resolution.member source=localBox.value receiver=Box<User> type=User kind=field target_receiver=Box<User> key=value target=Box.value target_type=User
/// @resolution.place source=localBox placement="local" lifetime="static" access="readonly"
/// @resolution.access source=localBox root=localBox
/// @resolution.place source=localBox.value placement="local" lifetime="static" access="readonly"
/// @resolution.access source=localBox.value root=localBox keys=[value]
/// @resolution.name source=User target=User

sharedBox.value satisfies shared User;
/// @resolution.name source=sharedBox target=sharedBox
/// @resolution.member source=sharedBox.value receiver=Box<User> type=User kind=field target_receiver=Box<User> key=value target=Box.value target_type=User
/// @resolution.place source=sharedBox placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=sharedBox root=sharedBox
/// @resolution.place source=sharedBox.value placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedBox.value root=sharedBox keys=[value]
/// @resolution.name source=User target=User

mixedBox.value satisfies shared User;
/// @resolution.name source=mixedBox target=mixedBox
/// @resolution.member source=mixedBox.value receiver=Box<shared User> type=shared User kind=field target_receiver=Box<shared User> key=value target=Box.value target_type=shared User
/// @resolution.place source=mixedBox placement="local" lifetime="static" access="readonly"
/// @resolution.access source=mixedBox root=mixedBox
/// @resolution.place source=mixedBox.value placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=mixedBox.value root=mixedBox keys=[value]
/// @resolution.name source=User target=User
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
        "main.ds",
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
    borrow satisfies local &readonly int32;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function inspect(value: int32): void {
    const borrow: &'frame readonly int32 = &readonly value;
    borrow satisfies local &readonly int32;
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

    borrow satisfies local &readonly int32;
    /// @resolution.name source=borrow target=inspect.borrow
    /// @resolution.place source=borrow placement="local" lifetime="frame" access="readonly"
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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

function inspect<'a, P1: Place>(
    value: Borrowed<User, 'a & P1, "readonly">,
): Borrowed<User, 'a & P1, "readonly"> {
    return value;
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

function inspect(value: &readonly User): &readonly User {
/// @generic.template symbol=inspect parameters=('a, P1: Place)
/// @type.symbol symbol=inspect type=<inspect.'a, inspect.P1: Place>(Borrowed<User, inspect.'a & inspect.P1, "readonly">) => Borrowed<User, inspect.'a & inspect.P1, "readonly">
/// @type.symbol symbol=inspect.value source="value: &readonly User" type=Borrowed<User, inspect.'a & inspect.P1, "readonly">
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

    return value;
    /// @resolution.name source=value target=inspect.value
    /// @resolution.place source=value placement=inspect.P1 lifetime=inspect.'a access="readonly"
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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct Holder<out T> {
    value: T;
}

declare function maybe<'a, P1: Place>(
    value: Borrowed<User, 'a & P1, "readonly"> | undefined,
): &readonly User | undefined;
declare function inspect<'a, P1: Place>(holder: Holder<Borrowed<User, 'a & P1, "readonly">>): void;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
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
/// @generic.template symbol=maybe parameters=('a, P1: Place)
/// @type.symbol symbol=maybe type=<maybe.'a, maybe.P1: Place>(Borrowed<User, maybe.'a & maybe.P1, "readonly"> | undefined) => Borrowed<User, maybe.'a & maybe.P1, "readonly"> | undefined
/// @type.symbol symbol=maybe.value source="value: &readonly User | undefined" type=Borrowed<User, maybe.'a & maybe.P1, "readonly"> | undefined
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

declare function inspect(holder: Holder<&readonly User>): void;
/// @generic.template symbol=inspect parameters=('a, P1: Place)
/// @type.symbol symbol=inspect source="declare function inspect(holder: Holder<&readonly User>): void" type=<inspect.'a, inspect.P1: Place>(Holder<Borrowed<User, inspect.'a & inspect.P1, "readonly">>) => void
/// @type.symbol symbol=inspect.holder source="holder: Holder<&readonly User>" type=Holder<Borrowed<User, inspect.'a & inspect.P1, "readonly">>
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=User target=User
"#,
        r#"

"#,
    );
}

#[test]
fn test_resolve_constructor_relative_types_from_destination() {
    let session = TestSession::single(
        r#"
class User {}

class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

declare const localUser: local User;
declare const sharedUser: shared User;

const localBox: local Box<User> = new Box(localUser);
const sharedBox: shared Box<User> = new Box(sharedUser);
const mixedBox: local Box<shared User> = new Box(sharedUser);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

class Box<in out T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

declare const localUser: local User;
declare const sharedUser: shared User;

const localBox: local Box<User> = new Box<User, "local">(localUser);
const sharedBox: shared Box<User> = new Box<User, "shared">(sharedUser);
const mixedBox: local Box<shared User> = new Box<shared User, "local">(sharedUser);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

class Box<T> {
/// @generic.template symbol=Box parameters=(in out T)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(in out T)
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=<Box.constructor.P0: Place>(T) => Managed<this, Box.constructor.P0>
/// @type.symbol symbol=Box.T source=T type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

    constructor(value: T) {
    /// @generic.template symbol=Box.constructor parent=template#0 parameters=(P0: Place)
    /// @type.symbol symbol=Box.constructor type=<Box.constructor.P0: Place>(T) => Managed<this, Box.constructor.P0>
    /// @type.symbol symbol=Box.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<T>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Box<T>, target=field(receiver=Box<T>, target=Box.value, type=T), type=T" type=T
        /// @resolution.name source=value target=Box.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Box.constructor.value

    }
}

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=local User
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=shared User
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

const localBox: local Box<User> = new Box(localUser);
/// @type.symbol symbol=localBox source=localBox type=local Box<User>
/// @resolution.pattern source=localBox kind=binding target=localBox
/// @generic.instance id=Box<User> template=Box arguments=(User)
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User
/// @resolution.construct source="new Box(localUser)" parameters=(User) arguments=(provided(localUser) as User) return=local Box<User> kind=class target=Box constructor=Box.constructor instance=Box<User>
/// @generic.instantiation id="Box.constructor<User, \"local\">" template=Box.constructor arguments=(User, "local")
/// @generic.instantiation id="Box<User, \"local\">" template=Box arguments=(User, "local")
/// @generic.instance id="Box.constructor<User, \"local\">" template=Box.constructor arguments=(User, "local")
/// @generic.instance id="Box<User, \"local\">" template=Box arguments=(User, "local")
/// @resolution.name source=Box target=Box
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=localUser root=localUser

const sharedBox: shared Box<User> = new Box(sharedUser);
/// @type.symbol symbol=sharedBox source=sharedBox type=shared Box<User>
/// @resolution.pattern source=sharedBox kind=binding target=sharedBox
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User
/// @resolution.construct source="new Box(sharedUser)" parameters=(shared User) arguments=(provided(sharedUser) as shared User) return=shared Box<User> kind=class target=Box constructor=Box.constructor instance=Box<User>
/// @generic.instantiation id="Box.constructor<User, \"shared\">" template=Box.constructor arguments=(User, "shared")
/// @generic.instantiation id="Box<User, \"shared\">" template=Box arguments=(User, "shared")
/// @generic.instance id="Box.constructor<User, \"shared\">" template=Box.constructor arguments=(User, "shared")
/// @generic.instance id="Box<User, \"shared\">" template=Box arguments=(User, "shared")
/// @resolution.name source=Box target=Box
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedUser root=sharedUser

const mixedBox: local Box<shared User> = new Box(sharedUser);
/// @type.symbol symbol=mixedBox source=mixedBox type=local Box<shared User>
/// @resolution.pattern source=mixedBox kind=binding target=mixedBox
/// @generic.instance id="Box<shared User>" template=Box arguments=(shared User)
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User
/// @resolution.construct source="new Box(sharedUser)" parameters=(shared User) arguments=(provided(sharedUser) as shared User) return=local Box<shared User> kind=class target=Box constructor=Box.constructor instance="Box<shared User>"
/// @generic.instantiation id="Box.constructor<shared User, \"local\">" template=Box.constructor arguments=(shared User, "local")
/// @generic.instantiation id="Box<shared User, \"local\">" template=Box arguments=(shared User, "local")
/// @generic.instance id="Box.constructor<shared User, \"local\">" template=Box.constructor arguments=(shared User, "local")
/// @generic.instance id="Box<shared User, \"local\">" template=Box arguments=(shared User, "local")
/// @resolution.name source=Box target=Box
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedUser root=sharedUser
"#,
    );
}

#[test]
fn test_resolve_generic_borrow_result_place_from_receiver() {
    let session = TestSession::single(
        r#"
interface Box<T> { value: T; }

extension<T> of Box<T> {
    borrow(&readonly this): &readonly T {
        todo("borrow")
    }

    forward(&readonly this): &readonly T {
        this.borrow()
    }
}

class User {}

declare const localBox: local Box<User>;
declare const mixedBox: local Box<shared User>;

localBox.borrow() satisfies local &readonly User;
mixedBox.borrow() satisfies shared &readonly User;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Box<in out T> {
    value: T;
}

extension<T> of Box<T> {
    borrow(&readonly this): Borrowed<T, 'a & P1, "readonly"> {
        todo("borrow" as string | undefined)
    }

    forward(&readonly this): Borrowed<T, 'a & P1, "readonly"> {
        this.borrow<T, P1>()
    }
}

class User {}

declare const localBox: local Box<User>;
declare const mixedBox: local Box<shared User>;

localBox.borrow<User, "local">() satisfies local &readonly User;
mixedBox.borrow<shared User, "local">() satisfies shared &readonly User;

=== dir ===
interface Box<T> { value: T; }
/// @generic.template symbol=Box parameters=(in out T#1)
/// @type.symbol symbol=Box source="interface Box<T> { value: T; }" type=Box
/// @definition.interface symbol=Box source="interface Box<T> { value: T; }" template=(in out T#1)
/// @definition.where symbol=Box source="interface Box<T> { value: T; }" relation=satisfies left=this right=Box<T#1>
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Box.T source=T type=T#1
/// @type.symbol symbol=Box.value source="value: T" type=T#1
/// @resolution.name source=T target=Box.T

extension<T> of Box<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<T#2>
/// @definition.method symbol=borrow slot=borrow type=<borrow.'a, borrow.P1: Place>(this: Borrowed<this, borrow.'a & borrow.P1, "readonly">) => Borrowed<T#2, borrow.'a & borrow.P1, "readonly">
/// @definition.method symbol=forward slot=forward type=<forward.'a, forward.P1: Place>(this: Borrowed<this, forward.'a & forward.P1, "readonly">) => Borrowed<T#2, forward.'a & forward.P1, "readonly">
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T

    borrow(&readonly this): &readonly T {
    /// @generic.template symbol=borrow parent=template#1 parameters=('a, P1: Place)
    /// @type.symbol symbol=borrow type=<borrow.'a, borrow.P1: Place>(this: Borrowed<this, borrow.'a & borrow.P1, "readonly">) => Borrowed<T#2, borrow.'a & borrow.P1, "readonly">
    /// @type.symbol symbol=borrow.this source="&readonly this" type=Borrowed<this, borrow.'a & borrow.P1, "readonly">
    /// @resolution.name source=T target=T

        todo("borrow")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"borrow\")" parameters=(string | undefined) arguments=(provided("borrow") as string | undefined) return=never kind=symbol target=todo

    }

    forward(&readonly this): &readonly T {
    /// @generic.template symbol=forward parent=template#1 parameters=('a, P1: Place)
    /// @type.symbol symbol=forward type=<forward.'a, forward.P1: Place>(this: Borrowed<this, forward.'a & forward.P1, "readonly">) => Borrowed<T#2, forward.'a & forward.P1, "readonly">
    /// @type.symbol symbol=forward.this source="&readonly this" type=Borrowed<this, forward.'a & forward.P1, "readonly">
    /// @resolution.name source=T target=T

        this.borrow()
        /// @resolution.member source=this.borrow receiver=Borrowed<Box<T#2>, forward.'a & forward.P1, "readonly"> type=<borrow.'a, borrow.P1: Place>(this: Borrowed<Box<T#2>, borrow.'a & borrow.P1, "readonly">) => Borrowed<T#2, borrow.'a & borrow.P1, "readonly"> kind=symbol target_receiver=Borrowed<Box<T#2>, forward.'a & forward.P1, "readonly"> dispatch=dynamic constraint=Box<T#2> target=borrow
        /// @resolution.call source=this.borrow() parameters=() return=Borrowed<T#2, forward.'a & forward.P1, "readonly"> kind=dynamic target=borrow receiver=Borrowed<Box<T#2>, forward.'a & forward.P1, "readonly"> constraint=Box<T#2> generic_arguments=(T#2, forward.P1)
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Borrowed<Box<T#2>, forward.'a & forward.P1, "readonly">
        /// @resolution.place source=this placement=forward.P1 lifetime=forward.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=borrow<T#2> template=borrow arguments=(T#2) owner=forward

    }
}

class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const localBox: local Box<User>;
/// @type.symbol symbol=localBox source=localBox type=local Box<User>
/// @resolution.pattern source=localBox kind=binding target=localBox
/// @generic.instance id=Box<User> template=Box arguments=(User)
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User

declare const mixedBox: local Box<shared User>;
/// @type.symbol symbol=mixedBox source=mixedBox type=local Box<shared User>
/// @resolution.pattern source=mixedBox kind=binding target=mixedBox
/// @generic.instance id="Box<shared User>" template=Box arguments=(shared User)
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User

localBox.borrow() satisfies local &readonly User;
/// @resolution.name source=localBox target=localBox
/// @resolution.member source=localBox.borrow receiver=local Box<User> type=<borrow.'a, borrow.P1: Place>(this: Borrowed<Box<User>, borrow.'a & borrow.P1, "readonly">) => Borrowed<User, borrow.'a & borrow.P1, "readonly"> kind=symbol target_receiver=local Box<User> dispatch=dynamic constraint=Box<User> target=borrow
/// @resolution.call source=localBox.borrow() parameters=() return=&'static readonly User kind=dynamic target=borrow receiver=local Box<User> constraint=Box<User> adjustments=(borrow(&'static readonly local Box<User>)) generic_arguments=(User, "local")
/// @resolution.place source=localBox placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=localBox root=localBox
/// @generic.instantiation id=borrow<User> template=borrow arguments=(User)
/// @generic.instance id="borrow<User, \"local\">" template=borrow arguments=(User, "local")
/// @resolution.name source=User target=User

mixedBox.borrow() satisfies shared &readonly User;
/// @resolution.name source=mixedBox target=mixedBox
/// @resolution.member source=mixedBox.borrow receiver=local Box<shared User> type=<borrow.'a, borrow.P1: Place>(this: Borrowed<Box<shared User>, borrow.'a & borrow.P1, "readonly">) => Borrowed<shared User, borrow.'a & borrow.P1, "readonly"> kind=symbol target_receiver=local Box<shared User> dispatch=dynamic constraint=Box<shared User> target=borrow
/// @resolution.call source=mixedBox.borrow() parameters=() return=&'static readonly shared User kind=dynamic target=borrow receiver=local Box<shared User> constraint=Box<shared User> adjustments=(borrow(&'static readonly local Box<shared User>)) generic_arguments=(shared User, "local")
/// @resolution.place source=mixedBox placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=mixedBox root=mixedBox
/// @generic.instantiation id="borrow<shared User>" template=borrow arguments=(shared User)
/// @generic.instance id="borrow<shared User, \"local\">" template=borrow arguments=(shared User, "local")
/// @resolution.name source=User target=User
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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Pair<'a, out T> {
    value: Borrowed<T, 'a, "readonly">;

    read(this): void {
        let v: Borrowed<T, 'a, "readonly"> = this.value;
    }
}

=== dir ===
struct Pair<'a, T> {
/// @generic.template symbol=Pair parameters=('a, out T)
/// @type.symbol symbol=Pair type=Pair
/// @definition.struct symbol=Pair template=('a, out T)
/// @definition.field symbol=Pair.value source="value: &'a readonly T" key=value type=&'a readonly T
/// @definition.method symbol=Pair.read slot=read type=(this: this) => void
/// @type.symbol symbol=Pair.'a source='a type='a
/// @type.symbol symbol=Pair.T source=T type=T

    value: &'a readonly T;
    /// @type.symbol symbol=Pair.value source="value: &'a readonly T" type=&'a readonly T
    /// @resolution.name source='a target=Pair.'a
    /// @resolution.name source=T target=Pair.T

    read(this): void {
    /// @type.symbol symbol=Pair.read type=(this: this) => void
    /// @type.symbol symbol=Pair.read.this source=this type=this

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
