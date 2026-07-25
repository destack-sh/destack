use crate::tests::TestProgram;

#[test]
fn test_propagate_call_result_aggregate_path_sources() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function callee<'L0, 'L1>(v0: Pair<'L0, 'L1>): Pair<'L0, 'L1> {
b0(v0: Pair<'L0, 'L1>):
    return v0
}

function caller<'L0, 'L1>(v0: Pair<'L0, 'L1>): ref<int32, borrowed, 'L1, readonly> {
b1(v0: Pair<'L0, 'L1>):
    v1: Pair<'L0, 'L1> = call callee(v0): (Pair<'L0, 'L1>) => Pair<'L0, 'L1>
    v2: ref<int32, borrowed, 'L1, readonly> = field.get v1, 1
    return v2
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_call_result_aggregate_wrong_path_source() {
    let mut program = TestProgram::mir(
        r#"
type Pair<'A, 'B> {
    left: ref<int32, borrowed, 'A, readonly>;
    right: ref<int32, borrowed, 'B, readonly>;
}

function callee<'L0, 'L1>(v0: Pair<'L0, 'L1>): Pair<'L0, 'L1> {
b0(v0: Pair<'L0, 'L1>):
    return v0
}

function caller<'L0, 'L1>(v0: Pair<'L0, 'L1>): ref<int32, borrowed, 'L0, readonly> {
b1(v0: Pair<'L0, 'L1>):
    v1: Pair<'L0, 'L1> = call callee(v0): (Pair<'L0, 'L1>) => Pair<'L0, 'L1>
    v2: ref<int32, borrowed, 'L1, readonly> = field.get v1, 1
    return v2
}"#,
    );

    program.assert_error_borrow_outlives_origin();
}
