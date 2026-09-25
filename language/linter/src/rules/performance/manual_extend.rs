use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer extend over loops that insert every iterated value.
    pub MANUAL_EXTEND {
        id: "manual-extend",
        summary: "Prefer extend over loops that insert every iterated value",
        explanation: r#"
A loop that only inserts each source value repeats the collection's bulk extension operation.
Instead, you SHOULD call `extend` with the source iterable.
"#,
        example: {
            reported: r#"
function append(target: int32[], source: int32[]): void {
    for (const value of source) {
        target.push(value);
    }
}
"#,
            accepted: r#"
function append(target: int32[], source: int32[]): void {
    target.extend(source);
}
"#,
        },
        provenance: [],
        category: Performance,
        level: Warning,
        fixable: Suggestion,
        check: DirModule(check),
    }
}

/// The bindings forwarded from one iteration into a collection insertion.
#[derive(Debug, Clone, Copy)]
enum InsertionBinding {
    /// One iterated value.
    Value {
        /// The value inserted by each iteration.
        value: dir::GlobalSymbolId,
    },
    /// One iterated key and value pair.
    Entry {
        /// The key inserted by each iteration.
        key: dir::GlobalSymbolId,
        /// The value inserted by each iteration.
        value: dir::GlobalSymbolId,
    },
}

impl InsertionBinding {
    /// Select direct value or entry bindings from one for-of loop.
    fn select(
        module: &DirModule<'_>,
        binding: &dir::ForEachBinding,
    ) -> Result<Option<Self>, ProviderError> {
        let dir::ForEachBinding::Pattern {
            pattern,
            keyword: Some(_),
        } = binding
        else {
            return Ok(None);
        };
        let view = module.view();

        // select one direct value binding
        if matches!(
            view.get(*pattern),
            dir::Pattern::Binding { pattern: None, .. }
        ) {
            let symbol = module.declaration_symbol(*pattern)?;

            return Ok(Some(Self::Value { value: symbol }));
        }

        // select one direct key and value tuple binding
        let dir::Pattern::Tuple { fields } = view.get(*pattern) else {
            return Ok(None);
        };
        let [key, value] = fields.as_slice() else {
            return Ok(None);
        };
        let (
            dir::PatternField::Positional { pattern: key },
            dir::PatternField::Positional { pattern: value },
        ) = (view.get(*key), view.get(*value))
        else {
            return Ok(None);
        };
        if !matches!(view.get(*key), dir::Pattern::Binding { pattern: None, .. })
            || !matches!(
                view.get(*value),
                dir::Pattern::Binding { pattern: None, .. }
            )
        {
            return Ok(None);
        }
        let key = module.declaration_symbol(*key)?;
        let value = module.declaration_symbol(*value)?;

        Ok(Some(Self::Entry { key, value }))
    }

    /// Return whether one canonical insertion forwards these bindings exactly.
    fn matches(
        self,
        module: &DirModule<'_>,
        member: dir::LanguageMember,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> Result<bool, ProviderError> {
        let view = module.view();

        match self {
            // match one value insertion supported by Extend
            Self::Value {
                value: value_symbol,
            } => {
                let [argument] = arguments else {
                    return Ok(false);
                };
                let dir::Argument::Positional { value: argument } = view.get(*argument) else {
                    return Ok(false);
                };
                if module.selected_symbol(*argument)? != Some(value_symbol) {
                    return Ok(false);
                }

                // select the canonical insertion method for this collection
                let method = match member.owner {
                    dir::LanguageItem::Array
                    | dir::LanguageItem::BinaryHeap
                    | dir::LanguageItem::ConcurrentQueue
                    | dir::LanguageItem::SmallArray => "push",
                    dir::LanguageItem::ConcurrentSet
                    | dir::LanguageItem::Set
                    | dir::LanguageItem::SortedSet => "add",
                    dir::LanguageItem::Deque | dir::LanguageItem::LinkedList => "pushBack",
                    dir::LanguageItem::Slab => "insert",
                    _ => return Ok(false),
                };

                Ok(member == member.owner.member(method))
            }
            // match one key and value insertion supported by Extend
            Self::Entry { key, value } => {
                let [key_argument, value_argument] = arguments else {
                    return Ok(false);
                };
                let (
                    dir::Argument::Positional { value: key_value },
                    dir::Argument::Positional { value: value_value },
                ) = (view.get(*key_argument), view.get(*value_argument))
                else {
                    return Ok(false);
                };
                let is_map = matches!(
                    member.owner,
                    dir::LanguageItem::ConcurrentMap
                        | dir::LanguageItem::Map
                        | dir::LanguageItem::SortedMap
                );

                Ok(is_map
                    && member == member.owner.member("insert")
                    && module.selected_symbol(*key_value)? == Some(key)
                    && module.selected_symbol(*value_value)? == Some(value))
            }
        }
    }
}

/// Report for-of loops whose sole action inserts every bound value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect synchronous for-of loops with one direct binding
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let Some(loop_) = module.for_of(expression) else {
            continue;
        };
        if loop_.asynchrony != dir::Asynchrony::Sync {
            continue;
        }
        let Some(binding) = InsertionBinding::select(module, loop_.binding)? else {
            continue;
        };
        let Some(action) = view.get(loop_.body).only_expression() else {
            continue;
        };

