use std::slice;

use crate::Value;
use crate::tests::{create_isolate, run_mir_expect};

/// Function environment state is preserved across repeated calls in one isolate.
#[test]
fn test_environment_multiple_calls_same_isolate() {
    let mir = r#"
type Env {
    count: ref<int32, managed>;
    base: int32;
}

@environment(ref<Env, managed>)
function step(): int32 {
b0:
    v0: ref<Env, managed> = callable.environment
    v1: ref<ref<int32, managed>, managed> = field.address v0, 0
    v2: ref<int32, managed> = load v1
    v3: int32 = load v2
    v4: int32 = 1int32
    v5: int32 = int.add v3, v4
    store v2, v5
    v6: ref<int32, managed> = field.address v0, 1
    v7: int32 = load v6
    v8: int32 = int.add v5, v7
    return v8
}

function makeEnv(): ref<Env, managed> {
b0:
    v0: ref<int32, managed> = new int32
    v1: int32 = 0int32
    store v0, v1
    v2: ref<Env, managed> = new Env
    v3: ref<ref<int32, managed>, managed> = field.address v2, 0
    store v3, v0
    v4: ref<int32, managed> = field.address v2, 1
    v5: int32 = 10int32
    store v4, v5
    return v2
}

function callOnce(v0: ref<Env, managed>): int32 {
b0(v0: ref<Env, managed>):
    v1: () => int32 = callable.bind step, v0
    v2: int32 = call.indirect v1(): () -> int32
    return v2
}"#;

    let mut isolate = create_isolate(mir);
    let env = isolate
        .run_function_by_name("makeEnv", &[])
        .expect("execution failed");
    let env = env.value;

    let first = isolate
        .run_function_by_name("callOnce", slice::from_ref(&env))
        .expect("execution failed");
    let first = first.value;
    let second = isolate
        .run_function_by_name("callOnce", &[env])
        .expect("execution failed");
    let second = second.value;

    assert_eq!(first, Value::int32(11));
    assert_eq!(second, Value::int32(12));
}

/// call.indirect passes the callable environment for callable.environment.
#[test]
fn test_call_indirect_environment() {
    let mir = r#"
@environment(ref<int32, raw, readonly, space(stack)>)
function readEnv(): int32 {
b0:
    v0: ref<int32, raw, readonly, space(stack)> = callable.environment
    v1: int32 = load v0
    return v1
}

function caller(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 41int32
    store v0, v1
    v2: ref<int32, raw, readonly, space(stack)> = cast.bit v0 -> ref<int32, raw, readonly, space(stack)>
    v3: () => int32 = callable.bind readEnv, v2
    v4: int32 = call.indirect v3(): () -> int32
    return v4
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(41));
}

/// Tail call indirect forwards the callable environment.
#[test]
fn test_tailcall_indirect_environment() {
    let mir = r#"
@environment(ref<int32, managed>)
function readEnv(): int32 {
b0:
    v0: ref<int32, managed> = callable.environment
    v1: int32 = load v0
    return v1
}

function caller(): int32 {
b0:
    v0: ref<int32, managed> = new int32
    v1: int32 = 99int32
    store v0, v1
    v2: () => int32 = callable.bind readEnv, v0
    tailCall.indirect v2(): () -> int32
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(99));
}

/// Managed callable environments hold by-reference capture cells.
#[test]
fn test_environment_heap_reference_cell() {
    let mir = r#"
type Env { cell: ref<int32, managed> }

@environment(ref<Env, managed>)
function increment(): int32 {
b0:
    v0: ref<Env, managed> = callable.environment
    v1: ref<ref<int32, managed>, managed> = field.address v0, 0
    v2: ref<int32, managed> = load v1
    v3: int32 = load v2
    v4: int32 = 1int32
    v5: int32 = int.add v3, v4
    store v2, v5
    return v5
}

function caller(): int32 {
b0:
    v0: ref<int32, managed> = new int32
    v1: int32 = 0int32
    store v0, v1
    v2: ref<Env, managed> = new Env
    v3: ref<ref<int32, managed>, managed> = field.address v2, 0
    store v3, v0
    v4: () => int32 = callable.bind increment, v2
    v5: int32 = call.indirect v4(): () -> int32
    v6: int32 = call.indirect v4(): () -> int32
    return v6
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(2));
}

