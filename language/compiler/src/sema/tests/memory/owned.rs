use crate::tests::{DirRows, TestSession};

#[test]
fn test_pass_const_owned_binding_by_value() {
    let session = TestSession::single(
        r#"
struct Payload {
    value: int32;
}

declare function consume(value: ^Payload): void;

const payload: ^Payload = Payload { value: 1 };
consume(payload);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Payload {
    value: int32;
}

declare function consume(value: ^Payload): void;

const payload: Payload = Payload { value: 1 };
consume(payload);

=== dir ===
struct Payload {
/// @type.symbol symbol=Payload type=Payload
/// @definition.struct symbol=Payload
/// @definition.field symbol=Payload.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Payload.value source="value: int32" type=int32

}

declare function consume(value: ^Payload): void;
/// @type.symbol symbol=consume source="declare function consume(value: ^Payload): void" type=(Payload) => void
/// @resolution.name source=Payload target=Payload

const payload: ^Payload = Payload { value: 1 };
/// @type.symbol symbol=payload source=payload type=Payload
/// @resolution.pattern source=payload kind=binding target=payload
/// @resolution.name source=Payload target=Payload
/// @type.node source="Payload { value: 1 }" type=Payload
/// @resolution.name source=Payload target=Payload
/// @type.node source=1 type=1

consume(payload);
/// @type.node source=consume type=(Payload) => void
/// @type.node source=consume(payload) type=void
/// @resolution.name source=consume target=consume
/// @resolution.call source=consume(payload) parameters=(Payload) arguments=(provided(payload) as Payload) return=void kind=symbol target=consume
/// @type.node source=payload type=Payload
/// @resolution.name source=payload target=payload
/// @resolution.place source=payload placement="local" lifetime="static" access="immutable"
/// @resolution.access source=payload root=payload
"#,
        r#"
"#,
    );
}

#[test]
fn test_yield_owned_value_from_move_expression() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
}

let point: ^Point = Point { x: 1 };

point satisfies ^Point;
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

point satisfies ^Point;

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

point satisfies ^Point;
/// @type.node source="point satisfies ^Point" type=Point
/// @type.node source=point type=Point
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point
/// @resolution.name source=Point target=Point
"#,
    );
}

#[test]
fn test_materialize_struct_call_result_into_owned_destination() {
    let session = TestSession::single(
        r#"
struct Buffer {
    values: int32[];
}

declare function makeBuffer(): Buffer;

let buffer: ^Buffer = makeBuffer();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Buffer {
    values: int32[];
}

declare function makeBuffer(): Buffer;

let buffer: Buffer = makeBuffer();

=== dir ===
struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.values source="values: int32[]" key=values type=int32[]

    values: int32[];
    /// @type.symbol symbol=Buffer.values source="values: int32[]" type=int32[]
    /// @generic.instance id=Array<int32> template=Array arguments=(int32)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
    /// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)

}

declare function makeBuffer(): Buffer;
/// @type.symbol symbol=makeBuffer source="declare function makeBuffer(): Buffer" type=() => Buffer
/// @resolution.name source=Buffer target=Buffer

let buffer: ^Buffer = makeBuffer();
/// @type.symbol symbol=buffer source=buffer type=Buffer
/// @resolution.pattern source=buffer kind=binding target=buffer
/// @resolution.name source=Buffer target=Buffer
/// @type.node source=makeBuffer type=() => Buffer
/// @type.node source=makeBuffer() type=Buffer
/// @resolution.name source=makeBuffer target=makeBuffer
/// @resolution.call source=makeBuffer() parameters=() return=Buffer kind=symbol target=makeBuffer
"#,
    );
}

#[test]
fn test_reject_managed_call_result_as_owned() {
    let session = TestSession::single(
        r#"
class User {}

declare function makeUser(): User;

let user: ^User = makeUser();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}

declare function makeUser(): User;

let user: ^User = makeUser();

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare function makeUser(): User;
/// @type.symbol symbol=makeUser source="declare function makeUser(): User" type=() => User
/// @resolution.name source=User target=User

let user: ^User = makeUser();
/// @type.symbol symbol=user source=user type=^User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User
/// @type.node source=makeUser type=() => User
/// @type.node source=makeUser() type=User
/// @resolution.name source=makeUser target=makeUser
/// @resolution.call source=makeUser() parameters=() return=User kind=symbol target=makeUser
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'User' is not assignable to type '^User'"
/// @diagnostic.label line=6 column=19 span="makeUser()" line_source="let user: ^User = makeUser();"
/// @diagnostic.related line=6 column=11 span="^" line_source="let user: ^User = makeUser();" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_preserve_owned_form_on_fields() {
    let session = TestSession::single(
        r#"
struct Data {
    value: int32;
}

struct Container {
    data: ^Data;
}

const container = Container { data: Data { value: 1 } };

container.data satisfies ^Data;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Data {
    value: int32;
}

struct Container {
    data: Data;
}

const container: Container = Container { data: Data { value: 1 } };

container.data satisfies ^Data;

=== dir ===
struct Data {
/// @type.symbol symbol=Data type=Data
/// @definition.struct symbol=Data
/// @definition.field symbol=Data.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Data.value source="value: int32" type=int32

}

struct Container {
/// @type.symbol symbol=Container type=Container
/// @definition.struct symbol=Container
/// @definition.field symbol=Container.data source="data: ^Data" key=data type=Data

    data: ^Data;
    /// @type.symbol symbol=Container.data source="data: ^Data" type=Data
    /// @resolution.name source=Data target=Data

}

const container = Container { data: Data { value: 1 } };
/// @type.symbol symbol=container source=container type=Container
/// @resolution.pattern source=container kind=binding target=container
/// @type.node source="Container { data: Data { value: 1 } }" type=Container
/// @resolution.name source=Container target=Container
/// @type.node source="Data { value: 1 }" type=Data
/// @resolution.name source=Data target=Data
/// @type.node source=1 type=1

container.data satisfies ^Data;
/// @type.node source="container.data satisfies ^Data" type=Data
/// @type.node source=container type=Container
/// @type.node source=container.data type=Data
/// @resolution.name source=container target=container
/// @resolution.member source=container.data receiver=Container type=Data kind=field target_receiver=Container key=data target=Container.data target_type=Data
/// @resolution.place source=container placement="local" lifetime="static" access="immutable"
/// @resolution.access source=container root=container
/// @resolution.place source=container.data placement="local" lifetime="static" access="immutable"
/// @resolution.access source=container.data root=container keys=[data]
/// @resolution.name source=Data target=Data
"#,
    );
}

