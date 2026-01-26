use destack_vm::Value;

use crate::TestProgram;

/// Verify non-capturing closures lower to function.addr + empty env.
#[test]
fn test_lower_closure_without_captures() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function makeIdentity(): (value: int32) => int32 {
    return (value: int32): int32 => value;
}

function applyIdentity(input: int32): int32 {
    let identity = makeIdentity();
    return identity(input);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type @fn#param.int32#return.int32 = { @function_ptr: fn(i32) -> i32, @env: ref?<managed mut void> }

function @makeIdentity() -> @fn#param.int32#return.int32 {
block0:
    v0: fn(i32) -> i32 = function.addr @makeIdentity.lambda#7
    v1: ref?<managed mut {  }> = iconst null
    v2: ref?<managed mut void> = bitcast v1 -> ref?<managed mut void>
    v3: @fn#param.int32#return.int32 = struct @fn#param.int32#return.int32 (v0, v2)
    return v3
}

#[closure_env(ref?<managed mut {  }>)]
function @makeIdentity.lambda#7(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}

function @applyIdentity(v0: i32) -> i32 {
block0(v0: i32):
    v1: @fn#param.int32#return.int32 = call @makeIdentity() -> fn() -> @fn#param.int32#return.int32
    v2: fn(i32) -> i32 = field.get v1, 0
    v3: ref?<managed mut void> = field.get v1, 1
    v4: i32 = call.indirect v2(v0, env=v3) -> fn(i32) -> i32
    return v4
}
        "#,
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "applyIdentity",
        &[Value::int32(7)],
        Value::int32(7),
    );
}

/// Verify by-value capture stores the value into the environment.
#[test]
fn test_lower_closure_capture_by_value() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function makeAdder(): (value: int32) => int32 {
    const base = 5;
    return (value: int32): int32 => base + value;
}

function applyAdder(input: int32): int32 {
    let add = makeAdder();
    return add(input);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type @fn#param.int32#return.int32 = { @function_ptr: fn(i32) -> i32, @env: ref?<managed mut void> }
type @Struct0 = { base: i32 }

function @makeAdder() -> @fn#param.int32#return.int32 {
block0:
    v0: i32 = iconst 5i32
    v1: fn(i32) -> i32 = function.addr @makeAdder.lambda#9
    v2: ref<managed mut @Struct0> = managed.alloc @Struct0
    v3: ref<managed mut i32> = field.addr v2, 0
    store v3, v0
    v4: ref?<managed mut void> = bitcast v2 -> ref?<managed mut void>
    v5: @fn#param.int32#return.int32 = struct @fn#param.int32#return.int32 (v1, v4)
    return v5
}

#[closure_env(ref<managed mut @Struct0>)]
function @makeAdder.lambda#9(v0: i32) -> i32 {
block0(v0: i32):
    v1: ref<managed mut @Struct0> = function.env
    v2: ref<managed mut i32> = field.addr v1, 0
    v3: i32 = load v2
    v4: i32 = trunc v3 -> i32
    v5: i32 = iadd v4, v0
    return v5
}

function @applyAdder(v0: i32) -> i32 {
block0(v0: i32):
    v1: @fn#param.int32#return.int32 = call @makeAdder() -> fn() -> @fn#param.int32#return.int32
    v2: fn(i32) -> i32 = field.get v1, 0
    v3: ref?<managed mut void> = field.get v1, 1
    v4: i32 = call.indirect v2(v0, env=v3) -> fn(i32) -> i32
    return v4
}
        "#,
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "applyAdder",
        &[Value::int32(7)],
        Value::int32(12),
    );
}

/// Verify by-reference capture stores a pointer to the mutable binding.
#[test]
fn test_lower_closure_capture_by_reference() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function makeCounter(): () => int32 {
    let count: int32 = 0;
    return (): int32 => {
        count = count + 1;
        return count;
    };
}