/// Managed callable environments support by-value fields.
#[test]
fn test_environment_by_value_field() {
    let mir = r#"
type Env { value: int32 }
type Reader = () => int32;
@environment(ref<Env, managed>)
function readEnv(): int32 {
b0:
    v0: ref<Env, managed> = callable.environment
    v1: ref<int32, managed> = field.address v0, 0
    v2: int32 = load v1
    v3: int32 = 2int32
    v4: int32 = int.add v2, v3
    return v4
}

function caller(): int32 {
b0:
    v0: ref<Env, managed> = new Env
    v1: ref<int32, managed> = field.address v0, 0
    v2: int32 = 40int32
    store v1, v2
    v3: () => int32 = callable.bind readEnv, v0
    v4: int32 = call.indirect v3(): () -> int32
    return v4
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(42));
}

/// call.indirect selects the environment provided at the callsite.
#[test]
fn test_environment_selects_callsite_environment() {
    let mir = r#"
type Env { value: int32 }

@environment(ref<Env, managed>)
function readEnv(): int32 {
b0:
    v0: ref<Env, managed> = callable.environment
    v1: ref<int32, managed> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

function caller(): int32 {
b0:
    v0: ref<Env, managed> = new Env
    v1: ref<Env, managed> = new Env
    v2: ref<int32, managed> = field.address v0, 0
    v3: ref<int32, managed> = field.address v1, 0
    v4: int32 = 10int32
    v5: int32 = 20int32
    store v2, v4
    store v3, v5
    v6: () => int32 = callable.bind readEnv, v0
    v7: () => int32 = callable.bind readEnv, v1
    v8: int32 = call.indirect v6(): () -> int32
    v9: int32 = call.indirect v7(): () -> int32
    v10: int32 = int.add v8, v9
    return v10
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(30));
}

/// call.indirect can swap callable environments within a single isolate.
#[test]
fn test_environment_switches_in_isolate() {
    let mir = r#"
type Env { value: int32 }

@environment(ref<Env, managed>)
function readEnv(): int32 {
b0:
    v0: ref<Env, managed> = callable.environment
    v1: ref<int32, managed> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

function makeEnv(v0: int32): ref<Env, managed> {
b0(v0: int32):
    v1: ref<Env, managed> = new Env
    v2: ref<int32, managed> = field.address v1, 0
    store v2, v0
    return v1
}

function callOnce(v0: ref<Env, managed>): int32 {
b0(v0: ref<Env, managed>):
    v1: () => int32 = callable.bind readEnv, v0
    v2: int32 = call.indirect v1(): () -> int32
    return v2
}"#;

    let mut isolate = create_isolate(mir);
    let env_a = isolate
        .run_function_by_name("makeEnv", &[Value::int32(7)])
        .expect("execution failed");
    let env_a = env_a.value;
    let env_b = isolate
        .run_function_by_name("makeEnv", &[Value::int32(13)])
        .expect("execution failed");
    let env_b = env_b.value;

    let first = isolate
        .run_function_by_name("callOnce", slice::from_ref(&env_a))
        .expect("execution failed");
    let first = first.value;
    let second = isolate
        .run_function_by_name("callOnce", &[env_b])
        .expect("execution failed");
    let second = second.value;
    let third = isolate
        .run_function_by_name("callOnce", &[env_a])
        .expect("execution failed");
    let third = third.value;

    assert_eq!(first, Value::int32(7));
    assert_eq!(second, Value::int32(13));
    assert_eq!(third, Value::int32(7));
}

/// Callable values can be stored in managed structs and invoked with their environment.
#[test]
fn test_environment_loaded_from_struct() {
    let mir = r#"
type Env {
    value: int32;
}
type Holder {
    fun: () => int32;
}
@environment(ref<Env, managed>)
function readEnv(): int32 {
b0:
    v0: ref<Env, managed> = callable.environment
    v1: ref<int32, managed> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

function makeEnv(v0: int32): ref<Env, managed> {
b0(v0: int32):
    v1: ref<Env, managed> = new Env
    v2: ref<int32, managed> = field.address v1, 0
    store v2, v0
    return v1
}

function caller(v0: int32): int32 {
b0(v0: int32):
    v1: ref<Env, managed> = call makeEnv(v0): (int32) -> ref<Env, managed>
    v2: ref<Holder, managed> = new Holder
    v3: ref<() => int32, managed> = field.address v2, 0
    v4: () => int32 = callable.bind readEnv, v1
    store v3, v4
    v5: () => int32 = load v3
    v6: int32 = call.indirect v5(): () -> int32
    return v6
}"#;

    run_mir_expect(mir, "caller", &[Value::int32(42)], Value::int32(42));
}

/// Nested callable environments can invoke inner callables via stored environments.
#[test]
fn test_environment_chain_calls_inner() {
    let mir = r#"
type InnerEnv { value: int32 }
type OuterEnv { fun: () => int32 }

@environment(ref<InnerEnv, managed>)
function inner(): int32 {
b0:
    v0: ref<InnerEnv, managed> = callable.environment
    v1: ref<int32, managed> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

@environment(ref<OuterEnv, managed>)
function outer(): int32 {
b0:
    v0: ref<OuterEnv, managed> = callable.environment
    v1: ref<() => int32, managed> = field.address v0, 0
    v2: () => int32 = load v1
    v3: int32 = call.indirect v2(): () -> int32
    return v3
}

function makeInner(v0: int32): ref<InnerEnv, managed> {
b0(v0: int32):
    v1: ref<InnerEnv, managed> = new InnerEnv
    v2: ref<int32, managed> = field.address v1, 0
    store v2, v0
    return v1
}

function makeOuter(v0: int32): ref<OuterEnv, managed> {
b0(v0: int32):
    v1: ref<InnerEnv, managed> = call makeInner(v0): (int32) -> ref<InnerEnv, managed>
    v2: ref<OuterEnv, managed> = new OuterEnv
    v3: ref<() => int32, managed> = field.address v2, 0
    v4: () => int32 = callable.bind inner, v1
    store v3, v4
    return v2
}

function caller(v0: int32): int32 {
b0(v0: int32):
    v1: ref<OuterEnv, managed> = call makeOuter(v0): (int32) -> ref<OuterEnv, managed>
    v2: () => int32 = callable.bind outer, v1
    v3: int32 = call.indirect v2(): () -> int32
    return v3
}"#;

    run_mir_expect(mir, "caller", &[Value::int32(55)], Value::int32(55));
}

/// Function pointers without env can be stored and called indirectly.
#[test]
fn test_function_ptr_loaded_from_struct() {
    let mir = r#"
type Holder { fun: (int32) -> int32 }

function double(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}

function caller(v0: int32): int32 {
b0(v0: int32):
    v1: ref<Holder, managed> = new Holder
    v2: ref<(int32) -> int32, managed> = field.address v1, 0
    v3: (int32) -> int32 = function.address double
    store v2, v3
    v4: (int32) -> int32 = load v2
    v5: int32 = call.indirect v4(v0): (int32) -> int32
    return v5
}"#;

    run_mir_expect(mir, "caller", &[Value::int32(21)], Value::int32(42));
}

/// Raw callable environments can carry stack allocated structs.
#[test]
fn test_environment_raw_struct_on_stack() {
    let mir = r#"
type Env { value: int32, extra: int32 }

@environment(ref<Env, raw, readonly, space(stack)>)
function readEnv(): int32 {
b0:
    v0: ref<Env, raw, readonly, space(stack)> = callable.environment
    v1: ref<int32, raw, readonly, space(stack)> = field.address v0, 0
    v2: int32 = load v1
    v3: ref<int32, raw, readonly, space(stack)> = field.address v0, 1
    v4: int32 = load v3
    v5: int32 = int.add v2, v4
    return v5
}

function caller(): int32 {
b0:
    v0: ref<Env, raw, space(stack)> = stack.alloc Env
    v1: ref<int32, raw, readonly, space(stack)> = field.address v0, 0
    v2: ref<int32, raw, readonly, space(stack)> = field.address v0, 1
    v3: int32 = 20int32
    v4: int32 = 22int32
    store v1, v3
    store v2, v4
    v5: ref<Env, raw, readonly, space(stack)> = cast.bit v0 -> ref<Env, raw, readonly, space(stack)>
    v6: () => int32 = callable.bind readEnv, v5
    v7: int32 = call.indirect v6(): () -> int32
    return v7
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(42));
}

/// Callable values can be stored in arrays and invoked later.
#[test]
fn test_environment_loaded_from_array() {
    let mir = r#"
type Env { value: int32 }
type Reader = () => int32;
@environment(ref<Env, managed>)
function readEnv(): int32 {
b0:
    v0: ref<Env, managed> = callable.environment
    v1: ref<int32, managed> = field.address v0, 0
    v2: int32 = load v1
    return v2
}

function makeEnv(v0: int32): ref<Env, managed> {
b0(v0: int32):
    v1: ref<Env, managed> = new Env
    v2: ref<int32, managed> = field.address v1, 0
    store v2, v0
    return v1
}

function caller(v0: int32): int32 {
b0(v0: int32):
    v1: ref<Env, managed> = call makeEnv(v0): (int32) -> ref<Env, managed>
    v2: Reader = callable.bind readEnv, v1
    v3: Reader[1] = array Reader[1] (v2)
    v4: int32 = 0int32
    v5: Reader = element.get v3, v4
    v6: int32 = call.indirect v5(): () -> int32
    return v6
}"#;

    run_mir_expect(mir, "caller", &[Value::int32(8)], Value::int32(8));
}
