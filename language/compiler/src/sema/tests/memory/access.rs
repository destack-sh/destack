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

function update<'a>(state: &'a exclusive State): void {
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

function update(state: &exclusive State): void {
/// @generic.template symbol=update parameters=('a)
/// @type.symbol symbol=update type=<update.'a>(&update.'a exclusive State) => void
/// @type.symbol symbol=update.state source="state: &exclusive State" type=&update.'a exclusive State
/// @resolution.name source=State target=State

    state.status = Status.Busy;
    /// @resolution.name source=state target=update.state
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="exclusive"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.status kind=place
    /// @resolution.access source=state.status root=update.state keys=[status]
    /// @resolution.assignment source=state.status write="receiver=&update.'a exclusive State, target=field(receiver=&update.'a exclusive State, target=State.status, type=Status), type=Status" type=Status
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy

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
fn test_reject_unstable_value_overwrite_through_mutable_borrow() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }

function update(value: &Status): void {
    *value = Status.Busy;
}
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

function update<'a>(value: &'a Status): void {
    *value = Status.Busy;
}

=== dir ===
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
    /// @resolution.pattern.assign source=*value kind=place
    /// @resolution.assignment source=*value write="&update.'a Status => direct -> Status" type=Status
    /// @resolution.name source=value target=update.value
    /// @resolution.place source=value placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=value root=update.value
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy

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

    session.assert_dir_and_diagnostics(
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

=== dir ===
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
    /// @resolution.place source=values placement=updateNumbers.'a lifetime=updateNumbers.'a access="mutable"
    /// @resolution.access source=values root=updateNumbers.values
    /// @resolution.pattern.assign source=values[0] kind=place
    /// @resolution.assignment source=values[0] write="indexSet#1(parameters=(isize, int32), arguments=(provided(0) as isize, supplied as int32), return=void)" type=int32
    /// @generic.instantiation id="indexSet#1<int32, 2>" template=indexSet#1 arguments=(int32, 2)

}

function updateStatuses(values: &[Status; 2]): void {
/// @generic.template symbol=updateStatuses parameters=('a)
/// @type.symbol symbol=updateStatuses type=<updateStatuses.'a>(&updateStatuses.'a FixedArray<Status, 2>) => void
/// @type.symbol symbol=updateStatuses.values source="values: &[Status; 2]" type=&updateStatuses.'a FixedArray<Status, 2>
/// @resolution.name source=Status target=Status

    values[0] = Status.Busy;
    /// @resolution.name source=values target=updateStatuses.values
    /// @resolution.place source=values placement=updateStatuses.'a lifetime=updateStatuses.'a access="mutable"
    /// @resolution.access source=values root=updateStatuses.values
    /// @resolution.pattern.assign source=values[0] kind=place
    /// @resolution.assignment source=values[0] write="indexSet#1(parameters=(isize, Status), arguments=(provided(0) as isize, supplied as Status), return=void)" type=Status
    /// @generic.instantiation id="indexSet#1<Status, 2>" template=indexSet#1 arguments=(Status, 2)
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy

}
"#,
        r#"
/// @diagnostic.error id=overwrite-stability-not-satisfied message="type 'Status' is not safe to overwrite through non-exclusive access"
/// @diagnostic.label line=9 column=11 span="[" line_source="values[0] = Status.Busy;"
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
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
    /// @resolution.member source=Status.Idle receiver=Status type=Status.Idle kind=symbol target_receiver=Status target=Status.Idle

}

declare const localState: local State;
/// @type.symbol symbol=localState source=localState type=local State
/// @resolution.pattern source=localState kind=binding target=localState
/// @resolution.name source=State target=State

declare const sharedState: shared State;
/// @type.symbol symbol=sharedState source=sharedState type=shared State
/// @resolution.pattern source=sharedState kind=binding target=sharedState
/// @resolution.name source=State target=State

