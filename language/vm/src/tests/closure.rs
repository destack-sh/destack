use crate::tests::{create_isolate, run_mir_expect};
use destack_heap::Value;

/// Function environment state is preserved across repeated calls in one isolate.
#[test]
fn test_environment_multiple_calls_same_isolate() {
    let mir = r#"
type @Env = { count: ref<managed i32>, base: i32 }

#[environment(ref<managed @Env>)]
function @step() -> i32 {
block0:
    v0: ref<managed @Env> = function.environment
    v1: ref<managed ref<managed i32>> = field.addr v0, 0
    v2: ref<managed i32> = load v1
    v3: i32 = load v2
    v4: i32 = iconst 1i32
    v5: i32 = iadd v3, v4
    store v2, v5
    v6: ref<managed i32> = field.addr v0, 1
    v7: i32 = load v6
    v8: i32 = iadd v5, v7
    return v8
}

function @make_env() -> ref<managed @Env> {
block0:
    v0: ref<managed i32> = managed.alloc i32
    v1: i32 = iconst 0i32
    store v0, v1
    v2: ref<managed @Env> = managed.alloc @Env
    v3: ref<managed ref<managed i32>> = field.addr v2, 0
    store v3, v0
    v4: ref<managed i32> = field.addr v2, 1
    v5: i32 = iconst 10i32
    store v4, v5
    return v2
}

function @call_once(v0: ref<managed @Env>) -> i32 {
block0(v0: ref<managed @Env>):
    v1: fnvalue<fn() -> i32> = function.value @step, v0
    v2: i32 = call.indirect v1() -> fnvalue<fn() -> i32>
    return v2
}"#;

    let mut isolate = create_isolate(mir);
    let env = isolate
        .run_function_by_name("make_env", &[])
        .expect("execution failed")
        .value;

    let first = isolate
        .run_function_by_name("call_once", &[env])
        .expect("execution failed")
        .value;
    let second = isolate
        .run_function_by_name("call_once", &[env])
        .expect("execution failed")
        .value;

    assert_eq!(first, Value::int32(11));
    assert_eq!(second, Value::int32(12));
}

/// call.indirect passes the function environment for function.environment.
#[test]
fn test_call_indirect_environment() {
    let mir = r#"
#[environment(ref<raw addrspace(stack) readonly i32>)]
function @read_env() -> i32 {
block0:
    v0: ref<raw addrspace(stack) readonly i32> = function.environment
    v1: i32 = load v0
    return v1
}

function @caller() -> i32 {
block0:
    v0: ref<raw addrspace(stack) i32> = stack.alloc i32
    v1: i32 = iconst 41
    store v0, v1
    v2: ref<raw addrspace(stack) readonly i32> = bitcast v0 -> ref<raw addrspace(stack) readonly i32>
    v3: fnvalue<fn() -> i32> = function.value @read_env, v2
    v4: i32 = call.indirect v3() -> fnvalue<fn() -> i32>
    return v4
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(41));
}

/// Tail call indirect forwards the function environment.
#[test]
fn test_tailcall_indirect_environment() {
    let mir = r#"
#[environment(ref<managed i32>)]
function @read_env() -> i32 {
block0:
    v0: ref<managed i32> = function.environment
    v1: i32 = load v0
    return v1
}

function @caller() -> i32 {
block0:
    v0: ref<managed i32> = managed.alloc i32
    v1: i32 = iconst 99i32
    store v0, v1
    v2: fnvalue<fn() -> i32> = function.value @read_env, v0
    tailcall.indirect v2() -> fnvalue<fn() -> i32>
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(99));
}

