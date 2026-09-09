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

    session.assert_dir_and_diagnostics(
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

const cache: Cache = Cache { localUser, sharedUser };

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

struct Cache {
/// @type.symbol symbol=Cache type=Cache
/// @definition.struct symbol=Cache
/// @definition.field symbol=Cache.localUser source="localUser: local User" key=localUser type=local User
/// @definition.field symbol=Cache.sharedUser source="sharedUser: shared User" key=sharedUser type=shared User

    localUser: local User;
    /// @type.symbol symbol=Cache.localUser source="localUser: local User" type=local User
    /// @resolution.name source=User target=User

    sharedUser: shared User;
    /// @type.symbol symbol=Cache.sharedUser source="sharedUser: shared User" type=shared User
    /// @resolution.name source=User target=User

}

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=local User
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=shared User
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

const cache: local Cache = Cache { localUser, sharedUser };
/// @type.symbol symbol=cache source=cache type=Cache
/// @resolution.pattern source=cache kind=binding target=cache
/// @resolution.name source=Cache target=Cache
/// @resolution.name source=Cache target=Cache
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=localUser root=localUser
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=sharedUser root=sharedUser
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

    session.assert_dir_and_diagnostics(
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
const cache: Cache = Cache { user, count: 1 };

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

shared struct Cache {
/// @type.symbol symbol=Cache type=Cache
/// @definition.struct symbol=Cache
/// @definition.field symbol=Cache.count source="count: int32" key=count type=int32
/// @definition.field symbol=Cache.user source="user: shared User" key=user type=shared User

    user: shared User;
    /// @type.symbol symbol=Cache.user source="user: shared User" type=shared User
    /// @resolution.name source=User target=User

    count: int32;
    /// @type.symbol symbol=Cache.count source="count: int32" type=int32

}

declare const user: shared User;
/// @type.symbol symbol=user source=user type=shared User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const cache: shared Cache = Cache { user, count: 1 };
/// @type.symbol symbol=cache source=cache type=Cache
/// @resolution.pattern source=cache kind=binding target=cache
/// @resolution.name source=Cache target=Cache
/// @resolution.name source=Cache target=Cache
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=user root=user
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct BoxedUser {
    user: local User;
}

declare const user: local User;

const field: BoxedUser = BoxedUser { user };
const tuple: (local User, int32) = (user, 1);
const union: local User | undefined = user as local User | undefined;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

struct BoxedUser {
/// @type.symbol symbol=BoxedUser type=BoxedUser
/// @definition.struct symbol=BoxedUser
/// @definition.field symbol=BoxedUser.user source="user: local User" key=user type=local User

    user: local User;
    /// @type.symbol symbol=BoxedUser.user source="user: local User" type=local User
    /// @resolution.name source=User target=User

}

declare const user: local User;
/// @type.symbol symbol=user source=user type=local User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const field: shared BoxedUser = BoxedUser { user };
/// @type.symbol symbol=field source=field type=BoxedUser
/// @resolution.pattern source=field kind=binding target=field
/// @resolution.name source=BoxedUser target=BoxedUser
/// @resolution.name source=BoxedUser target=BoxedUser
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=user root=user

const tuple: shared (local User, int32) = (user, 1);
/// @type.symbol symbol=tuple source=tuple type=(local User, int32)
/// @resolution.pattern source=tuple kind=binding target=tuple
/// @resolution.name source=User target=User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=user root=user

const union: shared (local User | undefined) = user;
/// @type.symbol symbol=union source=union type=local User | undefined
/// @resolution.pattern source=union kind=binding target=union
/// @resolution.name source=User target=User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=user root=user
"#,
        r#"

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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared struct State {
    user: local User;
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

shared struct State {
/// @type.symbol symbol=State type=State
/// @definition.struct symbol=State
/// @definition.field symbol=State.user source="user: local User" key=user type=local User

    user: local User;
    /// @type.symbol symbol=State.user source="user: local User" type=local User
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
    user: User = new User();

    accept(user: local User): void {}
}

declare const service: Service;
service.user satisfies shared User;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared class Service {
    user: User = new User();

    accept(user: local User): void {}
}

declare const service: Service;
service.user satisfies shared User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

shared class Service {
/// @type.symbol symbol=Service type=Service
/// @definition.class symbol=Service
/// @definition.field symbol=Service.user source="user: User = new User()" key=user type=User
/// @definition.method symbol=Service.accept source="accept(user: local User): void {}" slot=accept type=(this: Service, local User) => void

    user: User = new User();
    /// @type.symbol symbol=Service.user source="user: User = new User()" type=User
    /// @resolution.name source=User target=User
    /// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
    /// @resolution.name source=User target=User

    accept(user: local User): void {}
    /// @type.symbol symbol=Service.accept source="accept(user: local User): void {}" type=(this: Service, local User) => void
    /// @type.symbol symbol=Service.accept.this type=Service
    /// @type.symbol symbol=Service.accept.user source="user: local User" type=local User
    /// @resolution.name source=User target=User

}

declare const service: Service;
/// @type.symbol symbol=service source=service type=Service
/// @resolution.pattern source=service kind=binding target=service
/// @resolution.name source=Service target=Service

service.user satisfies shared User;
/// @resolution.name source=service target=service
/// @resolution.member source=service.user receiver=Service type=shared User kind=field target_receiver=Service key=user target=Service.user target_type=shared User
/// @resolution.place source=service placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=service root=service
/// @resolution.place source=service.user placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=service.user root=service keys=[user]
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

    session.assert_dir_and_diagnostics(
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

const rejected: Box<local User> = Box<local User> { value: localUser };
const accepted: Box<shared User> = Box<shared User> { value: sharedUser };

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

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=local User
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=shared User
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

const rejected: shared Box<local User> = Box { value: localUser };
/// @type.symbol symbol=rejected source=rejected type=Box<local User>
/// @resolution.pattern source=rejected kind=binding target=rejected
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User
/// @resolution.name source=Box target=Box
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=localUser root=localUser

const accepted: shared Box<shared User> = Box { value: sharedUser };
/// @type.symbol symbol=accepted source=accepted type=Box<shared User>
/// @resolution.pattern source=accepted kind=binding target=accepted
/// @resolution.name source=Box target=Box
/// @resolution.name source=User target=User
/// @resolution.name source=Box target=Box
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=sharedUser root=sharedUser
"#,
        r#"

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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared struct EscapeHatch {
    pointer: *User;
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

shared struct EscapeHatch {
/// @type.symbol symbol=EscapeHatch type=EscapeHatch
/// @definition.struct symbol=EscapeHatch
/// @definition.field symbol=EscapeHatch.pointer source="pointer: local *User" key=pointer type=Raw<User>

    pointer: local *User;
    /// @type.symbol symbol=EscapeHatch.pointer source="pointer: local *User" type=Raw<User>
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct OwnedBox {
    value: ^User;
}
struct BorrowedBox {
    value: &'static User;
}

declare const owned: ^User;
declare const borrowed: &'static User;

const ownedBox: OwnedBox = OwnedBox { value: owned };
const borrowedBox: BorrowedBox = BorrowedBox { value: borrowed };

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

struct OwnedBox { value: local ^User; }
/// @type.symbol symbol=OwnedBox source="struct OwnedBox { value: local ^User; }" type=OwnedBox
/// @definition.struct symbol=OwnedBox source="struct OwnedBox { value: local ^User; }"
/// @definition.field symbol=OwnedBox.value source="value: local ^User" key=value type=^User
/// @type.symbol symbol=OwnedBox.value source="value: local ^User" type=^User
/// @resolution.name source=User target=User

struct BorrowedBox { value: local Borrowed<User, "static">; }
/// @type.symbol symbol=BorrowedBox source="struct BorrowedBox { value: local Borrowed<User, \"static\">; }" type=BorrowedBox
/// @definition.struct symbol=BorrowedBox source="struct BorrowedBox { value: local Borrowed<User, \"static\">; }"
/// @definition.field symbol=BorrowedBox.value source="value: local Borrowed<User, \"static\">" key=value type=&'static User
/// @type.symbol symbol=BorrowedBox.value source="value: local Borrowed<User, \"static\">" type=&'static User
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=User target=User

declare const owned: local ^User;
/// @type.symbol symbol=owned source=owned type=^User
/// @resolution.pattern source=owned kind=binding target=owned
/// @resolution.name source=User target=User

declare const borrowed: local Borrowed<User, "static">;
/// @type.symbol symbol=borrowed source=borrowed type=&'static User
/// @resolution.pattern source=borrowed kind=binding target=borrowed
/// @resolution.name source=Borrowed target=Borrowed
/// @resolution.name source=User target=User

const ownedBox: shared OwnedBox = OwnedBox { value: owned };
/// @type.symbol symbol=ownedBox source=ownedBox type=OwnedBox
/// @resolution.pattern source=ownedBox kind=binding target=ownedBox
/// @resolution.name source=OwnedBox target=OwnedBox
/// @resolution.name source=OwnedBox target=OwnedBox
/// @resolution.name source=owned target=owned
/// @resolution.place source=owned placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=owned root=owned

const borrowedBox: shared BorrowedBox = BorrowedBox { value: borrowed };
/// @type.symbol symbol=borrowedBox source=borrowedBox type=BorrowedBox
/// @resolution.pattern source=borrowedBox kind=binding target=borrowedBox
/// @resolution.name source=BorrowedBox target=BorrowedBox
/// @resolution.name source=BorrowedBox target=BorrowedBox
/// @resolution.name source=borrowed target=borrowed
/// @resolution.place source=borrowed placement="local" lifetime="static" access="mutable"
/// @resolution.access source=borrowed root=borrowed
"#,
        r#"
/// @diagnostic.error id=placement-on-owned message="an owned value lives in its container's space and takes no placement"
/// @diagnostic.label line=4 column=26 span="local" line_source="struct OwnedBox { value: local ^User; }"
/// @diagnostic.error id=placement-on-owned message="an owned value lives in its container's space and takes no placement"
/// @diagnostic.label line=7 column=22 span="local" line_source="declare const owned: local ^User;"
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

class Box {
    user: User;
    constructor(user: User) {
        this.user = user;
    }
}

declare const user: local User;
const box: shared Box = new Box<"shared">(user);

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

class Box {
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box
/// @definition.field symbol=Box.user source="user: User" key=user type=User
/// @definition.method symbol=Box.constructor source="constructor(user: User) { this.user = user; }" slot=constructor role=constructor type=<Box.constructor.P0: Place>(User) => Managed<this, Box.constructor.P0>

    user: User;
    /// @type.symbol symbol=Box.user source="user: User" type=User
    /// @resolution.name source=User target=User

    constructor(user: User) { this.user = user; }
    /// @generic.template symbol=Box.constructor parameters=(P0: Place)
    /// @type.symbol symbol=Box.constructor source="constructor(user: User) { this.user = user; }" type=<Box.constructor.P0: Place>(User) => Managed<this, Box.constructor.P0>
    /// @type.symbol symbol=Box.constructor.this type=Box
    /// @type.symbol symbol=Box.constructor.user source="user: User" type=User
    /// @resolution.name source=User target=User
    /// @resolution.receiver source=this kind=this declaration=Box type=Box
    /// @resolution.place source=this placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=this root=this
    /// @resolution.pattern.assign source=this.user kind=place
    /// @resolution.access source=this.user root=this keys=[user]
    /// @resolution.assignment source=this.user write="receiver=Box, target=field(receiver=Box, target=Box.user, type=User), type=User" type=User
    /// @resolution.name source=user target=Box.constructor.user
    /// @resolution.place source=user placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=user root=Box.constructor.user

}

declare const user: local User;
/// @type.symbol symbol=user source=user type=local User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

const box: shared Box = new Box(user);
/// @type.symbol symbol=box source=box type=shared Box
/// @resolution.pattern source=box kind=binding target=box
/// @resolution.name source=Box target=Box
/// @resolution.construct source="new Box(user)" parameters=(shared User) arguments=(provided(user) as shared User) return=shared Box kind=class target=Box constructor=Box.constructor
/// @generic.instantiation id="Box.constructor<\"shared\">" template=Box.constructor arguments=("shared")
/// @generic.instantiation id="Box<\"shared\">" template=Box arguments=("shared")
/// @resolution.name source=Box target=Box
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=user root=user
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
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
/// @definition.field symbol=CleanEnvelope.message source="message: shared Message" key=message type=shared Message

    message: shared Message;
    /// @type.symbol symbol=CleanEnvelope.message source="message: shared Message" type=shared Message
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
/// @generic.template symbol=publish parameters=(T: SharedSafe)
/// @type.symbol symbol=publish source="declare function publish<T: SharedSafe>(value: T): void" type=<T: SharedSafe>(T) => void
/// @type.symbol symbol=publish.T source="T: SharedSafe" type=T
/// @resolution.name source=SharedSafe target=SharedSafe
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
/// @generic.instantiation id=publish<CleanEnvelope> template=publish arguments=(CleanEnvelope)
/// @resolution.name source=CleanEnvelope target=CleanEnvelope
/// @resolution.name source=cleanEnvelope target=cleanEnvelope
/// @resolution.place source=cleanEnvelope placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=cleanEnvelope root=cleanEnvelope

publish<LocalEnvelope>(localEnvelope);
/// @resolution.name source=publish target=publish
/// @resolution.call source=publish<LocalEnvelope>(localEnvelope) parameters=(LocalEnvelope) arguments=(provided(localEnvelope) as LocalEnvelope) return=void kind=symbol target=publish instance=publish<LocalEnvelope>
/// @generic.instantiation id=publish<LocalEnvelope> template=publish arguments=(LocalEnvelope)
/// @resolution.name source=LocalEnvelope target=LocalEnvelope
/// @resolution.name source=localEnvelope target=localEnvelope
/// @resolution.place source=localEnvelope placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=localEnvelope root=localEnvelope

publish(cleanEnvelope);
/// @resolution.name source=publish target=publish
/// @resolution.call source=publish(cleanEnvelope) parameters=(CleanEnvelope) arguments=(provided(cleanEnvelope) as CleanEnvelope) return=void kind=symbol target=publish instance=publish<CleanEnvelope>
/// @resolution.name source=cleanEnvelope target=cleanEnvelope
/// @resolution.place source=cleanEnvelope placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=cleanEnvelope root=cleanEnvelope

publish(handle);
/// @resolution.name source=publish target=publish
/// @resolution.call source=publish(handle) parameters=(Handle) arguments=(provided(handle) as Handle) return=void kind=symbol target=publish instance=publish<Handle>
/// @generic.instantiation id=publish<Handle> template=publish arguments=(Handle)
/// @resolution.name source=handle target=handle
/// @resolution.place source=handle placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=handle root=handle

cleanEnvelope satisfies SharedSafe;
/// @resolution.name source=cleanEnvelope target=cleanEnvelope
/// @resolution.place source=cleanEnvelope placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=cleanEnvelope root=cleanEnvelope
/// @resolution.name source=SharedSafe target=SharedSafe
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'LocalEnvelope' does not satisfy 'SharedSafe'"
/// @diagnostic.label line=23 column=1 span="publish<LocalEnvelope>(localEnvelope)" line_source="publish<LocalEnvelope>(localEnvelope);"
/// @diagnostic.related line=16 column=26 span="T" line_source="declare function publish<T: SharedSafe>(value: T): void;" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Handle' does not satisfy 'SharedSafe'"
/// @diagnostic.label line=25 column=1 span="publish(handle)" line_source="publish(handle);"
/// @diagnostic.related line=16 column=26 span="T" line_source="declare function publish<T: SharedSafe>(value: T): void;" message="required by this bound on 'T'"
"#,
    );
}

#[test]
fn test_accept_unsafe_shared_safe_implementation() {
    let session = TestSession::single(
        r#"
import { SharedSafe } from "destack:memory";

local class Handle {}

@unsafe
extension of Handle implements SharedSafe {}

declare const handle: Handle;
handle satisfies SharedSafe;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_definitions().with_decorators(),
        r#"
=== annotated ===
import { SharedSafe } from "destack:memory";

local class Handle {}

@unsafe
extension of Handle implements SharedSafe {}

declare const handle: Handle;
handle satisfies SharedSafe;

=== dir ===
import { SharedSafe } from "destack:memory";

local class Handle {}
/// @type.symbol symbol=Handle source="local class Handle {}" type=Handle
/// @definition.class symbol=Handle source="local class Handle {}"

@unsafe
/// @decorator.node source=@unsafe owner="extension of Handle implements SharedSafe {}" expression=unsafe target=decorator.unsafe type=unsafe kind=newtype parameters=() newtype=unsafe backing=() value=unsafe()
/// @resolution.name source=unsafe target=unsafe

extension of Handle implements SharedSafe {}
/// @definition.extension symbol=<module>#2 source="extension of Handle implements SharedSafe {}" form=local target=Handle
/// @definition.implements symbol=<module>#2 source=SharedSafe target=SharedSafe
/// @resolution.name source=Handle target=Handle
/// @resolution.name source=SharedSafe target=SharedSafe

declare const handle: Handle;
/// @type.symbol symbol=handle source=handle type=Handle
/// @resolution.pattern source=handle kind=binding target=handle
/// @resolution.name source=Handle target=Handle

handle satisfies SharedSafe;
/// @resolution.name source=handle target=handle
/// @resolution.place source=handle placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=handle root=handle
/// @resolution.name source=SharedSafe target=SharedSafe
"#,
    );
}