localState.status = Status.Busy;
/// @resolution.name source=localState target=localState
/// @resolution.place source=localState placement="local" lifetime="managed" access="exclusive"
/// @resolution.access source=localState root=localState
/// @resolution.pattern.assign source=localState.status kind=place
/// @resolution.access source=localState.status root=localState keys=[status]
/// @resolution.assignment source=localState.status write="receiver=local State, target=field(receiver=local State, target=State.status, type=Status), type=Status" type=Status
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy

sharedState.status = Status.Busy;
/// @resolution.name source=sharedState target=sharedState
/// @resolution.place source=sharedState placement="shared" lifetime="managed" access="mutable"
/// @resolution.access source=sharedState root=sharedState
/// @resolution.pattern.assign source=sharedState.status kind=place
/// @resolution.access source=sharedState.status root=sharedState keys=[status]
/// @resolution.assignment source=sharedState.status write="receiver=shared State, target=field(receiver=shared State, target=State.status, type=Status), type=Status" type=Status
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy
"#,
        r#"
/// @diagnostic.error id=overwrite-stability-not-satisfied message="type 'Status' is not safe to overwrite through non-exclusive access"
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
/// @resolution.place source=state placement="local" lifetime="static" access="exclusive"
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
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=<Cell.constructor.P0: Place>(T) => Managed<this, Cell.constructor.P0>
/// @type.symbol symbol=Cell.T source=T type=T

    value: T;
    /// @type.symbol symbol=Cell.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

    constructor(value: T) {
    /// @generic.template symbol=Cell.constructor parent=template#0 parameters=(P0: Place)
    /// @type.symbol symbol=Cell.constructor type=<Cell.constructor.P0: Place>(T) => Managed<this, Cell.constructor.P0>
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
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Cell.constructor.value

    }
}
"#,
    );
}

#[test]
fn test_reject_unstable_field_overwrite_through_generic_borrow() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }

function update<T: { status: Status }>(state: &T): void {
    state.status = Status.Busy;
}
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

function update<T: { status: Status }, 'a>(state: &'a T): void {
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

function update<T: { status: Status }>(state: &T): void {
/// @generic.template symbol=update parameters=(T: { status: Status }, 'a)
/// @type.symbol symbol=update type=<T: { status: Status }, update.'a>(&update.'a T) => void
/// @type.symbol symbol=update.T source="T: { status: Status }" type=T
/// @type.symbol symbol=update.status source="status: Status" type=Status
/// @resolution.name source=Status target=Status
/// @type.symbol symbol=update.state source="state: &T" type=&update.'a T
/// @resolution.name source=T target=update.T

    state.status = Status.Busy;
    /// @resolution.name source=state target=update.state
    /// @resolution.place source=state placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.status kind=place
    /// @resolution.access source=state.status root=update.state keys=[status]
    /// @resolution.assignment source=state.status write="receiver=&update.'a T, target=field(receiver=&update.'a T, target=status, type=Status), type=Status" type=Status
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy

}
"#,
        r#"
/// @diagnostic.error id=overwrite-stability-not-satisfied message="type 'Status' is not safe to overwrite through non-exclusive access"
/// @diagnostic.label line=5 column=11 span="status" line_source="state.status = Status.Busy;"
/// @diagnostic.note message="overwriting may invalidate live borrows of the old value"
/// @diagnostic.help message="write through an exclusive or owned path or store an overwrite-stable type"
"#,
    );
}

#[test]
fn test_reject_exclusive_receiver_on_const_owned_array() {
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
/// @resolution.member source=items.push receiver=^int32[] type=<push.'a>(this: &push.'a exclusive int32[], ...int32[]) => isize kind=symbol target_receiver=^int32[] target=push
/// @resolution.call source=items.push(1) parameters=(int32[]) arguments=(rest(1) pack=arrayFromOwnedSlice as int32) return=isize kind=symbol target=push receiver=^int32[] instance=Array<int32>.<extension#6>.push
/// @resolution.place source=items placement="constant" lifetime="static" access="readonly"
/// @resolution.access source=items root=items
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instantiation id=push<int32> template=push arguments=(int32)
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '^int32[]' is not assignable to the method's 'this' type '&exclusive int32[]'"
/// @diagnostic.label line=4 column=1 span="items.push(1)" line_source="items.push(1);"
"#,
    );
}

