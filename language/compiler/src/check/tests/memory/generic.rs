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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

function identity(value: User): User {
    return value;
}

declare function create(): User;

=== checked ===
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare function consume(user: User): void;
declare const user: shared User;

consume(user);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function consume(user: User): void;
/// @type.symbol symbol=consume source="declare function consume(user: User): void" type=(User) => void
/// @type.symbol symbol=consume.user source="user: User" type=User
/// @resolution.name source=User target=User

declare const user: shared User;
/// @type.symbol symbol=user source=user type=Placed<User, "shared">
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

consume(user);
/// @resolution.name source=consume target=consume
/// @resolution.call source=consume(user) parameters=(User) arguments=(provided(user) as User) return=void kind=symbol target=consume
/// @resolution.name source=user target=user
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

declare function consume<comptime L0: Lifetime>(value: Borrowed<User, L0, "exclusive">): void;

function replace(user: User): void {
    consume(user as Borrowed<User, "frame", "exclusive">);
}

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare function consume(value: &exclusive User): void;
/// @generic.template symbol=consume parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=consume source="declare function consume(value: &exclusive User): void" type=<comptime consume.L0: Lifetime>(Borrowed<User, consume.L0, "exclusive">) => void
/// @type.symbol symbol=consume.value source="value: &exclusive User" type=Borrowed<User, consume.L0, "exclusive">
/// @resolution.name source=User target=User

function replace(user: User): void {
/// @type.symbol symbol=replace type=(User) => void
/// @type.symbol symbol=replace.user source="user: User" type=User
/// @resolution.name source=User target=User

    consume(user);
    /// @resolution.name source=consume target=consume
    /// @resolution.call source=consume(user) parameters=(Borrowed<User, "frame", "exclusive">) arguments=(provided(user) as Borrowed<User, "frame", "exclusive">) return=void kind=symbol target=consume
    /// @resolution.name source=user target=replace.user

}
"#,
    );
}

