use crate::tests::TestProgram;
use destack_artifact::EmitFormat;

#[test]
fn test_reify_implicit_cast_in_binding() {
    // binding casts are inserted for mismatched types
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    let value: float = intValue();
    return value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function intValue(): int32 {
    return 1;
}
function test(): float64 {
    let value = (intValue() as float64);
    return value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_assignment() {
    // assignment casts are inserted for mismatched types
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    let value: float = intValue();
    value = intValue();
    return value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function intValue(): int32 {
    return 1;
}
function test(): float64 {
    let value = (intValue() as float64);
    value = (intValue() as float64);
    return value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_return() {
    // return casts are inserted for declared return types
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    return intValue();
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function intValue(): int32 {
    return 1;
}
function test(): float64 {
    return (intValue() as float64);
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_call_argument() {
    // call arguments are cast to parameter types
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function intValue(): int32 {
    return 1;
}

function takeFloat(value: float): float {
    return value;
}

function test(): float {
    return takeFloat(intValue());
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function intValue(): int32 {
    return 1;
}
function takeFloat(value): float64 {
    return value;
}
function test(): float64 {
    return takeFloat((intValue() as float64));
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_ternary() {
    // ternary branches cast to the expression type
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function floatValue(): float {
    return 1;
}

function intValue(): int32 {
    return 1;
}

function test(condition: boolean): float {
    return condition ? floatValue() : intValue();
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function floatValue(): float64 {
    return 1;
}
function intValue(): int32 {
    return 1;
}
function test(condition): float64 {
    return condition ? floatValue() : (intValue() as float64);
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_match_expression() {
    // match case expressions cast to the match expression type
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function intValue(): int32 {
    return 1;
}

function test(condition: boolean): float {
    let value: float = match (condition) {
        true => intValue()
        false => intValue()
    };
    return value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function intValue(): int32 {
    return 1;
}
function test(condition): float64 {
    let value;
    if (condition == true) {
        value = (intValue() as float64);
    } else {
        value = (intValue() as float64);
    }
    return value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_match_block() {
    // match case blocks cast their trailing expressions
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function intValue(): int32 {
    return 1;
}

function test(condition: boolean): float {
    let value: float = match (condition) {
        true => {
            let value = intValue();
            value
        }
        false => {
            let value = intValue();
            value
        }
    };
    return value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function intValue(): int32 {
    return 1;
}
function test(condition): float64 {
    let value;
    if (condition == true) {
        let value = intValue();
        value = (value as float64);
    } else {
        let value = intValue();
        value = (value as float64);
    }
    return value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_binary_comparison() {
    // comparison expressions cast numeric literals for alignment
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function test(value: float): boolean {
    return value < 2;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function test(value): boolean {
    return value < (2 as float64);
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_binary_arithmetic() {
    // arithmetic expressions do not cast numeric literals
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function test(value: float): float {
    return value - 1;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function test(value): float64 {
    return value - 1;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_binary_left_literal() {
    // numeric literals do not cast to the non literal side
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function test(value: float): float {
    return 2 + value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function test(value): float64 {
    return 2 + value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_union_upcast() {
    // union upcasts are inserted for union bindings
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 | float64 {
    let value: int32 | float64 = intValue();
    return value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function intValue(): int32 {
    return 1;
}
function test(): int32 | float64 {
    let value = (intValue() as int32 | float64);
    return value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_reference_narrowing() {
    // narrowed references insert implicit downcasts
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function test(value: Foo | Bar): int32 {
    if (value is Foo) {
        return value.x;
    }
    return 0;
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Foo {
    x: int32;
}
struct Bar {
    y: int32;
}
function test(value): int32 {
    if (value is Foo) {
        return (value as Foo).x;
    }
    return 0;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_guard_after_narrowing() {
    // narrowed references inside guard expressions insert casts
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function test(value: Foo | Bar): int32 {
    if (value is Foo && value.x > 0) {
        return value.x;
    }
    return 0;
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Foo {
    x: int32;
}
struct Bar {
    y: int32;
}
function test(value): int32 {
    if (value is Foo && (value as Foo).x > (0 as int32)) {
        return (value as Foo).x;
    }
    return 0;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_guard_multiple_checks() {
    // narrowed references inside compound guards insert casts consistently
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function test(value: Foo | Bar): int32 {
    if (value is Foo && value.x > 0 && value.x < 10) {
        return value.x;
    }
    return 0;
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Foo {
    x: int32;
}
struct Bar {
    y: int32;
}
function test(value): int32 {
    if (value is Foo && (value as Foo).x > (0 as int32) && (value as Foo).x < (10 as int32)) {
        return (value as Foo).x;
    }
    return 0;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_while_guard() {
    // while guards insert casts for narrowed references
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function test(value: Foo | Bar): int32 {
    while (value is Foo && value.x > 0) {
        break;
    }
    return 0;
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Foo {
    x: int32;
}
struct Bar {
    y: int32;
}
function test(value): int32 {
    while (value is Foo && (value as Foo).x > (0 as int32)) {
        break;
    }
    return 0;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_skips_is_operand() {
    // guard operands keep their original references
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function isFoo(value: Foo | Bar): boolean {
    return value is Foo;
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Foo {
    x: int32;
}
struct Bar {
    y: int32;
}
function isFoo(value): boolean {
    return value is Foo;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_typeof_guard() {
    // typeof guards narrow references without casting the guard operand
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function test(value: (() => int32) | int32): int32 {
    if (typeof value == "function") {
        return value();
    }
    return value;
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function test(value): int32 {
    if (typeof value == "function") {
        return (value as () => int32)();
    }
    return (value as int32);
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_instanceof_guard() {
    // instanceof guards narrow references without casting the guard operand
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Foo {
    x: int32 = 0;
}

class Bar {
    y: int32 = 0;
}

function test(value: Foo | Bar): int32 {
    if (value instanceof Foo) {
        return value.x;
    }
    return 0;
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
class Foo {
    x: int32 = 0;
}
class Bar {
    y: int32 = 0;
}
function test(value): int32 {
    if (value instanceof Foo) {
        return (value as Foo).x;
    }
    return 0;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_in_guard() {
    // in guards narrow references without casting the guard operand
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
type WithX = { x: int32 };
type WithY = { y: int32 };

function test(value: WithX | WithY): int32 {
    if ("x" in value) {
        return value.x;
    }
    return 0;
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
type WithX = { x: int32 };
type WithY = { y: int32 };
function test(value): int32 {
    if ('x' in value) {
        return (value as { x: int32 }).x;
    }
    return 0;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_match_guard() {
    // match guards narrow case bodies without casting the guard operand
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function test(value: Foo | Bar): int32 {
    match (value) {
        _ if value is Foo => value.x
        _ => 0
    }
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Foo {
    x: int32;
}
struct Bar {
    y: int32;
}
function test(value): int32 {
    if (value is Foo) return (value as Foo).x; else {
        return 0;
    }
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_skips_cast_operand() {
    // explicit casts do not add extra implicit casts
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Foo {
    x: int32;
}

struct Bar {
    y: int32;
}

function test(value: Foo | Bar): Foo {
    return value as Foo;
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Foo {
    x: int32;
}
struct Bar {
    y: int32;
}
function test(value): Foo {
    return value as Foo;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_skips_tagged_type_reference() {
    // tagged constructors keep their type reference intact
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: int32;
}

function make(value: int32): Counter {
    return Counter { value };
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Counter {
    value: int32;
}
function make(value): Counter {
    return Counter { value };
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_interface_upcast() {
    // interface upcasts are inserted for contextual constructor values
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Greeter {
    greet(): int32;
}

class GreeterImpl implements Greeter {
    value: int32;

    constructor(value: int32) {
        this.value = value;
        return;
    }

    greet(): int32 {
        return this.value;
    }
}

function test(): Greeter {
    let value: Greeter = new GreeterImpl(1);
    return value;
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
interface Greeter {
    greet(): int32;
}
class GreeterImpl implements Greeter {
    value: int32;
    constructor(value) {
        this.value = value;
        return;
    }
    greet(): int32 {
        return this.value;
    }
}
function test(): Greeter {
    let value = (new GreeterImpl(1) as Greeter);
    return value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_interface_to_interface() {
    // interface to interface casts are reified
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Greeter {
    greet(): int32;
}
interface Speaker {
    greet(): int32;
}

function test(value: Greeter): Speaker {
    let assigned: Speaker = value;
    return assigned;
}
"#,
    );

    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
interface Greeter {
    greet(): int32;
}
interface Speaker {
    greet(): int32;
}
function test(value): Speaker {
    let assigned = (value as Speaker);
    return assigned;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_nullable_upcast() {
    // nullable upcasts are inserted for nullable bindings
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 | null {
    let value: int32 | null = intValue();
    return value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function intValue(): int32 {
    return 1;
}
function test(): int32 | null {
    let value = (intValue() as int32 | null);
    return value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_nullable_null_literal() {
    // nullable upcasts are inserted for null literals
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function test(): int32 | null {
    let value: int32 | null = null;
    return value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function test(): int32 | null {
    let value = (null as int32 | null);
    return value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_undefined_literal_upcast() {
    // undefined upcasts are inserted for undefined literals
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function test(): int32 | undefined {
    let value: int32 | undefined = undefined;
    return value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function test(): int32 | undefined {
    let value = (undefined as int32 | undefined);
    return value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_nullable_return_literal() {
    // nullable upcasts are inserted for return literals
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function test(): int32 | null {
    return null;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function test(): int32 | null {
    return (null as int32 | null);
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_undefined_upcast() {
    // undefined upcasts are inserted for undefined unions
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 | undefined {
    let value: int32 | undefined = intValue();
    return value;
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function intValue(): int32 {
    return 1;
}
function test(): int32 | undefined {
    let value = (intValue() as int32 | undefined);
    return value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_nullable_undefined_call() {
    // nullable and undefined upcasts are inserted for call arguments
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function accept(value: int32 | null | undefined): int32 | null | undefined {
    return value;
}

function intValue(): int32 {
    return 1;
}

function test(): int32 | null | undefined {
    return accept(intValue());
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function accept(value): int32 | null | undefined {
    return value;
}
function intValue(): int32 {
    return 1;
}
function test(): int32 | null | undefined {
    return accept((intValue() as int32 | null | undefined));
}
"#,
    );
}

#[test]
fn test_reify_explicit_cast_expression() {
    // explicit casts stay explicit
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function intValue(): int32 {
    return 1;
}

function test(): float {
    return intValue() as float;
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated
    test.assert_elaborated(
        module_id,
        r#"
function intValue(): int32 {
    return 1;
}
function test(): float64 {
    return intValue() as float64;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_in_using_binding() {
    // using bindings cast initializers when needed
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function intValue(): int32 {
    return 1;
}

function test(): void {
    using value: float = intValue();
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated
    test.assert_elaborated(
        module_id,
        r#"
function intValue(): int32 {
    return 1;
}
function test(): void {
    using value = (intValue() as float64);
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_skip_same_type() {
    // matching types do not insert casts
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function intValue(): int32 {
    return 1;
}

function test(): int32 {
    let value: int32 = intValue();
    return value;
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated
    test.assert_elaborated(
        module_id,
        r#"
function intValue(): int32 {
    return 1;
}
function test(): int32 {
    let value = intValue();
    return value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_skip_numeric_literal() {
    // scalar literal numeric casts are omitted
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function test(): float {
    let value: float = 1;
    return value;
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated
    test.assert_elaborated(
        module_id,
        r#"
function test(): float64 {
    let value = 1;
    return value;
}
"#,
    );
}

#[test]
fn test_reify_implicit_cast_object_upcast() {
    // non-primitives are implicitly upcast to object
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function getArray(): int32[] {
    return [1, 2, 3];
}

function test(): object {
    let value: object = getArray();
    return value;
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated: array literal is typed, getArray() → object needs upcast
    test.assert_elaborated(
        module_id,
        r#"
function getArray(): int32[] {
    return [1, 2, 3];
}
function test(): object {
    let value = (getArray() as object);
    return value;
}
"#,
    );
}

#[test]
fn test_reify_explicit_cast_object_downcast() {
    // object is explicitly downcast to specific types
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Foo { x: int32 }

function getObject(): object {
    return { x: 1 };
}

function test(): Foo {
    return getObject() as Foo;
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated: object literal → object upcast, explicit Foo downcast preserved
    test.assert_elaborated(
        module_id,
        r#"
interface Foo {
    x: int32;
}
function getObject(): object {
    return ({ x: 1 } as object);
}
function test(): Foo {
    return getObject() as Foo;
}
"#,
    );
}

#[test]
fn test_reify_record_like_object_literal_in_binding() {
    // record like bindings reify object literals into map construction
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function build(): Record<string, int32> {
    let record: Record<string, int32> = { alpha: 1, beta: 2 };
    return record;
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated: record literal reifies to Map.from
    test.assert_elaborated(
        module_id,
        r#"
function build(): Record<string, int32> {
    let record = Map.from([("alpha", 1,), ("beta", 2,)]);
    return record;
}
"#,
    );
}

#[test]
fn test_reify_record_like_object_literal_in_assignment() {
    // record like assignments reify object literals into map construction
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function build(): Record<string, int32> {
    let record: Record<string, int32> = { alpha: 1 };
    record = { beta: 2 };
    return record;
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated: record literals reify to Map.from
    test.assert_elaborated(
        module_id,
        r#"
function build(): Record<string, int32> {
    let record = Map.from([("alpha", 1,)]);
    record = Map.from([("beta", 2,)]);
    return record;
}
"#,
    );
}

#[test]
fn test_reify_record_like_object_literal_in_return() {
    // record like returns reify object literals into map construction
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function build(): Record<string, int32> {
    return { beta: 2 };
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated: record literal reifies to Map.from
    test.assert_elaborated(
        module_id,
        r#"
function build(): Record<string, int32> {
    return Map.from([("beta", 2,)]);
}
"#,
    );
}

#[test]
fn test_reify_record_like_object_literal_in_call_argument() {
    // record like arguments reify object literals into map construction
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function take(record: Record<string, int32>): void {
    return;
}

function test(): void {
    take({ beta: 2 });
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated: record literal reifies to Map.from
    test.assert_elaborated(
        module_id,
        r#"
function take(record): void {
    return;
}
function test(): void {
    take(Map.from([("beta", 2,)]));
}
"#,
    );
}

#[test]
fn test_reify_record_like_object_literal_explicit_cast() {
    // record like explicit casts reify object literals into map construction
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function build(): Record<string, int32> {
    return { beta: 2 } as Record<string, int32>;
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated: record literal reifies to Map.from
    test.assert_elaborated(
        module_id,
        r#"
function build(): Record<string, int32> {
    return Map.from([("beta", 2,)]);
}
"#,
    );
}

#[test]
fn test_reify_record_like_object_literal_skips_plain_assignment() {
    // non-record assignments keep object literals
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function buildPlain(): { beta: int32 } {
    let plain = { beta: 2 } as { beta: int32 };
    plain = { beta: 4 } as { beta: int32 };
    return plain;
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated: object literals remain
    test.assert_elaborated(
        module_id,
        r#"
function buildPlain(): { beta: int32 } {
    let plain = { beta: 2 } as { beta: int32 };
    plain = { beta: 4 } as { beta: int32 };
    return plain;
}
"#,
    );
}

#[test]
fn test_reify_array_sized_value_in_binding() {
    // sized arrays reify into dynamic arrays in bindings
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function build(values: int32[3]): int32[] {
    let dynamic: int32[] = values;
    return dynamic;
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated: sized array reifies to Array.fromSized
    test.assert_elaborated(
        module_id,
        r#"
function build(values): int32[] {
    let dynamic = Array.fromSized(values);
    return dynamic;
}
"#,
    );
}

#[test]
fn test_reify_array_sized_value_in_return() {
    // sized arrays reify into dynamic arrays in returns
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function build(values: int32[3]): int32[] {
    return values;
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated: sized array reifies to Array.fromSized
    test.assert_elaborated(
        module_id,
        r#"
function build(values): int32[] {
    return Array.fromSized(values);
}
"#,
    );
}

#[test]
fn test_reify_array_sized_value_in_call_argument() {
    // sized arrays reify into dynamic arrays in call arguments
    let test = TestProgram::memory_sequential_with_prelude().with_profile_emit(EmitFormat::Native);
    let module_id = test.add_module(
        "test.ds",
        r#"
function take(values: int32[]): void {
    return;
}

function test(values: int32[3]): void {
    take(values);
}
"#,
    );

    // run elaborate
    test.elaborate_module(module_id);
    test.compile_check_clean();

    // assert elaborated: sized array reifies to Array.fromSized
    test.assert_elaborated(
        module_id,
        r#"
function take(values): void {
    return;
}
function test(values): void {
    take(Array.fromSized(values));
}
"#,
    );
}
