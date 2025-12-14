use std::sync::Arc;

use destack_ast::NodeParentIndex;
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions};
use destack_parser::Parser;
use destack_source::{
    File, FileRegistry, FileSystem, FileType, LanguageType, MemoryFileSystem, Uri,
};
use destack_workspace::{FormatterOptions, LinterOptions, Program};

use crate::harness::diff::print_diff;
use crate::harness::{
    RunContext, Runner, Suite, TestCase, TestOptions, TestResult, check_diagnostics,
    discover_test_files, fixtures_dir,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct FormatterSuite;

impl Suite for FormatterSuite {
    fn name(&self) -> &'static str {
        "formatter"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let formatter_directory = fixtures_dir().join("formatter");
        discover_test_files(
            &formatter_directory,
            &["ds", ".d.ds"],
            "destack_test::formatter",
        )
        .expect("failed to discover tests")
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        run_roundtrip_case(case)
    }
}

/// Run all formatter roundtrip tests.
pub fn run_formatter_tests(options: &TestOptions) -> std::process::ExitCode {
    Runner::run_suite(&FormatterSuite, options)
}

/// Run a single formatter roundtrip test.
fn run_roundtrip_case(test: &TestCase) -> TestResult {
    // set up a minimal program for diagnostics
    let cwd = test.path.parent().unwrap().to_path_buf();
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let program = Arc::new(Program::new(
        FormatterOptions::default(),
        LinterOptions::default(),
        cwd,
        fs,
        files,
    ));

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
    let file_id = program.files.next_id();
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
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file(file.clone(), language_type);
    let expressions = parser.parse();
    parser.finish();
    program.diagnostics.merge_from(&parser.diagnostics);
    // bail on parse errors
    let parse_result = check_diagnostics(test, &program.files, &program.diagnostics);
    if parse_result.is_failed() {
        return parse_result;
    }

    // format the file
    let formatted = format_expressions(
        &parser,
        &expressions,
        &file,
        language_type,
        program.formatter,
    );

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
    language_type: LanguageType,
    formatter: FormatterOptions,
) -> String {
    let side_span = parser.compute_side_span();
    let strings = parser.strings.clone().into_immutable();
    let parents = NodeParentIndex::from_tree(&parser.tree);
    let format_options = DestackFormatOptions {
        language_type,
        line_ending: formatter.line_ending,
        indent_style: formatter.indent_style,
        indent_width: formatter.indent_width,
        line_width: formatter.line_width,
    };

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
