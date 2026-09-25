use tspp_compiler::Compiler;
use tspp_linter::{Lint, LinterError};
use tspp_mir::parse;
use tspp_parser::ParserError;
use tspp_source::{DiagnosticDefinition, DiagnosticRegistry};

/// Iterate every built-in diagnostic definition.
pub fn definitions() -> impl Iterator<Item = &'static DiagnosticDefinition> {
    ParserError::ALL
        .iter()
        .chain(Compiler::diagnostic_definitions())
        .chain(parse::ParseError::ALL)
        .chain(LinterError::ALL)
}

/// Build the complete built-in diagnostic registry.
pub fn registry() -> DiagnosticRegistry {
    let definitions = definitions().map(|definition| (definition.id, definition.is_controllable));
    let lints = Lint::all().map(|lint| (lint.id.as_ref(), true));

    DiagnosticRegistry::new(definitions.chain(lints))
}
