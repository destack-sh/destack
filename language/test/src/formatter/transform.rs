use std::path::Path;
use std::sync::Arc;

use crate::core::{CaseResult, format_diagnostics};
use crate::mdtest::MdTestCase;
use destack_core::StringPool;
use destack_dir::{NodeParentIndex, TokenSpan};
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_parser::{Parser, ParserOptions, source_colorizer};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, DiffOptions, File, FileId, FileType, IndentStyle,
    LanguageType, PrintOptions, Uri, print_diff,
};
use destack_workspace::FormatterOptions;

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
    if let Some(indent_style) = input_file.options.get("indent-style") {
        let indent_style = match indent_style.as_str() {
            "space" => Some(IndentStyle::Space),
            "tab" => Some(IndentStyle::Tab),
            _ => None,
        };
        if let Some(indent_style) = indent_style {
            formatter_options = formatter_options.with_indent_style(indent_style);
        }
    }
    // create file
    let uri = Uri::from_string(format!("/test/{}", input_file.path));
    let file_id = FileId::from_logical_str(uri.as_ref());
    let file = Arc::new(File::from_text(
        file_id,
        input_file.path.clone(),
        uri,
        None,
        file_type,
        input_file.content.clone(),
    ));
    let file_for_id = |current_file_id| {
        if current_file_id == file_id {
            Some(file.clone())
        } else {
            None
        }
    };

    // parse
    let language_type = LanguageType::try_from(file.ty).expect("file type has no parser language");
    let mut parser = Parser::lex_file_with_options(
        file.clone(),
        language_type,
        ParserOptions {
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        },
        Arc::new(StringPool::new()),
    );
    let expressions = parser.parse();

    // bail on parse errors
    let has_errors = parser
        .diagnostics
        .to_vec()
        .into_iter()
        .any(|d| d.severity == DiagnosticSeverity::Error);
    if has_errors {
        let mut diagnostics = DiagnosticCollection::new();
        for d in parser.diagnostics.to_vec() {
            diagnostics.insert(d);
        }
        let options = PrintOptions::new().with_colorizer(source_colorizer());
        let rendered = format_diagnostics(&file_for_id, &diagnostics, options);
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
        formatter_options,
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
    // trim trailing whitespace and normalize the final newline
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
    expressions: &[destack_dir::LocalNodeId<destack_dir::Expression>],
    file: &File,
    language_type: LanguageType,
    formatter: FormatterOptions,
) -> String {
    // build formatter context
    let side_span = parser.compute_side_span();
    let strings = parser.strings.as_ref();
    let parents = NodeParentIndex::from_expression_roots(&parser.tree, expressions);

    // format options
    let format_options = DestackFormatOptions::from_formatter_options(formatter, language_type);

    // format context
    let context = DestackFormatContext::new(
        format_options,
        file,
        &parser.tree,
        tokens,
        side_tokens,
        &side_span,
        strings,
        parents,
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
