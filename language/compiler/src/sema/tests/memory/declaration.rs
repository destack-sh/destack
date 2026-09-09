use crate::tests::{DirRows, TestSession};

#[test]
fn test_place_newtypes_from_declaration_modifiers() {
    let session = TestSession::single(
        r#"
local newtype LocalCount = int32;
shared newtype SharedCount = int32;

declare const localCount: LocalCount;
declare const sharedCount: SharedCount;

localCount satisfies local LocalCount;
sharedCount satisfies shared SharedCount;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
local newtype LocalCount = int32;
shared newtype SharedCount = int32;

declare const localCount: LocalCount;
declare const sharedCount: SharedCount;

localCount satisfies local LocalCount;
sharedCount satisfies shared SharedCount;

=== dir ===
local newtype LocalCount = int32;
/// @type.symbol symbol=LocalCount source="local newtype LocalCount = int32" type=LocalCount
/// @definition.newtype symbol=LocalCount source="local newtype LocalCount = int32" backing=int32 constructors=[(int32) => LocalCount]

shared newtype SharedCount = int32;
/// @type.symbol symbol=SharedCount source="shared newtype SharedCount = int32" type=SharedCount
/// @definition.newtype symbol=SharedCount source="shared newtype SharedCount = int32" backing=int32 constructors=[(int32) => SharedCount]

declare const localCount: LocalCount;
/// @type.symbol symbol=localCount source=localCount type=LocalCount
/// @resolution.pattern source=localCount kind=binding target=localCount
/// @resolution.name source=LocalCount target=LocalCount

declare const sharedCount: SharedCount;
/// @type.symbol symbol=sharedCount source=sharedCount type=SharedCount
/// @resolution.pattern source=sharedCount kind=binding target=sharedCount
/// @resolution.name source=SharedCount target=SharedCount

localCount satisfies local LocalCount;
/// @resolution.name source=localCount target=localCount
/// @resolution.place source=localCount placement="local" lifetime="static" access="readonly"
/// @resolution.access source=localCount root=localCount
/// @resolution.name source=LocalCount target=LocalCount

sharedCount satisfies shared SharedCount;
/// @resolution.name source=sharedCount target=sharedCount
/// @resolution.place source=sharedCount placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=sharedCount root=sharedCount
/// @resolution.name source=SharedCount target=SharedCount
"#,
        r#"

"#,
    );
}

