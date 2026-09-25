use crate::tests::{DirRows, TestSession};

#[test]
fn test_place_generic_newtypes_from_declaration_modifiers() {
    let session = TestSession::single(
        r#"
newtype LocalBox<T> = { value: T };
shared newtype SharedBox<T> = { value: T };

declare const localBox: LocalBox<int32>;
declare const sharedBox: SharedBox<int32>;

localBox satisfies LocalBox<int32>;
sharedBox satisfies SharedBox<int32>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype LocalBox<in out T> = { value: T };
shared newtype SharedBox<in out T> = { value: T };

declare const localBox: LocalBox<int32>;
declare const sharedBox: SharedBox<int32>;

localBox satisfies LocalBox<int32>;
sharedBox satisfies SharedBox<int32>;

=== dir ===
newtype LocalBox<T> = { value: T };
/// @generic.template symbol=LocalBox parameters=(in out T#1)
/// @type.symbol symbol=LocalBox source="newtype LocalBox<T> = { value: T }" type=LocalBox
/// @definition.newtype symbol=LocalBox source="newtype LocalBox<T> = { value: T }" template=(in out T#1) backing={ value: T#1 } constructors=[<T#1>({ value: T#1 }) => LocalBox<T#1>]
/// @type.symbol symbol=LocalBox.T source=T type=T#1
/// @type.symbol symbol=LocalBox.value source="value: T" type=T#1
/// @resolution.name source=T target=LocalBox.T

shared newtype SharedBox<T> = { value: T };
/// @generic.template symbol=SharedBox parameters=(in out T#2)
/// @type.symbol symbol=SharedBox source="shared newtype SharedBox<T> = { value: T }" type=SharedBox
/// @definition.newtype symbol=SharedBox source="shared newtype SharedBox<T> = { value: T }" template=(in out T#2) backing={ value: T#2 } constructors=[<T#2>({ value: T#2 }) => SharedBox<T#2>]
/// @type.symbol symbol=SharedBox.T source=T type=T#2
/// @type.symbol symbol=SharedBox.value source="value: T" type=T#2
/// @resolution.name source=T target=SharedBox.T

declare const localBox: LocalBox<int32>;
/// @type.symbol symbol=localBox source=localBox type=LocalBox<int32>
/// @resolution.pattern source=localBox kind=binding target=localBox
/// @resolution.name source=LocalBox target=LocalBox

declare const sharedBox: SharedBox<int32>;
/// @type.symbol symbol=sharedBox source=sharedBox type=SharedBox<int32>
/// @resolution.pattern source=sharedBox kind=binding target=sharedBox
/// @resolution.name source=SharedBox target=SharedBox

localBox satisfies LocalBox<int32>;
/// @resolution.name source=localBox target=localBox
/// @resolution.place source=localBox placement="local" lifetime="static" access="immutable"
/// @resolution.access source=localBox root=localBox
/// @resolution.name source=LocalBox target=LocalBox

sharedBox satisfies SharedBox<int32>;
/// @resolution.name source=sharedBox target=sharedBox
/// @resolution.place source=sharedBox placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=sharedBox root=sharedBox
/// @resolution.name source=SharedBox target=SharedBox
"#,
        r#"
"#,
    );
}

