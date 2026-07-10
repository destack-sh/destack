use std::sync::Arc;
use std::time::Instant;

use destack_core::StringPool;
use destack_dir::{Declaration, Expression, LocalNodeId, Name, Pattern};
use destack_parser::Parser;
use destack_source::{File, FileId, LanguageType, Uri};

use crate::core::CaseResult;

use super::stats::collect_parser_stats;
use crate::stress::throughput::Throughput;
use crate::stress::{StressExpectation, StressFixture};

const RECOVERY_SENTINEL: &str = "stressRecovered";

/// Run one parser stress fixture.
pub(super) fn run_parser_stress(fixture: &StressFixture) -> CaseResult {
    let source = match fixture.source() {
        Ok(source) => source,
        Err(message) => return CaseResult::Failed { message },
    };
    let file_size = source.len();
    let line_count = source.lines().count();
    let file_name = match fixture.file_name() {
        Ok(file_name) => file_name,
        Err(message) => return CaseResult::Failed { message },
    };
    let file_id = FileId::from_logical_path(&fixture.logical_path());
    let file = File::from_text(
        file_id,
        file_name.clone(),
        Uri::from_string(&file_name),
        None,
        fixture.file_type,
        source.clone(),
    );
    let file = Arc::new(file);

    // parse the stress source
    let start = Instant::now();
    let language_type =
        LanguageType::try_from(file.ty).expect("stress file type has no parser language");
    let mut parser = Parser::lex_file(file, language_type, Arc::new(StringPool::new()));
    let roots = parser.parse();
    let elapsed = start.elapsed();
    let has_errors = !parser.errors.is_empty();

    // bounded fixtures only assert process safety
    if fixture.expectation == StressExpectation::Bounded {
        let stats = collect_parser_stats(&mut parser);
        let throughput = Throughput::new(file_size, line_count, elapsed);
        eprintln!("{}{}", throughput.format("bounded"), stats.format());

        return CaseResult::Passed;
    }

    // valid fixtures must be clean
    if fixture.expectation == StressExpectation::Valid && has_errors {
        let message = parser
            .diagnostics()
            .to_vec()
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect::<Vec<_>>()
            .join("\n");

        return CaseResult::Failed {
            message: format!("{message}\n\nsource:\n{source}"),
        };
    }

    // damaged fixtures must report the damage and resume after it
    if fixture.expectation == StressExpectation::Recovery {
        if !has_errors {
            return CaseResult::Failed {
                message: "recovery fixture parsed without diagnostics".to_string(),
            };
        }

        if !contains_recovery_sentinel(&parser, &roots) {
            return CaseResult::Failed {
                message: format!("recovery fixture did not reach {RECOVERY_SENTINEL}"),
            };
        }
    }

    let stats = collect_parser_stats(&mut parser);
    let throughput = Throughput::new(file_size, line_count, elapsed);
    eprintln!("{}{}", throughput.format("parsed"), stats.format());

    CaseResult::Passed
}

/// Return whether any parsed root reaches the recovery sentinel.
fn contains_recovery_sentinel(parser: &Parser, roots: &[LocalNodeId<Expression>]) -> bool {
    roots
        .iter()
        .any(|root| expression_names_sentinel(parser, *root))
}

/// Return whether one expression names the recovery sentinel.
fn expression_names_sentinel(parser: &Parser, expression_id: LocalNodeId<Expression>) -> bool {
    match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => {
            declaration_names_sentinel(parser, *declaration_id)
        }
        Expression::Let { declarators, .. } => declarators.iter().any(|declarator_id| {
            let declarator = parser.tree.get(*declarator_id);

            pattern_names_sentinel(parser, declarator.pattern)
        }),
        _ => false,
    }
}

/// Return whether one declaration names the recovery sentinel.
fn declaration_names_sentinel(parser: &Parser, declaration_id: LocalNodeId<Declaration>) -> bool {
    let name = match parser.tree.get(declaration_id) {
        Declaration::Type(declaration) => Some(declaration.name),
        Declaration::Struct(declaration) => Some(declaration.name),
        Declaration::Class(declaration) => declaration.name,
        Declaration::Enum(declaration) => declaration.name,
        Declaration::Interface(declaration) => declaration.name,
        Declaration::Function(declaration) => declaration.name,
        Declaration::Global(_) | Declaration::Module(_) | Declaration::Extension(_) => None,
    };

    name.is_some_and(|name| name_is_sentinel(parser, name))
}

/// Return whether one pattern names the recovery sentinel.
fn pattern_names_sentinel(parser: &Parser, pattern_id: LocalNodeId<Pattern>) -> bool {
    match parser.tree.get(pattern_id) {
        Pattern::Binding { name, .. } => parser.strings.get(*name) == RECOVERY_SENTINEL,
        Pattern::Must(pattern) => pattern_names_sentinel(parser, *pattern),
        Pattern::Expression { value } => expression_names_sentinel(parser, *value),
        Pattern::Default { pattern, .. }
        | Pattern::BorrowOf { right: pattern, .. }
        | Pattern::MoveOf { right: pattern, .. }
        | Pattern::DereferenceOf { right: pattern } => pattern_names_sentinel(parser, *pattern),
        _ => false,
    }
}

/// Return whether one name is the recovery sentinel.
fn name_is_sentinel(parser: &Parser, name: Name) -> bool {
    parser.strings.get(name.string()) == RECOVERY_SENTINEL
}
