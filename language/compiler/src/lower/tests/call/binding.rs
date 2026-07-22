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

function main.read<L0: lifetime>(v0: ref<User, borrowed, lifetime(L0), readonly>): int32 {
entry(v0: ref<User, borrowed, lifetime(L0), readonly>):
    v1: int32 = call host.user.inspect(v0)
    return v1
}

@binding("host.user.inspect")
external function host.user.inspect<L0: lifetime>(ref<User, borrowed, lifetime(L0), readonly>): int32
/// @layout.struct name=User size=4 align=4
/// @layout.field owner=User index=0 name=id offset=0 size=4 align=4
"#,
    );
}