function runCounter(): int32 {
    let next = makeCounter();
    return next();
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
type @fn#return.int32 = { @function_ptr: fn() -> i32, @env: ref?<managed mut void> }
type @Struct0 = { count: ref<managed mut i32> }

function @makeCounter() -> @fn#return.int32 {
block0:
    v0: i32 = iconst 0i32
    v1: ref<managed mut i32> = managed.alloc i32
    store v1, v0
    v2: fn() -> i32 = function.addr @makeCounter.lambda#8
    v3: ref<managed mut @Struct0> = managed.alloc @Struct0
    v4: ref<managed mut ref<managed mut i32>> = field.addr v3, 0
    v5: ref<managed mut i32> = bitcast v1 -> ref<managed mut i32>
    store v4, v5
    v6: ref?<managed mut void> = bitcast v3 -> ref?<managed mut void>
    v7: @fn#return.int32 = struct @fn#return.int32 (v2, v6)
    return v7
}

#[closure_env(ref<managed mut @Struct0>)]
function @makeCounter.lambda#8() -> i32 {
block0:
    v0: ref<managed mut @Struct0> = function.env
    v1: ref<managed mut ref<managed mut i32>> = field.addr v0, 0
    v2: ref<managed mut i32> = load v1
    v3: i32 = load v2
    v4: i32 = iconst 1i32
    v5: i32 = trunc v4 -> i32
    v6: i32 = iadd v3, v5
    v7: ref<managed mut ref<managed mut i32>> = field.addr v0, 0
    v8: ref<managed mut i32> = load v7
    store v8, v6
    v9: ref<managed mut ref<managed mut i32>> = field.addr v0, 0
    v10: ref<managed mut i32> = load v9
    v11: i32 = load v10
    return v11
}

function @runCounter() -> i32 {
block0:
    v0: @fn#return.int32 = call @makeCounter() -> fn() -> @fn#return.int32
    v1: fn() -> i32 = field.get v0, 0
    v2: ref?<managed mut void> = field.get v0, 1
    v3: i32 = call.indirect v1(env=v2) -> fn() -> i32
    return v3
}
        "#,
    );

    test.assert_mir_function_output(module_id, "native", "runCounter", &[], Value::int32(1));
}

/// Verify closures can capture implicit `this` from member methods.
#[test]
fn test_lower_closure_captures_this() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Counter {
    value: int32 = 0;

    constructor() {
        this.value = 0;
        return;
    }

    make(step: int32): () => int32 {
        const current = this;
        return (): int32 => {
            return current.value + step;
        };
    }
}

function run(): int32 {
    let counter: Counter = new Counter();
    let next = counter.make(2);
    return next();
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "run", &[], Value::int32(2));
}

/// Verify named functions lower to closure values with empty environments.
#[test]
fn test_lower_named_function_as_value() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function double(value: int32): int32 {
    return value + value;
}

function applyDouble(input: int32): int32 {
    let op = double;
    return op(input);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir(
        module_id,
        "native",
        r#"
#[closure_env(ref?<managed mut {  }>)]
function @double(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iadd v0, v0
    return v1
}

function @applyDouble(v0: i32) -> i32 {
block0(v0: i32):
    v1: fn(i32) -> i32 = function.addr @double
    v2: ref?<managed mut {  }> = iconst null
    v3: ref?<managed mut void> = bitcast v2 -> ref?<managed mut void>
    v4: { @function_ptr: fn(i32) -> i32, @env: ref?<managed mut void> } = struct { @function_ptr: fn(i32) -> i32, @env: ref?<managed mut void> } (v1, v3)
    v5: fn(i32) -> i32 = field.get v4, 0
    v6: ref?<managed mut void> = field.get v4, 1
    v7: i32 = call.indirect v5(v0, env=v6) -> fn(i32) -> i32
    return v7
}
        "#,
    );

    test.assert_mir_function_output(
        module_id,
        "native",
        "applyDouble",
        &[Value::int32(9)],
        Value::int32(18),
    );
}

/// Verify nested named functions capture mutable locals and parameters.
// FUGU #Broken: nested lambda call resolution is missing in Analyze
#[test]
#[ignore]
fn test_lower_nested_named_function_captures() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
function outer(seed: int32): int32 {
    let value: int32 = seed;
    let middle = (step: int32): int32 => {
        let offset: int32 = 1;
        return ((delta: int32): int32 => {
            value = value + delta;
            offset = offset + step;
            return value + offset;
        })(2);
    };
    return middle(3);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(
        module_id,
        "native",
        "outer",
        &[Value::int32(10)],
        Value::int32(16),
    );
}
