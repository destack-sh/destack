use crate::tests::{DirRows, TestSession};

#[test]
fn test_store_local_and_shared_references_in_local_aggregate() {
    let session = TestSession::single(
        r#"
class User {}

shared class SharedUser {}

struct Cache {
    localUser: User;
    sharedUser: SharedUser;
}

declare const localUser: User;
declare const sharedUser: SharedUser;

const cache: Cache = Cache { localUser, sharedUser };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared class SharedUser {}

struct Cache {
    localUser: User;
    sharedUser: SharedUser;
}

declare const localUser: User;
declare const sharedUser: SharedUser;

const cache: Cache = Cache { localUser, sharedUser };

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

shared class SharedUser {}
/// @type.symbol symbol=SharedUser source="shared class SharedUser {}" type=typeof SharedUser
/// @definition.class symbol=SharedUser source="shared class SharedUser {}"

struct Cache {
/// @type.symbol symbol=Cache type=Cache
/// @definition.struct symbol=Cache
/// @definition.field symbol=Cache.localUser source="localUser: User" key=localUser type=User
/// @definition.field symbol=Cache.sharedUser source="sharedUser: SharedUser" key=sharedUser type=SharedUser

    localUser: User;
    /// @type.symbol symbol=Cache.localUser source="localUser: User" type=User
    /// @resolution.name source=User target=User

    sharedUser: SharedUser;
    /// @type.symbol symbol=Cache.sharedUser source="sharedUser: SharedUser" type=SharedUser
    /// @resolution.name source=SharedUser target=SharedUser

}

declare const localUser: User;
/// @type.symbol symbol=localUser source=localUser type=User
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: SharedUser;
/// @type.symbol symbol=sharedUser source=sharedUser type=SharedUser
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=SharedUser target=SharedUser

const cache: Cache = Cache { localUser, sharedUser };
/// @type.symbol symbol=cache source=cache type=Cache
/// @resolution.pattern source=cache kind=binding target=cache
/// @resolution.name source=Cache target=Cache
/// @resolution.name source=Cache target=Cache
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="static" access="immutable"
/// @resolution.access source=localUser root=localUser
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="immutable"
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
shared class User {}

struct Cache {
    user: User;
    count: int32;
}

declare const user: User;
shared const cache: Cache = Cache { user, count: 1 };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
shared class User {}

struct Cache {
    user: User;
    count: int32;
}

declare const user: User;
shared const cache: Cache = Cache { user, count: 1 };

=== dir ===
shared class User {}
/// @type.symbol symbol=User source="shared class User {}" type=typeof User
/// @definition.class symbol=User source="shared class User {}"

struct Cache {
/// @type.symbol symbol=Cache type=Cache
/// @definition.struct symbol=Cache
/// @definition.field symbol=Cache.count source="count: int32" key=count type=int32
/// @definition.field symbol=Cache.user source="user: User" key=user type=User

    user: User;
    /// @type.symbol symbol=Cache.user source="user: User" type=User
    /// @resolution.name source=User target=User

    count: int32;
    /// @type.symbol symbol=Cache.count source="count: int32" type=int32

}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

shared const cache: Cache = Cache { user, count: 1 };
/// @type.symbol symbol=cache source=cache type=Cache
/// @resolution.pattern source=cache kind=binding target=cache
/// @resolution.name source=Cache target=Cache
/// @resolution.name source=Cache target=Cache
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="shared" lifetime="static" access="immutable"
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
    user: User;
}

declare const user: User;

shared const field: BoxedUser = BoxedUser { user };
shared const tuple: (User, int32) = (user, 1);
shared const union: User | undefined = user;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct BoxedUser {
    user: User;
}

declare const user: User;

shared const field: BoxedUser = BoxedUser { user };
shared const tuple: (User, int32) = (user, 1);
shared const union: User | undefined = user as User | undefined;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

struct BoxedUser {
/// @type.symbol symbol=BoxedUser type=BoxedUser
/// @definition.struct symbol=BoxedUser
/// @definition.field symbol=BoxedUser.user source="user: User" key=user type=User

    user: User;
    /// @type.symbol symbol=BoxedUser.user source="user: User" type=User
    /// @resolution.name source=User target=User

}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

shared const field: BoxedUser = BoxedUser { user };
/// @type.symbol symbol=field source=field type=BoxedUser
/// @resolution.pattern source=field kind=binding target=field
/// @resolution.name source=BoxedUser target=BoxedUser
/// @resolution.name source=BoxedUser target=BoxedUser
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user

shared const tuple: (User, int32) = (user, 1);
/// @type.symbol symbol=tuple source=tuple type=(User, int32)
/// @resolution.pattern source=tuple kind=binding target=tuple
/// @resolution.name source=User target=User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user

shared const union: User | undefined = user;
/// @type.symbol symbol=union source=union type=User | undefined
/// @resolution.pattern source=union kind=binding target=union
/// @resolution.name source=User target=User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user
"#,
        r#"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=10 column=14 span="field" line_source="shared const field: BoxedUser = BoxedUser { user };"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=11 column=14 span="tuple" line_source="shared const tuple: (User, int32) = (user, 1);"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=12 column=14 span="union" line_source="shared const union: User | undefined = user;"
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
    user: User;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared struct State {
    user: User;
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

shared struct State {
/// @type.symbol symbol=State type=State
/// @definition.struct symbol=State
/// @definition.field symbol=State.user source="user: User" key=user type=User

    user: User;
    /// @type.symbol symbol=State.user source="user: User" type=User
    /// @resolution.name source=User target=User

}
"#,
        r#"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=5 column=5 span="user" line_source="user: User;"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
"#,
    );
}

