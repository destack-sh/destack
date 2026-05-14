//! Test assertion macros.

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

/// Assert a `StringId` directly against an expected string.
#[macro_export]
macro_rules! assert_string {
    ($program:expr, $id:expr, $expected:expr) => {{
        let got = $program.strings.get($id).to_string();
        assert_eq!(got, $expected, "expected string");
    }};
}

/// Assert a `Name` directly against an expected string.
#[macro_export]
macro_rules! assert_name {
    ($program:expr, $name:expr, $expected:expr) => {{
        let got = $program.strings.get($name.string()).to_string();
        assert_eq!(got, $expected, "expected name");
    }};
}

/// Assert a `Path` directly against an expected string.
#[macro_export]
macro_rules! assert_path {
    ($program:expr, $path:expr, $expected:expr) => {{
        let path_str = $path
            .segments
            .iter()
            .map(|s| $program.strings.get(*s).to_string())
            .collect::<Vec<_>>()
            .join(".");
        assert_eq!(path_str, $expected, "expected path");
    }};
}

/// Assert a reference-like DIR expression directly against an expected string.
#[macro_export]
macro_rules! assert_expression_path {
    ($program:expr, $expr:expr, $expected:expr) => {{
        use destack_dir as dir;

        let got = match $expr {
            dir::Expression::Identifier { name } => $program.strings.get(*name).to_string(),
            dir::Expression::QualifiedReference { path, .. } => path
                .segments
                .iter()
                .map(|segment| $program.strings.get(*segment).to_string())
                .collect::<Vec<_>>()
                .join("."),
            dir::Expression::PrivateIdentifier { name } => {
                format!("#{}", $program.strings.get(*name))
            }
            dir::Expression::This => "this".to_string(),
            dir::Expression::Super => "super".to_string(),
            dir::Expression::ImportMeta => "import.meta".to_string(),
            dir::Expression::NewTarget => "new.target".to_string(),
            other => panic!("expected reference-like expression, got {other:?}"),
        };

        assert_eq!(got, $expected, "expected reference-like expression");
    }};
}

/// Assert that `types.get_type(id)` matches `$pat`.
/// If a body is provided (`=> { ... }`), it runs with the pattern bindings.
///
/// Examples:
/// ```
/// assert_type!(types, type_id, Type::Unknown);
/// assert_type!(types, type_id, Type::Tuple { elements, copy: _ } => {
///     assert_eq!(elements.len(), 2);
/// });
/// // Or with an already-resolved type:
/// assert_type!(*ty_ref, Type::Tuple { elements, copy: _ } => { ... });
/// ```
#[macro_export]
macro_rules! assert_type {
    // `types.get_type(id)` matches a pattern, no body.
    // e.g., `assert_type!(types, type_id, Type::Unknown);`
    ($types:expr, $id:expr, $pat:pat_param) => {{
        #[allow(unreachable_patterns)]
        match $types.get_type($id) {
            $pat => {}
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    // `types.get_type(id)` matches a pattern, then run a block with the bindings.
    // e.g., `assert_type!(types, type_id, Type::Tuple { elements, copy: _ } => { /* ... */ });`
    ($types:expr, $id:expr, $pat:pat_param => $body:block) => {{
        #[allow(unreachable_patterns)]
        match $types.get_type($id) {
            $pat => $body,
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    // Already-resolved type matches a pattern, run a block.
    // e.g., `assert_type!(*ty_ref, Type::Tuple { elements, copy: _ } => { /* ... */ });`
    ($ty:expr, $pat:pat_param => $body:block) => {{
        #[allow(unreachable_patterns)]
        match $ty {
            $pat => $body,
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    // Already-resolved type matches a pattern, no body.
    // e.g., `assert_type!(*ty_ref, Type::Unknown);`
    ($ty:expr, $pat:pat_param) => {{
        #[allow(unreachable_patterns)]
        match $ty {
            $pat => {}
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
}