#[test]
fn test_place_generic_newtypes_from_declaration_modifiers() {
    let session = TestSession::single(
        r#"
local newtype LocalBox<T> = { value: T };
shared newtype SharedBox<T> = { value: T };

declare const localBox: LocalBox<int32>;
declare const sharedBox: SharedBox<int32>;

localBox satisfies local LocalBox<int32>;
sharedBox satisfies shared SharedBox<int32>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
local newtype LocalBox<in out T> = { value: T };
shared newtype SharedBox<in out T> = { value: T };

declare const localBox: LocalBox<int32>;
declare const sharedBox: SharedBox<int32>;

localBox satisfies local LocalBox<int32>;
sharedBox satisfies shared SharedBox<int32>;

=== dir ===
local newtype LocalBox<T> = { value: T };
/// @generic.template symbol=LocalBox parameters=(in out T#1)
/// @type.symbol symbol=LocalBox source="local newtype LocalBox<T> = { value: T }" type=LocalBox
/// @definition.newtype symbol=LocalBox source="local newtype LocalBox<T> = { value: T }" template=(in out T#1) backing={ value: T#1 } constructors=[<T#1>({ value: T#1 }) => LocalBox<T#1>]
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

localBox satisfies local LocalBox<int32>;
/// @resolution.name source=localBox target=localBox
/// @resolution.place source=localBox placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=localBox root=localBox
/// @resolution.name source=LocalBox target=LocalBox

sharedBox satisfies shared SharedBox<int32>;
/// @resolution.name source=sharedBox target=sharedBox
/// @resolution.place source=sharedBox placement="shared" lifetime="managed" access="mutable"
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
local class LocalUser {}
shared class SharedUser {}
local struct LocalPoint { x: int32; }
shared struct SharedPoint { x: int32; }
local enum LocalStatus { Ready }
shared enum SharedStatus { Ready }
local newtype interface LocalReadable { read(): int32; }
shared newtype interface SharedReadable { read(): int32; }

declare const localUser: LocalUser;
declare const sharedUser: SharedUser;
declare const localPoint: LocalPoint;
declare const sharedPoint: SharedPoint;
declare const localStatus: LocalStatus;
declare const sharedStatus: SharedStatus;
declare const localReadable: LocalReadable;
declare const sharedReadable: SharedReadable;

localUser satisfies local LocalUser;
sharedUser satisfies shared SharedUser;
localPoint satisfies local LocalPoint;
sharedPoint satisfies shared SharedPoint;
localStatus satisfies local LocalStatus;
sharedStatus satisfies shared SharedStatus;
localReadable satisfies local LocalReadable;
sharedReadable satisfies shared SharedReadable;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
local class LocalUser {}
shared class SharedUser {}
local struct LocalPoint {
    x: int32;
}
shared struct SharedPoint {
    x: int32;
}
local enum LocalStatus {
    Ready,
}
shared enum SharedStatus {
    Ready,
}
local newtype interface LocalReadable {
    read(): int32;
}
shared newtype interface SharedReadable {
    read(): int32;
}

declare const localUser: LocalUser;
declare const sharedUser: SharedUser;
declare const localPoint: LocalPoint;
declare const sharedPoint: SharedPoint;
declare const localStatus: LocalStatus;
declare const sharedStatus: SharedStatus;
declare const localReadable: LocalReadable;
declare const sharedReadable: SharedReadable;

localUser satisfies local LocalUser;
sharedUser satisfies shared SharedUser;
localPoint satisfies local LocalPoint;
sharedPoint satisfies shared SharedPoint;
localStatus satisfies local LocalStatus;
sharedStatus satisfies shared SharedStatus;
localReadable satisfies local LocalReadable;
sharedReadable satisfies shared SharedReadable;

=== dir ===
local class LocalUser {}
/// @type.symbol symbol=LocalUser source="local class LocalUser {}" type=LocalUser
/// @definition.class symbol=LocalUser source="local class LocalUser {}"

shared class SharedUser {}
/// @type.symbol symbol=SharedUser source="shared class SharedUser {}" type=SharedUser
/// @definition.class symbol=SharedUser source="shared class SharedUser {}"

local struct LocalPoint { x: int32; }
/// @type.symbol symbol=LocalPoint source="local struct LocalPoint { x: int32; }" type=LocalPoint
/// @definition.struct symbol=LocalPoint source="local struct LocalPoint { x: int32; }"
/// @definition.field symbol=LocalPoint.x source="x: int32" key=x type=int32
/// @type.symbol symbol=LocalPoint.x source="x: int32" type=int32

shared struct SharedPoint { x: int32; }
/// @type.symbol symbol=SharedPoint source="shared struct SharedPoint { x: int32; }" type=SharedPoint
/// @definition.struct symbol=SharedPoint source="shared struct SharedPoint { x: int32; }"
/// @definition.field symbol=SharedPoint.x source="x: int32" key=x type=int32
/// @type.symbol symbol=SharedPoint.x source="x: int32" type=int32

local enum LocalStatus { Ready }
/// @type.symbol symbol=LocalStatus source="local enum LocalStatus { Ready }" type=LocalStatus
/// @definition.enum symbol=LocalStatus source="local enum LocalStatus { Ready }"
/// @definition.variant symbol=LocalStatus.Ready source=Ready key=Ready value=0
/// @type.symbol symbol=LocalStatus.Ready source=Ready type=LocalStatus.Ready

shared enum SharedStatus { Ready }
/// @type.symbol symbol=SharedStatus source="shared enum SharedStatus { Ready }" type=SharedStatus
/// @definition.enum symbol=SharedStatus source="shared enum SharedStatus { Ready }"
/// @definition.variant symbol=SharedStatus.Ready source=Ready key=Ready value=0
/// @type.symbol symbol=SharedStatus.Ready source=Ready type=SharedStatus.Ready

local newtype interface LocalReadable { read(): int32; }
/// @generic.template symbol=LocalReadable parameters=(this: LocalReadable)
/// @type.symbol symbol=LocalReadable source="local newtype interface LocalReadable { read(): int32; }" type=LocalReadable
/// @definition.interface symbol=LocalReadable source="local newtype interface LocalReadable { read(): int32; }" template=(this: LocalReadable) nominal=true
/// @definition.where symbol=LocalReadable source="local newtype interface LocalReadable { read(): int32; }" relation=satisfies left=this right=LocalReadable
/// @definition.method symbol=LocalReadable.read source="read(): int32" slot=read type=(this: this) => int32
/// @type.symbol symbol=LocalReadable.read source="read(): int32" type=(this: this) => int32

shared newtype interface SharedReadable { read(): int32; }
/// @generic.template symbol=SharedReadable parameters=(this: SharedReadable)
/// @type.symbol symbol=SharedReadable source="shared newtype interface SharedReadable { read(): int32; }" type=SharedReadable
/// @definition.interface symbol=SharedReadable source="shared newtype interface SharedReadable { read(): int32; }" template=(this: SharedReadable) nominal=true
/// @definition.where symbol=SharedReadable source="shared newtype interface SharedReadable { read(): int32; }" relation=satisfies left=this right=SharedReadable
/// @definition.method symbol=SharedReadable.read source="read(): int32" slot=read type=(this: this) => int32
/// @type.symbol symbol=SharedReadable.read source="read(): int32" type=(this: this) => int32

declare const localUser: LocalUser;
/// @type.symbol symbol=localUser source=localUser type=LocalUser
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=LocalUser target=LocalUser

declare const sharedUser: SharedUser;
/// @type.symbol symbol=sharedUser source=sharedUser type=SharedUser
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=SharedUser target=SharedUser

declare const localPoint: LocalPoint;
/// @type.symbol symbol=localPoint source=localPoint type=LocalPoint
/// @resolution.pattern source=localPoint kind=binding target=localPoint
/// @resolution.name source=LocalPoint target=LocalPoint

declare const sharedPoint: SharedPoint;
/// @type.symbol symbol=sharedPoint source=sharedPoint type=SharedPoint
/// @resolution.pattern source=sharedPoint kind=binding target=sharedPoint
/// @resolution.name source=SharedPoint target=SharedPoint

declare const localStatus: LocalStatus;
/// @type.symbol symbol=localStatus source=localStatus type=LocalStatus
/// @resolution.pattern source=localStatus kind=binding target=localStatus
/// @resolution.name source=LocalStatus target=LocalStatus

declare const sharedStatus: SharedStatus;
/// @type.symbol symbol=sharedStatus source=sharedStatus type=SharedStatus
/// @resolution.pattern source=sharedStatus kind=binding target=sharedStatus
/// @resolution.name source=SharedStatus target=SharedStatus

declare const localReadable: LocalReadable;
/// @type.symbol symbol=localReadable source=localReadable type=LocalReadable
/// @resolution.pattern source=localReadable kind=binding target=localReadable
/// @resolution.name source=LocalReadable target=LocalReadable

declare const sharedReadable: SharedReadable;
/// @type.symbol symbol=sharedReadable source=sharedReadable type=SharedReadable
/// @resolution.pattern source=sharedReadable kind=binding target=sharedReadable
/// @resolution.name source=SharedReadable target=SharedReadable

localUser satisfies local LocalUser;
/// @resolution.name source=localUser target=localUser
/// @resolution.place source=localUser placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=localUser root=localUser
/// @resolution.name source=LocalUser target=LocalUser

sharedUser satisfies shared SharedUser;
/// @resolution.name source=sharedUser target=sharedUser
/// @resolution.place source=sharedUser placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=sharedUser root=sharedUser
/// @resolution.name source=SharedUser target=SharedUser

localPoint satisfies local LocalPoint;
/// @resolution.name source=localPoint target=localPoint
/// @resolution.place source=localPoint placement="local" lifetime="static" access="readonly"
/// @resolution.access source=localPoint root=localPoint
/// @resolution.name source=LocalPoint target=LocalPoint

sharedPoint satisfies shared SharedPoint;
/// @resolution.name source=sharedPoint target=sharedPoint
/// @resolution.place source=sharedPoint placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=sharedPoint root=sharedPoint
/// @resolution.name source=SharedPoint target=SharedPoint

localStatus satisfies local LocalStatus;
/// @resolution.name source=localStatus target=localStatus
/// @resolution.place source=localStatus placement="local" lifetime="static" access="readonly"
/// @resolution.access source=localStatus root=localStatus
/// @resolution.name source=LocalStatus target=LocalStatus

sharedStatus satisfies shared SharedStatus;
/// @resolution.name source=sharedStatus target=sharedStatus
/// @resolution.place source=sharedStatus placement="shared" lifetime="static" access="readonly"
/// @resolution.access source=sharedStatus root=sharedStatus
/// @resolution.name source=SharedStatus target=SharedStatus

localReadable satisfies local LocalReadable;
/// @resolution.name source=localReadable target=localReadable
/// @resolution.place source=localReadable placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=localReadable root=localReadable
/// @resolution.name source=LocalReadable target=LocalReadable

sharedReadable satisfies shared SharedReadable;
/// @resolution.name source=sharedReadable target=sharedReadable
/// @resolution.place source=sharedReadable placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=sharedReadable root=sharedReadable
/// @resolution.name source=SharedReadable target=SharedReadable
"#,
        r#"
"#,
    );
}

