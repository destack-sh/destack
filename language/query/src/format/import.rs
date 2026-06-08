use std::cmp::Ordering;

use destack_core::StringPool;
use destack_dir as dir;
use destack_repository::ImportSortOrder;

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

/// Return dependency items sorted by space and configured key order.
pub fn sort_dependency_items(
    items: &[dir::LocalNodeId<dir::DependencyItem>],
    tree: &dir::Tree,
    strings: &StringPool,
    sort_order: ImportSortOrder,
) -> Vec<dir::LocalNodeId<dir::DependencyItem>> {
    let mut sorted_items: Vec<_> = items.to_vec();
    sorted_items.sort_by(|left_id, right_id| {
        let left_item = tree.get(*left_id);
        let right_item = tree.get(*right_id);

        // type imports come before value imports
        let left_is_type = left_item.form() == Some(dir::DependencyForm::Type);
        let right_is_type = right_item.form() == Some(dir::DependencyForm::Type);
        match (left_is_type, right_is_type) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }

        // alias key first when present, then item name
        let left_key = left_item
            .local_string_key()
            .map(|string_id| strings.get(string_id))
            .unwrap_or("");
        let right_key = right_item
            .local_string_key()
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