#[test]
fn test_reject_nested_mutation_of_readonly_owned_value() {
    let session = TestSession::single(
        r#"
struct Profile {
    name: string;
}

struct User {
    profile: Profile;
}

let user: ^readonly User = User {
    profile: Profile { name: "Ada" },
};

user.profile.name = "Grace";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Profile {
    name: string;
}

struct User {
    profile: Profile;
}

let user: readonly User = readonly User {
    profile: Profile { name: "Ada" },
};

user.profile.name = "Grace";

=== dir ===
struct Profile {
/// @type.symbol symbol=Profile type=Profile
/// @definition.struct symbol=Profile
/// @definition.field symbol=Profile.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Profile.name source="name: string" type=string

}

struct User {
/// @type.symbol symbol=User type=User
/// @definition.struct symbol=User
/// @definition.field symbol=User.profile source="profile: Profile" key=profile type=Profile

    profile: Profile;
    /// @type.symbol symbol=User.profile source="profile: Profile" type=Profile
    /// @resolution.name source=Profile target=Profile

}

let user: ^readonly User = User {
/// @type.symbol symbol=user source=user type=readonly User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User
/// @type.node type=readonly User
/// @resolution.name source=User target=User

    profile: Profile { name: "Ada" },
    /// @type.node source="Profile { name: \"Ada\" }" type=Profile
    /// @resolution.name source=Profile target=Profile
    /// @type.node source="\"Ada\"" type="Ada"

};

user.profile.name = "Grace";
/// @type.node source="user.profile.name = \"Grace\"" type="Grace"
/// @type.node source=user type=readonly User
/// @type.node source=user.profile type=Profile
/// @type.node source=user.profile.name type=string
/// @resolution.name source=user target=user
/// @resolution.member source=user.profile receiver=readonly User type=Profile kind=field target_receiver=readonly User key=profile target=User.profile target_type=Profile
/// @resolution.place source=user placement="local" lifetime="static" access="readonly"
/// @resolution.access source=user root=user
/// @resolution.place source=user.profile placement="local" lifetime="static" access="readonly"
/// @resolution.access source=user.profile root=user keys=[profile]
/// @resolution.pattern.assign source=user.profile.name kind=place
/// @resolution.place source=user.profile.name placement="local" lifetime="static" access="readonly"
/// @resolution.access source=user.profile.name root=user keys=[profile, name]
/// @resolution.assignment source=user.profile.name write="receiver=readonly Profile, target=field(receiver=readonly Profile, target=Profile.name, type=string), type=string" type=string
/// @type.node source="\"Grace\"" type="Grace"
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'name'"
/// @diagnostic.label line=14 column=14 span="name" line_source="user.profile.name = \"Grace\";"
"#,
    );
}

#[test]
fn test_copy_only_owned_inline_storage() {
    let session = TestSession::single(
        r#"
import { Copy } from "destack:memory";

struct Point {
    x: int32 = 0;
    y: int32 = 0;
}

class Pair {
    left: int32 = 0;
    right: int32 = 0;
}

class Named {
    label: string = "";
}

function witness<T: Copy>(value: T): T {
    return value;
}

declare const point: ^Point;
witness(point);

declare const pair: ^Pair;
witness(pair);

declare const object: ^{ value: int32 };
witness(object);

declare const named: ^Named;
witness(named);

declare const values: ^Array<int32>;
witness(values);

declare const slice: ^[int32];
witness(slice);

declare const callable: ^(() => void);
witness(callable);

declare const dynamic: ^Dynamic<unknown>;
witness(dynamic);

declare const buffer: ^{ items: ^[int32] };
witness(buffer);

declare const text: ^string;
witness(text);

declare const big: ^bigint;
witness(big);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Copy } from "destack:memory";

struct Point {
    x: int32 = 0;
    y: int32 = 0;
}

class Pair {
    left: int32 = 0;
    right: int32 = 0;
}

class Named {
    label: string = "";
}

function witness<T: Copy>(value: T): T {
    return value;
}

declare const point: Point;
witness<Point>(point);

declare const pair: ^Pair;
witness<^Pair>(pair);

declare const object: ^{ value: int32 };
witness<^{ value: int32 }>(object);

declare const named: ^Named;
witness<^Named>(named);

declare const values: ^int32[];
witness<^int32[]>(values);

declare const slice: ^[int32];
witness<^[int32]>(slice);

declare const callable: ^(() => void);
witness<^(() => void)>(callable);

declare const dynamic: ^Dynamic<unknown>;
witness<^Dynamic<unknown>>(dynamic);

declare const buffer: ^{ items: ^[int32] };
witness<^{ items: ^[int32] }>(buffer);

declare const text: ^string;
witness<^string>(text);

declare const big: ^bigint;
witness<^bigint>(big);

=== dir ===
import { Copy } from "destack:memory";

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32 = 0" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32 = 0" key=y type=int32

    x: int32 = 0;
    /// @type.symbol symbol=Point.x source="x: int32 = 0" type=int32

    y: int32 = 0;
    /// @type.symbol symbol=Point.y source="y: int32 = 0" type=int32

}

class Pair {
/// @type.symbol symbol=Pair type=typeof Pair
/// @definition.class symbol=Pair
/// @definition.field symbol=Pair.left source="left: int32 = 0" key=left type=int32
/// @definition.field symbol=Pair.right source="right: int32 = 0" key=right type=int32

    left: int32 = 0;
    /// @type.symbol symbol=Pair.left source="left: int32 = 0" type=int32

    right: int32 = 0;
    /// @type.symbol symbol=Pair.right source="right: int32 = 0" type=int32

}

class Named {
/// @type.symbol symbol=Named type=typeof Named
/// @definition.class symbol=Named
/// @definition.field symbol=Named.label source="label: string = \"\"" key=label type=string

    label: string = "";
    /// @type.symbol symbol=Named.label source="label: string = \"\"" type=string

}

function witness<T: Copy>(value: T): T {
/// @generic.template symbol=witness parameters=(T: Copy)
/// @type.symbol symbol=witness type=<T: Copy>(T) => T
/// @type.symbol symbol=witness.T source="T: Copy" type=T
/// @resolution.name source=Copy target=Copy
/// @type.symbol symbol=witness.value source="value: T" type=T
/// @resolution.name source=T target=witness.T
/// @resolution.name source=T target=witness.T

    return value;
    /// @resolution.name source=value target=witness.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=witness.value

}

declare const point: ^Point;
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

witness(point);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(point) parameters=(Point) arguments=(provided(point) as Point) return=Point kind=symbol target=witness instance=witness<Point>
/// @generic.instantiation id=witness<Point> template=witness arguments=(Point)
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="immutable"
/// @resolution.access source=point root=point

declare const pair: ^Pair;
/// @type.symbol symbol=pair source=pair type=^Pair
/// @resolution.pattern source=pair kind=binding target=pair
/// @resolution.name source=Pair target=Pair

witness(pair);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(pair) parameters=(^Pair) arguments=(provided(pair) as ^Pair) return=^Pair kind=symbol target=witness instance=witness<^Pair>
/// @generic.instantiation id=witness<^Pair> template=witness arguments=(^Pair)
/// @resolution.name source=pair target=pair
/// @resolution.place source=pair placement="local" lifetime="static" access="immutable"
/// @resolution.access source=pair root=pair

declare const object: ^{ value: int32 };
/// @type.symbol symbol=object source=object type=^{ value: int32 }
/// @resolution.pattern source=object kind=binding target=object
/// @type.symbol symbol=value source="value: int32" type=int32

witness(object);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(object) parameters=(^{ value: int32 }) arguments=(provided(object) as ^{ value: int32 }) return=^{ value: int32 } kind=symbol target=witness instance="witness<^{ value: int32 }>"
/// @generic.instantiation id="witness<^{ value: int32 }>" template=witness arguments=(^{ value: int32 })
/// @resolution.name source=object target=object
/// @resolution.place source=object placement="local" lifetime="static" access="immutable"
/// @resolution.access source=object root=object

declare const named: ^Named;
/// @type.symbol symbol=named source=named type=^Named
/// @resolution.pattern source=named kind=binding target=named
/// @resolution.name source=Named target=Named

