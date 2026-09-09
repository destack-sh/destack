use crate::tests::{DirRows, TestSession};

#[test]
fn test_overwrite_copy_and_handle_fields_through_a_mutable_borrow() {
    let session = TestSession::single(
        r#"
class User {}

struct State {
    count: int32;
    user: User;
}

function update(state: &State): void {
    state.count = 1;
    state.user = state.user;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct State {
    count: int32;
    user: User;
}

function update<'a>(state: &'a State): void {
    state.count = 1;
    state.user = state.user;
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

struct State {
/// @type.symbol symbol=State type=State
/// @definition.struct symbol=State
/// @definition.field symbol=State.count source="count: int32" key=count type=int32
/// @definition.field symbol=State.user source="user: User" key=user type=User

    count: int32;
    /// @type.symbol symbol=State.count source="count: int32" type=int32

    user: User;
    /// @type.symbol symbol=State.user source="user: User" type=User
    /// @resolution.name source=User target=User

}

function update(state: &State): void {
/// @generic.template symbol=update parameters=('a)
/// @type.symbol symbol=update type=<update.'a>(&update.'a State) => void
/// @type.symbol symbol=update.state source="state: &State" type=&update.'a State
/// @resolution.name source=State target=State

    state.count = 1;
    /// @resolution.name source=state target=update.state
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.count kind=place
    /// @resolution.access source=state.count root=update.state keys=[count]
    /// @resolution.assignment source=state.count write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.count, type=int32), type=int32" type=int32

    state.user = state.user;
    /// @resolution.name source=state target=update.state
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.user kind=place
    /// @resolution.access source=state.user root=update.state keys=[user]
    /// @resolution.assignment source=state.user write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.user, type=User), type=User" type=User
    /// @resolution.name source=state target=update.state
    /// @resolution.member source=state.user receiver=&update.'a State type=User kind=field target_receiver=&update.'a State key=user target=State.user target_type=User
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.place source=state.user placement=update.'a lifetime="managed" access="mutable"
    /// @resolution.access source=state.user root=update.state keys=[user]

}
"#,
    );
}

#[test]
fn test_overwrite_copy_and_handle_fields_through_a_shared_mutable_borrow() {
    let session = TestSession::single(
        r#"
class User {}

shared struct State {
    count: int32;
    user: User;
}

function update(state: &State): void {
    state.count = 1;
    state.user = state.user;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

shared struct State {
    count: int32;
    user: User;
}

function update<'a>(state: &'a State): void {
    state.count = 1;
    state.user = state.user;
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

shared struct State {
/// @type.symbol symbol=State type=State
/// @definition.struct symbol=State
/// @definition.field symbol=State.count source="count: int32" key=count type=int32
/// @definition.field symbol=State.user source="user: User" key=user type=User

    count: int32;
    /// @type.symbol symbol=State.count source="count: int32" type=int32

    user: User;
    /// @type.symbol symbol=State.user source="user: User" type=User
    /// @resolution.name source=User target=User

}

function update(state: &State): void {
/// @generic.template symbol=update parameters=('a)
/// @type.symbol symbol=update type=<update.'a>(&update.'a State) => void
/// @type.symbol symbol=update.state source="state: &State" type=&update.'a State
/// @resolution.name source=State target=State

    state.count = 1;
    /// @resolution.name source=state target=update.state
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.count kind=place
    /// @resolution.access source=state.count root=update.state keys=[count]
    /// @resolution.assignment source=state.count write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.count, type=int32), type=int32" type=int32

    state.user = state.user;
    /// @resolution.name source=state target=update.state
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.user kind=place
    /// @resolution.access source=state.user root=update.state keys=[user]
    /// @resolution.assignment source=state.user write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.user, type=shared User), type=shared User" type=shared User
    /// @resolution.name source=state target=update.state
    /// @resolution.member source=state.user receiver=&update.'a State type=shared User kind=field target_receiver=&update.'a State key=user target=State.user target_type=shared User
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.place source=state.user placement="shared" lifetime="managed" access="mutable"
    /// @resolution.access source=state.user root=update.state keys=[user]

}
"#,
    );
}

#[test]
fn test_overwrite_an_enum_field_through_a_mutable_borrow() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }

struct State { status: Status; }

function update(state: &State): void {
    state.status = Status.Busy;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Status {
    Idle,
    Busy,
}

struct State {
    status: Status;
}

function update<'a>(state: &'a State): void {
    state.status = Status.Busy;
}

=== dir ===
enum Status { Idle, Busy }
/// @type.symbol symbol=Status source="enum Status { Idle, Busy }" type=Status
/// @definition.enum symbol=Status source="enum Status { Idle, Busy }"
/// @definition.variant symbol=Status.Busy source=Busy key=Busy value=1
/// @definition.variant symbol=Status.Idle source=Idle key=Idle value=0
/// @type.symbol symbol=Status.Idle source=Idle type=Status.Idle
/// @type.symbol symbol=Status.Busy source=Busy type=Status.Busy

struct State { status: Status; }
/// @type.symbol symbol=State source="struct State { status: Status; }" type=State
/// @definition.struct symbol=State source="struct State { status: Status; }"
/// @definition.field symbol=State.status source="status: Status" key=status type=Status
/// @type.symbol symbol=State.status source="status: Status" type=Status
/// @resolution.name source=Status target=Status

function update(state: &State): void {
/// @generic.template symbol=update parameters=('a)
/// @type.symbol symbol=update type=<update.'a>(&update.'a State) => void
/// @type.symbol symbol=update.state source="state: &State" type=&update.'a State
/// @resolution.name source=State target=State

    state.status = Status.Busy;
    /// @resolution.name source=state target=update.state
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.status kind=place
    /// @resolution.access source=state.status root=update.state keys=[status]
    /// @resolution.assignment source=state.status write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.status, type=Status), type=Status" type=Status
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy

}
"#,
    );
}

