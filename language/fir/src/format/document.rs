use std::ops::Deref;

use rustc_hash::FxHashMap;

use crate::format::{ArenaVec, FitsExpanded, FormatNode, FormatTag, LineMode, NodeSlice, group};

/// A formatted document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document<'a> {
    nodes: &'a [FormatNode<'a>],
}

/// One active document scope that can receive or bound expansion.
#[derive(Debug)]
enum ExpansionScope<'a> {
    /// One enclosing group.
    Group(&'a group::Group),
    /// One enclosing conditional group.
    ConditionalGroup(&'a group::ConditionalGroup),
    /// One enclosing fits-expanded tag.
    FitsExpanded {
        /// The fits-expanded tag.
        tag: &'a FitsExpanded,
        /// Whether the enclosing frame expanded before this tag.
        expands_before: bool,
    },
    /// One best-fitting boundary.
    BestFitting,
    /// One best-fit parenthesize scope.
    BestFitParenthesize {
        /// Whether the enclosing frame expanded before this boundary.
        expanded: bool,
    },
}

impl ExpansionScope<'_> {
    /// Mark this scope as expanded when it owns line breaking.
    fn expand(&self) {
        match self {
            ExpansionScope::Group(group) => group.propagate_expand(),
            ExpansionScope::ConditionalGroup(group) => group.propagate_expand(),
            ExpansionScope::FitsExpanded { tag, .. } => tag.propagate_expand(),
            ExpansionScope::BestFitting | ExpansionScope::BestFitParenthesize { .. } => {}
        }
    }
}

/// The owner that receives a completed traversal frame.
#[derive(Debug)]
enum DocumentFrameOwner<'a> {
    /// The root document frame.
    Root,
    /// One reusable node-slice frame.
    Slice(&'a NodeSlice<'a>),
    /// One best-fitting variant frame.
    BestFittingVariant,
    /// One best-fitting boundary frame.
    BestFitting,
}

/// One node-slice traversal frame.
#[derive(Debug)]
struct DocumentFrame<'a> {
    /// The node slice being traversed.
    nodes: &'a [FormatNode<'a>],
    /// The next node index inside the slice.
    index: usize,
    /// Whether this frame has expanded.
    expands: bool,
    /// The owner that receives this frame's expansion state.
    owner: DocumentFrameOwner<'a>,
}

impl<'a> DocumentFrame<'a> {
    /// Create one traversal frame for a node slice.
    fn new(nodes: &'a [FormatNode<'a>], owner: DocumentFrameOwner<'a>) -> Self {
        Self {
            nodes,
            index: 0,
            expands: false,
            owner,
        }
    }

    /// Create one frame that exits a best-fitting boundary after its variants.
    fn best_fitting_boundary() -> Self {
        Self::new(&[], DocumentFrameOwner::BestFitting)
    }

    /// Apply expansion to this frame and its nearest expandable parent.
    fn expand(&mut self, enclosing: &[ExpansionScope<'_>]) {
        self.expands = true;

        if let Some(enclosing) = enclosing.last() {
            enclosing.expand();
        }
    }
}

