use std::sync::Arc;

use crate::core::{Case, CaseResult, check_diagnostics};
use destack_ast::{NodeParentIndex, TokenSpan};
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_parser::{Parser, ParserOptions};
use destack_source::{DiffOptions, File, FileId, FileType, LanguageType, Uri, print_diff};
use destack_workspace::FormatterOptions;

/// Run a single formatter roundtrip test.
///
/// Verifies that formatting a well-formatted file produces identical output.
pub(super) fn run(test: &Case) -> CaseResult {
    // read original
    let original = match std::fs::read_to_string(&test.path) {
        Ok(content) => content,
        Err(e) => {
            return CaseResult::Failed {
                message: format!("failed to read file: {e}"),
            };
        }
    };

    // create file
    let uri = Uri::from_path(&test.path);
    let Some(file_type) = FileType::from_path(&test.path) else {
        return CaseResult::Failed {
            message: format!("unsupported roundtrip file type: {}", test.path.display()),
        };
    };
    let name = test.path.file_name().unwrap().to_string_lossy().to_string();
    let path = Some(test.path.clone());
    let file_id = FileId::from_logical_path(&test.path);
    let file = Arc::new(File::from_text(
        file_id,
        name,
        uri,
        path,
        file_type,
        original.clone(),
    ));
    let file_for_id = |current_file_id| {
        if current_file_id == file_id {
            Some(file.clone())
        } else {
            None
        }
    };

    // parse
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file_with_options(
        file.clone(),
        language_type,
        ParserOptions {
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        },
    );
    let expressions = parser.parse();
    let parse_result = check_diagnostics(test, &file_for_id, &parser.diagnostics);
    if parse_result.is_failed() {
        return parse_result;
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
        FormatterOptions::default(),
    );

    if formatted == original {
        CaseResult::Passed
    } else {
        print_diff(&original, &formatted, &DiffOptions::new());

        CaseResult::Failed {
            message: "formatted output differs from original".to_string(),
        }
    }
}

fn format_expressions(
    parser: &Parser,
    tokens: &Vec<TokenSpan>,
    side_tokens: &Vec<TokenSpan>,
    expressions: &[destack_ast::LocalNodeId<destack_ast::Expression>],
    file: &File,
    language_type: LanguageType,
    formatter: FormatterOptions,
) -> String {
    // build formatter context
    let side_span = parser.compute_side_span();
    let strings = parser.strings.clone().into_immutable();
    let parents = NodeParentIndex::from_expression_roots(&parser.tree, expressions);

    // convert options and format
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

    // ensure a trailing newline
    let formatted = fir_format!(context.clone(), [statement_list(expressions)]).unwrap();
    let printed = formatted.print().unwrap();
    let mut result = printed.as_str().to_string();

    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
}
