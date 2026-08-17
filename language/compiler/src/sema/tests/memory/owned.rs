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

const payload: ^Payload = ^Payload { value: 1 };
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
/// @type.symbol symbol=consume source="declare function consume(value: ^Payload): void" type=(Owned<Payload>) => void
/// @type.symbol symbol=consume.value source="value: ^Payload" type=Owned<Payload>
/// @resolution.name source=Payload target=Payload

const payload: ^Payload = Payload { value: 1 };
/// @type.symbol symbol=payload source=payload type=Owned<Payload>
/// @resolution.pattern source=payload kind=binding target=payload
/// @resolution.name source=Payload target=Payload
/// @type.node source="Payload { value: 1 }" type=Owned<Payload>
/// @resolution.name source=Payload target=Payload
/// @type.node source=1 type=1

consume(payload);
/// @type.node source=consume type=(Owned<Payload>) => void
/// @type.node source=consume(payload) type=void
/// @resolution.name source=consume target=consume
/// @resolution.call source=consume(payload) parameters=(Owned<Payload>) arguments=(provided(payload) as Owned<Payload>) return=void kind=symbol target=consume
/// @type.node source=payload type=Owned<Payload>
/// @resolution.name source=payload target=payload
/// @resolution.place source=payload placement="local" lifetime="static" access="readonly"
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

let point: ^Point = ^Point { x: 1 };

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
/// @type.symbol symbol=point source=point type=Owned<Point>
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point
/// @type.node source="Point { x: 1 }" type=Owned<Point>
/// @resolution.name source=Point target=Point
/// @type.node source=1 type=1

point satisfies ^Point;
/// @type.node source="point satisfies ^Point" type=Owned<Point>
/// @type.node source=point type=Owned<Point>
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

let buffer: ^Buffer = makeBuffer();

=== dir ===
struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.values source="values: int32[]" key=values type=int32[]

    values: int32[];
    /// @type.symbol symbol=Buffer.values source="values: int32[]" type=int32[]
    /// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
    /// @generic.instance id=memory.init.MaybeUninit<int32> template=memory.init.MaybeUninit arguments=(int32)
    /// @generic.instance id=memory.raw.dangling<memory.init.MaybeUninit<int32>> template=memory.raw.dangling arguments=(memory.init.MaybeUninit<int32>)
    /// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<int32>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<int32>>)
    /// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<int32>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<int32>)
    /// @generic.instance id=memory.unique.emptyUniqueSlice<memory.init.MaybeUninit<int32>> template=memory.unique.emptyUniqueSlice arguments=(memory.init.MaybeUninit<int32>)
    /// @generic.instance id=memory.unique.uniqueSliceFromRaw<memory.init.MaybeUninit<int32>> template=memory.unique.uniqueSliceFromRaw arguments=(memory.init.MaybeUninit<int32>)

}

declare function makeBuffer(): Buffer;
/// @type.symbol symbol=makeBuffer source="declare function makeBuffer(): Buffer" type=() => Buffer
/// @resolution.name source=Buffer target=Buffer

let buffer: ^Buffer = makeBuffer();
/// @type.symbol symbol=buffer source=buffer type=Owned<Buffer>
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
/// @type.symbol symbol=user source=user type=Owned<User>
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
    data: ^Data;
}

const container: Container = Container { data: ^Data { value: 1 } };

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
/// @definition.field symbol=Container.data source="data: ^Data" key=data type=Owned<Data>

    data: ^Data;
    /// @type.symbol symbol=Container.data source="data: ^Data" type=Owned<Data>
    /// @resolution.name source=Data target=Data

}

const container = Container { data: Data { value: 1 } };
/// @type.symbol symbol=container source=container type=Container
/// @resolution.pattern source=container kind=binding target=container
/// @type.node source="Container { data: Data { value: 1 } }" type=Container
/// @resolution.name source=Container target=Container
/// @type.node source="Data { value: 1 }" type=Owned<Data>
/// @resolution.name source=Data target=Data
/// @type.node source=1 type=1

