use super::Program;
use destack_machine::memory::Value;

/// All memory benchmark programs.
pub(crate) const ALL: &[&Program] = &[
    &ALLOC_SINGLE,
    &ALLOC_BURST,
    &LOAD_STORE,
    &LINKED_WALK,
    &FIELD_ACCESS,
];

/// Single allocation per iteration.
pub(crate) const ALLOC_SINGLE: Program = Program {
    name: "alloc_single",
    source: r#"
function @alloc_single(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    jump block1(v1)
block1(v2: i64):
    v3 = icmp_sge v2, v0
    branch v3, block3, block2
block2:
    v4 = managed.alloc i64
    v5 = iconst 1i64
    v6 = iadd v2, v5
    jump block1(v6)
block3:
    return v0
}
"#,
    entry: "alloc_single",
    expected: || Some(Value::int64(1000)),
    default_args: || vec![Value::int64(1000)],
};

/// Burst allocations with ten allocations per iteration.
pub(crate) const ALLOC_BURST: Program = Program {
    name: "alloc_burst",
    source: r#"
function @alloc_burst(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    jump block1(v1)
block1(v2: i64):
    v3 = icmp_sge v2, v0
    branch v3, block3, block2
block2:
    v4 = managed.alloc i64
    v5 = managed.alloc i64
    v6 = managed.alloc i64
    v7 = managed.alloc i64
    v8 = managed.alloc i64
    v9 = managed.alloc i64
    v10 = managed.alloc i64
    v11 = managed.alloc i64
    v12 = managed.alloc i64
    v13 = managed.alloc i64
    v14 = iconst 1i64
    v15 = iadd v2, v14
    jump block1(v15)
block3:
    return v0
}
"#,
    entry: "alloc_burst",
    expected: || Some(Value::int64(1000)),
    default_args: || vec![Value::int64(1000)],
};

/// Load and store loop on heap cells.
pub(crate) const LOAD_STORE: Program = Program {
    name: "load_store",
    source: r#"
function @load_store(v0: i64) -> i64 {
block0(v0: i64):
    v1 = managed.alloc i64
    v2 = managed.alloc i64
    v3 = iconst 0i64
    v4 = iconst 1i64
    store v1, v3
    store v2, v4
    jump block1(v3)
block1(v5: i64):
    v6 = icmp_sge v5, v0
    branch v6, block3, block2
block2:
    v7 = load v1
    v8 = load v2
    v9 = iadd v7, v8
    store v1, v8
    store v2, v9
    v10 = iconst 1i64
    v11 = iadd v5, v10
    jump block1(v11)
block3:
    v12 = load v2
    return v12
}
"#,
    entry: "load_store",
    expected: || None,
    default_args: || vec![Value::int64(10_000)],
};

/// Walk a linked list simulated via recursive calls.
pub(crate) const LINKED_WALK: Program = Program {
    name: "linked_walk",
    source: r#"
function @build_list(v0: i64, v1: ref<i64>) -> ref<i64> {
block0(v0: i64, v1: ref<i64>):
    v2 = iconst 0i64
    v3 = icmp_eq v0, v2
    branch v3, block2, block1
block1:
    v4 = managed.alloc i64
    store v4, v0
    v5 = iconst 1i64
    v6 = isub v0, v5
    v7 = call @build_list(v6, v4)
    return v7
block2:
    return v1
}

function @walk_list(v0: ref<i64>, v1: i64) -> i64 {
block0(v0: ref<i64>, v1: i64):
    v2 = load v0
    v3 = iconst 0i64
    v4 = icmp_eq v2, v3
    branch v4, block2, block1
block1:
    v5 = iadd v1, v2
    v6 = iconst 1i64
    v7 = isub v2, v6
    v8 = managed.alloc i64
    store v8, v7
    v9 = call @walk_list(v8, v5)
    return v9
block2:
    return v1
}

function @linked_walk(v0: i64) -> i64 {
block0(v0: i64):
    v1 = managed.alloc i64
    v2 = iconst 0i64
    store v1, v2
    v3 = call @build_list(v0, v1)
    v4 = call @walk_list(v3, v2)
    return v4
}
"#,
    entry: "linked_walk",
    // algorithm is flawed, doesn't actually traverse linked list
    expected: || None,
    default_args: || vec![Value::int64(100)],
};

/// Field access on composite values (tuples from overflow intrinsics).
pub(crate) const FIELD_ACCESS: Program = Program {
    name: "field_access",
    source: r#"
function @field_access(v0: i64) -> i64 {
block0(v0: i64):
    v1 = iconst 0i64
    v2 = iconst 1i64
    jump block1(v1, v2)
block1(v3: i64, v4: i64):
    v5 = icmp_sge v3, v0
    branch v5, block3(v4), block2
block2:
    v6 = intrinsic.add.overflow(v4, v4)
    v7 = field.get v6, 0
    v8 = intrinsic.mul.overflow(v7, v4)
    v9 = field.get v8, 0
    v10 = intrinsic.sub.overflow(v9, v4)
    v11 = field.get v10, 0
    v12 = iconst 255i64
    v13 = band v11, v12
    v14 = iconst 1i64
    v15 = iadd v3, v14
    jump block1(v15, v13)
block3(v16: i64):
    return v16
}
"#,
    entry: "field_access",
    expected: || None,
    default_args: || vec![Value::int64(10_000)],
};