witness(named);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(named) parameters=(^Named) arguments=(provided(named) as ^Named) return=^Named kind=symbol target=witness instance=witness<^Named>
/// @generic.instantiation id=witness<^Named> template=witness arguments=(^Named)
/// @resolution.name source=named target=named
/// @resolution.place source=named placement="local" lifetime="static" access="immutable"
/// @resolution.access source=named root=named

declare const values: ^Array<int32>;
/// @type.symbol symbol=values source=values type=^Array<int32>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=Array target=Array

witness(values);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(values) parameters=(^Array<int32>) arguments=(provided(values) as ^Array<int32>) return=^Array<int32> kind=symbol target=witness instance=witness<^Array<int32>>
/// @generic.instantiation id=witness<^Array<int32>> template=witness arguments=(^Array<int32>)
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values

declare const slice: ^[int32];
/// @type.symbol symbol=slice source=slice type=^Slice<int32>
/// @resolution.pattern source=slice kind=binding target=slice

witness(slice);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(slice) parameters=(^Slice<int32>) arguments=(provided(slice) as ^Slice<int32>) return=^Slice<int32> kind=symbol target=witness instance=witness<^Slice<int32>>
/// @generic.instantiation id=witness<^Slice<int32>> template=witness arguments=(^Slice<int32>)
/// @resolution.name source=slice target=slice
/// @resolution.place source=slice placement="local" lifetime="static" access="immutable"
/// @resolution.access source=slice root=slice

declare const callable: ^(() => void);
/// @type.symbol symbol=callable source=callable type=^(() => void)
/// @resolution.pattern source=callable kind=binding target=callable

witness(callable);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(callable) parameters=(^(() => void)) arguments=(provided(callable) as ^(() => void)) return=^(() => void) kind=symbol target=witness instance="witness<^(() => void)>"
/// @generic.instantiation id="witness<^(() => void)>" template=witness arguments=(^(() => void))
/// @resolution.name source=callable target=callable
/// @resolution.place source=callable placement="local" lifetime="static" access="immutable"
/// @resolution.access source=callable root=callable

declare const dynamic: ^Dynamic<unknown>;
/// @type.symbol symbol=dynamic source=dynamic type=^Dynamic<unknown>
/// @resolution.pattern source=dynamic kind=binding target=dynamic
/// @resolution.name source=Dynamic target=Dynamic

witness(dynamic);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(dynamic) parameters=(^Dynamic<unknown>) arguments=(provided(dynamic) as ^Dynamic<unknown>) return=^Dynamic<unknown> kind=symbol target=witness instance=witness<^Dynamic<unknown>>
/// @generic.instantiation id=witness<^Dynamic<unknown>> template=witness arguments=(^Dynamic<unknown>)
/// @resolution.name source=dynamic target=dynamic
/// @resolution.place source=dynamic placement="local" lifetime="static" access="immutable"
/// @resolution.access source=dynamic root=dynamic

declare const buffer: ^{ items: ^[int32] };
/// @type.symbol symbol=buffer source=buffer type=^{ items: ^Slice<int32> }
/// @resolution.pattern source=buffer kind=binding target=buffer
/// @type.symbol symbol=items source="items: ^[int32]" type=^Slice<int32>

witness(buffer);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(buffer) parameters=(^{ items: ^Slice<int32> }) arguments=(provided(buffer) as ^{ items: ^Slice<int32> }) return=^{ items: ^Slice<int32> } kind=symbol target=witness instance="witness<^{ items: ^Slice<int32> }>"
/// @generic.instantiation id="witness<^{ items: ^Slice<int32> }>" template=witness arguments=(^{ items: ^Slice<int32> })
/// @resolution.name source=buffer target=buffer
/// @resolution.place source=buffer placement="local" lifetime="static" access="immutable"
/// @resolution.access source=buffer root=buffer

declare const text: ^string;
/// @type.symbol symbol=text source=text type=^string
/// @resolution.pattern source=text kind=binding target=text

witness(text);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(text) parameters=(^string) arguments=(provided(text) as ^string) return=^string kind=symbol target=witness instance=witness<^string>
/// @generic.instantiation id=witness<^string> template=witness arguments=(^string)
/// @resolution.name source=text target=text
/// @resolution.place source=text placement="local" lifetime="static" access="immutable"
/// @resolution.access source=text root=text

declare const big: ^bigint;
/// @type.symbol symbol=big source=big type=^bigint
/// @resolution.pattern source=big kind=binding target=big

witness(big);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(big) parameters=(^bigint) arguments=(provided(big) as ^bigint) return=^bigint kind=symbol target=witness instance=witness<^bigint>
/// @generic.instantiation id=witness<^bigint> template=witness arguments=(^bigint)
/// @resolution.name source=big target=big
/// @resolution.place source=big placement="local" lifetime="static" access="immutable"
/// @resolution.access source=big root=big
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type '^Pair' does not satisfy 'Copy'"
/// @diagnostic.label line=26 column=1 span="witness(pair)" line_source="witness(pair);"
/// @diagnostic.related line=18 column=18 span="T" line_source="function witness<T: Copy>(value: T): T {" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type '^Named' does not satisfy 'Copy'"
/// @diagnostic.label line=32 column=1 span="witness(named)" line_source="witness(named);"
/// @diagnostic.related line=18 column=18 span="T" line_source="function witness<T: Copy>(value: T): T {" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type '^int32[]' does not satisfy 'Copy'"
/// @diagnostic.label line=35 column=1 span="witness(values)" line_source="witness(values);"
/// @diagnostic.related line=18 column=18 span="T" line_source="function witness<T: Copy>(value: T): T {" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type '^Slice<int32>' does not satisfy 'Copy'"
/// @diagnostic.label line=38 column=1 span="witness(slice)" line_source="witness(slice);"
/// @diagnostic.related line=18 column=18 span="T" line_source="function witness<T: Copy>(value: T): T {" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type '^(() => void)' does not satisfy 'Copy'"
/// @diagnostic.label line=41 column=1 span="witness(callable)" line_source="witness(callable);"
/// @diagnostic.related line=18 column=18 span="T" line_source="function witness<T: Copy>(value: T): T {" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type '^Dynamic<unknown>' does not satisfy 'Copy'"
/// @diagnostic.label line=44 column=1 span="witness(dynamic)" line_source="witness(dynamic);"
/// @diagnostic.related line=18 column=18 span="T" line_source="function witness<T: Copy>(value: T): T {" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type '^{ items: ^Slice<int32> }' does not satisfy 'Copy'"
/// @diagnostic.label line=47 column=1 span="witness(buffer)" line_source="witness(buffer);"
/// @diagnostic.related line=18 column=18 span="T" line_source="function witness<T: Copy>(value: T): T {" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type '^string' does not satisfy 'Copy'"
/// @diagnostic.label line=50 column=1 span="witness(text)" line_source="witness(text);"
/// @diagnostic.related line=18 column=18 span="T" line_source="function witness<T: Copy>(value: T): T {" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type '^bigint' does not satisfy 'Copy'"
/// @diagnostic.label line=53 column=1 span="witness(big)" line_source="witness(big);"
/// @diagnostic.related line=18 column=18 span="T" line_source="function witness<T: Copy>(value: T): T {" message="required by this bound on 'T'"
"#,
    );
}

