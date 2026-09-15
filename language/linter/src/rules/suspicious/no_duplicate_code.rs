use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{ModuleId, Span};

use crate::rules::declare_lint;
use crate::{DirModule, DirProgram, Lint, LintOutput, LintResult};

/// The minimum structural node count for one duplicated code region.
const MINIMUM_DUPLICATE_NODES: usize = 12;
/// The minimum statement count for one duplicated partial region.
const MINIMUM_DUPLICATE_STATEMENTS: usize = 2;

declare_lint! {
    /// Disallow repeated nontrivial code regions.
    pub NO_DUPLICATE_CODE {
        id: "no-duplicate-code",
        summary: "Disallow repeated nontrivial code regions",
        explanation: r#"
Multiple nontrivial callables or statement sequences perform the same operations.
Instead, you SHOULD extract the repeated statements into one callable.
"#,
        example: {
            reported: r#"
function incrementLeft(value: int32): int32 {
    const next = value + 1;
    return next * 2;
}

function incrementRight(input: int32): int32 {
    const result = input + 1;
    return result * 2;
}
"#,
            accepted: r#"
function increment(value: int32): int32 {
    const next = value + 1;
    return next * 2;
}
"#,
        },
        provenance: [SonarJs("no-identical-functions")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        indexes: [Code],
        check: DirProgram(check),
    }
}

/// One callable implementation considered by duplicate-code detection.
#[derive(Debug, Clone, Copy)]
struct CallableImplementation {
    /// The callable root.
    callable: dir::GlobalNodeIdAny,
    /// The callable body.
    body: dir::GlobalNodeIdAny,
    /// The body source extent.
    span: Span,
}

/// One block's statements and cumulative structural sizes.
#[derive(Debug)]
struct StatementSequence {
    /// The module containing every statement.
    module_id: ModuleId,
    /// The statements in execution order.
    statements: Vec<dir::GlobalNodeIdAny>,
    /// The cumulative structural node counts between statement positions.
    node_offsets: Vec<usize>,
}

/// One minimum-sized statement range used to locate repeated sections.
#[derive(Debug, Clone, Copy)]
struct StatementSeed {
    /// The containing statement sequence.
    sequence_index: usize,
    /// The first statement index.
    start: usize,
    /// The number of statements in the hashed seed.
    statement_count: usize,
    /// The source extent of the minimum range.
    span: Span,
}

impl CallableImplementation {
    /// Collect substantial authored callable implementations from one module.
    fn collect(module: DirModule<'_>) -> Result<Vec<Self>, ProviderError> {
        let view = module.view();
        let mut implementations = Vec::new();

        // retain concrete callables above the structural size threshold
        for callable in view.iter_node_ids() {
            let Some(body) = module.callable_body(callable) else {
                continue;
            };
            let body = body.into_any();
            let fingerprint = module.code_fingerprint(body)?;
            if fingerprint.node_count() < MINIMUM_DUPLICATE_NODES as u32 {
                continue;
            }

            implementations.push(Self {
                callable: callable.into_global(module.id),
                body: body.into_global(module.id),
                span: module.source_extent(body)?,
            });
        }

        Ok(implementations)
    }
}

impl StatementSequence {
    /// Build one statement sequence.
    fn new(
        program: &DirProgram<'_>,
        statements: Vec<dir::GlobalNodeIdAny>,
    ) -> Result<Self, ProviderError> {
        let Some(first) = statements.first() else {
            return Err(ProviderError::internal(
                "duplicate statement sequence is empty",
            ));
        };
        let module_id = first.module_id;
        if statements
            .iter()
            .any(|statement| statement.module_id != module_id)
        {
            return Err(ProviderError::internal(
                "duplicate statement sequence spans multiple modules",
            ));
        }

        // initialize cumulative structural node counts
        let mut node_offsets = Vec::with_capacity(statements.len() + 1);
        let mut nodes = 0;
        node_offsets.push(0);

        // accumulate each statement's structural size
        for statement in &statements {
            let fingerprint = program.dir.code_fingerprint(&[*statement])?;
            nodes += fingerprint.node_count() as usize;
            node_offsets.push(nodes);
        }

        Ok(Self {
            module_id,
            statements,
            node_offsets,
        })
    }

    /// Collect executable statement sequences from every package-owned block.
    fn collect(program: &DirProgram<'_>) -> Result<Vec<Self>, ProviderError> {
        let mut sequences = Vec::new();

        // divide each block at declaration and module syntax
        for module in program.owned_modules() {
            let view = module.view();
            for (_, block) in view.iter_nodes::<dir::Block>() {
                let mut statements = Vec::new();
                for statement in block.iter_expressions() {
                    if matches!(
                        view.get(statement),
                        dir::Expression::Declaration(_)
                            | dir::Expression::Import { .. }
                            | dir::Expression::Export { .. }
                    ) {
                        if statements.len() >= MINIMUM_DUPLICATE_STATEMENTS {
                            sequences.push(Self::new(program, statements)?);
                        }
                        statements = Vec::new();
                    } else {
                        statements.push(statement.into_global_any(module.id));
                    }
                }

                // retain the executable suffix after the final declaration
                if statements.len() >= MINIMUM_DUPLICATE_STATEMENTS {
                    sequences.push(Self::new(program, statements)?);
                }
            }
        }

        Ok(sequences)
    }

    /// Return the shortest statement end meeting the duplicate size threshold.
    fn minimum_end(&self, start: usize) -> Option<usize> {
        let statement_end = start + MINIMUM_DUPLICATE_STATEMENTS;
        let required_nodes = self.node_offsets[start] + MINIMUM_DUPLICATE_NODES;
        let node_end = self
            .node_offsets
            .partition_point(|nodes| *nodes < required_nodes);
        let end = statement_end.max(node_end);

        (end <= self.statements.len()).then_some(end)
    }

    /// Return the source extent of one nonempty statement range.
    fn span(
        &self,
        program: &DirProgram<'_>,
        start: usize,
        end: usize,
    ) -> Result<Span, ProviderError> {
        let Some(statements) = self.statements.get(start..end) else {
            return Err(ProviderError::internal(
                "duplicate statement range is invalid",
            ));
        };
        let (Some(first), Some(last)) = (statements.first(), statements.last()) else {
            return Err(ProviderError::internal(
                "duplicate statement range is empty",
            ));
        };

        // merge the first and last source extents
        let module = program.module(self.module_id)?;
        let first = module.source_extent(first.local_id)?;
        let last = module.source_extent(last.local_id)?;

        Ok(first.merge(last))
    }
}

impl StatementSeed {
    /// Return the exact common statement prefix length of two nonoverlapping seeds.
    fn matching_len(
        self,
        right: Self,
        program: &DirProgram<'_>,
        sequences: &[StatementSequence],
    ) -> Result<usize, ProviderError> {
        let left_sequence = &sequences[self.sequence_index];
        let right_sequence = &sequences[right.sequence_index];
        let mut left_end = left_sequence.statements.len();
        let mut right_end = right_sequence.statements.len();

        // prevent repeated regions in the same block from overlapping
        if self.sequence_index == right.sequence_index {
            let distance = right.start - self.start;
            left_end = left_end.min(self.start + distance);
            right_end = right_end.min(right.start + distance);
        }
        let left_statements = &left_sequence.statements[self.start..left_end];
        let right_statements = &right_sequence.statements[right.start..right_end];

        program
            .dir
            .alpha_prefix_len(left_statements, right_statements)
    }
}

/// Report substantial repeated callable implementations and statement sections.
fn check(program: &DirProgram<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // report complete callable implementations before their internal sections
    let duplicate_callables = report_duplicate_callables(program, lint, &mut output)?;

    // report maximal repeated statement sections outside complete duplicates
    report_duplicate_sections(program, lint, &duplicate_callables, &mut output)?;

    Ok(output)
}

/// Report later substantial callables that repeat an earlier implementation.
fn report_duplicate_callables(
    program: &DirProgram<'_>,
    lint: &Lint,
    output: &mut LintOutput,
) -> Result<Vec<(Span, Span)>, ProviderError> {
    let mut implementations_by_fingerprint =
        FxIndexMap::<dir::CodeFingerprint, Vec<CallableImplementation>>::default();
    let mut duplicates = Vec::<(Span, Span)>::new();
    let mut implementations = Vec::new();

    // collect package-owned implementations in stable source order
    for module in program.owned_modules() {
        implementations.extend(CallableImplementation::collect(module)?);
    }
    implementations.sort_by_key(|implementation| {
        (
            implementation.span.file,
            implementation.span.start,
            std::cmp::Reverse(implementation.span.end),
        )
    });

    // group implementations by name-insensitive structural fingerprint
    for implementation in implementations {
        let nodes = [implementation.body];
        let fingerprint = program.dir.code_fingerprint(&nodes)?;
        let candidates = implementations_by_fingerprint
            .entry(fingerprint)
            .or_default();

        // confirm every hash candidate with exact alpha-equivalence
        let mut duplicate = None;
        for earlier in candidates.iter() {
            let left_callable = [earlier.callable];
            let right_callable = [implementation.callable];
            let Some(mut comparison) = program
                .dir
                .alpha_comparison(&left_callable, &right_callable)?
            else {
                continue;
            };
            let left = [earlier.body];
            let right = [implementation.body];
            if comparison.compare_nodes(&left, &right)? {
                duplicate = Some(earlier.span);
                break;
            }
        }

        // report the later body while retaining it for subsequent matches
        if let Some(earlier) = duplicate
            && !duplicates.iter().any(|(outer_earlier, outer_repeated)| {
                outer_earlier.contains_span(earlier)
                    && outer_repeated.contains_span(implementation.span)
            })
        {
            let diagnostic = lint
                .diagnostic(
                    "callable repeats an earlier implementation",
                    implementation.span,
                )
                .primary("repeated implementation")
                .label(earlier, "earlier implementation");
            output.report(diagnostic);
            duplicates.push((earlier, implementation.span));
        }
        candidates.push(implementation);
    }

    Ok(duplicates)
}

/// Report maximal repeated statement sections outside complete duplicated callables.
fn report_duplicate_sections(
    program: &DirProgram<'_>,
    lint: &Lint,
    duplicate_callables: &[(Span, Span)],
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let sequences = StatementSequence::collect(program)?;
    let mut seeds_by_fingerprint =
        FxIndexMap::<dir::CodeFingerprint, Vec<StatementSeed>>::default();
    let mut reported = Vec::<(Span, Span)>::new();
    let mut seeds = Vec::new();

    // collect one minimum-sized seed from every statement position
    for (sequence_index, sequence) in sequences.iter().enumerate() {
        for start in 0..sequence.statements.len() {
            let Some(end) = sequence.minimum_end(start) else {
                continue;
            };
            let span = sequence.span(program, start, end)?;
            let seed = StatementSeed {
                sequence_index,
                start,
                statement_count: end - start,
                span,
            };
            let fingerprint = program
                .dir
                .code_fingerprint(&sequence.statements[start..end])?;
            seeds.push((fingerprint, seed));
        }
    }
    seeds.sort_by_key(|(_, seed)| (seed.span.file, seed.span.start, seed.span.end));

    // select the earlier candidate sharing the longest exact continuation
    for (fingerprint, seed) in seeds {
        let mut best = None;
        if let Some(candidates) = seeds_by_fingerprint.get(&fingerprint) {
            for earlier in candidates.iter().copied() {
                if earlier.span.intersects(seed.span) {
                    continue;
                }
                let matched = earlier.matching_len(seed, program, &sequences)?;
                if matched < seed.statement_count {
                    continue;
                }
                if best.is_none_or(|(_, best_matched)| matched > best_matched) {
                    best = Some((earlier, matched));
                }
            }
        }

        // index this seed for later occurrences
        seeds_by_fingerprint
            .entry(fingerprint)
            .or_default()
            .push(seed);
        let Some((earlier, matched)) = best else {
            continue;
        };
        let earlier_sequence = &sequences[earlier.sequence_index];
        let repeated_sequence = &sequences[seed.sequence_index];
        let earlier_span =
            earlier_sequence.span(program, earlier.start, earlier.start + matched)?;
        let repeated_span = repeated_sequence.span(program, seed.start, seed.start + matched)?;

        // suppress sections already represented by a wider duplicate report
        if duplicate_callables
            .iter()
            .chain(&reported)
            .any(|(left, right)| {
                left.contains_span(earlier_span) && right.contains_span(repeated_span)
            })
        {
            continue;
        }

        // report the later maximal section against its earlier occurrence
        let diagnostic = lint
            .diagnostic("statement sequence repeats earlier code", repeated_span)
            .primary("repeated statements")
            .label(earlier_span, "earlier statements");
        output.report(diagnostic);
        reported.push((earlier_span, repeated_span));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report substantial functions modulo parameter and local binding names.
    #[test]
    fn test_reports_alpha_equivalent_functions() {
        let session = TestSession::dir(
            &NO_DUPLICATE_CODE,
            r#"
function normalizeLeft(value: int32): int32 {
    const incremented = value + 1;
    const doubled = incremented * 2;
    return doubled - 3;
}

function normalizeRight(input: int32): int32 {
    const added = input + 1;
    const scaled = added * 2;
    return scaled - 3;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-code]: callable repeats an earlier implementation
  ──▶ main.ds:7:46
   │
 1 │ function normalizeLeft(value: int32): int32 {
   │                                             - earlier implementation
 2 │     const incremented = value + 1;
   │     ------------------------------
 3 │     const doubled = incremented * 2;
   │     --------------------------------
 4 │     return doubled - 3;
   │     -------------------
 5 │ }
   │ -
 6 │
 7 │ function normalizeRight(input: int32): int32 {
   │                                              ^ repeated implementation
 8 │     const added = input + 1;
   │     ^^^^^^^^^^^^^^^^^^^^^^^^
 9 │     const scaled = added * 2;
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^
10 │     return scaled - 3;
   │     ^^^^^^^^^^^^^^^^^^
11 │ }
   │ ^
   │
"#,
        );
    }

    /// Report substantial methods modulo their names and local bindings.
    #[test]
    fn test_reports_alpha_equivalent_methods() {
        let session = TestSession::dir(
            &NO_DUPLICATE_CODE,
            r#"
class Calculator {
    normalizeLeft(value: int32): int32 {
        const incremented = value + 1;
        const doubled = incremented * 2;
        return doubled - 3;
    }

    normalizeRight(input: int32): int32 {
        const added = input + 1;
        const scaled = added * 2;
        return scaled - 3;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-code]: callable repeats an earlier implementation
  ──▶ main.ds:8:41
   │
 1 │ class Calculator {
 2 │     normalizeLeft(value: int32): int32 {
   │                                        - earlier implementation
 3 │         const incremented = value + 1;
   │         ------------------------------
 4 │         const doubled = incremented * 2;
   │         --------------------------------
 5 │         return doubled - 3;
   │         -------------------
 6 │     }
   │     -
 7 │
 8 │     normalizeRight(input: int32): int32 {
   │                                         ^ repeated implementation
 9 │         const added = input + 1;
   │         ^^^^^^^^^^^^^^^^^^^^^^^^
10 │         const scaled = added * 2;
   │         ^^^^^^^^^^^^^^^^^^^^^^^^^
11 │         return scaled - 3;
   │         ^^^^^^^^^^^^^^^^^^
12 │     }
   │     ^
13 │ }
   │
"#,
        );
    }

    /// Report substantial functions that call through the same imported namespace.
    #[test]
    fn test_reports_namespace_calls() {
        let session = TestSession::dir(
            &NO_DUPLICATE_CODE,
            r#"
import * as assert from "destack:assert";

function checkLeft(value: int32): void {
    assert.assertEqual(value, 1);
    assert.assertEqual(value, 2);
}

function checkRight(input: int32): void {
    assert.assertEqual(input, 1);
    assert.assertEqual(input, 2);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-code]: callable repeats an earlier implementation
  ──▶ main.ds:8:41
   │
 1 │ import * as assert from "destack:assert";
 2 │
 3 │ function checkLeft(value: int32): void {
   │                                        - earlier implementation
 4 │     assert.assertEqual(value, 1);
   │     -----------------------------
 5 │     assert.assertEqual(value, 2);
   │     -----------------------------
 6 │ }
   │ -
 7 │
 8 │ function checkRight(input: int32): void {
   │                                         ^ repeated implementation
 9 │     assert.assertEqual(input, 1);
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
10 │     assert.assertEqual(input, 2);
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 │ }
   │ ^
   │
"#,
        );
    }

    /// Report a maximal repeated statement section within one callable.
    #[test]
    fn test_reports_repeated_statement_section() {
        let session = TestSession::dir(
            &NO_DUPLICATE_CODE,
            r#"
declare function record(value: int32): void;
declare function flush(): void;
function process(): void {
    record(0);
    const left = 1;
    record(left);
    record(left + 1);
    record(left + 2);
    flush();
    record(100);
    const right = 1;
    record(right);
    record(right + 1);
    record(right + 2);
    flush();
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-code]: statement sequence repeats earlier code
  ──▶ main.ds:11:5
   │
 3 │ function process(): void {
 4 │     record(0);
 5 │     const left = 1;
   │     --------------- earlier statements
 6 │     record(left);
   │     -------------
 7 │     record(left + 1);
   │     -----------------
 8 │     record(left + 2);
   │     -----------------
 9 │     flush();
   │     -------
10 │     record(100);
11 │     const right = 1;
   │     ^^^^^^^^^^^^^^^^ repeated statements
12 │     record(right);
   │     ^^^^^^^^^^^^^^
13 │     record(right + 1);
   │     ^^^^^^^^^^^^^^^^^^
14 │     record(right + 2);
   │     ^^^^^^^^^^^^^^^^^^
15 │     flush();
   │     ^^^^^^^
16 │ }
   │
"#,
        );
    }

    /// Report substantial functions repeated across reachable modules.
    #[test]
    fn test_reports_across_modules() {
        let session = TestSession::dir_files(
            &NO_DUPLICATE_CODE,
            "main.ds",
            r#"
import "./worker.ds";

function normalizeLeft(value: int32): int32 {
    const incremented = value + 1;
    const doubled = incremented * 2;
    return doubled - 3;
}
"#,
            &[(
                "worker.ds",
                r#"
function normalizeRight(input: int32): int32 {
    const added = input + 1;
    const scaled = added * 2;
    return scaled - 3;
}
"#,
            )],
        );

        session.assert_diagnostics(
            r#"
warning[no-duplicate-code]: callable repeats an earlier implementation
 ──▶ worker.ds:1:46
  │
1 │ function normalizeRight(input: int32): int32 {
  │                                              ^ repeated implementation
2 │     const added = input + 1;
  │     ^^^^^^^^^^^^^^^^^^^^^^^^
3 │     const scaled = added * 2;
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^
4 │     return scaled - 3;
  │     ^^^^^^^^^^^^^^^^^^
5 │ }
  │ ^
  │

 ──▶ main.ds:3:45
  │
2 │
3 │ function normalizeLeft(value: int32): int32 {
  │                                             - earlier implementation
4 │     const incremented = value + 1;
  │     ------------------------------
5 │     const doubled = incremented * 2;
  │     --------------------------------
6 │     return doubled - 3;
  │     -------------------
7 │ }
  │ -
  │
"#,
        );
    }

    /// Accept similar sections that read distinct bindings declared outside them.
    #[test]
    fn test_accepts_sections_with_different_free_bindings() {
        let session = TestSession::dir(
            &NO_DUPLICATE_CODE,
            r#"
declare function record(value: int32): void;
declare function flush(): void;
function process(left: int32, right: int32): void {
    record(left);
    flush();
    record(left);
    record(right);
    flush();
    record(right);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept similar functions that select different free declarations.
    #[test]
    fn test_accepts_different_free_declarations() {
        let session = TestSession::dir(
            &NO_DUPLICATE_CODE,
            r#"
declare function increment(value: int32): int32;
declare function decrement(value: int32): int32;

function normalizeLeft(value: int32): int32 {
    const adjusted = increment(value);
    const doubled = adjusted * 2;
    return doubled - 3;
}

function normalizeRight(value: int32): int32 {
    const adjusted = decrement(value);
    const doubled = adjusted * 2;
    return doubled - 3;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept structurally equal bodies under different function signatures.
    #[test]
    fn test_accepts_different_signatures() {
        let session = TestSession::dir(
            &NO_DUPLICATE_CODE,
            r#"
function normalizeNumber(value: int32): int32 {
    const first = value;
    const second = first;
    return second;
}

function normalizeText(value: string): string {
    const first = value;
    const second = first;
    return second;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept small functions below the structural threshold.
    #[test]
    fn test_accepts_small_functions() {
        let session = TestSession::dir(
            &NO_DUPLICATE_CODE,
            r#"
function left(value: int32): int32 {
    return value + 1;
}

function right(value: int32): int32 {
    return value + 1;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
