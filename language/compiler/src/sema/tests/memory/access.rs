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
        "main.tspp",
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
/// @type.symbol symbol=User source="class User {}" type=typeof User
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
    /// @resolution.place source=state.count placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state.count root=update.state keys=[count]
    /// @resolution.assignment source=state.count write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.count, type=int32), type=int32" type=int32

    state.user = state.user;
    /// @resolution.name source=state target=update.state
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.user kind=place
    /// @resolution.place source=state.user placement="local" lifetime=update.'a access="mutable"
    /// @resolution.access source=state.user root=update.state keys=[user]
    /// @resolution.assignment source=state.user write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.user, type=User), type=User" type=User
    /// @resolution.name source=state target=update.state
    /// @resolution.member source=state.user receiver=&update.'a State type=User kind=field target_receiver=&update.'a State key=user target=State.user target_type=User
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.place source=state.user placement="local" lifetime=update.'a access="mutable"
    /// @resolution.access source=state.user root=update.state keys=[user]

}
"#,
    );
}

#[test]
fn test_overwrite_copy_and_handle_fields_through_a_shared_mutable_borrow() {
    let session = TestSession::single(
        r#"
shared class User {}

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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
shared class User {}

shared struct State {
    count: int32;
    user: User;
}

function update<'a>(state: &'a State): void {
    state.count = 1;
    state.user = state.user;
}

=== dir ===
shared class User {}
/// @type.symbol symbol=User source="shared class User {}" type=typeof User
/// @definition.class symbol=User source="shared class User {}"

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
    /// @resolution.place source=state.count placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state.count root=update.state keys=[count]
    /// @resolution.assignment source=state.count write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.count, type=int32), type=int32" type=int32

    state.user = state.user;
    /// @resolution.name source=state target=update.state
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.user kind=place
    /// @resolution.place source=state.user placement="shared" lifetime=update.'a access="readonly"
    /// @resolution.access source=state.user root=update.state keys=[user]
    /// @resolution.assignment source=state.user write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.user, type=User), type=User" type=User
    /// @resolution.name source=state target=update.state
    /// @resolution.member source=state.user receiver=&update.'a State type=User kind=field target_receiver=&update.'a State key=user target=State.user target_type=User
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.place source=state.user placement="shared" lifetime=update.'a access="readonly"
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
        "main.tspp",
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
    /// @resolution.place source=state.status placement=update.'a lifetime=update.'a access="mutable"
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
        "main.tspp",
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
    /// @resolution.place source=*value placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.assignment source=*value write="&update.'a int32 => builtin -> int32" type=int32
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
        "main.tspp",
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
        "main.tspp",
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
/// @resolution.place source=state placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=state root=state
/// @resolution.pattern.assign source=state.status kind=place
/// @resolution.place source=state.status placement="local" lifetime="static" access="exclusive"
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
        "main.tspp",
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
/// @resolution.place source=state placement="local" lifetime="static" access="immutable"
/// @resolution.access source=state root=state
/// @resolution.pattern.assign source=state.count kind=place
/// @resolution.place source=state.count placement="local" lifetime="static" access="immutable"
/// @resolution.access source=state.count root=state keys=[count]
/// @resolution.assignment source=state.count write="receiver=readonly State, target=field(receiver=readonly State, target=State.count, type=int32), type=int32" type=int32
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
        "main.tspp",
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
/// @type.symbol symbol=Cell type=typeof Cell
/// @definition.class symbol=Cell template=(in out T)
/// @definition.field symbol=Cell.value source="value: T" key=value type=T
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=(this: &'managed Cell<T>, T) => Cell<T>
/// @type.symbol symbol=Cell.T source=T type=T

    value: T;
    /// @type.symbol symbol=Cell.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

    constructor(value: T) {
    /// @type.symbol symbol=Cell.constructor type=(this: &'managed Cell<T>, T) => Cell<T>
    /// @type.symbol symbol=Cell.constructor.this type=&'managed Cell<T>
    /// @type.symbol symbol=Cell.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Cell type=&'managed Cell<T>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Cell<T>, target=field(receiver=&'managed Cell<T>, target=Cell.value, type=T), type=T" type=T
        /// @resolution.name source=value target=Cell.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare const items: ^int32[];

items.push<int32, "frame">(1);

=== dir ===
declare const items: ^Array<int32>;
/// @type.symbol symbol=items source=items type=^int32[]
/// @resolution.pattern source=items kind=binding target=items
/// @resolution.name source=Array target=Array

items.push(1);
/// @resolution.name source=items target=items
/// @resolution.member source=items.push receiver=^int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=^int32[] target=push
/// @resolution.call source=items.push(1) parameters=(int32[]) arguments=(rest(provided(1) as int32) pack=arrayFromOwnedSlice as int32) return=isize regions=("frame") kind=symbol target=push receiver=^int32[] instance="Array<int32>.<extension#6>.push<\"frame\">"
/// @resolution.place source=items placement="local" lifetime="static" access="immutable"
/// @resolution.access source=items root=items
/// @generic.instantiation id="push<int32, \"frame\">" template=push arguments=(int32, "frame")
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
declare const items: int32[];

items.push<int32, "managed">(1);

=== dir ===
declare const items: Array<int32>;
/// @type.symbol symbol=items source=items type=int32[]
/// @resolution.pattern source=items kind=binding target=items
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @resolution.name source=Array target=Array

items.push(1);
/// @resolution.name source=items target=items
/// @resolution.member source=items.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
/// @resolution.call source=items.push(1) parameters=(int32[]) arguments=(rest(provided(1) as int32) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(&'managed int32[])) instance="Array<int32>.<extension#6>.push<\"managed\" & \"local\">"
/// @resolution.place source=items placement="local" lifetime="static" access="immutable"
/// @resolution.access source=items root=items
/// @generic.instantiation id="push<int32, \"managed\" & \"local\">" template=push arguments=(int32, "managed" & "local")
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instantiation id=push<int32> template=push arguments=(int32)
/// @generic.instance id="push<int32, \"bound0\" & \"local\">" template=push arguments=(int32, "bound0" & "local")
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
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
        "main.tspp",
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
    return identity<'a>(counter);
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
/// @flow.use symbol=counter#1 uses=read+mutable
/// @resolution.name source='a target=identity.'a
/// @resolution.name source=Counter target=Counter
/// @resolution.name source='a target=identity.'a
/// @resolution.name source=Counter target=Counter

    return counter;
    /// @flow.diverging source="return counter"
    /// @resolution.name source=counter target=identity.counter
    /// @resolution.place source=counter placement='a#1 lifetime='a#1 access="mutable"
    /// @resolution.access source=counter root=identity.counter
    /// @flow.access source=counter root=identity.counter uses=read+mutable

}

function forward<'a>(counter: &'a Counter): &'a Counter {
/// @generic.template symbol=forward parameters=('a#2)
/// @type.symbol symbol=forward type=<'a#2>(&'a#2 Counter) => &'a#2 Counter
/// @type.symbol symbol=forward.'a source='a type='a#2
/// @flow.use symbol='a#2 uses=read
/// @type.symbol symbol=forward.counter source="counter: &'a Counter" type=&'a#2 Counter
/// @flow.use symbol=counter#2 uses=read+mutable
/// @resolution.name source='a target=forward.'a
/// @resolution.name source=Counter target=Counter
/// @resolution.name source='a target=forward.'a
/// @resolution.name source=Counter target=Counter

    return identity(counter);
    /// @flow.diverging source="return identity(counter)"
    /// @resolution.name source=identity target=identity
    /// @resolution.call source=identity(counter) parameters=(&'a#2 Counter) arguments=(provided(counter) as &'a#2 Counter) return=&'a#2 Counter regions=('a#2) kind=symbol target=identity instance=identity<'a#2>
    /// @generic.instantiation id=identity<'a#2> template=identity arguments=('a#2)
    /// @resolution.name source=counter target=forward.counter
    /// @resolution.place source=counter placement='a#2 lifetime='a#2 access="mutable"
    /// @resolution.access source=counter root=forward.counter
    /// @flow.access source=counter root=forward.counter uses=read+mutable

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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function grow(): int32[] {
    const values: int32[] = [1, 2, 3];
    values.push<int32, "managed">(4);
    values.fill<int32>(0);

    return values;
}

=== dir ===
function grow(): int32[] {
/// @type.symbol symbol=grow type=() => int32[]

    const values = [1, 2, 3];
    /// @type.symbol symbol=grow.values source=values type=int32[]
    /// @resolution.pattern source=values kind=binding target=grow.values
    /// @resolution.call source=[1, 2, 3] parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32, provided(2) as int32, provided(3) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)

    values.push(4);
    /// @resolution.name source=values target=grow.values
    /// @resolution.member source=values.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
    /// @resolution.call source=values.push(4) parameters=(int32[]) arguments=(rest(provided(4) as int32) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(&'managed int32[])) instance="Array<int32>.<extension#6>.push<\"managed\" & \"local\">"
    /// @resolution.place source=values placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=values root=grow.values
    /// @generic.instantiation id="push<int32, \"managed\" & \"local\">" template=push arguments=(int32, "managed" & "local")
    /// @generic.instantiation id=push<int32> template=push arguments=(int32)

    values.fill(0);
    /// @resolution.name source=values target=grow.values
    /// @resolution.member source=values.fill receiver=int32[] type=(this: int32[], int32, isize | undefined?, isize | undefined?) => int32[] kind=symbol target_receiver=int32[] target=fill#1
    /// @resolution.call source=values.fill(0) parameters=(int32, isize | undefined, isize | undefined) arguments=(provided(0) as int32, omitted as isize | undefined, omitted as isize | undefined) return=int32[] kind=symbol target=fill#1 receiver=int32[] instance=Array<int32>.<extension#4>.fill#1
    /// @resolution.place source=values placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=values root=grow.values
    /// @generic.instantiation id=fill#1<int32> template=fill#1 arguments=(int32)

    return values;
    /// @resolution.name source=values target=grow.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=values root=grow.values

}
"#,
        r#"

"#,
    );
}