#[test]
fn test_satisfy_owned_subject_conformance_with_owned_class_value() {
    let session = TestSession::single(
        r#"
newtype interface Collect<T> {
    add(value: T): void;
}

class Bag<T> {
    last: T | undefined;

    constructor() {
        this.last = undefined;
    }
}

extension<T> of ^Bag<T> implements Collect<T> {
    add(value: T): void {}
}

declare function gather<C>(): C where C: Collect<int32>;

const bag = gather<^Bag<int32>>();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype interface Collect<in T> {
    add(value: T): void;
}

class Bag<in out T> {
    last: T | undefined;

    constructor() {
        this.last = undefined as T | undefined;
    }
}

extension<T> of ^Bag<T> implements Collect<T> {
    add(value: T): void {}
}

declare function gather<C>(): C where C: Collect<int32>;

const bag: ^Bag<int32> = gather<^Bag<int32>>();

=== dir ===
newtype interface Collect<T> {
/// @generic.template symbol=Collect parameters=(in T#1, this: Collect<T#1>)
/// @type.symbol symbol=Collect type=Collect
/// @definition.interface symbol=Collect template=(in T#1, this: Collect<T#1>) nominal=true
/// @definition.where symbol=Collect relation=satisfies left=this right=Collect<T#1>
/// @definition.method symbol=Collect.add source="add(value: T): void" slot=add type=(T#1) => void
/// @type.symbol symbol=Collect.T source=T type=T#1

    add(value: T): void;
    /// @type.symbol symbol=Collect.add source="add(value: T): void" type=(T#1) => void
    /// @type.symbol symbol=Collect.add.value source="value: T" type=T#1
    /// @resolution.name source=T target=Collect.T

}

class Bag<T> {
/// @generic.template symbol=Bag parameters=(in out T#2)
/// @type.symbol symbol=Bag type=typeof Bag
/// @definition.class symbol=Bag template=(in out T#2)
/// @definition.field symbol=Bag.last source="last: T | undefined" key=last type=T#2 | undefined
/// @definition.method symbol=Bag.constructor slot=constructor role=constructor type=(this: &'managed Bag<T#2>) => Bag<T#2>
/// @type.symbol symbol=Bag.T source=T type=T#2

    last: T | undefined;
    /// @type.symbol symbol=Bag.last source="last: T | undefined" type=T#2 | undefined
    /// @resolution.name source=T target=Bag.T

    constructor() {
    /// @type.symbol symbol=Bag.constructor type=(this: &'managed Bag<T#2>) => Bag<T#2>
    /// @type.symbol symbol=Bag.constructor.this type=&'managed Bag<T#2>

        this.last = undefined;
        /// @type.node source="this.last = undefined" type=undefined
        /// @type.node source=this type=&'managed Bag<T#2>
        /// @type.node source=this.last type=T#2 | undefined
        /// @resolution.receiver source=this kind=this declaration=Bag type=&'managed Bag<T#2>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.last kind=place
        /// @resolution.place source=this.last placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.last root=this keys=[last]
        /// @resolution.assignment source=this.last write="receiver=&'managed Bag<T#2>, target=field(receiver=&'managed Bag<T#2>, target=Bag.last, type=T#2 | undefined), type=T#2 | undefined" type=T#2 | undefined
        /// @type.node source=undefined type=undefined

    }
}

extension<T> of ^Bag<T> implements Collect<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @generic.instance id=Bag<T#3> template=Bag arguments=(T#3)
/// @generic.instance id=Collect<T#3> template=Collect arguments=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=^Bag<T#3>
/// @definition.implements symbol=<module>#2 source=Collect<T> target=Collect<T#3>
/// @definition.method symbol=add source="add(value: T): void {}" slot=add type=(this: ^Bag<T#3>, T#3) => void
/// @definition.conformance symbol=<module>#2 member=add requirement=Collect.add
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=T target=T
/// @resolution.name source=Collect target=Collect
/// @resolution.name source=T target=T

    add(value: T): void {}
    /// @type.symbol symbol=add source="add(value: T): void {}" type=(this: ^Bag<T#3>, T#3) => void
    /// @type.symbol symbol=add.this type=^Bag<T#3>
    /// @type.symbol symbol=add.value source="value: T" type=T#3
    /// @resolution.name source=T target=T

}

declare function gather<C>(): C where C: Collect<int32>;
/// @generic.template symbol=gather parameters=(C)
/// @type.symbol symbol=gather source="declare function gather<C>(): C where C: Collect<int32>" type=<C>() => C
/// @type.symbol symbol=gather.C source=C type=C
/// @resolution.name source=C target=gather.C
/// @resolution.name source=C target=gather.C
/// @resolution.name source=Collect target=Collect
/// @generic.instance id=Collect<int32> template=Collect arguments=(int32)

const bag = gather<^Bag<int32>>();
/// @type.symbol symbol=bag source=bag type=^Bag<int32>
/// @resolution.pattern source=bag kind=binding target=bag
/// @generic.instance id=Bag<int32> template=Bag arguments=(int32)
/// @type.node source=gather type=() => ^Bag<int32>
/// @type.node source=gather<^Bag<int32>>() type=^Bag<int32>
/// @resolution.name source=gather target=gather
/// @resolution.call source=gather<^Bag<int32>>() parameters=() return=^Bag<int32> kind=symbol target=gather instance=gather<^Bag<int32>>
/// @generic.instantiation id=gather<^Bag<int32>> template=gather arguments=(^Bag<int32>)
/// @generic.instance id=add<int32> template=add arguments=(int32)
/// @generic.instance id=gather<^Bag<int32>> template=gather arguments=(^Bag<int32>)
/// @resolution.name source=Bag target=Bag
"#,
    );
}

