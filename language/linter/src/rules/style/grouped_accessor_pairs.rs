use tspp_core::FxIndexMap;
use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require getter and setter pairs to be adjacent.
    pub GROUPED_ACCESSOR_PAIRS {
        id: "grouped-accessor-pairs",
        summary: "Require getter and setter pairs to be adjacent",
        explanation: r#"
A separated getter and setter split one property's readable and writable behavior across a declaration.
Instead, you SHOULD keep each getter adjacent to its corresponding setter.
"#,
        example: {
            reported: r#"
interface Store {
    get value(): string;
    clear(): void;
    set value(next: string);
}
"#,
            accepted: r#"
interface Store {
    get value(): string;
    set value(next: string);
    clear(): void;
}
"#,
        },
        provenance: [Eslint("grouped-accessor-pairs")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report getter and setter pairs separated by another member.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect every structured declaration member list
    for (_, declaration) in view.iter_nodes::<dir::Declaration>() {
        // inspect declaration members
        if let Some(members) = declaration.member_ids() {
            let accessors = members
                .iter()
                .map(|member| module.accessor(member.into_any()));
            report_separated_accessors(module, lint, accessors, &mut output)?;
        }
    }

    // inspect every structural type member list
    module.visit_type_member_lists(|members| {
        let accessors = members
            .iter()
            .map(|member| module.accessor(member.into_any()));

        report_separated_accessors(module, lint, accessors, &mut output)
    })?;

    // inspect object literal property lists
    for (_, expression) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::ObjectExpression { properties } = expression else {
            continue;
        };
        let accessors = properties
            .iter()
            .map(|property| module.accessor(property.into_any()));
        report_separated_accessors(module, lint, accessors, &mut output)?;
    }

    Ok(output)
}

/// Report separated accessors from one member list.
fn report_separated_accessors(
    module: &DirModule<'_>,
    lint: &Lint,
    accessors: impl IntoIterator<Item = Result<Option<crate::Accessor>, ProviderError>>,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let mut seen = FxIndexMap::default();

    // compare each accessor with its counterpart already seen for the property
    for (index, accessor) in accessors.into_iter().enumerate() {
        let Some(accessor) = accessor? else {
            continue;
        };

        // report a counterpart separated by another member
        if seen
            .get(&(accessor.space, accessor.slot, !accessor.is_getter))
            .is_some_and(|previous| *previous + 1 != index)
        {
            let span = module.main_span(accessor.node)?;
            output.report(lint.diagnostic("accessor is separated from its pair", span));
        }

        // retain this accessor for a later counterpart
        seen.insert((accessor.space, accessor.slot, accessor.is_getter), index);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report separated accessors on a class declaration.
    #[test]
    fn test_reports_separated_class_accessors() {
        let session = TestSession::dir(
            &GROUPED_ACCESSOR_PAIRS,
            r#"
declare class Store {
    get value(): string;
    clear(): void;
    set value(next: string);
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[grouped-accessor-pairs]: accessor is separated from its pair
 ──▶ main.tspp:4:9
  │
2 │     get value(): string;
3 │     clear(): void;
4 │     set value(next: string);
  │         ^^^^^
5 │ }
  │
"#,
        );
    }

    /// Accept a lone getter without requiring a setter.
    #[test]
    fn test_accepts_lone_getter() {
        let session = TestSession::dir(
            &GROUPED_ACCESSOR_PAIRS,
            r#"
interface Store {
    get value(): string;
    clear(): void;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an adjacent setter followed by its getter.
    #[test]
    fn test_accepts_setter_before_getter() {
        let session = TestSession::dir(
            &GROUPED_ACCESSOR_PAIRS,
            r#"
interface Store {
    set value(next: string);
    get value(): string;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Keep static and instance accessors in distinct property groups.
    #[test]
    fn test_accepts_static_and_instance_accessors() {
        let session = TestSession::dir(
            &GROUPED_ACCESSOR_PAIRS,
            r#"
declare class Store {
    static get value(): string;
    clear(): void;
    set value(next: string);
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report separated accessors in an object literal.
    #[test]
    fn test_reports_separated_object_accessors() {
        let session = TestSession::dir(
            &GROUPED_ACCESSOR_PAIRS,
            r#"
const store = {
    get value(): string {
        return "";
    },
    clear(): void {},
    set value(next: string): void {},
};
"#,
        );

        session.assert_diagnostics(
            r#"
warning[grouped-accessor-pairs]: accessor is separated from its pair
 ──▶ main.tspp:6:9
  │
4 │     },
5 │     clear(): void {},
6 │     set value(next: string): void {},
  │         ^^^^^
7 │ };
  │
"#,
        );
    }

    /// Accept adjacent accessors in an object literal.
    #[test]
    fn test_accepts_adjacent_object_accessors() {
        let session = TestSession::dir(
            &GROUPED_ACCESSOR_PAIRS,
            r#"
const store = {
    get value(): string {
        return "";
    },
    set value(next: string): void {},
};
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report separated accessors in a structural object type.
    #[test]
    fn test_reports_separated_object_type_accessors() {
        let session = TestSession::dir(
            &GROUPED_ACCESSOR_PAIRS,
            r#"
type Store = {
    get value(): string;

    clear(): void;

    set value(next: string);
};
"#,
        );

        session.assert_diagnostics(
            r#"
warning[grouped-accessor-pairs]: accessor is separated from its pair
 ──▶ main.tspp:6:9
  │
4 │     clear(): void;
5 │
6 │     set value(next: string);
  │         ^^^^^
7 │ };
  │
"#,
        );
    }
}
