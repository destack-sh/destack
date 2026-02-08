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

/// Compare two strings using natural sort order (numbers ordered as integers).
/// Example: `"a1" < "a2" < "a10"` (not `"a1" < "a10" < "a2"`).
pub(super) fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ac), Some(bc)) => {
                // both are digits: compare as numbers
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let mut a_num: u64 = 0;
                    while let Some(&c) = a_chars.peek()
                        && c.is_ascii_digit()
                    {
                        a_num = a_num
                            .saturating_mul(10)
                            .saturating_add((c as u64) - ('0' as u64));
                        a_chars.next();
                    }
                    let mut b_num: u64 = 0;
                    while let Some(&c) = b_chars.peek()
                        && c.is_ascii_digit()
                    {
                        b_num = b_num
                            .saturating_mul(10)
                            .saturating_add((c as u64) - ('0' as u64));
                        b_chars.next();
                    }
                    match a_num.cmp(&b_num) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }

                // compare characters case-insensitively
                let ac_lower = ac.to_ascii_lowercase();
                let bc_lower = bc.to_ascii_lowercase();
                match ac_lower.cmp(&bc_lower) {
                    Ordering::Equal => {
                        // same letter different case: uppercase comes first
                        match ac.cmp(bc) {
                            Ordering::Equal => {
                                a_chars.next();
                                b_chars.next();
                            }
                            other => return other,
                        }
                    }
                    other => return other,
                }
            }
        }
    }
}

/// Sort dependency items, import and export specifiers, by the configured order.
pub(super) fn sort_dependency_items(
    items: &[LocalNodeId<DependencyItem>],
    tree: &NodeTree,
    strings: &destack_base::ImmutableStringPool,
    sort_order: ImportSortOrder,
) -> Vec<LocalNodeId<DependencyItem>> {
    let mut sorted: Vec<_> = items.to_vec();

    sorted.sort_by(|a, b| {
        let a_item = tree.get(*a);
        let b_item = tree.get(*b);

        // type imports come before value imports
        let a_is_type = a_item.kind == Some(DependencyKind::Type);
        let b_is_type = b_item.kind == Some(DependencyKind::Type);
        match (a_is_type, b_is_type) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }

        // sort key: use alias if present, otherwise name
        let a_key = a_item
            .alias
            .or(a_item.name.map(|name| name.string()))
            .map(|string_id| strings.get(string_id))
            .unwrap_or("");
        let b_key = b_item
            .alias
            .or(b_item.name.map(|name| name.string()))
            .map(|string_id| strings.get(string_id))
            .unwrap_or("");

        match sort_order {
            ImportSortOrder::Natural => natural_cmp(a_key, b_key),
            ImportSortOrder::Alphabetical => a_key.cmp(b_key),
        }
    });

    sorted
}
