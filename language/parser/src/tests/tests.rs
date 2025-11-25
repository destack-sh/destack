use std::sync::Arc;

use dyst_source::{File, FileId, FileType, LanguageOptions, Uri};

use crate::Parser;

/// A test wrapper for Parser.
#[derive(Debug)]
pub(crate) struct TestParser {
    pub file: Arc<File>,
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
        let file = File::from_text(
            file_id,
            "<string>".to_string(),
            Uri::from_string("<string>"),
            FileType::Dyst,
            input.to_string(),
        );
        Self {
            file: Arc::new(file),
            language: options,
        }
    }

    /// Get a Parser for this test.
    pub(crate) fn prepare(&mut self) -> Parser {
        Parser::lex_file(self.file.clone(), self.language)
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

/// Assert a `StringId` directly against an expected string.
#[macro_export]
macro_rules! assert_string {
    ($parser:expr, $id:expr, $expected:expr) => {{
        let got = $parser.strings.get($id).to_string();
        assert_eq!(got, $expected, "expected string");
    }};
}

/// Assert a `Name` directly against an expected string.
#[macro_export]
macro_rules! assert_name {
    ($parser:expr, $name:expr, $expected:expr) => {{
        let got = $parser.strings.get($name.string()).to_string();
        assert_eq!(got, $expected, "expected name");
    }};
}

/// Assert a `Path` directly against an expected string.
#[macro_export]
macro_rules! assert_path {
    ($parser:expr, $path:expr, $expected:expr) => {{
        let path_str = $path
            .segments
            .iter()
            .map(|s| $parser.strings.get(*s).to_string())
            .collect::<Vec<_>>()
            .join(".");
        assert_eq!(path_str, $expected, "expected path");
    }};
}

/// Assert an "Expression::Path(path)" directly against an expected string.
#[macro_export]
macro_rules! assert_expression_path {
    ($parser:expr, $expr:expr, $expected:expr) => {{
        match $expr {
            ::dyst_ast::Expression::Path {
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
    use std::sync::Arc;

    use dyst_ast::{BlockFormat, TokenType};
    use dyst_source::{DiagnosticCollector, File, FileId, FileType, LanguageOptions, Uri, glob};

    use crate::Parser;

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
        let ds_files = glob(&format!("{workspace_root}/**/*.ds"));

        let mut files: HashMap<FileId, Arc<File>> = HashMap::new();
        let diagnostics: DiagnosticCollector = DiagnosticCollector::new();
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
            let file = File::from_text(
                file_id,
                name,
                Uri::from_string(path),
                FileType::Dyst,
                fs::read_to_string(ds_file).unwrap(),
            );
            let file = Arc::new(file);
            files.insert(file_id, file.clone());
            let mut parser = Parser::lex_file(file.clone(), language);
            let _ = parser.with_recovery(
                parser.mark(),
                |parser| parser.eat_block_body(BlockFormat::Implicit),
                Vec::new(),
                TokenType::End,
            );
            diagnostics.merge_from(&parser.diagnostics);
        }

        // dump diagnostics
        if !diagnostics.is_empty() {
            for diagnostic in diagnostics.iter() {
                let file = files.get(&diagnostic.file_id).unwrap();
                let annotated = dyst_source::annotate_file(
                    file,
                    &diagnostic.primary_span,
                    dyst_source::AnnotateOptions::default(),
                );
                let diagnostic_header = format!("{}: {}", diagnostic.code, diagnostic.message);
                eprintln!("{diagnostic_header}");
                eprintln!("{annotated}");
            }
        }
    }
}