/// Managed function environments hold by-reference capture cells.
#[test]
fn test_environment_managed_reference_cell() {
    let mir = r#"
type @Env = { cell: ref<managed i32> }

#[environment(ref<managed @Env>)]
function @increment() -> i32 {
block0:
    v0: ref<managed @Env> = function.environment
    v1: ref<managed ref<managed i32>> = field.addr v0, 0
    v2: ref<managed i32> = load v1
    v3: i32 = load v2
    v4: i32 = iconst 1i32
    v5: i32 = iadd v3, v4
    store v2, v5
    return v5
}

function @caller() -> i32 {
block0:
    v0: ref<managed i32> = managed.alloc i32
    v1: i32 = iconst 0i32
    store v0, v1
    v2: ref<managed @Env> = managed.alloc @Env
    v3: ref<managed ref<managed i32>> = field.addr v2, 0
    store v3, v0
    v4: fnvalue<fn() -> i32> = function.value @increment, v2
    v5: i32 = call.indirect v4() -> fnvalue<fn() -> i32>
    v6: i32 = call.indirect v4() -> fnvalue<fn() -> i32>
    return v6
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(2));
}

/// Managed function environments support by-value fields.
#[test]
fn test_environment_by_value_field() {
    let mir = r#"
type @Env = { value: i32 }

#[environment(ref<managed @Env>)]
function @read_env() -> i32 {
block0:
    v0: ref<managed @Env> = function.environment
    v1: ref<managed i32> = field.addr v0, 0
    v2: i32 = load v1
    v3: i32 = iconst 2i32
    v4: i32 = iadd v2, v3
    return v4
}

function @caller() -> i32 {
block0:
    v0: ref<managed @Env> = managed.alloc @Env
    v1: ref<managed i32> = field.addr v0, 0
    v2: i32 = iconst 40i32
    store v1, v2
    v3: fnvalue<fn() -> i32> = function.value @read_env, v0
    v4: i32 = call.indirect v3() -> fnvalue<fn() -> i32>
    return v4
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(42));
}

/// call.indirect selects the environment provided at the callsite.
#[test]
fn test_environment_selects_callsite_environment() {
    let mir = r#"
type @Env = { value: i32 }

#[environment(ref<managed @Env>)]
function @read_env() -> i32 {
block0:
    v0: ref<managed @Env> = function.environment
    v1: ref<managed i32> = field.addr v0, 0
    v2: i32 = load v1
    return v2
}

function @caller() -> i32 {
block0:
    v0: ref<managed @Env> = managed.alloc @Env
    v1: ref<managed @Env> = managed.alloc @Env
    v2: ref<managed i32> = field.addr v0, 0
    v3: ref<managed i32> = field.addr v1, 0
    v4: i32 = iconst 10i32
    v5: i32 = iconst 20i32
    store v2, v4
    store v3, v5
    v6: fnvalue<fn() -> i32> = function.value @read_env, v0
    v7: fnvalue<fn() -> i32> = function.value @read_env, v1
    v8: i32 = call.indirect v6() -> fnvalue<fn() -> i32>
    v9: i32 = call.indirect v7() -> fnvalue<fn() -> i32>
    v10: i32 = iadd v8, v9
    return v10
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(30));
}

/// call.indirect can swap function environments within a single isolate.
#[test]
fn test_environment_switches_in_isolate() {
    let mir = r#"
type @Env = { value: i32 }

#[environment(ref<managed @Env>)]
function @read_env() -> i32 {
block0:
    v0: ref<managed @Env> = function.environment
    v1: ref<managed i32> = field.addr v0, 0
    v2: i32 = load v1
    return v2
}

function @make_env(v0: i32) -> ref<managed @Env> {
block0(v0: i32):
    v1: ref<managed @Env> = managed.alloc @Env
    v2: ref<managed i32> = field.addr v1, 0
    store v2, v0
    return v1
}

function @call_once(v0: ref<managed @Env>) -> i32 {
block0(v0: ref<managed @Env>):
    v1: fnvalue<fn() -> i32> = function.value @read_env, v0
    v2: i32 = call.indirect v1() -> fnvalue<fn() -> i32>
    return v2
}"#;

    let mut isolate = create_isolate(mir);
    let env_a = isolate
        .run_function_by_name("make_env", &[Value::int32(7)])
        .expect("execution failed")
        .value;
    let env_b = isolate
        .run_function_by_name("make_env", &[Value::int32(13)])
        .expect("execution failed")
        .value;

    let first = isolate
        .run_function_by_name("call_once", &[env_a])
        .expect("execution failed")
        .value;
    let second = isolate
        .run_function_by_name("call_once", &[env_b])
        .expect("execution failed")
        .value;
    let third = isolate
        .run_function_by_name("call_once", &[env_a])
        .expect("execution failed")
        .value;

    assert_eq!(first, Value::int32(7));
    assert_eq!(second, Value::int32(13));
    assert_eq!(third, Value::int32(7));
}