#[test]
fn test_place_nominal_types_from_declaration_modifiers() {
    let session = TestSession::single(
        r#"
class LocalUser {}
shared class SharedUser {}
newtype interface LocalReadable { read(): int32; }
shared newtype interface SharedReadable { read(): int32; }

declare const localUser: LocalUser;
declare const sharedUser: SharedUser;
declare const localReadable: LocalReadable;
declare const sharedReadable: SharedReadable;

localUser satisfies LocalUser;
sharedUser satisfies SharedUser;
localReadable satisfies LocalReadable;
sharedReadable satisfies SharedReadable;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class LocalUser {}
shared class SharedUser {}
newtype interface LocalReadable {
    read(): int32;
}
shared newtype interface SharedReadable {
    read(): int32;
}

declare const localUser: LocalUser;
declare const sharedUser: SharedUser;
declare const localReadable: LocalReadable;
declare const sharedReadable: SharedReadable;

localUser satisfies LocalUser;
sharedUser satisfies SharedUser;
localReadable satisfies LocalReadable;
sharedReadable satisfies SharedReadable;

=== dir ===
class LocalUser {}
/// @type.symbol symbol=LocalUser source="class LocalUser {}" type=typeof LocalUser
/// @definition.class symbol=LocalUser source="class LocalUser {}"

shared class SharedUser {}
/// @type.symbol symbol=SharedUser source="shared class SharedUser {}" type=typeof SharedUser
/// @definition.class symbol=SharedUser source="shared class SharedUser {}"

newtype interface LocalReadable { read(): int32; }
/// @generic.template symbol=LocalReadable parameters=(this: LocalReadable)
/// @type.symbol symbol=LocalReadable source="newtype interface LocalReadable { read(): int32; }" type=LocalReadable
/// @definition.interface symbol=LocalReadable source="newtype interface LocalReadable { read(): int32; }" template=(this: LocalReadable) nominal=true
/// @definition.where symbol=LocalReadable source="newtype interface LocalReadable { read(): int32; }" relation=satisfies left=this right=LocalReadable
/// @definition.method symbol=LocalReadable.read source="read(): int32" slot=read type=() => int32
/// @type.symbol symbol=LocalReadable.read source="read(): int32" type=() => int32

shared newtype interface SharedReadable { read(): int32; }
/// @generic.template symbol=SharedReadable parameters=(this: SharedReadable)
/// @type.symbol symbol=SharedReadable source="shared newtype interface SharedReadable { read(): int32; }" type=SharedReadable
/// @definition.interface symbol=SharedReadable source="shared newtype interface SharedReadable { read(): int32; }" template=(this: SharedReadable) nominal=true
/// @definition.where symbol=SharedReadable source="shared newtype interface SharedReadable { read(): int32; }" relation=satisfies left=this right=SharedReadable
/// @definition.method symbol=SharedReadable.read source="read(): int32" slot=read type=() => int32
/// @type.symbol symbol=SharedReadable.read source="read(): int32" type=() => int32

declare const localUser: LocalUser;
/// @type.symbol symbol=localUser source=localUser type=LocalUser
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=LocalUser target=LocalUser

declare const sharedUser: SharedUser;
/// @type.symbol symbol=sharedUser source=sharedUser type=SharedUser
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=SharedUser target=SharedUser

declare const localReadable: LocalReadable;
/// @type.symbol symbol=localReadable source=localReadable type=LocalReadable
/// @resolution.pattern source=localReadable kind=binding target=localReadable
/// @resolution.name source=LocalReadable target=LocalReadable

declare const sharedReadable: SharedReadable;
/// @type.symbol symbol=sharedReadable source=sharedReadable type=SharedReadable
/// @resolution.pattern source=sharedReadable kind=binding target=sharedReadable
/// @resolution.name source=SharedReadable target=SharedReadable

localUser satisfies LocalUser;
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="static" access="immutable"
/// @resolution.access source=localUser root=localUser
/// @resolution.name source=LocalUser target=LocalUser

sharedUser satisfies SharedUser;
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=sharedUser root=sharedUser
/// @resolution.name source=SharedUser target=SharedUser

localReadable satisfies LocalReadable;
/// @resolution.name source=localReadable target=localReadable
/// @resolution.place source=localReadable placement="local" lifetime="static" access="immutable"
/// @resolution.access source=localReadable root=localReadable
/// @resolution.name source=LocalReadable target=LocalReadable

sharedReadable satisfies SharedReadable;
/// @resolution.name source=sharedReadable target=sharedReadable
/// @resolution.place source=sharedReadable placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=sharedReadable root=sharedReadable
/// @resolution.name source=SharedReadable target=SharedReadable
"#,
        r#"
"#,
    );
}

