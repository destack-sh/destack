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
    /// @resolution.place source=state placement="local" lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.count kind=place
    /// @resolution.access source=state.count root=update.state keys=[count]
    /// @resolution.assignment source=state.count write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.count, type=int32), type=int32" type=int32

    state.user = state.user;
    /// @resolution.name source=state target=update.state
    /// @resolution.place source=state placement="local" lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.user kind=place
    /// @resolution.access source=state.user root=update.state keys=[user]
    /// @resolution.assignment source=state.user write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.user, type=User), type=User" type=User
    /// @resolution.name source=state target=update.state
    /// @resolution.member source=state.user receiver=&update.'a State type=User kind=field target_receiver=&update.'a State key=user target=State.user target_type=User
    /// @resolution.place source=state placement="local" lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.place source=state.user placement="local" lifetime=update.'a access="mutable"
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
    /// @resolution.place source=state placement="shared" lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.count kind=place
    /// @resolution.access source=state.count root=update.state keys=[count]
    /// @resolution.assignment source=state.count write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.count, type=int32), type=int32" type=int32

    state.user = state.user;
    /// @resolution.name source=state target=update.state
    /// @resolution.place source=state placement="shared" lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.pattern.assign source=state.user kind=place
    /// @resolution.access source=state.user root=update.state keys=[user]
    /// @resolution.assignment source=state.user write="receiver=&update.'a State, target=field(receiver=&update.'a State, target=State.user, type=Placed<User, \"shared\">), type=Placed<User, \"shared\">" type=Placed<User, "shared">
    /// @resolution.name source=state target=update.state
    /// @resolution.member source=state.user receiver=&update.'a State type=Placed<User, "shared"> kind=field target_receiver=&update.'a State key=user target=State.user target_type=Placed<User, "shared">
    /// @resolution.place source=state placement="shared" lifetime=update.'a access="mutable"
    /// @resolution.access source=state root=update.state
    /// @resolution.place source=state.user placement="shared" lifetime=update.'a access="mutable"
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
    /// @resolution.place source=state placement="local" lifetime=update.'a access="mutable"
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
    /// @resolution.place source=state placement="local" lifetime=update.'a access="exclusive"
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
    /// @resolution.place source=value placement="local" lifetime=update.'a access="mutable"
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
    /// @resolution.place source=value placement="local" lifetime=update.'a access="mutable"
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
    /// @resolution.place source=value placement="local" lifetime=update.'a access="readonly"
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
    /// @resolution.place source=values placement="local" lifetime=updateNumbers.'a access="mutable"
    /// @resolution.access source=values root=updateNumbers.values
    /// @resolution.pattern.assign source=values[0] kind=place
    /// @resolution.assignment source=values[0] write="collections.fixed-array.indexSet#1(parameters=(isize, int32), arguments=(provided(0) as isize, write as int32), return=void)" type=int32
    /// @generic.instantiation id="collections.fixed-array.indexSet#1<int32, 2>" template=collections.fixed-array.indexSet#1 arguments=(int32, 2)

}

function updateStatuses(values: &[Status; 2]): void {
/// @generic.template symbol=updateStatuses parameters=('a)
/// @type.symbol symbol=updateStatuses type=<updateStatuses.'a>(&updateStatuses.'a FixedArray<Status, 2>) => void
/// @type.symbol symbol=updateStatuses.values source="values: &[Status; 2]" type=&updateStatuses.'a FixedArray<Status, 2>
/// @resolution.name source=Status target=Status

    values[0] = Status.Busy;
    /// @resolution.name source=values target=updateStatuses.values
    /// @resolution.place source=values placement="local" lifetime=updateStatuses.'a access="mutable"
    /// @resolution.access source=values root=updateStatuses.values
    /// @resolution.pattern.assign source=values[0] kind=place
    /// @resolution.assignment source=values[0] write="collections.fixed-array.indexSet#1(parameters=(isize, Status), arguments=(provided(0) as isize, write as Status), return=void)" type=Status
    /// @generic.instantiation id="collections.fixed-array.indexSet#1<Status, 2>" template=collections.fixed-array.indexSet#1 arguments=(Status, 2)
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
/// @type.symbol symbol=localState source=localState type=Placed<State, "local">
/// @resolution.pattern source=localState kind=binding target=localState
/// @resolution.name source=State target=State

declare const sharedState: shared State;
/// @type.symbol symbol=sharedState source=sharedState type=Placed<State, "shared">
/// @resolution.pattern source=sharedState kind=binding target=sharedState
/// @resolution.name source=State target=State

localState.status = Status.Busy;
/// @resolution.name source=localState target=localState
/// @resolution.place source=localState placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=localState root=localState
/// @resolution.pattern.assign source=localState.status kind=place
/// @resolution.access source=localState.status root=localState keys=[status]
/// @resolution.assignment source=localState.status write="receiver=Placed<State, \"local\">, target=field(receiver=Placed<State, \"local\">, target=State.status, type=Status), type=Status" type=Status
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy

sharedState.status = Status.Busy;
/// @resolution.name source=sharedState target=sharedState
/// @resolution.place source=sharedState placement="shared" lifetime="static" access="mutable"
/// @resolution.access source=sharedState root=sharedState
/// @resolution.pattern.assign source=sharedState.status kind=place
/// @resolution.access source=sharedState.status root=sharedState keys=[status]
/// @resolution.assignment source=sharedState.status write="receiver=Placed<State, \"shared\">, target=field(receiver=Placed<State, \"shared\">, target=State.status, type=Placed<Status, \"shared\">), type=Placed<Status, \"shared\">" type=Placed<Status, "shared">
/// @resolution.name source=Status target=Status
/// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy
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

declare let state: ^State;

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
/// @type.symbol symbol=state source=state type=Owned<State>
/// @resolution.pattern source=state kind=binding target=state
/// @resolution.name source=State target=State

state.status = Status.Busy;
/// @resolution.name source=state target=state
/// @resolution.place source=state placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=state root=state
/// @resolution.pattern.assign source=state.status kind=place
/// @resolution.access source=state.status root=state keys=[status]
/// @resolution.assignment source=state.status write="receiver=Owned<State>, target=field(receiver=Owned<State>, target=State.status, type=Status), type=Status" type=Status
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
/// @resolution.place source=state placement="local" lifetime="static" access="readonly"
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

    constructor(value: T): this {
        this.value = value;
    }
}