#[test]
fn test_overwrite_a_copy_value_through_a_mutable_borrow() {
    let session = TestSession::single(
        r#"
function update(value: &int32): void {
    *value = 1;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function update<'a>(value: &'a int32): void {
    *value = 1;
}

=== dir ===
function update(value: &int32): void {
/// @generic.template symbol=update parameters=('a)
/// @type.symbol symbol=update type=<update.'a>(&update.'a int32) => void
/// @type.symbol symbol=update.value source="value: &int32" type=&update.'a int32

    *value = 1;
    /// @resolution.pattern.assign source=*value kind=place
    /// @resolution.assignment source=*value write="&update.'a int32 => direct -> int32" type=int32
    /// @resolution.name source=value target=update.value
    /// @resolution.place source=value placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=value root=update.value

}
"#,
    );
}

#[test]
fn test_reject_write_through_readonly_borrow() {
    let session = TestSession::single(
        r#"
function update(value: &readonly int32): void {
    *value = 1;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function update<'a>(value: &'a readonly int32): void {
    *value = 1;
}

=== dir ===
function update(value: &readonly int32): void {
/// @generic.template symbol=update parameters=('a)
/// @type.symbol symbol=update type=<update.'a>(&update.'a readonly int32) => void
/// @type.symbol symbol=update.value source="value: &readonly int32" type=&update.'a readonly int32

    *value = 1;
    /// @resolution.rejected source=*value
    /// @resolution.name source=value target=update.value
    /// @resolution.place source=value placement=update.'a lifetime=update.'a access="readonly"
    /// @resolution.access source=value root=update.value

}
"#,
        r#"
/// @diagnostic.error id=borrow-access-not-granted message="'mutable' access is not granted by a value of type '&'a readonly int32'"
/// @diagnostic.label line=3 column=5 span="*" line_source="*value = 1;"
/// @diagnostic.note message="the source grants at most 'readonly' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
"#,
    );
}

#[test]
fn test_overwrite_an_enum_field_through_an_owned_value() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }

struct State { status: Status; }

declare let state: ^State;

state.status = Status.Busy;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Status {
    Idle,
    Busy,
}

struct State {
    status: Status;
}

declare let state: State;

state.status = Status.Busy;

=== dir ===
enum Status { Idle, Busy }
/// @type.symbol symbol=Status source="enum Status { Idle, Busy }" type=Status
/// @definition.enum symbol=Status source="enum Status { Idle, Busy }"
/// @definition.variant symbol=Status.Busy source=Busy key=Busy value=1
/// @definition.variant symbol=Status.Idle source=Idle key=Idle value=0
/// @type.symbol symbol=Status.Idle source=Idle type=Status.Idle
/// @type.symbol symbol=Status.Busy source=Busy type=Status.Busy

struct State { status: Status; }
/// @type.symbol symbol=State source="struct State { status: Status; }" type=State
/// @definition.struct symbol=State source="struct State { status: Status; }"
/// @definition.field symbol=State.status source="status: Status" key=status type=Status
/// @type.symbol symbol=State.status source="status: Status" type=Status
/// @resolution.name source=Status target=Status

declare let state: ^State;
/// @type.symbol symbol=state source=state type=State
/// @resolution.pattern source=state kind=binding target=state
/// @resolution.name source=State target=State

