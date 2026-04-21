use crate::tests::TestProgram;
use destack_artifact::EmitFormat;
use destack_dir as dir;
use dir::{Expression, Resolution};

#[test]
fn test_reify_resolution_keeps_static_member_access() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point { x: int32; y: int32 }

function getX(p: Point): int32 {
    p.x
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Point {
    x: int32;
    y: int32;
}
function getX(p): int32 {
    return p.x;
}
"#,
    );
}

#[test]
fn test_reify_resolution_keeps_builtin_operator() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function add(a: int32, b: int32): int32 {
    a + b
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function add(a, b): int32 {
    return a + b;
}
"#,
    );
}

#[test]
fn test_reify_static_index_access_array() {
    // array indexing returns T | undefined because of potential out-of-bounds
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function first(arr: int32[]): int32 | undefined {
    arr[0]
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
function first(arr): int32 | undefined {
    return arr[0];
}
"#,
    );
}

#[test]
fn test_reify_dynamic_member_access_on_union() {
    // member access on union types where both have same-named field
    // needs dynamic dispatch because User.name and Admin.name are different symbols
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct User { name: string; role: string }
struct Admin { name: string; level: int32 }

function getName(person: User | Admin): string {
    person.name
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    // after elaborate: return if becomes if { return } else { return }
    test.assert_elaborated(
        module_id,
        r#"
struct User {
    name: string;
    role: string;
}
struct Admin {
    name: string;
    level: int32;
}
function getName(person): string {
    if (person is User) {
        return person.name;
    } else {
        return person.name;
    }
}
"#,
    );
}

#[test]
fn test_reify_dynamic_method_call_on_union() {
    // method call on union type with different method implementations
    // should be reified into if-else chain with is type checks
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Cat {
    name: string

    speak(): string { "meow" }
}

struct Dog {
    name: string

    speak(): string { "woof" }
}

function greet(pet: Cat | Dog): string {
    pet.speak()
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Cat {
    name: string;
    speak(): string {
        return "meow";
    }
}
struct Dog {
    name: string;
    speak(): string {
        return "woof";
    }
}
function greet(pet): string {
    if (pet is Cat) {
        return pet.speak();
    } else {
        return pet.speak();
    }
}
"#,
    );
}

#[test]
fn test_reify_dynamic_method_call_three_variants() {
    // method call on union with 3 types creates nested if-else chain
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Cat {
    name: string

    speak(): string { "meow" }
}

struct Dog {
    name: string

    speak(): string { "woof" }
}

struct Bird {
    name: string

    speak(): string { "chirp" }
}

function greet(pet: Cat | Dog | Bird): string {
    pet.speak()
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Cat {
    name: string;
    speak(): string {
        return "meow";
    }
}
struct Dog {
    name: string;
    speak(): string {
        return "woof";
    }
}
struct Bird {
    name: string;
    speak(): string {
        return "chirp";
    }
}
function greet(pet): string {
    if (pet is Cat) {
        return pet.speak();
    } else if (pet is Dog) {
        return pet.speak();
    } else {
        return pet.speak();
    }
}
"#,
    );
}

#[test]
fn test_reify_dynamic_method_call_on_union_extension() {
    // method call on union type with extension methods should reify to dynamic dispatch
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Cat {
    name: string
}

struct Dog {
    name: string
}

extension of Cat {
    speak(): string { "meow" }
}

extension of Dog {
    speak(): string { "woof" }
}

function greet(pet: Cat | Dog): string {
    pet.speak()
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Cat {
    name: string;
}
struct Dog {
    name: string;
}
extension of Cat {
    speak(): string {
        return "meow";
    }
}
extension of Dog {
    speak(): string {
        return "woof";
    }
}
function greet(pet): string {
    if (pet is Cat) {
        return pet.speak();
    } else {
        return pet.speak();
    }
}
"#,
    );
}

#[test]
fn test_reify_dynamic_method_call_casts_per_static_branch() {
    // dynamic call branches reify argument casts per selected signature
    let test = TestProgram::memory_sequential_with_prelude_and_libs()
        .with_profile_emit(EmitFormat::Native)
        .with_profile_libs(&["native"]);
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Cat {
    speak(amount: int64): int32 { 1 }
}

struct Dog {
    speak(amount: int32): int32 { 2 }
}

function speakVolume(pet: Cat | Dog, amount: int32): int32 {
    pet.speak(amount)
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Cat {
    speak(amount): int32 {
        return 1;
    }
}
struct Dog {
    speak(amount): int32 {
        return 2;
    }
}
function speakVolume(pet, amount): int32 {
    if (pet is Cat) {
        return pet.speak((amount as int64));
    } else {
        return pet.speak(amount);
    }
}
"#,
    );
}

/// TODO #Incomplete: enable when Analyze produces dynamic operator union resolution instead of EA203
#[test]
#[ignore = "TODO #Incomplete: dynamic operator union resolution is blocked by Analyze (EA203)"]
fn test_reify_dynamic_operator_call_on_union() {
    // dynamic operator calls on unions should split into static branches
    let test = TestProgram::memory_sequential_with_prelude_and_libs()
        .with_profile_emit(EmitFormat::Native)
        .with_profile_libs(&["native"]);
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Cat {
    value: int32
}

struct Dog {
    value: int32
}

extension of Cat implements Add<int32, int32> {
    add(other: int32): int32 {
        this.value + other
    }
}

extension of Dog implements Add<int32, int32> {
    add(other: int32): int32 {
        this.value + other
    }
}

function addValue(pet: Cat | Dog, amount: int32): int32 {
    pet + amount
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Cat {
    value: int32;
}

struct Dog {
    value: int32;
}

extension of Cat implements Add<int32, int32> {
    add(other): int32 {
        return this.value + other;
    }
}

extension of Dog implements Add<int32, int32> {
    add(other): int32 {
        return this.value + other;
    }
}

function addValue(pet, amount): int32 {
    if (pet is Cat) {
        return pet.add(amount);
    } else {
        return pet.add(amount);
    }
}
"#,
    );
}

#[test]
fn test_reify_static_method_call_single_type() {
    // method call on non-union type stays unchanged
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: int32

    increment(): Counter {
        Counter { value: this.value + 1 }
    }
}

function bump(c: Counter): Counter {
    c.increment()
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
    increment(): Counter {
        return Counter { value: this.value + 1 };
    }
}
function bump(c): Counter {
    return c.increment();
}
"#,
    );
}