=== dir ===
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
/// @resolution.name source=Status target=Status
/// @type.symbol symbol=update.state source="state: &T" type=&update.'a T
/// @resolution.name source=T target=update.T

    state.status = Status.Busy;
    /// @resolution.name source=state target=update.state
    /// @resolution.place source=state placement="local" lifetime=update.'a access="mutable"
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
declare const items: ^Array<int32>;

items.push<int32>(1);

=== dir ===
declare const items: ^Array<int32>;
/// @type.symbol symbol=items source=items type=Owned<Array<int32>>
/// @resolution.pattern source=items kind=binding target=items
/// @resolution.name source=Array target=collections.array.Array

items.push(1);
/// @resolution.name source=items target=items
/// @resolution.member source=items.push receiver=Owned<Array<int32>> type=<collections.array.push.'a>(this: &collections.array.push.'a exclusive Owned<Array<int32>>, ...int32[]) => isize kind=symbol target_receiver=Owned<Array<int32>> target=collections.array.push
/// @resolution.call source=items.push(1) parameters=(Array<int32>) arguments=(rest(1) pack=collections.array.arrayFromSlice as int32) return=isize kind=symbol target=collections.array.push receiver=Owned<Array<int32>> instance=Array<int32>.<extension#5>.push
/// @resolution.place source=items placement="local" lifetime="static" access="readonly"
/// @resolution.access source=items root=items
/// @generic.instantiation id=collections.array.arrayFromSlice<int32> template=collections.array.arrayFromSlice arguments=(int32)
/// @generic.instantiation id=collections.array.push<int32> template=collections.array.push arguments=(int32)
"#,
        r#"
/// @diagnostic.error id=receiver-not-assignable message="receiver type '^Array<int32>' is not assignable to the method's 'this' type '&exclusive ^Array<int32>'"
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
/// @type.symbol symbol=items source=items type=Array<int32>
/// @resolution.pattern source=items kind=binding target=items
/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
/// @generic.instance id=memory.init.MaybeUninit<int32> template=memory.init.MaybeUninit arguments=(int32)
/// @generic.instance id=memory.raw.dangling<memory.init.MaybeUninit<int32>> template=memory.raw.dangling arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.Unique<Slice<memory.init.MaybeUninit<int32>>> template=memory.unique.Unique arguments=(Slice<memory.init.MaybeUninit<int32>>)
/// @generic.instance id=memory.unique.empty<memory.init.MaybeUninit<int32>> template=memory.unique.empty arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.emptyUniqueSlice<memory.init.MaybeUninit<int32>> template=memory.unique.emptyUniqueSlice arguments=(memory.init.MaybeUninit<int32>)
/// @generic.instance id=memory.unique.uniqueSliceFromRaw<memory.init.MaybeUninit<int32>> template=memory.unique.uniqueSliceFromRaw arguments=(memory.init.MaybeUninit<int32>)
/// @resolution.name source=Array target=collections.array.Array

items.push(1);
/// @resolution.name source=items target=items
/// @resolution.member source=items.push receiver=Array<int32> type=<collections.array.push.'a>(this: &collections.array.push.'a exclusive Array<int32>, ...int32[]) => isize kind=symbol target_receiver=Array<int32> target=collections.array.push
/// @resolution.call source=items.push(1) parameters=(Array<int32>) arguments=(rest(1) pack=collections.array.arrayFromSlice as int32) return=isize kind=symbol target=collections.array.push receiver=Array<int32> adjustments=(borrow(&'static exclusive Array<int32>)) instance=Array<int32>.<extension#5>.push
/// @resolution.place source=items placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=items root=items
/// @generic.instantiation id=collections.array.arrayFromSlice<int32> template=collections.array.arrayFromSlice arguments=(int32)
/// @generic.instantiation id=collections.array.push<int32> template=collections.array.push arguments=(int32)
/// @generic.instance id=collections.array.arrayFromSlice<int32> template=collections.array.arrayFromSlice arguments=(int32)
/// @generic.instance id=collections.array.push<int32> template=collections.array.push arguments=(int32)
"#,
    );
}
