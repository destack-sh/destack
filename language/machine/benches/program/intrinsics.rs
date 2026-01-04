use super::Program;
use destack_machine::memory::Value;

/// All intrinsics benchmark programs.
pub(crate) const ALL: &[&Program] = &[&BIT_OPS, &MATH_FLOAT, &CHECKED_ARITH];

/// Bit manipulation intrinsics including clz, ctz, and popcnt.
pub(crate) const BIT_OPS: Program = Program {
    name: "bit_ops",
    source: r#"
function @bit_ops(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    v2 = iconst 773738404492881801i64
    jump block1(v1, v2)
block1(v3: i64, v4: i64):
    v5 = icmp_sge v3, v0
    branch v5, block3(v4), block2
block2:
    v6 = intrinsic.clz(v4)
    v7 = intrinsic.ctz(v4)
    v8 = intrinsic.popcnt(v4)
    v9 = iconst 1i64
    v10 = ishl v4, v9
    v11 = bxor v10, v4
    v12 = iadd v3, v9
    jump block1(v12, v11)
block3(v13: i64):
    return v13
}
"#,
    entry: "bit_ops",
    expected: || None,
    default_args: || vec![Value::int64(10_000)],
};

/// Float math intrinsics including sqrt, sin, and cos.
pub(crate) const MATH_FLOAT: Program = Program {
    name: "math_float",
    source: r#"
function @math_float(v0: i64, v1: f64, v2: f64) -> f64 {
block0(v0: i64, v1: f64, v2: f64):
    v3 = iconst 0i64
    jump block1(v3, v1, v2)
block1(v4: i64, v5: f64, v6: f64):
    v7 = icmp_sge v4, v0
    branch v7, block3(v5), block2
block2:
    v8 = intrinsic.sqrt(v6)
    v9 = intrinsic.sin(v6)
    v10 = intrinsic.cos(v6)
    v11 = fmul v9, v9
    v12 = fmul v10, v10
    v13 = fadd v11, v12
    v14 = fadd v5, v13
    v15 = fadd v6, v2
    v16 = iconst 1i64
    v17 = iadd v4, v16
    jump block1(v17, v14, v15)
block3(v18: f64):
    return v18
}
"#,
    entry: "math_float",
    expected: || None,
    default_args: || {
        vec![
            Value::int64(10_000),
            Value::float64(0.0),
            Value::float64(0.01),
        ]
    },
};

/// Checked arithmetic intrinsics with overflow detection.
pub(crate) const CHECKED_ARITH: Program = Program {
    name: "checked_arith",
    source: r#"
function @checked_arith(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    v2 = iconst 1i64
    jump block1(v1, v2, v1)
block1(v3: i64, v4: i64, v5: i64):
    v6 = icmp_sge v3, v0
    branch v6, block3(v5), block2
block2:
    v7 = intrinsic.add.overflow(v4, v2)
    v8 = field.get v7, 0
    v9 = field.get v7, 1
    branch v9, block4, block6
block4:
    v10 = iconst 1i64
    jump block5(v10)
block6:
    v18 = iconst 0i64
    jump block5(v18)
block5(v11: i64):
    v12 = iadd v5, v11
    v13 = iconst 1i64
    v14 = iadd v3, v13
    v15 = iconst 127i64
    v16 = srem v8, v15
    jump block1(v14, v16, v12)
block3(v17: i64):
    return v17
}
"#,
    entry: "checked_arith",
    expected: || None,
    default_args: || vec![Value::int64(10_000)],
};
