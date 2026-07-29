use crate::tests::TestSession;

#[test]
fn test_lower_module_constants_to_globals() {
    let session = TestSession::single(
        r#"
newtype Flags = uint32

const READ: Flags = Flags(4);

function pick(): Flags {
    return READ;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Flags = newtype<uint32>;

readonly global test.main.READ: Flags = {4uint32}

function test.main.pick(): Flags {
entry:
    v0: ref<Flags, raw, readonly> = global.address test.main.READ
    v1: Flags = load v0
    return v1
}
"#,
    );
}