#[test]
fn test_accept_exclusive_receiver_on_const_managed_array() {
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
/// @generic.instance id="initAsPointer<int32, \"exclusive\">" template=initAsPointer arguments=(int32, "exclusive") evaluated=(<initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(WithAccess<&initAsPointer.'a MaybeUninit<initAsPointer.T>, initAsPointer.A>) => Raw<initAsPointer.T> => <initAsPointer.T, const initAsPointer.A: Access = "mutable", initAsPointer.'a>(&initAsPointer.'a exclusive MaybeUninit<int32>) => Raw<int32>)
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=MaybeUninit<MaybeUninit<int32>> template=MaybeUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=MaybeUninit<int32> template=MaybeUninit arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32) evaluated=((WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive">) => Raw<assumeInitDrop.T> => (&assumeInitDrop.'a exclusive MaybeUninit<int32>) => Raw<int32>, WithAccess<&assumeInitDrop.'a MaybeUninit<assumeInitDrop.T>, "exclusive"> => &assumeInitDrop.'a exclusive MaybeUninit<int32>)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=new<MaybeUninit<int32>> template=new arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32) evaluated=(WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => &truncate.'a exclusive MaybeUninit<int32>, (WithAccess<&truncate.'a T#6[], "exclusive">, usize) => WithAccess<&truncate.'a MaybeUninit<T#6>, "exclusive"> => (&truncate.'a exclusive int32[], usize) => &truncate.'a exclusive MaybeUninit<int32>, WithAccess<&truncate.'a T#6[], "exclusive"> => &truncate.'a exclusive int32[])
/// @resolution.name source=Array target=Array

items.push(1);
/// @resolution.name source=items target=items
/// @resolution.member source=items.push receiver=int32[] type=<push.'a>(this: &push.'a exclusive int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
/// @resolution.call source=items.push(1) parameters=(int32[]) arguments=(rest(1) pack=arrayFromOwnedSlice as int32) return=isize kind=symbol target=push receiver=int32[] adjustments=(borrow(Borrowed<int32[], "managed" & "local", "exclusive">)) instance=Array<int32>.<extension#6>.push
/// @resolution.place source=items placement="local" lifetime="managed" access="exclusive"
/// @resolution.access source=items root=items
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instantiation id=push<int32> template=push arguments=(int32)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
/// @generic.instance id="elementSlot<int32, \"exclusive\">" template=elementSlot arguments=(int32, "exclusive") evaluated=(<elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => <elementSlot.T, const elementSlot.A: Access = "readonly", elementSlot.'a>(&elementSlot.'a exclusive int32[], usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int32>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int32>>, usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int32>>, WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => &elementSlot.'a exclusive MaybeUninit<int32>, (WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A>, usize) => WithAccess<&elementSlot.'a MaybeUninit<elementSlot.T>, elementSlot.A> => (&elementSlot.'a exclusive Slice<MaybeUninit<int32>>, usize) => &elementSlot.'a exclusive MaybeUninit<int32>, WithAccess<&elementSlot.'a elementSlot.T[], elementSlot.A> => &elementSlot.'a exclusive int32[], WithAccess<&elementSlot.'a Slice<MaybeUninit<elementSlot.T>>, elementSlot.A> => &elementSlot.'a exclusive Slice<MaybeUninit<int32>>)
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"exclusive\">" template=sliceIndex arguments=(MaybeUninit<int32>, "exclusive") evaluated=(<sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(WithAccess<&sliceIndex.'a Slice<sliceIndex.T>, sliceIndex.A>, usize) => WithAccess<&sliceIndex.'a sliceIndex.T, sliceIndex.A> => <sliceIndex.T, const sliceIndex.A: Access = "readonly", sliceIndex.'a>(&sliceIndex.'a exclusive Slice<MaybeUninit<int32>>, usize) => &sliceIndex.'a exclusive MaybeUninit<int32>)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
/// @generic.instance id=Slice<int32> template=Slice arguments=(int32)
/// @generic.instance id=append<int32> template=append arguments=(int32) evaluated=(WithAccess<&append.'b MaybeUninit<T#6>, "exclusive"> => &append.'b exclusive MaybeUninit<int32>, (WithAccess<&append.'b T#6[], "exclusive">, usize) => WithAccess<&append.'b MaybeUninit<T#6>, "exclusive"> => (&append.'b exclusive int32[], usize) => &append.'b exclusive MaybeUninit<int32>, WithAccess<&append.'b T#6[], "exclusive"> => &append.'b exclusive int32[], WithAccess<&append.'a MaybeUninit<T#6>, "exclusive"> => &append.'a exclusive MaybeUninit<int32>, (WithAccess<&append.'a T#6[], "exclusive">, usize) => WithAccess<&append.'a MaybeUninit<T#6>, "exclusive"> => (&append.'a exclusive int32[], usize) => &append.'a exclusive MaybeUninit<int32>, WithAccess<&append.'a T#6[], "exclusive"> => &append.'a exclusive int32[])
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=assumeInitRead#1<int32> template=assumeInitRead#1 arguments=(int32)
/// @generic.instance id=assumeInitRead<int32> template=assumeInitRead arguments=(int32)
/// @generic.instance id=fromOwnedSlice<int32> template=fromOwnedSlice arguments=(int32)
/// @generic.instance id=initWrite<int32> template=initWrite arguments=(int32)
/// @generic.instance id=intoUninit<int32> template=intoUninit arguments=(int32)
/// @generic.instance id=push<int32> template=push arguments=(int32)
/// @generic.instance id=reserve<int32> template=reserve arguments=(int32) evaluated=(WithAccess<&reserve.'a MaybeUninit<T#6>, "exclusive"> => &reserve.'a exclusive MaybeUninit<int32>, (WithAccess<&reserve.'a T#6[], "exclusive">, usize) => WithAccess<&reserve.'a MaybeUninit<T#6>, "exclusive"> => (&reserve.'a exclusive int32[], usize) => &reserve.'a exclusive MaybeUninit<int32>, WithAccess<&reserve.'a T#6[], "exclusive"> => &reserve.'a exclusive int32[], (WithAccess<&'frame Slice<MaybeUninit<T#6>>, "exclusive">, usize) => WithAccess<&'frame MaybeUninit<T#6>, "exclusive"> => (&'frame exclusive Slice<MaybeUninit<int32>>, usize) => &'frame exclusive MaybeUninit<int32>, WithAccess<&'frame Slice<MaybeUninit<T#6>>, "exclusive"> => &'frame exclusive Slice<MaybeUninit<int32>>, (WithAccess<&'frame Slice<MaybeUninit<T#6>>, "exclusive">, usize) => WithAccess<&'frame MaybeUninit<T#6>, "exclusive"> => (&'frame exclusive Slice<MaybeUninit<int32>>, usize) => &'frame exclusive MaybeUninit<int32>, WithAccess<&'frame Slice<MaybeUninit<T#6>>, "exclusive"> => &'frame exclusive Slice<MaybeUninit<int32>>)
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
fn test_record_an_exclusive_use_for_a_returned_exclusive_borrow() {
    let session = TestSession::single(
        r#"
struct Counter {
    value: int32;
}

function identity<'a>(counter: &'a exclusive Counter): &'a exclusive Counter {
    return counter;
}

function forward<'a>(counter: &'a exclusive Counter): &'a exclusive Counter {
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

function identity<'a>(counter: &'a exclusive Counter): &'a exclusive Counter {
    return counter;
}

