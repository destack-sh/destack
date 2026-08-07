use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, FilePatch, PatchSet, Span};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require const for bindings never reassigned after initialization.
    pub PREFER_CONST {
        id: "prefer-const",
        summary: "Require const for bindings never reassigned after initialization",
        explanation: "A binding that is initialized once and never reassigned should be declared with `const`. This documents the binding cell without restricting mutation performed through the stored value's API.",
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
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// One binding introduced by a let declaration.
#[derive(Clone, Copy)]
struct LetBinding {
    /// The bound symbol.
    symbol: dir::GlobalSymbolId,
    /// The binding source node.
    declaration: dir::LocalNodeIdAny,
    /// Whether the declarator supplies an initializer.
    is_initialized: bool,
}

/// Report let bindings that do not require rebinding.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mutable_uses = mutable_binding_uses(module)?;
    let declarator_bindings = declarator_bindings(module);
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect mutable binding declarations
    for expression in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::Let {
            kind: dir::LetKind::Let,
            declarators,
            ..
        } = view.get(expression)
        else {
            continue;
        };
        let mut bindings = Vec::new();
        for declarator in declarators {
            if let Some(declared) = declarator_bindings.get(declarator) {
                bindings.extend_from_slice(declared);
            }
        }
        if bindings.is_empty() {
            return Err(ProviderError::internal(format!(
                "let expression {} in module {:?} declares no binding symbols",
                expression.id, module.id
            )));
        }

        // select bindings with no writes after their initialization
        let candidates = bindings
            .iter()
            .filter(|binding| {
                let uses = mutable_uses
                    .get(&binding.symbol)
                    .map(Vec::as_slice)
                    .unwrap_or_default();

                if binding.is_initialized {
                    uses.is_empty()
                } else {
                    let [use_] = uses else {
                        return false;
                    };

                    view.ancestor::<dir::Block>(expression.into_any())
                        == view.ancestor::<dir::Block>(*use_)
                }
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            continue;
        }

        // replace the declaration keyword only when every binding is initialized and constant
        let can_replace = candidates.len() == bindings.len()
            && bindings.iter().all(|binding| binding.is_initialized);
        if can_replace {
            let span = module.main_span(expression.into_any())?;
            let suggestion = replace_let(span, lint)?;
            let diagnostic = lint
                .diagnostic("binding is never reassigned", span)
                .suggestion(suggestion);
            output.report(diagnostic);
            continue;
        }

        // report candidates that require a declaration restructure
        for binding in candidates {
            let span = module.span(binding.declaration)?;
            output.report(lint.diagnostic("binding is never reassigned", span));
        }
    }

    Ok(output)
}

/// Index declarator bindings by their nearest declarator ancestor.
fn declarator_bindings(
    module: &DirModule<'_>,
) -> FxIndexMap<dir::LocalNodeId<dir::Declarator>, Vec<LetBinding>> {
    let view = module.view();
    let mut bindings = FxIndexMap::default();

    // climb each declaration once to its declarator
    for (declaration, symbol) in module.bindings.declaration_symbols() {
        if declaration.module_id != module.id {
            continue;
        }
        let declaration = declaration.local_id;
        let Some(declarator) = view.ancestor::<dir::Declarator>(declaration) else {
            continue;
        };
        let is_initialized = view.get(declarator).value.is_some();
        bindings
            .entry(declarator)
            .or_insert_with(Vec::new)
            .push(LetBinding {
                symbol: symbol.into_global(module.id),
                declaration,
                is_initialized,
            });
    }

    bindings
}

/// Collect uses that require mutable access to each binding cell.
fn mutable_binding_uses(
    module: &DirModule<'_>,
) -> Result<FxIndexMap<dir::GlobalSymbolId, Vec<dir::LocalNodeIdAny>>, ProviderError> {
    let mut uses: FxIndexMap<dir::GlobalSymbolId, Vec<dir::LocalNodeIdAny>> = FxIndexMap::default();

    // collect direct binding writes
    for (source, resolution) in module.decisions.decision_entries() {
        let dir::Decision::Assignment(assignment) = resolution else {
            continue;
        };
        let dir::WriteResolution::Binding { symbol, .. } = assignment.write else {
            continue;
        };

        uses.entry(symbol).or_default().push(source.local_id);
    }

    // collect explicit mutable and exclusive borrows of root binding storage
    let view = module.view();
    for (expression, value) in view.iter_nodes_of_type::<dir::Expression>() {
        let dir::Expression::BorrowOf { right, .. } = value else {
            continue;
        };
        let target = module.node_type_id(expression.into_any())?;
        let borrow = module.dir.get_borrow(target)?;
        let access = module.dir.get_access(borrow.access)?;
        if access == dir::Access::Readonly {
            continue;
        }

        record_mutable_binding_use(
            module,
            right.into_global_any(module.id),
            expression.into_any(),
            &mut uses,
        );
    }

    // collect implicit mutable and exclusive borrow coercions
    for (source, coercion) in module.coercions.coercions() {
        if !has_mutable_borrow(module, &coercion.adjustments)? {
            continue;
        }

        record_mutable_binding_use(module, source, source.local_id, &mut uses);
    }

    Ok(uses)
}

/// Record one mutable use when its source is root binding storage.
fn record_mutable_binding_use(
    module: &DirModule<'_>,
    source: dir::GlobalNodeIdAny,
    use_: dir::LocalNodeIdAny,
    uses: &mut FxIndexMap<dir::GlobalSymbolId, Vec<dir::LocalNodeIdAny>>,
) {
    let Some(access) = module.decisions.access_resolution(source) else {
        return;
    };
    let dir::AccessRoot::Symbol(symbol) = access.path().root() else {
        return;
    };
    if !access.path().keys().is_empty() {
        return;
    }

    uses.entry(symbol).or_default().push(use_);
}

/// Return whether checked adjustments acquire mutable or exclusive access.
fn has_mutable_borrow(
    module: &DirModule<'_>,
    adjustments: &[dir::CoercionAdjustment],
) -> Result<bool, ProviderError> {
    for adjustment in adjustments {
        match adjustment {
            dir::CoercionAdjustment::Borrow { target } => {
                let borrow = module.dir.get_borrow(*target)?;
                let access = module.dir.get_access(borrow.access)?;
                if access != dir::Access::Readonly {
                    return Ok(true);
                }
            }
            dir::CoercionAdjustment::Union { cases, .. } => {
                for case in cases {
                    if has_mutable_borrow(module, &case.adjustments)? {
                        return Ok(true);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(false)
}

/// Replace one let keyword with const.
fn replace_let(span: Span, lint: &Lint) -> Result<DiagnosticSuggestion, ProviderError> {
    let mut file = FilePatch::new(span.file);
    file.replace(span, "const");
    let patches = PatchSet::single(file);

    lint.fix("declare the binding with const", patches)
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

    /// Ignore mutation behind a binding because const only prevents rebinding.
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

    /// Replace an initialized destructuring declaration when no binding is reassigned.
    #[test]
    fn test_replaces_destructured_bindings() {
        let session = TestSession::dir(
            &PREFER_CONST,
            r#"
function sum(point: { x: int32, y: int32 }): int32 {
    let { x, y } = point;
    return x + y;
}
"#,
        );

        session.assert_fixes(
            r#"
function sum(point: { x: int32, y: int32 }): int32 {
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
function sum(point: { x: int32, y: int32 }): int32 {
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
1 │ function sum(point: { x: int32, y: int32 }): int32 {
2 │     let { x, y } = point;
  │              ^
3 │     x += 1;
4 │     return x + y;
  │
"#,
        );
    }
}
