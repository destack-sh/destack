use crate::tests::TestSession;

#[test]
fn test_dispatch_a_method_through_an_erased_interface() {
    let session = TestSession::single(
        r#"
interface Greeter {
    greet(): int32;
}

class Console implements Greeter {
    greet(): int32 {
        return 1;
    }
}

function talk(greeter: Greeter): int32 {
    return greeter.greet();
}

function run(): int32 {
    const console = new Console();
    return talk(console);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Console { }

type Greeter { }

function test.main.Console.greet(v0: ref<Console, managed, mutable>): int32 {
entry(v0: ref<Console, managed, mutable>):
    v1: int32 = 1
    return v1
}

function test.main.talk(v0: dynamic<Greeter>): int32 {
entry(v0: dynamic<Greeter>):
    v1: int32 = call.dynamic v0, Greeter, 0(): () => int32
    return v1
}

function test.main.run(): int32 {
entry:
    v0: ref<Console, managed, mutable> = new.zeroed Console
    v1: dynamic<Greeter> = dynamic.bind v0, Console
    v2: int32 = call test.main.talk(v1)
    return v2
}
/// @layout.struct name=Console size=0 align=1
/// @layout.struct name=Greeter size=0 align=1

/// @dispatch.shape constraint=type@5 function=greet
/// @dispatch.table concrete=type@0 constraint=type@5 function@4
"#,
    );
}

/// Test and dispatch a nullable erased interface through its carrier niche.
#[test]
fn test_compare_and_dispatch_a_nullable_erased_interface() {
    let session = TestSession::single(
        r#"
interface Sink {
    put(value: int32): int32;
}

function write(sink: Sink | undefined): int32 {
    if (sink == undefined) {
        return 0;
    }

    return sink.put(1);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Sink { }

function test.main.write(v0: dynamic<Sink, undefined>): int32 {
entry(v0: dynamic<Sink, undefined>):
    v1: dynamic<Sink, undefined> = undefined
    v2: boolean = int.eq v0, v1
    branch v2, b1, b2

b1:
    v3: int32 = 0
    return v3

b2:
    v4: int32 = 1
    v5: int32 = call.dynamic v0, Sink, 0(v4): (int32) => int32
    return v5
}
/// @layout.struct name=Sink size=0 align=1

/// @dispatch.shape constraint=type@0 function=put
"#,
    );
}
