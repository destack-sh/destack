#![no_main]

use std::path::Path;
use std::sync::Arc;

use destack_ast::NodeParentIndex;
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions};
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
    parser.finish();

    // Only format if parsing succeeded without errors
    if parser.diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        return;
    }

    // Format - should not panic
    let side_span = parser.compute_side_span();
    let strings = parser.strings.clone().into_immutable();
    let parents = NodeParentIndex::from_tree(&parser.tree);
    let format_options = DestackFormatOptions::default();

    let context = DestackFormatContext {
        options: format_options,
        file: &file,
        tree: &parser.tree,
        source_map: &parser.tree.source_map,
        parents,
        tokens: &parser.tokens,
        side_tokens: &parser.side_tokens,
        side_span: &side_span,
        strings: &strings,
    };

    for expr in &expressions {
        if let Ok(formatted) = fir_format!(context.clone(), [expr]) {
            let _ = formatted.print();
        }
    }
});