/// Closure values can be stored in aggregates and invoked with their env.
#[test]
fn test_environment_loaded_from_struct() {
    let mir = r#"
type @Env = { value: i32 }
type @Holder = { fun: fnvalue<fn() -> i32> }

#[environment(ref<managed @Env>)]
function @read_env() -> i32 {
block0:
    v0: ref<managed @Env> = function.environment
    v1: ref<managed i32> = field.addr v0, 0
    v2: i32 = load v1
    return v2
}

function @make_env(v0: i32) -> ref<managed @Env> {
block0(v0: i32):
    v1: ref<managed @Env> = managed.alloc @Env
    v2: ref<managed i32> = field.addr v1, 0
    store v2, v0
    return v1
}

function @caller(v0: i32) -> i32 {
block0(v0: i32):
    v1: ref<managed @Env> = call @make_env(v0)
    v2: ref<managed @Holder> = managed.alloc @Holder
    v3: ref<managed fnvalue<fn() -> i32>> = field.addr v2, 0
    v4: fnvalue<fn() -> i32> = function.value @read_env, v1
    store v3, v4
    v5: fnvalue<fn() -> i32> = load v3
    v6: i32 = call.indirect v5() -> fnvalue<fn() -> i32>
    return v6
}"#;

    run_mir_expect(mir, "caller", &[Value::int32(42)], Value::int32(42));
}

/// Nested function environments can invoke inner closures via stored environments.
#[test]
fn test_environment_chain_calls_inner() {
    let mir = r#"
type @InnerEnv = { value: i32 }
type @OuterEnv = { fun: fnvalue<fn() -> i32> }

#[environment(ref<managed @InnerEnv>)]
function @inner() -> i32 {
block0:
    v0: ref<managed @InnerEnv> = function.environment
    v1: ref<managed i32> = field.addr v0, 0
    v2: i32 = load v1
    return v2
}

#[environment(ref<managed @OuterEnv>)]
function @outer() -> i32 {
block0:
    v0: ref<managed @OuterEnv> = function.environment
    v1: ref<managed fnvalue<fn() -> i32>> = field.addr v0, 0
    v2: fnvalue<fn() -> i32> = load v1
    v3: i32 = call.indirect v2() -> fnvalue<fn() -> i32>
    return v3
}

function @make_inner(v0: i32) -> ref<managed @InnerEnv> {
block0(v0: i32):
    v1: ref<managed @InnerEnv> = managed.alloc @InnerEnv
    v2: ref<managed i32> = field.addr v1, 0
    store v2, v0
    return v1
}

function @make_outer(v0: i32) -> ref<managed @OuterEnv> {
block0(v0: i32):
    v1: ref<managed @InnerEnv> = call @make_inner(v0)
    v2: ref<managed @OuterEnv> = managed.alloc @OuterEnv
    v3: ref<managed fnvalue<fn() -> i32>> = field.addr v2, 0
    v4: fnvalue<fn() -> i32> = function.value @inner, v1
    store v3, v4
    return v2
}

function @caller(v0: i32) -> i32 {
block0(v0: i32):
    v1: ref<managed @OuterEnv> = call @make_outer(v0)
    v2: fnvalue<fn() -> i32> = function.value @outer, v1
    v3: i32 = call.indirect v2() -> fnvalue<fn() -> i32>
    return v3
}"#;

    run_mir_expect(mir, "caller", &[Value::int32(55)], Value::int32(55));
}

