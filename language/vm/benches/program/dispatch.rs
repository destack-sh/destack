use super::Program;
use destack_vm::memory::Value;

/// All dispatch benchmark programs.
pub(crate) const ALL: &[&Program] = &[
    &LOOP_COUNTDOWN,
    &LOOP_NESTED,
    &BRANCH_PREDICTABLE,
    &BRANCH_ALTERNATING,
    &SWITCH_4WAY,
    &SWITCH_16WAY,
    &STATE_MACHINE,
];

/// Simple countdown loop measuring baseline dispatch overhead.
pub(crate) const LOOP_COUNTDOWN: Program = Program {
    name: "loop_countdown",
    source: r#"
function @loop_countdown(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    v2 = icmp_eq v0, v1
    branch v2, block2, block1
block1:
    v3 = iconst 1i64
    v4 = isub v0, v3
    jump block0(v4)
block2:
    return v0
}
"#,
    entry: "loop_countdown",
    expected: || Some(Value::int64(0)),
    default_args: || vec![Value::int64(10_000)],
};

/// Nested loop with outer times inner iterations.
pub(crate) const LOOP_NESTED: Program = Program {
    name: "loop_nested",
    source: r#"
function @loop_nested(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    jump block1(v1, v1)
block1(v2: i64, v3: i64):
    v4 = icmp_sge v2, v0
    branch v4, block5(v3), block2
block2:
    v5 = iconst 0i64
    jump block3(v5, v3)
block3(v6: i64, v7: i64):
    v8 = icmp_sge v6, v0
    branch v8, block4(v7), block6
block6:
    v9 = iconst 1i64
    v10 = iadd v7, v9
    v11 = iadd v6, v9
    jump block3(v11, v10)
block4(v12: i64):
    v13 = iconst 1i64
    v14 = iadd v2, v13
    jump block1(v14, v12)
block5(v15: i64):
    return v15
}
"#,
    entry: "loop_nested",
    expected: || Some(Value::int64(10000)),
    default_args: || vec![Value::int64(100)],
};

/// Predictable branch where condition is always true.
pub(crate) const BRANCH_PREDICTABLE: Program = Program {
    name: "branch_predictable",
    source: r#"
function @branch_predictable(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    v2 = iconst 1i64
    jump block1(v1, v1)
block1(v3: i64, v4: i64):
    v5 = icmp_sge v3, v0
    branch v5, block4(v4), block2
block2:
    v6 = iconst 1i64
    v7 = icmp_sgt v6, v1
    branch v7, block3, block3
block3:
    v8 = iadd v4, v6
    v9 = iadd v3, v6
    jump block1(v9, v8)
block4(v10: i64):
    return v10
}
"#,
    entry: "branch_predictable",
    expected: || Some(Value::int64(10_000)),
    default_args: || vec![Value::int64(10_000)],
};

/// Alternating branch where condition flips each iteration.
pub(crate) const BRANCH_ALTERNATING: Program = Program {
    name: "branch_alternating",
    source: r#"
function @branch_alternating(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    jump block1(v1, v1)
block1(v2: i64, v3: i64):
    v4 = icmp_sge v2, v0
    branch v4, block5(v3), block2
block2:
    v5 = iconst 2i64
    v6 = srem v2, v5
    v7 = iconst 0i64
    v8 = icmp_eq v6, v7
    branch v8, block3, block4
block3:
    v9 = iconst 1i64
    v10 = iadd v3, v9
    v11 = iadd v2, v9
    jump block1(v11, v10)
block4:
    v12 = iconst 1i64
    v13 = iadd v2, v12
    jump block1(v13, v3)
block5(v14: i64):
    return v14
}
"#,
    entry: "branch_alternating",
    expected: || Some(Value::int64(5000)),
    default_args: || vec![Value::int64(10_000)],
};

