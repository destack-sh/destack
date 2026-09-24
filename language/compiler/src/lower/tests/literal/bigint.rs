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

    session.assert_mir_function(
        "main.ds",
        "test.main.big",
        r#"
@nocopy
@languageItem("math.BigInt")
type BigInt;

function test.main.big(): ref<BigInt, managed, mutable, local> {
entry:
    v0: ref<BigInt, managed, mutable, local> = address @bigint.0
    return v0
}
"#,
    );
}
