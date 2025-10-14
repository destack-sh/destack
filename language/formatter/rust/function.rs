#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_function_lambda_empty() {
        assert_format!("() => void", "() => void", |p| p.eat_function(None, None));
    }

    #[test]
    fn test_format_function_lambda_with_parameters() {
        assert_format!("(a: int32) => a > 2", "(a: int32) => a > 2", |p| p
            .eat_function(None, None));
    }

    #[test]
    fn test_format_function_simple() {
        assert_format!("function foo() {}", "function foo() { }", |p| p
            .eat_function(None, None));
    }

    #[test]
    fn test_format_function_with_parameters() {
        assert_format!(
            "function bar(x: int32, y: boolean) {}",
            "function bar(x: int32, y: boolean) { }",
            |p| p.eat_function(None, None)
        );
    }

    #[test]
    fn test_format_function_with_parameters_overflow() {
        assert_format!(
            "function bar(x: int32, y: boolean, z: string) {}",
            "function bar(\n\tx: int32\n\ty: boolean\n\tz: string\n) { }",
            |p| p.eat_function(None, None),
            DystFormatOptions::default_tab_with_line_width(40)
        );
    }

    #[test]
    fn test_format_function_with_return_type() {
        assert_format!(
            "function baz() => int32 {}",
            "function baz() => int32 { }",
            |p| p.eat_function(None, None)
        );
    }

    #[test]
    fn test_format_function_with_static_parameters() {
        assert_format!(
            "function generic<T, U>() {}",
            "function generic<T, U>() { }",
            |p| p.eat_function(None, None)
        );
    }

    #[test]
    fn test_format_function_with_mutable_self_reference() {
        assert_format!(
            "function mutate(&var(x, y) self) {}",
            "function mutate(&var(x, y) self) { }",
            |p| p.eat_function(None, None)
        );
    }

    #[test]
    fn test_format_function_with_with_clause_overflow() {
        assert_format!(
            "function foo() with Time, Place, Something, Foo, Baz {}",
            "function foo() with (\n\tTime\n\tPlace\n\tSomething\n\tFoo\n\tBaz\n) { }",
            |p| p.eat_function(None, None),
            DystFormatOptions::default_tab_with_line_width(40)
        );
    }

    #[test]
    fn test_format_function_declaration() {
        assert_format!(
            "function external() => int32",
            "function external() => int32",
            |p| p.eat_function(None, None)
        );
    }

    #[test]
    fn test_format_function_static_runtime() {
        assert_format!("function @comptime() {}", "function @comptime() { }", |p| p
            .eat_function(None, None));
    }

    #[test]
    fn test_format_function_with_with_and_return() {
        assert_format!(
            "function foo() => int32 with Disk {}",
            "function foo() => int32 with Disk { }",
            |p| p.eat_function(None, None)
        );
    }

    #[test]
    fn test_format_function_with_self() {
        let source = r"function init(capacity: int32) => Self {
    Self {
        map: Map.new(capacity)
        queue: Queue.new(capacity)
        capacity: capacity
        somethingElse: something
        moreStuff: bar()
        evenMoreStuff: foo()
        moreMoreMoreStuff: baz()
    }
}";
        assert_format!(source, source, |p| p.eat_function(None, None));
    }
}
