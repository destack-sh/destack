use crate::tests::TestSession;

/// Read a bigint literal from the immortal BigInt object declared for its value.
#[test]
fn test_lower_bigint_literal_to_immortal_object_read() {
    let session = TestSession::single(
        r#"
function big(): bigint {
    return 42n;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
type destack.memory.unique.Unique<slice<uninit<uint64>, managed, mutable>> = slice<uninit<uint64>, unique, exclusive>;

type destack.math.bigint.BigInt {
    sign: int8;
    limbs: destack.memory.unique.Unique<slice<uninit<uint64>, managed, mutable>>;
    length: usize;
}

immortal constant bigint.42.limbs: [uint64; 1] = b"*\x00\x00\x00\x00\x00\x00\x00"

immortal constant bigint.42: destack.math.bigint.BigInt = {1, {globalAddress bigint.42.limbs, 1uint64}, 1}

function test.main.big(): ref<destack.math.bigint.BigInt, managed, mutable> {
entry:
    v0: ref<destack.math.bigint.BigInt, managed, mutable> = global.address bigint.42
    return v0
}
/// @layout.struct name=destack.math.bigint.BigInt size=32 align=8
/// @layout.field owner=destack.math.bigint.BigInt index=0 name=sign offset=24 size=1 align=1
/// @layout.field owner=destack.math.bigint.BigInt index=1 name=limbs offset=0 size=16 align=8
/// @layout.field owner=destack.math.bigint.BigInt index=2 name=length offset=16 size=8 align=8
"#,
    );
}