function forward<'a>(counter: &'a exclusive Counter): &'a exclusive Counter {
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

function identity<'a>(counter: &'a exclusive Counter): &'a exclusive Counter {
/// @generic.template symbol=identity parameters=('a#1)
/// @type.symbol symbol=identity type=<'a#1>(&'a#1 exclusive Counter) => &'a#1 exclusive Counter
/// @flow.use symbol=identity uses=read
/// @type.symbol symbol=identity.'a source='a type='a#1
/// @flow.use symbol='a#1 uses=read
/// @type.symbol symbol=identity.counter source="counter: &'a exclusive Counter" type=&'a#1 exclusive Counter
/// @flow.use symbol=counter#1 uses=read+exclusive
/// @resolution.name source='a target=identity.'a
/// @resolution.name source=Counter target=Counter
/// @resolution.name source='a target=identity.'a
/// @resolution.name source=Counter target=Counter

    return counter;
    /// @flow.diverging source="return counter"
    /// @resolution.name source=counter target=identity.counter
    /// @resolution.place source=counter placement='a#1 lifetime='a#1 access="exclusive"
    /// @resolution.access source=counter root=identity.counter
    /// @flow.access source=counter root=identity.counter uses=read+exclusive

}

function forward<'a>(counter: &'a exclusive Counter): &'a exclusive Counter {
/// @generic.template symbol=forward parameters=('a#2)
/// @type.symbol symbol=forward type=<'a#2>(&'a#2 exclusive Counter) => &'a#2 exclusive Counter
/// @type.symbol symbol=forward.'a source='a type='a#2
/// @flow.use symbol='a#2 uses=read
/// @type.symbol symbol=forward.counter source="counter: &'a exclusive Counter" type=&'a#2 exclusive Counter
/// @flow.use symbol=counter#2 uses=read+exclusive
/// @resolution.name source='a target=forward.'a
/// @resolution.name source=Counter target=Counter
/// @resolution.name source='a target=forward.'a
/// @resolution.name source=Counter target=Counter

    return identity(counter);
    /// @flow.diverging source="return identity(counter)"
    /// @resolution.name source=identity target=identity
    /// @resolution.call source=identity(counter) parameters=(&'a#2 exclusive Counter) arguments=(provided(counter) as &'a#2 exclusive Counter) return=&'a#2 exclusive Counter kind=symbol target=identity
    /// @resolution.name source=counter target=forward.counter
    /// @resolution.place source=counter placement='a#2 lifetime='a#2 access="exclusive"
    /// @resolution.access source=counter root=forward.counter
    /// @flow.access source=counter root=forward.counter uses=read+exclusive

}
"#,
        r#"