#[test]
fn test_accept_explicit_placement_matching_declaration() {
    let session = TestSession::single(
        r#"
local class LocalUser {}
shared class SharedUser {}

declare const localUser: local LocalUser;
declare const sharedUser: shared SharedUser;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
local class LocalUser {}
shared class SharedUser {}

declare const localUser: LocalUser;
declare const sharedUser: SharedUser;

=== dir ===
local class LocalUser {}
/// @type.symbol symbol=LocalUser source="local class LocalUser {}" type=LocalUser
/// @definition.class symbol=LocalUser source="local class LocalUser {}"

shared class SharedUser {}
/// @type.symbol symbol=SharedUser source="shared class SharedUser {}" type=SharedUser
/// @definition.class symbol=SharedUser source="shared class SharedUser {}"

declare const localUser: local LocalUser;
/// @type.symbol symbol=localUser source=localUser type=LocalUser
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=LocalUser target=LocalUser

declare const sharedUser: shared SharedUser;
/// @type.symbol symbol=sharedUser source=sharedUser type=SharedUser
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=SharedUser target=SharedUser
"#,
        r#"

"#,
    );
}

#[test]
fn test_reject_explicit_placement_conflicting_with_declaration() {
    let session = TestSession::single(
        r#"
local class LocalUser {}
shared class SharedUser {}

declare const wrongLocal: shared LocalUser;
declare const wrongShared: local SharedUser;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
local class LocalUser {}
shared class SharedUser {}

declare const wrongLocal: shared LocalUser;
declare const wrongShared: local SharedUser;

=== dir ===
local class LocalUser {}
/// @type.symbol symbol=LocalUser source="local class LocalUser {}" type=LocalUser
/// @definition.class symbol=LocalUser source="local class LocalUser {}"

shared class SharedUser {}
/// @type.symbol symbol=SharedUser source="shared class SharedUser {}" type=SharedUser
/// @definition.class symbol=SharedUser source="shared class SharedUser {}"

declare const wrongLocal: shared LocalUser;
/// @type.symbol symbol=wrongLocal source=wrongLocal type=shared LocalUser
/// @resolution.pattern source=wrongLocal kind=binding target=wrongLocal
/// @resolution.name source=LocalUser target=LocalUser

declare const wrongShared: local SharedUser;
/// @type.symbol symbol=wrongShared source=wrongShared type=local SharedUser
/// @resolution.pattern source=wrongShared kind=binding target=wrongShared
/// @resolution.name source=SharedUser target=SharedUser
"#,
        r#"
/// @diagnostic.error id=placement-conflict message="placement 'shared' conflicts with the declaration placement 'local'"
/// @diagnostic.label line=5 column=27 span="shared" line_source="declare const wrongLocal: shared LocalUser;"
/// @diagnostic.related line=2 column=13 span="LocalUser" line_source="local class LocalUser {}" message="'LocalUser' is local"
/// @diagnostic.help message="remove 'shared' or use a shared type"
/// @diagnostic.error id=placement-conflict message="placement 'local' conflicts with the declaration placement 'shared'"
/// @diagnostic.label line=6 column=28 span="local" line_source="declare const wrongShared: local SharedUser;"
/// @diagnostic.related line=3 column=14 span="SharedUser" line_source="shared class SharedUser {}" message="'SharedUser' is shared"
/// @diagnostic.help message="remove 'local' or use a local type"
"#,
    );
}

