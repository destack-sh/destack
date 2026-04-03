use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use destack_source::{File, FileId, FileType, Uri};

use crate::Namespace;
use crate::parse::builder::HtmlBuilder;
use crate::parse::parser::{ParseContextName, Parser, ParserOptions};

const HTML5LIB_TREE_DIRECTORY: &str = "../test/fixtures/html/html5lib/tree-construction";

/// One parsed html5lib tree-construction case.
#[derive(Debug, Default)]
struct Html5libTreeCase {
    /// The parsed named sections.
    sections: BTreeMap<String, String>,
    /// The current section key while scanning.
    current_key: Option<String>,
    /// The current section value while scanning.
    current_value: String,
}

impl Html5libTreeCase {
    /// Finish the current section value.
    fn finish_value(&mut self) {
        let Some(key) = self.current_key.take() else {
            return;
        };

        let previous = self
            .sections
            .insert(key, std::mem::take(&mut self.current_value));
        assert!(previous.is_none(), "duplicate tree test section");
    }

    /// Return whether this case is empty.
    fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// Return the HTML source for this case.
    fn source(&self) -> Option<String> {
        let mut source = self.sections.get("data")?.clone();

        if source.ends_with('\n') {
            source.pop();
        }

        Some(source)
    }

    /// Return the expected html5lib tree snapshot.
    fn expected(&self) -> Option<String> {
        Some(
            self.sections
                .get("document")?
                .trim_end_matches('\n')
                .to_string(),
        )
    }

    /// Return the fragment context when this case is a fragment parse.
    fn context(&self) -> Option<ParseContextName> {
        let context = self.sections.get("document-fragment")?;
        let context = context.trim_end_matches('\n');

        Some(Html5libTreeSuite::parse_context_name(context))
    }

    /// Return the scripting modes this case should run under.
    fn scripting_modes(&self) -> &'static [bool] {
        if self.sections.contains_key("script-on") {
            &[true]
        } else if self.sections.contains_key("script-off") {
            &[false]
        } else {
            &[false, true]
        }
    }
}

/// One html5lib tree-construction harness.
#[derive(Debug)]
struct Html5libTreeSuite {
    /// The fixture root directory.
    root: PathBuf,
}

impl Html5libTreeSuite {
    /// Create one tree-construction suite.
    fn new() -> Self {
        Self {
            root: Path::new(env!("CARGO_MANIFEST_DIR")).join(HTML5LIB_TREE_DIRECTORY),
        }
    }

    /// Match the shared html5lib tree-construction corpus.
    fn run(&self) {
        let mut paths = Vec::new();

        self.collect_tree_files(&self.root, &mut paths);
        paths.sort();

        for path in paths {
            self.run_fixture_file(&path);
        }
    }

