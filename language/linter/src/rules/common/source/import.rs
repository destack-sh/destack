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

/// Resolve the span that removes one top level `type` keyword from an import declaration.
pub fn import_type_keyword_removal_span(import_span: Span, import_text: &str) -> Option<Span> {
    let keyword_offset = import_text.find("import")?;
    let mut type_start = keyword_offset + "import".len();
    let bytes = import_text.as_bytes();

    while type_start < bytes.len() && bytes[type_start].is_ascii_whitespace() {
        type_start += 1;
    }

    if !import_text[type_start..].starts_with("type") {
        return None;
    }

    let type_end = type_start + "type".len();
    if type_end < bytes.len() && !bytes[type_end].is_ascii_whitespace() && bytes[type_end] != b'{' {
        return None;
    }

    let removal_start = import_span.start + (keyword_offset + "import".len()) as u32;
    let removal_end = import_span.start + type_end as u32;
    Some(Span::new(import_span.file, removal_start, removal_end))
}

/// Remove one leading `type` keyword from one dependency item text.
pub fn dependency_item_strip_inline_type_keyword(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut cursor = 0;

    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }

    if !text[cursor..].starts_with("type") {
        return None;
    }

    let keyword_end = cursor + "type".len();
    if keyword_end >= bytes.len() || !bytes[keyword_end].is_ascii_whitespace() {
        return None;
    }

    let mut content_start = keyword_end;
    while content_start < bytes.len() && bytes[content_start].is_ascii_whitespace() {
        content_start += 1;
    }

    let mut rewritten = String::new();
    rewritten.push_str(&text[..cursor]);
    rewritten.push_str(&text[content_start..]);
    Some(rewritten)
}

/// Add one leading `type` keyword to one dependency item text.
pub fn dependency_item_insert_inline_type_keyword(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut cursor = 0;

    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }

    if text[cursor..].starts_with("type")
        && (cursor + "type".len()) < bytes.len()
        && bytes[cursor + "type".len()].is_ascii_whitespace()
    {
        return text.to_string();
    }

    let mut rewritten = String::new();
    rewritten.push_str(&text[..cursor]);
    rewritten.push_str("type ");
    rewritten.push_str(&text[cursor..]);
    rewritten
}

#[cfg(test)]
mod tests {
    use super::*;
    use destack_source::FileId;

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

    /// Resolve the exact removal span for top level `import type`.
    #[test]
    fn test_import_type_keyword_removal_span() {
        let file = FileId::new(1);
        let text = "import type { Foo } from \"foo\"";
        let span = Span::new(file, 0, text.len() as u32);
        let removal_span = import_type_keyword_removal_span(span, text)
            .expect("expected top level type keyword span");
        let removed_text = &text[removal_span.start as usize..removal_span.end as usize];
        assert_eq!(removed_text, " type");
    }

    /// Remove one inline `type` keyword from one dependency item.
    #[test]
    fn test_dependency_item_strip_inline_type_keyword() {
        let rewritten = dependency_item_strip_inline_type_keyword(" type Foo as Bar")
            .expect("expected rewritten item");
        assert_eq!(rewritten, " Foo as Bar");
    }

    /// Add one inline `type` keyword to one dependency item.
    #[test]
    fn test_dependency_item_insert_inline_type_keyword() {
        let rewritten = dependency_item_insert_inline_type_keyword(" Foo as Bar");
        assert_eq!(rewritten, " type Foo as Bar");
    }
}
