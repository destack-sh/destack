use crate::tests::TestSession;

#[test]
fn test_lower_binding_call_to_a_dotted_host_extern() {
    let session = TestSession::single(
        r#"
@binding("destack.clock.now", {
    provider: "host",
    effect: "external",
    replay: "forbidden",
    affinity: "worker",
    requires: ["host.time.clock.read"],
    platforms: ["linux"],
    families: ["unix"],
    hosts: ["native"],
})
declare function now(): float64;

function sample(): float64 {
    return now();
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.sample",
        r#"
function test.main.sample(): float64 {
entry:
    v0: float64 = call destack.clock.now(): () => float64
    return v0
}
"#,
    );
}

#[test]
fn test_lower_binding_borrow_parameters_with_polymorphic_lifetimes() {
    let session = TestSession::single(
        r#"
class User {
    id: int32 = 0;
}

@binding("host.user.inspect", {
    provider: "host",
    effect: "external",
    requires: [],
})
declare function inspect(value: &readonly User): int32;

function read(value: &readonly User): int32 {
    return inspect(value);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.User.constructor",
        r#"
@nocopy
type test.main.User {
    id: int32;
}

constructor test.main.User.constructor<'a>(v0: ref<uninit<test.main.User>, borrowed, 'a, exclusive>): void {
    local l0: ref<uninit<test.main.User>, borrowed, 'a, exclusive>

entry(v0: ref<uninit<test.main.User>, borrowed, 'a, exclusive>):
    store l0, v0
    v1: int32 = 0
    v2: ref<uninit<test.main.User>, borrowed, 'a, exclusive> = address (*l0)
    store (*v2).0, v1
    return
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
@nocopy
type test.main.User {
    id: int32;
}

function test.main.read<'a>(v0: ref<test.main.User, borrowed, 'a, readonly>): int32 {
    local l0: ref<test.main.User, borrowed, 'a, readonly>

entry(v0: ref<test.main.User, borrowed, 'a, readonly>):
    store l0, v0
    v1: ref<test.main.User, borrowed, 'a, readonly> = load l0
    v2: int32 = call host.user.inspect(v1): (ref<test.main.User, borrowed, 'a, readonly>) => int32
    return v2
}

/// @layout.struct name=test.main.User size=4 align=4
/// @layout.field owner=test.main.User index=0 name=id offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_omitted_optional_argument_to_undefined() {
    let session = TestSession::single(
        r#"
function greet(name?: boolean | undefined): void {}

function run(): void {
    greet();
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.greet", r#"
function test.main.greet(v0: variant<uint1> { 0uint1 = boolean; 1uint1 = void; }): void {
    local l0: variant<uint1> { 0uint1 = boolean; 1uint1 = void; }

entry(v0: variant<uint1> { 0uint1 = boolean; 1uint1 = void; }):
    store l0, v0
    return
}

/// @layout.variant name=type@3 size=1 align=1
/// @layout.discriminant owner=type@3 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=2
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=0
"#);

    session.assert_mir_function("main.ds", "test.main.run", r#"
function test.main.run(): void {
entry:
    v0: variant<uint1> { 0uint1 = boolean; 1uint1 = void; } = variant.new 1
    call test.main.greet(v0): (variant<uint1> { 0uint1 = boolean; 1uint1 = void; }) => void
    return
}

/// @layout.variant name=type@3 size=1 align=1
/// @layout.discriminant owner=type@3 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_start=2
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=0
"#);
}
