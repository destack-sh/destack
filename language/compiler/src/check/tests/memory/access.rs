use crate::tests::{DirRows, TestSession};

#[test]
fn test_overwrite_stable_fields_through_mutable_borrow() {
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

    session.assert_dir_checked(
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

=== checked ===
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
    /// @resolution.pattern.assign source=state.count kind=place place=field(State.count) type=int32

    state.user = state.user;
    /// @resolution.name source=state target=update.state
    /// @resolution.pattern.assign source=state.user kind=place place=field(State.user) type=User
    /// @resolution.name source=state target=update.state
    /// @resolution.member source=state.user receiver=&update.'a State kind=symbol target=State.user

}
"#,
    );
}

#[test]
fn test_overwrite_stable_fields_through_shared_mutable_borrow() {
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

    session.assert_dir_checked(
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

=== checked ===
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
    /// @resolution.pattern.assign source=state.count kind=place place=field(State.count) type=int32

    state.user = state.user;
    /// @resolution.name source=state target=update.state
    /// @resolution.pattern.assign source=state.user kind=place place=field(State.user) type=Placed<User, "shared">
    /// @resolution.name source=state target=update.state
    /// @resolution.member source=state.user receiver=&update.'a State kind=symbol target=State.user

}
"#,
    );
}

#[test]
fn test_reject_unstable_field_overwrite_through_mutable_borrow() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }

struct State { status: Status; }

function update(state: &State): void {
    state.status = Status.Busy;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
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
    /// @resolution.pattern.assign source=state.status kind=place place=field(State.status) type=Status
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Busy receiver=Status kind=symbol target=Status.Busy

}
"#,
        r#"
/// @diagnostic.error id=overwrite-stability-not-satisfied message="type 'Status' is not safe to overwrite through non-exclusive access"
/// @diagnostic.label line=7 column=11 span="status" line_source="state.status = Status.Busy;"
/// @diagnostic.note message="overwriting may invalidate live borrows of the old value"
/// @diagnostic.help message="write through an exclusive or owned path or store an overwrite-stable type"
"#,
    );
}

#[test]
fn test_overwrite_unstable_field_through_exclusive_borrow() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }

struct State { status: Status; }

function update(state: &exclusive State): void {
    state.status = Status.Busy;
}
"#,
    );

    session.assert_dir_checked(
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

function update<'a>(state: &'a exclusive State): void {
    state.status = Status.Busy;
}

=== checked ===
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

function update(state: &exclusive State): void {
/// @generic.template symbol=update parameters=('a)
/// @type.symbol symbol=update type=<update.'a>(&update.'a exclusive State) => void
/// @type.symbol symbol=update.state source="state: &exclusive State" type=&update.'a exclusive State
/// @resolution.name source=State target=State

    state.status = Status.Busy;
    /// @resolution.name source=state target=update.state
    /// @resolution.pattern.assign source=state.status kind=place place=field(State.status) type=Status
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Busy receiver=Status kind=symbol target=Status.Busy

}
"#,
    );
}

#[test]
fn test_overwrite_stable_value_through_mutable_borrow() {
    let session = TestSession::single(
        r#"
function update(value: &int32): void {
    *value = 1;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function update<'a>(value: &'a int32): void {
    *value = 1;
}

=== checked ===
function update(value: &int32): void {
/// @generic.template symbol=update parameters=('a)
/// @type.symbol symbol=update type=<update.'a>(&update.'a int32) => void
/// @type.symbol symbol=update.value source="value: &int32" type=&update.'a int32

    *value = 1;
    /// @resolution.pattern.assign source=*value kind=place place=dereference(direct) type=int32
    /// @resolution.name source=value target=update.value

}
"#,
    );
}

#[test]
fn test_reject_unstable_value_overwrite_through_mutable_borrow() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }

function update(value: &Status): void {
    *value = Status.Busy;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Status {
    Idle,
    Busy,
}

function update<'a>(value: &'a Status): void {
    *value = Status.Busy;
}

=== checked ===
enum Status { Idle, Busy }
/// @type.symbol symbol=Status source="enum Status { Idle, Busy }" type=Status
/// @definition.enum symbol=Status source="enum Status { Idle, Busy }"
/// @definition.variant symbol=Status.Busy source=Busy key=Busy value=1
/// @definition.variant symbol=Status.Idle source=Idle key=Idle value=0
/// @type.symbol symbol=Status.Idle source=Idle type=Status.Idle
/// @type.symbol symbol=Status.Busy source=Busy type=Status.Busy

