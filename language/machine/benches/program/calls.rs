use super::Program;
use destack_machine::memory::Value;

/// All function call benchmark programs.
pub(crate) const ALL: &[&Program] = &[
    &CALL_DIRECT_SHALLOW,
    &CALL_DIRECT_DEEP,
    &CALL_MANY_ARGS,
    &CALL_RECURSIVE_TAIL,
    &CALL_MUTUAL,
];

/// Five deep call chain measuring call overhead.
pub(crate) const CALL_DIRECT_SHALLOW: Program = Program {
    name: "call_direct_shallow",
    source: r#"
function @f5(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 1i64
    v2 = iadd v0, v1
    return v2
}

function @f4(v0: i64) -> i64 {
block0(v0: i64):
    v1 = call @f5(v0)
    return v1
}

function @f3(v0: i64) -> i64 {
block0(v0: i64):
    v1 = call @f4(v0)
    return v1
}

function @f2(v0: i64) -> i64 {
block0(v0: i64):
    v1 = call @f3(v0)
    return v1
}

function @f1(v0: i64) -> i64 {
block0(v0: i64):
    v1 = call @f2(v0)
    return v1
}

function @call_direct_shallow(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    jump block1(v1, v1)
block1(v2: i64, v3: i64):
    v4 = icmp_sge v2, v0
    branch v4, block3(v3), block2
block2:
    v5 = call @f1(v3)
    v6 = iconst 1i64
    v7 = iadd v2, v6
    jump block1(v7, v5)
block3(v8: i64):
    return v8
}
"#,
    entry: "call_direct_shallow",
    expected: || Some(Value::int64(1000)),
    default_args: || vec![Value::int64(1000)],
};

/// Twenty deep call chain measuring deep call stacks.
pub(crate) const CALL_DIRECT_DEEP: Program = Program {
    name: "call_direct_deep",
    source: r#"
function @d20(v0: i64) -> i64 { block0(v0: i64): v1 = iconst 1i64 v2 = iadd v0, v1 return v2 }
function @d19(v0: i64) -> i64 { block0(v0: i64): v1 = call @d20(v0) return v1 }
function @d18(v0: i64) -> i64 { block0(v0: i64): v1 = call @d19(v0) return v1 }
function @d17(v0: i64) -> i64 { block0(v0: i64): v1 = call @d18(v0) return v1 }
function @d16(v0: i64) -> i64 { block0(v0: i64): v1 = call @d17(v0) return v1 }
function @d15(v0: i64) -> i64 { block0(v0: i64): v1 = call @d16(v0) return v1 }
function @d14(v0: i64) -> i64 { block0(v0: i64): v1 = call @d15(v0) return v1 }
function @d13(v0: i64) -> i64 { block0(v0: i64): v1 = call @d14(v0) return v1 }
function @d12(v0: i64) -> i64 { block0(v0: i64): v1 = call @d13(v0) return v1 }
function @d11(v0: i64) -> i64 { block0(v0: i64): v1 = call @d12(v0) return v1 }
function @d10(v0: i64) -> i64 { block0(v0: i64): v1 = call @d11(v0) return v1 }
function @d9(v0: i64) -> i64 { block0(v0: i64): v1 = call @d10(v0) return v1 }
function @d8(v0: i64) -> i64 { block0(v0: i64): v1 = call @d9(v0) return v1 }
function @d7(v0: i64) -> i64 { block0(v0: i64): v1 = call @d8(v0) return v1 }
function @d6(v0: i64) -> i64 { block0(v0: i64): v1 = call @d7(v0) return v1 }
function @d5(v0: i64) -> i64 { block0(v0: i64): v1 = call @d6(v0) return v1 }
function @d4(v0: i64) -> i64 { block0(v0: i64): v1 = call @d5(v0) return v1 }
function @d3(v0: i64) -> i64 { block0(v0: i64): v1 = call @d4(v0) return v1 }
function @d2(v0: i64) -> i64 { block0(v0: i64): v1 = call @d3(v0) return v1 }
function @d1(v0: i64) -> i64 { block0(v0: i64): v1 = call @d2(v0) return v1 }

function @call_direct_deep(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    jump block1(v1, v1)
block1(v2: i64, v3: i64):
    v4 = icmp_sge v2, v0
    branch v4, block3(v3), block2
block2:
    v5 = call @d1(v3)
    v6 = iconst 1i64
    v7 = iadd v2, v6
    jump block1(v7, v5)
block3(v8: i64):
    return v8
}
"#,
    entry: "call_direct_deep",
    expected: || Some(Value::int64(1000)),
    default_args: || vec![Value::int64(1000)],
};

