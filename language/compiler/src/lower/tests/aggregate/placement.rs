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

    session.assert_mir_function("main.ds", "test.main.Counter.constructor", r#"
type test.main.Counter {
    value: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, shared>): void {
    local l0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, shared>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, shared>):
    local.set l0, v0
    v1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, shared> = local.get l0
    v2: int32 = 0
    v3: ref<uninit<int32>, borrowed, 'a, mutable, shared> = field.project v1, 0
    store v3, v2
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=value offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.make", r#"
type test.main.Counter {
    value: int32;
}

function test.main.make(): ref<test.main.Counter, managed, mutable, shared> {
entry:
    v0: ref<test.main.Counter, managed, mutable, shared> = new.zeroed test.main.Counter
    v1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, shared> = cast.bit v0 -> ref<uninit<test.main.Counter>, borrowed, 'managed, mutable, shared>
    call test.main.Counter.constructor(v1): <'a>(ref<uninit<test.main.Counter>, borrowed, 'a, mutable, shared>) => void
    return v0
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=value offset=0 size=4 align=4
"#);
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
type test.main.User { }

@copy
type test.main.Cache {
    localUser: ref<test.main.User, managed, mutable, local>;
    sharedUser: ref<test.main.User, managed, mutable, shared>;
}

/// @layout.struct name=test.main.User size=0 align=1
/// @layout.struct name=test.main.Cache size=16 align=8
/// @layout.field owner=test.main.Cache index=0 name=localUser offset=0 size=8 align=8
/// @layout.field owner=test.main.Cache index=1 name=sharedUser offset=8 size=8 align=8
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

    session.assert_mir_function("main.ds", "test.main.keep", r#"
type test.main.User { }

function test.main.keep(v0: ref<test.main.User, managed, mutable, local>): ref<test.main.User, managed, mutable, local> {
    local l0: ref<test.main.User, managed, mutable, local>

entry(v0: ref<test.main.User, managed, mutable, local>):
    local.set l0, v0
    v1: ref<test.main.User, managed, mutable, local> = local.get l0
    return v1
}

/// @layout.struct name=test.main.User size=0 align=1
"#);
}

/// Lower one instance of a place generic function per space its calls require.
#[test]
fn test_lower_place_generic_functions_per_demanded_space() {
    let session = TestSession::single(
        r#"
export class User {}
export shared class Team {}

export function keep<T, const P: Place>(value: Managed<T, P>): Managed<T, P> {
    return value;
}

export function run(user: User, team: Team): void {
    keep(user);
    keep(team);
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.run", r#"
type test.main.User { }

type test.main.Team { }

function test.main.run(v0: ref<test.main.User, managed, mutable, local>, v1: ref<test.main.Team, managed, mutable, shared>): void {
    local l0: ref<test.main.User, managed, mutable, local>
    local l1: ref<test.main.Team, managed, mutable, shared>

entry(v0: ref<test.main.User, managed, mutable, local>, v1: ref<test.main.Team, managed, mutable, shared>):
    local.set l0, v0
    local.set l1, v1
    v2: ref<test.main.User, managed, mutable, local> = local.get l0
    v3: ref<ref<test.main.User, managed, mutable, local>, managed, mutable, local> = call test.main.keep<ref<test.main.User, managed, mutable, local>>(v2): (ref<ref<test.main.User, managed, mutable, local>, managed, mutable, local>) => ref<ref<test.main.User, managed, mutable, local>, managed, mutable, local>
    v4: ref<test.main.Team, managed, mutable, shared> = local.get l1
    v5: ref<ref<test.main.Team, managed, mutable, shared>, managed, mutable, shared> = call test.main.keep<ref<test.main.Team, managed, mutable, shared>, shared>(v4): (ref<ref<test.main.Team, managed, mutable, shared>, managed, mutable, shared>) => ref<ref<test.main.Team, managed, mutable, shared>, managed, mutable, shared>
    return
}

/// @layout.struct name=test.main.User size=0 align=1
/// @layout.struct name=test.main.Team size=0 align=1
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.keep<ref<test.main.User, managed, mutable, local>>",
        r#"
type test.main.User { }

shared function test.main.keep<ref<test.main.User, managed, mutable, local>>(v0: ref<ref<test.main.User, managed, mutable, local>, managed, mutable, local>): ref<ref<test.main.User, managed, mutable, local>, managed, mutable, local>;

/// @layout.struct name=test.main.User size=0 align=1
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.keep<ref<test.main.Team, managed, mutable, shared>, shared>",
        r#"
type test.main.Team { }

shared function test.main.keep<ref<test.main.Team, managed, mutable, shared>, shared>(v0: ref<ref<test.main.Team, managed, mutable, shared>, managed, mutable, shared>): ref<ref<test.main.Team, managed, mutable, shared>, managed, mutable, shared>;

/// @layout.struct name=test.main.Team size=0 align=1
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

    session.assert_mir_function("main.ds", "test.main.Counter.constructor", r#"
type test.main.Counter {
    value: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, shared>): void {
    local l0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, shared>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, shared>):
    local.set l0, v0
    v1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, shared> = local.get l0
    v2: int32 = 0
    v3: ref<uninit<int32>, borrowed, 'a, mutable, shared> = field.project v1, 0
    store v3, v2
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=value offset=0 size=4 align=4
"#);

    session.assert_mir_function(
        "main.ds",
        "test.main.Counter.bump",
        r#"
type test.main.Counter {
    value: int32;
}

function test.main.Counter.bump(v0: ref<test.main.Counter, managed, mutable, shared>): int32 {
    local l0: ref<test.main.Counter, managed, mutable, shared>

entry(v0: ref<test.main.Counter, managed, mutable, shared>):
    local.set l0, v0
    v1: ref<test.main.Counter, managed, mutable, shared> = local.get l0
    v2: ref<test.main.Counter, managed, mutable, shared> = local.get l0
    v3: ref<int32, borrowed, 'managed, readonly, shared> = field.project v2, 0
    v4: int32 = load v3
    v5: int32 = 1
    v6: int32 = add v4, v5
    v7: ref<int32, borrowed, 'managed, mutable, shared> = field.project v1, 0
    store v7, v6
    v8: ref<test.main.Counter, managed, mutable, shared> = local.get l0
    v9: ref<int32, borrowed, 'managed, readonly, shared> = field.project v8, 0
    v10: int32 = load v9
    return v10
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=value offset=0 size=4 align=4
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

    session.assert_mir_function(
        "main.ds",
        "test.main.submit",
        r#"
type test.main.Job { }

function test.main.submit(): ref<test.main.Job, managed, mutable, shared> {
    local l0: ref<test.main.Job, managed, mutable, shared>

entry:
    v0: ref<test.main.Job, managed, mutable, shared> = new.zeroed test.main.Job
    local.set l0, v0
    v1: ref<test.main.Job, managed, mutable, shared> = local.get l0
    return v1
}

/// @layout.struct name=test.main.Job size=0 align=1
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
@languageItem("collections.Array")
type Array<T> {
    storage: slice<uninit<T>, unique, mutable, local>;
    count: isize;
    allocated: usize;
}

type test.main.Report { }

@copy
type test.main.Batch {
    values: ref<Array<int32>, managed, mutable, shared>;
    report: ref<test.main.Report, managed, mutable, shared>;
}

@languageItem("memory.Drop")
type Drop { }

@languageItem("memory.Clone")
type Clone { }

@languageItem("math.Zero")
type Zero { }

@languageItem("math.One")
type One { }

external function Drop.drop<this: Drop, 'a>(ref<this, borrowed, 'a, mutable, local>): void

external function Array.Drop.drop<T, 'a>(ref<Array<T>, borrowed, 'a, mutable, local>): void

shared function Array.Drop.drop<int32, 'a>(v0: ref<Array<int32>, borrowed, 'a, mutable, local>): void;

/// @layout.struct name=test.main.Report size=0 align=1
/// @layout.struct name=test.main.Batch size=16 align=8
/// @layout.field owner=test.main.Batch index=0 name=values offset=0 size=8 align=8
/// @layout.field owner=test.main.Batch index=1 name=report offset=8 size=8 align=8
/// @layout.struct name=Drop size=0 align=1
/// @layout.struct name=Clone size=0 align=1
/// @layout.struct name=Zero size=0 align=1
/// @layout.struct name=One size=0 align=1
/// @layout.struct name=Array<int32> size=32 align=8
/// @layout.field owner=Array<int32> index=0 name=storage offset=0 size=16 align=8
/// @layout.field owner=Array<int32> index=1 name=count offset=16 size=8 align=8
/// @layout.field owner=Array<int32> index=2 name=allocated offset=24 size=8 align=8
/// @layout.struct name=type@18 size=32 align=8
/// @layout.field owner=type@18 index=0 name=storage offset=0 size=16 align=8
/// @layout.field owner=type@18 index=1 name=count offset=16 size=8 align=8
/// @layout.field owner=type@18 index=2 name=allocated offset=24 size=8 align=8

/// @dispatch.shape constraint=type@21 function=describe
/// @dispatch.shape constraint=type@28 function=drop
/// @dispatch.shape constraint=type@41 function=clone function=cloneFrom
/// @dispatch.shape constraint=type@47 function=zero
/// @dispatch.shape constraint=type@51 function=one
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

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
type test.state.User;

@copy
type test.state.Holder;

function test.main.read(v0: test.state.Holder): void {
    local l0: test.state.Holder
    local l1: ref<test.state.User, managed, mutable, shared>

entry(v0: test.state.Holder):
    local.set l0, v0
    v1: test.state.Holder = local.get l0
    v2: ref<test.state.User, managed, mutable, shared> = field.get v1, 0
    local.set l1, v2
    return
}
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
    const localGauge = new Gauge();
    return localGauge.read() + shared_gauge.read();
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Gauge.constructor",
        r#"
type test.main.Gauge {
    level: int32;
}

function test.main.Gauge.constructor<'a>(v0: ref<uninit<test.main.Gauge>, borrowed, 'a, mutable, local>): void {
    local l0: ref<uninit<test.main.Gauge>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.Gauge>, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<uninit<test.main.Gauge>, borrowed, 'a, mutable, local> = local.get l0
    v2: int32 = 0
    v3: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v1, 0
    store v3, v2
    return
}

/// @layout.struct name=test.main.Gauge size=4 align=4
/// @layout.field owner=test.main.Gauge index=0 name=level offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Gauge.read",
        r#"
type test.main.Gauge {
    level: int32;
}

function test.main.Gauge.read(v0: ref<test.main.Gauge, managed, mutable, local>): int32 {
    local l0: ref<test.main.Gauge, managed, mutable, local>

entry(v0: ref<test.main.Gauge, managed, mutable, local>):
    local.set l0, v0
    v1: ref<test.main.Gauge, managed, mutable, local> = local.get l0
    v2: ref<int32, borrowed, 'managed, readonly, local> = field.project v1, 0
    v3: int32 = load v2
    return v3
}

/// @layout.struct name=test.main.Gauge size=4 align=4
/// @layout.field owner=test.main.Gauge index=0 name=level offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.poll",
        r#"
type test.main.Gauge {
    level: int32;
}

function test.main.poll(): int32 {
    local l0: ref<test.main.Gauge, managed, mutable, local>

entry:
    v0: ref<test.main.Gauge, managed, mutable, local> = new.zeroed test.main.Gauge
    v1: ref<uninit<test.main.Gauge>, borrowed, 'managed, mutable, local> = cast.bit v0 -> ref<uninit<test.main.Gauge>, borrowed, 'managed, mutable, local>
    call test.main.Gauge.constructor(v1): <'a>(ref<uninit<test.main.Gauge>, borrowed, 'a, mutable, local>) => void
    local.set l0, v0
    v2: ref<test.main.Gauge, managed, mutable, local> = local.get l0
    v3: int32 = call test.main.Gauge.read(v2): (ref<test.main.Gauge, managed, mutable, local>) => int32
    v4: ref<ref<test.main.Gauge, managed, mutable, shared>, borrowed, readonly, static> = global.project test.main.shared_gauge
    v5: ref<test.main.Gauge, managed, mutable, shared> = load v4
    v6: int32 = call test.main.Gauge.read<shared>(v5): (ref<test.main.Gauge, managed, mutable, local>) => int32
    v7: int32 = add v3, v6
    return v7
}

/// @layout.struct name=test.main.Gauge size=4 align=4
/// @layout.field owner=test.main.Gauge index=0 name=level offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Gauge.read<shared>",
        r#"
type test.main.Gauge {
    level: int32;
}

shared function test.main.Gauge.read<shared>(v0: ref<test.main.Gauge, managed, mutable, local>): int32;

/// @layout.struct name=test.main.Gauge size=4 align=4
/// @layout.field owner=test.main.Gauge index=0 name=level offset=0 size=4 align=4
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

    session.assert_mir_function("main.ds", "test.main.User.constructor", r#"
type test.main.User {
    id: int32;
}

function test.main.User.constructor<'a>(v0: ref<uninit<test.main.User>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.User>, borrowed, 'a, mutable, local>

entry(v0: ref<uninit<test.main.User>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.User>, borrowed, 'a, mutable, local> = local.get l1
    v3: int32 = local.get l0
    v4: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    return
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.localUser", r#"
type test.main.User {
    id: int32;
}

function test.main.localUser(): ref<test.main.User, managed, mutable, local> {
entry:
    v0: int32 = 1
    v1: ref<test.main.User, managed, mutable, local> = new.zeroed test.main.User
    v2: ref<uninit<test.main.User>, borrowed, 'managed, mutable, local> = cast.bit v1 -> ref<uninit<test.main.User>, borrowed, 'managed, mutable, local>
    call test.main.User.constructor(v2, v0): <'a>(ref<uninit<test.main.User>, borrowed, 'a, mutable, local>, int32) => void
    return v1
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.sharedUser", r#"
type test.main.User {
    id: int32;
}

function test.main.sharedUser(): ref<test.main.User, managed, mutable, shared> {
entry:
    v0: int32 = 2
    v1: ref<test.main.User, managed, mutable, shared> = new.zeroed test.main.User
    v2: ref<uninit<test.main.User>, borrowed, 'managed, mutable, shared> = cast.bit v1 -> ref<uninit<test.main.User>, borrowed, 'managed, mutable, shared>
    call test.main.User.constructor<shared>(v2, v0): <'a>(ref<uninit<test.main.User>, borrowed, 'a, mutable, local>, int32) => void
    return v1
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);

    session.assert_mir_function("main.ds", "test.main.User.constructor<shared>", r#"
type test.main.User {
    id: int32;
}

shared function test.main.User.constructor<shared, 'a>(v0: ref<uninit<test.main.User>, borrowed, 'a, mutable, local>, v1: int32): void;

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#);
}