impl Document<'_> {
    /// Propagate expanded layout state from line-breaking nodes to their enclosing groups.
    ///
    /// Groups expand if they contain any of:
    /// - a group with [`expand`](tag::Group::expand) set to [`GroupMode::Propagated`] or [`GroupMode::Expand`].
    /// - a non-soft [line break](FormatNode::Line) with mode [`LineMode::Hard`] or [`LineMode::Empty`].
    /// - a [`FormatNode::ExpandParent`]
    ///
    /// [`BestFitting`] nodes act as expand boundaries, meaning that the fact that a
    /// [`BestFitting`]'s content expands is not propagated past the [`BestFitting`] node.
    ///
    /// [`BestFitting`]: FormatNode::BestFitting
    pub(crate) fn propagate_expand(&mut self) {
        // create traversal state
        let mut enclosing = Vec::new();
        let mut slice_expands: FxHashMap<*const FormatNode<'_>, bool> = FxHashMap::default();
        let mut frames = vec![DocumentFrame::new(self, DocumentFrameOwner::Root)];

        while let Some(frame) = frames.last_mut() {
            // complete the current slice
            if frame.index >= frame.nodes.len() {
                let frame_index = frames.len() - 1;
                let DocumentFrame { expands, owner, .. } = frames.remove(frame_index);

                match owner {
                    DocumentFrameOwner::Root => break,
                    DocumentFrameOwner::Slice(slice) => {
                        slice_expands.insert(slice.as_ptr(), expands);

                        if expands && let Some(parent) = frames.last_mut() {
                            parent.expand(&enclosing);
                        }
                    }
                    DocumentFrameOwner::BestFittingVariant => {}
                    DocumentFrameOwner::BestFitting => {
                        enclosing.pop();
                    }
                }

                continue;
            }

            // read the next node
            let node = &frame.nodes[frame.index];
            frame.index += 1;

            // advance traversal state
            let node_expands = match node {
                FormatNode::Tag(FormatTag::StartGroup(group)) => {
                    enclosing.push(ExpansionScope::Group(group));
                    false
                }
                FormatNode::Tag(FormatTag::EndGroup) => match enclosing.pop() {
                    Some(ExpansionScope::Group(group)) => !group.mode().is_flat(),
                    _ => false,
                },
                FormatNode::Tag(FormatTag::StartBestFitParenthesize { .. }) => {
                    enclosing.push(ExpansionScope::BestFitParenthesize {
                        expanded: frame.expands,
                    });
                    frame.expands = false;
                    continue;
                }

                FormatNode::Tag(FormatTag::EndBestFitParenthesize) => {
                    if let Some(ExpansionScope::BestFitParenthesize { expanded }) = enclosing.pop()
                    {
                        frame.expands = expanded;
                    }
                    false
                }
                FormatNode::Tag(FormatTag::StartConditionalGroup(group)) => {
                    enclosing.push(ExpansionScope::ConditionalGroup(group));
                    false
                }
                FormatNode::Tag(FormatTag::EndConditionalGroup) => match enclosing.pop() {
                    Some(ExpansionScope::ConditionalGroup(group)) => !group.mode().is_flat(),
                    _ => false,
                },
                FormatNode::Slice(slice) => {
                    if let Some(expands) = slice_expands.get(&slice.as_ptr()) {
                        *expands
                    } else {
                        frames.push(DocumentFrame::new(slice, DocumentFrameOwner::Slice(slice)));
                        continue;
                    }
                }
                FormatNode::BestFitting { variants, mode: _ } => {
                    enclosing.push(ExpansionScope::BestFitting);

                    frames.push(DocumentFrame::best_fitting_boundary());
                    for variant in variants.as_slice().iter().rev() {
                        frames.push(DocumentFrame::new(
                            variant,
                            DocumentFrameOwner::BestFittingVariant,
                        ));
                    }
                    continue;
                }
                FormatNode::Tag(FormatTag::StartFitsExpanded(fits_expanded)) => {
                    enclosing.push(ExpansionScope::FitsExpanded {
                        tag: fits_expanded,
                        expands_before: frame.expands,
                    });
                    false
                }
                FormatNode::Tag(FormatTag::EndFitsExpanded) => {
                    if let Some(ExpansionScope::FitsExpanded { expands_before, .. }) =
                        enclosing.pop()
                    {
                        frame.expands = expands_before;
                    }

                    false
                }
                FormatNode::Text {
                    text: _,
                    width: text_width,
                } => text_width.is_multiline(),
                FormatNode::FileSlice {
                    width: text_width, ..
                } => text_width.is_multiline(),
                FormatNode::ExpandParent | FormatNode::Line(LineMode::Hard | LineMode::Empty) => {
                    true
                }
                _ => false,
            };

            // apply expansion to the current owner
            if node_expands && let Some(frame) = frames.last_mut() {
                frame.expand(&enclosing);
            }
        }
    }
}

impl<'a> From<ArenaVec<'a, FormatNode<'a>>> for Document<'a> {
    fn from(nodes: ArenaVec<'a, FormatNode<'a>>) -> Self {
        Self {
            nodes: nodes.into_slice(),
        }
    }
}

impl<'a> Deref for Document<'a> {
    type Target = [FormatNode<'a>];

    fn deref(&self) -> &Self::Target {
        self.nodes
    }
}

#[cfg(test)]
mod tests {
    use crate::format::{Allocator, ArenaVec, FormatTag, Group, GroupMode};

    use super::*;

    /// Propagate expansion through deeply nested node slices.
    #[test]
    fn test_propagate_expand_through_nested_node_slices() {
        let allocator = Allocator::default();
        let mut node = FormatNode::Line(LineMode::Hard);
        for _ in 0..100_000 {
            let nodes = ArenaVec::from_array_in([node], &allocator);
            node = FormatNode::Slice(NodeSlice::new(nodes));
        }

        let nodes = ArenaVec::from_array_in(
            [
                FormatNode::Tag(FormatTag::StartGroup(Group::new())),
                node,
                FormatNode::Tag(FormatTag::EndGroup),
            ],
            &allocator,
        );
        let mut document = Document::from(nodes);
        document.propagate_expand();

        let FormatNode::Tag(FormatTag::StartGroup(group)) = &document[0] else {
            panic!("expected start group");
        };

        assert_eq!(group.mode(), GroupMode::Propagated);
    }
}
