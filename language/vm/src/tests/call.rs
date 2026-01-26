use crate::diagnostic::Error;
use crate::memory::Value;
use crate::tests::{run_mir, run_mir_expect};

/// function.addr produces a callable pointer for call.indirect.
#[test]
fn test_function_addr_indirect_call() {
    let mir = r#"
function @add(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iadd v0, v0
    return v1
}

function @caller(v0: i32) -> i32 {
block0(v0: i32):
    v1: fn(i32) -> i32 = function.addr @add
    v2: i32 = call.indirect v1(v0) -> fn(i32) -> i32
    return v2
}"#;
    run_mir_expect(mir, "caller", &[Value::int32(21)], Value::int32(42));
}

/// call.indirect passes the closure environment for function.env.
#[test]
fn test_call_indirect_env() {
    let mir = r#"
#[closure_env(ref<raw addrspace(stack) i32>)]
function @read_env() -> i32 {
block0:
    v0: ref<raw addrspace(stack) i32> = function.env
    v1: i32 = load v0
    return v1
}

function @caller() -> i32 {
block0:
    v0: ref<raw addrspace(stack) mut i32> = stack.alloc i32
    v1: i32 = iconst 41
    store v0, v1
    v2: fn() -> i32 = function.addr @read_env
    v3: i32 = call.indirect v2(env=v0) -> fn() -> i32
    return v3
}"#;
    run_mir_expect(mir, "caller", &[], Value::int32(41));
}

/// function.env errors when the frame has no closure environment.
#[test]
fn test_function_env_requires_env() {
    let mir = r#"
#[closure_env(ref<raw addrspace(stack) i32>)]
function @read_env() -> i32 {
block0:
    v0: ref<raw addrspace(stack) i32> = function.env
    v1: i32 = load v0
    return v1
}

function @caller() -> i32 {
block0:
    v0: i32 = call @read_env()
    return v0
}"#;
    let result = run_mir(mir, "caller", &[]);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(matches!(error.error, Error::InvalidInstruction));
}

/// Tail call indirect forwards the closure environment.
#[test]
fn test_tailcall_indirect_env() {
    let mir = r#"
#[closure_env(ref<managed mut i32>)]
function @read_env() -> i32 {
block0:
    v0: ref<managed mut i32> = function.env
    v1: i32 = load v0
    return v1
}

function @caller() -> i32 {
block0:
    v0: ref<managed mut i32> = managed.alloc i32
    v1: i32 = iconst 99i32
    store v0, v1
    v2: fn() -> i32 = function.addr @read_env
    tailcall.indirect v2(env=v0) -> fn() -> i32
}"#;
    run_mir_expect(mir, "caller", &[], Value::int32(99));
}

/// Managed closure environments hold by-reference capture cells.
#[test]
fn test_closure_env_managed_reference_cell() {
    let mir = r#"
type @Env = { cell: ref<managed mut i32> }

#[closure_env(ref<managed mut @Env>)]
function @increment() -> i32 {
block0:
    v0: ref<managed mut @Env> = function.env
    v1: ref<managed mut ref<managed mut i32>> = field.addr v0, 0
    v2: ref<managed mut i32> = load v1
    v3: i32 = load v2
    v4: i32 = iconst 1i32
    v5: i32 = iadd v3, v4
    store v2, v5
    return v5
}

function @caller() -> i32 {
block0:
    v0: ref<managed mut i32> = managed.alloc i32
    v1: i32 = iconst 0i32
    store v0, v1
    v2: ref<managed mut @Env> = managed.alloc @Env
    v3: ref<managed mut ref<managed mut i32>> = field.addr v2, 0
    store v3, v0
    v4: fn() -> i32 = function.addr @increment
    v5: i32 = call.indirect v4(env=v2) -> fn() -> i32
    v6: i32 = call.indirect v4(env=v2) -> fn() -> i32
    return v6
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(2));
}

/// Managed closure environments support by-value fields.
#[test]
fn test_closure_env_by_value_field() {
    let mir = r#"
type @Env = { value: i32 }

#[closure_env(ref<managed mut @Env>)]
function @read_env() -> i32 {
block0:
    v0: ref<managed mut @Env> = function.env
    v1: ref<managed mut i32> = field.addr v0, 0
    v2: i32 = load v1
    v3: i32 = iconst 2i32
    v4: i32 = iadd v2, v3
    return v4
}

function @caller() -> i32 {
block0:
    v0: ref<managed mut @Env> = managed.alloc @Env
    v1: ref<managed mut i32> = field.addr v0, 0
    v2: i32 = iconst 40i32
    store v1, v2
    v3: fn() -> i32 = function.addr @read_env
    v4: i32 = call.indirect v3(env=v0) -> fn() -> i32
    return v4
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(42));
}

/// call.indirect selects the environment provided at the callsite.
#[test]
fn test_closure_env_selects_callsite_env() {
    let mir = r#"
type @Env = { value: i32 }

#[closure_env(ref<managed mut @Env>)]
function @read_env() -> i32 {
block0:
    v0: ref<managed mut @Env> = function.env
    v1: ref<managed mut i32> = field.addr v0, 0
    v2: i32 = load v1
    return v2
}

function @caller() -> i32 {
block0:
    v0: ref<managed mut @Env> = managed.alloc @Env
    v1: ref<managed mut @Env> = managed.alloc @Env
    v2: ref<managed mut i32> = field.addr v0, 0
    v3: ref<managed mut i32> = field.addr v1, 0
    v4: i32 = iconst 10i32
    v5: i32 = iconst 20i32
    store v2, v4
    store v3, v5
    v6: fn() -> i32 = function.addr @read_env
    v7: i32 = call.indirect v6(env=v0) -> fn() -> i32
    v8: i32 = call.indirect v6(env=v1) -> fn() -> i32
    v9: i32 = iadd v7, v8
    return v9
}"#;

    run_mir_expect(mir, "caller", &[], Value::int32(30));
}
