use std::collections::HashMap;
use std::ops::Deref;

use crate::format::{FitsExpanded, FormatNode, FormatTag, Interned, LineMode, group};

/// A formatted document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    nodes: Vec<FormatNode>,
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
    /// One interned node-slice frame.
    Interned(&'a Interned),
    /// One best-fitting variant frame.
    BestFittingVariant,
    /// One best-fitting boundary frame.
    BestFitting,
}

/// One node-slice traversal frame.
#[derive(Debug)]
struct DocumentFrame<'a> {
    /// The node slice being traversed.
    nodes: &'a [FormatNode],
    /// The next node index inside the slice.
    index: usize,
    /// Whether this frame has expanded.
    expands: bool,
    /// The owner that receives this frame's expansion state.
    owner: DocumentFrameOwner<'a>,
}

impl<'a> DocumentFrame<'a> {
    /// Create one traversal frame for a node slice.
    fn new(nodes: &'a [FormatNode], owner: DocumentFrameOwner<'a>) -> Self {
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

impl Document {
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
        let mut enclosing = Vec::with_capacity(if self.is_empty() {
            0
        } else {
            self.len().ilog2() as usize
        });
        let mut interned_expands: HashMap<*const Vec<FormatNode>, bool> = HashMap::new();
        let mut frames = vec![DocumentFrame::new(self, DocumentFrameOwner::Root)];

        while let Some(frame) = frames.last_mut() {
            // complete the current slice
            if frame.index >= frame.nodes.len() {
                let Some(DocumentFrame { expands, owner, .. }) = frames.pop() else {
                    break;
                };

                match owner {
                    DocumentFrameOwner::Root => break,
                    DocumentFrameOwner::Interned(interned) => {
                        interned_expands.insert(interned.nodes_ptr(), expands);

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
                FormatNode::Interned(interned) => {
                    if let Some(expands) = interned_expands.get(&interned.nodes_ptr()) {
                        *expands
                    } else {
                        frames.push(DocumentFrame::new(
                            interned,
                            DocumentFrameOwner::Interned(interned),
                        ));
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

impl From<Vec<FormatNode>> for Document {
    fn from(nodes: Vec<FormatNode>) -> Self {
        Self { nodes }
    }
}

impl Deref for Document {
    type Target = [FormatNode];

    fn deref(&self) -> &Self::Target {
        self.nodes.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use crate::format::{FormatTag, Group, GroupMode};

    use super::*;

    /// Expansion should propagate through large interned chains.
    #[test]
    fn test_propagate_expand_deep_interned_chain() {
        let mut node = FormatNode::Line(LineMode::Hard);
        for _ in 0..100_000 {
            node = FormatNode::Interned(Interned::new(vec![node]));
        }

        let mut document = Document::from(vec![
            FormatNode::Tag(FormatTag::StartGroup(Group::new())),
            node,
            FormatNode::Tag(FormatTag::EndGroup),
        ]);
        document.propagate_expand();

        let FormatNode::Tag(FormatTag::StartGroup(group)) = &document[0] else {
            panic!("expected start group");
        };

        assert_eq!(group.mode(), GroupMode::Propagated);
    }
}
