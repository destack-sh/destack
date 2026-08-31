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

declare function consume(value: Payload): void;

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
/// @type.symbol symbol=consume.value source="value: ^Payload" type=Payload
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
/// @resolution.place source=payload placement="constant" lifetime="static" access="readonly"
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
    /// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
    /// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)

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
/// @type.symbol symbol=User source="class User {}" type=User
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
/// @resolution.place source=container placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=container root=container
/// @resolution.place source=container.data placement="constant" lifetime="static" access="readonly"
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
/// @type.symbol symbol=user source=user type=Readonly<User>
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User
/// @type.node type=Readonly<User>
/// @resolution.name source=User target=User

    profile: Profile { name: "Ada" },
    /// @type.node source="Profile { name: \"Ada\" }" type=Profile
    /// @resolution.name source=Profile target=Profile
    /// @type.node source="\"Ada\"" type="Ada"

};

user.profile.name = "Grace";
/// @type.node source="user.profile.name = \"Grace\"" type="Grace"
/// @type.node source=user type=Readonly<User>
/// @type.node source=user.profile type=Profile
/// @type.node source=user.profile.name type=Readonly<string>
/// @resolution.name source=user target=user
/// @resolution.member source=user.profile receiver=Readonly<User> type=Profile kind=field target_receiver=Readonly<User> key=profile target=User.profile target_type=Profile
/// @resolution.place source=user placement="local" lifetime="static" access="readonly"
/// @resolution.access source=user root=user
/// @resolution.place source=user.profile placement="local" lifetime="static" access="readonly"
/// @resolution.access source=user.profile root=user keys=[profile]
/// @resolution.pattern.assign source=user.profile.name kind=place
/// @resolution.access source=user.profile.name root=user keys=[profile, name]
/// @resolution.assignment source=user.profile.name write="receiver=Readonly<Profile>, target=field(receiver=Readonly<Profile>, target=Profile.name, type=Readonly<string>), type=Readonly<string>" type=Readonly<string>
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
/// @type.symbol symbol=Pair type=Pair
/// @definition.class symbol=Pair
/// @definition.field symbol=Pair.left source="left: int32 = 0" key=left type=int32
/// @definition.field symbol=Pair.right source="right: int32 = 0" key=right type=int32

    left: int32 = 0;
    /// @type.symbol symbol=Pair.left source="left: int32 = 0" type=int32

    right: int32 = 0;
    /// @type.symbol symbol=Pair.right source="right: int32 = 0" type=int32

}

