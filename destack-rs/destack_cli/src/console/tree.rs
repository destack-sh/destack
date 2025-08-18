//! Simple ASCII tree rendering.

use super::console::color;
use std::collections::BTreeMap;

/// Render a tree from a mapping of id -> children and labels.
///
/// Creates an ASCII tree visualization with optional highlighting of a specific node.
/// Children are sorted alphabetically by their labels for consistent output.
pub fn render_tree(
    root_id: &str,
    children_by_id: &BTreeMap<String, Vec<String>>,
    label_by_id: &BTreeMap<String, String>,
    highlight_id: Option<&str>,
) -> String {
    /// Style a label with optional highlighting.
    fn style_label(
        id: &str,
        label_by_id: &BTreeMap<String, String>,
        highlight: Option<&str>,
    ) -> String {
        let label = label_by_id
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string());
        if Some(id) == highlight {
            color(&label, "97;1")
        } else {
            label
        }
    }

    /// Recursively walk the tree and build the ASCII representation.
    fn walk(
        out: &mut String,
        id: &str,
        prefix: &str,
        is_last: bool,
        children_by_id: &BTreeMap<String, Vec<String>>,
        label_by_id: &BTreeMap<String, String>,
        highlight: Option<&str>,
    ) {
        let connector = if is_last { "└─ " } else { "├─ " };
        if prefix.is_empty() {
            // root node has no prefix
            out.push_str(&style_label(id, label_by_id, highlight));
        } else {
            out.push_str(prefix);
            out.push_str(connector);
            out.push_str(&style_label(id, label_by_id, highlight));
        }
        out.push('\n');

        // sort children by label for consistent output
        let mut children = children_by_id.get(id).cloned().unwrap_or_default();
        children.sort_by_key(|cid| {
            label_by_id
                .get(cid)
                .cloned()
                .unwrap_or_else(|| cid.to_string())
        });
        if children.is_empty() {
            return;
        }

        // build prefix for child nodes
        let next_prefix = format!("{}{}", prefix, if is_last { "   " } else { "│  " });
        for (idx, child) in children.iter().enumerate() {
            let last = idx + 1 == children.len();
            walk(
                out,
                child,
                &next_prefix,
                last,
                children_by_id,
                label_by_id,
                highlight,
            );
        }
    }

    // start from the root node
    let mut out = String::new();
    walk(
        &mut out,
        root_id,
        "",
        true,
        children_by_id,
        label_by_id,
        highlight_id,
    );
    out
}
