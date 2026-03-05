use std::cmp::Ordering;

use destack_source::Span;

/// Import group category for declaration ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportGroup {
    /// Builtin modules with protocol prefix.
    Builtin = 0,
    /// External packages.
    Package = 1,
    /// Path aliases.
    Alias = 2,
    /// Relative imports.
    Relative = 3,
}

impl ImportGroup {
    /// Categorize an import target path into one import group.
    pub fn from_path(path: &str) -> Self {
        // treat protocol imports as builtins but keep URLs as packages
        if let Some(colon_pos) = path.find(':')
            && !path[colon_pos..].starts_with("://")
        {
            return ImportGroup::Builtin;
        }

        // keep relative and absolute file paths together
        if path.starts_with("./") || path.starts_with("../") || path.starts_with('/') {
            return ImportGroup::Relative;
        }

        // keep known aliases grouped separately from packages
        if is_alias_specifier(path) {
            return ImportGroup::Alias;
        }

        ImportGroup::Package
    }
}

/// One declaration key used to build canonical import order.
#[derive(Debug, Clone, Copy)]
pub struct ImportDeclarationKey<'a> {
    /// The import target string.
    pub target: &'a str,
    /// Whether this declaration is side-effect-only.
    pub is_side_effect: bool,
}

/// Categorize one import target path.
pub fn categorize_import(target: &str) -> ImportGroup {
    ImportGroup::from_path(target)
}

/// Compare two import targets by canonical declaration order.
pub fn compare_import_targets(left: &str, right: &str) -> Ordering {
    match categorize_import(left).cmp(&categorize_import(right)) {
        Ordering::Equal => left.cmp(right),
        ordering => ordering,
    }
}

/// Return declaration indices ordered by canonical import order.
///
/// Side-effect imports preserve source order and stay above regular imports.
pub fn sort_import_declaration_indices(keys: &[ImportDeclarationKey<'_>]) -> Vec<usize> {
    let mut side_effect_indices = Vec::new();
    let mut regular_indices = Vec::new();

    // split side-effect and regular imports
    for (index, key) in keys.iter().enumerate() {
        if key.is_side_effect {
            side_effect_indices.push(index);
        } else {
            regular_indices.push(index);
        }
    }

    // sort regular imports by canonical target order
    regular_indices.sort_by(|left, right| {
        let left_target = keys[*left].target;
        let right_target = keys[*right].target;
        compare_import_targets(left_target, right_target)
    });

    // side-effect imports first, then sorted regular imports
    let mut result = Vec::with_capacity(keys.len());
    result.extend(side_effect_indices);
    result.extend(regular_indices);
    result
}

/// Return true when one import path uses a known alias prefix.
pub fn is_alias_specifier(specifier: &str) -> bool {
    specifier.starts_with("@/") || specifier.starts_with("~/") || specifier.starts_with('#')
}

/// Resolve a source span that safely removes one import item.
pub fn import_item_removal_span(
    source: &str,
    import_expression_span: Span,
    item_spans: &[Span],
    item_index: usize,
) -> Option<Span> {
    // require one valid item index
    if item_index >= item_spans.len() {
        return None;
    }

    // single binding import: remove the whole import expression statement
    if item_spans.len() == 1 {
        return Some(import_expression_span);
    }

    // resolve span for the unused binding
    let item_span = item_spans[item_index];

    // first binding: remove this item and the separator up to the next item
    if item_index == 0 {
        let next_span = item_spans[1];
        if item_span.end > next_span.start {
            return None;
        }

        // validate comma separation between the first and next binding
        let between = &source[item_span.end as usize..next_span.start as usize];
        if !between.contains(',') || between.contains('{') || between.contains('}') {
            return None;
        }

        return Some(Span::new(item_span.file, item_span.start, next_span.start));
    }

    // non first binding: remove from previous end through this binding
    let previous_span = item_spans[item_index - 1];
    if previous_span.end > item_span.start {
        return None;
    }
    let between = &source[previous_span.end as usize..item_span.start as usize];
    if !between.contains(',') || between.contains('{') || between.contains('}') {
        return None;
    }

    Some(Span::new(item_span.file, previous_span.end, item_span.end))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keep side-effect imports stable and ahead of sorted regular imports.
    #[test]
    fn test_sort_import_declaration_indices_preserves_side_effect_order() {
        let keys = vec![
            ImportDeclarationKey {
                target: "./side_b",
                is_side_effect: true,
            },
            ImportDeclarationKey {
                target: "zod",
                is_side_effect: false,
            },
            ImportDeclarationKey {
                target: "./side_a",
                is_side_effect: true,
            },
            ImportDeclarationKey {
                target: "axios",
                is_side_effect: false,
            },
        ];

        let order = sort_import_declaration_indices(&keys);
        assert_eq!(order, vec![0, 2, 3, 1]);
    }

    /// Sort regular imports by canonical group then lexical target.
    #[test]
    fn test_sort_import_declaration_indices_groups_then_targets() {
        let keys = vec![
            ImportDeclarationKey {
                target: "./local",
                is_side_effect: false,
            },
            ImportDeclarationKey {
                target: "@/internal",
                is_side_effect: false,
            },
            ImportDeclarationKey {
                target: "react",
                is_side_effect: false,
            },
            ImportDeclarationKey {
                target: "node:fs",
                is_side_effect: false,
            },
        ];

        let order = sort_import_declaration_indices(&keys);
        assert_eq!(order, vec![3, 2, 1, 0]);
    }
}
