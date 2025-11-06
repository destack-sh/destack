use dyst_source::{DiagnosticCollector, File, FileId, FileType, LanguageOptions, Uri};

use crate::Parser;

/// A test wrapper for Parser.
#[derive(Debug)]
pub(crate) struct TestParser {
    pub source: File,
    pub diagnostics: DiagnosticCollector,
    pub language: LanguageOptions,
}

impl TestParser {
    /// Create a new TestParser with default options.
    pub(crate) fn new(input: &str) -> Self {
        Self::new_with_options(input, LanguageOptions::default())
    }

    /// Create a new TestParser with custom options.
    pub(crate) fn new_with_options(input: &str, options: LanguageOptions) -> Self {
        let file_id = FileId::new(0);
        let source = File::from_string(
            file_id,
            "<string>".to_string(),
            Uri::from_string("<string>"),
            FileType::Dyst,
            input.to_string(),
        );
        Self {
            source,
            diagnostics: DiagnosticCollector::new(),
            language: options,
        }
    }

    /// Get a Parser for this test.
    pub(crate) fn prepare(&mut self) -> Parser<'_> {
        Parser::prepare(&self.source, self.language, &mut self.diagnostics)
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
        $crate::assert_node!($tree, $id, $crate::ScalarLiteral::Integer(n) => {
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
    // Exact float value (elementwise equal).
    ($tree:expr, $id:expr, $expected:expr) => {{
        $crate::assert_node!($tree, $id, $crate::ScalarLiteral::Float(f) => {
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

/// Assert a `StringId` directly against an expected string.
#[macro_export]
macro_rules! assert_string {
    ($parser:expr, $id:expr, $expected:expr) => {{
        let got = $parser.get_string($id);
        assert_eq!(got, $expected, "expected string");
    }};
}

/// Assert a `Name` directly against an expected string.
#[macro_export]
macro_rules! assert_name {
    ($parser:expr, $name:expr, $expected:expr) => {{
        let got = $parser.get_string($name.string());
        assert_eq!(got, $expected, "expected name");
    }};
}

/// Assert a `ScalarLiteral::String`.
#[macro_export]
macro_rules! assert_lit_string {
    ($parser:expr, $id:expr, $expected:expr) => {{
        match $id {
            $crate::ScalarLiteral::String(s) => {
                let got = $parser.get_string(*s);
                assert_eq!(got, $expected, "expected string");
            }
            other => panic!("expected ScalarLiteral::String, got {other:?}"),
        }
    }};
}

/// Assert a `ScalarLiteral::Integer`.
#[macro_export]
macro_rules! assert_lit_int {
    ($parser:expr, $id:expr, $expected:expr) => {{
        match $id {
            $crate::ScalarLiteral::Integer(n) => {
                assert_eq!(*n, $expected, "expected integer");
            }
            other => panic!("expected ScalarLiteral::Integer, got {other:?}"),
        }
    }};
}

/// Assert a `Path` directly against an expected string.
#[macro_export]
macro_rules! assert_path {
    ($parser:expr, $path:expr, $expected:expr) => {{
        let path_str = $path
            .segments
            .iter()
            .map(|s| $parser.get_string(*s))
            .collect::<Vec<_>>()
            .join(".");
        assert_eq!(path_str, $expected, "expected path");
    }};
}

/// Assert an "Expression::Path(path)" directly against an expected string.
#[macro_export]
macro_rules! assert_expr_path {
    ($parser:expr, $expr:expr, $expected:expr) => {{
        match $expr {
            $crate::Expression::Path {
                path,
                static_arguments: _,
            } => {
                assert_path!($parser, *path, $expected);
            }
            other => panic!("expected Expression::Path, got {other:?}"),
        }
    }};
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    use crate::TokenType;
    use destack_file::glob;
    use dyst_source::{DiagnosticCollector, File, FileId, FileType, LanguageOptions, Uri};

    use crate::{BlockFormat, Parser};

    #[test]
    #[ignore = "slow"]
    fn test_parse_every_ds_file() {
        // find workspace root by walking up until we find a known repo marker
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root_path = manifest_dir
            .ancestors()
            .find(|p| p.join("version.txt").exists())
            .unwrap_or(&manifest_dir)
            .to_path_buf();
        let workspace_root = workspace_root_path.to_string_lossy().into_owned();

        // glob all .ds files under the workspace root
        let ds_files = glob::glob(&format!("{workspace_root}/**/*.ds"));

        let mut sources: HashMap<FileId, File> = HashMap::new();
        let mut diagnostics = DiagnosticCollector::new();
        let language = LanguageOptions::default();

        // parse every ds file
        for (i, ds_file) in ds_files.iter().enumerate() {
            let file_id = FileId::new(i as u32);
            let name = ds_file
                .iter()
                .next_back()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or("<file>".to_string());
            let path = ds_file.to_string_lossy().into_owned();
            let source = File::from_string(
                file_id,
                name,
                Uri::from_string(path),
                FileType::Dyst,
                fs::read_to_string(ds_file).unwrap(),
            );
            sources.insert(file_id, source);
            let mut parser =
                Parser::prepare(sources.get(&file_id).unwrap(), language, &mut diagnostics);
            let _ = parser.with_recovery(
                parser.mark(),
                |parser| parser.eat_block_body(BlockFormat::Implicit),
                Vec::new(),
                TokenType::End,
            );
        }

        // dump diagnostics
        if !diagnostics.is_empty() {
            for diagnostic in diagnostics.iter() {
                let source = sources.get(&diagnostic.source).unwrap();
                let annotated = dyst_source::annotate_source(
                    source,
                    &diagnostic.primary_span,
                    dyst_source::AnnotateOptions {
                        max_line_width: 100,
                        prefix_lines: 1,
                        suffix_lines: 1,
                        use_color: false,
                    },
                );
                let diagnostic_header = format!("{}: {}", diagnostic.code, diagnostic.message);
                eprintln!("{diagnostic_header}");
                eprintln!("{annotated}");
            }
        }
    }
}
