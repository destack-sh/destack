use std::cmp::Ordering;

use destack_ast::{DependencyItem, DependencySpace, LocalNodeId, Tree};
use destack_core::{ImmutableStringPool, StringId};
use destack_workspace::ImportSortOrder;

/// The import group category for declaration ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportGroup {
    /// Builtin modules with protocol prefixes.
    Builtin = 0,
    /// External packages.
    Package = 1,
    /// Path aliases.
    Alias = 2,
    /// Relative imports.
    Relative = 3,
}

impl ImportGroup {
    /// Categorize one import target path into a group.
    pub fn from_path(path: &str) -> Self {
        // builtin protocols: "protocol:module" but not urls
        if let Some(colon_position) = path.find(':')
            && !path[colon_position..].starts_with("://")
        {
            return Self::Builtin;
        }

        // relative imports
        if path.starts_with("./") || path.starts_with("../") || path.starts_with('/') {
            return Self::Relative;
        }

        // alias imports: @/ ~/ # but not scoped packages like @org/pkg
        if is_alias_specifier(path) {
            return Self::Alias;
        }

        Self::Package
    }
}

/// One import declaration key for ordering.
#[derive(Debug, Clone, Copy)]
pub struct ImportDeclarationKey<'a> {
    /// The import target string.
    pub target: &'a str,
    /// Whether this declaration is a side effect only import.
    pub is_side_effect: bool,
}

/// Categorize one import target path into a group.
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
/// Side effect imports preserve source order and stay above regular imports.
pub fn sort_import_declaration_indices(keys: &[ImportDeclarationKey<'_>]) -> Vec<usize> {
    let mut side_effect_indices = Vec::new();
    let mut regular_indices = Vec::new();

    // split side effect and regular imports
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

    // side effects first, then sorted regular imports
    let mut result = Vec::with_capacity(keys.len());
    result.extend(side_effect_indices);
    result.extend(regular_indices);
    result
}

/// Return dependency items sorted by space and configured key order.
pub fn sort_dependency_items(
    items: &[LocalNodeId<DependencyItem>],
    tree: &Tree,
    strings: &ImmutableStringPool,
    sort_order: ImportSortOrder,
) -> Vec<LocalNodeId<DependencyItem>> {
    let mut sorted_items: Vec<_> = items.to_vec();
    sorted_items.sort_by(|left_id, right_id| {
        let left_item = tree.get(*left_id);
        let right_item = tree.get(*right_id);

        // type imports come before value imports
        let left_is_type = dependency_item_space(left_item) == Some(DependencySpace::Type);
        let right_is_type = dependency_item_space(right_item) == Some(DependencySpace::Type);
        match (left_is_type, right_is_type) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }

        // alias key first when present, then fallback to item name
        let left_key = dependency_item_key(left_item)
            .map(|string_id| strings.get(string_id))
            .unwrap_or("");
        let right_key = dependency_item_key(right_item)
            .map(|string_id| strings.get(string_id))
            .unwrap_or("");

        match sort_order {
            ImportSortOrder::Natural => natural_cmp(left_key, right_key),
            ImportSortOrder::Alphabetical => left_key.cmp(right_key),
        }
    });

    sorted_items
}

/// Return true when one import path uses a known alias prefix.
fn is_alias_specifier(specifier: &str) -> bool {
    specifier.starts_with("@/") || specifier.starts_with("~/") || specifier.starts_with('#')
}

/// Return one dependency item's space when the item is valid.
fn dependency_item_space(item: &DependencyItem) -> Option<DependencySpace> {
    match item {
        DependencyItem::Item { space, .. } => *space,
        DependencyItem::Error => None,
    }
}

/// Return one dependency item's string key when present.
fn dependency_item_key(item: &DependencyItem) -> Option<StringId> {
    match item {
        DependencyItem::Item { alias, name, .. } => alias.or(name.map(|name| name.string())),
        DependencyItem::Error => None,
    }
}

/// Compare two strings using natural sort order.
fn natural_cmp(left: &str, right: &str) -> Ordering {
    let mut left_chars = left.chars().peekable();
    let mut right_chars = right.chars().peekable();

    loop {
        match (left_chars.peek(), right_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left_char), Some(right_char)) => {
                // compare numeric runs as integers
                if left_char.is_ascii_digit() && right_char.is_ascii_digit() {
                    let mut left_number: u64 = 0;
                    while let Some(&character) = left_chars.peek()
                        && character.is_ascii_digit()
                    {
                        left_number = left_number
                            .saturating_mul(10)
                            .saturating_add((character as u64) - ('0' as u64));
                        left_chars.next();
                    }

                    let mut right_number: u64 = 0;
                    while let Some(&character) = right_chars.peek()
                        && character.is_ascii_digit()
                    {
                        right_number = right_number
                            .saturating_mul(10)
                            .saturating_add((character as u64) - ('0' as u64));
                        right_chars.next();
                    }

                    match left_number.cmp(&right_number) {
                        Ordering::Equal => continue,
                        ordering => return ordering,
                    }
                }

                // compare case insensitively first
                let left_lower = left_char.to_ascii_lowercase();
                let right_lower = right_char.to_ascii_lowercase();
                match left_lower.cmp(&right_lower) {
                    Ordering::Equal => match left_char.cmp(right_char) {
                        Ordering::Equal => {
                            left_chars.next();
                            right_chars.next();
                        }
                        ordering => return ordering,
                    },
                    ordering => return ordering,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Order import groups by builtin, package, alias, then relative.
    #[test]
    fn test_import_group_ordering() {
        // compare group ordering
        assert!(ImportGroup::Builtin < ImportGroup::Package);
        assert!(ImportGroup::Package < ImportGroup::Alias);
        assert!(ImportGroup::Alias < ImportGroup::Relative);
    }

    /// Categorize import targets by builtin, package, alias, and relative groups.
    #[test]
    fn test_categorize_import() {
        // compare import categories
        assert_eq!(ImportGroup::from_path("node:fs"), ImportGroup::Builtin);
        assert_eq!(ImportGroup::from_path("bun:test"), ImportGroup::Builtin);
        assert_eq!(ImportGroup::from_path("react"), ImportGroup::Package);
        assert_eq!(ImportGroup::from_path("@org/pkg"), ImportGroup::Package);
        assert_eq!(ImportGroup::from_path("lodash"), ImportGroup::Package);
        assert_eq!(ImportGroup::from_path("@/utils"), ImportGroup::Alias);
        assert_eq!(ImportGroup::from_path("~/lib"), ImportGroup::Alias);
        assert_eq!(ImportGroup::from_path("#internal"), ImportGroup::Alias);
        assert_eq!(ImportGroup::from_path("./local"), ImportGroup::Relative);
        assert_eq!(ImportGroup::from_path("../parent"), ImportGroup::Relative);
    }

    /// Keep side effect imports stable and ahead of sorted regular imports.
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

        // keep side effects stable and sort regular imports
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
                target: "@/alias",
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

        // order by import group, then target
        let order = sort_import_declaration_indices(&keys);
        assert_eq!(order, vec![3, 2, 1, 0]);
    }

    /// Compare import targets by group priority before lexical order.
    #[test]
    fn test_compare_import_targets_uses_group_priority() {
        // compare target ordering across groups
        assert_eq!(compare_import_targets("node:fs", "react"), Ordering::Less);
        assert_eq!(compare_import_targets("react", "@/utils"), Ordering::Less);
        assert_eq!(compare_import_targets("@/utils", "./local"), Ordering::Less);
        assert_eq!(compare_import_targets("alpha", "beta"), Ordering::Less);
    }
}
