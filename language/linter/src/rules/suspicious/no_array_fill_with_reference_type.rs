use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow Array.fill values that share mutable references.
    pub NO_ARRAY_FILL_WITH_REFERENCE_TYPE {
        id: "no-array-fill-with-reference-type",
        summary: "Disallow Array.fill values that share mutable references",
        explanation: r#"
`Array.fill` places the same managed reference in every selected element, so later mutation is shared by all of them.
Instead, you SHOULD construct each mutable value independently with `Array.from` or an explicit loop.
"#,
        example: {
            reported: r#"
class Cell {
    value: int32 = 0;
}

function cells(): Cell[] {
    let values = [new Cell(), new Cell(), new Cell()];
    values.fill(new Cell());
    return values;
}
"#,
            accepted: r#"
class Cell {
    value: int32 = 0;
}

function cells(): Cell[] {
    return [new Cell(), new Cell(), new Cell()];
}
"#,
        },
        provenance: [Unicorn("no-array-fill-with-reference-type")],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report Array.fill calls that repeat one mutable reference.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect canonical Array.fill calls
    for expression in module.call_expressions() {
        let expression = expression?;
        let Some(call) = module.member_call(expression) else {
            continue;
        };
        if call.is_optional()
            || module.language_member(expression)? != Some(dir::LanguageItem::Array.member("fill"))
        {
            continue;
        }
        let Some(argument) = call.arguments.first() else {
            continue;
        };
        let dir::Argument::Positional { value } = view.get(*argument) else {
            continue;
        };

        // require aliases that expose the same mutable referent
        let type_id = module.node_type_id(value.into_any())?;
        if !module.dir.is_mutable_reference(type_id)? {
            continue;
        }

        let span = module.source_extent(value.into_any())?;
        let diagnostic = lint
            .diagnostic("Array.fill repeats one managed reference", span)
            .help("construct one value per element");
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report filling an Array with one class instance.
    #[test]
    fn test_reports_class_instance() {
        TestSession::assert_example(&NO_ARRAY_FILL_WITH_REFERENCE_TYPE);
    }

    /// Report filling an Array from an existing managed binding.
    #[test]
    fn test_reports_managed_binding() {
        let session = TestSession::dir(
            &NO_ARRAY_FILL_WITH_REFERENCE_TYPE,
            r#"
class Cell {
    value: int32 = 0;
}

function cells(value: Cell): Cell[] {
    let values = [new Cell(), new Cell(), new Cell()];
    values.fill(value);
    return values;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[no-array-fill-with-reference-type]: Array.fill repeats one managed reference
 ──▶ main.ds:7:17
  │
5 │ function cells(value: Cell): Cell[] {
6 │     let values = [new Cell(), new Cell(), new Cell()];
7 │     values.fill(value);
  │                 ^^^^^
8 │     return values;
9 │ }
  │

 = help: construct one value per element
"#,
        );
    }

    /// Accept filling an Array with a value type.
    #[test]
    fn test_accepts_value_type() {
        let session = TestSession::dir(
            &NO_ARRAY_FILL_WITH_REFERENCE_TYPE,
            r#"
function zeros(): int32[] {
    let values = [1, 2, 3];
    values.fill(0);
    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept filling an Array with immutable string values.
    #[test]
    fn test_accepts_string() {
        let session = TestSession::dir(
            &NO_ARRAY_FILL_WITH_REFERENCE_TYPE,
            r#"
function labels(value: string): string[] {
    let values = ["left", "middle", "right"];
    values.fill(value);
    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept filling an Array with an immutable function value.
    #[test]
    fn test_accepts_function() {
        let session = TestSession::dir(
            &NO_ARRAY_FILL_WITH_REFERENCE_TYPE,
            r#"
function identity(value: int32): int32 {
    return value;
}

function operations(): ((value: int32) => int32)[] {
    let values = [identity, identity, identity];
    values.fill(identity);
    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept filling an Array with a readonly managed value.
    #[test]
    fn test_accepts_readonly_reference() {
        let session = TestSession::dir(
            &NO_ARRAY_FILL_WITH_REFERENCE_TYPE,
            r#"
class Cell {
    value: int32 = 0;
}

function cells(value: readonly Cell): (readonly Cell)[] {
    let values = [value, value, value];
    values.fill(value);
    return values;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
