use crate::tests::TestSession;

/// Using resources dispose in reverse order when their block falls through.
#[test]
fn test_dispose_using_resources_in_reverse_order_at_the_block_exit() {
    let session = TestSession::single(
        r#"
import { Dispose } from "tspp:memory";

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
        "main.tspp",
        "test.main.File.dispose",
        r#"
type test.main.File {
    handle: int32;
}

function test.main.File.dispose<'a>(v0: ref<test.main.File, borrowed, 'a, mutable>): void {
    local l0: ref<test.main.File, borrowed, 'a, mutable>

entry(v0: ref<test.main.File, borrowed, 'a, mutable>):
    store l0, v0
    return
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.run",
        r#"
type test.main.File {
    handle: int32;
}

function test.main.run(): void {
    local l0: test.main.File
    local l1: test.main.File

entry:
    v0: int32 = 1
    v1: test.main.File = aggregate (v0)
    store l0, v1
    v2: int32 = 2
    v3: test.main.File = aggregate (v2)
    store l1, v3
    v4: ref<test.main.File, borrowed, 'frame, mutable> = address l1
    call test.main.File.dispose(v4): (ref<test.main.File, borrowed, 'frame, mutable>) => void
    v5: ref<test.main.File, borrowed, 'frame, mutable> = address l0
    call test.main.File.dispose(v5): (ref<test.main.File, borrowed, 'frame, mutable>) => void
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
import { Dispose } from "tspp:memory";

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
        "main.tspp",
        "test.main.File.dispose",
        r#"
type test.main.File {
    handle: int32;
}

function test.main.File.dispose<'a>(v0: ref<test.main.File, borrowed, 'a, mutable>): void {
    local l0: ref<test.main.File, borrowed, 'a, mutable>

entry(v0: ref<test.main.File, borrowed, 'a, mutable>):
    store l0, v0
    return
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.run",
        r#"
type test.main.File {
    handle: int32;
}

function test.main.run(v0: boolean): int32 {
    local l0: boolean
    local l1: test.main.File

entry(v0: boolean):
    store l0, v0
    v1: int32 = 1
    v2: test.main.File = aggregate (v1)
    store l1, v2
    v3: boolean = load l0
    branch v3 => b1 | b2

b1:
    v4: int32 = 1
    v5: ref<test.main.File, borrowed, 'frame, mutable> = address l1
    call test.main.File.dispose(v5): (ref<test.main.File, borrowed, 'frame, mutable>) => void
    return v4

b2:
    v6: int32 = 2
    v7: ref<test.main.File, borrowed, 'frame, mutable> = address l1
    call test.main.File.dispose(v7): (ref<test.main.File, borrowed, 'frame, mutable>) => void
    return v6
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
import { Dispose } from "tspp:memory";

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
        "main.tspp",
        "test.main.File.dispose",
        r#"
type test.main.File {
    handle: int32;
}

function test.main.File.dispose<'a>(v0: ref<test.main.File, borrowed, 'a, mutable>): void {
    local l0: ref<test.main.File, borrowed, 'a, mutable>

entry(v0: ref<test.main.File, borrowed, 'a, mutable>):
    store l0, v0
    return
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.run",
        r#"
type test.main.File {
    handle: int32;
}

function test.main.run(v0: variant<uint1> { 0uint1 = test.main.File; 1uint1 = void; }): void {
    local l0: variant<uint1> { 0uint1 = test.main.File; 1uint1 = void; }
    local l1: variant<uint1> { 0uint1 = test.main.File; 1uint1 = void; }

entry(v0: variant<uint1> { 0uint1 = test.main.File; 1uint1 = void; }):
    store l0, v0
    jump b1

b1:
    v1: variant<uint1> { 0uint1 = test.main.File; 1uint1 = void; } = load l0
    store l1, v1
    v2: uint1 = variant.tag.load l1
    switch v2, b3, 1 => b4

b2:
    return

b3:
    v3: ref<test.main.File, borrowed, 'frame, mutable> = address l1
    call test.main.File.dispose(v3): (ref<test.main.File, borrowed, 'frame, mutable>) => void
    jump b5

b4:
    jump b5

b5:
    jump b2
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
/// @layout.variant name=type@6 size=8 align=4
/// @layout.discriminant owner=type@6 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@6 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@6 index=1 discriminant=1 payload_offset=4
"#,
    );
}

/// A for-of using binding disposes its resource after every pass.
#[test]
fn test_dispose_a_for_of_using_resource_after_every_pass() {
    let session = TestSession::single(
        r#"
import { Dispose } from "tspp:memory";

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
        "main.tspp",
        "test.main.File.dispose",
        r#"
type test.main.File {
    handle: int32;
}

function test.main.File.dispose<'a>(v0: ref<test.main.File, borrowed, 'a, mutable>): void {
    local l0: ref<test.main.File, borrowed, 'a, mutable>

entry(v0: ref<test.main.File, borrowed, 'a, mutable>):
    store l0, v0
    return
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.total", r#"
type test.main.File {
    handle: int32;
}

@nocopy
@languageItem("collections.Array")
type Array<T>;

@nocopy
@languageItem("iter.Iterator")
type Iterator<T>;

@languageItem("iter.IteratorResult")
type IteratorResult<Y, R>;

@languageItem("iter.IteratorYield")
type IteratorYield<Y>;

@languageItem("iter.IteratorReturn")
type IteratorReturn<R>;

function test.main.total(v0: ref<Array<test.main.File>, managed, mutable, local>): int32 {
    local l0: ref<Array<test.main.File>, managed, mutable, local>
    local l1: int32
    local l2: dynamic<Iterator<test.main.File>, managed, mutable, local>
    local l3: test.main.File
    local l4: test.main.File

entry(v0: ref<Array<test.main.File>, managed, mutable, local>):
    store l0, v0
    v1: int32 = 0
    store l1, v1
    v2: ref<Array<test.main.File>, managed, mutable, local> = load l0
    v3: dynamic<Iterator<test.main.File>, managed, mutable, local> = call Array.Iterable.iterator<test.main.File>(v2): (ref<Array<test.main.File>, managed, mutable, local>) => dynamic<Iterator<test.main.File>, managed, mutable, local>
    store l2, v3
    jump b1

b1:
    v4: dynamic<Iterator<test.main.File>, managed, mutable, local> = load l2
    v5: dynamic<Iterator<test.main.File>, borrowed, 'managed, mutable> = cast.bit v4 -> dynamic<Iterator<test.main.File>, borrowed, 'managed, mutable>
    v6: IteratorResult<test.main.File, void> = call.dynamic v5, Iterator<test.main.File>, 0(): () => IteratorResult<test.main.File, void>
    v7: variant<uint1> { 0uint1 = IteratorYield<test.main.File>; 1uint1 = IteratorReturn<void>; } = field.get v6, 0
    variant.switch v7, 0 => b2, 1 => b3

b2:
    v8: IteratorYield<test.main.File> = variant.payload v7, 0
    v9: test.main.File = field.get v8, 1
    store l3, v9
    v10: test.main.File = load l3
    store l4, v10
    v11: int32 = load l1
    v12: int32 = load (l4).0
    v13: int32 = add v11, v12
    store l1, v13
    v14: ref<test.main.File, borrowed, 'frame, mutable> = address l4
    call test.main.File.dispose(v14): (ref<test.main.File, borrowed, 'frame, mutable>) => void
    jump b1

b3:
    v15: int32 = load l1
    return v15
}

/// @layout.struct name=test.main.File size=4 align=4
/// @layout.field owner=test.main.File index=0 name=handle offset=0 size=4 align=4
/// @layout.variant name=type@122 size=8 align=4
/// @layout.discriminant owner=type@122 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@122 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@122 index=1 discriminant=1 payload_offset=4
"#);
}

/// An await using resource parks on its asynchronous disposal at the scope exit.
#[test]
fn test_park_an_asynchronous_disposal_at_the_scope_exit() {
    let session = TestSession::single(
        r#"
import { Promise } from "tspp:async";
import { AsyncDispose } from "tspp:memory";

struct Connection implements AsyncDispose {
    handle: int32;

    async asyncDispose(&this): Promise<void> {}
}

async function run(): Promise<void> {
    await using connection = Connection { handle: 1 };
}
"#,
    );

    session.assert_mir_function("main.tspp", "test.main.Connection.asyncDispose", r#"
type test.main.Connection {
    handle: int32;
}

@nocopy
@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.Connection.asyncDispose<'a>(v0: ref<test.main.Connection, borrowed, 'a, mutable>): ref<Promise<void>, managed, mutable, local> {
    local l0: ref<test.main.Connection, borrowed, 'a, mutable>

entry(v0: ref<test.main.Connection, borrowed, 'a, mutable>):
    store l0, v0
    v1: ref<test.main.Connection, borrowed, 'a, mutable> = load l0
    v2: { ref<test.main.Connection, borrowed, 'a, mutable> } = aggregate (v1)
    v3: ref<{ ref<test.main.Connection, borrowed, 'a, mutable> }, unique, mutable> = new.complete v2
    v4: function<() => void, once, unique, mutable> = function.bind test.main.Connection.asyncDispose.body, v3
    v5: ref<Promise<void>, managed, mutable, local> = call Promise.create<void>(v4): (function<() => void, once, unique, mutable>) => ref<Promise<void>, managed, mutable, local>
    return v5
}

/// @layout.struct name=test.main.Connection size=4 align=4
/// @layout.field owner=test.main.Connection index=0 name=handle offset=0 size=4 align=4
/// @layout.struct name=type@53 size=8 align=8
/// @layout.field owner=type@53 index=0 offset=0 size=8 align=8
"#);

    session.assert_mir_function("main.tspp", "test.main.Connection.asyncDispose.body", r#"
type test.main.Connection {
    handle: int32;
}

@environment(ref<{ ref<test.main.Connection, borrowed, 'a, mutable> }, unique, mutable>)
function test.main.Connection.asyncDispose.body<'a>(): void {
    local l0: ref<test.main.Connection, borrowed, 'a, mutable>

entry:
    v0: ref<{ ref<test.main.Connection, borrowed, 'a, mutable> }, unique, mutable> = function.environment.current
    v1: { ref<test.main.Connection, borrowed, 'a, mutable> } = load (*v0)
    v2: ref<uninit<{ ref<test.main.Connection, borrowed, 'a, mutable> }>, unique, mutable> = cast.bit v0 -> ref<uninit<{ ref<test.main.Connection, borrowed, 'a, mutable> }>, unique, mutable>
    release v2
    v3: ref<test.main.Connection, borrowed, 'a, mutable> = field.get v1, 0
    store l0, v3
    return
}

/// @layout.struct name=test.main.Connection size=4 align=4
/// @layout.field owner=test.main.Connection index=0 name=handle offset=0 size=4 align=4
/// @layout.struct name=type@53 size=8 align=8
/// @layout.field owner=type@53 index=0 offset=0 size=8 align=8
"#);

    session.assert_mir_function("main.tspp", "test.main.run", r#"
@nocopy
@languageItem("async.Promise")
type Promise<T: Copy>;

function test.main.run(): ref<Promise<void>, managed, mutable, local> {
entry:
    v0: {  } = aggregate ()
    v1: ref<{  }, unique, mutable> = new.complete v0
    v2: function<() => void, once, unique, mutable> = function.bind test.main.run.body, v1
    v3: ref<Promise<void>, managed, mutable, local> = call Promise.create<void>(v2): (function<() => void, once, unique, mutable>) => ref<Promise<void>, managed, mutable, local>
    return v3
}

/// @layout.struct name=type@9 size=0 align=1
"#);

    session.assert_mir_function("main.tspp", "test.main.run.body", r#"
type test.main.Connection {
    handle: int32;
}

@nocopy
@languageItem("async.Promise")
type Promise<T: Copy>;

@environment(ref<{  }, unique, mutable>)
function test.main.run.body(): void {
    local l0: test.main.Connection

entry:
    v0: ref<{  }, unique, mutable> = function.environment.current
    v1: {  } = load (*v0)
    v2: ref<uninit<{  }>, unique, mutable> = cast.bit v0 -> ref<uninit<{  }>, unique, mutable>
    release v2
    v3: int32 = 1
    v4: test.main.Connection = aggregate (v3)
    store l0, v4
    v5: ref<test.main.Connection, borrowed, 'frame, mutable> = address l0
    v6: ref<Promise<void>, managed, mutable, local> = call test.main.Connection.asyncDispose(v5): (ref<test.main.Connection, borrowed, 'frame, mutable>) => ref<Promise<void>, managed, mutable, local>
    call Promise.park<void, void>(v6): (ref<Promise<void>, managed, mutable, local>) => void
    return
}

/// @layout.struct name=test.main.Connection size=4 align=4
/// @layout.field owner=test.main.Connection index=0 name=handle offset=0 size=4 align=4
/// @layout.struct name=type@9 size=0 align=1
"#);
}