/// Eight argument function call measuring argument passing.
pub(crate) const CALL_MANY_ARGS: Program = Program {
    name: "call_many_args",
    source: r#"
function @sum8(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64, v6: i64, v7: i64) -> i64 {
block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64, v5: i64, v6: i64, v7: i64):
    v8 = iadd v0, v1
    v9 = iadd v8, v2
    v10 = iadd v9, v3
    v11 = iadd v10, v4
    v12 = iadd v11, v5
    v13 = iadd v12, v6
    v14 = iadd v13, v7
    return v14
}

function @call_many_args(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    jump block1(v1, v1)
block1(v2: i64, v3: i64):
    v4 = icmp_sge v2, v0
    branch v4, block3(v3), block2
block2:
    v5 = iconst 1i64
    v6 = iconst 2i64
    v7 = iconst 3i64
    v8 = iconst 4i64
    v9 = iconst 5i64
    v10 = iconst 6i64
    v11 = iconst 7i64
    v12 = iconst 8i64
    v13 = call @sum8(v5, v6, v7, v8, v9, v10, v11, v12)
    v14 = iadd v3, v13
    v15 = iadd v2, v5
    jump block1(v15, v14)
block3(v16: i64):
    return v16
}
"#,
    entry: "call_many_args",
    expected: || Some(Value::int64(36000)),
    default_args: || vec![Value::int64(1000)],
};

/// Tail recursive countdown (not optimized).
pub(crate) const CALL_RECURSIVE_TAIL: Program = Program {
    name: "call_recursive_tail",
    source: r#"
function @countdown_rec(v0: i64, v1: i64) -> i64 {
block0(v0: i64, v1: i64):
    v2 = iconst 0i64
    v3 = icmp_eq v0, v2
    branch v3, block2, block1
block1:
    v4 = iconst 1i64
    v5 = isub v0, v4
    v6 = iadd v1, v4
    v7 = call @countdown_rec(v5, v6)
    return v7
block2:
    return v1
}

function @call_recursive_tail(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    v2 = call @countdown_rec(v0, v1)
    return v2
}
"#,
    entry: "call_recursive_tail",
    expected: || Some(Value::int64(1000)),
    default_args: || vec![Value::int64(1000)],
};

/// Mutually recursive even and odd functions.
pub(crate) const CALL_MUTUAL: Program = Program {
    name: "call_mutual",
    source: r#"
function @is_odd(v0: i64) -> bool {
block0(v0: i64):
    v1 = iconst 0i64
    v2 = icmp_eq v0, v1
    branch v2, block2, block1
block1:
    v3 = iconst 1i64
    v4 = isub v0, v3
    v5 = call @is_even(v4)
    return v5
block2:
    v6 = iconst false
    return v6
}

function @is_even(v0: i64) -> bool {
block0(v0: i64):
    v1 = iconst 0i64
    v2 = icmp_eq v0, v1
    branch v2, block2, block1
block1:
    v3 = iconst 1i64
    v4 = isub v0, v3
    v5 = call @is_odd(v4)
    return v5
block2:
    v6 = iconst true
    return v6
}

function @call_mutual(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    jump block1(v1, v1)
block1(v2: i64, v3: i64):
    v4 = icmp_sge v2, v0
    branch v4, block4(v3), block2
block2:
    v5 = call @is_even(v2)
    branch v5, block3, block6
block3:
    v6 = iconst 1i64
    v7 = iadd v3, v6
    jump block5(v7)
block6:
    jump block5(v3)
block5(v8: i64):
    v9 = iconst 1i64
    v10 = iadd v2, v9
    jump block1(v10, v8)
block4(v11: i64):
    return v11
}
"#,
    entry: "call_mutual",
    expected: || Some(Value::int64(500)),
    default_args: || vec![Value::int64(1000)],
};