#[test]
fn test_preserve_explicit_space_parameter() {
    let session = TestSession::single(
        r#"
class User {}

function identity<comptime S: Space>(value: Placed<User, S>): Placed<User, S> {
    return value;
}

declare const localUser: local User;
declare const sharedUser: shared User;

identity(localUser) satisfies local User;
identity(sharedUser) satisfies shared User;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

function identity<comptime S: Space>(value: Placed<User, S>): Placed<User, S> {
    return value;
}

declare const localUser: local User;
declare const sharedUser: shared User;

identity<"local">(localUser) satisfies local User;
identity<"shared">(sharedUser) satisfies shared User;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

function identity<comptime S: Space>(value: Placed<User, S>): Placed<User, S> {
/// @generic.template symbol=identity parameters=(comptime S: Space)
/// @type.symbol symbol=identity type=<comptime S: Space>(Placed<User, S>) => Placed<User, S>
/// @type.symbol symbol=identity.S source="comptime S: Space" type=S
/// @resolution.name source=Space target=memory.place.Space
/// @type.symbol symbol=identity.value source="value: Placed<User, S>" type=Placed<User, S>
/// @resolution.name source=Placed target=memory.place.Placed
/// @resolution.name source=User target=User
/// @resolution.name source=S target=identity.S
/// @resolution.name source=Placed target=memory.place.Placed
/// @resolution.name source=User target=User
/// @resolution.name source=S target=identity.S

    return value;
    /// @resolution.name source=value target=identity.value

}

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=Placed<User, "local">
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=Placed<User, "shared">
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

identity(localUser) satisfies local User;
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(localUser) parameters=(Placed<User, "local">) arguments=(provided(localUser) as Placed<User, "local">) return=Placed<User, "local"> kind=symbol target=identity instance="identity<\"local\">"
/// @generic.instance source=identity(localUser) id="identity<\"local\">"
/// @resolution.name source=localUser target=localUser
/// @resolution.name source=User target=User

identity(sharedUser) satisfies shared User;
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(sharedUser) parameters=(Placed<User, "shared">) arguments=(provided(sharedUser) as Placed<User, "shared">) return=Placed<User, "shared"> kind=symbol target=identity instance="identity<\"shared\">"
/// @generic.instance source=identity(sharedUser) id="identity<\"shared\">"
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.name source=User target=User

/// @generic.instance id="Placed<User, S>" template=memory.place.Placed arguments=(User, S)
/// @generic.instance id="identity<\"local\">" template=identity arguments=("local")
/// @generic.instance id="identity<\"shared\">" template=identity arguments=("shared")
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

    session.assert_dir_checked(
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

=== checked ===
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

}

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=Placed<User, "local">
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=Placed<User, "shared">
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

identity(localUser) satisfies local User;
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(localUser) parameters=(Placed<User, "local">) arguments=(provided(localUser) as Placed<User, "local">) return=Placed<User, "local"> kind=symbol target=identity instance="identity<Placed<User, \"local\">>"
/// @generic.instance source=identity(localUser) id="identity<Placed<User, \"local\">>"
/// @resolution.name source=localUser target=localUser
/// @resolution.name source=User target=User

identity(sharedUser) satisfies shared User;
/// @resolution.name source=identity target=identity
/// @resolution.call source=identity(sharedUser) parameters=(Placed<User, "shared">) arguments=(provided(sharedUser) as Placed<User, "shared">) return=Placed<User, "shared"> kind=symbol target=identity instance="identity<Placed<User, \"shared\">>"
/// @generic.instance source=identity(sharedUser) id="identity<Placed<User, \"shared\">>"
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.name source=User target=User

/// @generic.instance id="identity<Placed<User, \"local\">>" template=identity arguments=(Placed<User, "local">)
/// @generic.instance id="identity<Placed<User, \"shared\">>" template=identity arguments=(Placed<User, "shared">)
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

declare const localBox: local Box<User>;
declare const sharedBox: shared Box<User>;
declare const mixedBox: local Box<shared User>;

localBox.value satisfies local User;
sharedBox.value satisfies shared User;
mixedBox.value satisfies shared User;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct Box<out T> {
    value: T;
}

declare const localBox: local Box<User>;
declare const sharedBox: shared Box<User>;
declare const mixedBox: local Box<shared User>;

localBox.value satisfies local User;
sharedBox.value satisfies shared User;
mixedBox.value satisfies shared User;

=== checked ===
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

declare const localBox: local Box<User>;
/// @type.symbol symbol=localBox source=localBox type=Placed<Box<User>, "local">
/// @resolution.pattern source=localBox kind=binding target=localBox
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User

declare const sharedBox: shared Box<User>;
/// @type.symbol symbol=sharedBox source=sharedBox type=Placed<Box<User>, "shared">
/// @resolution.pattern source=sharedBox kind=binding target=sharedBox
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User

declare const mixedBox: local Box<shared User>;
/// @type.symbol symbol=mixedBox source=mixedBox type=Placed<Box<Placed<User, "shared">>, "local">
/// @resolution.pattern source=mixedBox kind=binding target=mixedBox
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User

localBox.value satisfies local User;
/// @resolution.name source=localBox target=localBox
/// @resolution.member source=localBox.value receiver=Placed<Box<User>, "local"> kind=symbol target=Box.value
/// @resolution.name source=User target=User

sharedBox.value satisfies shared User;
/// @resolution.name source=sharedBox target=sharedBox
/// @resolution.member source=sharedBox.value receiver=Placed<Box<User>, "shared"> kind=symbol target=Box.value
/// @resolution.name source=User target=User

mixedBox.value satisfies shared User;
/// @resolution.name source=mixedBox target=mixedBox
/// @resolution.member source=mixedBox.value receiver=Placed<Box<Placed<User, "shared">>, "local"> kind=symbol target=Box.value
/// @resolution.name source=User target=User

/// @generic.instance id="Box<Placed<User, \"shared\">>" template=Box arguments=(Placed<User, "shared">)
/// @generic.instance id=Box<User> template=Box arguments=(User)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function negate(value: boolean): boolean {
    return !value;
}

=== checked ===
function negate(value: boolean): boolean {
/// @type.symbol symbol=negate type=(boolean) => boolean
/// @type.symbol symbol=negate.value source="value: boolean" type=boolean

    return !value;
    /// @resolution.operator source=!value kind=builtin
    /// @resolution.name source=value target=negate.value

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function inspect(value: int32): void {
    const borrow: Borrowed<int32, "frame", "readonly"> = &readonly value;
    borrow satisfies local &readonly int32;
}

=== checked ===
function inspect(value: int32): void {
/// @type.symbol symbol=inspect type=(int32) => void
/// @type.symbol symbol=inspect.value source="value: int32" type=int32

    const borrow = &readonly value;
    /// @type.symbol symbol=inspect.borrow source=borrow type=Borrowed<int32, "frame", "readonly">
    /// @resolution.pattern source=borrow kind=binding target=inspect.borrow
    /// @resolution.name source=value target=inspect.value

    borrow satisfies local &readonly int32;
    /// @resolution.name source=borrow target=inspect.borrow

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

function inspect<comptime L0: Lifetime>(
    value: Borrowed<User, L0, "readonly">,
): Borrowed<User, L0, "readonly"> {
    return value;
}

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

function inspect(value: &readonly User): &readonly User {
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect type=<comptime inspect.L0: Lifetime>(Borrowed<User, inspect.L0, "readonly">) => Borrowed<User, inspect.L0, "readonly">
/// @type.symbol symbol=inspect.value source="value: &readonly User" type=Borrowed<User, inspect.L0, "readonly">
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

    return value;
    /// @resolution.name source=value target=inspect.value

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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct Holder<out T> {
    value: T;
}

declare function maybe<comptime L0: Lifetime>(
    value: Borrowed<User, L0, "readonly"> | undefined,
): &readonly User | undefined;
declare function inspect<comptime L0: Lifetime>(
    holder: Holder<Borrowed<User, L0, "readonly">>,
): void;

=== checked ===
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
/// @generic.template symbol=maybe parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=maybe type=<comptime maybe.L0: Lifetime>(Borrowed<User, maybe.L0, "readonly"> | undefined) => Borrowed<User, maybe.L0, "readonly"> | undefined
/// @type.symbol symbol=maybe.value source="value: &readonly User | undefined" type=Borrowed<User, maybe.L0, "readonly"> | undefined
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

declare function inspect(holder: Holder<&readonly User>): void;
/// @generic.template symbol=inspect parameters=(comptime L0: Lifetime)
/// @type.symbol symbol=inspect source="declare function inspect(holder: Holder<&readonly User>): void" type=<comptime inspect.L0: Lifetime>(Holder<Borrowed<User, inspect.L0, "readonly">>) => void
/// @type.symbol symbol=inspect.holder source="holder: Holder<&readonly User>" type=Holder<Borrowed<User, inspect.L0, "readonly">>
/// @resolution.name source=Holder target=Holder
/// @resolution.name source=User target=User

/// @generic.instance id="Holder<Borrowed<User, inspect.L0, \"readonly\">>" template=Holder arguments=(Borrowed<User, inspect.L0, "readonly">)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

class Box<in out T> {
    value: T;

    constructor(value: T): this {
        this.value = value;
    }
}

declare const localUser: local User;
declare const sharedUser: shared User;

const localBox: local Box<User> = new Box<User>(localUser);
const sharedBox: shared Box<User> = new Box<User>(sharedUser);
const mixedBox: local Box<shared User> = new Box<shared User>(sharedUser);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

class Box<T> {
/// @generic.template symbol=Box parameters=(in out T)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(in out T)
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(T) => this
/// @type.symbol symbol=Box.T source=T type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

    constructor(value: T) {
    /// @type.symbol symbol=Box.constructor type=(T) => this
    /// @type.symbol symbol=Box.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<T>
        /// @resolution.pattern.assign source=this.value kind=place place=field(Box.value) type=T
        /// @resolution.name source=value target=Box.constructor.value

    }
}

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=Placed<User, "local">
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=Placed<User, "shared">
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

const localBox: local Box<User> = new Box(localUser);
/// @type.symbol symbol=localBox source=localBox type=Placed<Box<User>, "local">
/// @resolution.pattern source=localBox kind=binding target=localBox
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User
/// @resolution.construct source="new Box(localUser)" parameters=(User) arguments=(provided(localUser) as User) return=Placed<Box<User>, "local"> kind=class target=Box constructor=Box.constructor instance=Box<User>
/// @generic.instance source="new Box(localUser)" id=Box<User>
/// @resolution.name source=Box target=Box
/// @resolution.name source=localUser target=localUser

const sharedBox: shared Box<User> = new Box(sharedUser);
/// @type.symbol symbol=sharedBox source=sharedBox type=Placed<Box<User>, "shared">
/// @resolution.pattern source=sharedBox kind=binding target=sharedBox
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User
/// @resolution.construct source="new Box(sharedUser)" parameters=(Placed<User, "shared">) arguments=(provided(sharedUser) as Placed<User, "shared">) return=Placed<Box<User>, "shared"> kind=class target=Box constructor=Box.constructor instance=Box<User>
/// @generic.instance source="new Box(sharedUser)" id=Box<User>
/// @resolution.name source=Box target=Box
/// @resolution.name source=sharedUser target=sharedUser

const mixedBox: local Box<shared User> = new Box(sharedUser);
/// @type.symbol symbol=mixedBox source=mixedBox type=Placed<Box<Placed<User, "shared">>, "local">
/// @resolution.pattern source=mixedBox kind=binding target=mixedBox
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User
/// @resolution.construct source="new Box(sharedUser)" parameters=(Placed<User, "shared">) arguments=(provided(sharedUser) as Placed<User, "shared">) return=Placed<Box<Placed<User, "shared">>, "local"> kind=class target=Box constructor=Box.constructor instance="Box<Placed<User, \"shared\">>"
/// @generic.instance source="new Box(sharedUser)" id="Box<Placed<User, \"shared\">>"
/// @resolution.name source=Box target=Box
/// @resolution.name source=sharedUser target=sharedUser

/// @generic.instance id="Box<Placed<User, \"shared\">>" template=Box arguments=(Placed<User, "shared">)
/// @generic.instance id=Box<User> template=Box arguments=(User)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Box<in out T> {
    value: T;
}

extension<T> of Box<T> {
    borrow(&readonly this): Borrowed<T, L0, "readonly"> {
        todo("borrow" as string | undefined)
    }

    forward(&readonly this): Borrowed<T, L0, "readonly"> {
        this.borrow<T>()
    }
}

class User {}

declare const localBox: local Box<User>;
declare const mixedBox: local Box<shared User>;

localBox.borrow<User>() satisfies local &readonly User;
mixedBox.borrow<shared User>() satisfies shared &readonly User;

=== checked ===
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
/// @definition.method symbol=borrow slot=borrow type=<comptime borrow.L0: Lifetime>(this: Borrowed<this, borrow.L0, "readonly">) => Borrowed<T#2, borrow.L0, "readonly">
/// @definition.method symbol=forward slot=forward type=<comptime forward.L0: Lifetime>(this: Borrowed<this, forward.L0, "readonly">) => Borrowed<T#2, forward.L0, "readonly">
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T

    borrow(&readonly this): &readonly T {
    /// @generic.template symbol=borrow parent=template#1 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=borrow type=<comptime borrow.L0: Lifetime>(this: Borrowed<this, borrow.L0, "readonly">) => Borrowed<T#2, borrow.L0, "readonly">
    /// @type.symbol symbol=borrow.this source="&readonly this" type=Borrowed<this, borrow.L0, "readonly">
    /// @resolution.name source=T target=T

        todo("borrow")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"borrow\")" parameters=(string | undefined) arguments=(provided("borrow") as string | undefined) return=never kind=symbol target=error.panic.todo

    }

    forward(&readonly this): &readonly T {
    /// @generic.template symbol=forward parent=template#1 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=forward type=<comptime forward.L0: Lifetime>(this: Borrowed<this, forward.L0, "readonly">) => Borrowed<T#2, forward.L0, "readonly">
    /// @type.symbol symbol=forward.this source="&readonly this" type=Borrowed<this, forward.L0, "readonly">
    /// @resolution.name source=T target=T

        this.borrow()
        /// @resolution.member source=this.borrow receiver=Borrowed<Box<T#2>, forward.L0, "readonly"> kind=symbol target=borrow
        /// @resolution.call source=this.borrow() parameters=() return=Borrowed<T#2, forward.L0, "readonly"> kind=symbol target=borrow receiver=Borrowed<Box<T#2>, forward.L0, "readonly"> instance=Box<T#2>.<extension#1>.borrow
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Borrowed<Box<T#2>, forward.L0, "readonly">
        /// @generic.instance source=this.borrow() id=Box<T#2>.<extension#1>.borrow

    }
}

class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const localBox: local Box<User>;
/// @type.symbol symbol=localBox source=localBox type=Placed<Box<User>, "local">
/// @resolution.pattern source=localBox kind=binding target=localBox
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User

declare const mixedBox: local Box<shared User>;
/// @type.symbol symbol=mixedBox source=mixedBox type=Placed<Box<Placed<User, "shared">>, "local">
/// @resolution.pattern source=mixedBox kind=binding target=mixedBox
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User

localBox.borrow() satisfies local &readonly User;
/// @resolution.name source=localBox target=localBox
/// @resolution.member source=localBox.borrow receiver=Placed<Box<User>, "local"> kind=symbol target=borrow
/// @resolution.call source=localBox.borrow() parameters=() return=Borrowed<User, "frame", "readonly"> kind=symbol target=borrow receiver=Placed<Box<User>, "local"> adjustments=(borrow) instance=Box<User>.<extension#1>.borrow
/// @generic.instance source=localBox.borrow() id=Box<User>.<extension#1>.borrow
/// @resolution.name source=User target=User

mixedBox.borrow() satisfies shared &readonly User;
/// @resolution.name source=mixedBox target=mixedBox
/// @resolution.member source=mixedBox.borrow receiver=Placed<Box<Placed<User, "shared">>, "local"> kind=symbol target=borrow
/// @resolution.call source=mixedBox.borrow() parameters=() return=Placed<Borrowed<User, "frame", "readonly">, "shared"> kind=symbol target=borrow receiver=Placed<Box<Placed<User, "shared">>, "local"> adjustments=(borrow) instance="Box<Placed<User, \"shared\">>.<extension#1>.borrow"
/// @generic.instance source=mixedBox.borrow() id="Box<Placed<User, \"shared\">>.<extension#1>.borrow"
/// @resolution.name source=User target=User

/// @generic.instance id="Box<Placed<User, \"shared\">>" template=Box arguments=(Placed<User, "shared">)
/// @generic.instance id="Box<Placed<User, \"shared\">>.<extension#1>.borrow" template=borrow arguments=(Placed<User, "shared">)
/// @generic.instance id=Box<T#2>.<extension#1>.borrow template=borrow arguments=(T#2)
/// @generic.instance id=Box<User> template=Box arguments=(User)
/// @generic.instance id=Box<User>.<extension#1>.borrow template=borrow arguments=(User)
"#,
    );
}