state.status = Status.Busy;
/// @resolution.name source=state target=state
/// @resolution.place source=state placement="local" lifetime="static" access="mutable"
/// @resolution.access source=state root=state
/// @resolution.pattern.assign source=state.status kind=place
/// @resolution.access source=state.status root=state keys=[status]
/// @resolution.assignment source=state.status write="receiver=State, target=field(receiver=State, target=State.status, type=Status), type=Status" type=Status
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy
"#,
        r#"
"#,
    );
}

#[test]
fn test_reject_field_write_through_immutable_direct_storage() {
    let session = TestSession::single(
        r#"
struct State { count: int32; }

declare const state: State;

state.count = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct State {
    count: int32;
}

declare const state: State;

state.count = 1;

=== dir ===
struct State { count: int32; }
/// @type.symbol symbol=State source="struct State { count: int32; }" type=State
/// @definition.struct symbol=State source="struct State { count: int32; }"
/// @definition.field symbol=State.count source="count: int32" key=count type=int32
/// @type.symbol symbol=State.count source="count: int32" type=int32

declare const state: State;
/// @type.symbol symbol=state source=state type=State
/// @resolution.pattern source=state kind=binding target=state
/// @resolution.name source=State target=State

state.count = 1;
/// @resolution.name source=state target=state
/// @resolution.place source=state placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=state root=state
/// @resolution.pattern.assign source=state.count kind=place
/// @resolution.access source=state.count root=state keys=[count]
/// @resolution.assignment source=state.count write="receiver=Readonly<State>, target=field(receiver=Readonly<State>, target=State.count, type=int32), type=int32" type=int32
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'count'"
/// @diagnostic.label line=6 column=7 span="count" line_source="state.count = 1;"
"#,
    );
}

#[test]
fn test_initialize_shared_fields_through_owned_constructor_receiver() {
    let session = TestSession::single(
        r#"
shared class Cell<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
shared class Cell<in out T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

=== dir ===
shared class Cell<T> {
/// @generic.template symbol=Cell parameters=(in out T)
/// @type.symbol symbol=Cell type=Cell
/// @definition.class symbol=Cell template=(in out T)
/// @definition.field symbol=Cell.value source="value: T" key=value type=T
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=<Cell.constructor.P0: Place>(T) => Managed<Cell<T>, Cell.constructor.P0>
/// @type.symbol symbol=Cell.T source=T type=T

    value: T;
    /// @type.symbol symbol=Cell.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

    constructor(value: T) {
    /// @generic.template symbol=Cell.constructor parent=template#0 parameters=(P0: Place)
    /// @type.symbol symbol=Cell.constructor type=<Cell.constructor.P0: Place>(T) => Managed<Cell<T>, Cell.constructor.P0>
    /// @type.symbol symbol=Cell.constructor.this type=Cell<T>
    /// @type.symbol symbol=Cell.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T>
        /// @resolution.place source=this placement="shared" lifetime="frame" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Cell<T>, target=field(receiver=Cell<T>, target=Cell.value, type=T), type=T" type=T
        /// @resolution.name source=value target=Cell.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=value root=Cell.constructor.value

    }
}
"#,
    );
}

#[test]
fn test_reject_a_mutable_receiver_on_a_const_owned_array() {
    let session = TestSession::single(
        r#"
declare const items: ^Array<int32>;

items.push(1);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const items: ^int32[];

items.push<int32>(1);

=== dir ===
declare const items: ^Array<int32>;
/// @type.symbol symbol=items source=items type=^int32[]
/// @resolution.pattern source=items kind=binding target=items
/// @resolution.name source=Array target=Array

items.push(1);
/// @resolution.name source=items target=items
/// @resolution.member source=items.push receiver=^int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=^int32[] target=push
/// @resolution.call source=items.push(1) parameters=(int32[]) arguments=(rest(1) pack=arrayFromOwnedSlice as int32) return=isize regions=("frame") kind=symbol target=push receiver=^int32[] instance=Array<int32>.<extension#6>.push
/// @resolution.place source=items placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=items root=items
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instantiation id=push<int32> template=push arguments=(int32)
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '^int32[]' is not assignable to the method's 'this' type '&int32[]'"
/// @diagnostic.label line=4 column=1 span="items.push(1)" line_source="items.push(1);"
"#,
    );
}

