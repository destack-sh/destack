use crate::format::{FormatNode, FormatTag, FormatTagKind, PrintResult};
use crate::print::stack::{Stack, StackedStack};
use crate::print::{invalid_end_tag, invalid_start_tag};
use std::fmt::Debug;
use std::iter::FusedIterator;
use std::marker::PhantomData;

/// Queue of [`FormatNode`]s.
pub(crate) trait Queue<'a> {
    type Stack: Stack<&'a [FormatNode]>;

    fn stack(&self) -> &Self::Stack;

    fn stack_mut(&mut self) -> &mut Self::Stack;

    fn next_index(&self) -> usize;

    fn set_next_index(&mut self, index: usize);

    /// Pops the node at the end of the queue.
    fn pop(&mut self) -> Option<&'a FormatNode> {
        match self.stack().top() {
            Some(top_slice) => {
                let next_index = self.next_index();
                let node = &top_slice[next_index];

                if next_index + 1 == top_slice.len() {
                    self.stack_mut().pop().unwrap();
                    self.set_next_index(0);
                } else {
                    self.set_next_index(next_index + 1);
                }

                Some(node)
            }
            None => None,
        }
    }

    /// Returns the next node, not traversing into [`FormatNode::Interned`].
    fn top_with_interned(&self) -> Option<&'a FormatNode> {
        self.stack()
            .top()
            .map(|top_slice| &top_slice[self.next_index()])
    }

    /// Returns the next node, recursively resolving the first node of [`FormatNode::Interned`].
    fn top(&self) -> Option<&'a FormatNode> {
        let mut top = self.top_with_interned();

        while let Some(FormatNode::Interned(interned)) = top {
            top = interned.first();
        }

        top
    }

    /// Queues a single node to process before the other nodes in this queue.
    fn push(&mut self, node: &'a FormatNode) {
        self.extend_back(std::slice::from_ref(node));
    }

    /// Queues a slice of nodes to process before the other nodes in this queue.
    fn extend_back(&mut self, nodes: &'a [FormatNode]) {
        match nodes {
            [] => {}
            slice => {
                let next_index = self.next_index();
                let stack = self.stack_mut();

                if let Some(top) = stack.pop() {
                    stack.push(&top[next_index..]);
                }

                stack.push(slice);
                self.set_next_index(0);
            }
        }
    }

    /// Removes top slice.
    fn pop_slice(&mut self) -> Option<&'a [FormatNode]> {
        self.set_next_index(0);
        self.stack_mut().pop()
    }

    /// Skips all content until it finds the corresponding end tag with the given kind.
    fn skip_content(&mut self, kind: FormatTagKind)
    where
        Self: Sized,
    {
        let iter = self.iter_content(kind);

        // consume whole iterator until end
        for _ in iter {}
    }

    /// Iterates over all nodes until it finds the matching end tag of the specified kind.
    fn iter_content<'q>(&'q mut self, kind: FormatTagKind) -> QueueContentIterator<'a, 'q, Self>
    where
        Self: Sized,
    {
        QueueContentIterator::new(self, kind)
    }
}

/// Queue with the nodes to print.
#[derive(Debug, Default, Clone)]
pub(crate) struct PrintQueue<'a> {
    slices: Vec<&'a [FormatNode]>,
    next_index: usize,
}

impl<'a> PrintQueue<'a> {
    pub(crate) fn new(slice: &'a [FormatNode]) -> Self {
        let slices = match slice {
            [] => Vec::new(),
            slice => vec![slice],
        };

        Self {
            slices,
            next_index: 0,
        }
    }
}

impl<'a> Queue<'a> for PrintQueue<'a> {
    type Stack = Vec<&'a [FormatNode]>;

    fn stack(&self) -> &Self::Stack {
        &self.slices
    }

    fn stack_mut(&mut self) -> &mut Self::Stack {
        &mut self.slices
    }

    fn next_index(&self) -> usize {
        self.next_index
    }

    fn set_next_index(&mut self, index: usize) {
        self.next_index = index;
    }
}

/// Queue for measuring if an node fits on the line.
///
/// The queue is a view on top of the [`PrintQueue`] because no nodes should be removed
/// from the [`PrintQueue`] while measuring.
#[must_use]
#[derive(Debug)]
pub(crate) struct FitsQueue<'a, 'print> {
    stack: StackedStack<'print, &'a [FormatNode]>,
    next_index: usize,
}

impl<'a, 'print> FitsQueue<'a, 'print> {
    pub(super) fn new(print_queue: &'print PrintQueue<'a>, saved: Vec<&'a [FormatNode]>) -> Self {
        let stack = StackedStack::with_vec(&print_queue.slices, saved);

        Self {
            stack,
            next_index: print_queue.next_index,
        }
    }

    pub(super) fn finish(self) -> Vec<&'a [FormatNode]> {
        self.stack.into_vec()
    }
}

impl<'a, 'print> Queue<'a> for FitsQueue<'a, 'print> {
    type Stack = StackedStack<'print, &'a [FormatNode]>;

    fn stack(&self) -> &Self::Stack {
        &self.stack
    }

    fn stack_mut(&mut self) -> &mut Self::Stack {
        &mut self.stack
    }

    fn next_index(&self) -> usize {
        self.next_index
    }

    fn set_next_index(&mut self, index: usize) {
        self.next_index = index;
    }
}

