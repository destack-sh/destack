use std::cmp::Ordering;

use destack_fir::format::{Format, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use super::trivia::{
    write_inline_comment_after, write_node_leading_comments,
    write_node_leading_comments_after_separator, write_top_level_comments_after,
};

use crate::{
    FormatNode, Formatter, Function, Global, LocalNodeId, Node, Tree, TreeImpl, TypeDeclaration,
    Writer,
};

/// One explicit top-level MIR item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Item {
    /// One named type declaration.
    Type(LocalNodeId<TypeDeclaration>),
    /// One global declaration.
    Global(LocalNodeId<Global>),
    /// One function declaration or definition.
    Function(LocalNodeId<Function>),
}

impl Item {
    /// Return the underlying node id.
    fn node_id(self) -> u32 {
        match self {
            Self::Type(id) => id.id,
            Self::Global(id) => id.id,
            Self::Function(id) => id.id,
        }
    }
}

impl<'a> Format<'a, Formatter<'a>> for Tree {
    fn format(&self, writer: &mut Writer<'a, '_>) -> FormatResult<()> {
        // drop the items a selective format leaves out
        let mut items = items(self);
        if let Some(selection) = writer.context().selection() {
            items.retain(|(_, item)| match item {
                Item::Type(id) => selection.declarations.contains(id),
                Item::Global(_) => false,
                Item::Function(id) => selection.functions.contains(id),
            });
        }

        // preserve authored order and place generated nodes after authored nodes
        for (index, (_, item)) in items.iter().enumerate() {
            let next_boundary = items.get(index + 1).and_then(|(start, _)| *start);
            let has_previous = index > 0;

            // consecutive globals group into one tight block
            let grouped = has_previous
                && matches!(item, Item::Global(_))
                && matches!(items[index - 1].1, Item::Global(_));

            match item {
                Item::Type(id) => {
                    write_item(self, *id, next_boundary, has_previous, false, writer)?;
                }
                Item::Global(id) => {
                    write_item(self, *id, next_boundary, has_previous, grouped, writer)?;
                }
                Item::Function(id) => {
                    write_item(self, *id, next_boundary, has_previous, false, writer)?;
                }
            }
        }

        Ok(())
    }
}

/// Collect top-level items in stable source order.
fn items(tree: &Tree) -> Vec<(Option<u32>, Item)> {
    let mut items = Vec::new();

    for (id, declaration) in tree.iter_nodes::<TypeDeclaration>() {
        if declaration.name.is_some() {
            items.push((item_start(tree, id), Item::Type(id)));
        }
    }
    for (id, _) in tree.iter_nodes::<Global>() {
        items.push((item_start(tree, id), Item::Global(id)));
    }
    for (id, _) in tree.iter_nodes::<Function>() {
        items.push((item_start(tree, id), Item::Function(id)));
    }

    items.sort_by(
        |(left_start, left), (right_start, right)| match (left_start, right_start) {
            (Some(left_start), Some(right_start)) => left_start
                .cmp(right_start)
                .then_with(|| left.node_id().cmp(&right.node_id())),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => left.cmp(right),
        },
    );

    items
}

/// Return the authored source start for one top-level item.
fn item_start<T>(tree: &Tree, id: LocalNodeId<T>) -> Option<u32>
where
    T: Node,
{
    // skip items anchored at a lowered source origin
    if tree.get_source(id.id).is_some() {
        return None;
    }

    if let Some(span) = tree.leading_comment_span(id) {
        Some(span.start)
    } else {
        tree.get_span(id).map(|span| span.start)
    }
}

/// Write one explicit top-level item.
fn write_item<'a, T>(
    tree: &Tree,
    id: LocalNodeId<T>,
    next_boundary: Option<u32>,
    has_previous: bool,
    grouped: bool,
    writer: &mut Writer<'a, '_>,
) -> FormatResult<()>
where
    T: FormatNode,
    Tree: TreeImpl<T>,
{
    // separate declarations with one empty line, grouped items with a plain break
    if grouped {
        write!(writer, [hard_line_break()])?;
        write_node_leading_comments_after_separator(tree, id, writer)?;
    } else if has_previous {
        write!(writer, [empty_line()])?;
        write_node_leading_comments_after_separator(tree, id, writer)?;
    } else {
        write_node_leading_comments(tree, id, writer)?;
    }

    // write the item and any following comments
    write!(writer, [id])?;
    if let Some(next_boundary) = next_boundary
        && let Some(span) = tree.get_span(id)
    {
        write_inline_comment_after(tree, span.end, next_boundary, writer)?;
    } else if next_boundary.is_none() {
        write_top_level_comments_after(tree, id, writer)?;
    }

    Ok(())
}