"#,
    );
}

/// A const handle keeps its referent writable, so its methods borrow exclusively.
#[test]
fn test_borrow_a_const_array_literal_exclusively_for_a_method() {
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
    /// @resolution.member source=values.push receiver=int32[] type=<push.'a>(this: &push.'a exclusive int32[], ...int32[]) => isize kind=symbol target_receiver=int32[] target=push
    /// @resolution.call source=values.push(4) parameters=(int32[]) arguments=(rest(4) pack=arrayFromOwnedSlice as int32) return=isize kind=symbol target=push receiver=int32[] adjustments=(borrow(&'frame exclusive int32[])) instance=Array<int32>.<extension#6>.push
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=grow.values
    /// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
    /// @generic.instantiation id=push<int32> template=push arguments=(int32)

    values.fill(0);
    /// @resolution.name source=values target=grow.values
    /// @resolution.member source=values.fill receiver=int32[] type=<fill.'a>(this: &fill.'a exclusive int32[], int32, isize | undefined?, isize | undefined?) => int32[] kind=symbol target_receiver=int32[] target=fill
    /// @resolution.call source=values.fill(0) parameters=(int32, isize | undefined, isize | undefined) arguments=(provided(0) as int32, omitted as isize | undefined, omitted as isize | undefined) return=int32[] kind=symbol target=fill receiver=int32[] adjustments=(borrow(Borrowed<int32[], "managed" & "local", "exclusive">)) instance=Array<int32>.<extension#6>.fill
    /// @resolution.place source=values placement="local" lifetime="managed" access="exclusive"
    /// @resolution.access source=values root=grow.values
    /// @generic.instantiation id=fill<int32> template=fill arguments=(int32)

    return values;
    /// @resolution.name source=values target=grow.values
    /// @resolution.place source=values placement="local" lifetime="managed" access="exclusive"
    /// @resolution.access source=values root=grow.values

}
"#,
        r#"
"#,
    );
}
