use crate::tests::{DirRows, TestSession};

#[test]
fn test_store_local_and_shared_references_in_local_aggregate() {
    let session = TestSession::single(
        r#"
class User {}

struct Cache {
    localUser: local User;
    sharedUser: shared User;
}

declare const localUser: local User;
declare const sharedUser: shared User;

const cache: local Cache = Cache { localUser, sharedUser };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct Cache {
    localUser: local User;
    sharedUser: shared User;
}

declare const localUser: local User;
declare const sharedUser: shared User;

const cache: local Cache = local Cache { localUser, sharedUser };

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

struct Cache {
/// @type.symbol symbol=Cache type=Cache
/// @definition.struct symbol=Cache
/// @definition.field symbol=Cache.localUser source="localUser: local User" key=localUser type=Placed<User, "local">
/// @definition.field symbol=Cache.sharedUser source="sharedUser: shared User" key=sharedUser type=Placed<User, "shared">

    localUser: local User;
    /// @type.symbol symbol=Cache.localUser source="localUser: local User" type=Placed<User, "local">
    /// @resolution.name source=User target=User

    sharedUser: shared User;
    /// @type.symbol symbol=Cache.sharedUser source="sharedUser: shared User" type=Placed<User, "shared">
    /// @resolution.name source=User target=User

}

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=Placed<User, "local">
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=Placed<User, "shared">
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

const cache: local Cache = Cache { localUser, sharedUser };
/// @type.symbol symbol=cache source=cache type=Placed<Cache, "local">
/// @resolution.pattern source=cache kind=binding target=cache
/// @resolution.name source=Cache target=Cache
/// @resolution.name source=Cache target=Cache
/// @resolution.name source=localUser target=localUser
/// @resolution.name source=sharedUser target=sharedUser
"#,
        r#"
"#,
    );
}

#[test]
fn test_store_shared_references_and_values_in_shared_aggregate() {
    let session = TestSession::single(
        r#"
class User {}

shared struct Cache {
    user: shared User;
    count: int32;
}

declare const user: shared User;
const cache: shared Cache = Cache { user, count: 1 };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared struct Cache {
    user: shared User;
    count: int32;
}

declare const user: shared User;
const cache: shared Cache = Cache { user, count: 1 };

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

shared struct Cache {
/// @type.symbol symbol=Cache type=Cache
/// @definition.struct symbol=Cache
/// @definition.field symbol=Cache.count source="count: int32" key=count type=int32
/// @definition.field symbol=Cache.user source="user: shared User" key=user type=Placed<User, "shared">

    user: shared User;
    /// @type.symbol symbol=Cache.user source="user: shared User" type=Placed<User, "shared">
    /// @resolution.name source=User target=User

    count: int32;
    /// @type.symbol symbol=Cache.count source="count: int32" type=int32

}

declare const user: shared User;
/// @type.symbol symbol=user source=user type=Placed<User, "shared">
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const cache: shared Cache = Cache { user, count: 1 };
/// @type.symbol symbol=cache source=cache type=Placed<Cache, "shared">
/// @resolution.pattern source=cache kind=binding target=cache
/// @resolution.name source=Cache target=Cache
/// @resolution.name source=Cache target=Cache
/// @resolution.name source=user target=user
"#,
        r#"
"#,
    );
}