#[test]
fn test_resolve_relative_fields_in_shared_declarations() {
    let session = TestSession::single(
        r#"
shared class User {}

shared class Service {
    user: User = new User();

    accept(user: User): void {}
}

declare const service: Service;
service.user satisfies User;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
shared class User {}

shared class Service {
    user: User = new User();

    accept(user: User): void {}
}

declare const service: Service;
service.user satisfies User;

=== dir ===
shared class User {}
/// @type.symbol symbol=User source="shared class User {}" type=typeof User
/// @definition.class symbol=User source="shared class User {}"

shared class Service {
/// @type.symbol symbol=Service type=typeof Service
/// @definition.class symbol=Service
/// @definition.field symbol=Service.user source="user: User = new User()" key=user type=User
/// @definition.method symbol=Service.accept source="accept(user: User): void {}" slot=accept type=(this: Service, User) => void

    user: User = new User();
    /// @type.symbol symbol=Service.user source="user: User = new User()" type=User
    /// @resolution.name source=User target=User
    /// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
    /// @resolution.name source=User target=User

    accept(user: User): void {}
    /// @type.symbol symbol=Service.accept source="accept(user: User): void {}" type=(this: Service, User) => void
    /// @type.symbol symbol=Service.accept.this type=Service
    /// @type.symbol symbol=Service.accept.user source="user: User" type=User
    /// @resolution.name source=User target=User

}

declare const service: Service;
/// @type.symbol symbol=service source=service type=Service
/// @resolution.pattern source=service kind=binding target=service
/// @resolution.name source=Service target=Service

service.user satisfies User;
/// @resolution.name source=service target=service
/// @resolution.member source=service.user receiver=Service type=User kind=field target_receiver=Service key=user target=Service.user target_type=User
/// @resolution.place source=service placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=service root=service
/// @resolution.place source=service.user placement="shared" lifetime="managed" access="readonly"
/// @resolution.access source=service.user root=service keys=[user]
/// @resolution.name source=User target=User
"#,
    );
}

#[test]
fn test_allow_raw_local_pointer_in_shared_declaration() {
    let session = TestSession::single(
        r#"
class User {}

shared struct EscapeHatch {
    pointer: *User;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared struct EscapeHatch {
    pointer: *User;
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

shared struct EscapeHatch {
/// @type.symbol symbol=EscapeHatch type=EscapeHatch
/// @definition.struct symbol=EscapeHatch
/// @definition.field symbol=EscapeHatch.pointer source="pointer: *User" key=pointer type=*User

    pointer: *User;
    /// @type.symbol symbol=EscapeHatch.pointer source="pointer: *User" type=*User
    /// @resolution.name source=User target=User

}
"#,
        r#"
"#,
    );
}

/// Reject local borrowed references stored in shared space.
#[test]
fn test_reject_local_borrowed_references_in_shared_space() {
    let session = TestSession::single(
        r#"
class User {}

struct BorrowedBox<'a> { value: &'a readonly User; }

declare const borrowed: &'static readonly User;

shared const borrowedBox: BorrowedBox<'static> = BorrowedBox { value: borrowed };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct BorrowedBox<'a> {
    value: &'a readonly User;
}

declare const borrowed: &'static readonly User;

shared const borrowedBox: BorrowedBox<"static"> = BorrowedBox<"static"> { value: borrowed };

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

struct BorrowedBox<'a> { value: &'a readonly User; }
/// @generic.template symbol=BorrowedBox parameters=('a)
/// @type.symbol symbol=BorrowedBox source="struct BorrowedBox<'a> { value: &'a readonly User; }" type=BorrowedBox
/// @definition.struct symbol=BorrowedBox source="struct BorrowedBox<'a> { value: &'a readonly User; }" template=('a)
/// @definition.field symbol=BorrowedBox.value source="value: &'a readonly User" key=value type=&'a readonly User
/// @type.symbol symbol=BorrowedBox.'a source='a type='a
/// @type.symbol symbol=BorrowedBox.value source="value: &'a readonly User" type=&'a readonly User
/// @resolution.name source='a target=BorrowedBox.'a
/// @resolution.name source=User target=User

declare const borrowed: &'static readonly User;
/// @type.symbol symbol=borrowed source=borrowed type=&'static readonly User
/// @resolution.pattern source=borrowed kind=binding target=borrowed
/// @resolution.name source=User target=User

shared const borrowedBox: BorrowedBox<'static> = BorrowedBox { value: borrowed };
/// @type.symbol symbol=borrowedBox source=borrowedBox type=BorrowedBox<"static">
/// @resolution.pattern source=borrowedBox kind=binding target=borrowedBox
/// @resolution.name source=BorrowedBox target=BorrowedBox
/// @resolution.name source=BorrowedBox target=BorrowedBox
/// @resolution.name source=borrowed target=borrowed
/// @resolution.place source=borrowed placement="local" lifetime="static" access="immutable"
/// @resolution.access source=borrowed root=borrowed
"#,
        r#"
/// @diagnostic.error id=local-reference-in-shared-storage message="shared space cannot hold references into local space"
/// @diagnostic.label line=8 column=14 span="borrowedBox" line_source="shared const borrowedBox: BorrowedBox<'static> = BorrowedBox { value: borrowed };"
/// @diagnostic.note message="managed, owned, and borrowed references retain their referent"
/// @diagnostic.help message="place the referenced value in shared space or keep the destination local"
"#,
    );
}

