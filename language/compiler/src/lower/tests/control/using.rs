use crate::tests::TestSession;

/// Using resources dispose in reverse order when their block falls through.
#[test]
fn test_dispose_using_resources_in_reverse_order_at_the_block_exit() {
    let session = TestSession::single(
        r#"
import { Dispose } from "destack:memory";

struct File implements Dispose {
    handle: int32;

    dispose(&this): void {}
}

function run(): void {
    using first = File { handle: 1 };
    using second = File { handle: 2 };
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.File.dispose",
        r#"
@copy
type test.main.File {
    handle: int32;
}

function test.main.File.dispose<'a>(v0: ref<test.main.File, borrowed, 'a, mutable, local>): void {
    local l0: ref<test.main.File, borrowed, 'a, mutable, local>

entry(v0: ref<test.main.File, borrowed, 'a, mutable, local>):
    local.set l0, v0
    return
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.run",
        r#"
@copy
type test.main.File {
    handle: int32;
}

function test.main.run(): void {
    local l0: test.main.File
    local l1: test.main.File

entry:
    v0: int32 = 1
    v1: test.main.File = aggregate (v0)
    local.set l0, v1
    v2: int32 = 2
    v3: test.main.File = aggregate (v2)
    local.set l1, v3
    v4: test.main.File = local.get l1
    v5: ref<test.main.File, borrowed, 'frame, mutable, local> = local.address l1
    call test.main.File.dispose(v5): <'a>(ref<test.main.File, borrowed, 'a, mutable, local>) => void
    v6: test.main.File = local.get l0
    v7: ref<test.main.File, borrowed, 'frame, mutable, local> = local.address l0
    call test.main.File.dispose(v7): <'a>(ref<test.main.File, borrowed, 'a, mutable, local>) => void
    return
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
"#,
    );
}

/// A return disposes every live using resource before leaving.
#[test]
fn test_dispose_using_resources_before_an_early_return() {
    let session = TestSession::single(
        r#"
import { Dispose } from "destack:memory";

struct File implements Dispose {
    handle: int32;

    dispose(&this): void {}
}

function run(flag: boolean): int32 {
    using file = File { handle: 1 };
    if (flag) {
        return 1;
    }
    return 2;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.File.dispose",
        r#"
@copy
type test.main.File {
    handle: int32;
}

function test.main.File.dispose<'a>(v0: ref<test.main.File, borrowed, 'a, mutable, local>): void {
    local l0: ref<test.main.File, borrowed, 'a, mutable, local>

entry(v0: ref<test.main.File, borrowed, 'a, mutable, local>):
    local.set l0, v0
    return
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.run",
        r#"
@copy
type test.main.File {
    handle: int32;
}

function test.main.run(v0: boolean): int32 {
    local l0: boolean
    local l1: test.main.File

entry(v0: boolean):
    local.set l0, v0
    v1: int32 = 1
    v2: test.main.File = aggregate (v1)
    local.set l1, v2
    v3: boolean = local.get l0
    branch v3 => b1 | b2

b1:
    v4: int32 = 1
    v5: test.main.File = local.get l1
    v6: ref<test.main.File, borrowed, 'frame, mutable, local> = local.address l1
    call test.main.File.dispose(v6): <'a>(ref<test.main.File, borrowed, 'a, mutable, local>) => void
    return v4

b2:
    v7: int32 = 2
    v8: test.main.File = local.get l1
    v9: ref<test.main.File, borrowed, 'frame, mutable, local> = local.address l1
    call test.main.File.dispose(v9): <'a>(ref<test.main.File, borrowed, 'a, mutable, local>) => void
    return v7
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
"#,
    );
}

/// A break disposes the resources of the scopes it leaves, and a nullable resource is skipped when absent.
#[test]
fn test_dispose_a_nullable_using_resource_before_a_break() {
    let session = TestSession::single(
        r#"
import { Dispose } from "destack:memory";

struct File implements Dispose {
    handle: int32;

    dispose(&this): void {}
}

function run(file: File | undefined): void {
    loop {
        using resource = file;
        break;
    }
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.File.dispose",
        r#"
@copy
type test.main.File {
    handle: int32;
}

function test.main.File.dispose<'a>(v0: ref<test.main.File, borrowed, 'a, mutable, local>): void {
    local l0: ref<test.main.File, borrowed, 'a, mutable, local>

entry(v0: ref<test.main.File, borrowed, 'a, mutable, local>):
    local.set l0, v0
    return
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.run", r#"
@copy
type test.main.File {
    handle: int32;
}

function test.main.run(v0: variant<uint1> { 0uint1 = void; 1uint1 = test.main.File; }): void {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = test.main.File; }
    local l1: variant<uint1> { 0uint1 = void; 1uint1 = test.main.File; }

entry(v0: variant<uint1> { 0uint1 = void; 1uint1 = test.main.File; }):
    local.set l0, v0
    jump b1

b1:
    v1: variant<uint1> { 0uint1 = void; 1uint1 = test.main.File; } = local.get l0
    local.set l1, v1
    v2: variant<uint1> { 0uint1 = void; 1uint1 = test.main.File; } = local.get l1
    variant.switch v2, 0 => b4, else b3

b2:
    return

b3:
    v3: ref<variant<uint1> { 0uint1 = void; 1uint1 = test.main.File; }, borrowed, 'frame, mutable, frame> = local.project l1
    v4: ref<test.main.File, borrowed, 'frame, mutable, local> = variant.payload.address v3, 1
    call test.main.File.dispose(v4): <'a>(ref<test.main.File, borrowed, 'a, mutable, local>) => void
    jump b5

b4:
    jump b5

b5:
    jump b2
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
/// @layout.variant name=type@8 size=8 align=4
/// @layout.discriminant owner=type@8 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@8 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@8 index=1 discriminant=1 payload_offset=4
"#);
}

/// A for-of using binding disposes its resource after every pass.
#[test]
fn test_dispose_a_for_of_using_resource_after_every_pass() {
    let session = TestSession::single(
        r#"
import { Dispose } from "destack:memory";

struct File implements Dispose {
    handle: int32;

    dispose(&this): void {}
}

function total(files: File[]): int32 {
    let sum: int32 = 0;
    for (using file of files) {
        sum += file.handle;
    }

    return sum;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.File.dispose",
        r#"
@copy
type test.main.File {
    handle: int32;
}

function test.main.File.dispose<'a>(v0: ref<test.main.File, borrowed, 'a, mutable, local>): void {
    local l0: ref<test.main.File, borrowed, 'a, mutable, local>

entry(v0: ref<test.main.File, borrowed, 'a, mutable, local>):
    local.set l0, v0
    return
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.ds", "test.main.total", r#"
@copy
type test.main.File {
    handle: int32;
}

@languageItem("collections.Array")
type Array<T>;

@languageItem("iter.Iterator")
type Iterator<T>;

@copy
@languageItem("iter.IteratorReturn")
type IteratorReturn<R>;

@copy
@languageItem("iter.IteratorYield")
type IteratorYield<Y>;

@copy
@languageItem("iter.IteratorResult")
type IteratorResult<Y, R>;

function test.main.total(v0: ref<Array<test.main.File>, managed, mutable, local>): int32 {
    local l0: ref<Array<test.main.File>, managed, mutable, local>
    local l1: int32
    local l2: dynamic<Iterator<test.main.File>, managed, mutable, local>
    local l3: test.main.File
    local l4: test.main.File

entry(v0: ref<Array<test.main.File>, managed, mutable, local>):
    local.set l0, v0
    v1: int32 = 0
    local.set l1, v1
    v2: ref<Array<test.main.File>, managed, mutable, local> = local.get l0
    v3: dynamic<Iterator<test.main.File>, managed, mutable, local> = call Array.Iterable.iterator<test.main.File>(v2): (ref<Array<test.main.File>, managed, mutable, local>) => dynamic<Iterator<test.main.File>, managed, mutable, local>
    local.set l2, v3
    jump b1

b1:
    v4: dynamic<Iterator<test.main.File>, managed, mutable, local> = local.get l2
    v5: IteratorResult<test.main.File, void> = call.dynamic v4, Iterator<test.main.File>, 0(): () => IteratorResult<test.main.File, void>
    v6: variant<uint1> { 0uint1 = IteratorReturn<void>; 1uint1 = IteratorYield<test.main.File>; } = field.get v5, 0
    variant.switch v6, 1 => b2, 0 => b3

b2:
    v7: IteratorYield<test.main.File> = variant.payload v6, 1
    v8: test.main.File = field.get v7, 1
    local.set l3, v8
    v9: test.main.File = local.get l3
    local.set l4, v9
    v10: int32 = local.get l1
    v11: test.main.File = local.get l4
    v12: int32 = field.get v11, 0
    v13: int32 = add v10, v12
    local.set l1, v13
    v14: test.main.File = local.get l4
    v15: ref<test.main.File, borrowed, 'frame, mutable, local> = local.address l4
    call test.main.File.dispose(v15): <'a>(ref<test.main.File, borrowed, 'a, mutable, local>) => void
    jump b1

b3:
    v16: int32 = local.get l1
    return v16
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
/// @layout.variant name=type@111 size=8 align=4
/// @layout.discriminant owner=type@111 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@111 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@111 index=1 discriminant=1 payload_offset=4
"#);
}

/// An await using resource parks on its asynchronous disposal at the scope exit.
#[test]
fn test_park_an_asynchronous_disposal_at_the_scope_exit() {
    let session = TestSession::single(
        r#"
import { Promise } from "destack:async";
import { AsyncDispose } from "destack:memory";

struct Connection implements AsyncDispose {
    handle: int32;

    async asyncDispose(&this): Promise<void> {}
}

async function run(): Promise<void> {
    await using connection = Connection { handle: 1 };
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.Connection.asyncDispose", r#"
@copy
type test.main.Connection {
    handle: int32;
}

@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.Connection.asyncDispose<'a>(v0: ref<test.main.Connection, borrowed, 'a, mutable, local>): ref<Promise<void>, managed, mutable, local> {
    local l0: ref<test.main.Connection, borrowed, 'a, mutable, local>

entry(v0: ref<test.main.Connection, borrowed, 'a, mutable, local>):
    local.set l0, v0
    v1: ref<test.main.Connection, borrowed, 'a, mutable, local> = local.get l0
    v2: { ref<test.main.Connection, borrowed, 'a, mutable, local> } = aggregate (v1)
    v3: ref<{ ref<test.main.Connection, borrowed, 'a, mutable, local> }, unique, mutable, local> = new.complete v2
    v4: function<() => void, once, unique, mutable, local> = function.bind test.main.Connection.asyncDispose.body, v3
    v5: ref<Promise<void>, managed, mutable, local> = call Promise.create<void>(v4): (function<() => void, once, unique, mutable, local>) => ref<Promise<void>, managed, mutable, local>
    return v5
}

/// @layout.struct name=test.main.Connection size=4 align=4
/// @layout.field owner=test.main.Connection index=0 name=handle offset=0 size=4 align=4
/// @layout.struct name=type@116 size=8 align=8
/// @layout.field owner=type@116 index=0 offset=0 size=8 align=8
"#);

    session.assert_mir_function("main.ds", "test.main.Connection.asyncDispose.body", r#"
@copy
type test.main.Connection {
    handle: int32;
}

@environment(ref<{ ref<test.main.Connection, borrowed, 'l0, mutable, local> }, unique, mutable, local>)
function test.main.Connection.asyncDispose.body<'a>(): void {
    local l0: ref<test.main.Connection, borrowed, 'a, mutable, local>

entry:
    v0: ref<{ ref<test.main.Connection, borrowed, 'a, mutable, local> }, unique, mutable, local> = function.environment.current
    v1: { ref<test.main.Connection, borrowed, 'a, mutable, local> } = load v0
    release v0
    v2: ref<test.main.Connection, borrowed, 'a, mutable, local> = field.get v1, 0
    local.set l0, v2
    return
}

/// @layout.struct name=test.main.Connection size=4 align=4
/// @layout.field owner=test.main.Connection index=0 name=handle offset=0 size=4 align=4
/// @layout.struct name=type@116 size=8 align=8
/// @layout.field owner=type@116 index=0 offset=0 size=8 align=8
"#);

    session.assert_mir_function("main.ds", "test.main.run", r#"
@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.run(): ref<Promise<void>, managed, mutable, local> {
entry:
    v0: {  } = aggregate ()
    v1: ref<{  }, unique, mutable, local> = new.complete v0
    v2: function<() => void, once, unique, mutable, local> = function.bind test.main.run.body, v1
    v3: ref<Promise<void>, managed, mutable, local> = call Promise.create<void>(v2): (function<() => void, once, unique, mutable, local>) => ref<Promise<void>, managed, mutable, local>
    return v3
}

/// @layout.struct name=type@130 size=0 align=1
"#);

    session.assert_mir_function("main.ds", "test.main.run.body", r#"
@copy
type test.main.Connection {
    handle: int32;
}

@languageItem("async.Promise")
type Promise<T: Copy>;

@environment(ref<{  }, unique, mutable, local>)
function test.main.run.body(): void {
    local l0: test.main.Connection

entry:
    v0: ref<{  }, unique, mutable, local> = function.environment.current
    v1: {  } = load v0
    release v0
    v2: int32 = 1
    v3: test.main.Connection = aggregate (v2)
    local.set l0, v3
    v4: test.main.Connection = local.get l0
    v5: ref<test.main.Connection, borrowed, 'frame, mutable, local> = local.address l0
    v6: ref<Promise<void>, managed, mutable, local> = call test.main.Connection.asyncDispose(v5): <'a>(ref<test.main.Connection, borrowed, 'a, mutable, local>) => ref<Promise<void>, managed, mutable, local>
    call Promise.park<void, void>(v6): (ref<Promise<void>, managed, mutable, local>) => void
    return
}

/// @layout.struct name=test.main.Connection size=4 align=4
/// @layout.field owner=test.main.Connection index=0 name=handle offset=0 size=4 align=4
/// @layout.struct name=type@130 size=0 align=1
"#);
}