#[test]
fn test_inherit_class_placement_from_base_class() {
    let session = TestSession::single(
        r#"
class LocalBase {}
class LocalDerived extends LocalBase {}

shared class SharedBase {}
class SharedDerived extends SharedBase {}

shared class SharedSharedDerived extends SharedBase {}

declare const localDerived: LocalDerived;
declare const sharedDerived: SharedDerived;

localDerived satisfies LocalDerived;
sharedDerived satisfies SharedSharedDerived;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class LocalBase {}
class LocalDerived extends LocalBase {}

shared class SharedBase {}
class SharedDerived extends SharedBase {}

shared class SharedSharedDerived extends SharedBase {}

declare const localDerived: LocalDerived;
declare const sharedDerived: SharedDerived;

localDerived satisfies LocalDerived;
sharedDerived satisfies SharedSharedDerived;

=== dir ===
class LocalBase {}
/// @type.symbol symbol=LocalBase source="class LocalBase {}" type=typeof LocalBase
/// @definition.class symbol=LocalBase source="class LocalBase {}"

class LocalDerived extends LocalBase {}
/// @type.symbol symbol=LocalDerived source="class LocalDerived extends LocalBase {}" type=typeof LocalDerived
/// @definition.class symbol=LocalDerived source="class LocalDerived extends LocalBase {}"
/// @definition.extends symbol=LocalDerived source=LocalBase target=LocalBase
/// @resolution.name source=LocalBase target=LocalBase

shared class SharedBase {}
/// @type.symbol symbol=SharedBase source="shared class SharedBase {}" type=typeof SharedBase
/// @definition.class symbol=SharedBase source="shared class SharedBase {}"

class SharedDerived extends SharedBase {}
/// @type.symbol symbol=SharedDerived source="class SharedDerived extends SharedBase {}" type=typeof SharedDerived
/// @definition.class symbol=SharedDerived source="class SharedDerived extends SharedBase {}"
/// @definition.extends symbol=SharedDerived source=SharedBase target=SharedBase
/// @resolution.name source=SharedBase target=SharedBase

shared class SharedSharedDerived extends SharedBase {}
/// @type.symbol symbol=SharedSharedDerived source="shared class SharedSharedDerived extends SharedBase {}" type=typeof SharedSharedDerived
/// @definition.class symbol=SharedSharedDerived source="shared class SharedSharedDerived extends SharedBase {}"
/// @definition.extends symbol=SharedSharedDerived source=SharedBase target=SharedBase
/// @resolution.name source=SharedBase target=SharedBase

declare const localDerived: LocalDerived;
/// @type.symbol symbol=localDerived source=localDerived type=LocalDerived
/// @resolution.pattern source=localDerived kind=binding target=localDerived
/// @resolution.name source=LocalDerived target=LocalDerived

declare const sharedDerived: SharedDerived;
/// @type.symbol symbol=sharedDerived source=sharedDerived type=SharedDerived
/// @resolution.pattern source=sharedDerived kind=binding target=sharedDerived
/// @resolution.name source=SharedDerived target=SharedDerived

localDerived satisfies LocalDerived;
/// @resolution.name source=localDerived target=localDerived
/// @resolution.place source=localDerived placement="local" lifetime="static" access="immutable"
/// @resolution.access source=localDerived root=localDerived
/// @resolution.name source=LocalDerived target=LocalDerived

sharedDerived satisfies SharedSharedDerived;
/// @resolution.name source=sharedDerived target=sharedDerived
/// @resolution.place source=sharedDerived placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=sharedDerived root=sharedDerived
/// @resolution.name source=SharedSharedDerived target=SharedSharedDerived
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'SharedDerived' does not satisfy 'SharedSharedDerived'"
/// @diagnostic.label line=14 column=1 span="sharedDerived" line_source="sharedDerived satisfies SharedSharedDerived;"
"#,
    );
}