class Named {
/// @type.symbol symbol=Named type=Named
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
/// @resolution.place source=point placement="constant" lifetime="static" access="readonly"
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
/// @resolution.place source=pair placement="constant" lifetime="static" access="readonly"
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
/// @resolution.place source=object placement="constant" lifetime="static" access="readonly"
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
/// @resolution.place source=named placement="constant" lifetime="static" access="readonly"
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
/// @resolution.place source=values placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=values root=values

declare const slice: ^[int32];
/// @type.symbol symbol=slice source=slice type=^Slice<int32>
/// @resolution.pattern source=slice kind=binding target=slice

witness(slice);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(slice) parameters=(^Slice<int32>) arguments=(provided(slice) as ^Slice<int32>) return=^Slice<int32> kind=symbol target=witness instance=witness<^Slice<int32>>
/// @generic.instantiation id=witness<^Slice<int32>> template=witness arguments=(^Slice<int32>)
/// @resolution.name source=slice target=slice
/// @resolution.place source=slice placement="local" lifetime="static" access="readonly"
/// @resolution.access source=slice root=slice

declare const callable: ^(() => void);
/// @type.symbol symbol=callable source=callable type=^Function<(), void>
/// @resolution.pattern source=callable kind=binding target=callable

witness(callable);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(callable) parameters=(^Function<(), void>) arguments=(provided(callable) as ^Function<(), void>) return=^Function<(), void> kind=symbol target=witness instance="witness<^Function<(), void>>"
/// @generic.instantiation id="witness<^Function<(), void>>" template=witness arguments=(^Function<(), void>)
/// @resolution.name source=callable target=callable
/// @resolution.place source=callable placement="local" lifetime="static" access="readonly"
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
/// @resolution.place source=dynamic placement="local" lifetime="static" access="readonly"
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
/// @resolution.place source=buffer placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=buffer root=buffer

declare const text: ^string;
/// @type.symbol symbol=text source=text type=^string
/// @resolution.pattern source=text kind=binding target=text

witness(text);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(text) parameters=(^string) arguments=(provided(text) as ^string) return=^string kind=symbol target=witness instance=witness<^string>
/// @generic.instantiation id=witness<^string> template=witness arguments=(^string)
/// @resolution.name source=text target=text
/// @resolution.place source=text placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=text root=text

declare const big: ^bigint;
/// @type.symbol symbol=big source=big type=^bigint
/// @resolution.pattern source=big kind=binding target=big

witness(big);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(big) parameters=(^bigint) arguments=(provided(big) as ^bigint) return=^bigint kind=symbol target=witness instance=witness<^bigint>
/// @generic.instantiation id=witness<^bigint> template=witness arguments=(^bigint)
/// @resolution.name source=big target=big
/// @resolution.place source=big placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=big root=big
"#,
        r#"

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

const bag: Bag<int32> = gather<^Bag<int32>>() as Bag<int32>;

=== dir ===
newtype interface Collect<T> {
/// @generic.template symbol=Collect parameters=(in T#1)
/// @type.symbol symbol=Collect type=Collect
/// @definition.interface symbol=Collect template=(in T#1) nominal=true
/// @definition.where symbol=Collect relation=satisfies left=this right=Collect<T#1>
/// @definition.method symbol=Collect.add source="add(value: T): void" slot=add type=(this: this, T#1) => void
/// @type.symbol symbol=Collect.T source=T type=T#1

    add(value: T): void;
    /// @type.symbol symbol=Collect.add source="add(value: T): void" type=(this: this, T#1) => void
    /// @type.symbol symbol=Collect.add.value source="value: T" type=T#1
    /// @resolution.name source=T target=Collect.T

}

class Bag<T> {
/// @generic.template symbol=Bag parameters=(in out T#2)
/// @type.symbol symbol=Bag type=Bag
/// @definition.class symbol=Bag template=(in out T#2)
/// @definition.field symbol=Bag.last source="last: T | undefined" key=last type=T#2 | undefined
/// @definition.method symbol=Bag.constructor slot=constructor role=constructor type=<Bag.constructor.P0: Place>() => Managed<this, Bag.constructor.P0>
/// @type.symbol symbol=Bag.T source=T type=T#2

    last: T | undefined;
    /// @type.symbol symbol=Bag.last source="last: T | undefined" type=T#2 | undefined
    /// @resolution.name source=T target=Bag.T

    constructor() {
    /// @generic.template symbol=Bag.constructor parent=template#1 parameters=(P0: Place)
    /// @type.symbol symbol=Bag.constructor type=<Bag.constructor.P0: Place>() => Managed<this, Bag.constructor.P0>
    /// @type.symbol symbol=Bag.constructor.this type=Bag<T#2>

        this.last = undefined;
        /// @type.node source="this.last = undefined" type=undefined
        /// @type.node source=this type=Bag<T#2>
        /// @type.node source=this.last type=T#2 | undefined
        /// @resolution.receiver source=this kind=this declaration=Bag type=Bag<T#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.last kind=place
        /// @resolution.access source=this.last root=this keys=[last]
        /// @resolution.assignment source=this.last write="receiver=Bag<T#2>, target=field(receiver=Bag<T#2>, target=Bag.last, type=T#2 | undefined), type=T#2 | undefined" type=T#2 | undefined
        /// @type.node source=undefined type=undefined

    }
}

extension<T> of ^Bag<T> implements Collect<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=local target=^Bag<T#3>
/// @definition.implements symbol=<module>#2 source=Collect<T> target=Collect<T#3>
/// @definition.method symbol=add source="add(value: T): void {}" slot=add type=(this: this, T#3) => void
/// @definition.conformance symbol=<module>#2 member=add requirement=Collect.add
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=T target=T
/// @resolution.name source=Collect target=Collect
/// @resolution.name source=T target=T

