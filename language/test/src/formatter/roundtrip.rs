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
use crate::harness::{TestCase, TestResult, check_diagnostics};

/// Run a single formatter roundtrip test.
///
/// Verifies that formatting a well-formatted file produces identical output.
pub(super) fn run(test: &TestCase) -> TestResult {
    let cwd = test.path.parent().unwrap().to_path_buf();
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let program = Arc::new(Program::from_options(
        FormatterOptions::default(),
        LinterOptions::default(),
        cwd,
        fs,
        files,
    ));

    // read original
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

    // parse
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file(file.clone(), language_type);
    let expressions = parser.parse();
    parser.finish();
    program.diagnostics.merge_from(&parser.diagnostics);

    let parse_result = check_diagnostics(test, &program.files, &program.diagnostics);
    if parse_result.is_failed() {
        return parse_result;
    }

    // format
    let formatted = format_expressions(
        &parser,
        &expressions,
        &file,
        language_type,
        program.formatter,
    );

    if formatted == original {
        TestResult::Passed
    } else {
        print_diff(&original, &formatted);
        TestResult::Failed {
            message: "formatted output differs from original".to_string(),
        }
    }
}

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

    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
}
