use crate::format::{
    Allocator, ArenaVec, Arguments, FormatNode, FormatResult, FormatState, FormatTag, LineMode,
    NodeSlice, write,
};
use rustc_hash::FxHashMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

/// One sink for FIR nodes and formatting state.
pub trait Buffer<'a> {
    /// The context used during formatting.
    type Context;

    /// Write one FIR node.
    fn write_node(&mut self, node: FormatNode<'a>);

    /// Return all nodes written so far.
    ///
    /// Prefer using [BufferExtensions::start_recording] over accessing [Buffer::nodes] directly.
    #[doc(hidden)]
    fn nodes(&self) -> &[FormatNode<'a>];

    /// Write preconstructed format arguments.
    #[inline]
    fn write_format(
        mut self: &mut Self,
        arguments: Arguments<'_, 'a, Self::Context>,
    ) -> FormatResult<()> {
        write(&mut self, arguments)
    }

    /// Return the formatting state.
    fn state(&self) -> &FormatState<'a, Self::Context>;

    /// Return the formatting state mutably.
    fn state_mut(&mut self) -> &mut FormatState<'a, Self::Context>;
}

/// Forward buffer operations through mutable references.
impl<'a, W: Buffer<'a, Context = Context> + ?Sized, Context> Buffer<'a> for &mut W {
    type Context = Context;

    fn write_node(&mut self, node: FormatNode<'a>) {
        (**self).write_node(node);
    }

    fn nodes(&self) -> &[FormatNode<'a>] {
        (**self).nodes()
    }

    fn write_format(&mut self, args: Arguments<'_, 'a, Context>) -> FormatResult<()> {
        (**self).write_format(args)
    }

    fn state(&self) -> &FormatState<'a, Self::Context> {
        (**self).state()
    }

    fn state_mut(&mut self) -> &mut FormatState<'a, Self::Context> {
        (**self).state_mut()
    }
}

/// Arena-backed FIR buffer.
#[derive(Debug)]
pub struct VecBuffer<'state, 'a, Context> {
    state: &'state mut FormatState<'a, Context>,
    nodes: ArenaVec<'a, FormatNode<'a>>,
}

impl<'state, 'a, Context> VecBuffer<'state, 'a, Context> {
    /// Create an empty buffer in the formatting arena.
    pub fn new(state: &'state mut FormatState<'a, Context>) -> Self {
        let nodes = ArenaVec::new_in(state.allocator());

        Self::new_with_vec(state, nodes)
    }

    /// Create a buffer from an existing arena vector.
    pub fn new_with_vec(
        state: &'state mut FormatState<'a, Context>,
        nodes: ArenaVec<'a, FormatNode<'a>>,
    ) -> Self {
        Self { state, nodes }
    }

    /// Create a buffer with the specified capacity.
    pub fn with_capacity(capacity: usize, state: &'state mut FormatState<'a, Context>) -> Self {
        let nodes = ArenaVec::with_capacity_in(capacity, state.allocator());

        Self { state, nodes }
    }

    /// Return the written nodes.
    pub fn into_vec(self) -> ArenaVec<'a, FormatNode<'a>> {
        self.nodes
    }

    /// Take the written nodes and leave an empty buffer.
    pub fn take_vec(&mut self) -> ArenaVec<'a, FormatNode<'a>> {
        let nodes = ArenaVec::new_in(self.state.allocator());

        std::mem::replace(&mut self.nodes, nodes)
    }
}

impl<'a, Context> Deref for VecBuffer<'_, 'a, Context> {
    type Target = [FormatNode<'a>];

    fn deref(&self) -> &Self::Target {
        &self.nodes
    }
}

impl<Context> DerefMut for VecBuffer<'_, '_, Context> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.nodes
    }
}

impl<'a, Context> Buffer<'a> for VecBuffer<'_, 'a, Context> {
    type Context = Context;

    fn write_node(&mut self, node: FormatNode<'a>) {
        self.nodes.push(node);
    }