#[test]
fn test_reify_static_method_call_polymorphic_type() {
    // method call on polymorphic type stays unchanged
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Counter {
    increment(): Counter;
}

function bump(c: Counter): Counter {
    c.increment()
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
interface Counter {
    increment(): Counter;
}
function bump(c): Counter {
    return c.increment();
}
"#,
    );
}

#[test]
fn test_reify_static_operator_add_to_method_call() {
    // overloaded add rewrites to `add` method call
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: int32
}

extension of Counter implements Add<int32> {
    add(other: int32): Counter {
        Counter { value: this.value + other }
    }
}

function increment(counter: Counter): Counter {
    counter + 1
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
extension of Counter implements Add<int32> {
    add(other): Counter {
        return Counter { value: this.value + other };
    }
}
function increment(counter): Counter {
    return counter.add(1);
}
"#,
    );
}

#[test]
fn test_reify_static_operator_not_equal_to_equal_call() {
    // overloaded not equal rewrites to `!a.equal(b)`
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: int32
}

extension of Counter implements Equal {
    equal(other: Counter): boolean {
        this.value == other.value
    }
}

function differs(left: Counter, right: Counter): boolean {
    left != right
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
extension of Counter implements Equal {
    equal(other): boolean {
        return this.value == other.value;
    }
}
function differs(left, right): boolean {
    return !left.equal(right);
}
"#,
    );
}

#[test]
fn test_reify_static_operator_compare_to_ordering_check() {
    // overloaded comparisons rewrite through compare and Ordering
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: int32
}

extension of Counter implements Compare {
    compare(other: Counter): Ordering {
        if (this.value < other.value) {
            return Ordering.Less;
        } else if (this.value > other.value) {
            return Ordering.Greater;
        }

        return Ordering.Equal;
    }
}

function isLess(left: Counter, right: Counter): boolean {
    left < right
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
extension of Counter implements Compare {
    compare(other): Ordering {
        if (this.value < other.value) {
            return Ordering.Less;
        } else if (this.value > other.value) {
            return Ordering.Greater;
        }
        return Ordering.Equal;
    }
}
function isLess(left, right): boolean {
    return left.compare(right) == Ordering.Less;
}
"#,
    );
}

#[test]
fn test_reify_static_operator_greater_than_or_equal_to_ordering_check() {
    // overloaded >= rewrites through compare and ordering less check
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: int32
}

extension of Counter implements Compare {
    compare(other: Counter): Ordering {
        if (this.value < other.value) {
            return Ordering.Less;
        } else if (this.value > other.value) {
            return Ordering.Greater;
        }

        return Ordering.Equal;
    }
}

function isAtLeast(left: Counter, right: Counter): boolean {
    left >= right
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
extension of Counter implements Compare {
    compare(other): Ordering {
        if (this.value < other.value) {
            return Ordering.Less;
        } else if (this.value > other.value) {
            return Ordering.Greater;
        }
        return Ordering.Equal;
    }
}
function isAtLeast(left, right): boolean {
    return left.compare(right) != Ordering.Less;
}
"#,
    );
}

