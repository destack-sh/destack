use super::Program;
use destack_machine::memory::Value;

/// All arithmetic benchmark programs.
pub(crate) const ALL: &[&Program] = &[
    &FIB_RECURSIVE,
    &FIB_ITERATIVE,
    &PRIME_SIEVE,
    &BINARY_INT_MIX,
    &BINARY_FLOAT,
    &COLLATZ,
    &UNARY_OPS,
    &CAST_MIX,
];

/// Recursive fibonacci with exponential call tree.
pub(crate) const FIB_RECURSIVE: Program = Program {
    name: "fib_recursive",
    source: r#"
function @fib(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 2i64
    v2 = icmp_slt v0, v1
    branch v2, block1, block2
block1:
    return v0
block2:
    v3 = iconst 1i64
    v4 = isub v0, v3
    v5 = call @fib(v4)
    v6 = iconst 2i64
    v7 = isub v0, v6
    v8 = call @fib(v7)
    v9 = iadd v5, v8
    return v9
}
"#,
    entry: "fib",
    expected: || Some(Value::int64(55)),
    default_args: || vec![Value::int64(10)],
};

/// Iterative fibonacci using SSA block parameters.
pub(crate) const FIB_ITERATIVE: Program = Program {
    name: "fib_iterative",
    source: r#"
function @fib_iter(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    v2 = iconst 1i64
    jump block1(v1, v1, v2)
block1(v3: i64, v4: i64, v5: i64):
    v6 = icmp_sge v3, v0
    branch v6, block3(v4), block2
block2:
    v7 = iadd v4, v5
    v8 = iconst 1i64
    v9 = iadd v3, v8
    jump block1(v9, v5, v7)
block3(v10: i64):
    return v10
}
"#,
    entry: "fib_iter",
    expected: || Some(Value::int64(55)),
    default_args: || vec![Value::int64(10)],
};

/// Prime counting via trial division.
pub(crate) const PRIME_SIEVE: Program = Program {
    name: "prime_sieve",
    source: r#"
function @count_primes(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 2i64
    v2 = iconst 0i64
    jump block1(v1, v2)
block1(v3: i64, v4: i64):
    v5 = icmp_sgt v3, v0
    branch v5, block6(v4), block2
block2:
    v6 = iconst 2i64
    jump block3(v6)
block3(v7: i64):
    v8 = imul v7, v7
    v9 = icmp_sgt v8, v3
    branch v9, block5, block4
block4:
    v10 = srem v3, v7
    v11 = iconst 0i64
    v12 = icmp_eq v10, v11
    branch v12, block7, block8
block5:
    v13 = iconst 1i64
    v14 = iadd v4, v13
    v15 = iadd v3, v13
    jump block1(v15, v14)
block6(v16: i64):
    return v16
block7:
    v17 = iconst 1i64
    v18 = iadd v3, v17
    jump block1(v18, v4)
block8:
    v19 = iconst 1i64
    v20 = iadd v7, v19
    jump block3(v20)
}
"#,
    entry: "count_primes",
    expected: || Some(Value::int64(25)),
    default_args: || vec![Value::int64(100)],
};

/// Mixed integer operations including add, sub, mul, div, mod, shifts, and bitwise.
pub(crate) const BINARY_INT_MIX: Program = Program {
    name: "binary_int_mix",
    source: r#"
function @binary_int_mix(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    v2 = iconst 12345i64
    jump block1(v1, v2)
block1(v3: i64, v4: i64):
    v5 = icmp_sge v3, v0
    branch v5, block3(v4), block2
block2:
    v6 = iconst 1i64
    v7 = iadd v4, v3
    v8 = isub v7, v6
    v9 = imul v8, v6
    v10 = iconst 7i64
    v11 = sdiv v9, v10
    v12 = iconst 13i64
    v13 = srem v11, v12
    v14 = iconst 3i64
    v15 = ishl v13, v14
    v16 = iconst 1i64
    v17 = sshr v15, v16
    v18 = iconst 255i64
    v19 = band v17, v18
    v20 = iconst 170i64
    v21 = bxor v19, v20
    v22 = iconst 85i64
    v23 = bor v21, v22
    v24 = iadd v3, v6
    jump block1(v24, v23)
block3(v25: i64):
    return v25
}
"#,
    entry: "binary_int_mix",
    expected: || None,
    default_args: || vec![Value::int64(10_000)],
};

