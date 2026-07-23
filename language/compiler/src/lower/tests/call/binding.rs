use crate::tests::TestSession;

#[test]
fn test_lower_binding_call_to_a_dotted_host_extern() {
    let session = TestSession::single(
        r#"
@binding("destack.clock.now", {
    provider: "host",
    effect: "external",
    requires: [],
})
declare function now(): float64;

function sample(): float64 {
    return now();
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.sample(): float64 {
entry:
    v0: float64 = call destack.clock.now()
    return v0
}

@binding("destack.clock.now")
external function destack.clock.now(): float64
"#,
    );
}

#[test]
fn test_lower_binding_borrow_parameters_with_polymorphic_lifetimes() {
    let session = TestSession::single(
        r#"
class User {
    id: int32;
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
type User {
    id: int32;
}

function main.read<'a>(v0: ref<User, borrowed, 'a, readonly>): int32 {
entry(v0: ref<User, borrowed, 'a, readonly>):
    v1: int32 = call host.user.inspect(v0)
    return v1
}

@binding("host.user.inspect")
external function host.user.inspect<'a>(ref<User, borrowed, 'a, readonly>): int32
/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
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

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.greet(v0: variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; }): void {
entry(v0: variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; }):
    return
}

function main.run(): void {
entry:
    v0: variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; } = variant.new 1
    call main.greet(v0)
    return
}
/// @layout.variant name=type@3 size=1 align=1
/// @layout.discriminant owner=type@3 kind=niche offset=0 byte_len=1 bit_offset=0 bit_len=8 untagged=0 niche_cases=1..1 niche_start=2
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=0
"#,
    );
}