function update(value: &Status): void {
/// @generic.template symbol=update parameters=('a)
/// @type.symbol symbol=update type=<update.'a>(&update.'a Status) => void
/// @type.symbol symbol=update.value source="value: &Status" type=&update.'a Status
/// @resolution.name source=Status target=Status

    *value = Status.Busy;
    /// @resolution.pattern.assign source=*value kind=place place=dereference(direct) type=Status
    /// @resolution.name source=value target=update.value
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Busy receiver=Status kind=symbol target=Status.Busy

}
"#,
        r#"
/// @diagnostic.error id=overwrite-stability-not-satisfied message="type 'Status' is not safe to overwrite through non-exclusive access"
/// @diagnostic.label line=5 column=5 span="*" line_source="*value = Status.Busy;"
/// @diagnostic.note message="overwriting may invalidate live borrows of the old value"
/// @diagnostic.help message="write through an exclusive or owned path or store an overwrite-stable type"
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function update<'a>(value: &'a readonly int32): void {
    *value = 1;
}

=== checked ===
function update(value: &readonly int32): void {
/// @generic.template symbol=update parameters=('a)
/// @type.symbol symbol=update type=<update.'a>(&update.'a readonly int32) => void
/// @type.symbol symbol=update.value source="value: &readonly int32" type=&update.'a readonly int32

    *value = 1;
    /// @resolution.name source=value target=update.value

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
fn test_overwrite_stable_and_reject_unstable_elements_through_mutable_borrow() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }

function updateNumbers(values: &[int32; 2]): void {
    values[0] = 1;
}

function updateStatuses(values: &[Status; 2]): void {
    values[0] = Status.Busy;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Status {
    Idle,
    Busy,
}

function updateNumbers<'a>(values: &'a [int32; 2]): void {
    values[0] = 1;
}

function updateStatuses<'a>(values: &'a [Status; 2]): void {
    values[0] = Status.Busy;
}

=== checked ===
enum Status { Idle, Busy }
/// @type.symbol symbol=Status source="enum Status { Idle, Busy }" type=Status
/// @definition.enum symbol=Status source="enum Status { Idle, Busy }"
/// @definition.variant symbol=Status.Busy source=Busy key=Busy value=1
/// @definition.variant symbol=Status.Idle source=Idle key=Idle value=0
/// @type.symbol symbol=Status.Idle source=Idle type=Status.Idle
/// @type.symbol symbol=Status.Busy source=Busy type=Status.Busy

function updateNumbers(values: &[int32; 2]): void {
/// @generic.template symbol=updateNumbers parameters=('a)
/// @type.symbol symbol=updateNumbers type=<updateNumbers.'a>(&updateNumbers.'a FixedArray<int32, 2>) => void
/// @type.symbol symbol=updateNumbers.values source="values: &[int32; 2]" type=&updateNumbers.'a FixedArray<int32, 2>

    values[0] = 1;
    /// @resolution.name source=values target=updateNumbers.values
    /// @resolution.pattern.assign source=values[0] kind=place place=subscript(collections.array.indexSet#1) type=int32

}

function updateStatuses(values: &[Status; 2]): void {
/// @generic.template symbol=updateStatuses parameters=('a)
/// @type.symbol symbol=updateStatuses type=<updateStatuses.'a>(&updateStatuses.'a FixedArray<Status, 2>) => void
/// @type.symbol symbol=updateStatuses.values source="values: &[Status; 2]" type=&updateStatuses.'a FixedArray<Status, 2>
/// @resolution.name source=Status target=Status

    values[0] = Status.Busy;
    /// @resolution.name source=values target=updateStatuses.values
    /// @resolution.pattern.assign source=values[0] kind=place place=subscript(collections.array.indexSet#1) type=Status
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Busy receiver=Status kind=symbol target=Status.Busy

}
"#,
        r#"
/// @diagnostic.error id=overwrite-stability-not-satisfied message="type 'Status' is not safe to overwrite through non-exclusive access"
/// @diagnostic.label line=9 column=5 span="values[0]" line_source="values[0] = Status.Busy;"
/// @diagnostic.note message="overwriting may invalidate live borrows of the old value"
/// @diagnostic.help message="write through an exclusive or owned path or store an overwrite-stable type"
"#,
    );
}

