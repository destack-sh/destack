//! Formatter roundtrip tests.

use std::sync::Arc;

use destack_ast::NodeParentIndex;
use destack_dir::Program;
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions};
use destack_parser::Parser;
use destack_source::{
    File, FileId, FileRegistry, FileSystem, FileType, LanguageOptions, MemoryFileSystem, Uri,
};

use crate::harness::{
    TestCase, TestOptions, TestResult, check_diagnostics, discover_test_files, fixtures_dir,
    run_tests,
};

/// Run all formatter roundtrip tests.
pub fn run_formatter_tests(options: &TestOptions) -> std::process::ExitCode {
    let formatter_dir = fixtures_dir().join("formatter");
    let tests = discover_test_files(&formatter_dir, &["ds", ".d.ds"], "destack_test::formatter")
        .expect("failed to discover tests");
    run_tests(tests, options, run_roundtrip_test)
}

/// Run a single formatter roundtrip test.
fn run_roundtrip_test(test: &TestCase) -> TestResult {
    // set up a minimal program for diagnostics
    let cwd = test.path.parent().unwrap().to_path_buf();
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let language = LanguageOptions::default();
    let program = Arc::new(Program::new(language, cwd, fs, files));

    // read the original file
    let original = match std::fs::read_to_string(&test.path) {
        Ok(content) => content,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to read file: {e}"),
            };
        }
    };

    // create file
    let uri = Uri::from_path(&test.path);
    let file_type = if test.path.to_string_lossy().ends_with(".d.ds") {
        FileType::DestackDeclaration
    } else {
        FileType::Destack
    };
    let file_id = FileId::new(0);
    let name = test.path.file_name().unwrap().to_string_lossy().to_string();
    let path = Some(test.path.clone());
    let file = Arc::new(File::from_text(
        file_id,
        name,
        uri,
        path,
        file_type,
        original.clone(),
    ));
    program.files.insert((*file).clone());

    // parse the file
    let mut parser = Parser::lex_file(file.clone(), program.language);
    let expressions = parser.parse();
    parser.finish();
    program.diagnostics.merge_from(&parser.diagnostics);
    // bail on parse errors
    let parse_result = check_diagnostics(test, &program.files, &program.diagnostics);
    if parse_result.is_failed() {
        return parse_result;
    }

    // format the file
    let formatted = format_expressions(&parser, &expressions, &file, program.language);

    // compare to original
    if formatted == original {
        TestResult::Passed
    } else {
        // print a nice diff
        print_diff(&original, &formatted);
        TestResult::Failed {
            message: "formatted output differs from original".to_string(),
        }
    }
}

/// Format parsed expressions back to a string.
fn format_expressions(
    parser: &Parser,
    expressions: &[destack_ast::LocalNodeId<destack_ast::Expression>],
    file: &File,
    language: LanguageOptions,
) -> String {
    let side_span = parser.compute_side_span();
    let strings = parser.strings.clone().into_immutable();
    let parents = NodeParentIndex::from_tree(&parser.tree);
    let format_options = DestackFormatOptions::from(language);

    let context = DestackFormatContext {
        options: format_options,
        file,
        tree: &parser.tree,
        source_map: &parser.tree.source_map,
        parents,
        tokens: &parser.tokens,
        side_tokens: &parser.side_tokens,
        side_span: &side_span,
        strings: &strings,
    };

    let mut result = String::new();
    for (i, expr) in expressions.iter().enumerate() {
        let formatted = fir_format!(context.clone(), [expr]).unwrap();
        let printed = formatted.print().unwrap();
        result.push_str(printed.as_str());
        if i < expressions.len() - 1 {
            result.push('\n');
        }
    }

    // ensure trailing newline
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// Print a simple diff between original and formatted.
fn print_diff(old: &str, new: &str) {
    use crate::harness::print::color;

    eprintln!();
    eprintln!("{}:", color::red("diff"));

    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let max_lines = old_lines.len().max(new_lines.len());
    let mut in_diff = false;
    for i in 0..max_lines {
        let old = old_lines.get(i).copied();
        let new = new_lines.get(i).copied();
        match (old, new) {
            (Some(o), Some(f)) if o == f => {
                if in_diff {
                    eprintln!("  ...");
                    in_diff = false;
                }
            }
            (Some(o), Some(f)) => {
                eprintln!("{} {}", color::red(&format!("-{:>4}|", i + 1)), o);
                eprintln!("{} {}", color::green(&format!("+{:>4}|", i + 1)), f);
                in_diff = true;
            }
            (Some(o), None) => {
                eprintln!("{} {}", color::red(&format!("-{:>4}|", i + 1)), o);
                in_diff = true;
            }
            (None, Some(f)) => {
                eprintln!("{} {}", color::green(&format!("+{:>4}|", i + 1)), f);
                in_diff = true;
            }
            (None, None) => {}
        }
    }
    eprintln!();
}
