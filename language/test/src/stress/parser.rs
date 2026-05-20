use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use destack_core::StringPool;
use destack_dir::{Declaration, Expression, LocalNodeId, Name, Pattern};
use destack_parser::Parser;
use destack_source::{File, FileId, FileType, LanguageType, Uri};

use crate::core::{Case, CaseResult, RunContext, RunOptions, Suite};

use super::metric::StressMetric;
use super::process::{StressWorker, run_stress_child};
use super::{StressCase, StressExpectation, materialize_parser_cases};

const RECOVERY_SENTINEL: &str = "stressRecovered";
const WORKER_TIMEOUT: Duration = Duration::from_secs(30);
const HARNESS_TIMEOUT: Duration = Duration::from_secs(35);

/// Stress test suite for the parser.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParserStressSuite;

impl ParserStressSuite {
    /// Materialize parser stress fixtures and return the generated case count.
    pub fn generate() -> Result<usize, String> {
        materialize_parser_cases().map(|cases| cases.len())
    }

    /// Run one parser stress fixture inside a worker process.
    pub fn run_worker(path: PathBuf) -> CaseResult {
        let stress_case = match StressCase::from_path(path) {
            Ok(stress_case) => stress_case,
            Err(message) => return CaseResult::Failed { message },
        };

        run_parser_stress(&stress_case)
    }
}

impl Suite for ParserStressSuite {
    fn name(&self) -> &'static str {
        "stress-parser"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        materialize_parser_cases()
            .unwrap_or_else(|error| {
                eprintln!("{error}");
                Vec::new()
            })
            .into_iter()
            .map(|case| Case::file(case.name, case.path, StressCase::parser_category()))
            .collect()
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        let stress_case = match StressCase::from_case(case) {
            Ok(stress_case) => stress_case,
            Err(message) => return CaseResult::Failed { message },
        };

        run_stress_child(StressWorker::Parser, &stress_case, WORKER_TIMEOUT)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(HARNESS_TIMEOUT)
    }
}

impl StressCase {
    /// Load one stress case from a generated suite case.
    pub fn from_case(case: &Case) -> Result<Self, String> {
        let file_type = FileType::from_path(&case.path)
            .ok_or_else(|| format!("unsupported stress file type: {}", case.path.display()))?;
        let expectation = if case.name.starts_with("damaged_") {
            StressExpectation::Recovery
        } else {
            StressExpectation::Valid
        };

        Ok(Self {
            name: case.name.clone(),
            path: case.path.clone(),
            file_type,
            expectation,
        })
    }
}

fn run_parser_stress(test: &StressCase) -> CaseResult {
    let source = match test.source() {
        Ok(source) => source,
        Err(message) => return CaseResult::Failed { message },
    };
    let file_size = source.len();
    let line_count = source.lines().count();
    let file_name = test.file_name();
    let file_id = FileId::from_logical_path(&test.logical_path());
    let file = File::from_text(
        file_id,
        file_name.clone(),
        Uri::from_string(&file_name),
        None,
        test.file_type,
        source.clone(),
    );
    let file = Arc::new(file);

    // parse and retain the string pool for semantic recovery checks
    let start = Instant::now();
    let strings = Arc::new(StringPool::new());
    let language_type =
        LanguageType::try_from(file.ty).expect("stress file type has no parser language");
    let mut parser = Parser::lex_file(file, language_type, strings.clone());
    let roots = parser.parse();
    let elapsed = start.elapsed();
    let has_errors = !parser.errors.is_empty();

    // valid cases must be clean
    if test.expectation == StressExpectation::Valid && has_errors {
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

    // damaged cases must report the damage and resume after it
    if test.expectation == StressExpectation::Recovery {
        if !has_errors {
            return CaseResult::Failed {
                message: "recovery case parsed without diagnostics".to_string(),
            };
        }

        if !contains_recovery_sentinel(&parser, &roots, &strings) {
            return CaseResult::Failed {
                message: format!("recovery case did not reach {RECOVERY_SENTINEL}"),
            };
        }
    }

    let metric = StressMetric::new(file_size, line_count, elapsed);
    eprintln!("{}", metric.format("parsed"));

    CaseResult::Passed
}

fn contains_recovery_sentinel(
    parser: &Parser,
    roots: &[LocalNodeId<Expression>],
    strings: &StringPool,
) -> bool {
    roots
        .iter()
        .any(|root| expression_names_sentinel(parser, *root, strings))
}

fn expression_names_sentinel(
    parser: &Parser,
    expression_id: LocalNodeId<Expression>,
    strings: &StringPool,
) -> bool {
    match parser.tree.get(expression_id) {
        Expression::Declaration(declaration_id) => {
            declaration_names_sentinel(parser, *declaration_id, strings)
        }
        Expression::Let { declarators, .. } => declarators.iter().any(|declarator_id| {
            let declarator = parser.tree.get(*declarator_id);

            pattern_names_sentinel(parser, declarator.pattern, strings)
        }),
        _ => false,
    }
}

fn declaration_names_sentinel(
    parser: &Parser,
    declaration_id: LocalNodeId<Declaration>,
    strings: &StringPool,
) -> bool {
    let name = match parser.tree.get(declaration_id) {
        Declaration::Type(declaration) => Some(declaration.name),
        Declaration::Struct(declaration) => Some(declaration.name),
        Declaration::Class(declaration) => declaration.name,
        Declaration::Enum(declaration) => declaration.name,
        Declaration::Interface(declaration) => declaration.name,
        Declaration::Function(declaration) => declaration.name,
        Declaration::Global(_) | Declaration::Module(_) | Declaration::Extension(_) => None,
    };

    name.is_some_and(|name| name_is_sentinel(name, strings))
}

fn pattern_names_sentinel(
    parser: &Parser,
    pattern_id: LocalNodeId<Pattern>,
    strings: &StringPool,
) -> bool {
    match parser.tree.get(pattern_id) {
        Pattern::Binding { name, .. } => strings.get(*name) == RECOVERY_SENTINEL,
        Pattern::Must(pattern) => pattern_names_sentinel(parser, pattern.clone(), strings),
        Pattern::Expression { value } => expression_names_sentinel(parser, value.clone(), strings),
        Pattern::Assign { pattern, .. }
        | Pattern::BorrowOf { right: pattern, .. }
        | Pattern::MoveOf { right: pattern, .. }
        | Pattern::DereferenceOf { right: pattern } => {
            pattern_names_sentinel(parser, *pattern, strings)
        }
        _ => false,
    }
}

fn name_is_sentinel(name: Name, strings: &StringPool) -> bool {
    strings.get(name.string()) == RECOVERY_SENTINEL
}
