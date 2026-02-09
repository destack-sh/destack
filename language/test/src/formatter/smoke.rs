use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::harness::{TestCase, TestResult, format_diagnostics};
use destack_ast::{NodeParentIndex, TokenSpan};
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_parser::{Parser, source_colorizer};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, DiffOptions, File, FileRegistry, FileSystem,
    FileType, LanguageType, MemoryFileSystem, PrintOptions, Uri, print_diff,
};
use destack_workspace::{FormatterOptions, LinterOptions, Program};

/// Formatter smoke category prefix.
pub(super) const SMOKE_CATEGORY: &str = "destack_test::formatter::smoke";

/// A formatter smoke case discovered from fixture files.
#[derive(Debug, Clone)]
pub(super) struct FormatterSmokeCase {
    /// The source input file.
    pub input_path: PathBuf,
    /// The expected output file when present.
    pub expected_path: Option<PathBuf>,
}

/// Discover formatter smoke cases in a fixture directory.
pub(super) fn discover_cases(base_dir: &Path) -> Vec<(TestCase, FormatterSmokeCase)> {
    let mut cases = Vec::new();
    discover_cases_in_dir(base_dir, base_dir, &mut cases);
    cases.sort_by(|left, right| left.0.name.cmp(&right.0.name));
    cases
}

/// Recursively discover formatter smoke files in a directory.
fn discover_cases_in_dir(
    base_dir: &Path,
    directory: &Path,
    cases: &mut Vec<(TestCase, FormatterSmokeCase)>,
) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            if name.starts_with('.') || name == "staging" || name == "node_modules" {
                continue;
            }
            discover_cases_in_dir(base_dir, &path, cases);
            continue;
        }
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !file_name.starts_with("input.") {
            continue;
        }

        let Some(file_type) = FileType::from_path(&path) else {
            continue;
        };
        if !is_smoke_file_type(file_type) {
            continue;
        }

        let Some(parent) = path.parent() else {
            continue;
        };
        let relative_parent = parent.strip_prefix(base_dir).unwrap_or(parent);
        let name = relative_parent.to_string_lossy().replace('\\', "/");
        if name.is_empty() {
            continue;
        }

        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let expected_path = parent.join(format!("expected.{extension}"));
        let expected_path = if expected_path.is_file() {
            Some(expected_path)
        } else {
            None
        };

        let is_skipped = relative_parent
            .components()
            .any(|component| component.as_os_str().to_string_lossy().starts_with('_'));
        let test = TestCase::file(name, path.clone(), SMOKE_CATEGORY).with_skipped(is_skipped);
        let case = FormatterSmokeCase {
            input_path: path,
            expected_path,
        };
        cases.push((test, case));
    }
}

/// Run one formatter smoke case.
pub(super) fn run(test: &TestCase, case: &FormatterSmokeCase) -> TestResult {
    let cwd = case
        .input_path
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_default();
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let program = Arc::new(Program::from_options(
        FormatterOptions::default(),
        LinterOptions::default(),
        cwd,
        fs,
        files,
    ));

    let original = match std::fs::read_to_string(&case.input_path) {
        Ok(content) => content,
        Err(error) => {
            return TestResult::Failed {
                message: format!(
                    "failed to read input '{}': {error}",
                    case.input_path.display()
                ),
            };
        }
    };

    let first_pass = match format_source(&program, &case.input_path, &original) {
        Ok(formatted) => formatted,
        Err(message) => {
            return TestResult::Failed { message };
        }
    };

    if let Some(expected_path) = &case.expected_path {
        let expected = match std::fs::read_to_string(expected_path) {
            Ok(content) => content,
            Err(error) => {
                return TestResult::Failed {
                    message: format!(
                        "failed to read expected '{}': {error}",
                        expected_path.display()
                    ),
                };
            }
        };

        let expected = normalize_output(&expected);
        let actual = normalize_output(&first_pass);
        if actual == expected {
            return TestResult::Passed;
        }

        print_diff(&expected, &actual, &DiffOptions::new());
        return TestResult::Failed {
            message: format!("formatted output differs from expected for '{}'", test.name),
        };
    }

    let second_pass = match format_source(&program, &case.input_path, &first_pass) {
        Ok(formatted) => formatted,
        Err(message) => {
            return TestResult::Failed { message };
        }
    };

    let first_pass = normalize_output(&first_pass);
    let second_pass = normalize_output(&second_pass);
    if first_pass == second_pass {
        return TestResult::Passed;
    }

    print_diff(&first_pass, &second_pass, &DiffOptions::new());
    TestResult::Failed {
        message: format!("formatter is not idempotent for '{}'", test.name),
    }
}

