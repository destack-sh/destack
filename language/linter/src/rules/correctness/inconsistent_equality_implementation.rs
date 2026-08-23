use destack_dir as dir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow inconsistent derived and written equality or hashing implementations.
    pub INCONSISTENT_EQUALITY_IMPLEMENTATION {
        id: "inconsistent-equality-implementation",
        summary: "Disallow mixing derived and written equality or hashing implementations",
        explanation: r#"
Writing equality while deriving hashing, or writing hashing while deriving equality, can assign different hashes to equal values.
Instead, you MUST derive both protocols or implement both protocols from the same fields.
"#,
        example: {
            reported: r#"
@derive(Hash)
struct Key {
    value: int32;
}

extension of Key implements Equal<Key> {
    equal(&readonly this, other: &readonly Key): boolean {
        return this.value % 10 == other.value % 10;
    }
}
"#,
            accepted: r#"
@derive(Equal, Hash)
struct Key {
    value: int32;
}
"#,
        },
        category: Correctness,
        level: Error,
        fixable: None,
        check: DirModule(check),
    }
}

/// Equality or hashing behavior whose implementations must share one origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImplementationKind {
    /// Total equality.
    Equality,
    /// Stable hashing.
    Hash,
}

impl ImplementationKind {
    /// Return the protocol name used in diagnostics.
    fn name(self) -> &'static str {
        match self {
            Self::Equality => "equality",
            Self::Hash => "hashing",
        }
    }
}

/// One authored equality or hashing conformance.
#[derive(Debug, Clone, Copy)]
struct WrittenImplementation {
    /// The nominal type receiving the implementation.
    target: dir::GlobalSymbolId,
    /// The implemented behavior.
    kind: ImplementationKind,
    /// The authored interface type.
    source: dir::GlobalNodeIdAny,
}

/// Report targets that mix derived hashing with written equality or the reverse.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut written = Vec::new();

    // collect authored equality and hashing conformances
    for (symbol, definition) in module.definitions.iter_definitions() {
        let Some(target) = implementation_target(symbol, definition) else {
            continue;
        };

        // retain written equality and hashing conformances
        for implementation in definition.implementations() {
            if view
                .ancestor::<dir::Decorator>(implementation.source.local_id)
                .is_some()
            {
                continue;
            }
            let Some(kind) = conformance_kind(implementation, module)? else {
                continue;
            };
            written.push(WrittenImplementation {
                target,
                kind,
                source: implementation.source,
            });
        }
    }

    // report behaviors whose counterpart remains derived
    let mut output = LintOutput::default();
    for implementation in &written {
        if written
            .iter()
            .any(|other| other.target == implementation.target && other.kind != implementation.kind)
        {
            continue;
        }
        let Some(derived) = conflicting_derive(implementation.target, implementation.kind, module)?
        else {
            continue;
        };

        let span = module.main_span(implementation.source.local_id)?;
        let message = format!(
            "written {} conflicts with derived {}",
            implementation.kind.name(),
            derived.name(),
        );
        output.report(lint.diagnostic(message, span));
    }

    Ok(output)
}

/// Return the equality or hashing behavior selected by one written conformance.
fn conformance_kind(
    conformance: &dir::NominalConformance,
    module: &DirModule<'_>,
) -> Result<Option<ImplementationKind>, ProviderError> {
    let item = module.dir.representation_item(conformance.interface)?;
    let kind = match item {
        Some(dir::LanguageItem::Equal | dir::LanguageItem::Compare) => {
            Some(ImplementationKind::Equality)
        }
        Some(dir::LanguageItem::Hash) => Some(ImplementationKind::Hash),
        _ => None,
    };

    Ok(kind)
}

/// Return the derived behavior that conflicts with one written implementation.
fn conflicting_derive(
    target: dir::GlobalSymbolId,
    written: ImplementationKind,
    module: &DirModule<'_>,
) -> Result<Option<ImplementationKind>, ProviderError> {
    let derives = module.dir.written_derives(target)?;

    // absence of an explicit selection enables every automatic conformance
    let has_derived = |interface| {
        derives
            .as_ref()
            .is_none_or(|derives| derives.contains(&interface))
    };

    // select the generated counterpart of the written implementation
    let conflicting = match written {
        ImplementationKind::Equality if has_derived(dir::AutoInterface::Hash) => {
            Some(ImplementationKind::Hash)
        }
        ImplementationKind::Hash
            if has_derived(dir::AutoInterface::Equal)
                || has_derived(dir::AutoInterface::Compare) =>
        {
            Some(ImplementationKind::Equality)
        }
        ImplementationKind::Equality | ImplementationKind::Hash => None,
    };

    Ok(conflicting)
}

/// Return the nominal declaration that receives one definition's implementations.
fn implementation_target(
    symbol: dir::GlobalSymbolId,
    definition: &dir::Definition,
) -> Option<dir::GlobalSymbolId> {
    match definition {
        dir::Definition::Extension(extension) => extension.target.declaration(),
        _ => Some(symbol),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report written equality paired with derived hashing.
    #[test]
    fn test_reports_written_equality() {
        let session = TestSession::dir(
            &INCONSISTENT_EQUALITY_IMPLEMENTATION,
            r#"
@derive(Hash)
struct Key {
    value: int32;
}

extension of Key implements Equal<Key> {
    equal(&readonly this, other: &readonly Key): boolean {
        return this.value % 10 == other.value % 10;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[inconsistent-equality-implementation]: written equality conflicts with derived hashing
 ──▶ main.ds:6:29
  │
4 │ }
5 │
6 │ extension of Key implements Equal<Key> {
  │                             ^^^^^
7 │     equal(&readonly this, other: &readonly Key): boolean {
8 │         return this.value % 10 == other.value % 10;
  │
"#,
        );
    }

    /// Report written hashing paired with derived equality.
    #[test]
    fn test_reports_written_hashing() {
        let session = TestSession::dir(
            &INCONSISTENT_EQUALITY_IMPLEMENTATION,
            r#"
@derive(Equal)
struct Key {
    value: int32;
}

extension of Key implements Hash {
    hash(&readonly this, state: &Hasher): void {
        todo("custom hash")
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[inconsistent-equality-implementation]: written hashing conflicts with derived equality
 ──▶ main.ds:6:29
  │
4 │ }
5 │
6 │ extension of Key implements Hash {
  │                             ^^^^
7 │     hash(&readonly this, state: &Hasher): void {
8 │         todo("custom hash")
  │
"#,
        );
    }

    /// Accept equality and hashing derived together.
    #[test]
    fn test_accepts_derived_pair() {
        let session = TestSession::dir(
            &INCONSISTENT_EQUALITY_IMPLEMENTATION,
            r#"
@derive(Equal, Hash)
struct Key {
    value: int32;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept equality and hashing written together.
    #[test]
    fn test_accepts_written_pair() {
        let session = TestSession::dir(
            &INCONSISTENT_EQUALITY_IMPLEMENTATION,
            r#"
struct Key {
    value: int32;
}

extension of Key implements Equal<Key>, Hash {
    equal(&readonly this, other: &readonly Key): boolean {
        return this.value % 10 == other.value % 10;
    }

    hash(&readonly this, state: &Hasher): void {
        todo("custom hash")
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
