use crate::tests::TestSession;

#[test]
fn test_lower_if_return_to_branch_with_join() {
    let session = TestSession::single(
        r#"
function max(a: int32, b: int32): int32 {
    if (a > b) {
        return a;
    }
    return b;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.max",
        r#"
function test.main.max(v0: int32, v1: int32): int32 {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32):
    store l0, v0
    store l1, v1
    v2: int32 = load l0
    v3: int32 = load l1
    v4: boolean = gt v2, v3
    branch v4 => b1 | b2

b1:
    v5: int32 = load l0
    return v5

b2:
    v6: int32 = load l1
    return v6
}
"#,
    );
}

#[test]
fn test_lower_if_else_with_both_arms_returning() {
    let session = TestSession::single(
        r#"
function pick(flag: boolean, a: int32, b: int32): int32 {
    if (flag) {
        return a;
    } else {
        return b;
    }
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.pick",
        r#"
function test.main.pick(v0: boolean, v1: int32, v2: int32): int32 {
    local l0: boolean
    local l1: int32
    local l2: int32

entry(v0: boolean, v1: int32, v2: int32):
    store l0, v0
    store l1, v1
    store l2, v2
    v3: boolean = load l0
    branch v3 => b1 | b3

b1:
    v4: int32 = load l1
    return v4

b2:
    unreachable

b3:
    v5: int32 = load l2
    return v5
}
"#,
    );
}

#[test]
fn test_join_the_arm_values_of_a_ternary() {
    let session = TestSession::single(
        r#"
function clamp(value: float32, limit: float32): float32 {
    return value > limit ? limit : value;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.clamp",
        r#"
function test.main.clamp(v0: float32, v1: float32): float32 {
    local l0: float32
    local l1: float32
    local l2: float32

entry(v0: float32, v1: float32):
    store l0, v0
    store l1, v1
    v2: float32 = load l0
    v3: float32 = load l1
    v4: boolean = gt v2, v3
    branch v4 => b1 | b2

b1:
    v5: float32 = load l1
    store l2, v5
    jump b3

b2:
    v6: float32 = load l0
    store l2, v6
    jump b3

b3:
    v7: float32 = load l2
    return v7
}
"#,
    );
}

#[test]
fn test_lower_a_value_if_with_block_arms() {
    let session = TestSession::single(
        r#"
function clamp(count: isize): isize {
    let kept = if (count < 0) { 0 } else { count };

    return kept;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.clamp",
        r#"
function test.main.clamp(v0: isize): isize {
    local l0: isize
    local l1: isize
    local l2: isize

entry(v0: isize):
    store l0, v0
    v1: isize = load l0
    v2: isize = 0
    v3: boolean = lt v1, v2
    branch v3 => b1 | b2

b1:
    v4: isize = 0
    store l1, v4
    jump b3

b2:
    v5: isize = load l0
    store l1, v5
    jump b3

b3:
    v6: isize = load l1
    store l2, v6
    v7: isize = load l2
    return v7
}
"#,
    );
}

#[test]
fn test_lower_a_returning_arm_in_value_position() {
    let session = TestSession::single(
        r#"
function checked(count: isize): isize {
    let kept = if (count < 0) { return 0 } else { count };

    return kept;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.checked",
        r#"
function test.main.checked(v0: isize): isize {
    local l0: isize
    local l1: isize
    local l2: isize

entry(v0: isize):
    store l0, v0
    v1: isize = load l0
    v2: isize = 0
    v3: boolean = lt v1, v2
    branch v3 => b1 | b2

b1:
    v4: isize = 0
    return v4

b2:
    v5: isize = load l0
    store l1, v5
    jump b3

b3:
    v6: isize = load l1
    store l2, v6
    v7: isize = load l2
    return v7
}
"#,
    );
}

#[test]
fn test_lower_a_value_if_whose_arms_both_return() {
    let session = TestSession::single(
        r#"
function pick(flag: boolean): isize {
    let kept = if (flag) { return 1 } else { return 2 };

    return kept;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.pick",
        r#"
function test.main.pick(v0: boolean): isize {
    local l0: boolean
    local l1: never
    local l2: never

entry(v0: boolean):
    store l0, v0
    v1: boolean = load l0
    branch v1 => b1 | b2

b1:
    v2: isize = 1
    return v2

b2:
    v3: isize = 2
    return v3

b3:
    unreachable

b4:
    v4: never = uninit
    store l2, v4
    v5: never = load l2
    unreachable
}
"#,
    );
}
