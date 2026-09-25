use tspp_core::FxIndexSet;
use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Accessor, DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow setters without a matching getter.
    pub REQUIRE_ACCESSOR_PAIR {
        id: "require-accessor-pair",
        summary: "Disallow setters without a matching getter",
        explanation: r#"
A write-only property prevents callers from observing the value represented by its setter.
Instead, you SHOULD provide a getter in the same property namespace.

Getter-only properties remain valid readonly properties.
"#,
        example: {
            reported: r#"
interface Store {
    set value(next: string);
}
"#,
            accepted: r#"
interface Store {
    get value(): string;
    set value(next: string);
}
"#,
        },
        provenance: [Eslint("accessor-pairs")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report setters without getters in the same member list.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect declaration and structural member lists
    for (_, declaration) in view.iter_nodes::<dir::Declaration>() {
        if let Some(members) = declaration.member_ids() {
            let accessors = members
                .iter()
                .map(|member| module.accessor(member.into_any()));
            report_missing_getters(module, lint, accessors, &mut output)?;
        }
    }

    // inspect every structural type member list
    module.visit_type_member_lists(|members| {
        let accessors = members
            .iter()
            .map(|member| module.accessor(member.into_any()));

        report_missing_getters(module, lint, accessors, &mut output)
    })?;

    // inspect object literal property lists
    for (_, expression) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::ObjectExpression { properties } = expression else {
            continue;
        };
        let accessors = properties
            .iter()
            .map(|property| module.accessor(property.into_any()));
        report_missing_getters(module, lint, accessors, &mut output)?;
    }

    Ok(output)
}

/// Report setters absent from the getter keys in one member list.
fn report_missing_getters(
    module: &DirModule<'_>,
    lint: &Lint,
    accessors: impl IntoIterator<Item = Result<Option<Accessor>, ProviderError>>,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let accessors = accessors
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let getters = accessors
        .iter()
        .filter(|accessor| accessor.is_getter)
        .map(|accessor| (accessor.space, accessor.slot))
        .collect::<FxIndexSet<_>>();

    // report each setter without a readable property in the same namespace
    for accessor in accessors {
        let key = (accessor.space, accessor.slot);
        if accessor.is_getter || getters.contains(&key) {
            continue;
        }

        let span = module.main_span(accessor.node)?;
        output.report(lint.diagnostic("setter has no matching getter", span));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Keep static and instance properties in distinct namespaces.
    #[test]
    fn test_reports_instance_setter_with_static_getter() {
        let session = TestSession::dir(
            &REQUIRE_ACCESSOR_PAIR,
            r#"
declare class Store {
    static get value(): string;
    set value(next: string);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[require-accessor-pair]: setter has no matching getter
 ──▶ main.tspp:3:9
  │
1 │ declare class Store {
2 │     static get value(): string;
3 │     set value(next: string);
  │         ^^^^^
4 │ }
  │
"#,
        );
    }

    /// Accept a getter-only readonly property.
    #[test]
    fn test_accepts_getter_only_property() {
        let session = TestSession::dir(
            &REQUIRE_ACCESSOR_PAIR,
            r#"
interface Store {
    get value(): string;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a write-only property in an object literal.
    #[test]
    fn test_reports_object_setter_without_getter() {
        let session = TestSession::dir(
            &REQUIRE_ACCESSOR_PAIR,
            r#"
const store = {
    set value(next: string): void {},
};
"#,
        );

        session.assert_diagnostics(
            r#"
warning[require-accessor-pair]: setter has no matching getter
 ──▶ main.tspp:2:9
  │
1 │ const store = {
2 │     set value(next: string): void {},
  │         ^^^^^
3 │ };
  │
"#,
        );
    }

    /// Report a write-only property in a structural object type.
    #[test]
    fn test_reports_object_type_setter_without_getter() {
        let session = TestSession::dir(
            &REQUIRE_ACCESSOR_PAIR,
            r#"
type Store = {
    set value(next: string);
};
"#,
        );

        session.assert_diagnostics(
            r#"
warning[require-accessor-pair]: setter has no matching getter
 ──▶ main.tspp:2:9
  │
1 │ type Store = {
2 │     set value(next: string);
  │         ^^^^^
3 │ };
  │
"#,
        );
    }
}