#[test]
fn test_accept_a_mutable_receiver_on_a_const_managed_array() {
    let session = TestSession::single(
        r#"
declare const items: Array<int32>;

items.push(1);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const items: int32[];

items.push<int32>(1);

=== dir ===
declare const items: Array<int32>;
/// @type.symbol symbol=items source=items type=int32[]
/// @resolution.pattern source=items kind=binding target=items
/// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
/// @resolution.name source=Array target=Array

items.push(1);
/// @resolution.name source=items target=items
/// @resolution.member source=items.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
/// @resolution.call source=items.push(1) parameters=(int32[]) arguments=(rest(1) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(Borrowed<int32[], "managed" & "local", "mutable">)) instance=Array<int32>.<extension#6>.push
/// @resolution.place source=items placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=items root=items
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instantiation id=push<int32> template=push arguments=(int32)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=append<int32> template=append arguments=(int32)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=assumeInitRead#1<int32> template=assumeInitRead#1 arguments=(int32)
/// @generic.instance id=assumeInitRead<int32> template=assumeInitRead arguments=(int32)
/// @generic.instance id=fromOwnedSlice<int32> template=fromOwnedSlice arguments=(int32)
/// @generic.instance id=initWrite<int32> template=initWrite arguments=(int32)
/// @generic.instance id=intoUninit<int32> template=intoUninit arguments=(int32)
/// @generic.instance id=push<int32> template=push arguments=(int32)
/// @generic.instance id=reserve<int32> template=reserve arguments=(int32)
/// @generic.instance id=size<int32> template=size arguments=(int32)
/// @generic.instance id=sliceIntoUninit<int32> template=sliceIntoUninit arguments=(int32)
/// @generic.instance id=sliceLength<int32> template=sliceLength arguments=(int32)
/// @generic.instance id=sliceUninit<int32> template=sliceUninit arguments=(int32)
/// @generic.instance id=uninit<int32> template=uninit arguments=(int32)
/// @generic.instance id=write<int32> template=write arguments=(int32)
"#,
    );
}

#[test]
fn test_record_a_mutable_use_for_a_returned_mutable_borrow() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;
}

function identity<'a>(counter: &'a Counter): &'a Counter {
    return counter;
}

function forward<'a>(counter: &'a Counter): &'a Counter {
    return identity(counter);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_flows(),
        r#"
=== annotated ===
struct Counter {
    value: int32;
}

function identity<'a>(counter: &'a Counter): &'a Counter {
    return counter;
}

function forward<'a>(counter: &'a Counter): &'a Counter {
    return identity(counter);
}

=== dir ===
struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @flow.use symbol=Counter uses=read

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

}

function identity<'a>(counter: &'a Counter): &'a Counter {
/// @generic.template symbol=identity parameters=('a#1)
/// @type.symbol symbol=identity type=<'a#1>(&'a#1 Counter) => &'a#1 Counter
/// @flow.use symbol=identity uses=read
/// @type.symbol symbol=identity.'a source='a type='a#1
/// @flow.use symbol='a#1 uses=read
/// @type.symbol symbol=identity.counter source="counter: &'a Counter" type=&'a#1 Counter
/// @flow.use symbol=counter#1 uses=read+mutable+moved
/// @resolution.name source='a target=identity.'a
/// @resolution.name source=Counter target=Counter
/// @resolution.name source='a target=identity.'a
/// @resolution.name source=Counter target=Counter

    return counter;
    /// @flow.diverging source="return counter"
    /// @resolution.name source=counter target=identity.counter
    /// @resolution.place source=counter placement='a#1 lifetime='a#1 access="mutable"
    /// @resolution.access source=counter root=identity.counter
    /// @flow.access source=counter root=identity.counter uses=read+mutable+moved

}

function forward<'a>(counter: &'a Counter): &'a Counter {
/// @generic.template symbol=forward parameters=('a#2)
/// @type.symbol symbol=forward type=<'a#2>(&'a#2 Counter) => &'a#2 Counter
/// @type.symbol symbol=forward.'a source='a type='a#2
/// @flow.use symbol='a#2 uses=read
/// @type.symbol symbol=forward.counter source="counter: &'a Counter" type=&'a#2 Counter
/// @flow.use symbol=counter#2 uses=read+mutable+moved
/// @resolution.name source='a target=forward.'a
/// @resolution.name source=Counter target=Counter
/// @resolution.name source='a target=forward.'a
/// @resolution.name source=Counter target=Counter

    return identity(counter);
    /// @flow.diverging source="return identity(counter)"
    /// @resolution.name source=identity target=identity
    /// @resolution.call source=identity(counter) parameters=(&'a#2 Counter) arguments=(provided(counter) as &'a#2 Counter) return=&'a#2 Counter regions=('a#2) kind=symbol target=identity
    /// @resolution.name source=counter target=forward.counter
    /// @resolution.place source=counter placement='a#2 lifetime='a#2 access="mutable"
    /// @resolution.access source=counter root=forward.counter
    /// @flow.access source=counter root=forward.counter uses=read+mutable+moved

}
"#,
        r#"
"#,
    );
}