#[test]
fn test_overwrite_unstable_field_in_local_but_not_shared_storage() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }

class State {
    status: Status = Status.Idle;
}

declare const localState: local State;
declare const sharedState: shared State;

localState.status = Status.Busy;
sharedState.status = Status.Busy;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
enum Status {
    Idle,
    Busy,
}

class State {
    status: Status = Status.Idle;
}

declare const localState: local State;
declare const sharedState: shared State;

localState.status = Status.Busy;
sharedState.status = Status.Busy;

=== checked ===
enum Status { Idle, Busy }
/// @type.symbol symbol=Status source="enum Status { Idle, Busy }" type=Status
/// @definition.enum symbol=Status source="enum Status { Idle, Busy }"
/// @definition.variant symbol=Status.Busy source=Busy key=Busy value=1
/// @definition.variant symbol=Status.Idle source=Idle key=Idle value=0
/// @type.symbol symbol=Status.Idle source=Idle type=Status.Idle
/// @type.symbol symbol=Status.Busy source=Busy type=Status.Busy

class State {
/// @type.symbol symbol=State type=State
/// @definition.class symbol=State
/// @definition.field symbol=State.status source="status: Status = Status.Idle" key=status type=Status

    status: Status = Status.Idle;
    /// @type.symbol symbol=State.status source="status: Status = Status.Idle" type=Status
    /// @resolution.name source=Status target=Status
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Idle receiver=Status kind=symbol target=Status.Idle

}

declare const localState: local State;
/// @type.symbol symbol=localState source=localState type=Placed<State, "local">
/// @resolution.pattern source=localState kind=binding target=localState
/// @resolution.name source=State target=State

declare const sharedState: shared State;
/// @type.symbol symbol=sharedState source=sharedState type=Placed<State, "shared">
/// @resolution.pattern source=sharedState kind=binding target=sharedState
/// @resolution.name source=State target=State

localState.status = Status.Busy;
/// @resolution.name source=localState target=localState
/// @resolution.pattern.assign source=localState.status kind=place place=field(State.status) type=Status
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Busy receiver=Status kind=symbol target=Status.Busy

sharedState.status = Status.Busy;
/// @resolution.name source=sharedState target=sharedState
/// @resolution.pattern.assign source=sharedState.status kind=place place=field(State.status) type=Placed<Status, "shared">
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Busy receiver=Status kind=symbol target=Status.Busy
"#,
        r#"
/// @diagnostic.error id=overwrite-stability-not-satisfied message="type 'shared Status' is not safe to overwrite through non-exclusive access"
/// @diagnostic.label line=12 column=13 span="status" line_source="sharedState.status = Status.Busy;"
/// @diagnostic.note message="overwriting may invalidate live borrows of the old value"
/// @diagnostic.help message="write through an exclusive or owned path or store an overwrite-stable type"
"#,
    );
}

#[test]
fn test_overwrite_unstable_field_through_owned_value() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }

struct State { status: Status; }

declare const state: ^State;

state.status = Status.Busy;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
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

declare const state: ^State;

state.status = Status.Busy;

=== checked ===
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

declare const state: ^State;
/// @type.symbol symbol=state source=state type=Owned<State> reduced=State
/// @resolution.pattern source=state kind=binding target=state
/// @resolution.name source=State target=State

state.status = Status.Busy;
/// @resolution.name source=state target=state
/// @resolution.pattern.assign source=state.status kind=place place=field(State.status) type=Status
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Busy receiver=Status kind=symbol target=Status.Busy
"#,
        r#"
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
shared class Cell<in out T> {
    value: T;

    constructor(value: T): this {
        this.value = value;
    }
}

=== checked ===
shared class Cell<T> {
/// @generic.template symbol=Cell parameters=(in out T)
/// @type.symbol symbol=Cell type=Cell
/// @definition.class symbol=Cell template=(in out T)
/// @definition.field symbol=Cell.value source="value: T" key=value type=T
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=(T) => this
/// @type.symbol symbol=Cell.T source=T type=T

    value: T;
    /// @type.symbol symbol=Cell.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

    constructor(value: T) {
    /// @type.symbol symbol=Cell.constructor type=(T) => this
    /// @type.symbol symbol=Cell.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T>
        /// @resolution.pattern.assign source=this.value kind=place place=field(Cell.value) type=T
        /// @resolution.name source=value target=Cell.constructor.value

    }
}
"#,
    );
}
