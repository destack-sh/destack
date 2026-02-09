use super::*;
use destack_fir::write;

/// Format `export import ... = require(...)` when modeled as an export let.
pub(super) fn format_export_import_equals(
    f: &mut DestackFormatter<'_, '_>,
    tree: &NodeTree,
    descriptor: &DeclarationDescriptor,
    declarators: &[LocalNodeId<Declarator>],
) -> FormatResult<bool> {
    // descriptor.export is only set for export forms
    let Some(export) = descriptor.export else {
        return Ok(false);
    };

    // expect single declarator: const Alias = importEquals
    if declarators.len() != 1 {
        return Ok(false);
    }

    let Declarator {
        pattern,
        ty: None,
        value: Some(value),
    } = tree.get(declarators[0])
    else {
        return Ok(false);
    };

    if !matches!(tree.get(*pattern), Pattern::Binding { .. }) {
        return Ok(false);
    }

    let Expression::Import {
        source,
        kind,
        target,
        items,
        ..
    } = tree.get(*value)
    else {
        return Ok(false);
    };

    if *source != ImportSource::ImportEquals {
        return Ok(false);
    }

    let alias =
        items
            .first()
            .and_then(|item| tree.get(*item).alias)
            .ok_or(FormatError::SyntaxError {
                message: "import equals requires an alias",
            })?;

    write!(f, [export, space(), Keyword::Import, space()])?;
    if *kind == DependencyKind::Type {
        write!(f, [Keyword::Type, space()])?;
    }
    write!(
        f,
        [
            alias,
            space(),
            token("="),
            space(),
            token("require"),
            token("("),
            token("\""),
            target,
            token("\""),
            token(")")
        ]
    )?;
    Ok(true)
}