#[test]
fn test_place_unmodified_nominal_declarations_from_context() {
    let session = TestSession::single(
        r#"
class User {}
struct Point { x: int32; }

declare const localUser: local User;
declare const sharedUser: shared User;
declare const localPoint: local Point;
declare const sharedPoint: shared Point;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}
struct Point {
    x: int32;
}

declare const localUser: local User;
declare const sharedUser: shared User;
declare const localPoint: Point;
declare const sharedPoint: Point;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

struct Point { x: int32; }
/// @type.symbol symbol=Point source="struct Point { x: int32; }" type=Point
/// @definition.struct symbol=Point source="struct Point { x: int32; }"
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @type.symbol symbol=Point.x source="x: int32" type=int32

declare const localUser: local User;
/// @type.symbol symbol=localUser source=localUser type=local User
/// @resolution.pattern source=localUser kind=binding target=localUser
/// @resolution.name source=User target=User

declare const sharedUser: shared User;
/// @type.symbol symbol=sharedUser source=sharedUser type=shared User
/// @resolution.pattern source=sharedUser kind=binding target=sharedUser
/// @resolution.name source=User target=User

declare const localPoint: local Point;
/// @type.symbol symbol=localPoint source=localPoint type=Point
/// @resolution.pattern source=localPoint kind=binding target=localPoint
/// @resolution.name source=Point target=Point

declare const sharedPoint: shared Point;
/// @type.symbol symbol=sharedPoint source=sharedPoint type=Point
/// @resolution.pattern source=sharedPoint kind=binding target=sharedPoint
/// @resolution.name source=Point target=Point
"#,
        r#"
"#,
    );
}

