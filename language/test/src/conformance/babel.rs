use std::path::{Path, PathBuf};

use destack_source::FileType;

use super::parse::{ParseOptions, ParseOutcome, TestArea, parse_file};
use crate::conformance::{
    Case, CaseOutcome, ConformanceDriver, ConformanceSuiteResult, run_conformance_driver,
    suite_fixtures_dir, suite_tests_dir,
};
use crate::core::RunOptions;

// pinned version of babel parser tests
const BABEL_VERSION: &str = "7.26";
const BABEL_COMMIT: &str = "b8ef443"; // short sha

/// Babel parser conformance suite.
#[derive(Debug, Clone)]
pub struct BabelSuite {
    tests_dir: PathBuf,
    suite_dir: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct BabelOptions {
    source_type: Option<String>,
    plugins: Vec<String>,
    disallow_ambiguous_jsx_like: bool,
    has_throws: bool,
}

impl BabelOptions {
    /// Parse Babel options from JSON content.
    fn from_content(content: &str) -> Option<Self> {
        let value = serde_json::from_str::<serde_json::Value>(content).ok()?;
        let object = value.as_object()?;

        let source_type = object
            .get("sourceType")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());

        let plugins = object
            .get("plugins")
            .and_then(|value| value.as_array())
            .map(|plugins| {
                plugins
                    .iter()
                    .filter_map(Self::plugin_name)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut disallow_ambiguous_jsx_like = object
            .get("disallowAmbiguousJSXLike")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        if let Some(plugin_values) = object.get("plugins").and_then(|value| value.as_array()) {
            for plugin in plugin_values {
                let Some(items) = plugin.as_array() else {
                    continue;
                };
                let Some(name) = items.first().and_then(|value| value.as_str()) else {
                    continue;
                };
                if name != "typescript" {
                    continue;
                }
                let Some(options) = items.get(1).and_then(|value| value.as_object()) else {
                    continue;
                };
                if let Some(value) = options
                    .get("disallowAmbiguousJSXLike")
                    .and_then(|value| value.as_bool())
                {
                    disallow_ambiguous_jsx_like |= value;
                }
            }
        }

        let has_throws = object.contains_key("throws");

        Some(Self {
            source_type,
            plugins,
            disallow_ambiguous_jsx_like,
            has_throws,
        })
    }

    /// Extract a plugin name from a plugin entry.
    fn plugin_name(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::String(name) => Some(name.to_string()),
            serde_json::Value::Array(items) => items
                .first()
                .and_then(|value| value.as_str())
                .map(|value| value.to_string()),
            _ => None,
        }
    }

    /// Return true when the plugin list contains a plugin name.
    fn has_plugin(&self, plugin: &str) -> bool {
        self.plugins.iter().any(|name| name == plugin)
    }
}

impl BabelSuite {
    /// Create one Babel conformance suite.
    pub fn new() -> Self {
        let suite_dir = suite_fixtures_dir("babel");
        let tests_dir = suite_tests_dir("babel");
        Self {
            tests_dir,
            suite_dir,
        }
    }

    fn discover_recursive(&self, directory: &Path, prefix: &str) -> Vec<Case> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // check if this directory is a test case (has input.*)
                    let input_info = self.get_input_file(&path);

