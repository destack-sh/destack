use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require nominal literal fields in declaration order.
    pub INCONSISTENT_FIELD_ORDER {
        id: "inconsistent-field-order",
        summary: "Require nominal literal fields in declaration order",
        explanation: r#"
Nominal literals written in a different order from their declaration are harder to compare with their type.
Instead, you SHOULD initialize supplied fields in declaration order.

Spreads begin a new ordering region because moving an initializer across a spread can change behavior.
"#,
        example: {
            reported: r#"
struct Point {
    x: int32;
    y: int32;
}

const point = Point { y: 2, x: 1 };
"#,
            accepted: r#"
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2 };
"#,
        },
        provenance: [Clippy("inconsistent_struct_constructor")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report nominal literal fields that occur before an earlier declaration field.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect every nominal literal independently
    for (node, expression) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::StructExpression { ty, properties } = expression else {
            continue;
        };
        let type_id = module.node_type_id(ty.into_any())?;
        let type_id = module.dir.strip_form(type_id)?;
        let ty = module.dir.get_type(type_id)?;
        let symbol = ty.symbol().ok_or_else(|| {
            ProviderError::internal(format!(
                "nominal literal {node:?} has non-nominal type {type_id:?}"
            ))
        })?;

        // index the declaration's instance fields once
        let positions = module
            .dir
            .instance_field_keys(symbol)?
            .into_iter()
            .enumerate()
            .map(|(position, key)| (key, position))
            .collect::<FxIndexMap<_, _>>();
        report_out_of_order_fields(module, lint, properties, &positions, &mut output)?;
    }

    Ok(output)
}

/// Report property fields whose declaration positions move backwards.
fn report_out_of_order_fields(
    module: &DirModule<'_>,
    lint: &Lint,
    properties: &[dir::LocalNodeId<dir::Property>],
    positions: &FxIndexMap<dir::StaticKey, usize>,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let view = module.view();
    let mut furthest: Option<usize> = None;

    // compare each supplied field within its spread-delimited region
    for property in properties {
        let name = match view.get(*property) {
            dir::Property::Field { name, .. }
            | dir::Property::Method {
                name: Some(name), ..
            } => *name,
            dir::Property::Spread { .. } => {
                furthest = None;

                continue;
            }
            dir::Property::Method { name: None, .. } | dir::Property::Error => continue,
        };
        let key = dir::StaticKey::from(name);
        let position = positions.get(&key).copied().ok_or_else(|| {
            ProviderError::internal(format!(
                "nominal literal field {key:?} is absent from its declaration"
            ))
        })?;
        let is_out_of_order = furthest.is_some_and(|furthest| position < furthest);
        furthest = Some(furthest.map_or(position, |furthest| furthest.max(position)));
        if !is_out_of_order {
            continue;
        }

        // identify the field that moves backwards
        let span = module.main_span(property.into_any())?;
        let name = key.debug_string(&module.dir.strings);
        let message = format!("field {name} is out of declaration order");
        output.report(lint.diagnostic(message, span));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report every field behind the furthest preceding field.
    #[test]
    fn test_reports_reordered_fields() {
        let session = TestSession::dir(
            &INCONSISTENT_FIELD_ORDER,
            r#"
struct Position {
    x: int32;
    y: int32;
    z: int32;
}

const position = Position { z: 3, x: 1, y: 2 };
"#,
        );

        session.assert_diagnostics(
            r#"
warning[inconsistent-field-order]: field 'x' is out of declaration order
 ──▶ main.ds:7:35
  │
5 │ }
6 │
7 │ const position = Position { z: 3, x: 1, y: 2 };
  │                                   ^
  │

warning[inconsistent-field-order]: field 'y' is out of declaration order
 ──▶ main.ds:7:41
  │
5 │ }
6 │
7 │ const position = Position { z: 3, x: 1, y: 2 };
  │                                         ^
  │
"#,
        );
    }

    /// Keep structural object field order unconstrained.
    #[test]
    fn test_accepts_structural_object_order() {
        let session = TestSession::dir(
            &INCONSISTENT_FIELD_ORDER,
            r#"
const point = { y: 2, x: 1 };
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Compare nominal fields independently across spread-delimited regions.
    #[test]
    fn test_accepts_reordered_fields_across_spread() {
        let session = TestSession::dir(
            &INCONSISTENT_FIELD_ORDER,
            r#"
struct Position {
    x: int32;
    y: int32;
}

const base = Position { x: 1, y: 2 };
const position = Position { y: 3, ...base, x: 4 };
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Order method initializers with the fields they satisfy.
    #[test]
    fn test_reports_reordered_method_initializer() {
        let session = TestSession::dir(
            &INCONSISTENT_FIELD_ORDER,
            r#"
struct Task {
    run: () => void;
    name: string;
}

const task = Task {
    name: "compile",
    run(): void {},
};
"#,
        );

        session.assert_diagnostics(
            r#"
warning[inconsistent-field-order]: field 'run' is out of declaration order
 ──▶ main.ds:8:5
  │
6 │ const task = Task {
7 │     name: "compile",
8 │     run(): void {},
  │     ^^^
9 │ };
  │
"#,
        );
    }
}