#[test]
fn test_inherit_class_placement_from_base_class() {
    let session = TestSession::single(
        r#"
local class LocalBase {}
class LocalDerived extends LocalBase {}

shared class SharedBase {}
class SharedDerived extends SharedBase {}

declare const localDerived: LocalDerived;
declare const sharedDerived: SharedDerived;

localDerived satisfies local LocalDerived;
sharedDerived satisfies shared SharedDerived;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
local class LocalBase {}
class LocalDerived extends LocalBase {}

shared class SharedBase {}
class SharedDerived extends SharedBase {}

declare const localDerived: LocalDerived;
declare const sharedDerived: SharedDerived;

localDerived satisfies local LocalDerived;
sharedDerived satisfies shared SharedDerived;

=== dir ===
local class LocalBase {}
/// @type.symbol symbol=LocalBase source="local class LocalBase {}" type=LocalBase
/// @definition.class symbol=LocalBase source="local class LocalBase {}"

class LocalDerived extends LocalBase {}
/// @type.symbol symbol=LocalDerived source="class LocalDerived extends LocalBase {}" type=LocalDerived
/// @definition.class symbol=LocalDerived source="class LocalDerived extends LocalBase {}"
/// @definition.extends symbol=LocalDerived source=LocalBase target=LocalBase
/// @resolution.name source=LocalBase target=LocalBase

shared class SharedBase {}
/// @type.symbol symbol=SharedBase source="shared class SharedBase {}" type=SharedBase
/// @definition.class symbol=SharedBase source="shared class SharedBase {}"

class SharedDerived extends SharedBase {}
/// @type.symbol symbol=SharedDerived source="class SharedDerived extends SharedBase {}" type=SharedDerived
/// @definition.class symbol=SharedDerived source="class SharedDerived extends SharedBase {}"
/// @definition.extends symbol=SharedDerived source=SharedBase target=SharedBase
/// @resolution.name source=SharedBase target=SharedBase

declare const localDerived: LocalDerived;
/// @type.symbol symbol=localDerived source=localDerived type=LocalDerived
/// @resolution.pattern source=localDerived kind=binding target=localDerived
/// @resolution.name source=LocalDerived target=LocalDerived

declare const sharedDerived: SharedDerived;
/// @type.symbol symbol=sharedDerived source=sharedDerived type=SharedDerived
/// @resolution.pattern source=sharedDerived kind=binding target=sharedDerived
/// @resolution.name source=SharedDerived target=SharedDerived

localDerived satisfies local LocalDerived;
/// @resolution.name source=localDerived target=localDerived
/// @resolution.place source=localDerived placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=localDerived root=localDerived
/// @resolution.name source=LocalDerived target=LocalDerived

sharedDerived satisfies shared SharedDerived;
/// @resolution.name source=sharedDerived target=sharedDerived
/// @resolution.place source=sharedDerived placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=sharedDerived root=sharedDerived
/// @resolution.name source=SharedDerived target=SharedDerived
"#,
        r#"

"#,
    );
}