/// Format a single source file with formatter defaults.
fn format_source(program: &Arc<Program>, path: &Path, source: &str) -> Result<String, String> {
    let file_type = FileType::from_path(path)
        .ok_or_else(|| format!("unsupported file type: {}", path.display()))?;
    let language_type = LanguageType::from(file_type);
    let file_id = program.files.next_id();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("input")
        .to_string();
    let uri = Uri::from_path(path);
    let file = Arc::new(File::from_text(
        file_id,
        name,
        uri,
        Some(path.to_path_buf()),
        file_type,
        source.to_string(),
    ));
    program.files.insert((*file).clone());

    let mut parser = Parser::lex_file(file.clone(), language_type);
    let expressions = parser.parse();
    parser.finish();

    let has_errors = parser
        .diagnostics
        .iter()
        .into_iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);
    if has_errors {
        let mut diagnostics = DiagnosticCollection::new();
        for diagnostic in parser.diagnostics.iter() {
            diagnostics.insert(diagnostic);
        }
        let options = PrintOptions::new().with_colorizer(source_colorizer());
        let rendered = format_diagnostics(&program.files, &diagnostics, options);
        return Err(format!(
            "parse errors in '{}':\n\n{rendered}",
            path.display()
        ));
    }

    let (tokens, side_tokens) = parser.take_tokens();
    Ok(format_expressions(
        &parser,
        &tokens,
        &side_tokens,
        &expressions,
        &file,
        language_type,
        program.formatter,
    ))
}

/// Format parsed expressions into source code.
fn format_expressions(
    parser: &Parser,
    tokens: &Vec<TokenSpan>,
    side_tokens: &Vec<TokenSpan>,
    expressions: &[destack_ast::LocalNodeId<destack_ast::Expression>],
    file: &File,
    language_type: LanguageType,
    formatter: FormatterOptions,
) -> String {
    let side_span = parser.compute_side_span();
    let strings = parser.strings.clone().into_immutable();
    let parents = NodeParentIndex::from_tree(&parser.tree);

    let format_options = DestackFormatOptions::from_formatter_options(formatter, language_type);
    let context = DestackFormatContext::new(
        format_options,
        file,
        &parser.tree,
        tokens,
        side_tokens,
        &side_span,
        &strings,
        parents,
    );

    let mut result = if expressions.is_empty() {
        String::new()
    } else {
        let formatted = fir_format!(context.clone(), [statement_list(expressions)]).unwrap();
        let printed = formatted.print().unwrap();
        printed.as_str().to_string()
    };

    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// Normalize output for stable comparisons.
fn normalize_output(content: &str) -> String {
    let lines: Vec<&str> = content.lines().map(|line| line.trim_end()).collect();
    let mut result = lines.join("\n");
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Return whether this file type is supported by formatter smoke cases.
fn is_smoke_file_type(file_type: FileType) -> bool {
    matches!(
        file_type,
        FileType::Destack
            | FileType::DestackDeclaration
            | FileType::JavaScript
            | FileType::JavaScriptXml
            | FileType::TypeScript
            | FileType::TypeScriptXml
            | FileType::TypeScriptDeclaration
    )
}