/// Four way switch dispatch testing jump tables.
pub(crate) const SWITCH_4WAY: Program = Program {
    name: "switch_4way",
    source: r#"
function @switch_4way(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    jump block1(v1, v1)
block1(v2: i64, v3: i64):
    v4 = icmp_sge v2, v0
    branch v4, block8(v3), block2
block2:
    v5 = iconst 4i64
    v6 = srem v2, v5
    switch v6, block7, 0 => block3, 1 => block4, 2 => block5, 3 => block7
block3:
    v7 = iconst 1i64
    jump block6(v7)
block4:
    v8 = iconst 2i64
    jump block6(v8)
block5:
    v9 = iconst 3i64
    jump block6(v9)
block7:
    v14 = iconst 0i64
    jump block6(v14)
block6(v10: i64):
    v11 = iadd v3, v10
    v12 = iconst 1i64
    v13 = iadd v2, v12
    jump block1(v13, v11)
block8(v15: i64):
    return v15
}
"#,
    entry: "switch_4way",
    // 0+1+2+3 = 6 per 4 iters, so 10000/4 * 6 = 15000
    expected: || Some(Value::int64(15000)),
    default_args: || vec![Value::int64(10_000)],
};

/// Sixteen way switch dispatch testing larger jump tables.
pub(crate) const SWITCH_16WAY: Program = Program {
    name: "switch_16way",
    source: r#"
function @switch_16way(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    jump block1(v1, v1)
block1(v2: i64, v3: i64):
    v4 = icmp_sge v2, v0
    branch v4, block19(v3), block2
block2:
    v5 = iconst 16i64
    v6 = srem v2, v5
    switch v6, block3, 0 => block3, 1 => block4, 2 => block5, 3 => block6, 4 => block7, 5 => block8, 6 => block9, 7 => block10, 8 => block11, 9 => block12, 10 => block13, 11 => block14, 12 => block15, 13 => block16, 14 => block17, 15 => block18
block3:
    v7 = iconst 0i64
    jump block20(v7)
block4:
    v8 = iconst 1i64
    jump block20(v8)
block5:
    v9 = iconst 2i64
    jump block20(v9)
block6:
    v10 = iconst 3i64
    jump block20(v10)
block7:
    v11 = iconst 4i64
    jump block20(v11)
block8:
    v12 = iconst 5i64
    jump block20(v12)
block9:
    v13 = iconst 6i64
    jump block20(v13)
block10:
    v14 = iconst 7i64
    jump block20(v14)
block11:
    v15 = iconst 8i64
    jump block20(v15)
block12:
    v16 = iconst 9i64
    jump block20(v16)
block13:
    v17 = iconst 10i64
    jump block20(v17)
block14:
    v18 = iconst 11i64
    jump block20(v18)
block15:
    v19 = iconst 12i64
    jump block20(v19)
block16:
    v20 = iconst 13i64
    jump block20(v20)
block17:
    v21 = iconst 14i64
    jump block20(v21)
block18:
    v22 = iconst 15i64
    jump block20(v22)
block20(v24: i64):
    v25 = iadd v3, v24
    v26 = iconst 1i64
    v27 = iadd v2, v26
    jump block1(v27, v25)
block19(v28: i64):
    return v28
}
"#,
    entry: "switch_16way",
    expected: || Some(Value::int64(75000)),
    default_args: || vec![Value::int64(10_000)],
};

/// Simple state machine cycling through three states.
pub(crate) const STATE_MACHINE: Program = Program {
    name: "state_machine",
    source: r#"
function @state_machine(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    jump block1(v1, v1, v1)
block1(v2: i64, v3: i64, v4: i64):
    v5 = icmp_sge v2, v0
    branch v5, block5(v4), block2
block2:
    switch v3, block3, 0 => block3, 1 => block6, 2 => block7
block3:
    v6 = iconst 1i64
    v7 = iadd v4, v6
    v8 = iconst 1i64
    jump block4(v8, v7)
block6:
    v9 = iconst 2i64
    v10 = iadd v4, v9
    v11 = iconst 2i64
    jump block4(v11, v10)
block7:
    v12 = iconst 3i64
    v13 = iadd v4, v12
    v14 = iconst 0i64
    jump block4(v14, v13)
block4(v15: i64, v16: i64):
    v17 = iconst 1i64
    v18 = iadd v2, v17
    jump block1(v18, v15, v16)
block5(v19: i64):
    return v19
}
"#,
    entry: "state_machine",
    expected: || Some(Value::int64(19999)),
    default_args: || vec![Value::int64(10_000)],
};
