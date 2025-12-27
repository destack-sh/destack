#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};
    use destack_ast::DeclarationDescriptor;

    #[test]
    fn test_format_function_lambda_empty() {
        assert_format!("() => void", "() => void", |p| p.eat_function(
            p.mark(),
            DeclarationDescriptor::default(),
            false,
            false
        ));
    }

    #[test]
    fn test_format_function_lambda_with_parameters() {
        assert_format!("(a: int32) => a > 2", "(a: int32) => a > 2", |p| p
            .eat_function(p.mark(), DeclarationDescriptor::default(), false, false));
    }

    #[test]
    fn test_format_function_simple() {
        assert_format!("function foo() {}", "function foo() { }", |p| p
            .eat_function(p.mark(), DeclarationDescriptor::default(), false, false));
    }

    #[test]
    fn test_format_function_with_parameters() {
        assert_format!(
            "function bar(x: int32, y: boolean) {}",
            "function bar(x: int32, y: boolean) { }",
            |p| p.eat_function(p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }

    #[test]
    fn test_format_function_with_parameters_overflow() {
        assert_format!(
            "function bar(x: int32, y: boolean, z: string) {}",
            "function bar(\n\tx: int32,\n\ty: boolean,\n\tz: string,\n) { }",
            |p| p.eat_function(p.mark(), DeclarationDescriptor::default(), false, false),
            DestackFormatOptions::default_tab_with_line_width(40)
        );
    }

    #[test]
    fn test_format_function_with_return_type() {
        assert_format!(
            "function baz(): int32 {}",
            "function baz(): int32 { }",
            |p| p.eat_function(p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }

    #[test]
    fn test_format_function_with_static_parameters() {
        assert_format!(
            "function generic<T, U>() {}",
            "function generic<T, U>() { }",
            |p| p.eat_function(p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }

    #[test]
    fn test_format_function_declaration() {
        assert_format!(
            "function external(): int32",
            "function external(): int32",
            |p| p.eat_function(p.mark(), DeclarationDescriptor::default(), false, false)
        );
    }

    #[test]
    fn test_format_function_with_self_parameter() {
        let source = r"function foo(self: int32): void";
        assert_format!(source, source, |p| p.eat_function(
            p.mark(),
            DeclarationDescriptor::default(),
            false,
            false
        ));
    }

    #[test]
    fn test_format_function_with_this_parameter() {
        let source = r"function foo(this: int32): void";
        assert_format!(source, source, |p| p.eat_function(
            p.mark(),
            DeclarationDescriptor::default(),
            false,
            false
        ));
    }

    /// Formats return type predicates with explicit targets.
    #[test]
    fn test_format_function_with_type_predicate() {
        let source = r"function assertFoo(value: Foo): asserts value is Foo";
        assert_format!(source, source, |p| p.eat_function(
            p.mark(),
            DeclarationDescriptor::default(),
            false,
            false
        ));
    }

    /// Formats return type predicates without targets.
    #[test]
    fn test_format_function_with_type_predicate_asserts_value() {
        let source = r"function assertFoo(value: Foo): asserts value";
        assert_format!(source, source, |p| p.eat_function(
            p.mark(),
            DeclarationDescriptor::default(),
            false,
            false
        ));
    }

    /// Formats return type predicates with a this subject.
    #[test]
    fn test_format_function_with_type_predicate_this() {
        let source = r"function assertFoo(this: Foo): asserts this is Foo";
        assert_format!(source, source, |p| p.eat_function(
            p.mark(),
            DeclarationDescriptor::default(),
            false,
            false
        ));
    }

    /// Formats return type predicates with a this subject and no target.
    #[test]
    fn test_format_function_with_type_predicate_this_no_target() {
        let source = r"function assertFoo(this: Foo): asserts this";
        assert_format!(source, source, |p| p.eat_function(
            p.mark(),
            DeclarationDescriptor::default(),
            false,
            false
        ));
    }

    #[test]
    fn test_format_function_with_self_return_type() {
        let source = r"function init(capacity: int32): Self {
    Self {
        map: Map.new(capacity),
        queue: Queue.new(capacity),
        capacity: capacity,
        somethingElse: something,
        moreStuff: bar(),
        evenMoreStuff: foo(),
        moreMoreMoreStuff: baz(),
    }
}";
        assert_format!(source, source, |p| p.eat_function(
            p.mark(),
            DeclarationDescriptor::default(),
            false,
            false
        ));
    }
}