    fn nodes(&self) -> &[FormatNode<'a>] {
        self
    }

    fn state(&self) -> &FormatState<'a, Self::Context> {
        self.state
    }

    fn state_mut(&mut self) -> &mut FormatState<'a, Self::Context> {
        self.state
    }
}

/// Buffer that inspects each node before forwarding it.
pub struct Inspect<'inner, 'a, Context, Inspector> {
    inner: &'inner mut dyn Buffer<'a, Context = Context>,
    inspector: Inspector,
}

impl<'inner, 'a, Context, Inspector> Inspect<'inner, 'a, Context, Inspector> {
    fn new(inner: &'inner mut dyn Buffer<'a, Context = Context>, inspector: Inspector) -> Self {
        Self { inner, inspector }
    }
}

impl<Context, Inspector> Debug for Inspect<'_, '_, Context, Inspector> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inspect").finish()
    }
}

impl<'a, Context, Inspector> Buffer<'a> for Inspect<'_, 'a, Context, Inspector>
where
    Inspector: FnMut(&FormatNode<'a>),
{
    type Context = Context;

    fn write_node(&mut self, node: FormatNode<'a>) {
        (self.inspector)(&node);
        self.inner.write_node(node);
    }

    fn nodes(&self) -> &[FormatNode<'a>] {
        self.inner.nodes()
    }

    fn state(&self) -> &FormatState<'a, Self::Context> {
        self.inner.state()
    }

    fn state_mut(&mut self) -> &mut FormatState<'a, Self::Context> {
        self.inner.state_mut()
    }
}

/// One buffer that removes soft line behavior from forwarded FIR nodes.
///
/// Soft lines disappear, soft lines with spaces become spaces, expanded conditional content
/// disappears, and flat conditional content remains without its tags.
pub struct RemoveSoftLinesBuffer<'buf, 'a, Context> {
    inner: &'buf mut dyn Buffer<'a, Context = Context>,
    rewriter: SoftLineRewriter<'a>,
    filter: SoftLineFilter,
}

impl<'buf, 'a, Context> RemoveSoftLinesBuffer<'buf, 'a, Context> {
    /// Create one buffer that removes soft lines.
    pub fn new(inner: &'buf mut dyn Buffer<'a, Context = Context>) -> Self {
        let allocator = inner.state().allocator();

        Self {
            inner,
            rewriter: SoftLineRewriter::new(allocator),
            filter: SoftLineFilter::default(),
        }
    }
}

impl<Context> Debug for RemoveSoftLinesBuffer<'_, '_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoveSoftLinesBuffer").finish()
    }
}

/// Rewrites nested node slices without soft line behavior.
struct SoftLineRewriter<'a> {
    /// The formatter arena.
    allocator: &'a Allocator,
    /// Rewritten slices by original pointer identity.
    slices: FxHashMap<*const FormatNode<'a>, NodeSlice<'a>>,
}

impl<'a> SoftLineRewriter<'a> {
    /// Create one rewriter in a formatter arena.
    fn new(allocator: &'a Allocator) -> Self {
        Self {
            allocator,
            slices: FxHashMap::default(),
        }
    }

