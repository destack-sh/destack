use destack_vm::Value;

use crate::TestProgram;

/// Verify non-capturing closures lower to function.addr + empty env.
#[test]
fn test_lower_closure_without_captures() {
    let test = TestProgram::memory_sequential_with_prelude();
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
type @fn#param.int32#return.int32 = fnvalue<fn(i32) -> i32>

function @makeIdentity() -> @fn#param.int32#return.int32 {
block0:
    v0: ref?<managed {  }> = iconst null
    v1: fnvalue<fn(i32) -> i32> = function.value @makeIdentity.lambda#7, v0
    return v1
}

#[environment(ref?<managed {  }>)]
function @makeIdentity.lambda#7(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}

function @applyIdentity(v0: i32) -> i32 {
block0(v0: i32):
    v1: @fn#param.int32#return.int32 = call @makeIdentity() -> fn() -> @fn#param.int32#return.int32
    v2: i32 = call.indirect v1(v0) -> @fn#param.int32#return.int32
    return v2
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
    let test = TestProgram::memory_sequential_with_prelude();
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
type @fn#param.int32#return.int32 = fnvalue<fn(i32) -> i32>
type @environment#9 = { base: i32 }

function @makeAdder() -> @fn#param.int32#return.int32 {
block0:
    v0: i32 = iconst 5i32
    v1: ref<managed @environment#9> = managed.alloc @environment#9
    v2: ref<managed i32> = field.addr v1, 0
    store v2, v0
    v3: fnvalue<fn(i32) -> i32> = function.value @makeAdder.lambda#9, v1
    return v3
}

#[environment(ref<managed @environment#9>)]
function @makeAdder.lambda#9(v0: i32) -> i32 {
block0(v0: i32):
    v1: ref<managed @environment#9> = function.environment
    v2: ref<managed i32> = field.addr v1, 0
    v3: i32 = load v2
    v4: i32 = trunc v3 -> i32
    v5: i32 = iadd v4, v0
    return v5
}

function @applyAdder(v0: i32) -> i32 {
block0(v0: i32):
    v1: @fn#param.int32#return.int32 = call @makeAdder() -> fn() -> @fn#param.int32#return.int32
    v2: i32 = call.indirect v1(v0) -> @fn#param.int32#return.int32
    return v2
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
    let test = TestProgram::memory_sequential_with_prelude();
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
type @fn#return.int32 = fnvalue<fn() -> i32>
type @environment#8 = { count: ref<managed i32> }

function @makeCounter() -> @fn#return.int32 {
block0:
    v0: i32 = iconst 0i32
    v1: ref<managed i32> = managed.alloc i32
    store v1, v0
    v2: ref<managed @environment#8> = managed.alloc @environment#8
    v3: ref<managed ref<managed i32>> = field.addr v2, 0
    v4: ref<managed i32> = bitcast v1 -> ref<managed i32>
    store v3, v4
    v5: fnvalue<fn() -> i32> = function.value @makeCounter.lambda#8, v2
    return v5
}

#[environment(ref<managed @environment#8>)]
function @makeCounter.lambda#8() -> i32 {
block0:
    v0: ref<managed @environment#8> = function.environment
    v1: ref<managed ref<managed i32>> = field.addr v0, 0
    v2: ref<managed i32> = load v1
    v3: i32 = load v2
    v4: i32 = iconst 1i32
    v5: i32 = iadd v3, v4
    v6: ref<managed ref<managed i32>> = field.addr v0, 0
    v7: ref<managed i32> = load v6
    store v7, v5
    v8: ref<managed ref<managed i32>> = field.addr v0, 0
    v9: ref<managed i32> = load v8
    v10: i32 = load v9
    return v10
}

function @runCounter() -> i32 {
block0:
    v0: @fn#return.int32 = call @makeCounter() -> fn() -> @fn#return.int32
    v1: i32 = call.indirect v0() -> @fn#return.int32
    return v1
}
        "#,
    );

    test.assert_mir_function_output(module_id, "native", "runCounter", &[], Value::int32(1));
}

