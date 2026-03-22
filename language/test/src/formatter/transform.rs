use std::path::Path;
use std::sync::Arc;

use crate::core::{CaseResult, format_diagnostics};
use crate::mdtest::MdTestCase;
use destack_ast::{NodeParentIndex, TokenSpan};
use destack_fir::format as fir_format;
use destack_formatter::{
    DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions, statement_list,
};
use destack_parser::{Parser, source_colorizer};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, DiffOptions, File, FileRegistry, FileSystem,
    FileType, LanguageType, MemoryFileSystem, PrintOptions, Uri, print_diff,
};
use destack_workspace::{FormatterOptions, LinterOptions, Program};

/// Run a single formatter transform test.
///
/// Test cases are defined in markdown with `ds` input blocks and `ds expected` output blocks.
pub(super) fn run(test: &MdTestCase) -> CaseResult {
    let Some(input_file) = test.files.first() else {
        return CaseResult::Failed {
            message: "no input code block found".to_string(),
        };
    };

    // find expected block: "ds expected"
    let expected_block = test
        .extra_blocks
        .iter()
        .find(|b| b.language.contains("expected"));
    let Some(expected) = expected_block else {
        return CaseResult::Failed {
            message: "no expected code block found (add ```ds expected ... ``` block)".to_string(),
        };
    };

    let file_path = Path::new(&input_file.path);
    let file_type = FileType::from_path(file_path).unwrap_or(FileType::Destack);

    // build formatter options from test options
    let mut formatter_options = FormatterOptions::default();
    if let Some(line_width) = input_file.options.get("line-width")
        && let Ok(width) = line_width.parse::<u16>()
    {
        formatter_options = formatter_options.with_line_width(width);
    }
    if let Some(indent_width) = input_file.options.get("indent-width")
        && let Ok(width) = indent_width.parse::<u8>()
    {
        formatter_options = formatter_options.with_indent_width(width);
    }
    if let Some(bracket_same_line) = input_file.options.get("bracket-same-line")
        && let Ok(value) = bracket_same_line.parse::<bool>()
    {
        formatter_options = formatter_options.with_bracket_same_line(value);
    }
    if let Some(single_attr_per_line) = input_file.options.get("single-attribute-per-line")
        && let Ok(value) = single_attr_per_line.parse::<bool>()
    {
        formatter_options = formatter_options.with_single_attribute_per_line(value);
    }
    if let Some(organize_imports) = input_file.options.get("organize-imports")
        && let Some(value) = destack_workspace::OrganizeImports::parse(organize_imports)
    {
        formatter_options = formatter_options.with_organize_imports(value);
    }
    if let Some(quote_props) = input_file.options.get("quote-props")
        && let Some(value) = destack_workspace::QuoteProperty::parse(quote_props)
    {
        formatter_options = formatter_options.with_quote_props(value);
    }

    // set up program context
    let cwd = std::env::current_dir().unwrap_or_default();
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let program = Arc::new(Program::from_options(
        formatter_options,
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
        return CaseResult::Failed {
            message: format!("parse errors:\n\n{rendered}"),
        };
    }

    // format
    let (tokens, side_tokens) = parser.take_tokens();
    let formatted = format_expressions(
        &parser,
        &tokens,
        &side_tokens,
        &expressions,
        &file,
        language_type,
        program.formatter,
    );

    // compare
    let expected_normalized = normalize_output(&expected.content);
    let formatted_normalized = normalize_output(&formatted);

    if formatted_normalized == expected_normalized {
        CaseResult::Passed
    } else {
        print_diff(
            &expected_normalized,
            &formatted_normalized,
            &DiffOptions::new(),
        );
        CaseResult::Failed {
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

    // format options
    let format_options = DestackFormatOptions::from_formatter_options(formatter, language_type);

    // format context
    let context = DestackFormatContext::new(
        format_options,
        DestackFormatArtifacts {
            file,
            tree: &parser.tree,
            tokens,
            side_tokens,
            side_span: &side_span,
            strings: &strings,
            parents,
        },
    );

    // format expressions
    let mut result = if expressions.is_empty() {
        String::new()
    } else {
        let formatted = fir_format!(context.clone(), [statement_list(expressions)]).unwrap();
        let printed = formatted.print().unwrap();
        printed.as_str().to_string()
    };

    // ensure trailing newline
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
}