    /// Rewrite one node slice and cache its replacement.
    fn rewrite(&mut self, slice: NodeSlice<'a>) -> NodeSlice<'a> {
        let identity = slice.as_ptr();
        if let Some(rewritten) = self.slices.get(&identity) {
            return *rewritten;
        }

        // rewrite nested slices from leaves to root
        let mut frames = vec![SoftLineFrame::new(slice)];
        loop {
            let frame_index = frames.len() - 1;

            // complete one slice and publish it for its parent
            if frames[frame_index].index == frames[frame_index].original.len() {
                let frame = frames.remove(frame_index);
                let identity = frame.original.as_ptr();
                let rewritten = frame.finish();
                self.slices.insert(identity, rewritten);

                if frames.is_empty() {
                    return rewritten;
                }

                continue;
            }

            // rewrite one node
            let frame = &mut frames[frame_index];
            let node = &frame.original[frame.index];
            let nested = if !frame.filter.retain(node) {
                frame.replace(None, self.allocator);
                None
            } else {
                match node {
                    FormatNode::Line(LineMode::SoftOrSpace) => {
                        frame.replace(Some(FormatNode::Space), self.allocator);
                        None
                    }
                    FormatNode::Slice(slice) => {
                        let identity = slice.as_ptr();

                        if let Some(rewritten) = self.slices.get(&identity) {
                            if *rewritten == *slice {
                                frame.retain();
                            } else {
                                frame.replace(Some(FormatNode::Slice(*rewritten)), self.allocator);
                            }

                            None
                        } else {
                            Some(*slice)
                        }
                    }
                    _ => {
                        frame.retain();
                        None
                    }
                }
            };

            // process uncached nested content before advancing its parent
            if let Some(nested) = nested {
                frames.push(SoftLineFrame::new(nested));
            }
        }
    }
}

/// One active node-slice rewrite.
struct SoftLineFrame<'a> {
    /// The original node slice.
    original: NodeSlice<'a>,
    /// The next node index.
    index: usize,
    /// Conditional-content state at this slice depth.
    filter: SoftLineFilter,
    /// Rewritten nodes after the first change.
    rewritten: Option<ArenaVec<'a, FormatNode<'a>>>,
}

impl<'a> SoftLineFrame<'a> {
    /// Create one rewrite frame.
    fn new(original: NodeSlice<'a>) -> Self {
        Self {
            original,
            index: 0,
            filter: SoftLineFilter::default(),
            rewritten: None,
        }
    }

    /// Retain the current node and advance this frame.
    fn retain(&mut self) {
        if let Some(rewritten) = self.rewritten.as_mut() {
            rewritten.push(self.original[self.index].clone());
        }

        self.index += 1;
    }

    /// Replace or remove the current node and advance this frame.
    fn replace(&mut self, node: Option<FormatNode<'a>>, allocator: &'a Allocator) {
        // allocate at the first changed node
        if self.rewritten.is_none() {
            let mut rewritten = ArenaVec::with_capacity_in(self.original.len(), allocator);
            rewritten.extend_from_slice(&self.original[..self.index]);
            self.rewritten = Some(rewritten);
        }

        // write the replacement when present
        if let Some(rewritten) = self.rewritten.as_mut()
            && let Some(node) = node
        {
            rewritten.push(node);
        }

        self.index += 1;
    }

    /// Finish this frame as either its original or rewritten slice.
    fn finish(self) -> NodeSlice<'a> {
        match self.rewritten {
            Some(rewritten) => NodeSlice::new(rewritten),
            None => self.original,
        }
    }
}

impl<'a, Context> Buffer<'a> for RemoveSoftLinesBuffer<'_, 'a, Context> {
    type Context = Context;

    fn write_node(&mut self, node: FormatNode<'a>) {
        if !self.filter.retain(&node) {
            return;
        }

        let node = match node {
            FormatNode::Line(LineMode::SoftOrSpace) => FormatNode::Space,
            FormatNode::Slice(slice) => FormatNode::Slice(self.rewriter.rewrite(slice)),

            node => node,
        };

        self.inner.write_node(node);
    }

    fn nodes(&self) -> &[FormatNode<'a>] {
        self.inner.nodes()
    }

    fn state(&self) -> &FormatState<'a, Self::Context> {
        self.inner.state()
    }

    fn state_mut(&mut self) -> &mut FormatState<'a, Self::Context> {
        self.inner.state_mut()
    }
}

/// Conditional-content nesting hidden by soft-line removal.
#[derive(Copy, Clone, Debug, Default)]
struct SoftLineFilter {
    hidden_depth: usize,
}