/// Float arithmetic with add, sub, mul, and div loop.
pub(crate) const BINARY_FLOAT: Program = Program {
    name: "binary_float",
    source: r#"
function @binary_float(v0: i64, v1: f64, v2: f64, v3: f64) -> f64 {
block0(v0: i64, v1: f64, v2: f64, v3: f64):
    v4 = iconst 0i64
    jump block1(v4, v1)
block1(v5: i64, v6: f64):
    v7 = icmp_sge v5, v0
    branch v7, block3(v6), block2
block2:
    v8 = fadd v6, v2
    v9 = fmul v8, v3
    v10 = fsub v9, v2
    v11 = fdiv v10, v3
    v12 = iconst 1i64
    v13 = iadd v5, v12
    jump block1(v13, v11)
block3(v14: f64):
    return v14
}
"#,
    entry: "binary_float",
    expected: || None,
    default_args: || {
        vec![
            Value::int64(10_000),
            Value::float64(1.0),
            Value::float64(0.5),
            Value::float64(2.0),
        ]
    },
};

/// Collatz sequence with unpredictable branching and varied operations.
pub(crate) const COLLATZ: Program = Program {
    name: "collatz",
    source: r#"
function @collatz_steps(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    jump block1(v0, v1)
block1(v2: i64, v3: i64):
    v4 = iconst 1i64
    v5 = icmp_eq v2, v4
    branch v5, block5(v3), block2
block2:
    v6 = iconst 2i64
    v7 = srem v2, v6
    v8 = iconst 0i64
    v9 = icmp_eq v7, v8
    branch v9, block3, block4
block3:
    v10 = iconst 2i64
    v11 = sdiv v2, v10
    v12 = iconst 1i64
    v13 = iadd v3, v12
    jump block1(v11, v13)
block4:
    v14 = iconst 3i64
    v15 = imul v2, v14
    v16 = iconst 1i64
    v17 = iadd v15, v16
    v18 = iadd v3, v16
    jump block1(v17, v18)
block5(v19: i64):
    return v19
}

function @collatz_sum(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 1i64
    v2 = iconst 0i64
    jump block1(v1, v2)
block1(v3: i64, v4: i64):
    v5 = icmp_sgt v3, v0
    branch v5, block3(v4), block2
block2:
    v6 = call @collatz_steps(v3)
    v7 = iadd v4, v6
    v8 = iconst 1i64
    v9 = iadd v3, v8
    jump block1(v9, v7)
block3(v10: i64):
    return v10
}
"#,
    entry: "collatz_sum",
    expected: || Some(Value::int64(3142)),
    default_args: || vec![Value::int64(100)],
};

/// Unary operations: negation and bitwise not.
pub(crate) const UNARY_OPS: Program = Program {
    name: "unary_ops",
    source: r#"
function @unary_ops(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    v2 = iconst 12345i64
    jump block1(v1, v2)
block1(v3: i64, v4: i64):
    v5 = icmp_sge v3, v0
    branch v5, block3(v4), block2
block2:
    v6 = ineg v4
    v7 = bnot v6
    v8 = ineg v7
    v9 = bnot v8
    v10 = ineg v9
    v11 = bnot v10
    v12 = ineg v11
    v13 = bnot v12
    v14 = iconst 1i64
    v15 = iadd v3, v14
    jump block1(v15, v13)
block3(v16: i64):
    return v16
}
"#,
    entry: "unary_ops",
    expected: || None,
    default_args: || vec![Value::int64(10_000)],
};

/// Cast mix covering integer truncation and float conversions.
pub(crate) const CAST_MIX: Program = Program {
    name: "cast_mix",
    source: r#"
function @cast_mix(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    v2 = iconst 1i64
    jump block1(v1, v2)
block1(v3: i64, v4: i64):
    v5 = icmp_sge v3, v0
    branch v5, block3(v4), block2
block2:
    v6 = trunc v4 -> i32
    v7 = uextend v6 -> i64
    v8 = scvt_to_float v7 -> f64
    v9 = fcvt_to_sint v8 -> i64
    v10 = iadd v9, v2
    v11 = iadd v3, v2
    jump block1(v11, v10)
block3(v12: i64):
    return v12
}
"#,
    entry: "cast_mix",
    expected: || Some(Value::int64(10_001)),
    default_args: || vec![Value::int64(10_000)],
};