/// A mutable borrow of a narrowed inline Copy payload through a handle borrows a readonly copy.
#[test]
fn test_borrow_a_narrowed_copy_payload_mutably_through_a_handle() {
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
        "main.tspp",
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
        const slot: &'managed float64 = &cell.value;
        *slot = 2;
    }
}

=== dir ===
class Cell {
/// @type.symbol symbol=Cell type=typeof Cell
/// @definition.class symbol=Cell
/// @definition.field symbol=Cell.value source="value: string | number" key=value type=string | float64
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=(this: &'managed Cell, string | float64) => Cell

    value: string | number;
    /// @type.symbol symbol=Cell.value source="value: string | number" type=string | float64

    constructor(value: string | number) {
    /// @type.symbol symbol=Cell.constructor type=(this: &'managed Cell, string | float64) => Cell
    /// @type.symbol symbol=Cell.constructor.this type=&'managed Cell
    /// @type.symbol symbol=Cell.constructor.value source="value: string | number" type=string | float64

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Cell type=&'managed Cell
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Cell, target=field(receiver=&'managed Cell, target=Cell.value, type=string | float64), type=string | float64" type=string | float64
        /// @resolution.name source=value target=Cell.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
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
    /// @resolution.place source=cell placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=cell root=bump.cell
    /// @resolution.place source=cell.value placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=cell.value root=bump.cell keys=[value]

        const slot = &cell.value;
        /// @type.symbol symbol=bump.slot source=slot type=&'managed float64
        /// @resolution.pattern source=slot kind=binding target=bump.slot
        /// @resolution.name source=cell target=bump.cell
        /// @resolution.member source=cell.value receiver=Cell type=string | float64 kind=field target_receiver=Cell key=value target=Cell.value target_type=string | float64
        /// @resolution.place source=cell placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=cell root=bump.cell
        /// @resolution.place source=cell.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=cell.value root=bump.cell keys=[value]
        /// @resolution.narrowing source=cell.value union=string | float64 arms=float64

        *slot = 2;
        /// @resolution.pattern.assign source=*slot kind=place
        /// @resolution.place source=*slot placement="local" lifetime="managed" access="readonly"
        /// @resolution.assignment source=*slot write="&'managed Narrow<string | float64, float64> => builtin -> Narrow<string | float64, float64>" type=Narrow<string | float64, float64>
        /// @resolution.name source=slot target=bump.slot
        /// @resolution.place source=slot placement="local" lifetime="managed" access="immutable"
        /// @resolution.access source=slot root=bump.slot

    }
}
"#,
        r#"

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
        "main.tspp",
        DirRows::checked().with_flows(),
        r#"
=== annotated ===
function copy(values: int32[], output: int32[]): void {
    for (let index: isize = 0; index < values.length; index++) {
        output.push<int32, "managed">(values[index]);
    }
}

=== dir ===
function copy(values: int32[], output: int32[]): void {
/// @type.symbol symbol=copy type=(int32[], int32[]) => void
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @type.symbol symbol=copy.values source="values: int32[]" type=int32[]
/// @flow.use symbol=values uses=read
/// @type.symbol symbol=copy.output source="output: int32[]" type=int32[]
/// @flow.use symbol=output uses=read+mutable

    for (let index: isize = 0; index < values.length; index++) {
    /// @type.symbol symbol=copy.index source=index type=isize
    /// @resolution.pattern source=index kind=binding target=copy.index
    /// @flow.use symbol=index uses=read+written
    /// @resolution.name source=index target=copy.index
    /// @resolution.operator source="index < values.length" type=boolean operator="<" kind=builtin operands=[index as isize families=(integer), values.length as isize families=(integer)]
    /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=index root=copy.index
    /// @flow.access source=index root=copy.index uses=read
    /// @resolution.name source=values target=copy.values
    /// @resolution.member source=values.length receiver=int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=copy.values
    /// @generic.instantiation id="length<int32, \"managed\" & \"local\">" template=length arguments=(int32, "managed" & "local")
    /// @generic.instance id="length<int32, \"bound0\" & \"local\">" template=length arguments=(int32, "bound0" & "local")
    /// @flow.access source=values root=copy.values uses=read
    /// @resolution.name source=index target=copy.index
    /// @resolution.assignment source=index read=binding(copy.index) write=binding(copy.index) type=isize
    /// @resolution.access source=index root=copy.index
    /// @resolution.operator source=index++ type=isize operator="++" kind=builtin operands=[index as isize families=(integer)]
    /// @flow.access source=index root=copy.index uses=written

        output.push(values[index]);
        /// @resolution.name source=output target=copy.output
        /// @resolution.member source=output.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
        /// @resolution.call source=output.push(values[index]) parameters=(int32[]) arguments=(rest(provided(values[index]) as int32) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(&'managed int32[])) instance="Array<int32>.<extension#6>.push<\"managed\" & \"local\">"
        /// @resolution.place source=output placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=output root=copy.output
        /// @generic.instantiation id="push<int32, \"managed\" & \"local\">" template=push arguments=(int32, "managed" & "local")
        /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
        /// @generic.instantiation id=push<int32> template=push arguments=(int32)
        /// @generic.instance id="push<int32, \"bound0\" & \"local\">" template=push arguments=(int32, "bound0" & "local")
        /// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
        /// @flow.access source=output root=copy.output uses=read+mutable
        /// @resolution.name source=values target=copy.values
        /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=values root=copy.values
        /// @resolution.subscript source=values[index] type=int32 kind=call target="index#2(parameters=(isize), arguments=(provided(index) as isize), return=int32, regions=(\"managed\" & \"local\"))"
        /// @generic.instantiation id="index#2<int32, \"managed\" & \"local\">" template=index#2 arguments=(int32, "managed" & "local")
        /// @generic.instance id="index#2<int32, \"bound0\" & \"local\">" template=index#2 arguments=(int32, "bound0" & "local")
        /// @flow.access source=values root=copy.values uses=read
        /// @resolution.name source=index target=copy.index
        /// @resolution.place source=index placement="local" lifetime="frame" access="exclusive"
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
        "main.tspp",
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
    /// @resolution.member source=values.map receiver=Iterator<^string> type=<Iterator.map.U>(this: Iterator<^string>, (^string, isize) => Iterator.map.U) => MapIterator<Iterator<^string>, ^string, Iterator.map.U> kind=symbol target_receiver=Iterator<^string> dispatch=dynamic constraint=Iterator<^string> target=Iterator.map
    /// @resolution.call parameters=((^string, isize) => ^string) arguments=(provided(argument) as (^string, isize) => ^string) return=MapIterator<Iterator<^string>, ^string, ^string> kind=dynamic target=Iterator.map receiver=Iterator<^string> constraint=Iterator<^string> generic_arguments=(^string, ^string)
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
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
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=observe.symbol5.value
        /// @flow.access source=value root=observe.symbol5.value uses=read+moved

        value
        /// @resolution.name source=value target=observe.symbol5.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
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
        "main.tspp",
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
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"
/// @flow.use symbol=User uses=read

declare function observe(user: User): void;
/// @type.symbol symbol=observe source="declare function observe(user: User): void" type=(User) => void
/// @flow.use symbol=observe uses=read
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
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=user root=inspect.user
    /// @flow.access source=user root=inspect.user uses=read

    return user;
    /// @flow.diverging source="return user"
    /// @resolution.name source=user target=inspect.user
    /// @resolution.place source=user placement="local" lifetime="frame" access="exclusive"
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
        "main.tspp",
        DirRows::checked().with_flows(),
        r#"
=== annotated ===
function grow(values: int32[]): void {
    values.push<int32, "managed">(0);
}

=== dir ===
function grow(values: int32[]): void {
/// @type.symbol symbol=grow type=(int32[]) => void
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @type.symbol symbol=grow.values source="values: int32[]" type=int32[]
/// @flow.use symbol=values uses=read+mutable

    values.push(0);
    /// @resolution.name source=values target=grow.values
    /// @resolution.member source=values.push receiver=int32[] type=<push.'a>(this: &push.'a int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
    /// @resolution.call source=values.push(0) parameters=(int32[]) arguments=(rest(provided(0) as int32) pack=arrayFromOwnedSlice as int32) return=isize regions=("managed" & "local") kind=symbol target=push receiver=int32[] adjustments=(borrow(&'managed int32[])) instance="Array<int32>.<extension#6>.push<\"managed\" & \"local\">"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=grow.values
    /// @generic.instantiation id="push<int32, \"managed\" & \"local\">" template=push arguments=(int32, "managed" & "local")
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
    /// @generic.instantiation id=push<int32> template=push arguments=(int32)
    /// @generic.instance id="push<int32, \"bound0\" & \"local\">" template=push arguments=(int32, "bound0" & "local")
    /// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
    /// @flow.access source=values root=grow.values uses=read+mutable

}

/// @flow.foreign symbol=push uses=read
"#,
    );
}

/// A readonly borrow of a narrowed inline Copy payload through a handle reads a frame copy.
#[test]
fn test_read_a_narrowed_copy_payload_through_a_handle() {
    let session = TestSession::single(
        r#"
struct Payload {
    value: int32;
}

class Holder {
    slot: ^Payload | undefined;

    constructor(slot: ^Payload | undefined) {
        this.slot = slot;
    }
}

function read(holder: Holder): int32 {
    if (holder.slot !== undefined) {
        const held = &readonly holder.slot;
        return held.value;
    }
    return 0;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
struct Payload {
    value: int32;
}

class Holder {
    slot: Payload | undefined;

    constructor(slot: Payload | undefined) {
        this.slot = slot;
    }
}

function read(holder: Holder): int32 {
    if (holder.slot !== (undefined as Payload | undefined)) {
        const held: &'managed readonly Payload = &readonly holder.slot;
        return held.value;
    }
    return 0;
}

=== dir ===
struct Payload {
/// @type.symbol symbol=Payload type=Payload
/// @definition.struct symbol=Payload
/// @definition.field symbol=Payload.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Payload.value source="value: int32" type=int32

}

class Holder {
/// @type.symbol symbol=Holder type=typeof Holder
/// @definition.class symbol=Holder
/// @definition.field symbol=Holder.slot source="slot: ^Payload | undefined" key=slot type=Payload | undefined
/// @definition.method symbol=Holder.constructor slot=constructor role=constructor type=(this: &'managed Holder, Payload | undefined) => Holder

    slot: ^Payload | undefined;
    /// @type.symbol symbol=Holder.slot source="slot: ^Payload | undefined" type=Payload | undefined
    /// @resolution.name source=Payload target=Payload

    constructor(slot: ^Payload | undefined) {
    /// @type.symbol symbol=Holder.constructor type=(this: &'managed Holder, Payload | undefined) => Holder
    /// @type.symbol symbol=Holder.constructor.this type=&'managed Holder
    /// @type.symbol symbol=Holder.constructor.slot source="slot: ^Payload | undefined" type=Payload | undefined
    /// @resolution.name source=Payload target=Payload

        this.slot = slot;
        /// @resolution.receiver source=this kind=this declaration=Holder type=&'managed Holder
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.slot kind=place
        /// @resolution.place source=this.slot placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.slot root=this keys=[slot]
        /// @resolution.assignment source=this.slot write="receiver=&'managed Holder, target=field(receiver=&'managed Holder, target=Holder.slot, type=Payload | undefined), type=Payload | undefined" type=Payload | undefined
        /// @resolution.name source=slot target=Holder.constructor.slot
        /// @resolution.place source=slot placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=slot root=Holder.constructor.slot

    }
}

function read(holder: Holder): int32 {
/// @type.symbol symbol=read type=(Holder) => int32
/// @type.symbol symbol=read.holder source="holder: Holder" type=Holder
/// @resolution.name source=Holder target=Holder

    if (holder.slot !== undefined) {
    /// @resolution.name source=holder target=read.holder
    /// @resolution.member source=holder.slot receiver=Holder type=Payload | undefined kind=field target_receiver=Holder key=slot target=Holder.slot target_type=Payload | undefined
    /// @resolution.operator source="holder.slot !== undefined" type=boolean operator="!==" kind=builtin operands=[holder.slot as Payload | undefined, undefined as Payload | undefined]
    /// @resolution.place source=holder placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=holder root=read.holder
    /// @resolution.place source=holder.slot placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=holder.slot root=read.holder keys=[slot]

        const held = &readonly holder.slot;
        /// @type.symbol symbol=read.held source=held type=&'managed readonly Payload
        /// @resolution.pattern source=held kind=binding target=read.held
        /// @resolution.name source=holder target=read.holder
        /// @resolution.member source=holder.slot receiver=Holder type=Payload | undefined kind=field target_receiver=Holder key=slot target=Holder.slot target_type=Payload | undefined
        /// @resolution.place source=holder placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=holder root=read.holder
        /// @resolution.place source=holder.slot placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=holder.slot root=read.holder keys=[slot]
        /// @resolution.narrowing source=holder.slot union=Payload | undefined arms=Payload

        return held.value;
        /// @resolution.name source=held target=read.held
        /// @resolution.member source=held.value receiver=&'managed readonly Payload type=int32 kind=field target_receiver=&'managed readonly Payload key=value target=Payload.value target_type=int32
        /// @resolution.place source=held placement="local" lifetime="managed" access="immutable"
        /// @resolution.access source=held root=read.held
        /// @resolution.place source=held.value placement="local" lifetime="managed" access="readonly"
        /// @resolution.access source=held.value root=read.held keys=[value]

    }
    return 0;
}
"#, r#"
"#);
}

/// Reject a borrow of a narrowed inline payload through a handle when the payload is not Copy.
#[test]
fn test_reject_a_borrow_of_a_narrowed_non_copy_payload_through_a_handle() {
    let session = TestSession::single(
        r#"
struct Buffer {
    steps: ^Array<int32>;
}

class Holder {
    slot: ^Buffer | undefined;

    constructor(slot: ^Buffer | undefined) {
        this.slot = slot;
    }
}

function peek(holder: Holder): isize {
    if (holder.slot !== undefined) {
        const held = &readonly holder.slot;
        return held.steps.length;
    }
    return 0;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
struct Buffer {
    steps: ^int32[];
}

class Holder {
    slot: Buffer | undefined;

    constructor(slot: Buffer | undefined) {
        this.slot = slot;
    }
}

function peek(holder: Holder): isize {
    if (holder.slot !== (undefined as Buffer | undefined)) {
        const held: &'managed readonly Buffer = &readonly holder.slot;
        return held.steps.length;
    }
    return 0;
}

=== dir ===
struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.steps source="steps: ^Array<int32>" key=steps type=^int32[]

    steps: ^Array<int32>;
    /// @type.symbol symbol=Buffer.steps source="steps: ^Array<int32>" type=^int32[]
    /// @resolution.name source=Array target=Array

}

class Holder {
/// @type.symbol symbol=Holder type=typeof Holder
/// @definition.class symbol=Holder
/// @definition.field symbol=Holder.slot source="slot: ^Buffer | undefined" key=slot type=Buffer | undefined
/// @definition.method symbol=Holder.constructor slot=constructor role=constructor type=(this: &'managed Holder, Buffer | undefined) => Holder

    slot: ^Buffer | undefined;
    /// @type.symbol symbol=Holder.slot source="slot: ^Buffer | undefined" type=Buffer | undefined
    /// @resolution.name source=Buffer target=Buffer

    constructor(slot: ^Buffer | undefined) {
    /// @type.symbol symbol=Holder.constructor type=(this: &'managed Holder, Buffer | undefined) => Holder
    /// @type.symbol symbol=Holder.constructor.this type=&'managed Holder
    /// @type.symbol symbol=Holder.constructor.slot source="slot: ^Buffer | undefined" type=Buffer | undefined
    /// @resolution.name source=Buffer target=Buffer

        this.slot = slot;
        /// @resolution.receiver source=this kind=this declaration=Holder type=&'managed Holder
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.slot kind=place
        /// @resolution.place source=this.slot placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.slot root=this keys=[slot]
        /// @resolution.assignment source=this.slot write="receiver=&'managed Holder, target=field(receiver=&'managed Holder, target=Holder.slot, type=Buffer | undefined), type=Buffer | undefined" type=Buffer | undefined
        /// @resolution.name source=slot target=Holder.constructor.slot
        /// @resolution.place source=slot placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=slot root=Holder.constructor.slot

    }
}

function peek(holder: Holder): isize {
/// @type.symbol symbol=peek type=(Holder) => isize
/// @type.symbol symbol=peek.holder source="holder: Holder" type=Holder
/// @resolution.name source=Holder target=Holder

    if (holder.slot !== undefined) {
    /// @resolution.name source=holder target=peek.holder
    /// @resolution.member source=holder.slot receiver=Holder type=Buffer | undefined kind=field target_receiver=Holder key=slot target=Holder.slot target_type=Buffer | undefined
    /// @resolution.operator source="holder.slot !== undefined" type=boolean operator="!==" kind=builtin operands=[holder.slot as Buffer | undefined, undefined as Buffer | undefined]
    /// @resolution.place source=holder placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=holder root=peek.holder
    /// @resolution.place source=holder.slot placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=holder.slot root=peek.holder keys=[slot]

        const held = &readonly holder.slot;
        /// @type.symbol symbol=peek.held source=held type=&'managed readonly Buffer
        /// @resolution.pattern source=held kind=binding target=peek.held
        /// @resolution.name source=holder target=peek.holder
        /// @resolution.member source=holder.slot receiver=Holder type=Buffer | undefined kind=field target_receiver=Holder key=slot target=Holder.slot target_type=Buffer | undefined
        /// @resolution.place source=holder placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=holder root=peek.holder
        /// @resolution.place source=holder.slot placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=holder.slot root=peek.holder keys=[slot]
        /// @resolution.narrowing source=holder.slot union=Buffer | undefined arms=Buffer

        return held.steps.length;
        /// @resolution.name source=held target=peek.held
        /// @resolution.member source=held.steps receiver=&'managed readonly Buffer type=readonly ^int32[] kind=field target_receiver=&'managed readonly Buffer key=steps target=Buffer.steps target_type=readonly ^int32[]
        /// @resolution.member source=held.steps.length receiver=readonly ^int32[] type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
        /// @resolution.place source=held placement="local" lifetime="managed" access="immutable"
        /// @resolution.access source=held root=peek.held
        /// @resolution.place source=held.steps placement="local" lifetime="managed" access="readonly"
        /// @resolution.access source=held.steps root=peek.held keys=[steps]
        /// @generic.instantiation id="length<int32, \"managed\" & \"local\">" template=length arguments=(int32, "managed" & "local")

    }
    return 0;
}
"#, r#"
/// @diagnostic.error id=borrow-access-not-granted message="'immutable' access is not granted by a value of type 'Buffer'"
/// @diagnostic.label line=16 column=22 span="&" line_source="const held = &readonly holder.slot;"
/// @diagnostic.note message="the source grants at most 'mutable' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
"#);
}

/// Reject an exclusive borrow through a handle of an object holding owned storage.
#[test]
fn test_reject_an_exclusive_borrow_through_a_handle_of_an_object_holding_owned_storage() {
    let session = TestSession::single(
        r#"
import { Box } from "tspp:memory";

class Bag {
    item: ^Box<int32> = Box.new(0);
}

function edit(bag: Bag): void {
    const owned = &exclusive bag;
    const frozen = &immutable bag;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Box } from "tspp:memory";

class Bag {
    item: Box<int32> = Box.new<int32, "readonly">(0);
}

function edit(bag: Bag): void {
    const owned: &'managed exclusive Bag = &exclusive bag;
    const frozen: &'managed immutable Bag = &immutable bag;
}

=== dir ===
import { Box } from "tspp:memory";

class Bag {
/// @type.symbol symbol=Bag type=typeof Bag
/// @definition.class symbol=Bag
/// @definition.field symbol=Bag.item source="item: ^Box<int32> = Box.new(0)" key=item type=Box<int32>

    item: ^Box<int32> = Box.new(0);
    /// @type.symbol symbol=Bag.item source="item: ^Box<int32> = Box.new(0)" type=Box<int32>
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=Box target=Box
    /// @resolution.member source=Box.new receiver=Box type=(^T#2) => Box<T#2> kind=symbol target_receiver=Box target=new
    /// @resolution.call source=Box.new(0) parameters=(^int32) arguments=(provided(0) as ^int32) return=Box<int32> kind=symbol target=new instance=Box<int32>.<extension#2>.new
    /// @generic.instantiation id="new<int32, \"readonly\">" template=new arguments=(int32, "readonly")

}

function edit(bag: Bag): void {
/// @type.symbol symbol=edit type=(Bag) => void
/// @type.symbol symbol=edit.bag source="bag: Bag" type=Bag
/// @resolution.name source=Bag target=Bag

    const owned = &exclusive bag;
    /// @type.symbol symbol=edit.owned source=owned type=&'managed exclusive Bag
    /// @resolution.pattern source=owned kind=binding target=edit.owned
    /// @resolution.name source=bag target=edit.bag
    /// @resolution.place source=bag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=bag root=edit.bag

    const frozen = &immutable bag;
    /// @type.symbol symbol=edit.frozen source=frozen type=&'managed immutable Bag
    /// @resolution.pattern source=frozen kind=binding target=edit.frozen
    /// @resolution.name source=bag target=edit.bag
    /// @resolution.place source=bag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=bag root=edit.bag

}
"#,
        r#"
/// @diagnostic.error id=borrow-access-not-granted message="'exclusive' access is not granted by a value of type 'Bag'"
/// @diagnostic.label line=9 column=19 span="&" line_source="const owned = &exclusive bag;"
/// @diagnostic.note message="the source grants at most 'mutable' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
/// @diagnostic.error id=borrow-access-not-granted message="'immutable' access is not granted by a value of type 'Bag'"
/// @diagnostic.label line=10 column=20 span="&" line_source="const frozen = &immutable bag;"
/// @diagnostic.note message="the source grants at most 'mutable' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
"#,
    );
}

/// Reject an immutable borrow through a handle of an object holding an owned case.
#[test]
fn test_reject_an_immutable_borrow_through_a_handle_of_an_object_holding_an_owned_case() {
    let session = TestSession::single(
        r#"
import { Box } from "tspp:memory";

class Slot {
    value: ^Box<int32> | undefined = undefined;
}

function view(slot: Slot): void {
    const frozen = &immutable slot;
    const shared = &slot;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Box } from "tspp:memory";

class Slot {
    value: Box<int32> | undefined = undefined as Box<int32> | undefined;
}

function view(slot: Slot): void {
    const frozen: &'managed immutable Slot = &immutable slot;
    const shared: &'managed Slot = &slot;
}

=== dir ===
import { Box } from "tspp:memory";

class Slot {
/// @type.symbol symbol=Slot type=typeof Slot
/// @definition.class symbol=Slot
/// @definition.field symbol=Slot.value source="value: ^Box<int32> | undefined = undefined" key=value type=Box<int32> | undefined

    value: ^Box<int32> | undefined = undefined;
    /// @type.symbol symbol=Slot.value source="value: ^Box<int32> | undefined = undefined" type=Box<int32> | undefined
    /// @resolution.name source=Box target=Box

}

function view(slot: Slot): void {
/// @type.symbol symbol=view type=(Slot) => void
/// @type.symbol symbol=view.slot source="slot: Slot" type=Slot
/// @resolution.name source=Slot target=Slot

    const frozen = &immutable slot;
    /// @type.symbol symbol=view.frozen source=frozen type=&'managed immutable Slot
    /// @resolution.pattern source=frozen kind=binding target=view.frozen
    /// @resolution.name source=slot target=view.slot
    /// @resolution.place source=slot placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=slot root=view.slot

    const shared = &slot;
    /// @type.symbol symbol=view.shared source=shared type=&'managed Slot
    /// @resolution.pattern source=shared kind=binding target=view.shared
    /// @resolution.name source=slot target=view.slot
    /// @resolution.place source=slot placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=slot root=view.slot

}
"#,
        r#"
/// @diagnostic.error id=borrow-access-not-granted message="'immutable' access is not granted by a value of type 'Slot'"
/// @diagnostic.label line=9 column=20 span="&" line_source="const frozen = &immutable slot;"
/// @diagnostic.note message="the source grants at most 'mutable' access"
/// @diagnostic.help message="request the granted access or use a source that grants more"
"#,
    );
}
