use dyst_language_source::{SourceFile, SourceId};

use crate::Parser;

/// A test wrapper for Parser.
#[derive(Debug)]
pub(crate) struct TestParser {
    pub source: SourceFile,
}

impl TestParser {
    pub(crate) fn new(input: &str) -> Self {
        let source_id = SourceId::new(0);
        let source = SourceFile::new(source_id, input.to_string());
        Self { source }
    }

    /// Get a Parser for this test.
    pub(crate) fn parser(&self) -> Parser<'_> {
        Parser::from_file(&self.source, self.source.path)
    }
}

/// Assert that `tree.get(id)` matches `$pat`.
/// If a body is provided (`=> { ... }`), it runs with the pattern bindings.
///
/// Examples:
/// ```
/// assert_node!(tree, id, Pattern::Wildcard);
/// assert_node!(tree, id, Pattern::Pointer { mutability, target } => {
///     assert_eq!(*mutability, Mutability::Mutable);
///     assert_node!(tree, *target, Pattern::Wildcard);
/// });
/// ```
#[macro_export]
macro_rules! assert_node {
    // `tree.get(id)` matches a pattern, no body.
    // e.g., `assert_node!(tree, id, Pattern::Wildcard);`
    ($tree:expr, $id:expr, $pat:pat_param) => {{
        #[allow(unreachable_patterns)]
        match $tree.get($id) {
            $pat => {}
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    // `tree.get(id)` matches a pattern, then run a block with the bindings.
    // e.g., `assert_node!(tree, id, Pattern::Pointer { mutability, target } => { /* ... */ });`
    ($tree:expr, $id:expr, $pat:pat_param => $body:block) => {{
        #[allow(unreachable_patterns)]
        match $tree.get($id) {
            $pat => $body,
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    // Already-resolved node matches a pattern, run a block.
    // e.g., `assert_node!(node_ref, Pattern::Tuple { fields } => { /* ... */ });`
    ($node:expr, $pat:pat_param => $body:block) => {{
        #[allow(unreachable_patterns)]
        match $node {
            $pat => $body,
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    // Already-resolved node matches a pattern, no body.
    // e.g., `assert_node!(node_ref, Pattern::Rest);`
    ($node:expr, $pat:pat_param) => {{
        #[allow(unreachable_patterns)]
        match $node {
            $pat => {}
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
}

/// Run an arbitrary predicate/closure on the resolved node.
/// The closure receives `&T` and must return `bool`.
///
/// Examples:
/// ```
/// assert_check!(tree, id, |n| matches!(n, Pattern::Wildcard));
/// assert_check!(tree, id, |n| compute_ok(n), "bad node: {n:?}");
/// ```
#[macro_export]
macro_rules! assert_check {
    // Predicate + custom panic message.
    // e.g., `assert_check!(tree, id, |n| predicate(n), "msg {}", x);`
    ($tree:expr, $id:expr, $pred:expr, $($msg:tt)*) => {{
        let __n = $tree.get($id);
        if !($pred)(__n) {
            panic!($($msg)*);
        }
    }};
    // Predicate with default panic message.
    // e.g., `assert_check!(tree, id, |n| predicate(n));`
    ($tree:expr, $id:expr, $pred:expr) => {{
        let __n = $tree.get($id);
        if !($pred)(__n) {
            panic!("assert_check predicate failed for `{id}`", id = $id);
        }
    }};
}

/// Assert a `ScalarLiteral::Integer` equals an exact value (ignores `IntType` details).
#[macro_export]
macro_rules! assert_int {
    // Exact integer value.
    ($tree:expr, $id:expr, $expected:expr) => {{
        $crate::assert_node!($tree, $id, $crate::ScalarLiteral::Integer(n, _) => {
            assert_eq!(
							*n,
							$expected,
							"expected integer literal",
						);
        });
    }};
}

/// Assert a `ScalarLiteral::Float` equals an exact value with `==`.
#[macro_export]
macro_rules! assert_float {
    // Exact float value (bitwise equal).
    ($tree:expr, $id:expr, $expected:expr) => {{
        $crate::assert_node!($tree, $id, $crate::ScalarLiteral::Float(f, _) => {
            assert_eq!(
							*f,
							$expected,
							"expected float literal",
						);
        });
    }};
}

/// Assert a `ScalarLiteral::Boolean`.
#[macro_export]
macro_rules! assert_bool {
    // Exact bool.
    ($tree:expr, $id:expr, $expected:expr) => {{
        $crate::assert_node!($tree, $id, $crate::ScalarLiteral::Boolean(b) => {
            assert_eq!(
							*b,
							$expected,
							"expected bool literal",
						);
        });
    }};
}

/// Assert a `ScalarLiteral::Character`.
#[macro_export]
macro_rules! assert_char {
    // Exact char.
    ($tree:expr, $id:expr, $expected:expr) => {{
        $crate::assert_node!($tree, $id, $crate::ScalarLiteral::Character(c) => {
            assert_eq!(
							*c,
							$expected,
							"expected char literal",
						);
        });
    }};
}

/// Assert a `ScalarLiteral::String`.
#[macro_export]
macro_rules! assert_string {
    // Resolve StringId to string and compare to expected.
    ($tree:expr, $id:expr, $expected:expr, using $resolve:expr) => {{
        $crate::assert_node!($tree, $id, $crate::ScalarLiteral::String(s) => {
            let got = ($resolve)(*s);
            assert_eq!(got, $expected, "expected string literal");
        });
    }};
}

/// Assert an `Expression::Path { path }`.
#[macro_export]
macro_rules! assert_path {
    // Resolve PathId to string and compare to expected.
    ($tree:expr, $expr_id:expr, $expected:expr, using $resolve:expr) => {{
        $crate::assert_node!($tree, $expr_id, $crate::Expression::Path { path } => {
            let got = ($resolve)(*path);
            assert_eq!(got, $expected, "expected path");
        });
    }};
}