/// Verify mixed captures with values, references, and aggregates.
#[test]
fn test_lower_closure_capture_multiple_bindings() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Point {
    x: int32 = 0;
    y: int32 = 0;

    constructor(x: int32, y: int32) {
        this.x = x;
        this.y = y;
        return;
    }
}

function makeMixer(): () => int32 {
    const base = 3;
    let count: int32 = 0;
    let point: Point = new Point(4, 5);
    return (): int32 => {
        count = count + 1;
        return base + count + point.x + point.y;
    };
}

function runMixer(): int32 {
    let mixer = makeMixer();
    return mixer();
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "runMixer", &[], Value::int32(13));
}

/// Verify closures mutate captured bindings while reading class fields.
#[test]
fn test_lower_closure_mutates_captured_binding_with_class() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Accumulator {
    value: int32 = 0;

    constructor(value: int32) {
        this.value = value;
        return;
    }
}

function makeAccumulator(): () => int32 {
    let acc: Accumulator = new Accumulator(1);
    let count: int32 = 0;
    return (): int32 => {
        let step: int32 = 2;
        count = count + step;
        return acc.value + count;
    };
}

function runAccumulator(): int32 {
    let next = makeAccumulator();
    let first = next();
    let second = next();
    return first * 10 + second;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "runAccumulator", &[], Value::int32(35));
}

/// Verify closures combine by-value and by-reference captures.
#[test]
fn test_lower_closure_mixes_value_and_reference_captures() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function makeStepper(): () => int32 {
    const base = 10;
    let step: int32 = 1;
    return (): int32 => {
        step = step + 1;
        return base + step;
    };
}

function runStepper(): int32 {
    let next = makeStepper();
    let first = next();
    let second = next();
    return first * 100 + second;
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "runStepper", &[], Value::int32(1213));
}

/// Verify closures can capture implicit `this` from member methods.
#[test]
fn test_lower_closure_captures_this() {
    let test = TestProgram::memory_sequential_with_prelude();
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

/// Verify captured closures can be passed as function arguments.
#[test]
fn test_lower_closure_passed_as_argument() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function makeAdder(base: int32): (value: int32) => int32 {
    return (value: int32): int32 => base + value;
}

function applyOnce(value: int32, op: (value: int32) => int32): int32 {
    return op(value);
}

function run(): int32 {
    let add = makeAdder(7);
    return applyOnce(5, add);
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "run", &[], Value::int32(12));
}

/// Verify closures stored in class fields are invoked via call.indirect.
#[test]
fn test_lower_closure_stored_in_class_field() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Holder {
    action: () => int32;

    constructor(action: () => int32) {
        this.action = action;
        return;
    }

    run(): int32 {
        return this.action();
    }
}

function makeCounter(): () => int32 {
    let count: int32 = 0;
    return (): int32 => {
        count = count + 1;
        return count;
    };
}

function run(): int32 {
    let holder: Holder = new Holder(makeCounter());
    return holder.run();
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "run", &[], Value::int32(1));
}

/// Verify named functions lower to closure values with empty environments.
#[test]
fn test_lower_named_function_as_value() {
    let test = TestProgram::memory_sequential_with_prelude();
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

    test.assert_mir_function_output(
        module_id,
        "native",
        "applyDouble",
        &[Value::int32(9)],
        Value::int32(18),
    );
}

/// Verify nested named functions capture from multiple scopes.
#[test]
#[ignore]
// TODO #Broken: nested named function captures are not resolved yet
fn test_lower_nested_named_function_captures() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "test.ds",
        r#"
function outer(start: int32): () => int32 {
    let base = start;
    function middle(delta: int32): () => int32 {
        let step = delta;
        function inner(): int32 {
            base = base + 1;
            step = step + 2;
            return base + step;
        }
        return inner;
    }
    return middle(3);
}

function run(): int32 {
    let next = outer(10);
    return next();
}
"#,
    );

    test.add_target(module_id, "native");
    test.lower_module(module_id, "native");
    test.compile_check_clean();

    test.assert_mir_function_output(module_id, "native", "run", &[], Value::int32(16));
}