#[test]
fn test_satisfy_bare_subject_conformance_with_owned_struct_value() {
    let session = TestSession::single(
        r#"
newtype interface Collect<T> {
    add(value: T): void;
}

struct Point {
    x: int32;
    y: int32;
}

extension of Point implements Collect<int32> {
    add(value: int32): void {}
}

declare function gather<C>(): C where C: Collect<int32>;

const point = gather<^Point>();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype interface Collect<in T> {
    add(value: T): void;
}

struct Point {
    x: int32;
    y: int32;
}

extension of Point implements Collect<int32> {
    add(value: int32): void {}
}

declare function gather<C>(): C where C: Collect<int32>;

const point: Point = gather<^Point>();

=== dir ===
newtype interface Collect<T> {
/// @generic.template symbol=Collect parameters=(in T, this: Collect<T>)
/// @type.symbol symbol=Collect type=Collect
/// @definition.interface symbol=Collect template=(in T, this: Collect<T>) nominal=true
/// @definition.where symbol=Collect relation=satisfies left=this right=Collect<T>
/// @definition.method symbol=Collect.add source="add(value: T): void" slot=add type=(T) => void
/// @type.symbol symbol=Collect.T source=T type=T

    add(value: T): void;
    /// @type.symbol symbol=Collect.add source="add(value: T): void" type=(T) => void
    /// @type.symbol symbol=Collect.add.value source="value: T" type=T
    /// @resolution.name source=T target=Collect.T

}

struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

extension of Point implements Collect<int32> {
/// @generic.instance id=Collect<int32> template=Collect arguments=(int32)
/// @definition.extension symbol=<module>#2 form=local target=Point
/// @definition.implements symbol=<module>#2 source=Collect<int32> target=Collect<int32>
/// @definition.method symbol=add source="add(value: int32): void {}" slot=add type=<add.'a>(this: &add.'a readonly Point, int32) => void
/// @definition.conformance symbol=<module>#2 member=add requirement=Collect.add
/// @resolution.name source=Point target=Point
/// @resolution.name source=Collect target=Collect

    add(value: int32): void {}
    /// @generic.template symbol=add parent=template#1 parameters=('a)
    /// @type.symbol symbol=add source="add(value: int32): void {}" type=<add.'a>(this: &add.'a readonly Point, int32) => void
    /// @type.symbol symbol=add.this type=&add.'a readonly Point
    /// @type.symbol symbol=add.value source="value: int32" type=int32

}

declare function gather<C>(): C where C: Collect<int32>;
/// @generic.template symbol=gather parameters=(C)
/// @type.symbol symbol=gather source="declare function gather<C>(): C where C: Collect<int32>" type=<C>() => C
/// @type.symbol symbol=gather.C source=C type=C
/// @resolution.name source=C target=gather.C
/// @resolution.name source=C target=gather.C
/// @resolution.name source=Collect target=Collect

const point = gather<^Point>();
/// @type.symbol symbol=point source=point type=Point
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source=gather type=() => Point
/// @type.node source=gather<^Point>() type=Point
/// @resolution.name source=gather target=gather
/// @resolution.call source=gather<^Point>() parameters=() return=Point kind=symbol target=gather instance=gather<Point>
/// @generic.instantiation id=gather<Point> template=gather arguments=(Point)
/// @generic.instance id=gather<Point> template=gather arguments=(Point)
/// @resolution.name source=Point target=Point
"#,
    );
}
#[test]
fn test_select_the_inherent_static_over_its_conformance_member() {
    let session = TestSession::single(
        r#"
import { Deque } from "destack:collections";

const values = Deque.from([1, 2, 3]);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Deque } from "destack:collections";

const values: ^Deque<int64> = Deque.from<int64>([1, 2, 3] as Iterable<int64>);

=== dir ===
import { Deque } from "destack:collections";

const values = Deque.from([1, 2, 3]);
/// @type.symbol symbol=values source=values type=^Deque<int64>
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Deque<int64> template=Deque arguments=(int64)
/// @generic.instance id=new<MaybeUninit<int64>> template=new arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @resolution.name source=Deque target=Deque
/// @resolution.member source=Deque.from receiver=typeof Deque type=(Iterable<T#4>) => ^Deque<T#4> kind=symbol target_receiver=typeof Deque target=from#1
/// @resolution.call source="Deque.from([1, 2, 3])" parameters=(Iterable<int64>) arguments=(provided([1, 2, 3]) as Iterable<int64>) return=^Deque<int64> kind=symbol target=from#1 instance=Deque<int64>.<extension#4>.from#1
/// @generic.instantiation id=from#1<int64> template=from#1 arguments=(int64)
/// @generic.instance id=Iterable<int64> template=Iterable arguments=(int64)
/// @generic.instance id=from#1<int64> template=from#1 arguments=(int64)
/// @resolution.call source=[1, 2, 3] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64, provided(3) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
"#,
    );
}

