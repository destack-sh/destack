use crate::tests::TestSession;

/// Lower a shared pinned class into shared reference storage and a shared allocation.
#[test]
fn test_lower_shared_class_into_shared_storage() {
    let session = TestSession::single(
        r#"
export shared class Counter {
    value: int32 = 0;
}

export function make(): shared Counter {
    return new Counter();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Counter {
    value: int32;
}

function test.main.Counter.constructor(v0: ref<uninit<Counter>, borrowed, exclusive, shared>): void {
entry(v0: ref<uninit<Counter>, borrowed, exclusive, shared>):
    v1: int32 = 0
    v2: ref<uninit<int32>, borrowed, exclusive, shared> = field.address v0, 0
    store v2, v1
    return
}

function test.main.make(): ref<Counter, managed, mutable, shared> {
entry:
    v0: ref<Counter, managed, mutable, shared> = new.zeroed Counter
    v1: ref<uninit<Counter>, borrowed, exclusive, shared> = cast.bit v0 -> ref<uninit<Counter>, borrowed, exclusive, shared>
    call test.main.Counter.constructor(v1): (ref<uninit<Counter>, borrowed, exclusive, shared>) => void
    return v0
}

/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=value offset=0 size=4 align=4
"#,
    );
}

/// Lower explicitly placed fields into their written spaces' reference storage.
#[test]
fn test_lower_placed_fields() {
    let session = TestSession::single(
        r#"
export class User {}

export struct Cache {
    localUser: local User;
    sharedUser: shared User;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type User { }

@copy
type Cache {
    localUser: ref<User, managed, mutable, local>;
    sharedUser: ref<User, managed, mutable, shared>;
}

/// @layout.struct name=User size=0 align=1
/// @layout.struct name=Cache size=16 align=8
/// @layout.field owner=Cache index=0 name=localUser offset=0 size=8 align=8
/// @layout.field owner=Cache index=1 name=sharedUser offset=8 size=8 align=8
"#,
    );
}

/// Lower a bare managed field inside a shared struct into shared reference storage.
#[test]
fn test_lower_relative_field_inside_shared_struct() {
    let session = TestSession::single(
        r#"
export class User {}

export shared struct Holder {
    user: User;
}

export function keep(user: User): User {
    return user;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type User { }

@copy
type Holder {
    user: ref<User, managed, mutable, shared>;
}

function test.main.keep(v0: ref<User, managed, mutable, local>): ref<User, managed, mutable, local> {
entry(v0: ref<User, managed, mutable, local>):
    return v0
}

/// @layout.struct name=User size=0 align=1
/// @layout.struct name=Holder size=8 align=8
/// @layout.field owner=Holder index=0 name=user offset=0 size=8 align=8
"#,
    );
}

/// Lower one instance of a place generic function per space its calls require.
#[test]
fn test_lower_place_generic_functions_per_demanded_space() {
    let session = TestSession::single(
        r#"
export class User {}
export shared class Team {}

export function keep<T, const P: Place>(value: Placed<T, P>): Placed<T, P> {
    return value;
}

export function run(user: User, team: Team): void {
    keep(user);
    keep(team);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type User { }

type Team { }

function test.main.run(v0: ref<User, managed, mutable, local>, v1: ref<Team, managed, mutable, shared>): void {
entry(v0: ref<User, managed, mutable, local>, v1: ref<Team, managed, mutable, shared>):
    v2: ref<User, managed, mutable, local> = call test.main.keep<ref<User, managed, mutable, local>>(v0): (ref<User, managed, mutable, local>) => ref<User, managed, mutable, local>
    v3: ref<Team, managed, mutable, shared> = call test.main.keep<ref<Team, managed, mutable, shared>, shared>(v1): (ref<Team, managed, mutable, shared>) => ref<Team, managed, mutable, shared>
    return
}

function test.main.keep<ref<User, managed, mutable, local>>(v0: ref<User, managed, mutable, local>): ref<User, managed, mutable, local> {
entry(v0: ref<User, managed, mutable, local>):
    return v0
}

function test.main.keep<ref<Team, managed, mutable, shared>, shared>(v0: ref<Team, managed, mutable, shared>): ref<Team, managed, mutable, shared> {
entry(v0: ref<Team, managed, mutable, shared>):
    return v0
}

/// @layout.struct name=User size=0 align=1
/// @layout.struct name=Team size=0 align=1
"#,
    );
}

/// Lower a shared class method body over its pinned receiver and fields.
#[test]
fn test_lower_shared_class_method_body() {
    let session = TestSession::single(
        r#"
export shared class Counter {
    value: int32 = 0;

    bump(): int32 {
        this.value = this.value + 1;
        return this.value;
    }
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Counter {
    value: int32;
}

function test.main.Counter.constructor(v0: ref<uninit<Counter>, borrowed, exclusive, shared>): void {
entry(v0: ref<uninit<Counter>, borrowed, exclusive, shared>):
    v1: int32 = 0
    v2: ref<uninit<int32>, borrowed, exclusive, shared> = field.address v0, 0
    store v2, v1
    return
}

function test.main.Counter.bump(v0: ref<Counter, managed, mutable, shared>): int32 {
entry(v0: ref<Counter, managed, mutable, shared>):
    v1: ref<int32, borrowed, mutable, shared> = field.address v0, 0
    v2: int32 = load v1
    v3: int32 = 1
    v4: int32 = add v2, v3
    v5: ref<int32, borrowed, mutable, shared> = field.address v0, 0
    store v5, v4
    v6: ref<int32, borrowed, mutable, shared> = field.address v0, 0
    v7: int32 = load v6
    return v7
}

/// @layout.struct name=Counter size=4 align=4
/// @layout.field owner=Counter index=0 name=value offset=0 size=4 align=4
"#,
    );
}

/// Lower a construction into a placed expectation's space.
#[test]
fn test_lower_construction_into_placed_expectation() {
    let session = TestSession::single(
        r#"
export class Job {}

export function submit(): shared Job {
    const job: shared Job = new Job();
    return job;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Job { }

function test.main.submit(): ref<Job, managed, mutable, shared> {
entry:
    v0: ref<Job, managed, mutable, shared> = new.zeroed Job
    return v0
}

/// @layout.struct name=Job size=0 align=1
"#,
    );
}

/// Lower placed array and interface fields inside a shared struct.
#[test]
fn test_lower_placed_representation_fields() {
    let session = TestSession::single(
        r#"
export interface Report {
    describe(): int32;
}

export shared struct Batch {
    values: shared int32[];
    report: shared Report;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Array<int32> {
    storage: slice<uninit<int32>, unique, exclusive, local>;
    count: isize;
    allocated: usize;
}

type Report { }

@copy
type Batch {
    values: ref<Array<int32>, managed, mutable, shared>;
    report: ref<Report, managed, mutable, shared>;
}

/// @layout.struct name=Array<int32> size=32 align=8
/// @layout.field owner=Array<int32> index=0 name=storage offset=0 size=16 align=8
/// @layout.field owner=Array<int32> index=1 name=count offset=16 size=8 align=8
/// @layout.field owner=Array<int32> index=2 name=allocated offset=24 size=8 align=8
/// @layout.struct name=Report size=0 align=1
/// @layout.struct name=Batch size=16 align=8
/// @layout.field owner=Batch index=0 name=values offset=0 size=8 align=8
/// @layout.field owner=Batch index=1 name=report offset=8 size=8 align=8

/// @dispatch.shape constraint=type@14 function=describe
"#,
    );
}

/// Lower a shared struct's field read from another module.
#[test]
fn test_lower_shared_field_read_across_modules() {
    let session = TestSession::builder()
        .module(
            "state.ds",
            r#"
export shared class User {}

export shared struct Holder {
    user: User;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Holder } from "./state.ds";

export function read(holder: Holder): void {
    const user = holder.user;
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.ds",
        r#"
type test.state.User { }

@copy
type test.state.Holder {
    user: ref<test.state.User, managed, mutable, shared>;
}

function test.main.read(v0: test.state.Holder): void {
entry(v0: test.state.Holder):
    v1: ref<test.state.User, managed, mutable, shared> = field.get v0, 0
    return
}

/// @layout.struct name=test.state.User size=0 align=1
/// @layout.struct name=test.state.Holder size=8 align=8
/// @layout.field owner=test.state.Holder index=0 name=user offset=0 size=8 align=8
"#,
    );
}

/// Lower one ambient method into one specialization per bound place.
#[test]
fn test_lower_ambient_method_specialized_per_space() {
    let session = TestSession::single(
        r#"
export class Gauge {
    level: int32 = 0;

    read(): int32 {
        return this.level;
    }
}

declare const shared_gauge: shared Gauge;

export function poll(): int32 {
    const local_gauge = new Gauge();
    return local_gauge.read() + shared_gauge.read();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Gauge {
    level: int32;
}

global test.main.shared_gauge: ref<Gauge, managed, mutable, shared> = zeroinit

function test.main.Gauge.constructor(v0: ref<uninit<Gauge>, borrowed, exclusive, local>): void {
entry(v0: ref<uninit<Gauge>, borrowed, exclusive, local>):
    v1: int32 = 0
    v2: ref<uninit<int32>, borrowed, exclusive, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.Gauge.read(v0: ref<Gauge, managed, mutable, local>): int32 {
entry(v0: ref<Gauge, managed, mutable, local>):
    v1: ref<int32, borrowed, mutable, local> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

function test.main.poll(): int32 {
entry:
    v0: ref<Gauge, managed, mutable, local> = new.zeroed Gauge
    v1: ref<uninit<Gauge>, borrowed, exclusive, local> = cast.bit v0 -> ref<uninit<Gauge>, borrowed, exclusive, local>
    call test.main.Gauge.constructor(v1): (ref<uninit<Gauge>, borrowed, exclusive, local>) => void
    v2: int32 = call test.main.Gauge.read(v0): (ref<Gauge, managed, mutable, local>) => int32
    v3: ref<ref<Gauge, managed, mutable, shared>, borrowed, readonly, static> = global.address test.main.shared_gauge
    v4: ref<Gauge, managed, mutable, shared> = load v3
    v5: int32 = call test.main.Gauge.read<shared>(v4): (ref<Gauge, managed, mutable, shared>) => int32
    v6: int32 = add v2, v5
    return v6
}

function test.main.Gauge.read<shared>(v0: ref<Gauge, managed, mutable, shared>): int32 {
entry(v0: ref<Gauge, managed, mutable, shared>):
    v1: ref<int32, borrowed, mutable, shared> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

/// @layout.struct name=Gauge size=4 align=4
/// @layout.field owner=Gauge index=0 name=level offset=0 size=4 align=4
"#,
    );
}

/// Lower one unpinned class constructor per construction space.
#[test]
fn test_lower_unpinned_constructor_per_construction_space() {
    let session = TestSession::single(
        r#"
export class User {
    id: int32;

    constructor(id: int32) {
        this.id = id;
    }
}

export function localUser(): User {
    return new User(1);
}

export function sharedUser(): shared User {
    return new User(2);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type User {
    id: int32;
}

function test.main.User.constructor(v0: ref<uninit<User>, borrowed, exclusive, local>, v1: int32): void {
entry(v0: ref<uninit<User>, borrowed, exclusive, local>, v1: int32):
    v2: ref<uninit<int32>, borrowed, mutable, local> = field.address v0, 0
    store v2, v1
    return
}

function test.main.localUser(): ref<User, managed, mutable, local> {
entry:
    v0: int32 = 1
    v1: ref<User, managed, mutable, local> = new.zeroed User
    v2: ref<uninit<User>, borrowed, exclusive, local> = cast.bit v1 -> ref<uninit<User>, borrowed, exclusive, local>
    call test.main.User.constructor(v2, v0): (ref<uninit<User>, borrowed, exclusive, local>, int32) => void
    return v1
}

function test.main.sharedUser(): ref<User, managed, mutable, shared> {
entry:
    v0: int32 = 2
    v1: ref<User, managed, mutable, shared> = new.zeroed User
    v2: ref<uninit<User>, borrowed, exclusive, shared> = cast.bit v1 -> ref<uninit<User>, borrowed, exclusive, shared>
    call test.main.User.constructor<shared>(v2, v0): (ref<uninit<User>, borrowed, exclusive, shared>, int32) => void
    return v1
}

function test.main.User.constructor<shared>(v0: ref<uninit<User>, borrowed, exclusive, shared>, v1: int32): void {
entry(v0: ref<uninit<User>, borrowed, exclusive, shared>, v1: int32):
    v2: ref<uninit<int32>, borrowed, mutable, shared> = field.address v0, 0
    store v2, v1
    return
}

/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}
