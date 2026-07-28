use crate::tests::TestSession;

#[test]
fn test_lower_while_accumulation_through_locals() {
    let session = TestSession::single(
        r#"
function sum(n: int32): int32 {
    let total: int32 = 0;
    let i: int32 = 0;
    while (i < n) {
        total = total + i;
        i = i + 1;
    }
    return total;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.sum(v0: int32): int32 {
    local l0: int32
    local l1: int32

entry(v0: int32):
    v1: int32 = 0
    local.set l0, v1
    v2: int32 = 0
    local.set l1, v2
    jump b1

b1:
    v3: int32 = local.get l1
    v4: boolean = int.lt.s v3, v0
    branch v4, b2, b3

b2:
    v5: int32 = local.get l0
    v6: int32 = local.get l1
    v7: int32 = int.add v5, v6
    local.set l0, v7
    v8: int32 = local.get l1
    v9: int32 = 1
    v10: int32 = int.add v8, v9
    local.set l1, v10
    jump b1

b3:
    v11: int32 = local.get l0
    return v11
}
"#,
    );
}

#[test]
fn test_lower_break_out_of_unbounded_while() {
    let session = TestSession::single(
        r#"
function firstOver(limit: int32): int32 {
    let i: int32 = 0;
    while (true) {
        if (i * i > limit) {
            break;
        }
        i = i + 1;
    }
    return i;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.firstOver(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    v1: int32 = 0
    local.set l0, v1
    jump b1

b1:
    v2: boolean = true
    branch v2, b2, b3

b2:
    v3: int32 = local.get l0
    v4: int32 = local.get l0
    v5: int32 = int.mul v3, v4
    v6: boolean = int.gt.s v5, v0
    branch v6, b4, b5

b3:
    v10: int32 = local.get l0
    return v10

b4:
    jump b3

b5:
    v7: int32 = local.get l0
    v8: int32 = 1
    v9: int32 = int.add v7, v8
    local.set l0, v9
    jump b1
}
"#,
    );
}

#[test]
fn test_lower_for_loop_with_update_increment() {
    let session = TestSession::single(
        r#"
function sum(n: int32): int32 {
    let total: int32 = 0;
    for (let i: int32 = 0; i < n; i++) {
        total += i;
    }
    return total;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.sum(v0: int32): int32 {
    local l0: int32
    local l1: int32

entry(v0: int32):
    v1: int32 = 0
    local.set l0, v1
    v2: int32 = 0
    local.set l1, v2
    jump b1

b1:
    v3: int32 = local.get l1
    v4: boolean = int.lt.s v3, v0
    branch v4, b2, b4

b2:
    v5: int32 = local.get l0
    v6: int32 = local.get l1
    v7: int32 = int.add v5, v6
    local.set l0, v7
    jump b3

b3:
    v8: int32 = local.get l1
    v9: int32 = 1
    v10: int32 = int.add v8, v9
    local.set l1, v10
    jump b1

b4:
    v11: int32 = local.get l0
    return v11
}
"#,
    );
}

#[test]
fn test_lower_do_while_runs_body_before_condition() {
    let session = TestSession::single(
        r#"
function drain(n: int64): int64 {
    let left = n;
    do {
        left -= 1;
    } while (left > 0);
    return left;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.drain(v0: int64): int64 {
    local l0: int64

entry(v0: int64):
    local.set l0, v0
    jump b2

b1:
    v1: int64 = local.get l0
    v2: int64 = 0
    v3: boolean = int.gt.s v1, v2
    branch v3, b2, b3

b2:
    v4: int64 = local.get l0
    v5: int64 = 1
    v6: int64 = int.sub v4, v5
    local.set l0, v6
    jump b1

b3:
    v7: int64 = local.get l0
    return v7
}
"#,
    );
}

#[test]
fn test_lower_unconditional_loop_with_break() {
    let session = TestSession::single(
        r#"
function next(seed: int32): int32 {
    let value = seed;
    loop {
        value += 7;
        if (value > 100) {
            break;
        }
    }
    return value;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.next(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    jump b1

b1:
    v1: int32 = local.get l0
    v2: int32 = 7
    v3: int32 = int.add v1, v2
    local.set l0, v3
    v4: int32 = local.get l0
    v5: int32 = 100
    v6: boolean = int.gt.s v4, v5
    branch v6, b3, b4

b2:
    v7: int32 = local.get l0
    return v7

b3:
    jump b2

b4:
    jump b1
}
"#,
    );
}

#[test]
fn test_lower_labeled_break_exits_outer_loop() {
    let session = TestSession::single(
        r#"
function find(limit: int32): int32 {
    let hits: int32 = 0;
    outer: for (let i: int32 = 0; i < limit; i++) {
        for (let j: int32 = 0; j < limit; j++) {
            if (i * j > limit) {
                break outer;
            }
            hits += 1;
        }
    }
    return hits;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function test.main.find(v0: int32): int32 {
    local l0: int32
    local l1: int32
    local l2: int32

entry(v0: int32):
    v1: int32 = 0
    local.set l0, v1
    v2: int32 = 0
    local.set l1, v2
    jump b1

b1:
    v3: int32 = local.get l1
    v4: boolean = int.lt.s v3, v0
    branch v4, b2, b4

b2:
    v5: int32 = 0
    local.set l2, v5
    jump b5

b3:
    v18: int32 = local.get l1
    v19: int32 = 1
    v20: int32 = int.add v18, v19
    local.set l1, v20
    jump b1

b4:
    v21: int32 = local.get l0
    return v21

b5:
    v6: int32 = local.get l2
    v7: boolean = int.lt.s v6, v0
    branch v7, b6, b8

b6:
    v8: int32 = local.get l1
    v9: int32 = local.get l2
    v10: int32 = int.mul v8, v9
    v11: boolean = int.gt.s v10, v0
    branch v11, b9, b10

b7:
    v15: int32 = local.get l2
    v16: int32 = 1
    v17: int32 = int.add v15, v16
    local.set l2, v17
    jump b5

b8:
    jump b3

b9:
    jump b4

b10:
    v12: int32 = local.get l0
    v13: int32 = 1
    v14: int32 = int.add v12, v13
    local.set l0, v14
    jump b7
}
"#,
    );
}