    add(value: T): void {}
    /// @type.symbol symbol=add source="add(value: T): void {}" type=(this: this, T#3) => void
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
/// @type.symbol symbol=bag source=bag type=Bag<int32>
/// @resolution.pattern source=bag kind=binding target=bag
/// @generic.instance id=Bag<int32> template=Bag arguments=(int32)
/// @type.node source=gather type=() => ^Bag<int32>
/// @type.node source=gather<^Bag<int32>>() type=^Bag<int32>
/// @resolution.name source=gather target=gather
/// @resolution.call source=gather<^Bag<int32>>() parameters=() return=^Bag<int32> kind=symbol target=gather instance=gather<^Bag<int32>>
/// @generic.instantiation id=gather<^Bag<int32>> template=gather arguments=(^Bag<int32>)
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
/// @generic.template symbol=Collect parameters=(in T)
/// @type.symbol symbol=Collect type=Collect
/// @definition.interface symbol=Collect template=(in T) nominal=true
/// @definition.where symbol=Collect relation=satisfies left=this right=Collect<T>
/// @definition.method symbol=Collect.add source="add(value: T): void" slot=add type=(this: this, T) => void
/// @type.symbol symbol=Collect.T source=T type=T

    add(value: T): void;
    /// @type.symbol symbol=Collect.add source="add(value: T): void" type=(this: this, T) => void
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

const values: Deque<int64> = Deque.from<int64>([1, 2, 3] as Iterable<int64>) as Deque<int64>;

=== dir ===
import { Deque } from "destack:collections";

const values = Deque.from([1, 2, 3]);
/// @type.symbol symbol=values source=values type=Deque<int64>
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Deque<int64> template=Deque arguments=(int64)
/// @generic.instance id=MaybeUninit<int64> template=MaybeUninit arguments=(int64)
/// @generic.instance id=new<MaybeUninit<int64>> template=new arguments=(MaybeUninit<int64>)
/// @resolution.name source=Deque target=Deque
/// @resolution.member source=Deque.from receiver=Deque type=(Iterable<T#4>) => ^Deque<T#4> kind=symbol target_receiver=Deque target=from#1
/// @resolution.call source="Deque.from([1, 2, 3])" parameters=(Iterable<int64>) arguments=(provided([1, 2, 3]) as Iterable<int64>) return=^Deque<int64> kind=symbol target=from#1 instance=Deque<int64>.<extension#4>.from#1
/// @generic.instantiation id=from#1<int64> template=from#1 arguments=(int64)
/// @generic.instance id="DropIterator<Iterator<int64>, int64>" template=DropIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="DropWhileIterator<Iterator<int64>, int64>" template=DropWhileIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="EnumeratedIterator<Iterator<int64>, int64>" template=EnumeratedIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="FilterIterator<Iterator<int64>, int64>" template=FilterIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="InspectIterator<Iterator<int64>, int64>" template=InspectIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="Iterator.collect<Iterator<int64>, int64, ^int64[]>" template=Iterator.collect arguments=(int64, ^int64[])
/// @generic.instance id="IteratorResult<int64, void>" template=IteratorResult arguments=(int64, void)
/// @generic.instance id="PeekableIterator<Iterator<int64>, int64>" template=PeekableIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="TakeIterator<Iterator<int64>, int64>" template=TakeIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id="TakeWhileIterator<Iterator<int64>, int64>" template=TakeWhileIterator arguments=(Iterator<int64>, int64)
/// @generic.instance id=FromIterator.fromIterator<int64> template=FromIterator.fromIterator arguments=(int64)
/// @generic.instance id=Iterable<int64> template=Iterable arguments=(int64)
/// @generic.instance id=Iterator<int64> template=Iterator arguments=(int64) evaluated=(<Iterator.flatMap.U, Iterator.flatMap.V: Iterable<Iterator.flatMap.U>>(this: this, Function<(Iterator.T, isize), Iterator.flatMap.V>) => FlatMapIterator<this, Iterator.T, Iterator.flatMap.V, Iterator.flatMap.V.Iterator, Iterator.flatMap.U> => <Iterator.flatMap.U, Iterator.flatMap.V: Iterable<Iterator.flatMap.U>>(this: Iterator<int64>, Function<(int64, isize), Iterator.flatMap.V>) => FlatMapIterator<Iterator<int64>, int64, Iterator.flatMap.V, Iterator.flatMap.V.Iterator, Iterator.flatMap.U>, <Iterator.chain.V: Iterable<Iterator.T>>(this: this, Iterator.chain.V) => ChainIterator<this, Iterator.chain.V.Iterator, Iterator.T> => <Iterator.chain.V: Iterable<Iterator.T>>(this: Iterator<int64>, Iterator.chain.V) => ChainIterator<Iterator<int64>, Iterator.chain.V.Iterator, int64>, <Iterator.zip.U, Iterator.zip.V: Iterable<Iterator.zip.U>>(this: this, Iterator.zip.V) => ZipIterator<this, Iterator.zip.V.Iterator, Iterator.T, Iterator.zip.U> => <Iterator.zip.U, Iterator.zip.V: Iterable<Iterator.zip.U>>(this: Iterator<int64>, Iterator.zip.V) => ZipIterator<Iterator<int64>, Iterator.zip.V.Iterator, int64, Iterator.zip.U>, <Iterator.tryFold.F: Try>(this: this, Iterator.tryFold.F.Output, Function<(Iterator.tryFold.F.Output, Iterator.T, isize), Iterator.tryFold.F>) => Iterator.tryFold.F => <Iterator.tryFold.F: Try>(this: Iterator<int64>, Iterator.tryFold.F.Output, Function<(Iterator.tryFold.F.Output, int64, isize), Iterator.tryFold.F>) => Iterator.tryFold.F)
/// @generic.instance id=IteratorReturn<void> template=IteratorReturn arguments=(void)
/// @generic.instance id=IteratorYield<int64> template=IteratorYield arguments=(int64)
/// @generic.instance id=from#1<int64> template=from#1 arguments=(int64)
/// @resolution.call source=[1, 2, 3] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2, 3) as int64) return=int64[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int64>
/// @generic.instantiation id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
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
/// @definition.method symbol=Holder.isInline slot=isInline role=getter type=<Holder.isInline.'a>(this: &Holder.isInline.'a readonly this) => boolean
/// @type.symbol symbol=Holder.T source=T type=T

