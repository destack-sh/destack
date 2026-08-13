use crate::rules::declare_lint_stub;

declare_lint_stub! {
    /// Require const for bindings never reassigned after initialization.
    pub PREFER_CONST {
        id: "prefer-const",
        summary: "Require const for bindings never reassigned after initialization",
        explanation: r#"
A `let` binding that is never reassigned permits a write the function does not perform.
Instead, you SHOULD declare the binding with `const`.

`const` prevents reassignment of the binding and still permits mutation through the stored value's API.
"#,
        example: {
            reported: r#"
function identity(value: int32): int32 {
    let result = value;
    return result;
}
"#,
            accepted: r#"
function identity(value: int32): int32 {
    const result = value;
    return result;
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Validate the canonical lint example.
    #[ignore]
    #[test]
    fn test_lint_example() {
        TestSession::assert_example(&PREFER_CONST);
    }

    /// Accept a binding that is reassigned after initialization.
    #[ignore]
    #[test]
    fn test_accepts_reassigned_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function replace(value: int32): int32 {
    let result = value;
    result = 1;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a binding whose storage is borrowed mutably.
    #[ignore]
    #[test]
    fn test_accepts_mutably_borrowed_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function replace(): int32 {
    let value: int32 = 0;
    const borrowed = &value;
    *borrowed = 1;
    return value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a binding whose storage is only borrowed read-only.
    #[ignore]
    #[test]
    fn test_replaces_readonly_borrowed_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function read(): int32 {
    let value: int32 = 0;
    const borrowed = &readonly value;
    return *borrowed;
}
"#,
        );

        session.assert_fixes(
            r#"
function read(): int32 {
    const value: int32 = 0;
    const borrowed = &readonly value;
    return *borrowed;
}
"#,
        );
    }

    /// Accept a binding implicitly borrowed with mutable access.
    #[ignore]
    #[test]
    fn test_accepts_implicitly_borrowed_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
declare function replace(value: &int32): void;
function update(): int32 {
    let value: int32 = 0;
    replace(value);
    return value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a binding implicitly borrowed with read-only access.
    #[ignore]
    #[test]
    fn test_replaces_implicitly_readonly_borrowed_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
declare function read(value: &readonly int32): int32;
function inspect(): int32 {
    let value: int32 = 0;
    return read(value);
}
"#,
        );

        session.assert_fixes(
            r#"
declare function read(value: &readonly int32): int32;
function inspect(): int32 {
    const value: int32 = 0;
    return read(value);
}
"#,
        );
    }

    /// Report a separately initialized binding without offering an unsafe rewrite.
    #[ignore]
    #[test]
    fn test_reports_separately_initialized_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function identity(value: int32): int32 {
    let result: int32;
    result = value;
    return result;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-const]: binding is never reassigned
 ──▶ main.ds:2:9
  │
1 │ function identity(value: int32): int32 {
2 │     let result: int32;
  │         ^^^^^^
3 │     result = value;
4 │     return result;
  │
"#,
        );
    }

    /// Accept a separately initialized binding with another write.
    #[ignore]
    #[test]
    fn test_accepts_separately_reassigned_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function replace(value: int32): int32 {
    let result: int32;
    result = value;
    result = 1;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a binding initialized only inside a nested block.
    #[ignore]
    #[test]
    fn test_accepts_nested_initialization() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function initialize(active: boolean): void {
    let result: int32;
    if (active) {
        result = 1;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Ignore mutation behind a binding because const only prevents rebinding.
    #[ignore]
    #[test]
    fn test_replaces_binding_with_mutated_value() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
class Counter {
    value: int32 = 0;

    increment(): void {
        this.value += 1;
    }
}
function increment(counter: Counter): Counter {
    let result = counter;
    result.increment();
    return result;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-const]: binding is never reassigned
  ──▶ main.ds:9:5
   │
 7 │ }
 8 │ function increment(counter: Counter): Counter {
 9 │     let result = counter;
   │     ^^^
10 │     result.increment();
11 │     return result;
   │

 = fix: declare the binding with const
--- a/main.ds
+++ b/main.ds

    8│ function increment(counter: Counter): Counter {
-   9│     let result = counter;
+   9│     const result = counter;
"#,
        );
        session.assert_fixes(
            r#"
class Counter {
    value: int32 = 0;

    increment(): void {
        this.value += 1;
    }
}
function increment(counter: Counter): Counter {
    const result = counter;
    result.increment();
    return result;
}
"#,
        );
    }

    /// Replace an initialized destructuring declaration when no binding is reassigned.
    #[ignore]
    #[test]
    fn test_replaces_destructured_bindings() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function sum(point: { x: int32; y: int32 }): int32 {
    let { x, y } = point;
    return x + y;
}
"#,
        );

        session.assert_fixes(
            r#"
function sum(point: { x: int32; y: int32 }): int32 {
    const { x, y } = point;
    return x + y;
}
"#,
        );
    }

    /// Report only the constant binding in a partially reassigned destructuring declaration.
    #[ignore]
    #[test]
    fn test_reports_constant_destructured_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function sum(point: { x: int32; y: int32 }): int32 {
    let { x, y } = point;
    x += 1;
    return x + y;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-const]: binding is never reassigned
 ──▶ main.ds:2:14
  │
1 │ function sum(point: { x: int32; y: int32 }): int32 {
2 │     let { x, y } = point;
  │              ^
3 │     x += 1;
4 │     return x + y;
  │
"#,
        );
    }
}
