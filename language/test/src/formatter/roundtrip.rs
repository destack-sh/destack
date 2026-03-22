use std::sync::Arc;

use crate::core::{Case, CaseResult, check_diagnostics};
use destack_ast::{NodeParentIndex, TokenSpan};
use destack_fir::format as fir_format;
use destack_formatter::{
    DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions, statement_list,
};
use destack_parser::Parser;
use destack_source::{
    DiffOptions, File, FileRegistry, FileSystem, FileType, LanguageType, MemoryFileSystem, Uri,
    print_diff,
};
use destack_workspace::{FormatterOptions, LinterOptions, Program};

/// Run a single formatter roundtrip test.
///
/// Verifies that formatting a well-formatted file produces identical output.
pub(super) fn run(test: &Case) -> CaseResult {
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
    program.diagnostics.merge_from(&parser.diagnostics);

    let parse_result = check_diagnostics(test, &program.files, &program.diagnostics);
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
        program.formatter,
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
    let side_span = parser.compute_side_span();
    let strings = parser.strings.clone().into_immutable();
    let parents = NodeParentIndex::from_tree(&parser.tree);

    let format_options = DestackFormatOptions::from_formatter_options(formatter, language_type);

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
