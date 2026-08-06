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

function test.main.talk(v0: dynamic<Greeter, managed, mutable>): int32 {
entry(v0: dynamic<Greeter, managed, mutable>):
    v1: int32 = call.dynamic v0, Greeter, 0(): () => int32
    return v1
}

function test.main.run(): int32 {
entry:
    v0: ref<Console, managed, mutable> = new.zeroed Console
    v1: dynamic<Greeter, managed, mutable> = dynamic.bind v0, Console
    v2: int32 = call test.main.talk(v1): (dynamic<Greeter, managed, mutable>) => int32
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

function test.main.write(v0: dynamic<Sink, managed, mutable, undefined>): int32 {
entry(v0: dynamic<Sink, managed, mutable, undefined>):
    v1: dynamic<Sink, managed, mutable, undefined> = undefined
    v2: boolean = int.eq v0, v1
    branch v2 => b1 | b2

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

#[test]
fn test_call_inherited_interface_methods_through_one_erased_value() {
    let session = TestSession::single(
        r#"
interface Named {
    name(): int32;
}

interface Counted extends Named {
    count(): int32;
}

class Register implements Counted {
    name(): int32 {
        return 1;
    }

    count(): int32 {
        return 2;
    }
}

function read(counted: Counted): int32 {
    return counted.name() + counted.count();
}

function run(): int32 {
    const register = new Register();
    return read(register);
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type Register { }

type Counted { }

function test.main.Register.name(v0: ref<Register, managed, mutable>): int32 {
entry(v0: ref<Register, managed, mutable>):
    v1: int32 = 1
    return v1
}

function test.main.Register.count(v0: ref<Register, managed, mutable>): int32 {
entry(v0: ref<Register, managed, mutable>):
    v1: int32 = 2
    return v1
}

function test.main.read(v0: dynamic<Counted, managed, mutable>): int32 {
entry(v0: dynamic<Counted, managed, mutable>):
    v1: int32 = call.dynamic v0, Counted, 0(): () => int32
    v2: int32 = call.dynamic v0, Counted, 1(): () => int32
    v3: int32 = int.add v1, v2
    return v3
}

function test.main.run(): int32 {
entry:
    v0: ref<Register, managed, mutable> = new.zeroed Register
    v1: dynamic<Counted, managed, mutable> = dynamic.bind v0, Register
    v2: int32 = call test.main.read(v1): (dynamic<Counted, managed, mutable>) => int32
    return v2
}
/// @layout.struct name=Register size=0 align=1
/// @layout.struct name=Counted size=0 align=1

/// @dispatch.shape constraint=type@6 function=name function=count
/// @dispatch.table concrete=type@0 constraint=type@6 function@4 function@5
"#,
    );
}
