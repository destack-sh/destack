#![no_main]

use std::path::Path;
use std::sync::Arc;

use destack_ast::NodeParentIndex;
use destack_core::StringPool;
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_parser::{Parser, ParserOptions};
use destack_source::{DiagnosticSeverity, File, FileId, FileType, LanguageType, Uri};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    // minimal parser context
    let file_id = FileId::new(0);
    let uri = Uri::from_path(Path::new("fuzz.ds"));
    let file = Arc::new(File::from_text(
        file_id,
        "fuzz.ds".to_string(),
        uri,
        None,
        FileType::Destack,
        input.to_string(),
    ));

    // parse the input
    let mut parser = Parser::lex_file_with_options(
        file.clone(),
        LanguageType::Destack,
        ParserOptions {
            preserve_parenthesized_wrappers: false,
            ..ParserOptions::default()
        },
        Arc::new(StringPool::new()),
    );
    let expressions = parser.parse();

    // skip parse failures
    if parser
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        return;
    }

    // format without panicking
    let side_span = parser.compute_side_span();
    let strings = parser.strings.as_ref();
    let parents = NodeParentIndex::from_expression_roots(&parser.tree, &expressions);
    let (tokens, side_tokens) = parser.take_tokens();
    let format_options = DestackFormatOptions::default();

    let context = DestackFormatContext::new(
        format_options,
        file.as_ref(),
        &parser.tree,
        &tokens,
        &side_tokens,
        &side_span,
        strings,
        parents,
    );

    // ignore formatting failures but not panics
    if let Ok(formatted) = fir_format!(context.clone(), [statement_list(&expressions)]) {
        let _ = formatted.print();
    }
});
