#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

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

extension for Cat {
    speak(): string { "meow" }
}

extension for Dog {
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

extension for Cat {
    speak(): string {
        return "meow";
    }
}

extension for Dog {
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
}