/// A const handle keeps its referent writable, so its methods borrow exclusively.
#[test]
fn test_borrow_a_const_array_literal_mutably_for_a_method() {
    let session = TestSession::single(
        r#"
function grow(): int32[] {
    const values = [1, 2, 3];
    values.push(4);
    values.fill(0);

    return values;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function grow(): int32[] {
    const values: int32[] = [1, 2, 3];
    values.push<int32>(4);
    values.fill<int32>(0);

    return values;
}

=== dir ===
function grow(): int32[] {
/// @type.symbol symbol=grow type=() => int32[]

    const values = [1, 2, 3];
    /// @type.symbol symbol=grow.values source=values type=int32[]
    /// @resolution.pattern source=values kind=binding target=grow.values
    /// @resolution.call source=[1, 2, 3] parameters=(^Slice<arrayFromOwnedSlice.T>) arguments=(rest(1, 2, 3) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>

    values.push(4);
    /// @resolution.name source=values target=grow.values
    /// @resolution.member source=values.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
    /// @resolution.call source=values.push(4) parameters=(int32[]) arguments=(rest(4) pack=arrayFromOwnedSlice as int32) return=isize regions=("frame" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(&'frame int32[])) instance=Array<int32>.<extension#6>.push
    /// @resolution.place source=values placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=values root=grow.values
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
    /// @generic.instantiation id=push<int32> template=push arguments=(int32)

    values.fill(0);
    /// @resolution.name source=values target=grow.values
    /// @resolution.member source=values.fill receiver=int32[] type=<fill.'a>(this: &fill.'a int32[], int32, isize | undefined?, isize | undefined?) => int32[] kind=symbol target_receiver=int32[] target=fill
    /// @resolution.call source=values.fill(0) parameters=(int32, isize | undefined, isize | undefined) arguments=(provided(0) as int32, omitted as isize | undefined, omitted as isize | undefined) return=int32[] regions=("managed" & "local") kind=symbol target=fill receiver=int32[] adjustments=(borrow(Borrowed<int32[], "managed" & "local", "mutable">)) instance=Array<int32>.<extension#6>.fill
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=grow.values
    /// @generic.instantiation id=fill<int32> template=fill arguments=(int32)

    return values;
    /// @resolution.name source=values target=grow.values
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=grow.values

}
"#,
        r#"
"#,
    );
}

/// A mutable borrow of a narrowed inline Copy payload through a handle borrows a readonly copy.
#[test]
fn test_reject_a_mutable_borrow_of_a_narrowed_copy_payload_through_a_handle() {
    let session = TestSession::single(
        r#"
class Cell {
    value: string | number;

    constructor(value: string | number) {
        this.value = value;
    }
}

function bump(cell: Cell): void {
    if (cell.value is number) {
        const slot = &cell.value;
        *slot = 2;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Cell {
    value: string | float64;

    constructor(value: string | float64) {
        this.value = value;
    }
}

function bump(cell: Cell): void {
    if (cell.value is number) {
        const slot: &'frame float64 = &cell.value;
        *slot = 2;
    }
}

=== dir ===
class Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.class symbol=Cell
/// @definition.field symbol=Cell.value source="value: string | number" key=value type=string | float64
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=<Cell.constructor.P0: Place>(string | float64) => Managed<this, Cell.constructor.P0>

    value: string | number;
    /// @type.symbol symbol=Cell.value source="value: string | number" type=string | float64

    constructor(value: string | number) {
    /// @generic.template symbol=Cell.constructor parameters=(P0: Place)
    /// @type.symbol symbol=Cell.constructor type=<Cell.constructor.P0: Place>(string | float64) => Managed<this, Cell.constructor.P0>
    /// @type.symbol symbol=Cell.constructor.this type=Cell
    /// @type.symbol symbol=Cell.constructor.value source="value: string | number" type=string | float64

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell
        /// @resolution.place source=this placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Cell, target=field(receiver=Cell, target=Cell.value, type=string | float64), type=string | float64" type=string | float64
        /// @resolution.name source=value target=Cell.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=value root=Cell.constructor.value

    }
}

function bump(cell: Cell): void {
/// @type.symbol symbol=bump type=(Cell) => void
/// @type.symbol symbol=bump.cell source="cell: Cell" type=Cell
/// @resolution.name source=Cell target=Cell

    if (cell.value is number) {
    /// @resolution.name source=cell target=bump.cell
    /// @resolution.member source=cell.value receiver=Cell type=string | float64 kind=field target_receiver=Cell key=value target=Cell.value target_type=string | float64
    /// @resolution.guard source="cell.value is number" kind=is value=string | float64 target=float64 predicate="string | float64 is float64" narrowed=Narrow<string | float64, float64>
    /// @resolution.place source=cell placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=cell root=bump.cell
    /// @resolution.place source=cell.value placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=cell.value root=bump.cell keys=[value]

        const slot = &cell.value;
        /// @type.symbol symbol=bump.slot source=slot type=&'frame float64
        /// @resolution.pattern source=slot kind=binding target=bump.slot
        /// @resolution.name source=cell target=bump.cell
        /// @resolution.member source=cell.value receiver=Cell type=string | float64 kind=field target_receiver=Cell key=value target=Cell.value target_type=string | float64
        /// @resolution.place source=cell placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=cell root=bump.cell
        /// @resolution.place source=cell.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=cell.value root=bump.cell keys=[value]
        /// @resolution.narrowing source=cell.value union=string | float64 arms=float64

        *slot = 2;
        /// @resolution.pattern.assign source=*slot kind=place
        /// @resolution.assignment source=*slot write="&'frame Narrow<string | float64, float64> => direct -> Narrow<string | float64, float64>" type=Narrow<string | float64, float64>
        /// @resolution.name source=slot target=bump.slot
        /// @resolution.place source=slot placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=slot root=bump.slot

    }
}
"#,
        r#"
/// @diagnostic.error id=borrow-access-not-granted message="'mutable' access is not granted by a value of type 'Narrow<string | float64, float64>'"
/// @diagnostic.label line=12 column=22 span="&" line_source="const slot = &cell.value;"
/// @diagnostic.note message="the source grants at most 'readonly' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
"#,
    );
}

#[test]
fn test_read_array_elements_through_a_readonly_index_in_a_counted_loop() {
    let session = TestSession::single(
        r#"
function copy(values: int32[], output: int32[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        output.push(values[index]);
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_flows(),
        r#"
=== annotated ===
function copy(values: int32[], output: int32[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        output.push<int32>(values[index]);
    }
}

=== dir ===
function copy(values: int32[], output: int32[]): void {
/// @type.symbol symbol=copy type=(int32[], int32[]) => void
/// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
/// @type.symbol symbol=copy.values source="values: int32[]" type=int32[]
/// @flow.use symbol=values uses=read+mutable
/// @type.symbol symbol=copy.output source="output: int32[]" type=int32[]
/// @flow.use symbol=output uses=read+mutable

    for (let index: isize = 0; index < values.length; index++) {
    /// @type.symbol symbol=copy.index source=index type=isize
    /// @resolution.pattern source=index kind=binding target=copy.index
    /// @flow.use symbol=index uses=read+written
    /// @resolution.name source=index target=copy.index
    /// @resolution.operator source="index < values.length" type=boolean operator="<" kind=builtin operands=[index as isize families=(integer), values.length as isize families=(integer)]
    /// @resolution.place source=index placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=index root=copy.index
    /// @flow.access source=index root=copy.index uses=read
    /// @resolution.name source=values target=copy.values
    /// @resolution.member source=values.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=copy.values
    /// @generic.instantiation id=length<int32> template=length arguments=(int32)
    /// @generic.instance id=length<int32> template=length arguments=(int32)
    /// @flow.access source=values root=copy.values uses=read
    /// @resolution.name source=index target=copy.index
    /// @resolution.assignment source=index read=binding(copy.index) write=binding(copy.index) type=isize
    /// @resolution.access source=index root=copy.index
    /// @resolution.operator source=index++ type=isize operator="++" kind=builtin operands=[index as isize families=(integer)]
    /// @flow.access source=index root=copy.index uses=written

        output.push(values[index]);
        /// @resolution.name source=output target=copy.output
        /// @resolution.member source=output.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
        /// @resolution.call source=output.push(values[index]) parameters=(int32[]) arguments=(rest(values[index]) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(Borrowed<int32[], "managed" & "local", "mutable">)) instance=Array<int32>.<extension#6>.push
        /// @resolution.place source=output placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=output root=copy.output
        /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
        /// @generic.instantiation id=push<int32> template=push arguments=(int32)
        /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
        /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
        /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
        /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
        /// @generic.instance id=append<int32> template=append arguments=(int32)
        /// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
        /// @generic.instance id=assumeInitRead#1<int32> template=assumeInitRead#1 arguments=(int32)
        /// @generic.instance id=assumeInitRead<int32> template=assumeInitRead arguments=(int32)
        /// @generic.instance id=fromOwnedSlice<int32> template=fromOwnedSlice arguments=(int32)
        /// @generic.instance id=initWrite<int32> template=initWrite arguments=(int32)
        /// @generic.instance id=intoUninit<int32> template=intoUninit arguments=(int32)
        /// @generic.instance id=push<int32> template=push arguments=(int32)
        /// @generic.instance id=reserve<int32> template=reserve arguments=(int32)
        /// @generic.instance id=size<int32> template=size arguments=(int32)
        /// @generic.instance id=sliceIntoUninit<int32> template=sliceIntoUninit arguments=(int32)
        /// @generic.instance id=sliceLength<int32> template=sliceLength arguments=(int32)
        /// @generic.instance id=sliceUninit<int32> template=sliceUninit arguments=(int32)
        /// @generic.instance id=uninit<int32> template=uninit arguments=(int32)
        /// @generic.instance id=write<int32> template=write arguments=(int32)
        /// @flow.access source=output root=copy.output uses=read+mutable
        /// @resolution.name source=values target=copy.values
        /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=values root=copy.values
        /// @resolution.place source=values[index] placement="local" lifetime="managed" access="mutable"
        /// @resolution.subscript source=values[index] type=int32 kind=call target="index#1(parameters=(isize), arguments=(provided(index) as isize), return=WithAccess<Borrowed<int32, \"managed\" & \"local\", \"mutable\">, \"mutable\">, regions=(\"managed\" & \"local\"))"
        /// @generic.instantiation id="index#1<int32, \"mutable\">" template=index#1 arguments=(int32, "mutable")
        /// @generic.instance id="WithAccess<&'bound0 int32, \"mutable\">" template=WithAccess arguments=(&'bound0 int32, "mutable")
        /// @generic.instance id="WithAccess<&'bound0 int32[], \"mutable\">" template=WithAccess arguments=(&'bound0 int32[], "mutable")
        /// @generic.instance id="assumeInitReference<int32, \"mutable\">" template=assumeInitReference arguments=(int32, "mutable")
        /// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
        /// @generic.instance id="index#1<int32, \"mutable\">" template=index#1 arguments=(int32, "mutable")
        /// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
        /// @generic.instance id=elementPosition<int32> template=elementPosition arguments=(int32)
        /// @flow.access source=values root=copy.values uses=read+mutable
        /// @resolution.name source=index target=copy.index
        /// @resolution.place source=index placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=index root=copy.index
        /// @flow.access source=index root=copy.index uses=read

    }
}

/// @flow.foreign symbol=length uses=read
/// @flow.foreign symbol=push uses=read
"#,
    );
}

#[test]
fn test_record_a_move_of_an_owned_binding_passed_by_value() {
    let session = TestSession::single(
        r#"
declare function consume(value: ^string): void;

function observe(values: Iterator<^string>): Iterator<^string> {
    return values.map((value) => {
        consume(value);
        value
    });
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_flows(),
        r#"
=== annotated ===
declare function consume(value: ^string): void;

function observe(values: Iterator<^string>): Iterator<^string> {
    return values.map<^string, ^string>((value: ^string): ^string => {
        consume(value);
        value
    }) as Iterator<^string>;
}

=== dir ===
declare function consume(value: ^string): void;
/// @type.symbol symbol=consume source="declare function consume(value: ^string): void" type=(^string) => void
/// @flow.use symbol=consume uses=read
/// @type.symbol symbol=consume.value source="value: ^string" type=^string

function observe(values: Iterator<^string>): Iterator<^string> {
/// @type.symbol symbol=observe type=(Iterator<^string>) => Iterator<^string>
/// @generic.instance id=Iterator<^string> template=Iterator arguments=(^string)
/// @type.symbol symbol=observe.values source="values: Iterator<^string>" type=Iterator<^string>
/// @flow.use symbol=values uses=read
/// @resolution.name source=Iterator target=Iterator
/// @resolution.name source=Iterator target=Iterator

    return values.map((value) => {
    /// @flow.diverging
    /// @resolution.name source=values target=observe.values
    /// @resolution.member source=values.map receiver=Iterator<^string> type=<Iterator.map.U>(this: Iterator<^string>, Function<(^string, isize), Iterator.map.U>) => MapIterator<Iterator<^string>, ^string, Iterator.map.U> kind=symbol target_receiver=Iterator<^string> dispatch=dynamic constraint=Iterator<^string> target=Iterator.map
    /// @resolution.call parameters=(Function<(^string, isize), ^string>) arguments=(provided(argument) as Function<(^string, isize), ^string>) return=MapIterator<Iterator<^string>, ^string, ^string> kind=dynamic target=Iterator.map receiver=Iterator<^string> constraint=Iterator<^string> generic_arguments=(^string, ^string)
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=observe.values
    /// @generic.instantiation id=Iterator.map<^string> template=Iterator.map arguments=(^string)
    /// @generic.instance id="MapIterator<Iterator<^string>, ^string, ^string>" template=MapIterator arguments=(Iterator<^string>, ^string, ^string)
    /// @flow.access source=values root=observe.values uses=read
    /// @type.symbol symbol=observe.symbol5 type=Function<(^string,), ^string, "readonly">
    /// @type.symbol symbol=observe.symbol5.value source=value type=^string
    /// @flow.use symbol=value#2 uses=read+moved

        consume(value);
        /// @resolution.name source=consume target=consume
        /// @resolution.call source=consume(value) parameters=(^string) arguments=(provided(value) as ^string) return=void kind=symbol target=consume
        /// @resolution.name source=value target=observe.symbol5.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=value root=observe.symbol5.value
        /// @flow.access source=value root=observe.symbol5.value uses=read+moved

        value
        /// @resolution.name source=value target=observe.symbol5.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="mutable"
        /// @resolution.access source=value root=observe.symbol5.value
        /// @flow.access source=value root=observe.symbol5.value uses=read+moved

    });
}

/// @flow.foreign symbol=Iterator uses=read
/// @flow.foreign symbol=Iterator.map uses=read
"#,
    );
}

#[test]
fn test_record_no_move_of_a_managed_handle_passed_by_value() {
    let session = TestSession::single(
        r#"
class User {}

declare function observe(user: User): void;

function inspect(user: User): User {
    observe(user);
    return user;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_flows(),
        r#"
=== annotated ===
class User {}

declare function observe(user: User): void;

function inspect(user: User): User {
    observe(user);
    return user;
}

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"
/// @flow.use symbol=User uses=read

declare function observe(user: User): void;
/// @type.symbol symbol=observe source="declare function observe(user: User): void" type=(User) => void
/// @flow.use symbol=observe uses=read
/// @type.symbol symbol=observe.user source="user: User" type=User
/// @resolution.name source=User target=User

function inspect(user: User): User {
/// @type.symbol symbol=inspect type=(User) => User
/// @type.symbol symbol=inspect.user source="user: User" type=User
/// @flow.use symbol=user#2 uses=read
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User

    observe(user);
    /// @resolution.name source=observe target=observe
    /// @resolution.call source=observe(user) parameters=(User) arguments=(provided(user) as User) return=void kind=symbol target=observe
    /// @resolution.name source=user target=inspect.user
    /// @resolution.place source=user placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=user root=inspect.user
    /// @flow.access source=user root=inspect.user uses=read

    return user;
    /// @flow.diverging source="return user"
    /// @resolution.name source=user target=inspect.user
    /// @resolution.place source=user placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=user root=inspect.user
    /// @flow.access source=user root=inspect.user uses=read

}
"#,
    );
}

#[test]
fn test_record_a_mutable_borrow_for_a_mutating_method_on_a_managed_array() {
    let session = TestSession::single(
        r#"
function grow(values: int32[]): void {
    values.push(0);
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_flows(),
        r#"
=== annotated ===
function grow(values: int32[]): void {
    values.push<int32>(0);
}

=== dir ===
function grow(values: int32[]): void {
/// @type.symbol symbol=grow type=(int32[]) => void
/// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
/// @type.symbol symbol=grow.values source="values: int32[]" type=int32[]
/// @flow.use symbol=values uses=read+mutable

    values.push(0);
    /// @resolution.name source=values target=grow.values
    /// @resolution.member source=values.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
    /// @resolution.call source=values.push(0) parameters=(int32[]) arguments=(rest(0) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(Borrowed<int32[], "managed" & "local", "mutable">)) instance=Array<int32>.<extension#6>.push
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=grow.values
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
    /// @generic.instantiation id=push<int32> template=push arguments=(int32)
    /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
    /// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
    /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id=append<int32> template=append arguments=(int32)
    /// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
    /// @generic.instance id=assumeInitRead#1<int32> template=assumeInitRead#1 arguments=(int32)
    /// @generic.instance id=assumeInitRead<int32> template=assumeInitRead arguments=(int32)
    /// @generic.instance id=fromOwnedSlice<int32> template=fromOwnedSlice arguments=(int32)
    /// @generic.instance id=initWrite<int32> template=initWrite arguments=(int32)
    /// @generic.instance id=intoUninit<int32> template=intoUninit arguments=(int32)
    /// @generic.instance id=push<int32> template=push arguments=(int32)
    /// @generic.instance id=reserve<int32> template=reserve arguments=(int32)
    /// @generic.instance id=size<int32> template=size arguments=(int32)
    /// @generic.instance id=sliceIntoUninit<int32> template=sliceIntoUninit arguments=(int32)
    /// @generic.instance id=sliceLength<int32> template=sliceLength arguments=(int32)
    /// @generic.instance id=sliceUninit<int32> template=sliceUninit arguments=(int32)
    /// @generic.instance id=uninit<int32> template=uninit arguments=(int32)
    /// @generic.instance id=write<int32> template=write arguments=(int32)
    /// @flow.access source=values root=grow.values uses=read+mutable

}

/// @flow.foreign symbol=push uses=read
"#,
    );
}
