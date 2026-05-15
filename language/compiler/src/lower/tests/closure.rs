use destack_engine::Value;

use crate::TestProgram;

/// Verify non-capturing closures lower to function.address plus an empty environment.
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
type makeIdentity.return.function = (int32) => int32;

function makeIdentity(): makeIdentity.return.function {
entry0:
    value0: ref<{  }, managed, nullable> = null
    value1: (int32) => int32 = callable.bind makeIdentity.lambda#6, value0
    return value1
}

@environment(ref<{  }, managed, nullable>)
function makeIdentity.lambda#6(value0: int32): int32 {
entry0(value0: int32):
    return value0
}

function applyIdentity(value0: int32): int32 {
entry0(value0: int32):
    value1: makeIdentity.return.function =
        call makeIdentity(): () -> makeIdentity.return.function
    value2: int32 = call.indirect value1(value0): (int32) -> int32
    return value2
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
type makeAdder.return.function = (int32) => int32;
type env.7 {
    base: int32;
}

function makeAdder(): makeAdder.return.function {
entry0:
    value0: int32 = 5int32
    value1: ref<env.7, managed> = new env.7
    value2: ref<int32, managed> = field.address value1, 0
    store value2, value0
    value3: (int32) => int32 = callable.bind makeAdder.lambda#7, value1
    return value3
}

@environment(ref<env.7, managed>)
function makeAdder.lambda#7(value0: int32): int32 {
entry0(value0: int32):
    value1: ref<env.7, managed> = callable.environment
    value2: ref<int32, managed> = field.address value1, 0
    value3: int32 = load value2
    value4: int32 = int.add value3, value0
    return value4
}

function applyAdder(value0: int32): int32 {
entry0(value0: int32):
    value1: makeAdder.return.function = call makeAdder(): () -> makeAdder.return.function
    value2: int32 = call.indirect value1(value0): (int32) -> int32
    return value2
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
type makeCounter.return.function = () => int32;
type env.6 {
    count: ref<int32, managed>;
}

function makeCounter(): makeCounter.return.function {
entry0:
    value0: int32 = 0int32
    value1: ref<int32, managed> = new int32
    store value1, value0
    value2: ref<env.6, managed> = new env.6
    value3: ref<ref<int32, managed>, managed> = field.address value2, 0
    value4: ref<int32, managed> = cast.bit value1 -> ref<int32, managed>
    store value3, value4
    value5: () => int32 = callable.bind makeCounter.lambda#6, value2
    return value5
}

@environment(ref<env.6, managed>)
function makeCounter.lambda#6(): int32 {
entry0:
    value0: ref<env.6, managed> = callable.environment
    value1: ref<ref<int32, managed>, managed> = field.address value0, 0
    value2: ref<int32, managed> = load value1
    value3: int32 = load value2
    value4: int32 = 1int32
    value5: int32 = int.add value3, value4
    value6: ref<ref<int32, managed>, managed> = field.address value0, 0
    value7: ref<int32, managed> = load value6
    store value7, value5
    value8: ref<ref<int32, managed>, managed> = field.address value0, 0
    value9: ref<int32, managed> = load value8
    value10: int32 = load value9
    return value10
}

function runCounter(): int32 {
entry0:
    value0: makeCounter.return.function = call makeCounter(): () -> makeCounter.return.function
    value1: int32 = call.indirect value0(): () -> int32
    return value1
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