#[test]
fn test_inherit_class_placement_from_implemented_interface() {
    let session = TestSession::single(
        r#"
local newtype interface LocalService {}
class LocalServiceImpl implements LocalService {}

shared newtype interface SharedService {}
class SharedServiceImpl implements SharedService {}

declare const localService: LocalServiceImpl;
declare const sharedService: SharedServiceImpl;

localService satisfies local LocalServiceImpl;
sharedService satisfies shared SharedServiceImpl;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
local newtype interface LocalService {}
class LocalServiceImpl implements LocalService {}

shared newtype interface SharedService {}
class SharedServiceImpl implements SharedService {}

declare const localService: LocalServiceImpl;
declare const sharedService: SharedServiceImpl;

localService satisfies local LocalServiceImpl;
sharedService satisfies shared SharedServiceImpl;

=== dir ===
local newtype interface LocalService {}
/// @generic.template symbol=LocalService parameters=(this: LocalService)
/// @type.symbol symbol=LocalService source="local newtype interface LocalService {}" type=LocalService
/// @definition.interface symbol=LocalService source="local newtype interface LocalService {}" template=(this: LocalService) nominal=true
/// @definition.where symbol=LocalService source="local newtype interface LocalService {}" relation=satisfies left=this right=LocalService

class LocalServiceImpl implements LocalService {}
/// @type.symbol symbol=LocalServiceImpl source="class LocalServiceImpl implements LocalService {}" type=LocalServiceImpl
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
/// @type.symbol symbol=SharedServiceImpl source="class SharedServiceImpl implements SharedService {}" type=SharedServiceImpl
/// @definition.class symbol=SharedServiceImpl source="class SharedServiceImpl implements SharedService {}"
/// @definition.where symbol=SharedServiceImpl source=SharedService relation=satisfies left=this right=SharedService
/// @definition.implements symbol=SharedServiceImpl source=SharedService target=SharedService
/// @resolution.name source=SharedService target=SharedService

declare const localService: LocalServiceImpl;
/// @type.symbol symbol=localService source=localService type=LocalServiceImpl
/// @resolution.pattern source=localService kind=binding target=localService
/// @resolution.name source=LocalServiceImpl target=LocalServiceImpl

declare const sharedService: SharedServiceImpl;
/// @type.symbol symbol=sharedService source=sharedService type=SharedServiceImpl
/// @resolution.pattern source=sharedService kind=binding target=sharedService
/// @resolution.name source=SharedServiceImpl target=SharedServiceImpl

localService satisfies local LocalServiceImpl;
/// @resolution.name source=localService target=localService
/// @resolution.place source=localService placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=localService root=localService
/// @resolution.name source=LocalServiceImpl target=LocalServiceImpl

sharedService satisfies shared SharedServiceImpl;
/// @resolution.name source=sharedService target=sharedService
/// @resolution.place source=sharedService placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=sharedService root=sharedService
/// @resolution.name source=SharedServiceImpl target=SharedServiceImpl
"#,
        r#"

"#,
    );
}