/// Compare a readonly view of an owned union field arm by arm.
#[test]
fn test_compare_readonly_owned_union_field() {
    let session = TestSession::single(
        r#"
struct Holder<T> {
    private storage: ^[T] | undefined = undefined;

    get isInline(): boolean {
        this.storage == undefined
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Holder<in out T> {
    private storage: ^[T] | undefined = undefined as ^[T] | undefined;

    get isInline(): boolean {
        (this.storage as ^[T] | undefined) == undefined
    }
}

=== dir ===
struct Holder<T> {
/// @generic.template symbol=Holder parameters=(in out T)
/// @type.symbol symbol=Holder type=Holder
/// @definition.struct symbol=Holder template=(in out T)
/// @definition.field symbol=Holder.storage source="private storage: ^[T] | undefined = undefined" key=storage visibility=private type=^Slice<T> | undefined
/// @definition.method symbol=Holder.isInline slot=isInline role=getter type=<Holder.isInline.'a>(this: &Holder.isInline.'a readonly Holder<T>) => boolean
/// @type.symbol symbol=Holder.T source=T type=T

    private storage: ^[T] | undefined = undefined;
    /// @type.symbol symbol=Holder.storage source="private storage: ^[T] | undefined = undefined" type=^Slice<T> | undefined
    /// @resolution.name source=T target=Holder.T

    get isInline(): boolean {
    /// @generic.template symbol=Holder.isInline parent=template#0 parameters=('a)
    /// @type.symbol symbol=Holder.isInline type=<Holder.isInline.'a>(this: &Holder.isInline.'a readonly Holder<T>) => boolean
    /// @type.symbol symbol=Holder.isInline.this type=&Holder.isInline.'a readonly Holder<T>

        this.storage == undefined
        /// @resolution.member source=this.storage receiver=&Holder.isInline.'a readonly Holder<T> type=readonly (^Slice<T> | undefined) kind=field target_receiver=&Holder.isInline.'a readonly Holder<T> key=storage target=Holder.storage target_type=readonly (^Slice<T> | undefined)
        /// @resolution.operator source="this.storage == undefined" type=boolean operator="==" kind=builtin operands=[this.storage as ^Slice<T> | undefined, undefined as undefined families=(undefined)]
        /// @resolution.receiver source=this kind=this declaration=Holder type=&Holder.isInline.'a readonly Holder<T>
        /// @resolution.place source=this placement=Holder.isInline.'a lifetime=Holder.isInline.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.storage placement=Holder.isInline.'a lifetime=Holder.isInline.'a access="readonly"
        /// @resolution.access source=this.storage root=this keys=[storage]

    }
}
"#,
        r#"
"#,
    );
}

/// Store an owned operator result back into a managed compound assignment place.
#[test]
fn test_compound_assign_transfers_owned_result_into_managed_place() {
    let session = TestSession::single(
        r#"
function build(): string {
    let output = "";
    output += "x";
    return output;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function build(): string {
    let output: string = "";
    (output += "x") as string;
    return output;
}

=== dir ===
function build(): string {
/// @type.symbol symbol=build type=() => string

    let output = "";
    /// @type.symbol symbol=build.output source=output type=string
    /// @resolution.pattern source=output kind=binding target=build.output

    output += "x";
    /// @resolution.name source=output target=build.output
    /// @resolution.operator source="output += \"x\"" type=^string operator="+" kind=call parameters=(string) arguments=(provided("x") as string) return=^string regions=("managed" & "local") kind=symbol target=add receiver=string adjustments=(borrow(&'managed readonly string)) instance="string.<extension#2>.add<\"managed\" & \"local\">"
    /// @resolution.pattern.assign source=output kind=place
    /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
    /// @resolution.assignment source=output read=binding(build.output) write=binding(build.output) type=string
    /// @resolution.access source=output root=build.output
    /// @generic.instantiation id="add<\"managed\" & \"local\">" template=add arguments=("managed" & "local")

    return output;
    /// @resolution.name source=output target=build.output
    /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=output root=build.output

}
"#,
        r#"
"#,
    );
}

/// Preserve copyability when readonly access qualifies values and managed references.
#[test]
fn test_preserve_copyability_through_readonly() {
    let session = TestSession::single(
        r#"
import { Copy } from "destack:memory";

struct Buffer {
    storage: ^[int32];
}

declare class Object {
    storage: ^[int32];
}

declare function requireCopy<T: Copy>(): void;

requireCopy<readonly int32>();
requireCopy<readonly Object>();
requireCopy<readonly ^[int32]>();
requireCopy<readonly Buffer>();
requireCopy<readonly ^Object>();
"#,
    );

    session.assert_diagnostics(
        session.dir_checked_key("main.ds"),
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'readonly ^Slice<int32>' does not satisfy 'Copy'"
/// @diagnostic.label line=16 column=1 span="requireCopy<readonly ^[int32]>()" line_source="requireCopy<readonly ^[int32]>();"
/// @diagnostic.related line=12 column=30 span="T" line_source="declare function requireCopy<T: Copy>(): void;" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type 'readonly Buffer' does not satisfy 'Copy'"
/// @diagnostic.label line=17 column=1 span="requireCopy<readonly Buffer>()" line_source="requireCopy<readonly Buffer>();"
/// @diagnostic.related line=12 column=30 span="T" line_source="declare function requireCopy<T: Copy>(): void;" message="required by this bound on 'T'"
/// @diagnostic.error id=constraint-not-satisfied message="type 'readonly ^Object' does not satisfy 'Copy'"
/// @diagnostic.label line=18 column=1 span="requireCopy<readonly ^Object>()" line_source="requireCopy<readonly ^Object>();"
/// @diagnostic.related line=12 column=30 span="T" line_source="declare function requireCopy<T: Copy>(): void;" message="required by this bound on 'T'"
"#,
    );
}

/// An annotation `&exclusive ^T` on a class names the same borrow as `&exclusive T`.
#[test]
fn test_normalize_an_exclusive_borrow_of_an_owned_class_form() {
    let session = TestSession::single(
        r#"
class User {
    name: int32 = 0;
}

function edit(user: ^User): void {
    const plain: &exclusive User = &exclusive user;
    const owned: &exclusive ^User = &exclusive user;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    name: int32 = 0;
}

function edit(user: ^User): void {
    const plain: &'frame exclusive User = &exclusive user;
    const owned: &'frame exclusive User = &exclusive user;
}

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: int32 = 0" key=name type=int32

    name: int32 = 0;
    /// @type.symbol symbol=User.name source="name: int32 = 0" type=int32

}

function edit(user: ^User): void {
/// @type.symbol symbol=edit type=(^User) => void
/// @type.symbol symbol=edit.user source="user: ^User" type=^User
/// @resolution.name source=User target=User

    const plain: &exclusive User = &exclusive user;
    /// @type.symbol symbol=edit.plain source=plain type=&'frame exclusive User
    /// @resolution.pattern source=plain kind=binding target=edit.plain
    /// @resolution.name source=User target=User
    /// @resolution.name source=user target=edit.user
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=edit.user

    const owned: &exclusive ^User = &exclusive user;
    /// @type.symbol symbol=edit.owned source=owned type=&'frame exclusive User
    /// @resolution.pattern source=owned kind=binding target=edit.owned
    /// @resolution.name source=User target=User
    /// @resolution.name source=user target=edit.user
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=edit.user

}
"#,
        r#"
"#,
    );
}

/// Reject an owned construction of a class whose constructor receives a managed-only `this`.
#[test]
fn test_reject_owned_construction_through_a_managed_only_constructor() {
    let session = TestSession::single(
        r#"
class Registry {
    static register(item: Widget): void {}
    static keep(item: Deferred): void {}
    static hold(item: Placed): void {}
}

class Placed {
    constructor() {
        Registry.hold(this);
    }
}

class Settled<T: Copy> {
    constructor(value: T) {
        const settle = (item: T) => {
            Settled.fulfill(this, item);
        };
        settle(value);
    }

    private static fulfill<T: Copy>(settled: Settled<T>, value: T): void {}
}

class Widget {
    constructor() {
        Registry.register(this);
    }
}

class Deferred {
    constructor() {
        const settle = () => {
            Registry.keep(this);
        };
        settle();
    }
}

class Plain {
    value: int32 = 0;
}

class Point {
    x: int32 = 0;

    constructor(&exclusive this, x: int32) {
        this.x = x;
    }
}

function make(): void {
    const handle = new Widget();
    const owned: ^Widget = new Widget();
    const deferred: ^Deferred = new Deferred();
    const placed: ^Placed = new Placed();
    const settled: ^Settled<int32> = new Settled(1);
    const plain: ^Plain = new Plain();
    const point: ^Point = new Point(1);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Registry {
    static register(item: Widget): void {}
    static keep(item: Deferred): void {}
    static hold(item: Placed): void {}
}

class Placed {
    constructor() {
        Registry.hold(this as Placed);
    }
}

class Settled<T: Copy> {
    constructor(value: T) {
        const settle: (item: T) => void = (item: T): void => {
            Settled.fulfill<T>(this as Settled<T>, item);
        };
        settle(value);
    }

    private static fulfill<T: Copy>(settled: Settled<T>, value: T): void {}
}

class Widget {
    constructor() {
        Registry.register(this as Widget);
    }
}

class Deferred {
    constructor() {
        const settle: () => void = (): void => {
            Registry.keep(this as Deferred);
        };
        settle();
    }
}

class Plain {
    value: int32 = 0;
}

class Point {
    x: int32 = 0;

    constructor(&exclusive this, x: int32) {
        this.x = x;
    }
}

function make(): void {
    const handle: Widget = new Widget();
    const owned: ^Widget = new Widget();
    const deferred: ^Deferred = new Deferred();
    const placed: ^Placed = new Placed();
    const settled: ^Settled<int32> = new Settled<int32>(1);
    const plain: ^Plain = new Plain();
    const point: ^Point = new Point(1);
}

=== dir ===
class Registry {
/// @type.symbol symbol=Registry type=typeof Registry
/// @definition.class symbol=Registry
/// @definition.method symbol=Registry.hold source="static hold(item: Placed): void {}" slot=hold static=true type=(Placed) => void
/// @definition.method symbol=Registry.keep source="static keep(item: Deferred): void {}" slot=keep static=true type=(Deferred) => void
/// @definition.method symbol=Registry.register source="static register(item: Widget): void {}" slot=register static=true type=(Widget) => void

    static register(item: Widget): void {}
    /// @type.symbol symbol=Registry.register source="static register(item: Widget): void {}" type=(Widget) => void
    /// @type.symbol symbol=Registry.register.item source="item: Widget" type=Widget
    /// @resolution.name source=Widget target=Widget

    static keep(item: Deferred): void {}
    /// @type.symbol symbol=Registry.keep source="static keep(item: Deferred): void {}" type=(Deferred) => void
    /// @type.symbol symbol=Registry.keep.item source="item: Deferred" type=Deferred
    /// @resolution.name source=Deferred target=Deferred

    static hold(item: Placed): void {}
    /// @type.symbol symbol=Registry.hold source="static hold(item: Placed): void {}" type=(Placed) => void
    /// @type.symbol symbol=Registry.hold.item source="item: Placed" type=Placed
    /// @resolution.name source=Placed target=Placed

}

class Placed {
/// @type.symbol symbol=Placed type=typeof Placed
/// @definition.class symbol=Placed
/// @definition.method symbol=Placed.constructor slot=constructor role=constructor type=(this: &'managed Placed) => Placed

    constructor() {
    /// @type.symbol symbol=Placed.constructor type=(this: &'managed Placed) => Placed
    /// @type.symbol symbol=Placed.constructor.this type=&'managed Placed

        Registry.hold(this);
        /// @resolution.name source=Registry target=Registry
        /// @resolution.member source=Registry.hold receiver=typeof Registry type=(Placed) => void kind=symbol target_receiver=typeof Registry target=Registry.hold
        /// @resolution.call source=Registry.hold(this) parameters=(Placed) arguments=(provided(this) as Placed) return=void kind=symbol target=Registry.hold
        /// @resolution.receiver source=this kind=this declaration=Placed type=&'managed Placed
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this

    }
}

class Settled<T: Copy> {
/// @generic.template symbol=Settled parameters=(T#1: Copy)
/// @type.symbol symbol=Settled type=typeof Settled
/// @definition.class symbol=Settled template=(T#1: Copy)
/// @definition.method symbol=Settled.constructor slot=constructor role=constructor type=(this: &'managed Settled<T#1>, T#1) => Settled<T#1>
/// @definition.method symbol=Settled.fulfill source="private static fulfill<T: Copy>(settled: Settled<T>, value: T): void {}" slot=fulfill static=true visibility=private type=<T#2: Copy>(Settled<T#2>, T#2) => void
/// @type.symbol symbol=Settled.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=Copy

    constructor(value: T) {
    /// @type.symbol symbol=Settled.constructor type=(this: &'managed Settled<T#1>, T#1) => Settled<T#1>
    /// @type.symbol symbol=Settled.constructor.this type=&'managed Settled<T#1>
    /// @type.symbol symbol=Settled.constructor.value source="value: T" type=T#1
    /// @resolution.name source=T target=Settled.T

        const settle = (item: T) => {
        /// @type.symbol symbol=Settled.constructor.settle source=settle type=Function<(T#1,), void, "readonly">
        /// @resolution.pattern source=settle kind=binding target=Settled.constructor.settle
        /// @type.symbol symbol=Settled.constructor.symbol16 type=Function<(T#1,), void, "readonly">
        /// @type.symbol symbol=Settled.constructor.symbol16.item source="item: T" type=T#1
        /// @resolution.name source=T target=Settled.T

            Settled.fulfill(this, item);
            /// @resolution.name source=Settled target=Settled
            /// @resolution.member source=Settled.fulfill receiver=typeof Settled type=<T#2: Copy>(Settled<T#2>, T#2) => void kind=symbol target_receiver=typeof Settled target=Settled.fulfill
            /// @resolution.call source="Settled.fulfill(this, item)" parameters=(Settled<T#1>, T#1) arguments=(provided(this) as Settled<T#1>, provided(item) as T#1) return=void kind=symbol target=Settled.fulfill instance=Settled.fulfill<T#1>
            /// @generic.instantiation id=Settled.fulfill<T#1> template=Settled.fulfill arguments=(T#1) owner=Settled.constructor
            /// @resolution.name source=this target=Settled.constructor.this
            /// @resolution.receiver source=this kind=this declaration=Settled type=&'managed Settled<T#1>
            /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
            /// @resolution.access source=this root=this
            /// @resolution.name source=item target=Settled.constructor.symbol16.item
            /// @resolution.place source=item placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=item root=Settled.constructor.symbol16.item

        };
        settle(value);
        /// @resolution.name source=settle target=Settled.constructor.settle
        /// @resolution.call source=settle(value) parameters=(T#1) arguments=(provided(value) as T#1) return=void kind=expression target=expression
        /// @resolution.place source=settle placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=settle root=Settled.constructor.settle
        /// @resolution.name source=value target=Settled.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Settled.constructor.value

    }

    private static fulfill<T: Copy>(settled: Settled<T>, value: T): void {}
    /// @generic.template symbol=Settled.fulfill parent=template#0 parameters=(T#2: Copy)
    /// @type.symbol symbol=Settled.fulfill source="private static fulfill<T: Copy>(settled: Settled<T>, value: T): void {}" type=<T#2: Copy>(Settled<T#2>, T#2) => void
    /// @type.symbol symbol=Settled.fulfill.T source="T: Copy" type=T#2
    /// @resolution.name source=Copy target=Copy
    /// @type.symbol symbol=Settled.fulfill.settled source="settled: Settled<T>" type=Settled<T#2>
    /// @resolution.name source=Settled target=Settled
    /// @resolution.name source=T target=Settled.fulfill.T
    /// @type.symbol symbol=Settled.fulfill.value source="value: T" type=T#2
    /// @resolution.name source=T target=Settled.fulfill.T

}

class Widget {
/// @type.symbol symbol=Widget type=typeof Widget
/// @definition.class symbol=Widget
/// @definition.method symbol=Widget.constructor slot=constructor role=constructor type=(this: &'managed Widget) => Widget

    constructor() {
    /// @type.symbol symbol=Widget.constructor type=(this: &'managed Widget) => Widget
    /// @type.symbol symbol=Widget.constructor.this type=&'managed Widget

        Registry.register(this);
        /// @resolution.name source=Registry target=Registry
        /// @resolution.member source=Registry.register receiver=typeof Registry type=(Widget) => void kind=symbol target_receiver=typeof Registry target=Registry.register
        /// @resolution.call source=Registry.register(this) parameters=(Widget) arguments=(provided(this) as Widget) return=void kind=symbol target=Registry.register
        /// @resolution.receiver source=this kind=this declaration=Widget type=&'managed Widget
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this

    }
}

class Deferred {
/// @type.symbol symbol=Deferred type=typeof Deferred
/// @definition.class symbol=Deferred
/// @definition.method symbol=Deferred.constructor slot=constructor role=constructor type=(this: &'managed Deferred) => Deferred

    constructor() {
    /// @type.symbol symbol=Deferred.constructor type=(this: &'managed Deferred) => Deferred
    /// @type.symbol symbol=Deferred.constructor.this type=&'managed Deferred

        const settle = () => {
        /// @type.symbol symbol=Deferred.constructor.settle source=settle type=Function<(), void, "readonly">
        /// @resolution.pattern source=settle kind=binding target=Deferred.constructor.settle
        /// @type.symbol symbol=Deferred.constructor.symbol29 type=Function<(), void, "readonly">

            Registry.keep(this);
            /// @resolution.name source=Registry target=Registry
            /// @resolution.member source=Registry.keep receiver=typeof Registry type=(Deferred) => void kind=symbol target_receiver=typeof Registry target=Registry.keep
            /// @resolution.call source=Registry.keep(this) parameters=(Deferred) arguments=(provided(this) as Deferred) return=void kind=symbol target=Registry.keep
            /// @resolution.name source=this target=Deferred.constructor.this
            /// @resolution.receiver source=this kind=this declaration=Deferred type=&'managed Deferred
            /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
            /// @resolution.access source=this root=this

        };
        settle();
        /// @resolution.name source=settle target=Deferred.constructor.settle
        /// @resolution.call source=settle() parameters=() return=void kind=expression target=expression
        /// @resolution.place source=settle placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=settle root=Deferred.constructor.settle

    }
}

class Plain {
/// @type.symbol symbol=Plain type=typeof Plain
/// @definition.class symbol=Plain
/// @definition.field symbol=Plain.value source="value: int32 = 0" key=value type=int32

    value: int32 = 0;
    /// @type.symbol symbol=Plain.value source="value: int32 = 0" type=int32

}

class Point {
/// @type.symbol symbol=Point type=typeof Point
/// @definition.class symbol=Point
/// @definition.field symbol=Point.x source="x: int32 = 0" key=x type=int32
/// @definition.method symbol=Point.constructor slot=constructor role=constructor type=<Point.constructor.'a>(this: &Point.constructor.'a exclusive Point, int32) => Point

    x: int32 = 0;
    /// @type.symbol symbol=Point.x source="x: int32 = 0" type=int32

    constructor(&exclusive this, x: int32) {
    /// @generic.template symbol=Point.constructor parameters=('a)
    /// @type.symbol symbol=Point.constructor type=<Point.constructor.'a>(this: &Point.constructor.'a exclusive Point, int32) => Point
    /// @type.symbol symbol=Point.constructor.this source="&exclusive this" type=&Point.constructor.'a exclusive Point
    /// @type.symbol symbol=Point.constructor.x source="x: int32" type=int32

        this.x = x;
        /// @resolution.receiver source=this kind=this declaration=Point type=&Point.constructor.'a exclusive Point
        /// @resolution.place source=this placement=Point.constructor.'a lifetime=Point.constructor.'a access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.x kind=place
        /// @resolution.place source=this.x placement=Point.constructor.'a lifetime=Point.constructor.'a access="exclusive"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @resolution.assignment source=this.x write="receiver=&Point.constructor.'a exclusive Point, target=field(receiver=&Point.constructor.'a exclusive Point, target=Point.x, type=int32), type=int32" type=int32
        /// @resolution.name source=x target=Point.constructor.x
        /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=x root=Point.constructor.x

    }
}

function make(): void {
/// @type.symbol symbol=make type=() => void

    const handle = new Widget();
    /// @type.symbol symbol=make.handle source=handle type=Widget
    /// @resolution.pattern source=handle kind=binding target=make.handle
    /// @resolution.construct source="new Widget()" parameters=() return=Widget kind=class target=Widget constructor=Widget.constructor
    /// @resolution.name source=Widget target=Widget

    const owned: ^Widget = new Widget();
    /// @type.symbol symbol=make.owned source=owned type=^Widget
    /// @resolution.pattern source=owned kind=binding target=make.owned
    /// @resolution.name source=Widget target=Widget
    /// @resolution.construct source="new Widget()" parameters=() return=^Widget kind=class target=Widget constructor=Widget.constructor
    /// @resolution.name source=Widget target=Widget

    const deferred: ^Deferred = new Deferred();
    /// @type.symbol symbol=make.deferred source=deferred type=^Deferred
    /// @resolution.pattern source=deferred kind=binding target=make.deferred
    /// @resolution.name source=Deferred target=Deferred
    /// @resolution.construct source="new Deferred()" parameters=() return=^Deferred kind=class target=Deferred constructor=Deferred.constructor
    /// @resolution.name source=Deferred target=Deferred

    const placed: ^Placed = new Placed();
    /// @type.symbol symbol=make.placed source=placed type=^Placed
    /// @resolution.pattern source=placed kind=binding target=make.placed
    /// @resolution.name source=Placed target=Placed
    /// @resolution.construct source="new Placed()" parameters=() return=^Placed kind=class target=Placed constructor=Placed.constructor
    /// @resolution.name source=Placed target=Placed

    const settled: ^Settled<int32> = new Settled(1);
    /// @type.symbol symbol=make.settled source=settled type=^Settled<int32>
    /// @resolution.pattern source=settled kind=binding target=make.settled
    /// @resolution.name source=Settled target=Settled
    /// @resolution.construct source="new Settled(1)" parameters=(int32) arguments=(provided(1) as int32) return=^Settled<int32> kind=class target=Settled constructor=Settled.constructor instance=Settled<int32>
    /// @generic.instantiation id=Settled.constructor<int32> template=Settled.constructor arguments=(int32)
    /// @generic.instantiation id=Settled<int32> template=Settled arguments=(int32)
    /// @resolution.name source=Settled target=Settled

    const plain: ^Plain = new Plain();
    /// @type.symbol symbol=make.plain source=plain type=^Plain
    /// @resolution.pattern source=plain kind=binding target=make.plain
    /// @resolution.name source=Plain target=Plain
    /// @resolution.construct source="new Plain()" parameters=() return=^Plain kind=class target=Plain constructor=default
    /// @resolution.name source=Plain target=Plain

    const point: ^Point = new Point(1);
    /// @type.symbol symbol=make.point source=point type=^Point
    /// @resolution.pattern source=point kind=binding target=make.point
    /// @resolution.name source=Point target=Point
    /// @resolution.construct source="new Point(1)" parameters=(int32) arguments=(provided(1) as int32) return=^Point regions=("managed" & "local") kind=class target=Point constructor=Point.constructor call="Point.constructor<\"managed\" & \"local\">"
    /// @generic.instantiation id="Point.constructor<\"managed\" & \"local\">" template=Point.constructor arguments=("managed" & "local")
    /// @resolution.name source=Point target=Point

}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '^Widget' is not assignable to the method's 'this' type '&'managed Widget'"
/// @diagnostic.label line=54 column=28 span="new Widget()" line_source="const owned: ^Widget = new Widget();"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '^Deferred' is not assignable to the method's 'this' type '&'managed Deferred'"
/// @diagnostic.label line=55 column=33 span="new Deferred()" line_source="const deferred: ^Deferred = new Deferred();"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '^Placed' is not assignable to the method's 'this' type '&'managed Placed'"
/// @diagnostic.label line=56 column=29 span="new Placed()" line_source="const placed: ^Placed = new Placed();"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '^Settled<int32>' is not assignable to the method's 'this' type '&'managed Settled<T>'"
/// @diagnostic.label line=57 column=38 span="new Settled(1)" line_source="const settled: ^Settled<int32> = new Settled(1);"
"#,
    );
}

/// Reject an owned construction of an imported class through a managed-only constructor.
#[test]
fn test_reject_owned_construction_of_an_imported_class_through_a_managed_only_constructor() {
    let session = TestSession::builder()
        .module(
            "lib.ds",
            r#"
class Registry {
    static register(item: Widget): void {}
}

export class Widget {
    constructor() {
        Registry.register(this);
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Widget } from "./lib.ds";

function make(): void {
    const owned: ^Widget = new Widget();
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
import { Widget } from "./lib.ds";

function make(): void {
    const owned: ^Widget = new Widget();
}

=== dir ===
import { Widget } from "./lib.ds";

function make(): void {
    const owned: ^Widget = new Widget();
}
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '^lib.Widget' is not assignable to the method's 'this' type '&'managed lib.Widget'"
/// @diagnostic.label line=5 column=28 span="new Widget()" line_source="const owned: ^Widget = new Widget();"
"#,
    );
}

/// Reject escaping `this` from a constructor that borrows its receiver.
#[test]
fn test_reject_escaping_a_borrowed_constructor_receiver() {
    let session = TestSession::single(
        r#"
class Registry {
    static register(item: Widget): void {}
}

class Widget {
    constructor(&exclusive this) {
        Registry.register(this);
    }
}
"#,
    );

    session.assert_mir_verified_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=borrow-outlives-origin message="borrow does not live long enough"
/// @diagnostic.label line=8 column=27 span="this" line_source="Registry.register(this);"
"#,
    );
}
