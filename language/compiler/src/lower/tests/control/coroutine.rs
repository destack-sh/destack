use crate::tests::TestSession;

#[test]
fn test_pin_async_function_lowering() {
    let session = TestSession::single(
        r#"
async function fetchCount(): Promise<int32> {
    return 1;
}
"#,
    );

    session.assert_mir_diagnostics("main.ds", r#"
/// @diagnostic.error id=unsupported-native-construct message="native compilation does not support the 'Never' type"
/// @diagnostic.label file="main.ds"
"#);
}

#[test]
fn test_pin_generator_function_lowering() {
    let session = TestSession::single(
        r#"
function* count(limit: int32): Generator<int32, void, void> {
    for (let value: int32 = 0; value < limit; value += 1) {
        yield value;
    }
}
"#,
    );

    session.assert_mir_diagnostics("main.ds", r#"
/// @diagnostic.error id=unsupported-native-construct message="native compilation does not support 'Yield' statements"
/// @diagnostic.label file="main.ds"
"#);
}