impl SoftLineFilter {
    /// Advance conditional nesting and return whether one node should remain.
    fn retain(&mut self, node: &FormatNode<'_>) -> bool {
        // discard every node inside expanded conditional content
        if self.hidden_depth > 0 {
            match node {
                FormatNode::Tag(FormatTag::StartConditionalContent(_)) => {
                    self.hidden_depth += 1;
                }
                FormatNode::Tag(FormatTag::EndConditionalContent) => {
                    self.hidden_depth -= 1;
                }
                _ => {}
            }

            return false;
        }

        // remove soft lines and unwrap flat conditional content
        match node {
            FormatNode::Line(LineMode::Soft) => false,
            FormatNode::Tag(FormatTag::StartConditionalContent(condition)) => {
                if condition.mode.is_expanded() {
                    self.hidden_depth = 1;
                }

                false
            }
            FormatNode::Tag(FormatTag::EndConditionalContent) => false,
            _ => true,
        }
    }
}

/// Additional operations supported by every sized buffer.
pub trait BufferExtensions<'a>: Buffer<'a> + Sized {
    /// Create a buffer that inspects every written node.
    #[must_use]
    fn inspect<F>(&mut self, inspector: F) -> Inspect<'_, 'a, Self::Context, F>
    where
        F: FnMut(&FormatNode<'a>),
    {
        Inspect::new(self, inspector)
    }

    /// Start recording nodes written to this buffer.
    #[must_use]
    fn start_recording(&mut self) -> Recording<'_, 'a, Self> {
        Recording::new(self)
    }

    /// Write a sequence of nodes into this buffer.
    fn write_nodes<I>(&mut self, nodes: I)
    where
        I: IntoIterator<Item = FormatNode<'a>>,
    {
        for node in nodes {
            self.write_node(node);
        }
    }
}

impl<'a, T> BufferExtensions<'a> for T where T: Buffer<'a> {}

/// One active recording over a buffer.
#[derive(Debug)]
pub struct Recording<'buf, 'a, Buffer> {
    /// The first recorded node index.
    start: usize,
    /// The recorded buffer.
    buffer: &'buf mut Buffer,
    /// The FIR arena lifetime.
    lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'buf, 'a, B> Recording<'buf, 'a, B>
where
    B: Buffer<'a>,
{
    /// Start recording at the current buffer position.
    fn new(buffer: &'buf mut B) -> Self {
        Self {
            start: buffer.nodes().len(),
            buffer,
            lifetime: std::marker::PhantomData,
        }
    }

    /// Write preconstructed format arguments.
    #[inline]
    pub fn write_format(&mut self, arguments: Arguments<'_, 'a, B::Context>) -> FormatResult<()> {
        self.buffer.write_format(arguments)
    }

    /// Write one FIR node.
    #[inline]
    pub fn write_node(&mut self, node: FormatNode<'a>) {
        self.buffer.write_node(node);
    }

    /// Stop recording and return the recorded nodes.
    pub fn stop(self) -> Recorded<'buf, 'a> {
        let buffer: &'buf B = self.buffer;
        let nodes = buffer.nodes();

        let recorded = if self.start > nodes.len() {
            // May happen if buffer was rewound.
            &[]
        } else {
            &nodes[self.start..]
        };

        Recorded(recorded)
    }
}

/// FIR nodes captured by one recording.
#[derive(Debug, Copy, Clone)]
pub struct Recorded<'buf, 'a>(&'buf [FormatNode<'a>]);

impl<'a> Deref for Recorded<'_, 'a> {
    type Target = [FormatNode<'a>];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{Condition, PrintMode, SimpleFormatContext};
    use crate::prelude::*;
    use crate::{format, format_args, write};

    /// Write one FIR node into a vector buffer.
    #[test]
    fn test_buffer_write_node() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_destack(), &allocator);
        let mut buffer = VecBuffer::new(&mut state);

        buffer.write_node(FormatNode::Token { text: "test" });

