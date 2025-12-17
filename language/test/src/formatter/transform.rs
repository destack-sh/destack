use std::sync::Arc;

use destack_ast::NodeParentIndex;
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions};
use destack_parser::{Parser, source_colorizer};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, File, FileRegistry, FileSystem, FileType,
    LanguageType, MemoryFileSystem, PrintOptions, Uri,
};
use destack_workspace::{FormatterOptions, LinterOptions, Program};

use crate::harness::diff::print_diff;
use crate::harness::{TestResult, format_diagnostics};
use crate::mdtest::MdTestCase;

/// Run a single formatter transform test.
///
/// Test cases are defined in markdown with `ds` input blocks and `ds expected` output blocks.
pub(super) fn run(test: &MdTestCase) -> TestResult {
    let Some(input_file) = test.files.first() else {
        return TestResult::Failed {
            message: "no input code block found".to_string(),
        };
    };

    // find expected block: "ds expected"
    let expected_block = test
        .extra_blocks
        .iter()
        .find(|b| b.language.contains("expected"));
    let Some(expected) = expected_block else {
        return TestResult::Failed {
            message: "no expected code block found (add ```ds expected ... ``` block)".to_string(),
        };
    };

    let file_type = if input_file.path.ends_with(".d.ds") {
        FileType::DestackDeclaration
    } else {
        FileType::Destack
    };

    // set up program context
    let cwd = std::env::current_dir().unwrap_or_default();
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let program = Arc::new(Program::from_options(
        FormatterOptions::default(),
        LinterOptions::default(),
        cwd,
        fs,
        files,
    ));

    // create file
    let file_id = program.files.next_id();
    let uri = Uri::from_string(format!("/test/{}", input_file.path));
    let file = Arc::new(File::from_text(
        file_id,
        input_file.path.clone(),
        uri,
        None,
        file_type,
        input_file.content.clone(),
    ));
    program.files.insert((*file).clone());

    // parse
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file(file.clone(), language_type);
    let expressions = parser.parse();
    parser.finish();

    // bail on parse errors
    let has_errors = parser
        .diagnostics
        .iter()
        .into_iter()
        .any(|d| d.severity == DiagnosticSeverity::Error);
    if has_errors {
        let mut diagnostics = DiagnosticCollection::new();
        for d in parser.diagnostics.iter() {
            diagnostics.insert(d);
        }
        let options = PrintOptions::new().with_colorizer(source_colorizer());
        let rendered = format_diagnostics(&program.files, &diagnostics, options);
        return TestResult::Failed {
            message: format!("parse errors:\n\n{rendered}"),
        };
    }

    // format
    let formatted = format_expressions(
        &parser,
        &expressions,
        &file,
        language_type,
        program.formatter,
    );

    // compare
    let expected_normalized = normalize_output(&expected.content);
    let formatted_normalized = normalize_output(&formatted);

    if formatted_normalized == expected_normalized {
        TestResult::Passed
    } else {
        print_diff(&expected_normalized, &formatted_normalized);
        TestResult::Failed {
            message: "formatted output differs from expected".to_string(),
        }
    }
}

/// Normalize output to remove trailing newlines.
fn normalize_output(s: &str) -> String {
    let lines: Vec<&str> = s.lines().map(|line| line.trim_end()).collect();
    let mut result = lines.join("\n");
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Format a list of expressions.
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

    // format options
    let format_options = DestackFormatOptions {
        language_type,
        line_ending: formatter.line_ending,
        indent_style: formatter.indent_style,
        indent_width: formatter.indent_width,
        line_width: formatter.line_width,
    };

    // format context
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

    // format expressions
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