container.data satisfies ^Data;
/// @type.node source="container.data satisfies ^Data" type=Owned<Data>
/// @type.node source=container type=Container
/// @type.node source=container.data type=Owned<Data>
/// @resolution.name source=container target=container
/// @resolution.member source=container.data receiver=Container type=Owned<Data> kind=field target_receiver=Container key=data target=Container.data target_type=Owned<Data>
/// @resolution.place source=container placement="local" lifetime="static" access="readonly"
/// @resolution.access source=container root=container
/// @resolution.place source=container.data placement="local" lifetime="static" access="readonly"
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

let user: ^readonly User = ^readonly User {
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
/// @type.symbol symbol=user source=user type=Owned<Readonly<User>>
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User
/// @type.node type=Owned<Readonly<User>>
/// @resolution.name source=User target=User

    profile: Profile { name: "Ada" },
    /// @type.node source="Profile { name: \"Ada\" }" type=Profile
    /// @resolution.name source=Profile target=Profile
    /// @type.node source="\"Ada\"" type="Ada"

};

user.profile.name = "Grace";
/// @type.node source="user.profile.name = \"Grace\"" type="Grace"
/// @type.node source=user type=Owned<Readonly<User>>
/// @type.node source=user.profile type=Profile
/// @type.node source=user.profile.name type=Readonly<string>
/// @resolution.name source=user target=user
/// @resolution.member source=user.profile receiver=Owned<Readonly<User>> type=Profile kind=field target_receiver=Owned<Readonly<User>> key=profile target=User.profile target_type=Profile
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
fn test_copy_owned_values_through_their_stored_fields() {
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

declare const values: ^Array<int32>;
witness(values);

declare const named: ^Named;
witness(named);
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

declare const point: ^Point;
witness<Point>(point);

declare const pair: ^Pair;
witness<^Pair>(pair);

declare const values: ^int32[];
witness(values);

declare const named: ^Named;
witness<^Named>(named);

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
/// @generic.template symbol=witness parameters=(T: memory.capability.Copy)
/// @type.symbol symbol=witness type=<T: memory.capability.Copy>(T) => T
/// @type.symbol symbol=witness.T source="T: Copy" type=T
/// @resolution.name source=Copy target=memory.capability.Copy
/// @type.symbol symbol=witness.value source="value: T" type=T
/// @resolution.name source=T target=witness.T
/// @resolution.name source=T target=witness.T

    return value;
    /// @resolution.name source=value target=witness.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=witness.value

}

declare const point: ^Point;
/// @type.symbol symbol=point source=point type=Owned<Point>
/// @resolution.pattern source=point kind=binding target=point
/// @resolution.name source=Point target=Point

witness(point);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(point) parameters=(Point) arguments=(provided(point) as Point) return=Point kind=symbol target=witness instance=witness<Point>
/// @generic.instantiation id=witness<Point> template=witness arguments=(Point)
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="readonly"
/// @resolution.access source=point root=point

declare const pair: ^Pair;
/// @type.symbol symbol=pair source=pair type=Owned<Pair>
/// @resolution.pattern source=pair kind=binding target=pair
/// @resolution.name source=Pair target=Pair

witness(pair);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(pair) parameters=(Owned<Pair>) arguments=(provided(pair) as Owned<Pair>) return=Owned<Pair> kind=symbol target=witness instance=witness<Owned<Pair>>
/// @generic.instantiation id=witness<Owned<Pair>> template=witness arguments=(Owned<Pair>)
/// @resolution.name source=pair target=pair
/// @resolution.place source=pair placement="local" lifetime="static" access="readonly"
/// @resolution.access source=pair root=pair

declare const values: ^Array<int32>;
/// @type.symbol symbol=values source=values type=Owned<Array<int32>>
/// @resolution.pattern source=values kind=binding target=values
/// @resolution.name source=Array target=collections.array.Array

witness(values);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(values) parameters=(<error>) arguments=(provided(values) as <error>) return=<error> kind=symbol target=witness instance=witness<<error>>
/// @generic.instantiation id=witness<<error>> template=witness arguments=(<error>)
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="readonly"
/// @resolution.access source=values root=values

declare const named: ^Named;
/// @type.symbol symbol=named source=named type=Owned<Named>
/// @resolution.pattern source=named kind=binding target=named
/// @resolution.name source=Named target=Named