#[test]
fn test_inherit_class_placement_from_implemented_interface() {
    let session = TestSession::single(
        r#"
newtype interface LocalService {}
class LocalServiceImpl implements LocalService {}

shared newtype interface SharedService {}
class SharedServiceImpl implements SharedService {}

shared class SharedSharedServiceImpl implements SharedService {}

declare const localService: LocalServiceImpl;
declare const sharedService: SharedServiceImpl;

localService satisfies LocalServiceImpl;
sharedService satisfies SharedSharedServiceImpl;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface LocalService {}
class LocalServiceImpl implements LocalService {}

shared newtype interface SharedService {}
class SharedServiceImpl implements SharedService {}

shared class SharedSharedServiceImpl implements SharedService {}

declare const localService: LocalServiceImpl;
declare const sharedService: SharedServiceImpl;

localService satisfies LocalServiceImpl;
sharedService satisfies SharedSharedServiceImpl;

=== dir ===
newtype interface LocalService {}
/// @generic.template symbol=LocalService parameters=(this: LocalService)
/// @type.symbol symbol=LocalService source="newtype interface LocalService {}" type=LocalService
/// @definition.interface symbol=LocalService source="newtype interface LocalService {}" template=(this: LocalService) nominal=true
/// @definition.where symbol=LocalService source="newtype interface LocalService {}" relation=satisfies left=this right=LocalService

class LocalServiceImpl implements LocalService {}
/// @type.symbol symbol=LocalServiceImpl source="class LocalServiceImpl implements LocalService {}" type=typeof LocalServiceImpl
/// @definition.class symbol=LocalServiceImpl source="class LocalServiceImpl implements LocalService {}"
/// @definition.where symbol=LocalServiceImpl source=LocalService relation=satisfies left=this right=LocalService
/// @definition.implements symbol=LocalServiceImpl source=LocalService target=LocalService
/// @resolution.name source=LocalService target=LocalService

shared newtype interface SharedService {}
/// @generic.template symbol=SharedService parameters=(this: SharedService)
/// @type.symbol symbol=SharedService source="shared newtype interface SharedService {}" type=SharedService
/// @definition.interface symbol=SharedService source="shared newtype interface SharedService {}" template=(this: SharedService) nominal=true
/// @definition.where symbol=SharedService source="shared newtype interface SharedService {}" relation=satisfies left=this right=SharedService

class SharedServiceImpl implements SharedService {}
/// @type.symbol symbol=SharedServiceImpl source="class SharedServiceImpl implements SharedService {}" type=typeof SharedServiceImpl
/// @definition.class symbol=SharedServiceImpl source="class SharedServiceImpl implements SharedService {}"
/// @definition.where symbol=SharedServiceImpl source=SharedService relation=satisfies left=this right=SharedService
/// @definition.implements symbol=SharedServiceImpl source=SharedService target=SharedService
/// @resolution.name source=SharedService target=SharedService

shared class SharedSharedServiceImpl implements SharedService {}
/// @type.symbol symbol=SharedSharedServiceImpl source="shared class SharedSharedServiceImpl implements SharedService {}" type=typeof SharedSharedServiceImpl
/// @definition.class symbol=SharedSharedServiceImpl source="shared class SharedSharedServiceImpl implements SharedService {}"
/// @definition.where symbol=SharedSharedServiceImpl source=SharedService relation=satisfies left=this right=SharedService
/// @definition.implements symbol=SharedSharedServiceImpl source=SharedService target=SharedService
/// @resolution.name source=SharedService target=SharedService

declare const localService: LocalServiceImpl;
/// @type.symbol symbol=localService source=localService type=LocalServiceImpl
/// @resolution.pattern source=localService kind=binding target=localService
/// @resolution.name source=LocalServiceImpl target=LocalServiceImpl

declare const sharedService: SharedServiceImpl;
/// @type.symbol symbol=sharedService source=sharedService type=SharedServiceImpl
/// @resolution.pattern source=sharedService kind=binding target=sharedService
/// @resolution.name source=SharedServiceImpl target=SharedServiceImpl

localService satisfies LocalServiceImpl;
/// @resolution.name source=localService target=localService
/// @resolution.place source=localService placement="local" lifetime="static" access="immutable"
/// @resolution.access source=localService root=localService
/// @resolution.name source=LocalServiceImpl target=LocalServiceImpl

sharedService satisfies SharedSharedServiceImpl;
/// @resolution.name source=sharedService target=sharedService
/// @resolution.place source=sharedService placement="shared" lifetime="static" access="immutable"
/// @resolution.access source=sharedService root=sharedService
/// @resolution.name source=SharedSharedServiceImpl target=SharedSharedServiceImpl
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'SharedServiceImpl' does not satisfy 'SharedSharedServiceImpl'"
/// @diagnostic.label line=14 column=1 span="sharedService" line_source="sharedService satisfies SharedSharedServiceImpl;"
"#,
    );
}