        assert_eq!(
            buffer.into_vec().as_slice(),
            &[FormatNode::Token { text: "test" }]
        );
    }

    /// Write format arguments into a vector buffer.
    #[test]
    fn test_buffer_write_format() {
        let allocator = Allocator::default();
        let mut state = FormatState::new(SimpleFormatContext::empty_destack(), &allocator);
        let mut buffer = VecBuffer::new(&mut state);

        buffer
            .write_format(format_args!(token("Hello World")))
            .unwrap();

        assert_eq!(
            buffer.into_vec().as_slice(),
            &[FormatNode::Token {
                text: "Hello World"
            }]
        );
    }

    /// Remove soft line behavior while retaining ordinary content.
    #[test]
    fn test_remove_soft_lines_buffer() {
        let allocator = Allocator::default();
        let formatted = format!(
            &allocator,
            SimpleFormatContext::empty_destack(),
            [format_with(|f| {
                let mut buffer = RemoveSoftLinesBuffer::new(f);
                write!(
                    buffer,
                    [
                        token("The next soft line or space gets replaced by a space"),
                        soft_line_break_or_space(),
                        token("and the line here"),
                        soft_line_break(),
                        token("is removed entirely.")
                    ]
                )
            })]
        )
        .unwrap();

        assert_eq!(
            formatted.print().unwrap().as_str(),
            "The next soft line or space gets replaced by a space and the line hereis removed entirely."
        );
    }

    /// Record nodes written between two buffer positions.
    #[test]
    fn test_buffer_start_recording() {
        let allocator = Allocator::default();
        let formatted = format!(
            &allocator,
            SimpleFormatContext::empty_destack(),
            [format_with(|f| {
                let mut recording = f.start_recording();

                write!(recording, [token("A")])?;
                write!(recording, [token("B")])?;

                write!(
                    recording,
                    [format_with(|f| write!(f, [token("C"), token("D")]))]
                )?;

                let recorded = recording.stop();
                assert_eq!(
                    recorded.deref(),
                    &[
                        FormatNode::Token { text: "A" },
                        FormatNode::Token { text: "B" },
                        FormatNode::Token { text: "C" },
                        FormatNode::Token { text: "D" }
                    ]
                );

                Ok(())
            })]
        )
        .unwrap();

        assert_eq!(formatted.print().unwrap().as_str(), "ABCD");
    }

    /// Rewrite deeply nested node slices without consuming call stack.
    #[test]
    fn test_rewrite_nested_node_slice_soft_lines() {
        let allocator = Allocator::default();
        let nodes = ArenaVec::from_array_in([FormatNode::Line(LineMode::SoftOrSpace)], &allocator);
        let mut slice = NodeSlice::new(nodes);

        for _ in 0..100_000 {
            let nodes = ArenaVec::from_array_in([FormatNode::Slice(slice)], &allocator);
            slice = NodeSlice::new(nodes);
        }

        let mut rewriter = SoftLineRewriter::new(&allocator);
        let mut rewritten = rewriter.rewrite(slice);

        for _ in 0..100_000 {
            let [FormatNode::Slice(inner)] = &*rewritten else {
                panic!("expected nested node slice");
            };
            rewritten = *inner;
        }

        assert_eq!(&*rewritten, &[FormatNode::Space]);
    }

    /// Drop nested slices inside expanded conditional content.
    #[test]
    fn test_rewrite_nested_expanded_conditional_content() {
        let allocator = Allocator::default();
        let child = ArenaVec::from_array_in([FormatNode::Line(LineMode::SoftOrSpace)], &allocator);
        let child = NodeSlice::new(child);
        let nodes = ArenaVec::from_array_in(
            [
                FormatNode::Tag(FormatTag::StartConditionalContent(Condition::new(
                    PrintMode::Expanded,
                ))),
                FormatNode::Slice(child),
                FormatNode::Tag(FormatTag::EndConditionalContent),
            ],
            &allocator,
        );
        let slice = NodeSlice::new(nodes);

        let mut rewriter = SoftLineRewriter::new(&allocator);
        let rewritten = rewriter.rewrite(slice);

        assert!(rewritten.is_empty());
    }
}
