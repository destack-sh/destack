use crate::tests::TestProgram;

#[test]
fn test_reject_call_obligation_from_managed_borrow() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function callee<'L0>(v0: ref<int32, borrowed, 'L0, readonly> @suspensionSafe('L0)): int32 {
b0(v0: ref<int32, borrowed, 'L0, readonly>):
    v1: int32 = 0int32
    yield v1 => b1(v0)
b1(v2: ref<int32, borrowed, 'L0, readonly>):
    v3: int32 = load v2
    return v3
}

function caller(v0: ref<User, managed, mutable>): int32 {
b2(v0: ref<User, managed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = call callee(v1): <'L0>(ref<int32, borrowed, 'L0, readonly> @suspensionSafe('L0)) => int32
    return v2
}"#,
    );

    program.assert_error_managed_borrow_across_suspension();
}

#[test]
fn test_reject_call_result_borrow_from_managed_across_yield() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function callee<'L0>(v0: ref<User, managed, 'L0, mutable>): ref<int32, borrowed, 'L0, readonly> {
b0(v0: ref<User, managed, 'L0, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    return v1
}

function caller<'L0>(v0: ref<User, managed, 'L0, mutable>): int32 {
b1(v0: ref<User, managed, 'L0, mutable>):
    v1: ref<int32, borrowed, 'L0, readonly> = call callee(v0): (ref<User, managed, 'L0, mutable>) => ref<int32, borrowed, 'L0, readonly>
    v2: int32 = 0int32
    yield v2 => b2(v0, v1)
b2(v3: ref<User, managed, 'L0, mutable>, v4: ref<int32, borrowed, 'L0, readonly>):
    v5: int32 = load v4
    return v5
}"#,
    );

    program.assert_error_managed_borrow_across_suspension();
}

#[test]
fn test_reject_tail_call_result_borrow_from_managed_across_yield() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function callee<'L0>(v0: ref<User, managed, 'L0, mutable>): ref<int32, borrowed, 'L0, readonly> {
b0(v0: ref<User, managed, 'L0, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    return v1
}

function caller<'L0>(v0: ref<User, managed, 'L0, mutable>): ref<int32, borrowed, 'L0, readonly> {
b1(v0: ref<User, managed, 'L0, mutable>):
    tail.call callee(v0): (ref<User, managed, 'L0, mutable>) => ref<int32, borrowed, 'L0, readonly>
}

function outer<'L0>(v0: ref<User, managed, 'L0, mutable>): int32 {
b2(v0: ref<User, managed, 'L0, mutable>):
    v1: ref<int32, borrowed, 'L0, readonly> = call caller(v0): (ref<User, managed, 'L0, mutable>) => ref<int32, borrowed, 'L0, readonly>
    v2: int32 = 0int32
    yield v2 => b3(v0, v1)
b3(v3: ref<User, managed, 'L0, mutable>, v4: ref<int32, borrowed, 'L0, readonly>):
    v5: int32 = load v4
    return v5
}"#,
    );

    program.assert_error_managed_borrow_across_suspension();
}

#[test]
fn test_propagate_call_obligation_from_borrowed_parameter() {
    let mut program = TestProgram::mir(
        r#"
function callee<'L0>(v0: ref<int32, borrowed, 'L0, readonly> @suspensionSafe('L0)): int32 {
b0(v0: ref<int32, borrowed, 'L0, readonly>):
    v1: int32 = 0int32
    yield v1 => b1(v0)
b1(v2: ref<int32, borrowed, 'L0, readonly>):
    v3: int32 = load v2
    return v3
}

function caller<'L0>(v0: ref<int32, borrowed, 'L0, readonly> @suspensionSafe('L0)): int32 {
b2(v0: ref<int32, borrowed, 'L0, readonly>):
    v1: int32 = call callee(v0): (ref<int32, borrowed, 'L0, readonly> @suspensionSafe('L0)) => int32
    return v1
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_indirect_call_obligation_from_managed_borrow() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function caller(v0: <'L0>(ref<int32, borrowed, 'L0, readonly> @suspensionSafe('L0)) => int32, v1: ref<User, managed, mutable>): int32 {
entry(v0: <'L0>(ref<int32, borrowed, 'L0, readonly> @suspensionSafe('L0)) => int32, v1: ref<User, managed, mutable>):
    v2: ref<int32, borrowed, readonly> = field.address v1, 0
    v3: int32 = call.indirect v0(v2): <'L0>(ref<int32, borrowed, 'L0, readonly> @suspensionSafe('L0)) => int32
    return v3
}
"#,
    );

    program.assert_error_managed_borrow_across_suspension();
}

#[test]
fn test_propagate_indirect_call_obligation_from_borrowed_parameter() {
    let mut program = TestProgram::mir(
        r#"
function caller<'L1>(v0: <'L0>(ref<int32, borrowed, 'L0, readonly> @suspensionSafe('L0)) => int32, v1: ref<int32, borrowed, 'L1, readonly> @suspensionSafe('L1)): int32 {
entry(v0: <'L0>(ref<int32, borrowed, 'L0, readonly> @suspensionSafe('L0)) => int32, v1: ref<int32, borrowed, 'L1, readonly>):
    v2: int32 = call.indirect v0(v1): <'L0>(ref<int32, borrowed, 'L0, readonly> @suspensionSafe('L0)) => int32
    return v2
}
"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_require_call_result_borrow_source_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function callee<'L0>(v0: ref<int32, borrowed, 'L0, readonly>): ref<int32, borrowed, 'L0, readonly> {
b0(v0: ref<int32, borrowed, 'L0, readonly>):
    return v0
}

function caller<'L0>(v0: ref<int32, borrowed, 'L0, readonly> @suspensionSafe('L0)): int32 {
b1(v0: ref<int32, borrowed, 'L0, readonly>):
    v1: ref<int32, borrowed, 'L0, readonly> = call callee(v0): (ref<int32, borrowed, 'L0, readonly>) => ref<int32, borrowed, 'L0, readonly>
    v2: int32 = 0int32
    yield v2 => b2(v1)
b2(v3: ref<int32, borrowed, 'L0, readonly>):
    v4: int32 = load v3
    return v4
}"#,
    );

    program.assert_no_ownership_errors();
}

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