    /// Collect the sorted `.dat` files in one directory tree.
    fn collect_tree_files(&self, root: &Path, files: &mut Vec<PathBuf>) {
        let entries = fs::read_dir(root)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()));

        for entry in entries {
            let entry = entry.unwrap_or_else(|error| panic!("failed to read entry: {error}"));
            let path = entry.path();

            // nested directories
            if path.is_dir() {
                self.collect_tree_files(&path, files);
                continue;
            }

            // `.dat` fixtures
            if path.extension().and_then(|value| value.to_str()) != Some("dat") {
                continue;
            }

            files.push(path);
        }
    }

    /// Run one fixture file.
    fn run_fixture_file(&self, path: &Path) {
        let file = fs::File::open(path)
            .unwrap_or_else(|error| panic!("failed to open {}: {error}", path.display()));
        let reader = BufReader::new(file);
        let cases = self
            .parse_tree_cases(reader.lines())
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

        for (index, case) in cases.into_iter().enumerate() {
            self.run_case(path, index, case);
        }
    }

    /// Parse one html5lib tree-construction fixture file.
    fn parse_tree_cases<B>(
        &self,
        lines: std::io::Lines<B>,
    ) -> std::io::Result<Vec<Html5libTreeCase>>
    where
        B: BufRead,
    {
        let mut cases = Vec::new();
        let mut case = Html5libTreeCase::default();

        // line scan
        for line in lines {
            let line = line?;

            // section header
            if let Some(rest) = line.strip_prefix('#') {
                case.finish_value();

                if line == "#data" {
                    self.push_tree_case(&mut cases, &mut case);
                }

                case.current_key = Some(rest.to_string());
                continue;
            }

            // section body
            case.current_value.push_str(&line);
            case.current_value.push('\n');
        }

        case.finish_value();
        self.push_tree_case(&mut cases, &mut case);

        Ok(cases)
    }

    /// Push one finished parsed case when it is non-empty.
    fn push_tree_case(&self, cases: &mut Vec<Html5libTreeCase>, case: &mut Html5libTreeCase) {
        if case.is_empty() {
            return;
        }

        cases.push(std::mem::take(case));
    }

    /// Run one parsed tree-construction case.
    fn run_case(&self, path: &Path, index: usize, case: Html5libTreeCase) {
        let source = case
            .source()
            .unwrap_or_else(|| panic!("missing #data in {}", path.display()));
        let expected = case
            .expected()
            .unwrap_or_else(|| panic!("missing #document in {}", path.display()));
        let context = case.context();
        let file = self.fixture_file(path, &source);

        // scripting modes
        for &scripting_enabled in case.scripting_modes() {
            let actual = if let Some(context) = &context {
                self.parse_fragment_tree(&file, &source, context, scripting_enabled)
            } else {
                self.parse_document_tree(&file, &source, scripting_enabled)
            };

            // full snapshot equality
            assert_eq!(
                actual,
                expected,
                "tree-construction mismatch in {} case {} with scripting {}\ninput:\n{}\n",
                path.display(),
                index,
                scripting_enabled,
                source,
            );
        }
    }

    /// Lower one html5lib fragment context name.
    fn parse_context_name(context: &str) -> ParseContextName {
        if let Some(local) = context.strip_prefix("svg ") {
            return ParseContextName {
                prefix: None,
                namespace: Namespace::Svg,
                local: local.to_string(),
            };
        }

        if let Some(local) = context.strip_prefix("math ") {
            return ParseContextName {
                prefix: None,
                namespace: Namespace::MathMl,
                local: local.to_string(),
            };
        }

        ParseContextName {
            prefix: None,
            namespace: Namespace::Html,
            local: context.to_string(),
        }
    }

    /// Create one fixture file wrapper for parser entrypoints.
    fn fixture_file(&self, path: &Path, source: &str) -> File {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("<html5lib>");
        let uri = Uri::from_string(path.display().to_string());

        File::from_text(
            FileId::new(0),
            name.to_string(),
            uri,
            Some(path.to_path_buf()),
            FileType::Html,
            source.to_string(),
        )
    }

    /// Parse one HTML document into one html5lib-style tree snapshot.
    fn parse_document_tree(&self, file: &File, source: &str, scripting_enabled: bool) -> String {
        let builder = HtmlBuilder::new(file, source);
        let options = ParserOptions {
            scripting_enabled,
            ..Default::default()
        };

        Parser::new_with_options(builder, source, options).parse_tree_construction()
    }

    /// Parse one HTML fragment into one html5lib-style tree snapshot.
    fn parse_fragment_tree(
        &self,
        file: &File,
        source: &str,
        context: &ParseContextName,
        scripting_enabled: bool,
    ) -> String {
        let options = ParserOptions {
            scripting_enabled,
            ..Default::default()
        };

        Parser::new_fragment_with_options(file, source, context, options)
            .parse_fragment_tree_construction()
    }
}

/// Match the html5lib tree-construction corpus.
#[test]
fn test_match_html5lib_tree_construction() {
    Html5libTreeSuite::new().run();
}