                    // if this directory is a test case, register it
                    if let Some((_, file_type)) = input_info {
                        // skip script-mode tests (we only support strict module mode)
                        if self.is_script_mode(&path) {
                            continue;
                        }
                        let directory_name = path.file_name().unwrap().to_string_lossy();
                        let name = if prefix.is_empty() {
                            directory_name.to_string()
                        } else {
                            format!("{prefix}/{directory_name}")
                        };
                        let expect_error = self.should_throw(&path);
                        let test = if expect_error {
                            Case::fail(name, file_type)
                        } else {
                            Case::pass(name, file_type)
                        };
                        tests.push(test);
                    } else {
                        // otherwise recurse into the subdirectory
                        let directory_name = path.file_name().unwrap().to_string_lossy();
                        let new_prefix = if prefix.is_empty() {
                            directory_name.to_string()
                        } else {
                            format!("{prefix}/{directory_name}")
                        };
                        tests.extend(self.discover_recursive(&path, &new_prefix));
                    }
                }
            }
        }

        tests.sort_by(|a, b| a.name.cmp(&b.name));
        tests
    }

    /// Check if a test uses script mode (sourceType: "script").
    /// We only support strict module mode, so we skip these tests.
    fn is_script_mode(&self, test_dir: &Path) -> bool {
        let Some(options) = Self::options(test_dir) else {
            return false;
        };
        options.source_type.as_deref() == Some("script")
    }

    fn should_throw(&self, test_dir: &Path) -> bool {
        if let Some(options) = Self::options(test_dir)
            && options.has_throws
        {
            return true;
        }

        let output_path = test_dir.join("output.json");
        let Ok(content) = std::fs::read_to_string(&output_path) else {
            return false;
        };

        let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
            return false;
        };

        value
            .get("errors")
            .and_then(|errors| errors.as_array())
            .is_some_and(|errors| !errors.is_empty())
    }

    fn disallow_ambiguous_tree_literal(&self, test_dir: &Path) -> bool {
        let Some(options) = Self::options(test_dir) else {
            return false;
        };

        if options.disallow_ambiguous_jsx_like {
            return true;
        }

        let has_jsx = options.has_plugin("jsx");
        let has_flow = options.has_plugin("flow");
        let has_typescript = options.has_plugin("typescript");

        if has_jsx && has_typescript {
            return true;
        }

        has_jsx && !has_flow && !has_typescript
    }

    /// Read the nearest options.json (current dir or first ancestor).
    fn options_content(test_dir: &Path) -> Option<String> {
        let mut current = Some(test_dir);
        while let Some(dir) = current {
            let path = dir.join("options.json");
            if let Ok(content) = std::fs::read_to_string(&path) {
                return Some(content);
            }
            current = dir.parent();
        }
        None
    }

    /// Read the nearest options.json and parse it.
    fn options(test_dir: &Path) -> Option<BabelOptions> {
        let content = Self::options_content(test_dir)?;
        BabelOptions::from_content(&content)
    }

    fn get_input_file(&self, test_dir: &Path) -> Option<(PathBuf, FileType)> {
        let path_str = test_dir.to_string_lossy();
        let in_tsx_dir = path_str.contains("/tsx/") || path_str.contains("/tsx-");
        let in_jsx_dir = path_str.contains("/jsx/") || path_str.contains("/jsx-");
        let in_dts_dir = path_str.contains("/dts/") || path_str.contains("/dts-");
        let in_typescript_dir = path_str.contains("/typescript/");
        let options = Self::options(test_dir);
        let has_options = options.is_some();
        let has_jsx = options
            .as_ref()
            .is_some_and(|options| options.has_plugin("jsx"));
        let has_flow = options
            .as_ref()
            .is_some_and(|options| options.has_plugin("flow"));
        let has_typescript = options
            .as_ref()
            .is_some_and(|options| options.has_plugin("typescript"));
        let use_typescript = has_typescript || (!has_options && in_typescript_dir);
        let enable_jsx = if has_options { has_jsx } else { in_jsx_dir };
        for ext in &["ts", "tsx", "js", "jsx", "mjs"] {
            let input = test_dir.join(format!("input.{ext}"));
            if input.exists() {
                let file_type = match *ext {
                    "tsx" => FileType::TypeScriptXml,
                    "ts" if in_dts_dir => FileType::TypeScriptDeclaration,
                    "ts" if in_tsx_dir || has_jsx => FileType::TypeScriptXml,
                    "ts" => FileType::TypeScript,
                    "jsx" => FileType::JavaScriptXml,
                    "js" if enable_jsx && (has_flow || use_typescript) => FileType::TypeScriptXml,
                    "js" if use_typescript => FileType::TypeScript,
                    "js" if enable_jsx => FileType::JavaScriptXml,
                    _ => FileType::JavaScript,
                };
                return Some((input, file_type));
            }
        }
        None
    }
}

impl Default for BabelSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl ConformanceDriver for BabelSuite {
    fn name(&self) -> &str {
        "babel"
    }

    fn suite_dir(&self) -> &Path {
        &self.suite_dir
    }

    fn tests_dir(&self) -> &Path {
        &self.tests_dir
    }

    fn discover_cases(&self) -> Vec<Case> {
        // discover tests from typescript and jsx directories
        // (flow is intentionally excluded, we don't support Flow, only TypeScript)
        let mut tests = Vec::new();

        for category in &["typescript", "jsx"] {
            let category_dir = self.tests_dir.join(category);
            if category_dir.exists() {
                tests.extend(self.discover_recursive(&category_dir, category));
            }
        }

        tests
    }

    fn run(&self, test: &Case, show_diff: bool) -> CaseOutcome {
        let test_dir = self.tests_dir.join(&test.name);

        let Some((input_path, _)) = self.get_input_file(&test_dir) else {
            return CaseOutcome::FailedRead;
        };

        let content = match std::fs::read_to_string(&input_path) {
            Ok(s) => s,
            Err(_) => return CaseOutcome::FailedRead,
        };

        let area = if test.expect_error {
            TestArea::EarlySyntax
        } else {
            TestArea::Parse
        };
        let disallow_ambiguous_tree_literal = self.disallow_ambiguous_tree_literal(&test_dir);
        let parse_outcome = parse_file(
            &input_path,
            &content,
            test.file_type,
            ParseOptions {
                area,
                disallow_ambiguous_tree_literal,
                should_print_diagnostics: show_diff,
            },
        );

        match (test.expect_error, parse_outcome) {
            (true, ParseOutcome::Error) => CaseOutcome::Passed,
            (true, ParseOutcome::Ok) => CaseOutcome::FailedParse,
            (false, ParseOutcome::Ok) => CaseOutcome::Passed,
            (false, ParseOutcome::Error) => CaseOutcome::FailedParse,
        }
    }

    fn fetch_instructions(&self) -> String {
        format!(
            "To refresh Babel parser tests (version {BABEL_VERSION}, commit {BABEL_COMMIT}):\n\
             \n\
               python3 ./language/test/fixtures/conformance/fetch-suite.py ./language/test/fixtures/conformance/babel\n"
        )
    }

    fn category_for_case(&self, test_name: &str) -> String {
        // babel tests are typescript/subcategory/... or jsx/subcategory/...
        // use the second segment as the category
        let parts: Vec<&str> = test_name.split('/').collect();
        if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            parts.first().unwrap_or(&"unknown").to_string()
        }
    }
}

/// Run Babel conformance tests.
pub fn run_babel(
    options: &RunOptions,
    update_known_failures: bool,
) -> Option<ConformanceSuiteResult> {
    let suite = BabelSuite::new();
    run_conformance_driver(&suite, options, update_known_failures)
}