#[test]
fn test_satisfy_shared_safe_bound_by_containment() {
    let session = TestSession::single(
        r#"
import { SharedSafe } from "tspp:memory";

class Message {}

shared class SharedMessage {}
class Handle {}

struct CleanEnvelope {
    message: SharedMessage;
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { SharedSafe } from "tspp:memory";

class Message {}

shared class SharedMessage {}
class Handle {}

struct CleanEnvelope {
    message: SharedMessage;
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
import { SharedSafe } from "tspp:memory";

class Message {}
/// @type.symbol symbol=Message source="class Message {}" type=typeof Message
/// @definition.class symbol=Message source="class Message {}"

shared class SharedMessage {}
/// @type.symbol symbol=SharedMessage source="shared class SharedMessage {}" type=typeof SharedMessage
/// @definition.class symbol=SharedMessage source="shared class SharedMessage {}"

class Handle {}
/// @type.symbol symbol=Handle source="class Handle {}" type=typeof Handle
/// @definition.class symbol=Handle source="class Handle {}"

struct CleanEnvelope {
/// @type.symbol symbol=CleanEnvelope type=CleanEnvelope
/// @definition.struct symbol=CleanEnvelope
/// @definition.field symbol=CleanEnvelope.count source="count: int32" key=count type=int32
/// @definition.field symbol=CleanEnvelope.message source="message: SharedMessage" key=message type=SharedMessage

    message: SharedMessage;
    /// @type.symbol symbol=CleanEnvelope.message source="message: SharedMessage" type=SharedMessage
    /// @resolution.name source=SharedMessage target=SharedMessage

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
/// @resolution.place source=cleanEnvelope placement="local" lifetime="static" access="immutable"
/// @resolution.access source=cleanEnvelope root=cleanEnvelope

publish<LocalEnvelope>(localEnvelope);
/// @resolution.name source=publish target=publish
/// @resolution.call source=publish<LocalEnvelope>(localEnvelope) parameters=(LocalEnvelope) arguments=(provided(localEnvelope) as LocalEnvelope) return=void kind=symbol target=publish instance=publish<LocalEnvelope>
/// @generic.instantiation id=publish<LocalEnvelope> template=publish arguments=(LocalEnvelope)
/// @resolution.name source=LocalEnvelope target=LocalEnvelope
/// @resolution.name source=localEnvelope target=localEnvelope
/// @resolution.place source=localEnvelope placement="local" lifetime="static" access="immutable"
/// @resolution.access source=localEnvelope root=localEnvelope

publish(cleanEnvelope);
/// @resolution.name source=publish target=publish
/// @resolution.call source=publish(cleanEnvelope) parameters=(CleanEnvelope) arguments=(provided(cleanEnvelope) as CleanEnvelope) return=void kind=symbol target=publish instance=publish<CleanEnvelope>
/// @resolution.name source=cleanEnvelope target=cleanEnvelope
/// @resolution.place source=cleanEnvelope placement="local" lifetime="static" access="immutable"
/// @resolution.access source=cleanEnvelope root=cleanEnvelope

publish(handle);
/// @resolution.name source=publish target=publish
/// @resolution.call source=publish(handle) parameters=(Handle) arguments=(provided(handle) as Handle) return=void kind=symbol target=publish instance=publish<Handle>
/// @generic.instantiation id=publish<Handle> template=publish arguments=(Handle)
/// @resolution.name source=handle target=handle
/// @resolution.place source=handle placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handle root=handle

cleanEnvelope satisfies SharedSafe;
/// @resolution.name source=cleanEnvelope target=cleanEnvelope
/// @resolution.place source=cleanEnvelope placement="local" lifetime="static" access="immutable"
/// @resolution.access source=cleanEnvelope root=cleanEnvelope
/// @resolution.name source=SharedSafe target=SharedSafe
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'LocalEnvelope' does not satisfy 'SharedSafe'"
/// @diagnostic.label line=25 column=1 span="publish<LocalEnvelope>(localEnvelope)" line_source="publish<LocalEnvelope>(localEnvelope);"
/// @diagnostic.related line=18 column=26 span="T" line_source="declare function publish<T: SharedSafe>(value: T): void;" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Handle' does not satisfy 'SharedSafe'"
/// @diagnostic.label line=27 column=1 span="publish(handle)" line_source="publish(handle);"
/// @diagnostic.related line=18 column=26 span="T" line_source="declare function publish<T: SharedSafe>(value: T): void;" message="required by this bound on 'T'"
"#,
    );
}

#[test]
fn test_accept_unsafe_shared_safe_implementation() {
    let session = TestSession::single(
        r#"
import { SharedSafe } from "tspp:memory";

class Handle {}

@unsafe
extension of Handle implements SharedSafe {}

declare const handle: Handle;
handle satisfies SharedSafe;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_definitions().with_decorators(),
        r#"
=== annotated ===
import { SharedSafe } from "tspp:memory";

class Handle {}

@unsafe
extension of Handle implements SharedSafe {}

declare const handle: Handle;
handle satisfies SharedSafe;

=== dir ===
import { SharedSafe } from "tspp:memory";

class Handle {}
/// @type.symbol symbol=Handle source="class Handle {}" type=typeof Handle
/// @definition.class symbol=Handle source="class Handle {}"

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
/// @resolution.place source=handle placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handle root=handle
/// @resolution.name source=SharedSafe target=SharedSafe
"#,
    );
}

/// Fields read through a frame value, an owned value, a borrow, and a handle each name their place.
#[test]
fn test_read_fields_through_frame_owned_borrowed_and_handle_receivers() {
    let session = TestSession::single(
        r#"
struct Point { x: int32 }
class User { age: int32 = 0 }

function probe(borrowed: &Point, handle: User, exclusive: &exclusive int32[]): int32 {
    let local = Point { x: 1 };
    let owned: ^Point = Point { x: 2 };
    const fresh = new User();
    return local.x + owned.x + borrowed.x + handle.age + fresh.age + exclusive[0];
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
}
class User {
    age: int32 = 0;
}

function probe<'a, 'b>(borrowed: &'a Point, handle: User, exclusive: &'b exclusive int32[]): int32 {
    let local: Point = Point { x: 1 };
    let owned: Point = Point { x: 2 };
    const fresh: User = new User();
    return local.x + owned.x + borrowed.x + handle.age + fresh.age + exclusive[0];
}

=== dir ===
struct Point { x: int32 }
/// @type.symbol symbol=Point source="struct Point { x: int32 }" type=Point
/// @definition.struct symbol=Point source="struct Point { x: int32 }"
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @type.symbol symbol=Point.x source="x: int32" type=int32

class User { age: int32 = 0 }
/// @type.symbol symbol=User source="class User { age: int32 = 0 }" type=typeof User
/// @definition.class symbol=User source="class User { age: int32 = 0 }"
/// @definition.field symbol=User.age source="age: int32 = 0" key=age type=int32
/// @type.symbol symbol=User.age source="age: int32 = 0" type=int32

function probe(borrowed: &Point, handle: User, exclusive: &exclusive int32[]): int32 {
/// @generic.template symbol=probe parameters=('a, 'b)
/// @type.symbol symbol=probe type=<probe.'a, probe.'b>(&probe.'a Point, User, &probe.'b exclusive int32[]) => int32
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @type.symbol symbol=probe.borrowed source="borrowed: &Point" type=&probe.'a Point
/// @resolution.name source=Point target=Point
/// @type.symbol symbol=probe.handle source="handle: User" type=User
/// @resolution.name source=User target=User
/// @type.symbol symbol=probe.exclusive source="exclusive: &exclusive int32[]" type=&probe.'b exclusive int32[]

    let local = Point { x: 1 };
    /// @type.symbol symbol=probe.local source=local type=Point
    /// @resolution.pattern source=local kind=binding target=probe.local
    /// @resolution.name source=Point target=Point

    let owned: ^Point = Point { x: 2 };
    /// @type.symbol symbol=probe.owned source=owned type=Point
    /// @resolution.pattern source=owned kind=binding target=probe.owned
    /// @resolution.name source=Point target=Point
    /// @resolution.name source=Point target=Point

    const fresh = new User();
    /// @type.symbol symbol=probe.fresh source=fresh type=User
    /// @resolution.pattern source=fresh kind=binding target=probe.fresh
    /// @resolution.construct source="new User()" parameters=() return=User kind=class target=User constructor=default
    /// @resolution.name source=User target=User

    return local.x + owned.x + borrowed.x + handle.age + fresh.age + exclusive[0];
    /// @resolution.name source=local target=probe.local
    /// @resolution.member source=local.x receiver=Point type=int32 kind=field target_receiver=Point key=x target=Point.x target_type=int32
    /// @resolution.operator source="local.x + owned.x + borrowed.x + handle.age + fresh.age + exclusive[0]" type=int32 operator="+" kind=builtin operands=[local.x + owned.x + borrowed.x + handle.age + fresh.age as int32 families=(integer), exclusive[0] as int32 families=(integer)]
    /// @resolution.operator source="local.x + owned.x + borrowed.x + handle.age + fresh.age" type=int32 operator="+" kind=builtin operands=[local.x + owned.x + borrowed.x + handle.age as int32 families=(integer), fresh.age as int32 families=(integer)]
    /// @resolution.operator source="local.x + owned.x + borrowed.x + handle.age" type=int32 operator="+" kind=builtin operands=[local.x + owned.x + borrowed.x as int32 families=(integer), handle.age as int32 families=(integer)]
    /// @resolution.operator source="local.x + owned.x + borrowed.x" type=int32 operator="+" kind=builtin operands=[local.x + owned.x as int32 families=(integer), borrowed.x as int32 families=(integer)]
    /// @resolution.operator source="local.x + owned.x" type=int32 operator="+" kind=builtin operands=[local.x as int32 families=(integer), owned.x as int32 families=(integer)]
    /// @resolution.place source=local placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=local root=probe.local
    /// @resolution.place source=local.x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=local.x root=probe.local keys=[x]
    /// @resolution.name source=owned target=probe.owned
    /// @resolution.member source=owned.x receiver=Point type=int32 kind=field target_receiver=Point key=x target=Point.x target_type=int32
    /// @resolution.place source=owned placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=owned root=probe.owned
    /// @resolution.place source=owned.x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=owned.x root=probe.owned keys=[x]
    /// @resolution.name source=borrowed target=probe.borrowed
    /// @resolution.member source=borrowed.x receiver=&probe.'a Point type=int32 kind=field target_receiver=&probe.'a Point key=x target=Point.x target_type=int32
    /// @resolution.place source=borrowed placement=probe.'a lifetime=probe.'a access="mutable"
    /// @resolution.access source=borrowed root=probe.borrowed
    /// @resolution.place source=borrowed.x placement=probe.'a lifetime=probe.'a access="mutable"
    /// @resolution.access source=borrowed.x root=probe.borrowed keys=[x]
    /// @resolution.name source=handle target=probe.handle
    /// @resolution.member source=handle.age receiver=User type=int32 kind=field target_receiver=User key=age target=User.age target_type=int32
    /// @resolution.place source=handle placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=handle root=probe.handle
    /// @resolution.place source=handle.age placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=handle.age root=probe.handle keys=[age]
    /// @resolution.name source=fresh target=probe.fresh
    /// @resolution.member source=fresh.age receiver=User type=int32 kind=field target_receiver=User key=age target=User.age target_type=int32
    /// @resolution.place source=fresh placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=fresh root=probe.fresh
    /// @resolution.place source=fresh.age placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=fresh.age root=probe.fresh keys=[age]
    /// @resolution.name source=exclusive target=probe.exclusive
    /// @resolution.place source=exclusive placement=probe.'b lifetime=probe.'b access="exclusive"
    /// @resolution.access source=exclusive root=probe.exclusive
    /// @resolution.subscript source=exclusive[0] type=int32 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=int32, regions=(probe.'b))"
    /// @generic.instantiation id="index#2<int32, probe.'b>" template=index#2 arguments=(int32, probe.'b)
    /// @generic.instance id="index#2<int32, probe.'b>" template=index#2 arguments=(int32, probe.'b)

}
"#,
    );
}

/// Store a readonly handle from the instance place into a local record.
#[test]
fn test_store_a_readonly_handle_from_the_instance_place_into_a_local_record() {
    let session = TestSession::single(
        r#"
type Fields = { readonly [key: string]: int32 };
type Options = { fields?: Fields | undefined };

class Logger {
    write(&readonly this, options?: Options): void {}

    info(&readonly this, fields?: Fields): void {
        this.write({ fields });
    }
}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
type Fields = { readonly [key: string]: int32 };
type Options = { fields?: Fields | undefined };

class Logger {
    write(&readonly this, options?: { fields?: Fields | undefined }): void {}

    info(&readonly this, fields?: Fields): void {
        this.write<'a>({ fields } as { fields?: Fields | undefined } | undefined);
    }
}

=== dir ===
type Fields = { readonly [key: string]: int32 };
/// @type.symbol symbol=Fields source="type Fields = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Fields source="type Fields = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

type Options = { fields?: Fields | undefined };
/// @type.symbol symbol=Options source="type Options = { fields?: Fields | undefined }" type={ fields?: { readonly [key: string]: int32 } | undefined }
/// @definition.type symbol=Options source="type Options = { fields?: Fields | undefined }" value={ fields?: Fields | undefined }
/// @type.symbol symbol=Options.fields source="fields?: Fields | undefined" type={ readonly [key: string]: int32 } | undefined
/// @resolution.name source=Fields target=Fields

class Logger {
/// @type.symbol symbol=Logger type=typeof Logger
/// @definition.class symbol=Logger
/// @definition.method symbol=Logger.info slot=info type=<Logger.info.'a>(this: &Logger.info.'a readonly Logger, { readonly [key: string]: int32 } | undefined?) => void
/// @definition.method symbol=Logger.write source="write(&readonly this, options?: Options): void {}" slot=write type=<Logger.write.'a>(this: &Logger.write.'a readonly Logger, { fields?: Fields | undefined } | undefined?) => void

    write(&readonly this, options?: Options): void {}
    /// @generic.template symbol=Logger.write parameters=('a)
    /// @type.symbol symbol=Logger.write source="write(&readonly this, options?: Options): void {}" type=<Logger.write.'a>(this: &Logger.write.'a readonly Logger, { fields?: Fields | undefined } | undefined?) => void
    /// @type.symbol symbol=Logger.write.this source="&readonly this" type=&Logger.write.'a readonly Logger
    /// @type.symbol symbol=Logger.write.options source="options?: Options" type={ fields?: Fields | undefined } | undefined
    /// @resolution.name source=Options target=Options

    info(&readonly this, fields?: Fields): void {
    /// @generic.template symbol=Logger.info parameters=('a)
    /// @type.symbol symbol=Logger.info type=<Logger.info.'a>(this: &Logger.info.'a readonly Logger, { readonly [key: string]: int32 } | undefined?) => void
    /// @type.symbol symbol=Logger.info.this source="&readonly this" type=&Logger.info.'a readonly Logger
    /// @type.symbol symbol=Logger.info.fields source="fields?: Fields" type={ readonly [key: string]: int32 } | undefined
    /// @resolution.name source=Fields target=Fields

        this.write({ fields });
        /// @resolution.member source=this.write receiver=&Logger.info.'a readonly Logger type=<Logger.write.'a>(this: &Logger.write.'a readonly Logger, { fields?: Fields | undefined } | undefined?) => void kind=symbol target_receiver=&Logger.info.'a readonly Logger target=Logger.write
        /// @resolution.call source="this.write({ fields })" parameters=({ fields?: Fields | undefined } | undefined) arguments=(provided({ fields }) as { fields?: Fields | undefined } | undefined) return=void regions=(Logger.info.'a) kind=symbol target=Logger.write receiver=&Logger.info.'a readonly Logger instance=Logger.write<Logger.info.'a>
        /// @resolution.receiver source=this kind=this declaration=Logger type=&Logger.info.'a readonly Logger
        /// @resolution.place source=this placement=Logger.info.'a lifetime=Logger.info.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Logger.write<Logger.info.'a> template=Logger.write arguments=(Logger.info.'a)
        /// @generic.instance id=Logger.write<Logger.info.'a> template=Logger.write arguments=(Logger.info.'a)
        /// @resolution.name source=fields target=Logger.info.fields
        /// @resolution.place source=fields placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=fields root=Logger.info.fields

    }
}
"#);
}

/// Store a readonly handle from the instance place into a structural record.
#[test]
fn test_store_a_readonly_handle_from_the_instance_place_into_a_structural_record() {
    let session = TestSession::single(
        r#"
class Logger {
    write(&readonly this, options?: { fields?: { readonly [key: string]: int32 } | undefined }): void {}

    info(&readonly this, fields?: { readonly [key: string]: int32 }): void {
        this.write({ fields });
    }
}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
class Logger {
    write(
        &readonly this,
        options?: { fields?: { readonly [key: string]: int32 } | undefined },
    ): void {}

    info(&readonly this, fields?: { readonly [key: string]: int32 }): void {
        this.write<'a>({ fields });
    }
}

=== dir ===
class Logger {
/// @type.symbol symbol=Logger type=typeof Logger
/// @definition.class symbol=Logger
/// @definition.method symbol=Logger.info slot=info type=<Logger.info.'a>(this: &Logger.info.'a readonly Logger, { readonly [key: string]: int32 } | undefined?) => void
/// @definition.method symbol=Logger.write slot=write type=<Logger.write.'a>(this: &Logger.write.'a readonly Logger, { fields?: { readonly [key: string]: int32 } | undefined } | undefined?) => void

    write(&readonly this, options?: { fields?: { readonly [key: string]: int32 } | undefined }): void {}
    /// @generic.template symbol=Logger.write parameters=('a)
    /// @type.symbol symbol=Logger.write type=<Logger.write.'a>(this: &Logger.write.'a readonly Logger, { fields?: { readonly [key: string]: int32 } | undefined } | undefined?) => void
    /// @type.symbol symbol=Logger.write.this source="&readonly this" type=&Logger.write.'a readonly Logger
    /// @type.symbol symbol=Logger.write.options source="options?: { fields?: { readonly [key: string]: int32 } | undefined }" type={ fields?: { readonly [key: string]: int32 } | undefined } | undefined
    /// @type.symbol symbol=Logger.write.fields source="fields?: { readonly [key: string]: int32 } | undefined" type={ readonly [key: string]: int32 } | undefined

    info(&readonly this, fields?: { readonly [key: string]: int32 }): void {
    /// @generic.template symbol=Logger.info parameters=('a)
    /// @type.symbol symbol=Logger.info type=<Logger.info.'a>(this: &Logger.info.'a readonly Logger, { readonly [key: string]: int32 } | undefined?) => void
    /// @type.symbol symbol=Logger.info.this source="&readonly this" type=&Logger.info.'a readonly Logger
    /// @type.symbol symbol=Logger.info.fields source="fields?: { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 } | undefined

        this.write({ fields });
        /// @resolution.member source=this.write receiver=&Logger.info.'a readonly Logger type=<Logger.write.'a>(this: &Logger.write.'a readonly Logger, { fields?: { readonly [key: string]: int32 } | undefined } | undefined?) => void kind=symbol target_receiver=&Logger.info.'a readonly Logger target=Logger.write
        /// @resolution.call source="this.write({ fields })" parameters=({ fields?: { readonly [key: string]: int32 } | undefined } | undefined) arguments=(provided({ fields }) as { fields?: { readonly [key: string]: int32 } | undefined } | undefined) return=void regions=(Logger.info.'a) kind=symbol target=Logger.write receiver=&Logger.info.'a readonly Logger instance=Logger.write<Logger.info.'a>
        /// @resolution.receiver source=this kind=this declaration=Logger type=&Logger.info.'a readonly Logger
        /// @resolution.place source=this placement=Logger.info.'a lifetime=Logger.info.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Logger.write<Logger.info.'a> template=Logger.write arguments=(Logger.info.'a)
        /// @generic.instance id=Logger.write<Logger.info.'a> template=Logger.write arguments=(Logger.info.'a)
        /// @resolution.name source=fields target=Logger.info.fields
        /// @resolution.place source=fields placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=fields root=Logger.info.fields

    }
}
"#);
}

/// Pass a readonly handle to a bare class parameter.
#[test]
fn test_pass_a_readonly_handle_to_a_bare_parameter() {
    let session = TestSession::single(
        r#"
class Logger {
    write(&readonly this, fields: { readonly [key: string]: int32 }): void {}

    info(&readonly this, fields: { readonly [key: string]: int32 }): void {
        this.write(fields);
    }
}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
class Logger {
    write(&readonly this, fields: { readonly [key: string]: int32 }): void {}

    info(&readonly this, fields: { readonly [key: string]: int32 }): void {
        this.write<'a>(fields);
    }
}

=== dir ===
class Logger {
/// @type.symbol symbol=Logger type=typeof Logger
/// @definition.class symbol=Logger
/// @definition.method symbol=Logger.info slot=info type=<Logger.info.'a>(this: &Logger.info.'a readonly Logger, { readonly [key: string]: int32 }) => void
/// @definition.method symbol=Logger.write source="write(&readonly this, fields: { readonly [key: string]: int32 }): void {}" slot=write type=<Logger.write.'a>(this: &Logger.write.'a readonly Logger, { readonly [key: string]: int32 }) => void

    write(&readonly this, fields: { readonly [key: string]: int32 }): void {}
    /// @generic.template symbol=Logger.write parameters=('a)
    /// @type.symbol symbol=Logger.write source="write(&readonly this, fields: { readonly [key: string]: int32 }): void {}" type=<Logger.write.'a>(this: &Logger.write.'a readonly Logger, { readonly [key: string]: int32 }) => void
    /// @type.symbol symbol=Logger.write.this source="&readonly this" type=&Logger.write.'a readonly Logger
    /// @type.symbol symbol=Logger.write.fields source="fields: { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }

    info(&readonly this, fields: { readonly [key: string]: int32 }): void {
    /// @generic.template symbol=Logger.info parameters=('a)
    /// @type.symbol symbol=Logger.info type=<Logger.info.'a>(this: &Logger.info.'a readonly Logger, { readonly [key: string]: int32 }) => void
    /// @type.symbol symbol=Logger.info.this source="&readonly this" type=&Logger.info.'a readonly Logger
    /// @type.symbol symbol=Logger.info.fields source="fields: { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }

        this.write(fields);
        /// @resolution.member source=this.write receiver=&Logger.info.'a readonly Logger type=<Logger.write.'a>(this: &Logger.write.'a readonly Logger, { readonly [key: string]: int32 }) => void kind=symbol target_receiver=&Logger.info.'a readonly Logger target=Logger.write
        /// @resolution.call source=this.write(fields) parameters=({ readonly [key: string]: int32 }) arguments=(provided(fields) as { readonly [key: string]: int32 }) return=void regions=(Logger.info.'a) kind=symbol target=Logger.write receiver=&Logger.info.'a readonly Logger instance=Logger.write<Logger.info.'a>
        /// @resolution.receiver source=this kind=this declaration=Logger type=&Logger.info.'a readonly Logger
        /// @resolution.place source=this placement=Logger.info.'a lifetime=Logger.info.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Logger.write<Logger.info.'a> template=Logger.write arguments=(Logger.info.'a)
        /// @generic.instance id=Logger.write<Logger.info.'a> template=Logger.write arguments=(Logger.info.'a)
        /// @resolution.name source=fields target=Logger.info.fields
        /// @resolution.place source=fields placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=fields root=Logger.info.fields

    }
}
"#);
}

/// Store a readonly handle in a record field.
#[test]
fn test_store_a_readonly_handle_in_a_record_field() {
    let session = TestSession::single(
        r#"
class Logger {
    write(&readonly this, options: { fields: { readonly [key: string]: int32 } }): void {}

    info(&readonly this, fields: { readonly [key: string]: int32 }): void {
        this.write({ fields });
    }
}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
class Logger {
    write(&readonly this, options: { fields: { readonly [key: string]: int32 } }): void {}

    info(&readonly this, fields: { readonly [key: string]: int32 }): void {
        this.write<'a>({ fields });
    }
}

=== dir ===
class Logger {
/// @type.symbol symbol=Logger type=typeof Logger
/// @definition.class symbol=Logger
/// @definition.method symbol=Logger.info slot=info type=<Logger.info.'a>(this: &Logger.info.'a readonly Logger, { readonly [key: string]: int32 }) => void
/// @definition.method symbol=Logger.write slot=write type=<Logger.write.'a>(this: &Logger.write.'a readonly Logger, { fields: { readonly [key: string]: int32 } }) => void

    write(&readonly this, options: { fields: { readonly [key: string]: int32 } }): void {}
    /// @generic.template symbol=Logger.write parameters=('a)
    /// @type.symbol symbol=Logger.write type=<Logger.write.'a>(this: &Logger.write.'a readonly Logger, { fields: { readonly [key: string]: int32 } }) => void
    /// @type.symbol symbol=Logger.write.this source="&readonly this" type=&Logger.write.'a readonly Logger
    /// @type.symbol symbol=Logger.write.options source="options: { fields: { readonly [key: string]: int32 } }" type={ fields: { readonly [key: string]: int32 } }
    /// @type.symbol symbol=Logger.write.fields source="fields: { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }

    info(&readonly this, fields: { readonly [key: string]: int32 }): void {
    /// @generic.template symbol=Logger.info parameters=('a)
    /// @type.symbol symbol=Logger.info type=<Logger.info.'a>(this: &Logger.info.'a readonly Logger, { readonly [key: string]: int32 }) => void
    /// @type.symbol symbol=Logger.info.this source="&readonly this" type=&Logger.info.'a readonly Logger
    /// @type.symbol symbol=Logger.info.fields source="fields: { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }

        this.write({ fields });
        /// @resolution.member source=this.write receiver=&Logger.info.'a readonly Logger type=<Logger.write.'a>(this: &Logger.write.'a readonly Logger, { fields: { readonly [key: string]: int32 } }) => void kind=symbol target_receiver=&Logger.info.'a readonly Logger target=Logger.write
        /// @resolution.call source="this.write({ fields })" parameters=({ fields: { readonly [key: string]: int32 } }) arguments=(provided({ fields }) as { fields: { readonly [key: string]: int32 } }) return=void regions=(Logger.info.'a) kind=symbol target=Logger.write receiver=&Logger.info.'a readonly Logger instance=Logger.write<Logger.info.'a>
        /// @resolution.receiver source=this kind=this declaration=Logger type=&Logger.info.'a readonly Logger
        /// @resolution.place source=this placement=Logger.info.'a lifetime=Logger.info.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Logger.write<Logger.info.'a> template=Logger.write arguments=(Logger.info.'a)
        /// @generic.instance id=Logger.write<Logger.info.'a> template=Logger.write arguments=(Logger.info.'a)
        /// @resolution.name source=fields target=Logger.info.fields
        /// @resolution.place source=fields placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=fields root=Logger.info.fields

    }
}
"#);
}