#[test]
fn test_reject_local_reference_nested_in_shared_storage() {
    let session = TestSession::single(
        r#"
class User {}

struct BoxedUser {
    user: local User;
}

declare const user: local User;

const field: shared BoxedUser = BoxedUser { user };
const tuple: shared (local User, int32) = (user, 1);
const union: shared (local User | undefined) = user;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct BoxedUser {
    user: local User;
}

declare const user: local User;

const field: shared BoxedUser = shared BoxedUser { user };
const tuple: shared (local User, int32) = (user, 1);
const union: shared (local User | undefined) = user as shared (local User | undefined);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

struct BoxedUser {
/// @type.symbol symbol=BoxedUser type=BoxedUser
/// @definition.struct symbol=BoxedUser
/// @definition.field symbol=BoxedUser.user source="user: local User" key=user type=Placed<User, "local">

    user: local User;
    /// @type.symbol symbol=BoxedUser.user source="user: local User" type=Placed<User, "local">
    /// @resolution.name source=User target=User

}

declare const user: local User;
/// @type.symbol symbol=user source=user type=Placed<User, "local">
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const field: shared BoxedUser = BoxedUser { user };
/// @type.symbol symbol=field source=field type=Placed<BoxedUser, "shared">
/// @resolution.pattern source=field kind=binding target=field
/// @resolution.name source=BoxedUser target=BoxedUser
/// @resolution.name source=BoxedUser target=BoxedUser
/// @resolution.name source=user target=user

const tuple: shared (local User, int32) = (user, 1);
/// @type.symbol symbol=tuple source=tuple type=Placed<(Placed<User, "local">, int32), "shared">
/// @resolution.pattern source=tuple kind=binding target=tuple
/// @resolution.name source=User target=User
/// @resolution.name source=user target=user

const union: shared (local User | undefined) = user;
/// @type.symbol symbol=union source=union type=Placed<Placed<User, "local"> | undefined, "shared">
/// @resolution.pattern source=union kind=binding target=union
/// @resolution.name source=User target=User
/// @resolution.name source=user target=user
"#,
        r#"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=10 column=14 span="shared" line_source="const field: shared BoxedUser = BoxedUser { user };"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=11 column=14 span="shared" line_source="const tuple: shared (local User, int32) = (user, 1);"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=12 column=14 span="shared" line_source="const union: shared (local User | undefined) = user;"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
"#,
    );
}

