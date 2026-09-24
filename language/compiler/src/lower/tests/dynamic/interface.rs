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

    session.assert_mir_function(
        "main.ds",
        "test.main.Console.greet",
        r#"
@nocopy
type test.main.Console { }

function test.main.Console.greet(v0: ref<test.main.Console, managed, mutable, local>): int32 {
    local l0: ref<test.main.Console, managed, mutable, local>

entry(v0: ref<test.main.Console, managed, mutable, local>):
    store l0, v0
    v1: int32 = 1
    return v1
}

/// @layout.struct name=test.main.Console size=0 align=1
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.talk",
        r#"
@nocopy
type test.main.Greeter { }

function test.main.talk(v0: dynamic<test.main.Greeter, managed, mutable, local>): int32 {
    local l0: dynamic<test.main.Greeter, managed, mutable, local>

entry(v0: dynamic<test.main.Greeter, managed, mutable, local>):
    store l0, v0
    v1: dynamic<test.main.Greeter, managed, mutable, local> = load l0
    v2: int32 = call.dynamic v1, test.main.Greeter, 0(): () => int32
    return v2
}

/// @layout.struct name=test.main.Greeter size=0 align=1
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.run",
        r#"
@nocopy
type test.main.Console { }

@nocopy
type test.main.Greeter { }

function test.main.run(): int32 {
    local l0: ref<test.main.Console, managed, mutable, local>

entry:
    v0: ref<test.main.Console, managed, mutable, local> = new.zeroed test.main.Console, local
    store l0, v0
    v1: ref<test.main.Console, managed, mutable, local> = load l0
    v2: dynamic<test.main.Greeter, managed, mutable, local> = dynamic.bind v1, ref<test.main.Console, managed, mutable, local>
    v3: int32 = call test.main.talk(v2): (dynamic<test.main.Greeter, managed, mutable, local>) => int32
    return v3
}

/// @layout.struct name=test.main.Console size=0 align=1
/// @layout.struct name=test.main.Greeter size=0 align=1
"#,
    );
}

/// Compare and dispatch a nullable erased interface through its representation niche.
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

    session.assert_mir_function(
        "main.ds",
        "test.main.write",
        r#"
@nocopy
type test.main.Sink { }

function test.main.write(v0: variant<uint1> { 0uint1 = dynamic<test.main.Sink, managed, mutable, local>; 1uint1 = void; }): int32 {
    local l0: variant<uint1> { 0uint1 = dynamic<test.main.Sink, managed, mutable, local>; 1uint1 = void; }

entry(v0: variant<uint1> { 0uint1 = dynamic<test.main.Sink, managed, mutable, local>; 1uint1 = void; }):
    store l0, v0
    v1: uint1 = variant.tag.load l0
    v2: uint1 = 1
    v3: boolean = eq v1, v2
    branch v3 => b1 | b2

b1:
    v4: int32 = 0
    return v4

b2:
    v5: dynamic<test.main.Sink, managed, mutable, local> = load (l0 as 0)
    v6: int32 = 1
    v7: int32 = call.dynamic v5, test.main.Sink, 0(v6): (int32) => int32
    return v7
}

/// @layout.struct name=test.main.Sink size=0 align=1
/// @layout.variant name=type@7 size=16 align=8
/// @layout.discriminant owner=type@7 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=0
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

    session.assert_mir_function(
        "main.ds",
        "test.main.Register.name",
        r#"
@nocopy
type test.main.Register { }

function test.main.Register.name(v0: ref<test.main.Register, managed, mutable, local>): int32 {
    local l0: ref<test.main.Register, managed, mutable, local>

entry(v0: ref<test.main.Register, managed, mutable, local>):
    store l0, v0
    v1: int32 = 1
    return v1
}

/// @layout.struct name=test.main.Register size=0 align=1
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Register.count",
        r#"
@nocopy
type test.main.Register { }

function test.main.Register.count(v0: ref<test.main.Register, managed, mutable, local>): int32 {
    local l0: ref<test.main.Register, managed, mutable, local>

entry(v0: ref<test.main.Register, managed, mutable, local>):
    store l0, v0
    v1: int32 = 2
    return v1
}

/// @layout.struct name=test.main.Register size=0 align=1
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
@nocopy
type test.main.Counted extends test.main.Named { }

function test.main.read(v0: dynamic<test.main.Counted, managed, mutable, local>): int32 {
    local l0: dynamic<test.main.Counted, managed, mutable, local>

entry(v0: dynamic<test.main.Counted, managed, mutable, local>):
    store l0, v0
    v1: dynamic<test.main.Counted, managed, mutable, local> = load l0
    v2: int32 = call.dynamic v1, test.main.Counted, 0(): () => int32
    v3: dynamic<test.main.Counted, managed, mutable, local> = load l0
    v4: int32 = call.dynamic v3, test.main.Counted, 1(): () => int32
    v5: int32 = add v2, v4
    return v5
}

/// @layout.struct name=test.main.Counted size=0 align=1
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.run",
        r#"
@nocopy
type test.main.Register { }

@nocopy
type test.main.Counted extends test.main.Named { }

function test.main.run(): int32 {
    local l0: ref<test.main.Register, managed, mutable, local>

entry:
    v0: ref<test.main.Register, managed, mutable, local> = new.zeroed test.main.Register, local
    store l0, v0
    v1: ref<test.main.Register, managed, mutable, local> = load l0
    v2: dynamic<test.main.Counted, managed, mutable, local> = dynamic.bind v1, ref<test.main.Register, managed, mutable, local>
    v3: int32 = call test.main.read(v2): (dynamic<test.main.Counted, managed, mutable, local>) => int32
    return v3
}

/// @layout.struct name=test.main.Register size=0 align=1
/// @layout.struct name=test.main.Counted size=0 align=1
"#,
    );
}

/// An open payload erases into its constraint through a bind the specialization closes.
#[test]
fn test_lower_an_erasure_of_a_parameter_into_its_constraint() {
    let session = TestSession::single(
        r#"
interface Greeter {
    greet(): int32;
}

function erase<T: Greeter>(value: T): Greeter {
    return value;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.erase",
        r#"
@nocopy
type test.main.Greeter { }

function test.main.erase<T: test.main.Greeter>(v0: T): dynamic<test.main.Greeter, managed, mutable, local> {
    local l0: T

entry(v0: T):
    store l0, v0
    v1: T = load l0
    v2: dynamic<test.main.Greeter, managed, mutable, local> = dynamic.bind v1, T
    return v2
}
"#,
    );
}
