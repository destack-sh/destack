use std::cmp::Ordering;
use std::iter::Peekable;
use std::str::Chars;

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
        if let Some(colon_position) = path.find(':') {
            if !path[colon_position..].starts_with("://") {
                return Self::Builtin;
            }
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

        // alias name first when present, then item name
        let left_name = left_item
            .local_string_key()
            .map(|string_id| strings.get(string_id));
        let right_name = right_item
            .local_string_key()
            .map(|string_id| strings.get(string_id));

        match (left_name, right_name) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(left_name), Some(right_name)) => match sort_order {
                ImportSortOrder::Natural => compare_natural(left_name, right_name),
                ImportSortOrder::Alphabetical => left_name.cmp(right_name),
            },
        }
    });

    sorted_items
}

/// Return true when one import path uses a known alias prefix.
fn is_alias_specifier(specifier: &str) -> bool {
    specifier.starts_with("@/") || specifier.starts_with("~/") || specifier.starts_with('#')
}

/// Compare two strings using natural sort order.
fn compare_natural(left: &str, right: &str) -> Ordering {
    let mut left_chars = left.chars().peekable();
    let mut right_chars = right.chars().peekable();

    loop {
        match (left_chars.peek(), right_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left_char), Some(right_char)) => {
                // compare numeric runs without bounded integer parsing
                if left_char.is_ascii_digit() && right_char.is_ascii_digit() {
                    let left_digits = take_ascii_digits(&mut left_chars);
                    let right_digits = take_ascii_digits(&mut right_chars);

                    match compare_ascii_digits(&left_digits, &right_digits) {
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

/// Take one ASCII digit run from a character iterator.
fn take_ascii_digits(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut digits = String::new();

    while let Some(&character) = chars.peek() {
        if !character.is_ascii_digit() {
            break;
        }

        digits.push(character);
        chars.next();
    }

    digits
}

/// Compare two ASCII digit runs as natural-sort numbers.
fn compare_ascii_digits(left: &str, right: &str) -> Ordering {
    let left_significant = significant_digits(left);
    let right_significant = significant_digits(right);

    left_significant
        .len()
        .cmp(&right_significant.len())
        .then_with(|| left_significant.cmp(right_significant))
        .then_with(|| left.len().cmp(&right.len()))
}

/// Return the significant portion of one ASCII digit run.
fn significant_digits(digits: &str) -> &str {
    let significant = digits.trim_start_matches('0');

    if significant.is_empty() {
        "0"
    } else {
        significant
    }
}