/// Function pointers without env can be stored and called indirectly.
#[test]
fn test_function_ptr_loaded_from_struct() {
    let mir = r#"
type @Holder = { fun: fn(i32) -> i32 }

function @double(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iadd v0, v0
    return v1
}

function @caller(v0: i32) -> i32 {
block0(v0: i32):
    v1: ref<managed @Holder> = managed.alloc @Holder
    v2: ref<managed fn(i32) -> i32> = field.addr v1, 0
    v3: fn(i32) -> i32 = function.addr @double
    store v2, v3
    v4: fn(i32) -> i32 = load v2
    v5: i32 = call.indirect v4(v0) -> fn(i32) -> i32
    return v5
}"#;

    run_mir_expect(mir, "caller", &[Value::int32(21)], Value::int32(42));
}

/// Raw function environments can carry aggregates in stack address space.
#[test]
fn test_environment_raw_struct_on_stack() {
    let mir = r#"
type @Env = { value: i32, extra: i32 }

#[environment(ref<raw addrspace(stack) readonly @Env>)]
function @read_env() -> i32 {
block0:
    v0: ref<raw addrspace(stack) readonly @Env> = function.environment
    v1: ref<raw addrspace(stack) readonly i32> = field.addr v0, 0
    v2: i32 = load v1
    v3: ref<raw addrspace(stack) readonly i32> = field.addr v0, 1
    v4: i32 = load v3
    v5: i32 = iadd v2, v4
    return v5
}

function @caller() -> i32 {
block0:
    v0: ref<raw addrspace(stack) @Env> = stack.alloc @Env
    v1: ref<raw addrspace(stack) readonly i32> = field.addr v0, 0
    v2: ref<raw addrspace(stack) readonly i32> = field.addr v0, 1
    v3: i32 = iconst 20i32
    v4: i32 = iconst 22i32
    store v1, v3
    store v2, v4
    v5: ref<raw addrspace(stack) readonly @Env> = bitcast v0 -> ref<raw addrspace(stack) readonly @Env>
    v6: fnvalue<fn() -> i32> = function.value @read_env, v5
    v7: i32 = call.indirect v6() -> fnvalue<fn() -> i32>
    return v7
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(42));
}

/// Closure values can be stored in arrays and invoked later.
#[test]
fn test_environment_loaded_from_array() {
    let mir = r#"
type @Env = { value: i32 }

#[environment(ref<managed @Env>)]
function @read_env() -> i32 {
block0:
    v0: ref<managed @Env> = function.environment
    v1: ref<managed i32> = field.addr v0, 0
    v2: i32 = load v1
    return v2
}

function @make_env(v0: i32) -> ref<managed @Env> {
block0(v0: i32):
    v1: ref<managed @Env> = managed.alloc @Env
    v2: ref<managed i32> = field.addr v1, 0
    store v2, v0
    return v1
}

function @caller(v0: i32) -> i32 {
block0(v0: i32):
    v1: ref<managed @Env> = call @make_env(v0)
    v2: fnvalue<fn() -> i32> = function.value @read_env, v1
    v3: [fnvalue<fn() -> i32>; 1] = array [fnvalue<fn() -> i32>; 1] (v2)
    v4: i32 = iconst 0i32
    v5: fnvalue<fn() -> i32> = element.get v3, v4
    v6: i32 = call.indirect v5() -> fnvalue<fn() -> i32>
    return v6
}"#;

    run_mir_expect(mir, "caller", &[Value::int32(8)], Value::int32(8));
}
