use crate::tests::{assert_execution_completed, assert_execution_yielded, create_machine};
use destack_program::{ProfileOptions, SampleBucket, SampleKey, SampleValue, Value};

/// Profile instructions update explicit counter and sample rows.
#[test]
fn test_profile_records_explicit_sites() {
    let mir = r#"
function main(): int32 {
entry:
    v0: int32 = 42
    v1: int32 = 7
    profile.increment counter(0)
    profile.increment counter(0)
    profile.sample counter(1), v0
    profile.sample counter(1), v1
    return v0
}
"#;
    let mut machine = create_machine(mir);
    let mut profile = machine.empty_profile();
    let output = machine
        .run_function_by_name_profiled("main", &[], &mut profile)
        .expect("execution failed");

    assert_eq!(output, Value::int32(42));
    assert_eq!(profile.counters.len(), 2);
    assert_eq!(profile.counters[0].count, 2);
    assert!(profile.counters[1].is_empty());
    assert_eq!(profile.samples.len(), 2);
    assert!(profile.samples[0].is_empty());
    assert_eq!(profile.samples[1].count, 2);
    assert_eq!(profile.samples[1].buckets.len(), 2);
    assert_eq!(profile.samples[1].buckets[0].key, SampleKey::new(42));
    assert_eq!(profile.samples[1].buckets[0].count, 1);
    assert_eq!(profile.samples[1].buckets[1].key, SampleKey::new(7));
    assert_eq!(profile.samples[1].buckets[1].count, 1);

    let program = machine.machine.program();
    let sample_site = &program.sites().samples(program.sections())[0];
    let sample_value = program
        .sample_value(sample_site, profile.samples[1].buckets[0].key)
        .expect("sample key should decode");

    assert_eq!(
        sample_value,
        SampleValue::Int {
            value: 42,
            width: 32
        },
    );
    assert_eq!(std::mem::size_of::<SampleKey>(), 8);
    assert_eq!(std::mem::size_of::<SampleBucket>(), 16);
}

/// Function-local MIR counters project to distinct executable counters.
#[test]
fn test_profile_projects_function_local_counters() {
    let mir = r#"
function first(): int32 {
entry:
    v0: int32 = 1
    profile.increment counter(0)
    return v0
}

function second(): int32 {
entry:
    v0: int32 = 2
    profile.increment counter(0)
    return v0
}

function main(): int32 {
entry:
    v0: int32 = call first(): () => int32
    v1: int32 = call second(): () => int32
    v2: int32 = int.add v0, v1
    return v2
}
"#;
    let mut machine = create_machine(mir);
    let mut profile = machine.empty_profile();
    let output = machine
        .run_function_by_name_profiled("main", &[], &mut profile)
        .expect("execution failed");

    assert_eq!(output, Value::int32(3));
    assert_eq!(profile.counters.len(), 2);
    assert_eq!(
        profile
            .counters
            .iter()
            .map(|counter| counter.count)
            .collect::<Vec<_>>(),
        vec![1, 1],
    );
}

/// Sample profiles respect their configured exact bucket limit.
#[test]
fn test_profile_limits_sample_buckets() {
    let mir = r#"
function main(v0: int32): int32 {
entry(v0: int32):
    profile.sample counter(0), v0
    return v0
}
"#;
    let mut machine = create_machine(mir);
    let mut profile = machine.empty_profile_with_options(ProfileOptions {
        sample_bucket_limit: 1,
    });

    let first = machine
        .run_function_by_name_profiled("main", &[Value::int32(1)], &mut profile)
        .expect("first execution failed");
    let second = machine
        .run_function_by_name_profiled("main", &[Value::int32(2)], &mut profile)
        .expect("second execution failed");

    assert_eq!(first, Value::int32(1));
    assert_eq!(second, Value::int32(2));
    assert_eq!(profile.samples.len(), 1);
    assert_eq!(profile.samples[0].count, 2);
    assert_eq!(profile.samples[0].buckets.len(), 1);
    assert_eq!(profile.samples[0].buckets[0].key, SampleKey::new(1));
    assert_eq!(profile.samples[0].buckets[0].count, 1);
    assert_eq!(profile.samples[0].overflow, 1);
}

/// Profile execution counts ordinary call sites.
#[test]
fn test_profile_records_call_sites() {
    let mir = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function main(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = call callee(v0): (int32) => int32
    return v1
}
"#;
    let mut machine = create_machine(mir);
    let mut profile = machine.empty_profile();
    let output = machine
        .run_function_by_name_profiled("main", &[Value::int32(21)], &mut profile)
        .expect("execution failed");

    assert_eq!(output, Value::int32(42));
    assert_eq!(profile.calls.len(), 1);
    assert_eq!(profile.calls[0].count, 1);
}

/// Profile execution counts ordinary allocation sites.
#[test]
fn test_profile_records_allocation_sites() {
    let mir = r#"
function main(): ref<int32, managed, readonly> {
entry:
    v0: ref<int32, managed, readonly> = new.zeroed int32
    return v0
}
"#;
    let mut machine = create_machine(mir);
    let mut profile = machine.empty_profile();
    let output = machine
        .run_function_by_name_profiled("main", &[], &mut profile)
        .expect("execution failed");

    assert!(matches!(output, Value::HeapReference(_)));
    assert_eq!(profile.allocations.len(), 1);
    assert_eq!(profile.allocations[0].count, 1);
    assert_eq!(profile.allocations[0].bytes, 4);
}

/// Profile execution counts taken control-flow edges.
#[test]
fn test_profile_records_edge_sites() {
    let mir = r#"
function main(v0: boolean): int32 {
entry(v0: boolean):
    branch v0, b1, b2

b1:
    v1: int32 = 1
    return v1

b2:
    v2: int32 = 2
    return v2
}
"#;
    let mut machine = create_machine(mir);
    let mut profile = machine.empty_profile();
    let output = machine
        .run_function_by_name_profiled("main", &[Value::bool(false)], &mut profile)
        .expect("execution failed");

    assert_eq!(output, Value::int32(2));
    assert_eq!(profile.edges.len(), 2);
    assert_eq!(profile.edges.iter().map(|edge| edge.count).sum::<u64>(), 1);
    assert_eq!(
        profile.edges.iter().filter(|edge| edge.count == 1).count(),
        1
    );
}

/// Profile execution counts continuation captures and resumes.
#[test]
fn test_profile_records_continuation_sites() {
    let mir = r#"
function yieldOnce(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 5
    yield v1 => b1(v0)

b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    return v4
}
"#;
    let mut machine = create_machine(mir);
    let mut profile = machine.empty_profile();
    let (continuation, value) =
        assert_execution_yielded(machine.run_function_by_name_yielding_profiled(
            "yieldOnce",
            &[Value::int32(7)],
            &mut profile,
        ));

    assert_eq!(value, Value::int32(5));
    assert_eq!(profile.continuations.len(), 1);
    assert_eq!(profile.continuations[0].captured, 1);
    assert_eq!(profile.continuations[0].resumed, 0);

    let output = assert_execution_completed(machine.resume_profiled(
        continuation,
        Value::int32(11),
        &mut profile,
    ));

    assert_eq!(output, Value::int32(18));
    assert_eq!(profile.continuations[0].captured, 1);
    assert_eq!(profile.continuations[0].resumed, 1);
}
