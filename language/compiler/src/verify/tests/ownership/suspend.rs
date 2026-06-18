use crate::tests::TestProgram;

#[test]
fn test_reject_managed_borrow_across_yield() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable>): int32 {
b0(v0: ref<User, managed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = 0int32
    yield v2 => b1(v0, v1)
b1(v3: ref<User, managed, mutable>, v4: ref<int32, borrowed, readonly>):
    v5: int32 = load v4
    return v5
}"#,
    );

    program.assert_error_managed_borrow_across_suspension();
}

#[test]
fn test_reject_managed_borrow_yield_value() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable>): int32 {
b0(v0: ref<User, managed, mutable>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    yield v1 => b1(v0)
b1(v2: ref<User, managed, mutable>):
    v3: int32 = 0int32
    return v3
}"#,
    );

    program.assert_error_managed_borrow_across_suspension();
}

#[test]
fn test_reject_shared_managed_borrow_across_yield() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, managed, mutable, space(shared)>): int32 {
b0(v0: ref<User, managed, mutable, space(shared)>):
    v1: ref<int32, borrowed, readonly, space(shared)> = field.address v0, 0
    v2: int32 = 0int32
    yield v2 => b1(v0, v1)
b1(v3: ref<User, managed, mutable, space(shared)>, v4: ref<int32, borrowed, readonly, space(shared)>):
    v5: int32 = load v4
    return v5
}"#,
    );

    program.assert_error_managed_borrow_across_suspension();
}

#[test]
fn test_allow_shared_unique_borrow_across_yield() {
    let mut program = TestProgram::mir(
        r#"
type User {
    id: int32;
}

function test(v0: ref<User, unique, mutable, space(shared)>): int32 {
b0(v0: ref<User, unique, mutable, space(shared)>):
    v1: ref<int32, borrowed, readonly, space(shared)> = field.address v0, 0
    v2: int32 = 0int32
    yield v2 => b1(v0, v1)
b1(v3: ref<User, unique, mutable, space(shared)>, v4: ref<int32, borrowed, readonly, space(shared)>):
    v5: int32 = load v4
    return v5
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_frame_borrow_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32): int32 {
    local l0: int32
entry(v0: int32):
    local.set l0, v0
    v1: ref<int32, borrowed, mutable, space(frame)> = local.address l0
    yield v0 => b1(v1)
b1(v2: int32, v3: ref<int32, borrowed, mutable, space(frame)>):
    v4: int32 = load v3
    return v4
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_reject_missing_parameter_borrow_obligation() {
    let mut program = TestProgram::mir(
        r#"
function test<L0: lifetime>(v0: ref<int32, borrowed, lifetime(L0), readonly>): int32 {
entry(v0: ref<int32, borrowed, lifetime(L0), readonly>):
    yield v0 => b1(v0)
b1(v1: ref<int32, borrowed, lifetime(L0), readonly>):
    v2: int32 = load v1
    return v2
}"#,
    );

    program.assert_error_undeclared_borrow_obligation();
}

#[test]
fn test_require_mutable_parameter_borrow_source_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function test<L0: lifetime>(v0: ref<int32, borrowed, lifetime(L0), mutable> @suspensionSafe(L0)): int32 {
entry(v0: ref<int32, borrowed, lifetime(L0), mutable>):
    yield v0 => b1(v0)
b1(v1: ref<int32, borrowed, lifetime(L0), mutable>):
    v2: int32 = load v1
    return v2
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_require_readonly_parameter_borrow_source_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function test<L0: lifetime>(v0: ref<int32, borrowed, lifetime(L0), readonly> @suspensionSafe(L0)): int32 {
entry(v0: ref<int32, borrowed, lifetime(L0), readonly>):
    yield v0 => b1(v0)
b1(v1: ref<int32, borrowed, lifetime(L0), readonly>):
    v2: int32 = load v1
    return v2
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_apply_aggregate_parameter_lifetime_across_yield() {
    let mut program = TestProgram::mir(
        r#"
type Holder<L: lifetime> {
    value: ref<int32, borrowed, readonly, lifetime(L)>;
}

function test<L0: lifetime>(v0: ref<int32, borrowed, readonly>, v1: ref<int32, borrowed, readonly>, v2: Holder<lifetime(L0)> @suspensionSafe(L0)): int32 {
entry(v0: ref<int32, borrowed, readonly>, v1: ref<int32, borrowed, readonly>, v2: Holder<lifetime(L0)>):
    v3: ref<int32, borrowed, readonly, lifetime(L0)> = field.get v2, 0
    yield v3 => b1(v3)
b1(v4: ref<int32, borrowed, readonly, lifetime(L0)>):
    v5: int32 = load v4
    return v5
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_apply_only_live_aggregate_path_lifetime_across_yield() {
    let mut program = TestProgram::mir(
        r#"
type Pair<A: lifetime, B: lifetime> {
    left: ref<int32, borrowed, readonly, lifetime(A)>;
    right: ref<int32, borrowed, readonly, lifetime(B)>;
}

function test<L0: lifetime, L1: lifetime>(v0: Pair<lifetime(L0), lifetime(L1)> @suspensionSafe(L1)): int32 {
entry(v0: Pair<lifetime(L0), lifetime(L1)>):
    v1: ref<int32, borrowed, readonly, lifetime(L1)> = field.get v0, 1
    yield v1 => b1(v1)
b1(v2: ref<int32, borrowed, readonly, lifetime(L1)>):
    v3: int32 = load v2
    return v3
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_static_borrow_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: ref<int32, borrowed, readonly, lifetime(static)>): int32 {
entry(v0: ref<int32, borrowed, readonly, lifetime(static)>):
    yield v0 => b1(v0)
b1(v1: ref<int32, borrowed, readonly, lifetime(static)>):
    v2: int32 = load v1
    return v2
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_frame_exclusive_borrow_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32): int32 {
    local l0: int32
entry(v0: int32):
    local.set l0, v0
    v1: ref<int32, borrowed, exclusive, space(frame)> = local.address l0
    yield v0 => b1(v1)
b1(v2: int32, v3: ref<int32, borrowed, exclusive, space(frame)>):
    v4: int32 = load v3
    return v4
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_require_exclusive_parameter_source_across_yield() {
    let mut program = TestProgram::mir(
        r#"
function test<L0: lifetime>(v0: ref<int32, borrowed, lifetime(L0), exclusive> @suspensionSafe(L0)): int32 {
entry(v0: ref<int32, borrowed, lifetime(L0), exclusive>):
    yield v0 => b1(v0)
b1(v1: ref<int32, borrowed, lifetime(L0), exclusive>):
    v2: int32 = load v1
    return v2
}"#,
    );

    program.assert_no_ownership_errors();
}

#[test]
fn test_allow_borrow_before_yield() {
    let mut program = TestProgram::mir(
        r#"
function test(v0: int32): int32 {
    local l0: int32
entry(v0: int32):
    local.set l0, v0
    v1: ref<int32, borrowed, mutable, space(frame)> = local.address l0
    v2: int32 = load v1
    yield v2 => b1(v2)
b1(v3: int32, v4: int32):
    return v4
}"#,
    );

    program.assert_no_ownership_errors();
}