#[test]
fn test_reject_conflicting_placement_in_heritage() {
    let session = TestSession::single(
        r#"
class LocalBase {}
shared newtype interface SharedService {}

class Invalid extends LocalBase implements SharedService {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class LocalBase {}
shared newtype interface SharedService {}

class Invalid extends LocalBase implements SharedService {}

=== dir ===
class LocalBase {}
/// @type.symbol symbol=LocalBase source="class LocalBase {}" type=typeof LocalBase
/// @definition.class symbol=LocalBase source="class LocalBase {}"

shared newtype interface SharedService {}
/// @generic.template symbol=SharedService parameters=(this: SharedService)
/// @type.symbol symbol=SharedService source="shared newtype interface SharedService {}" type=SharedService
/// @definition.interface symbol=SharedService source="shared newtype interface SharedService {}" template=(this: SharedService) nominal=true
/// @definition.where symbol=SharedService source="shared newtype interface SharedService {}" relation=satisfies left=this right=SharedService

class Invalid extends LocalBase implements SharedService {}
/// @type.symbol symbol=Invalid source="class Invalid extends LocalBase implements SharedService {}" type=typeof Invalid
/// @definition.class symbol=Invalid source="class Invalid extends LocalBase implements SharedService {}"
/// @definition.extends symbol=Invalid source=LocalBase target=LocalBase
/// @definition.where symbol=Invalid source=SharedService relation=satisfies left=this right=SharedService
/// @definition.implements symbol=Invalid source=SharedService target=SharedService
/// @resolution.name source=LocalBase target=LocalBase
/// @resolution.name source=SharedService target=SharedService
"#,
        r#"
/// @diagnostic.error id=heritage-space-conflict message="heritage declarations require one space"
/// @diagnostic.label line=5 column=23 span="LocalBase" line_source="class Invalid extends LocalBase implements SharedService {}"
/// @diagnostic.related line=5 column=44 span="SharedService" line_source="class Invalid extends LocalBase implements SharedService {}" message="conflicting space"
/// @diagnostic.related line=2 column=7 span="LocalBase" line_source="class LocalBase {}" message="'LocalBase' is declared here"
/// @diagnostic.related line=3 column=26 span="SharedService" line_source="shared newtype interface SharedService {}" message="'SharedService' is declared here"
/// @diagnostic.help message="make every base and implemented interface use the same space"
"#,
    );
}

#[test]
fn test_fix_explicit_receiver_to_nominal_declaration_space() {
    let session = TestSession::single(
        r#"
struct Continuation<T> {
    resume(this, value: T): T {
        return value;
    }
}

shared newtype interface SharedQueue<T> {
    push(this, value: T): void;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Continuation<out T> {
    resume(this, value: T): T {
        return value;
    }
}

shared newtype interface SharedQueue<in T> {
    push(this, value: T): void;
}

=== dir ===
struct Continuation<T> {
/// @generic.template symbol=Continuation parameters=(out T#1)
/// @type.symbol symbol=Continuation type=Continuation
/// @definition.struct symbol=Continuation template=(out T#1)
/// @definition.method symbol=Continuation.resume slot=resume type=(this: Continuation<T#1>, T#1) => T#1
/// @type.symbol symbol=Continuation.T source=T type=T#1

    resume(this, value: T): T {
    /// @type.symbol symbol=Continuation.resume type=(this: Continuation<T#1>, T#1) => T#1
    /// @type.symbol symbol=Continuation.resume.this source=this type=Continuation<T#1>
    /// @type.symbol symbol=Continuation.resume.value source="value: T" type=T#1
    /// @resolution.name source=T target=Continuation.T
    /// @resolution.name source=T target=Continuation.T

        return value;
        /// @resolution.name source=value target=Continuation.resume.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Continuation.resume.value

    }
}

shared newtype interface SharedQueue<T> {
/// @generic.template symbol=SharedQueue parameters=(in T#2, this: SharedQueue<T#2>)
/// @type.symbol symbol=SharedQueue type=SharedQueue
/// @definition.interface symbol=SharedQueue template=(in T#2, this: SharedQueue<T#2>) nominal=true
/// @definition.where symbol=SharedQueue relation=satisfies left=this right=SharedQueue<T#2>
/// @definition.method symbol=SharedQueue.push source="push(this, value: T): void" slot=push type=(this: this, T#2) => void
/// @type.symbol symbol=SharedQueue.T source=T type=T#2

    push(this, value: T): void;
    /// @type.symbol symbol=SharedQueue.push source="push(this, value: T): void" type=(this: this, T#2) => void
    /// @type.symbol symbol=SharedQueue.push.this source=this type=this
    /// @type.symbol symbol=SharedQueue.push.value source="value: T" type=T#2
    /// @resolution.name source=T target=SharedQueue.T

}
"#,
        r#"

"#,
    );
}

#[test]
fn test_preserve_common_intrinsic_space_through_union_alias() {
    let session = TestSession::single(
        r#"
struct Next<T> { value: T; }
struct Return<T> { value: T; }

declare function consume<T>(request: Request<T>): void;

type Request<T> = Next<T> | Return<T>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Next<out T> {
    value: T;
}
struct Return<out T> {
    value: T;
}

declare function consume<T>(request: Request<T>): void;

type Request<T> = Next<T> | Return<T>;

=== dir ===
struct Next<T> { value: T; }
/// @generic.template symbol=Next parameters=(out T#1)
/// @type.symbol symbol=Next source="struct Next<T> { value: T; }" type=Next
/// @definition.struct symbol=Next source="struct Next<T> { value: T; }" template=(out T#1)
/// @definition.field symbol=Next.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Next.T source=T type=T#1
/// @type.symbol symbol=Next.value source="value: T" type=T#1
/// @resolution.name source=T target=Next.T

struct Return<T> { value: T; }
/// @generic.template symbol=Return parameters=(out T#2)
/// @type.symbol symbol=Return source="struct Return<T> { value: T; }" type=Return
/// @definition.struct symbol=Return source="struct Return<T> { value: T; }" template=(out T#2)
/// @definition.field symbol=Return.value source="value: T" key=value type=T#2
/// @type.symbol symbol=Return.T source=T type=T#2
/// @type.symbol symbol=Return.value source="value: T" type=T#2
/// @resolution.name source=T target=Return.T

declare function consume<T>(request: Request<T>): void;
/// @generic.template symbol=consume parameters=(T#3)
/// @type.symbol symbol=consume source="declare function consume<T>(request: Request<T>): void" type=<T#3>(Request<T#3>) => void
/// @type.symbol symbol=consume.T source=T type=T#3
/// @resolution.name source=Request target=Request
/// @resolution.name source=T target=consume.T

type Request<T> = Next<T> | Return<T>;
/// @generic.template symbol=Request parameters=(T#4)
/// @type.symbol symbol=Request source="type Request<T> = Next<T> | Return<T>" type=Next<T#4> | Return<T#4>
/// @definition.type symbol=Request source="type Request<T> = Next<T> | Return<T>" template=(T#4) value=Next<T#4> | Return<T#4>
/// @type.symbol symbol=Request.T source=T type=T#4
/// @resolution.name source=Next target=Next
/// @resolution.name source=T target=Request.T
/// @resolution.name source=Return target=Return
/// @resolution.name source=T target=Request.T
"#,
        r#"
"#,
    );
}