#[test]
fn test_reify_static_operator_less_than_or_equal_to_ordering_check() {
    // overloaded <= rewrites through compare and ordering greater check
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: int32
}

extension of Counter implements Compare {
    compare(other: Counter): Ordering {
        if (this.value < other.value) {
            return Ordering.Less;
        } else if (this.value > other.value) {
            return Ordering.Greater;
        }

        return Ordering.Equal;
    }
}

function isAtMost(left: Counter, right: Counter): boolean {
    left <= right
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
extension of Counter implements Compare {
    compare(other): Ordering {
        if (this.value < other.value) {
            return Ordering.Less;
        } else if (this.value > other.value) {
            return Ordering.Greater;
        }
        return Ordering.Equal;
    }
}
function isAtMost(left, right): boolean {
    return left.compare(right) != Ordering.Greater;
}
"#,
    );
}

#[test]
fn test_reify_static_operator_greater_than_to_ordering_check() {
    // overloaded > rewrites through compare and ordering greater check
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: int32
}

extension of Counter implements Compare {
    compare(other: Counter): Ordering {
        if (this.value < other.value) {
            return Ordering.Less;
        } else if (this.value > other.value) {
            return Ordering.Greater;
        }

        return Ordering.Equal;
    }
}

function isGreater(left: Counter, right: Counter): boolean {
    left > right
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
extension of Counter implements Compare {
    compare(other): Ordering {
        if (this.value < other.value) {
            return Ordering.Less;
        } else if (this.value > other.value) {
            return Ordering.Greater;
        }
        return Ordering.Equal;
    }
}
function isGreater(left, right): boolean {
    return left.compare(right) == Ordering.Greater;
}
"#,
    );
}

#[test]
fn test_reify_static_unary_operator_to_method_call() {
    // overloaded unary negate rewrites to method call
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: int32
}

extension of Counter implements Negate {
    negate(): Counter {
        Counter { value: -this.value }
    }
}

function flip(counter: Counter): Counter {
    -counter
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
extension of Counter implements Negate {
    negate(): Counter {
        return Counter { value: -this.value };
    }
}
function flip(counter): Counter {
    return counter.negate();
}
"#,
    );
}

#[test]
fn test_reify_static_unary_plus_to_method_call() {
    // overloaded unary plus rewrites to method call
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Counter {
    value: int32
}

extension of Counter implements Plus {
    plus(): Counter {
        Counter { value: this.value }
    }
}

function keep(counter: Counter): Counter {
    +counter
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
extension of Counter implements Plus {
    plus(): Counter {
        return Counter { value: this.value };
    }
}
function keep(counter): Counter {
    return counter.plus();
}
"#,
    );
}

#[test]
fn test_reify_resolution_eliminates_dynamic_resolutions_on_active_call_nodes() {
    // elaborate should leave only static or builtin resolutions on active call expressions
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Cat {
    speak(): string { "meow" }
}

struct Dog {
    speak(): string { "woof" }
}

function greet(pet: Cat | Dog): string {
    pet.speak()
}
"#,
    );
    test.elaborate_module(module_id);
    test.compile_check_clean();
    test.assert_elaborated(
        module_id,
        r#"
struct Cat {
    speak(): string {
        return "meow";
    }
}
struct Dog {
    speak(): string {
        return "woof";
    }
}
function greet(pet): string {
    if (pet is Cat) {
        return pet.speak();
    } else {
        return pet.speak();
    }
}
"#,
    );

    test.with_dir_read(
        module_id,
        |_module, _profile, _dir, tree, symbols, types| {
            let mut static_or_builtin_count = 0usize;

            for expression_id in tree.iter_node_ids_of_type::<Expression>() {
                if !test
                    .compiler
                    .is_node_active(tree, symbols, expression_id.into_any())
                {
                    continue;
                }

                if !matches!(tree.get(expression_id), Expression::Call { .. }) {
                    continue;
                }

                let Some(resolution_id) =
                    types.get_resolution_for_node(expression_id.into_global_any(module_id))
                else {
                    continue;
                };

                match types.get_resolution(resolution_id) {
                    Resolution::Dynamic { .. } => {
                        panic!("found dynamic resolution on active expression {expression_id:?}");
                    }
                    Resolution::Unresolved { .. } => {
                        panic!(
                            "found unresolved resolution on active expression {expression_id:?}"
                        );
                    }
                    Resolution::Static { .. } | Resolution::Builtin { .. } => {
                        static_or_builtin_count += 1;
                    }
                }
            }

            assert_eq!(static_or_builtin_count, 2);
        },
    );
}