#[test]
fn test_reject_conflicting_placement_in_heritage() {
    let session = TestSession::single(
        r#"
local class LocalBase {}
shared newtype interface SharedService {}

class Invalid extends LocalBase implements SharedService {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
local class LocalBase {}
shared newtype interface SharedService {}

class Invalid extends LocalBase implements SharedService {}

=== dir ===
local class LocalBase {}
/// @type.symbol symbol=LocalBase source="local class LocalBase {}" type=LocalBase
/// @definition.class symbol=LocalBase source="local class LocalBase {}"

shared newtype interface SharedService {}
/// @generic.template symbol=SharedService parameters=(this: SharedService)
/// @type.symbol symbol=SharedService source="shared newtype interface SharedService {}" type=SharedService
/// @definition.interface symbol=SharedService source="shared newtype interface SharedService {}" template=(this: SharedService) nominal=true
/// @definition.where symbol=SharedService source="shared newtype interface SharedService {}" relation=satisfies left=this right=SharedService

class Invalid extends LocalBase implements SharedService {}
/// @type.symbol symbol=Invalid source="class Invalid extends LocalBase implements SharedService {}" type=Invalid
/// @definition.class symbol=Invalid source="class Invalid extends LocalBase implements SharedService {}"
/// @definition.extends symbol=Invalid source=LocalBase target=LocalBase
/// @definition.where symbol=Invalid source=SharedService relation=satisfies left=this right=SharedService
/// @definition.implements symbol=Invalid source=SharedService target=SharedService
/// @resolution.name source=LocalBase target=LocalBase
/// @resolution.name source=SharedService target=SharedService
"#,
        r#"
/// @diagnostic.error id=heritage-placement-conflict message="heritage declarations require one consistent placement"
/// @diagnostic.label line=5 column=23 span="LocalBase" line_source="class Invalid extends LocalBase implements SharedService {}"
/// @diagnostic.related line=5 column=44 span="SharedService" line_source="class Invalid extends LocalBase implements SharedService {}" message="conflicting placement"
/// @diagnostic.related line=2 column=13 span="LocalBase" line_source="local class LocalBase {}" message="'LocalBase' is declared here"
/// @diagnostic.related line=3 column=26 span="SharedService" line_source="shared newtype interface SharedService {}" message="'SharedService' is declared here"
/// @diagnostic.help message="make every base and implemented interface use the same placement"
"#,
    );
}

#[test]
fn test_place_structural_interface_from_use_context() {
    let session = TestSession::single(
        r#"
interface Readable { read(): int32; }

declare const localReadable: local Readable;
declare const sharedReadable: shared Readable;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Readable {
    read(): int32;
}

declare const localReadable: local Readable;
declare const sharedReadable: shared Readable;

=== dir ===
interface Readable { read(): int32; }
/// @generic.template symbol=Readable parameters=(this: Readable)
/// @type.symbol symbol=Readable source="interface Readable { read(): int32; }" type=Readable
/// @definition.interface symbol=Readable source="interface Readable { read(): int32; }" template=(this: Readable)
/// @definition.where symbol=Readable source="interface Readable { read(): int32; }" relation=satisfies left=this right=Readable
/// @definition.method symbol=Readable.read source="read(): int32" slot=read type=(this: this) => int32
/// @type.symbol symbol=Readable.read source="read(): int32" type=(this: this) => int32

declare const localReadable: local Readable;
/// @type.symbol symbol=localReadable source=localReadable type=local Readable
/// @resolution.pattern source=localReadable kind=binding target=localReadable
/// @resolution.name source=Readable target=Readable

declare const sharedReadable: shared Readable;
/// @type.symbol symbol=sharedReadable source=sharedReadable type=shared Readable
/// @resolution.pattern source=sharedReadable kind=binding target=sharedReadable
/// @resolution.name source=Readable target=Readable
"#,
        r#"

"#,
    );
}

#[test]
fn test_fix_explicit_receiver_to_nominal_declaration_space() {
    let session = TestSession::single(
        r#"
local struct Continuation<T> {
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
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
local struct Continuation<out T> {
    resume(this, value: T): T {
        return value;
    }
}

shared newtype interface SharedQueue<in T> {
    push(this, value: T): void;
}

=== dir ===
local struct Continuation<T> {
/// @generic.template symbol=Continuation parameters=(out T#1)
/// @type.symbol symbol=Continuation type=Continuation
/// @definition.struct symbol=Continuation template=(out T#1)
/// @definition.method symbol=Continuation.resume slot=resume type=(this: this, T#1) => T#1
/// @type.symbol symbol=Continuation.T source=T type=T#1

    resume(this, value: T): T {
    /// @type.symbol symbol=Continuation.resume type=(this: this, T#1) => T#1
    /// @type.symbol symbol=Continuation.resume.this source=this type=this
    /// @type.symbol symbol=Continuation.resume.value source="value: T" type=T#1
    /// @resolution.name source=T target=Continuation.T
    /// @resolution.name source=T target=Continuation.T

        return value;
        /// @resolution.name source=value target=Continuation.resume.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="mutable"
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
local struct Next<T> { value: T; }
local struct Return<T> { value: T; }

declare function consume<T>(request: Request<T>): void;

type Request<T> = Next<T> | Return<T>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
local struct Next<out T> {
    value: T;
}
local struct Return<out T> {
    value: T;
}

declare function consume<T>(request: Request<T>): void;

type Request<T> = Next<T> | Return<T>;

=== dir ===
local struct Next<T> { value: T; }
/// @generic.template symbol=Next parameters=(out T#1)
/// @type.symbol symbol=Next source="local struct Next<T> { value: T; }" type=Next
/// @definition.struct symbol=Next source="local struct Next<T> { value: T; }" template=(out T#1)
/// @definition.field symbol=Next.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Next.T source=T type=T#1
/// @type.symbol symbol=Next.value source="value: T" type=T#1
/// @resolution.name source=T target=Next.T

local struct Return<T> { value: T; }
/// @generic.template symbol=Return parameters=(out T#2)
/// @type.symbol symbol=Return source="local struct Return<T> { value: T; }" type=Return
/// @definition.struct symbol=Return source="local struct Return<T> { value: T; }" template=(out T#2)
/// @definition.field symbol=Return.value source="value: T" key=value type=T#2
/// @type.symbol symbol=Return.T source=T type=T#2
/// @type.symbol symbol=Return.value source="value: T" type=T#2
/// @resolution.name source=T target=Return.T

declare function consume<T>(request: Request<T>): void;
/// @generic.template symbol=consume parameters=(T#3)
/// @type.symbol symbol=consume source="declare function consume<T>(request: Request<T>): void" type=<T#3>(Request<T#3>) => void
/// @type.symbol symbol=consume.T source=T type=T#3
/// @type.symbol symbol=consume.request source="request: Request<T>" type=Request<T#3>
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