#[test]
fn test_reject_explicit_local_field_in_shared_declaration() {
    let session = TestSession::single(
        r#"
class User {}

shared struct State {
    user: local User;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared struct State {
    user: local User;
}

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

shared struct State {
/// @type.symbol symbol=State type=State
/// @definition.struct symbol=State
/// @definition.field symbol=State.user source="user: local User" key=user type=Placed<User, "local">

    user: local User;
    /// @type.symbol symbol=State.user source="user: local User" type=Placed<User, "local">
    /// @resolution.name source=User target=User

}
"#,
        r#"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=5 column=5 span="user" line_source="user: local User;"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
"#,
    );
}

#[test]
fn test_resolve_relative_fields_in_shared_declarations() {
    let session = TestSession::single(
        r#"
class User {}

shared class Service {
    user!: User;

    accept(user: local User): void {}
}

declare const service: Service;
service.user satisfies shared User;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared class Service {
    user!: User;

    accept(user: local User): void {}
}

declare const service: Service;
service.user satisfies shared User;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

shared class Service {
/// @type.symbol symbol=Service type=Service
/// @definition.class symbol=Service
/// @definition.field symbol=Service.user source="user!: User" key=user type=User
/// @definition.method symbol=Service.accept source="accept(user: local User): void {}" slot=accept type=(this: this, Placed<User, "local">) => void

    user!: User;
    /// @type.symbol symbol=Service.user source="user!: User" type=User
    /// @resolution.name source=User target=User

    accept(user: local User): void {}
    /// @type.symbol symbol=Service.accept source="accept(user: local User): void {}" type=(this: this, Placed<User, "local">) => void
    /// @type.symbol symbol=Service.accept.user source="user: local User" type=Placed<User, "local">
    /// @resolution.name source=User target=User

}

declare const service: Service;
/// @type.symbol symbol=service source=service type=Service
/// @resolution.pattern source=service kind=binding target=service
/// @resolution.name source=Service target=Service

service.user satisfies shared User;
/// @resolution.name source=service target=service
/// @resolution.member source=service.user receiver=Service kind=symbol target=Service.user
/// @resolution.name source=User target=User
"#,
    );
}

#[test]
fn test_reject_local_reference_containment_after_generic_substitution() {
    let session = TestSession::single(
        r#"
class User {}

struct Box<T> {
    value: T;
}

declare const localUser: local User;
declare const sharedUser: shared User;

const rejected: shared Box<local User> = Box { value: localUser };
const accepted: shared Box<shared User> = Box { value: sharedUser };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct Box<out T> {
    value: T;
}

declare const localUser: local User;
declare const sharedUser: shared User;

const rejected: shared Box<local User> = shared Box<local User> { value: localUser };
const accepted: shared Box<shared User> = shared Box<shared User> { value: sharedUser };

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

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=Placed<User, "local">
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=Placed<User, "shared">
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

const rejected: shared Box<local User> = Box { value: localUser };
/// @type.symbol symbol=rejected source=rejected type=Placed<Box<Placed<User, "local">>, "shared">
/// @resolution.pattern source=rejected kind=binding target=rejected
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User
/// @resolution.name source=Box target=Box
/// @resolution.name source=localUser target=localUser

const accepted: shared Box<shared User> = Box { value: sharedUser };
/// @type.symbol symbol=accepted source=accepted type=Placed<Box<Placed<User, "shared">>, "shared">
/// @resolution.pattern source=accepted kind=binding target=accepted
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User
/// @resolution.name source=Box target=Box
/// @resolution.name source=sharedUser target=sharedUser

/// @generic.instance id="Box<Placed<User, \"local\">>" template=Box arguments=(Placed<User, "local">)
/// @generic.instance id="Box<Placed<User, \"shared\">>" template=Box arguments=(Placed<User, "shared">)
"#,
        r#"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=11 column=17 span="shared" line_source="const rejected: shared Box<local User> = Box { value: localUser };"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
"#,
    );
}

#[test]
fn test_allow_raw_local_pointer_in_shared_declaration() {
    let session = TestSession::single(
        r#"
class User {}

shared struct EscapeHatch {
    pointer: local *User;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared struct EscapeHatch {
    pointer: local *User;
}

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

shared struct EscapeHatch {
/// @type.symbol symbol=EscapeHatch type=EscapeHatch
/// @definition.struct symbol=EscapeHatch
/// @definition.field symbol=EscapeHatch.pointer source="pointer: local *User" key=pointer type=Placed<Raw<User>, "local">

    pointer: local *User;
    /// @type.symbol symbol=EscapeHatch.pointer source="pointer: local *User" type=Placed<Raw<User>, "local">
    /// @resolution.name source=User target=User

}
"#,
        r#""#,
    );
}

#[test]
fn test_reject_local_owned_and_borrowed_references_in_shared_space() {
    let session = TestSession::single(
        r#"
class User {}

struct OwnedBox { value: local ^User; }
struct BorrowedBox { value: local Borrowed<User, "static">; }

declare const owned: local ^User;
declare const borrowed: local Borrowed<User, "static">;

const ownedBox: shared OwnedBox = OwnedBox { value: owned };
const borrowedBox: shared BorrowedBox = BorrowedBox { value: borrowed };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct OwnedBox {
    value: local ^User;
}
struct BorrowedBox {
    value: local Borrowed<User, "static", "mutable">;
}

declare const owned: local ^User;
declare const borrowed: local Borrowed<User, "static", "mutable">;

const ownedBox: shared OwnedBox = shared OwnedBox { value: owned };
const borrowedBox: shared BorrowedBox = shared BorrowedBox { value: borrowed };

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

struct OwnedBox { value: local ^User; }
/// @type.symbol symbol=OwnedBox source="struct OwnedBox { value: local ^User; }" type=OwnedBox
/// @definition.struct symbol=OwnedBox source="struct OwnedBox { value: local ^User; }"
/// @definition.field symbol=OwnedBox.value source="value: local ^User" key=value type=Placed<Owned<User>, "local">
/// @type.symbol symbol=OwnedBox.value source="value: local ^User" type=Placed<Owned<User>, "local">
/// @resolution.name source=User target=User

struct BorrowedBox { value: local Borrowed<User, "static">; }
/// @type.symbol symbol=BorrowedBox source="struct BorrowedBox { value: local Borrowed<User, \"static\">; }" type=BorrowedBox
/// @definition.struct symbol=BorrowedBox source="struct BorrowedBox { value: local Borrowed<User, \"static\">; }"
/// @definition.field symbol=BorrowedBox.value source="value: local Borrowed<User, \"static\">" key=value type=Placed<Borrowed<User, "static", "mutable">, "local">
/// @type.symbol symbol=BorrowedBox.value source="value: local Borrowed<User, \"static\">" type=Placed<Borrowed<User, "static", "mutable">, "local">
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=User target=User

declare const owned: local ^User;
/// @type.symbol symbol=owned source=owned type=Placed<Owned<User>, "local">
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=User target=User

declare const borrowed: local Borrowed<User, "static">;
/// @type.symbol symbol=borrowed source=borrowed type=Placed<Borrowed<User, "static", "mutable">, "local">
/// @resolution.pattern source=borrowed kind=binding target=borrowed
/// @resolution.name source=Borrowed target=memory.borrow.Borrowed
/// @resolution.name source=User target=User

const ownedBox: shared OwnedBox = OwnedBox { value: owned };
/// @type.symbol symbol=ownedBox source=ownedBox type=Placed<OwnedBox, "shared">
/// @resolution.pattern source=ownedBox kind=binding target=ownedBox
/// @resolution.name source=OwnedBox target=OwnedBox
/// @resolution.name source=OwnedBox target=OwnedBox
/// @resolution.name source=owned target=owned

const borrowedBox: shared BorrowedBox = BorrowedBox { value: borrowed };
/// @type.symbol symbol=borrowedBox source=borrowedBox type=Placed<BorrowedBox, "shared">
/// @resolution.pattern source=borrowedBox kind=binding target=borrowedBox
/// @resolution.name source=BorrowedBox target=BorrowedBox
/// @resolution.name source=BorrowedBox target=BorrowedBox
/// @resolution.name source=borrowed target=borrowed

/// @generic.instance id="Borrowed<User, \"static\", \"mutable\">" template=memory.borrow.Borrowed arguments=(User, "static", "mutable")
"#,
        r#"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=10 column=17 span="shared" line_source="const ownedBox: shared OwnedBox = OwnedBox { value: owned };"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=11 column=20 span="shared" line_source="const borrowedBox: shared BorrowedBox = BorrowedBox { value: borrowed };"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
"#,
    );
}

#[test]
fn test_reject_local_constructor_argument_for_shared_relative_field() {
    let session = TestSession::single(
        r#"
class User {}

class Box {
    user: User;
    constructor(user: User) { this.user = user; }
}

declare const user: local User;
const box: shared Box = new Box(user);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

class Box {
    user: User;
    constructor(user: User): this {
        this.user = user;
    }
}

declare const user: local User;
const box: shared Box = new Box(user);

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

class Box {
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box
/// @definition.field symbol=Box.user source="user: User" key=user type=User
/// @definition.method symbol=Box.constructor source="constructor(user: User) { this.user = user; }" slot=constructor role=constructor type=(User) => this

    user: User;
    /// @type.symbol symbol=Box.user source="user: User" type=User
    /// @resolution.name source=User target=User

    constructor(user: User) { this.user = user; }
    /// @type.symbol symbol=Box.constructor source="constructor(user: User) { this.user = user; }" type=(User) => this
    /// @type.symbol symbol=Box.constructor.user source="user: User" type=User
    /// @resolution.name source=User target=User
    /// @resolution.receiver source=this kind=this declaration=Box type=Box
    /// @resolution.pattern.assign source=this.user kind=place place=field(Box.user) type=User
    /// @resolution.name source=user target=Box.constructor.user

}

declare const user: local User;
/// @type.symbol symbol=user source=user type=Placed<User, "local">
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const box: shared Box = new Box(user);
/// @type.symbol symbol=box source=box type=Placed<Box, "shared">
/// @resolution.pattern source=box kind=binding target=box
/// @resolution.name source=Box target=Box
/// @resolution.construct source="new Box(user)" parameters=(Placed<User, "shared">) arguments=(provided(user) as Placed<User, "shared">) return=Placed<Box, "shared"> kind=class target=Box constructor=Box.constructor
/// @resolution.name source=Box target=Box
/// @resolution.name source=user target=user
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'local User' is not assignable to parameter of type 'shared User'"
/// @diagnostic.label line=10 column=33 span="user" line_source="const box: shared Box = new Box(user);"
/// @diagnostic.related line=10 column=25 span="new Box(user)" line_source="const box: shared Box = new Box(user);" message="in this call"
/// @diagnostic.note message="a value never changes its space"
/// @diagnostic.help message="use a value in the destination placement or create a new value there"
"#,
    );
}

#[test]
fn test_satisfy_shared_safe_bound_by_containment() {
    let session = TestSession::single(
        r#"
import { SharedSafe } from "destack:memory";

class Message {}
local class Handle {}

struct CleanEnvelope {
    message: shared Message;
    count: int32;
}

struct LocalEnvelope {
    handle: Handle;
}

declare function publish<T: SharedSafe>(value: T): void;

declare const cleanEnvelope: CleanEnvelope;
declare const localEnvelope: LocalEnvelope;
declare const handle: Handle;

publish<CleanEnvelope>(cleanEnvelope);
publish<LocalEnvelope>(localEnvelope);
publish(cleanEnvelope);
publish(handle);
cleanEnvelope satisfies SharedSafe;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { SharedSafe } from "destack:memory";

class Message {}
local class Handle {}

struct CleanEnvelope {
    message: shared Message;
    count: int32;
}

struct LocalEnvelope {
    handle: Handle;
}

declare function publish<T: SharedSafe>(value: T): void;

declare const cleanEnvelope: CleanEnvelope;
declare const localEnvelope: LocalEnvelope;
declare const handle: Handle;

publish<CleanEnvelope>(cleanEnvelope);
publish<LocalEnvelope>(localEnvelope);
publish<CleanEnvelope>(cleanEnvelope);
publish<Handle>(handle);
cleanEnvelope satisfies SharedSafe;

=== checked ===
import { SharedSafe } from "destack:memory";

class Message {}
/// @type.symbol symbol=Message source="class Message {}" type=Message
/// @definition.class symbol=Message source="class Message {}"

local class Handle {}
/// @type.symbol symbol=Handle source="local class Handle {}" type=Handle
/// @definition.class symbol=Handle source="local class Handle {}"

struct CleanEnvelope {
/// @type.symbol symbol=CleanEnvelope type=CleanEnvelope
/// @definition.struct symbol=CleanEnvelope
/// @definition.field symbol=CleanEnvelope.count source="count: int32" key=count type=int32
/// @definition.field symbol=CleanEnvelope.message source="message: shared Message" key=message type=Placed<Message, "shared">

    message: shared Message;
    /// @type.symbol symbol=CleanEnvelope.message source="message: shared Message" type=Placed<Message, "shared">
    /// @resolution.name source=Message target=Message

    count: int32;
    /// @type.symbol symbol=CleanEnvelope.count source="count: int32" type=int32

}

struct LocalEnvelope {
/// @type.symbol symbol=LocalEnvelope type=LocalEnvelope
/// @definition.struct symbol=LocalEnvelope
/// @definition.field symbol=LocalEnvelope.handle source="handle: Handle" key=handle type=Handle

    handle: Handle;
    /// @type.symbol symbol=LocalEnvelope.handle source="handle: Handle" type=Handle
    /// @resolution.name source=Handle target=Handle

}

declare function publish<T: SharedSafe>(value: T): void;
/// @generic.template symbol=publish parameters=(T: memory.capability.SharedSafe)
/// @type.symbol symbol=publish source="declare function publish<T: SharedSafe>(value: T): void" type=<T: memory.capability.SharedSafe>(T) => void
/// @type.symbol symbol=publish.T source="T: SharedSafe" type=T
/// @resolution.name source=SharedSafe target=memory.capability.SharedSafe
/// @type.symbol symbol=publish.value source="value: T" type=T
/// @resolution.name source=T target=publish.T

declare const cleanEnvelope: CleanEnvelope;
/// @type.symbol symbol=cleanEnvelope source=cleanEnvelope type=CleanEnvelope
/// @resolution.pattern source=cleanEnvelope kind=binding target=cleanEnvelope
/// @resolution.name source=CleanEnvelope target=CleanEnvelope

declare const localEnvelope: LocalEnvelope;
/// @type.symbol symbol=localEnvelope source=localEnvelope type=LocalEnvelope
/// @resolution.pattern source=localEnvelope kind=binding target=localEnvelope
/// @resolution.name source=LocalEnvelope target=LocalEnvelope

declare const handle: Handle;
/// @type.symbol symbol=handle source=handle type=Handle
/// @resolution.pattern source=handle kind=binding target=handle
/// @resolution.name source=Handle target=Handle

publish<CleanEnvelope>(cleanEnvelope);
/// @resolution.name source=publish target=publish
/// @resolution.call source=publish<CleanEnvelope>(cleanEnvelope) parameters=(CleanEnvelope) arguments=(provided(cleanEnvelope) as CleanEnvelope) return=void kind=symbol target=publish instance=publish<CleanEnvelope>
/// @generic.instance source=publish<CleanEnvelope>(cleanEnvelope) id=publish<CleanEnvelope>
/// @resolution.name source=CleanEnvelope target=CleanEnvelope
/// @resolution.name source=cleanEnvelope target=cleanEnvelope

publish<LocalEnvelope>(localEnvelope);
/// @resolution.name source=publish target=publish
/// @resolution.call source=publish<LocalEnvelope>(localEnvelope) parameters=(LocalEnvelope) arguments=(provided(localEnvelope) as LocalEnvelope) return=void kind=symbol target=publish instance=publish<LocalEnvelope>
/// @generic.instance source=publish<LocalEnvelope>(localEnvelope) id=publish<LocalEnvelope>
/// @resolution.name source=LocalEnvelope target=LocalEnvelope
/// @resolution.name source=localEnvelope target=localEnvelope

publish(cleanEnvelope);
/// @resolution.name source=publish target=publish
/// @resolution.call source=publish(cleanEnvelope) parameters=(CleanEnvelope) arguments=(provided(cleanEnvelope) as CleanEnvelope) return=void kind=symbol target=publish instance=publish<CleanEnvelope>
/// @generic.instance source=publish(cleanEnvelope) id=publish<CleanEnvelope>
/// @resolution.name source=cleanEnvelope target=cleanEnvelope

publish(handle);
/// @resolution.name source=publish target=publish
/// @resolution.call source=publish(handle) parameters=(Handle) arguments=(provided(handle) as Handle) return=void kind=symbol target=publish instance=publish<Handle>
/// @generic.instance source=publish(handle) id=publish<Handle>
/// @resolution.name source=handle target=handle

cleanEnvelope satisfies SharedSafe;
/// @resolution.name source=cleanEnvelope target=cleanEnvelope
/// @resolution.name source=SharedSafe target=memory.capability.SharedSafe

/// @generic.instance id=publish<CleanEnvelope> template=publish arguments=(CleanEnvelope)
/// @generic.instance id=publish<Handle> template=publish arguments=(Handle)
/// @generic.instance id=publish<LocalEnvelope> template=publish arguments=(LocalEnvelope)
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'LocalEnvelope' does not satisfy 'SharedSafe'"
/// @diagnostic.label line=23 column=1 span="publish<LocalEnvelope>(localEnvelope)" line_source="publish<LocalEnvelope>(localEnvelope);"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Handle' does not satisfy 'SharedSafe'"
/// @diagnostic.label line=25 column=1 span="publish(handle)" line_source="publish(handle);"
/// @diagnostic.related line=16 column=26 span="T" line_source="declare function publish<T: SharedSafe>(value: T): void;" message="required by this bound on 'T'"
/// @diagnostic.error id=use-after-moved message="'cleanEnvelope' is used after being moved"
/// @diagnostic.label line=24 column=9 span="cleanEnvelope" line_source="publish(cleanEnvelope);"
/// @diagnostic.help message="reassign the binding before this use, or copy instead of moving"
/// @diagnostic.error id=use-after-moved message="'cleanEnvelope' is used after being moved"
/// @diagnostic.label line=26 column=1 span="cleanEnvelope" line_source="cleanEnvelope satisfies SharedSafe;"
/// @diagnostic.help message="reassign the binding before this use, or copy instead of moving"
"#,
    );
}