pub(crate) struct QueueContentIterator<'a, 'q, Q: Queue<'a>> {
    queue: &'q mut Q,
    kind: FormatTagKind,
    depth: usize,
    lifetime: PhantomData<&'a ()>,
}

impl<'a, 'q, Q> QueueContentIterator<'a, 'q, Q>
where
    Q: Queue<'a>,
{
    fn new(queue: &'q mut Q, kind: FormatTagKind) -> Self {
        Self {
            queue,
            kind,
            depth: 1,
            lifetime: PhantomData,
        }
    }
}

impl<'a, Q> Iterator for QueueContentIterator<'a, '_, Q>
where
    Q: Queue<'a>,
{
    type Item = &'a FormatNode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.depth == 0 {
            None
        } else {
            let mut top = self.queue.pop();

            // resolve interned nodes by extending the queue
            while let Some(FormatNode::Interned(interned)) = top {
                self.queue.extend_back(interned);
                top = self.queue.pop();
            }

            match top.expect("missing end signal") {
                node @ FormatNode::Tag(tag) if tag.kind() == self.kind => {
                    if tag.is_start() {
                        self.depth += 1;
                    } else {
                        self.depth -= 1;

                        if self.depth == 0 {
                            return None;
                        }
                    }

                    Some(node)
                }
                node => Some(node),
            }
        }
    }
}

impl<'a, Q> FusedIterator for QueueContentIterator<'a, '_, Q> where Q: Queue<'a> {}

/// A predicate determining when to end measuring if some content fits on the line.
///
/// Called for every [`node`](FormatNode) in the [`FitsQueue`] when measuring if a content
/// fits on the line.
/// The measuring of the content ends after the first node [`node`](FormatNode) for which this
/// predicate returns `true` (similar to a take while iterator except that it takes while the predicate returns `false`).
pub(super) trait FitsEndPredicate {
    fn is_end(&mut self, node: &FormatNode) -> PrintResult<bool>;
}

/// Filter that includes all nodes until it reaches the end of the document.
pub(super) struct AllPredicate;

impl FitsEndPredicate for AllPredicate {
    fn is_end(&mut self, _node: &FormatNode) -> PrintResult<bool> {
        Ok(false)
    }
}

/// Filter that takes all nodes between two matching [`Tag::StartEntry`] and [`Tag::EndEntry`] tags.
#[derive(Debug)]
pub(super) enum SingleEntryPredicate {
    Entry { depth: usize },
    Done,
}

impl SingleEntryPredicate {
    pub(super) const fn is_done(&self) -> bool {
        matches!(self, SingleEntryPredicate::Done)
    }
}

impl Default for SingleEntryPredicate {
    fn default() -> Self {
        SingleEntryPredicate::Entry { depth: 0 }
    }
}

impl FitsEndPredicate for SingleEntryPredicate {
    fn is_end(&mut self, node: &FormatNode) -> PrintResult<bool> {
        let result = match self {
            SingleEntryPredicate::Done => true,
            SingleEntryPredicate::Entry { depth } => match node {
                FormatNode::Tag(FormatTag::StartEntry) => {
                    *depth += 1;

                    false
                }
                FormatNode::Tag(FormatTag::EndEntry) => {
                    if *depth == 0 {
                        return invalid_end_tag(FormatTagKind::Entry, None);
                    }

                    *depth -= 1;

                    let is_end = *depth == 0;

                    if is_end {
                        *self = SingleEntryPredicate::Done;
                    }

                    is_end
                }
                FormatNode::Interned(_) => false,
                node if *depth == 0 => {
                    return invalid_start_tag(FormatTagKind::Entry, Some(node));
                }
                _ => false,
            },
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::format::{FormatNode, FormatTag, LineMode};
    use crate::print::queue::{PrintQueue, Queue};

    #[test]
    fn test_extend_back_pop_last() {
        // extend_back should add nodes to be processed before existing ones
        let mut queue =
            PrintQueue::new(&[FormatNode::Tag(FormatTag::StartEntry), FormatNode::Space]);

        assert_eq!(queue.pop(), Some(&FormatNode::Tag(FormatTag::StartEntry)));

        queue.extend_back(&[FormatNode::Line(LineMode::SoftOrSpace)]);

        assert_eq!(queue.pop(), Some(&FormatNode::Line(LineMode::SoftOrSpace)));
        assert_eq!(queue.pop(), Some(&FormatNode::Space));

        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn test_extend_back_empty_queue() {
        // extend_back should work correctly when queue becomes empty
        let mut queue =
            PrintQueue::new(&[FormatNode::Tag(FormatTag::StartEntry), FormatNode::Space]);

        assert_eq!(queue.pop(), Some(&FormatNode::Tag(FormatTag::StartEntry)));
        assert_eq!(queue.pop(), Some(&FormatNode::Space));

        queue.extend_back(&[FormatNode::Line(LineMode::SoftOrSpace)]);

        assert_eq!(queue.pop(), Some(&FormatNode::Line(LineMode::SoftOrSpace)));

        assert_eq!(queue.pop(), None);
    }
}
