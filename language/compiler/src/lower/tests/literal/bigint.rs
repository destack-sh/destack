use crate::tests::TestSession;

/// Read a bigint literal from the constant BigInt object declared for its value.
#[test]
fn test_lower_bigint_literal_to_constant_object_read() {
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
@languageItem("math.BigInt")
type BigInt {
    sign: int8;
    limbs: slice<uninit<uint64>, unique, exclusive, local>;
    length: usize;
}

constant bigint.0: BigInt = 42n

function test.main.big(): ref<BigInt, managed, mutable, local> {
entry:
    v0: ref<BigInt, managed, mutable, local> = global.address bigint.0
    return v0
}

/// @layout.struct name=BigInt size=32 align=8
/// @layout.field owner=BigInt index=0 name=sign offset=24 size=1 align=1
/// @layout.field owner=BigInt index=1 name=limbs offset=0 size=16 align=8
/// @layout.field owner=BigInt index=2 name=length offset=16 size=8 align=8
"#,
    );
}