        // require one canonical insertion of the exact loop bindings
        let Some(insertion) = module.member_call(action) else {
            continue;
        };
        if insertion.is_optional() {
            continue;
        }
        let Some(member) = module.language_member(action)? else {
            continue;
        };
        if !binding.matches(module, member, insertion.arguments)? {
            continue;
        }
        if !module.is_speculatable_expression(insertion.receiver)? {
            continue;
        }
        let target = module.access_resolution(insertion.receiver);
        let source = module.access_resolution(loop_.iterator);
        if target.is_some() && target == source {
            continue;
        }

        // replace the loop with the canonical bulk extension
        let span = module.source_extent(expression.into_any())?;
        let mut diagnostic = lint.diagnostic("loop inserts every source value", span);
        if let Some(suggestion) =
            suggestion(module, lint, expression, insertion.receiver, loop_.iterator)?
        {
            diagnostic = diagnostic.suggestion(suggestion);
        }
        output.report(diagnostic);
    }

    Ok(output)
}

/// Build one extend call from an insertion loop.
fn suggestion(
    module: &DirModule<'_>,
    lint: &Lint,
    expression: dir::LocalNodeId<dir::Expression>,
    target: dir::LocalNodeId<dir::Expression>,
    source: dir::LocalNodeId<dir::Expression>,
) -> Result<Option<DiagnosticSuggestion>, ProviderError> {
    let extent = module.source_extent(expression.into_any())?;
    let target_extent = module.source_extent(target.into_any())?;
    let source_extent = module.source_extent(source.into_any())?;
    if module.has_unretained_comment(extent, &[target_extent, source_extent])? {
        return Ok(None);
    }

    // preserve authored target and source expressions
    let target = module.expression_source(target, dir::OperatorPrecedence::Postfix)?;
    let source = module.source(source_extent)?;
    let replacement = format!("{target}.extend({source});");
    let patch = Patch::replace(extent, replacement);
    let suggestion = lint.suggestion("extend the collection", patch)?;

    Ok(Some(suggestion))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Replace an Array push loop with extend.
    #[test]
    fn test_replaces_array_push_loop() {
        let session = TestSession::dir(
            &MANUAL_EXTEND,
            r#"
function append(target: int32[], source: int32[]): void {
    for (const value of source) {
        target.push(value);
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-extend]: loop inserts every source value
 ──▶ main.tspp:2:5
  │
1 │ function append(target: int32[], source: int32[]): void {
2 │     for (const value of source) {
  │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
3 │         target.push(value);
  │         ^^^^^^^^^^^^^^^^^^^
4 │     }
  │     ^
5 │ }
  │

 = suggestion: extend the collection (requires review)
--- a/main.tspp
+++ b/main.tspp

    1│ function append(target: int32[], source: int32[]): void {
-   2│     for (const value of source) {
-   3│         target.push(value);
-   4│     }
+   2│     target.extend(source);
    5│ }
"#,
        );
        session.assert_suggestions(
            r#"
function append(target: int32[], source: int32[]): void {
    target.extend(source);
}
"#,
        );
    }

    /// Accept a loop that transforms values before pushing them.
    #[test]
    fn test_accepts_transformed_push() {
        let session = TestSession::dir(
            &MANUAL_EXTEND,
            r#"
function append(target: int32[], source: int32[]): void {
    for (const value of source) {
        target.push(value + 1);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a loop with another action.
    #[test]
    fn test_accepts_additional_action() {
        let session = TestSession::dir(
            &MANUAL_EXTEND,
            r#"
function append(target: int32[], source: int32[]): void {
    for (const value of source) {
        target.push(value);
        target.push(value);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a user-defined push method.
    #[test]
    fn test_accepts_user_push() {
        let session = TestSession::dir(
            &MANUAL_EXTEND,
            r#"
class Values {
    push(value: int32): void {
        // intentionally empty
    }
}

function append(target: Values, source: int32[]): void {
    for (const value of source) {
        target.push(value);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a Set add loop with extend.
    #[test]
    fn test_replaces_set_add_loop() {
        let session = TestSession::dir(
            &MANUAL_EXTEND,
            r#"
function append(target: Set<int32>, source: int32[]): void {
    for (const value of source) {
        target.add(value);
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function append(target: Set<int32>, source: int32[]): void {
    target.extend(source);
}
"#,
        );
    }

    /// Replace a Map entry insertion loop with extend.
    #[test]
    fn test_replaces_map_insert_loop() {
        let session = TestSession::dir(
            &MANUAL_EXTEND,
            r#"
function append(target: Map<string, int32>, source: (string, int32)[]): void {
    for (const (key, value) of source) {
        target.insert(key, value);
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
function append(target: Map<string, int32>, source: (string, int32)[]): void {
    target.extend(source);
}
"#,
        );
    }

    /// Replace a Deque pushBack loop with extend.
    #[test]
    fn test_replaces_deque_push_back_loop() {
        let session = TestSession::dir(
            &MANUAL_EXTEND,
            r#"
import { Deque } from "tspp:collections";

function append(target: Deque<int32>, source: int32[]): void {
    for (const value of source) {
        target.pushBack(value);
    }
}
"#,
        );

        session.assert_suggestions(
            r#"
import { Deque } from "tspp:collections";

function append(target: Deque<int32>, source: int32[]): void {
    target.extend(source);
}
"#,
        );
    }
}
