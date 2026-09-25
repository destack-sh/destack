use tspp_core::FxIndexSet;
use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Prefer conversions over field-by-field reconstruction.
    pub MANUAL_STRUCT_CONVERSION {
        id: "manual-struct-conversion",
        summary: "Prefer conversions over field-by-field reconstruction",
        explanation: r#"
Reconstructing every field of one nominal value from an equally shaped value repeats a conversion wherever it is needed.
Instead, you SHOULD implement `From` for the destination type and call its `from` method.
"#,
        example: {
            reported: r#"
struct Input {
    x: int32;
    y: int32;
}
struct Point {
    x: int32;
    y: int32;
}

function convert(input: Input): Point {
    return Point { x: input.x, y: input.y };
}
"#,
            accepted: r#"
struct Input {
    x: int32;
    y: int32;
}
struct Point {
    x: int32;
    y: int32;
}

extension of Point implements From<Input> {
    static from(input: Input): Point {
        return Point { x: input.x, y: input.y };
    }
}

function convert(input: Input): Point {
    return Point.from(input);
}
"#,
        },
        provenance: [],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report complete nominal reconstruction from another nominal value.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect nominal literals outside canonical conversion implementations
    for (expression, node) in view.iter_nodes::<dir::Expression>() {
        let dir::Expression::StructExpression { ty, properties } = node else {
            continue;
        };
        if module.is_within_language_member(
            expression.into_any(),
            dir::LanguageItem::From.member("from"),
        )? {
            continue;
        }
        let Some(source) = conversion_source(module, *ty, properties)? else {
            continue;
        };

        // direct repeated construction toward one reusable conversion
        let span = module.source_extent(expression.into_any())?;
        let target = module.source(module.source_extent(ty.into_any())?)?;
        let source = module.expression_source(source, dir::OperatorPrecedence::Assignment)?;
        let help = format!("implement `From` for `{target}` and call `{target}.from({source})`");
        let diagnostic = lint
            .diagnostic("nominal value is reconstructed field by field", span)
            .help(help);
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the common source of one complete field-preserving conversion.
fn conversion_source(
    module: &DirModule<'_>,
    target: dir::LocalNodeId<dir::TypeExpression>,
    properties: &[dir::LocalNodeId<dir::Property>],
) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
    let target_type = module
        .dir
        .strip_form(module.node_type_id(target.into_any())?)?;
    let target_symbol = module.dir.get_type(target_type)?.symbol().ok_or_else(|| {
        ProviderError::internal(format!(
            "nominal literal target {target:?} has non-nominal type {target_type:?}"
        ))
    })?;
    let fields = module.dir.instance_field_keys(target_symbol)?;
    if fields.is_empty() || fields.len() != properties.len() {
        return Ok(None);
    }

    let view = module.view();
    let mut initialized = FxIndexSet::default();
    let mut source = None;

    // require every target field to copy the equally named source field
    for property in properties {
        let dir::Property::Field { name, value, .. } = view.get(*property) else {
            return Ok(None);
        };
        let dir::Expression::Member {
            left,
            name: Some(member),
            is_optional: false,
        } = view.get(*value)
        else {
            return Ok(None);
        };
        let key = dir::StaticKey::from(*name);
        if key != dir::StaticKey::Name(*member) || !initialized.insert(key) {
            return Ok(None);
        }
        if let Some(source) = source {
            if !module.is_same_computation(source, *left)? {
                return Ok(None);
            }
        } else {
            source = Some(*left);
        }
    }
    if initialized.len() != fields.len() || fields.iter().any(|field| !initialized.contains(field))
    {
        return Ok(None);
    }

    // require a distinct nominal source type
    let Some(source) = source else {
        return Ok(None);
    };
    let source_type = module
        .dir
        .strip_form(module.node_type_id(source.into_any())?)?;
    let Some(source_symbol) = module.dir.get_type(source_type)?.symbol() else {
        return Ok(None);
    };
    if source_symbol == target_symbol {
        return Ok(None);
    }

    Ok(Some(source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a complete field-preserving nominal conversion.
    #[test]
    fn test_reports_field_conversion() {
        let session = TestSession::dir(
            &MANUAL_STRUCT_CONVERSION,
            r#"
struct Input {
    x: int32;
    y: int32;
}
struct Point {
    x: int32;
    y: int32;
}

function convert(input: Input): Point {
    return Point { x: input.x, y: input.y };
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[manual-struct-conversion]: nominal value is reconstructed field by field
  ──▶ main.tspp:11:12
   │
 9 │
10 │ function convert(input: Input): Point {
11 │     return Point { x: input.x, y: input.y };
   │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
12 │ }
   │

 = help: implement `From` for `Point` and call `Point.from(input)`
"#,
        );
    }

    /// Accept reconstruction inside the canonical From implementation.
    #[test]
    fn test_accepts_from_implementation() {
        let session = TestSession::dir(
            &MANUAL_STRUCT_CONVERSION,
            r#"
struct Input {
    x: int32;
    y: int32;
}
struct Point {
    x: int32;
    y: int32;
}

extension of Point implements From<Input> {
    static from(input: Input): Point {
        return Point { x: input.x, y: input.y };
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept partial and transformed construction.
    #[test]
    fn test_accepts_distinct_construction() {
        let session = TestSession::dir(
            &MANUAL_STRUCT_CONVERSION,
            r#"
struct Input {
    x: int32;
    y: int32;
}
struct Point {
    x: int32;
    y?: int32;
}

function convert(input: Input): Point {
    const partial = Point { x: input.x };
    return Point { x: input.x + 1, y: input.y };
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