    private storage: ^[T] | undefined = undefined;
    /// @type.symbol symbol=Holder.storage source="private storage: ^[T] | undefined = undefined" type=^Slice<T> | undefined
    /// @resolution.name source=T target=Holder.T

    get isInline(): boolean {
    /// @generic.template symbol=Holder.isInline parent=template#0 parameters=('a)
    /// @type.symbol symbol=Holder.isInline type=<Holder.isInline.'a>(this: &Holder.isInline.'a readonly this) => boolean
    /// @type.symbol symbol=Holder.isInline.this type=&Holder.isInline.'a readonly Holder<T>

        this.storage == undefined
        /// @resolution.member source=this.storage receiver=&Holder.isInline.'a readonly Holder<T> type=Readonly<^Slice<T> | undefined> kind=field target_receiver=&Holder.isInline.'a readonly Holder<T> key=storage target=Holder.storage target_type=Readonly<^Slice<T> | undefined>
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
    /// @resolution.operator source="output += \"x\"" type=^string operator="+" kind=call parameters=(string) arguments=(provided("x") as string) return=^string kind=symbol target=add receiver=string adjustments=(borrow(&'frame readonly string))
    /// @resolution.pattern.assign source=output kind=place
    /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
    /// @resolution.assignment source=output read=binding(build.output) write=binding(build.output) type=string
    /// @resolution.access source=output root=build.output

    return output;
    /// @resolution.name source=output target=build.output
    /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=output root=build.output

}
"#,
        "",
    );
}