witness(named);
/// @resolution.name source=witness target=witness
/// @resolution.call source=witness(named) parameters=(Owned<Named>) arguments=(provided(named) as Owned<Named>) return=Owned<Named> kind=symbol target=witness instance=witness<Owned<Named>>
/// @generic.instantiation id=witness<Owned<Named>> template=witness arguments=(Owned<Named>)
/// @resolution.name source=named target=named
/// @resolution.place source=named placement="local" lifetime="static" access="readonly"
/// @resolution.access source=named root=named
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type '^int32[]' does not satisfy 'Copy'"
/// @diagnostic.label line=29 column=1 span="witness(values)" line_source="witness(values);"
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

    constructor(): this {
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
/// @definition.method symbol=Bag.constructor slot=constructor role=constructor type=() => this
/// @type.symbol symbol=Bag.T source=T type=T#2

    last: T | undefined;
    /// @type.symbol symbol=Bag.last source="last: T | undefined" type=T#2 | undefined
    /// @resolution.name source=T target=Bag.T

    constructor() {
    /// @type.symbol symbol=Bag.constructor type=() => this

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
/// @definition.extension symbol=<module>#2 form=local target=Owned<Bag<T#3>>
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
/// @type.symbol symbol=bag source=bag type=Owned<Bag<int32>>
/// @resolution.pattern source=bag kind=binding target=bag
/// @generic.instance id=Bag<int32> template=Bag arguments=(int32)
/// @type.node source=gather type=() => Owned<Bag<int32>>
/// @type.node source=gather<^Bag<int32>>() type=Owned<Bag<int32>>
/// @resolution.name source=gather target=gather
/// @resolution.call source=gather<^Bag<int32>>() parameters=() return=Owned<Bag<int32>> kind=symbol target=gather instance=gather<Owned<Bag<int32>>>
/// @generic.instantiation id=gather<Owned<Bag<int32>>> template=gather arguments=(Owned<Bag<int32>>)
/// @generic.instance id=gather<Owned<Bag<int32>>> template=gather arguments=(Owned<Bag<int32>>)
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

const point: ^Point = gather<^Point>();

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
/// @type.symbol symbol=point source=point type=Owned<Point>
/// @resolution.pattern source=point kind=binding target=point
/// @type.node source=gather type=() => Owned<Point>
/// @type.node source=gather<^Point>() type=Owned<Point>
/// @resolution.name source=gather target=gather
/// @resolution.call source=gather<^Point>() parameters=() return=Owned<Point> kind=symbol target=gather instance=gather<Owned<Point>>
/// @generic.instantiation id=gather<Owned<Point>> template=gather arguments=(Owned<Point>)
/// @generic.instance id=gather<Owned<Point>> template=gather arguments=(Owned<Point>)
/// @resolution.name source=Point target=Point
"#,
    );
}
#[test]
fn test_select_creation_statics_ahead_of_conformance_twins() {
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

const values: ^Deque<float64> = Deque.from<float64>([1, 2, 3]);

=== dir ===
import { Deque } from "destack:collections";

const values = Deque.from([1, 2, 3]);
/// @type.symbol symbol=values source=values type=Owned<collections.deque.Deque<float64>>
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=collections.deque.Deque<float64> template=collections.deque.Deque arguments=(float64)
/// @generic.instance id=memory.init.MaybeUninit<float64> template=memory.init.MaybeUninit arguments=(float64)
/// @generic.instance id=memory.raw.dangling<memory.init.MaybeUninit<float64>> template=memory.raw.dangling arguments=(memory.init.MaybeUninit<float64>)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<float64>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<float64>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<float64>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<float64>)
/// @generic.instance id=memory.unique.emptyUniqueSlice<memory.init.MaybeUninit<float64>> template=memory.unique.emptyUniqueSlice arguments=(memory.init.MaybeUninit<float64>)
/// @generic.instance id=memory.unique.uniqueSliceFromRaw<memory.init.MaybeUninit<float64>> template=memory.unique.uniqueSliceFromRaw arguments=(memory.init.MaybeUninit<float64>)
/// @resolution.name source=Deque target=collections.deque.Deque
/// @resolution.member source=Deque.from receiver=collections.deque.Deque type=(Dynamic<iter.iterator.Iterable<collections.deque.T#4>>) => Owned<collections.deque.Deque<collections.deque.T#4>> & (Dynamic<iter.iterator.Iterable<collections.deque.T#5>>) => collections.deque.Deque<collections.deque.T#5> & (Dynamic<iter.iterator.Iterable<collections.deque.T#6>>) => Owned<collections.deque.Deque<collections.deque.T#6>> kind=existential targets=[collections.deque.from#1, collections.deque.from#2, collections.deque.from#3]
/// @resolution.call source="Deque.from([1, 2, 3])" parameters=(Dynamic<iter.iterator.Iterable<float64>>) arguments=(provided([1, 2, 3]) as Dynamic<iter.iterator.Iterable<float64>>) return=Owned<collections.deque.Deque<float64>> kind=symbol target=collections.deque.from#1 instance=collections.deque.Deque<float64>.<extension#4>.from#1
/// @generic.instantiation id=collections.deque.from#1<float64> template=collections.deque.from#1 arguments=(float64)
/// @generic.instance id=collections.deque.from#1<float64> template=collections.deque.from#1 arguments=(float64)
/// @resolution.call source=[1, 2, 3] parameters=(&collections.array.arrayFromSlice.'a readonly Slice<collections.array.arrayFromSlice.T>) arguments=(rest(1, 2, 3) as float64) return=float64[] kind=symbol target=collections.array.arrayFromSlice instance=collections.array.arrayFromSlice<float64>
/// @generic.instantiation id=collections.array.arrayFromSlice<float64> template=collections.array.arrayFromSlice arguments=(float64)
/// @generic.instance id="iter.iterator.DropIterator<iter.iterator.Iterator<float64>, float64>" template=iter.iterator.DropIterator arguments=(iter.iterator.Iterator<float64>, float64)
/// @generic.instance id="iter.iterator.DropWhileIterator<iter.iterator.Iterator<float64>, float64>" template=iter.iterator.DropWhileIterator arguments=(iter.iterator.Iterator<float64>, float64)
/// @generic.instance id="iter.iterator.EnumeratedIterator<iter.iterator.Iterator<float64>, float64>" template=iter.iterator.EnumeratedIterator arguments=(iter.iterator.Iterator<float64>, float64)
/// @generic.instance id="iter.iterator.FilterIterator<iter.iterator.Iterator<float64>, float64>" template=iter.iterator.FilterIterator arguments=(iter.iterator.Iterator<float64>, float64)
/// @generic.instance id="iter.iterator.InspectIterator<iter.iterator.Iterator<float64>, float64>" template=iter.iterator.InspectIterator arguments=(iter.iterator.Iterator<float64>, float64)
/// @generic.instance id="iter.iterator.IteratorResult<float64, iter.iterator.Iterator<float64>.Return>" template=iter.iterator.IteratorResult arguments=(float64, iter.iterator.Iterator<float64>.Return)
/// @generic.instance id="iter.iterator.IteratorResult<float64, void>" template=iter.iterator.IteratorResult arguments=(float64, void)
/// @generic.instance id="iter.iterator.PeekableIterator<iter.iterator.Iterator<float64>, float64>" template=iter.iterator.PeekableIterator arguments=(iter.iterator.Iterator<float64>, float64)
/// @generic.instance id="iter.iterator.TakeIterator<iter.iterator.Iterator<float64>, float64>" template=iter.iterator.TakeIterator arguments=(iter.iterator.Iterator<float64>, float64)
/// @generic.instance id="iter.iterator.TakeWhileIterator<iter.iterator.Iterator<float64>, float64>" template=iter.iterator.TakeWhileIterator arguments=(iter.iterator.Iterator<float64>, float64)
/// @generic.instance id=Array<float64> template=collections.array.Array arguments=(float64)
/// @generic.instance id=collections.array.arrayFromSlice<float64> template=collections.array.arrayFromSlice arguments=(float64)
/// @generic.instance id=iter.iterator.Iterable<float64> template=iter.iterator.Iterable arguments=(float64)
/// @generic.instance id=iter.iterator.Iterator<float64> template=iter.iterator.Iterator arguments=(float64)
/// @generic.instance id=iter.iterator.IteratorReturn<iter.iterator.Iterator<float64>.Return> template=iter.iterator.IteratorReturn arguments=(iter.iterator.Iterator<float64>.Return)
/// @generic.instance id=iter.iterator.IteratorReturn<void> template=iter.iterator.IteratorReturn arguments=(void)
/// @generic.instance id=iter.iterator.IteratorYield<float64> template=iter.iterator.IteratorYield arguments=(float64)
"#,
    );
}
