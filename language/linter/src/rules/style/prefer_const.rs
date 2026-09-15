use std::collections::BTreeMap;

use destack_dir as dir;
use destack_source::{NodeSpanRegion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require const for bindings never reassigned after initialization.
    pub PREFER_CONST {
        id: "prefer-const",
        summary: "Require const for bindings never reassigned after initialization",
        explanation: r#"
A `let` binding that is never reassigned permits a write the function does not perform.
Instead, you SHOULD declare the binding with `const`.

`const` freezes directly stored values while preserving the access carried by references.
"#,
        example: {
            reported: r#"
function identity(value: int32): int32 {
    let result = value;
    return result;
}
"#,
            accepted: r#"
function identity(value: int32): int32 {
    const result = value;
    return result;
}
"#,
        },
        provenance: [Eslint("prefer-const")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One mutable binding declared by a let expression.
struct LetBinding {
    /// The let expression that owns the binding.
    declaration: dir::LocalNodeId<dir::Expression>,
    /// The node that introduces the binding.
    node: dir::LocalNodeIdAny,
    /// The binding symbol.
    symbol: dir::LocalSymbolId,
    /// Whether the declarator initializes the binding.
    is_initialized: bool,
}

/// Report mutable bindings whose storage does not require further writes.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();
    let mut let_bindings = Vec::new();
    let mut binding_uses = BTreeMap::<dir::LocalSymbolId, dir::BindingUse>::new();
    let mut binding_writes = BTreeMap::<dir::LocalSymbolId, Vec<dir::LocalNodeIdAny>>::new();

    // index uses and writes by their local binding
    for occurrence in module.flows.binding_occurrences() {
        // ignore foreign declarations
        if occurrence.symbol.module_id != module.flows.module_id {
            continue;
        }

        // merge the occurrence uses
        let symbol = occurrence.symbol.local_id;
        *binding_uses.entry(symbol).or_default() |= occurrence.uses;

        // retain every write site
        if occurrence.uses.contains(dir::BindingUse::WRITE) {
            binding_writes
                .entry(symbol)
                .or_default()
                .push(occurrence.node);
        }
    }

    // collect mutable local bindings under their owning let declarations
    for symbol in module.bindings.symbol_ids() {
        let binding = module.bindings.get_symbol(symbol);
        if binding.kind != dir::SymbolKind::Variable
            || binding.binding_mutability != Some(dir::Mutability::Mutable)
        {
            continue;
        }
        let Some(declaration) = binding.declaration else {
            continue;
        };
        let node = declaration.local_id;
        let Some(declarator) = view.ancestor::<dir::Declarator>(node) else {
            continue;
        };
        let Some(declaration) = view.ancestor::<dir::Expression>(declarator.into_any()) else {
            continue;
        };
        if !matches!(
            view.get(declaration),
            dir::Expression::Let {
                mutability: dir::Mutability::Mutable,
                ..
            }
        ) {
            continue;
        }

        let_bindings.push(LetBinding {
            declaration,
            node,
            symbol,
            is_initialized: view.get(declarator).value.is_some(),
        });
    }

    // inspect each mutable declaration once in source order
    for declaration in view.iter_node_ids_of_type::<dir::Expression>() {
        let dir::Expression::Let {
            mutability: dir::Mutability::Mutable,
            ..
        } = view.get(declaration)
        else {
            continue;
        };
        let declared = let_bindings
            .iter()
            .filter(|binding| binding.declaration == declaration)
            .collect::<Vec<_>>();
        if declared.is_empty() {
            continue;
        }

        // retain bindings with no mutable storage use and no reassignment
        let mut candidates = Vec::new();
        for binding in &declared {
            let is_mutable = binding_uses
                .get(&binding.symbol)
                .copied()
                .is_some_and(|uses| uses.contains(dir::BindingUse::MUTATE));
            if is_mutable {
                continue;
            }
            let writes = binding_writes.get(&binding.symbol);

            // initialized bindings qualify only when their slot is never written again
            if binding.is_initialized {
                if writes.is_none() {
                    candidates.push(*binding);
                }
                continue;
            }

            // one later write qualifies only when it initializes in the declaration's block
            let declaration_block = view.ancestor::<dir::Block>(binding.node);
            let is_initialized_once = writes.is_some_and(|writes| {
                writes.len() == 1 && view.ancestor::<dir::Block>(writes[0]) == declaration_block
            });
            if is_initialized_once {
                candidates.push(*binding);
            }
        }
        if candidates.is_empty() {
            continue;
        }

        // replace one fully initialized declaration when every binding qualifies
        let can_fix = candidates.len() == declared.len()
            && declared.iter().all(|binding| binding.is_initialized);
        if can_fix {
            let keyword = module.main_span(declaration.into_any())?;
            let patch = Patch::replace(keyword, "const");
            let suggestion = lint.fix("declare the binding with const", patch)?;
            let diagnostic = lint
                .diagnostic("binding is never reassigned", keyword)
                .suggestion(suggestion);
            output.report(diagnostic);

            continue;
        }

        // report individual bindings when rewriting the declaration would change siblings
        for binding in candidates {
            let span = module.main_span(binding.node)?;
            output.report(lint.diagnostic("binding is never reassigned", span));
        }
    }

    // inspect mutable bindings initialized by each for-of iteration
    for expression in view.iter_node_ids_of_type::<dir::Expression>() {
        let dir::Expression::ForEach {
            binding:
                dir::ForEachBinding::Pattern {
                    pattern,
                    keyword: Some(dir::BindingKeyword::Let),
                },
            ..
        } = view.get(expression)
        else {
            continue;
        };
        let declared = module
            .symbols_declared_within(pattern.into_any())
            .map(|symbol| symbol.local_id)
            .filter(|symbol| {
                let binding = module.bindings.get_symbol(*symbol);
                binding.kind == dir::SymbolKind::Variable
                    && binding.binding_mutability == Some(dir::Mutability::Mutable)
            })
            .collect::<Vec<_>>();
        if declared.is_empty() {
            continue;
        }

        // require every binding under the shared keyword to remain immutable
        let is_const = declared.iter().all(|symbol| {
            let is_mutable = binding_uses
                .get(symbol)
                .copied()
                .is_some_and(|uses| uses.contains(dir::BindingUse::MUTATE));
            let is_written = binding_writes.contains_key(symbol);

            !is_mutable && !is_written
        });
        if !is_const {
            continue;
        }

        // replace the shared declaration keyword
        let keyword =
            module.source_region(expression.into_any(), NodeSpanRegion::BindingKeyword)?;
        let patch = Patch::replace(keyword, "const");
        let suggestion = lint.fix("declare the binding with const", patch)?;
        let diagnostic = lint
            .diagnostic("binding is never reassigned", keyword)
            .suggestion(suggestion);
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;
    /// Accept a binding that is reassigned after initialization.
    #[test]
    fn test_accepts_reassigned_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function replace(value: int32): int32 {
    let result = value;
    result = 1;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a binding whose storage is borrowed mutably.
    #[test]
    fn test_accepts_mutably_borrowed_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function replace(): int32 {
    let value: int32 = 0;
    const borrowed = &value;
    *borrowed = 1;
    return value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a binding whose storage is borrowed exclusively.
    #[test]
    fn test_accepts_exclusively_borrowed_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function replace(): int32 {
    let value: int32 = 0;
    const borrowed = &value;
    *borrowed = 1;
    return value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a binding whose storage is only borrowed read-only.
    #[test]
    fn test_replaces_readonly_borrowed_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function read(): int32 {
    let value: int32 = 0;
    const borrowed = &readonly value;
    return *borrowed;
}
"#,
        );

        session.assert_fixes(
            r#"
function read(): int32 {
    const value: int32 = 0;
    const borrowed = &readonly value;
    return *borrowed;
}
"#,
        );
    }

    /// Accept a binding implicitly borrowed with mutable access.
    #[test]
    fn test_accepts_implicitly_borrowed_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
declare function replace(value: &int32): void;
function update(): int32 {
    let value: int32 = 0;
    replace(value);
    return value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a binding implicitly borrowed with read-only access.
    #[test]
    fn test_replaces_implicitly_readonly_borrowed_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
declare function read(value: &readonly int32): int32;
function inspect(): int32 {
    let value: int32 = 0;
    return read(value);
}
"#,
        );

        session.assert_fixes(
            r#"
declare function read(value: &readonly int32): int32;
function inspect(): int32 {
    const value: int32 = 0;
    return read(value);
}
"#,
        );
    }

    /// Report a separately initialized binding without offering an unsafe rewrite.
    #[test]
    fn test_reports_separately_initialized_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function identity(value: int32): int32 {
    let result: int32;
    result = value;
    return result;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-const]: binding is never reassigned
 ──▶ main.ds:2:9
  │
1 │ function identity(value: int32): int32 {
2 │     let result: int32;
  │         ^^^^^^
3 │     result = value;
4 │     return result;
  │
"#,
        );
    }

    /// Accept a separately initialized binding with another write.
    #[test]
    fn test_accepts_separately_reassigned_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function replace(value: int32): int32 {
    let result: int32;
    result = value;
    result = 1;
    return result;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a binding initialized only inside a nested block.
    #[test]
    fn test_accepts_nested_initialization() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function initialize(active: boolean): void {
    let result: int32;
    if (active) {
        result = 1;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a binding while preserving mutation of its managed referent.
    #[test]
    fn test_replaces_binding_with_mutated_value() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
class Counter {
    value: int32 = 0;

    increment(): void {
        this.value += 1;
    }
}
function increment(counter: Counter): Counter {
    let result = counter;
    result.increment();
    return result;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-const]: binding is never reassigned
  ──▶ main.ds:9:5
   │
 7 │ }
 8 │ function increment(counter: Counter): Counter {
 9 │     let result = counter;
   │     ^^^
10 │     result.increment();
11 │     return result;
   │

 = fix: declare the binding with const
--- a/main.ds
+++ b/main.ds

    8│ function increment(counter: Counter): Counter {
-   9│     let result = counter;
+   9│     const result = counter;
   10│     result.increment();
"#,
        );
        session.assert_fixes(
            r#"
class Counter {
    value: int32 = 0;

    increment(): void {
        this.value += 1;
    }
}
function increment(counter: Counter): Counter {
    const result = counter;
    result.increment();
    return result;
}
"#,
        );
    }

    /// Accept direct storage mutated through a field and an exclusive receiver.
    #[test]
    fn test_accepts_mutated_direct_storage() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
struct Counter {
    value: int32;

    increment(&this): void {
        this.value += 1;
    }
}
function increment(): int32 {
    let field = Counter { value: 0 };
    field.value = 1;

    let called = Counter { value: 0 };
    called.increment();
    return field.value + called.value;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Replace a binding whose borrowed referent is mutated.
    #[test]
    fn test_replaces_mutable_reference_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function replace(value: &int32): void {
    let reference = value;
    *reference = 1;
}
"#,
        );

        session.assert_fixes(
            r#"
function replace(value: &int32): void {
    const reference = value;
    *reference = 1;
}
"#,
        );
    }

    /// Replace an initialized destructuring declaration when no binding is reassigned.
    #[test]
    fn test_replaces_destructured_bindings() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function sum(point: { x: int32; y: int32 }): int32 {
    let { x, y } = point;
    return x + y;
}
"#,
        );

        session.assert_fixes(
            r#"
function sum(point: { x: int32; y: int32 }): int32 {
    const { x, y } = point;
    return x + y;
}
"#,
        );
    }

    /// Report only the constant binding in a partially reassigned destructuring declaration.
    #[test]
    fn test_reports_constant_destructured_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function sum(point: { x: int32; y: int32 }): int32 {
    let { x, y } = point;
    x += 1;
    return x + y;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[prefer-const]: binding is never reassigned
 ──▶ main.ds:2:14
  │
1 │ function sum(point: { x: int32; y: int32 }): int32 {
2 │     let { x, y } = point;
  │              ^
3 │     x += 1;
4 │     return x + y;
  │
"#,
        );
    }

    /// Replace a for-of binding that remains immutable during each iteration.
    #[test]
    fn test_replaces_for_of_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function visit(values: int32[]): void {
    for (let value of values) {
        value;
    }
}
"#,
        );

        session.assert_fixes(
            r#"
function visit(values: int32[]): void {
    for (const value of values) {
        value;
    }
}
"#,
        );
    }

    /// Accept a for-of binding whose storage is mutated.
    #[test]
    fn test_accepts_mutated_for_of_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function visit(values: int32[]): void {
    for (let value of values) {
        value += 1;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept destructured iteration when any binding requires mutable storage.
    #[test]
    fn test_accepts_partially_mutated_for_of_binding() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function visit(entries: (int32, int32)[]): void {
    for (let (key, value) of entries) {
        key += 1;
        value;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
