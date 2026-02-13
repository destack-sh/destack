#![no_main]

use std::path::Path;
use std::sync::Arc;

use destack_ast::NodeParentIndex;
use destack_fir::format as fir_format;
use destack_formatter::{
    DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions, statement_list,
};
use destack_parser::Parser;
use destack_source::{DiagnosticSeverity, File, FileId, FileType, LanguageType, Uri};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    // Set up minimal context for parsing
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

    // Parse the input
    let mut parser = Parser::lex_file(file.clone(), LanguageType::Destack);
    let expressions = parser.parse();

    // Only format if parsing succeeded without errors
    if parser.diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        return;
    }

    // Format - should not panic
    let side_span = parser.compute_side_span();
    let strings = parser.strings.clone().into_immutable();
    let parents = NodeParentIndex::from_tree(&parser.tree);
    let (tokens, side_tokens) = parser.take_tokens();
    let format_options = DestackFormatOptions::default();

    let context = DestackFormatContext::new(
        format_options,
        DestackFormatArtifacts {
            file: file.as_ref(),
            tree: &parser.tree,
            tokens: &tokens,
            side_tokens: &side_tokens,
            side_span: &side_span,
            strings: &strings,
            parents,
        },
    );

    if let Ok(formatted) = fir_format!(context.clone(), [statement_list(&expressions)]) {
        let _ = formatted.print();
    }
});
