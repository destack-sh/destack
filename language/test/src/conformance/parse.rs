use std::path::Path;
use std::sync::Arc;

use destack_core::StringPool;
use destack_parser::{Parser, ParserOptions, source_colorizer};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, File, FileId, FileType, LanguageType, PrintOptions,
    Uri,
};

use crate::conformance::{CaseOutcome, SourceValidity};
use crate::core::format_diagnostics;

/// Outcome of checking a file for conformance testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParseOutcome {
    /// File parsed without errors.
    Accepted,
    /// File had parser errors.
    Rejected,
}

impl ParseOutcome {
    /// Convert parser acceptance into one harness outcome.
    pub(super) fn case_outcome(self, source_validity: SourceValidity) -> CaseOutcome {
        match (source_validity, self) {
            (SourceValidity::Valid, Self::Accepted) => CaseOutcome::Passed,
            (SourceValidity::Valid, Self::Rejected) => CaseOutcome::FailedParse,
            (SourceValidity::Invalid, Self::Rejected) => CaseOutcome::Passed,
            (SourceValidity::Invalid, Self::Accepted) => CaseOutcome::FailedParse,
        }
    }
}

/// Options for parsing in conformance tests.
#[derive(Debug, Clone, Default)]
pub(super) struct ParseOptions {
    /// Whether to disallow ambiguous tree literal syntax.
    pub disallow_ambiguous_tree_literal: bool,
    /// Whether failed cases should print diagnostics.
    pub should_print_diagnostics: bool,
}

/// Parse one file and return whether parser diagnostics rejected it.
pub(super) fn parse_file(
    path: &Path,
    content: &str,
    file_type: FileType,
    options: ParseOptions,
) -> ParseOutcome {
    // build a synthetic file for parser only diagnostics
    let uri = Uri::from_path(path);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("case.js")
        .to_string();
    let file = Arc::new(File::from_text(
        FileId::new(0),
        name,
        uri,
        Some(path.to_path_buf()),
        file_type,
        content.to_string(),
    ));

    // parse and collect diagnostics
    let language = LanguageType::try_from(file_type).expect("file type has no parser language");
    let mut parser = Parser::lex_file_with_options(
        file.clone(),
        language,
        ParserOptions {
            disallow_ambiguous_tree_literal: options.disallow_ambiguous_tree_literal,
            ..ParserOptions::default()
        },
        Arc::new(StringPool::new()),
    );
    let _ = parser.parse();
    let diagnostics = parser.diagnostics();

    // collect parse errors for relevance checks
    let has_error = diagnostics
        .to_vec()
        .into_iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);

    if has_error {
        if options.should_print_diagnostics {
            let file_for_id = |current_file_id| {
                if current_file_id == file.id {
                    Some(file.clone())
                } else {
                    None
                }
            };
            print_conformance_diagnostics(path, &file_for_id, diagnostics);
        }

        ParseOutcome::Rejected
    } else {
        ParseOutcome::Accepted
    }
}

/// Print diagnostics for one conformance case.
fn print_conformance_diagnostics(
    path: &Path,
    file_for_id: &impl Fn(FileId) -> Option<Arc<File>>,
    diagnostics: DiagnosticCollection,
) {
    let options = PrintOptions::new().with_colorizer(source_colorizer());
    let rendered = format_diagnostics(file_for_id, &diagnostics, options);
    println!(
        "conformance diagnostics for {}:\n{rendered}",
        path.display()
    );
}
