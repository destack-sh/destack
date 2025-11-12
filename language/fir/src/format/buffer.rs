use crate::format::{
    Arguments, FormatNode, FormatResult, FormatState, FormatTag, Interned, LineMode, write,
};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::num::NonZeroUsize;
use std::ops::{Deref, DerefMut};

/// A trait for writing or formatting into [`FormatNode`]-accepting buffers or streams.
pub trait Buffer {
    /// The context used during formatting
    type Context;

    /// Writes a [`crate::FormatNode`] into this buffer, returning whether the write succeeded.
    ///
    /// # Errors
    /// This function will return an instance of [`crate::FormatError`] on error.
    fn write_node(&mut self, node: FormatNode);

    /// Returns a slice containing all nodes written into this buffer.
    ///
    /// Prefer using [BufferExtensions::start_recording] over accessing [Buffer::nodes] directly.
    #[doc(hidden)]
    fn nodes(&self) -> &[FormatNode];

    /// Glue for usage of the [`write!`] macro with implementers of this trait.
    ///
    /// This method should generally not be invoked manually, but rather through the [`write!`] macro itself.
    #[inline]
    fn write_format(
        mut self: &mut Self,
        arguments: Arguments<'_, Self::Context>,
    ) -> FormatResult<()> {
        write(&mut self, arguments)
    }

    /// Returns the formatting state relevant for this formatting session.
    fn state(&self) -> &FormatState<Self::Context>;

    /// Returns the mutable formatting state relevant for this formatting session.
    fn state_mut(&mut self) -> &mut FormatState<Self::Context>;

    /// Takes a snapshot of the Buffers state, excluding the formatter state.
    fn snapshot(&self) -> BufferSnapshot;

    /// Restores the snapshot buffer
    ///
    /// ## Panics
    /// If the passed snapshot id is a snapshot of another buffer OR
    /// if the snapshot is restored out of order
    fn restore_snapshot(&mut self, snapshot: BufferSnapshot);
}

/// Snapshot of a buffer state that can be restored at a later point.
///
/// Used in cases where the formatting of an object fails but a parent formatter knows an alternative
/// strategy on how to format the object that might succeed.
#[derive(Debug)]
pub enum BufferSnapshot {
    /// Stores an absolute position of a buffers state, for example, the offset of the last written node.
    Position(usize),

    /// Generic structure for custom buffers that need to store more complex data. Slightly more
    /// expensive because it requires allocating the buffer state on the heap.
    Any(Box<dyn Any>),
}

impl BufferSnapshot {
    /// Creates a new buffer snapshot that points to the specified position.
    pub const fn position(index: usize) -> Self {
        Self::Position(index)
    }

    /// Unwraps the position value.
    ///
    /// # Panics
    ///
    /// If self is not a [`BufferSnapshot::Position`]
    pub fn unwrap_position(&self) -> usize {
        match self {
            BufferSnapshot::Position(index) => *index,
            BufferSnapshot::Any(_) => panic!("cannot unwrap position from Any snapshot"),
        }
    }

    /// Unwraps the any value.
    ///
    /// # Panics
    ///
    /// If `self` is not a [`BufferSnapshot::Any`].
    pub fn unwrap_any<T: 'static>(self) -> T {
        match self {
            BufferSnapshot::Position(_) => {
                panic!("cannot unwrap Any snapshot from Position snapshot")
            }
            BufferSnapshot::Any(value) => match value.downcast::<T>() {
                Ok(snapshot) => *snapshot,
                Err(err) => {
                    panic!(
                        "cannot unwrap snapshot of type {:?} as {:?}",
                        (*err).type_id(),
                        TypeId::of::<T>()
                    )
                }
            },
        }
    }
}

/// Implements the `[Buffer]` trait for all mutable references of objects implementing [Buffer].
impl<W: Buffer<Context = Context> + ?Sized, Context> Buffer for &mut W {
    type Context = Context;

    fn write_node(&mut self, node: FormatNode) {
        (**self).write_node(node);
    }

    fn nodes(&self) -> &[FormatNode] {
        (**self).nodes()
    }

    fn write_format(&mut self, args: Arguments<'_, Context>) -> FormatResult<()> {
        (**self).write_format(args)
    }

    fn state(&self) -> &FormatState<Self::Context> {
        (**self).state()
    }

    fn state_mut(&mut self) -> &mut FormatState<Self::Context> {
        (**self).state_mut()
    }

    fn snapshot(&self) -> BufferSnapshot {
        (**self).snapshot()
    }

    fn restore_snapshot(&mut self, snapshot: BufferSnapshot) {
        (**self).restore_snapshot(snapshot);
    }
}

/// Vector backed [`Buffer`] implementation.
///
/// The buffer writes all nodes into the internal nodes buffer.
#[derive(Debug)]
pub struct VecBuffer<'a, Context> {
    state: &'a mut FormatState<Context>,
    nodes: Vec<FormatNode>,
}

impl<'a, Context> VecBuffer<'a, Context> {
    pub fn new(state: &'a mut FormatState<Context>) -> Self {
        Self::new_with_vec(state, Vec::new())
    }

    pub fn new_with_vec(state: &'a mut FormatState<Context>, nodes: Vec<FormatNode>) -> Self {
        Self { state, nodes }
    }

    /// Creates a buffer with the specified capacity
    pub fn with_capacity(capacity: usize, state: &'a mut FormatState<Context>) -> Self {
        Self {
            state,
            nodes: Vec::with_capacity(capacity),
        }
    }

    /// Consumes the buffer and returns the written [`FormatNode]`s as a vector.
    pub fn into_vec(self) -> Vec<FormatNode> {
        self.nodes
    }

    /// Takes the nodes without consuming self
    pub fn take_vec(&mut self) -> Vec<FormatNode> {
        std::mem::take(&mut self.nodes)
    }
}

impl<Context> Deref for VecBuffer<'_, Context> {
    type Target = [FormatNode];

    fn deref(&self) -> &Self::Target {
        &self.nodes
    }
}

impl<Context> DerefMut for VecBuffer<'_, Context> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.nodes
    }
}

impl<Context> Buffer for VecBuffer<'_, Context> {
    type Context = Context;

    fn write_node(&mut self, node: FormatNode) {
        self.nodes.push(node);
    }

    fn nodes(&self) -> &[FormatNode] {
        self
    }

    fn state(&self) -> &FormatState<Self::Context> {
        self.state
    }

    fn state_mut(&mut self) -> &mut FormatState<Self::Context> {
        self.state
    }

    fn snapshot(&self) -> BufferSnapshot {
        BufferSnapshot::position(self.nodes.len())
    }

    fn restore_snapshot(&mut self, snapshot: BufferSnapshot) {
        let position = snapshot.unwrap_position();
        assert!(
            self.nodes.len() >= position,
            r#"Outdated snapshot. This buffer contains fewer nodes than at the time the snapshot was taken.
Make sure that you take and restore the snapshot in order and that this snapshot belongs to the current buffer."#
        );

        self.nodes.truncate(position);
    }
}

/// Buffer that allows you inspecting nodes as they get written to the formatter.
pub struct Inspect<'inner, Context, Inspector> {
    inner: &'inner mut dyn Buffer<Context = Context>,
    inspector: Inspector,
}

impl<'inner, Context, Inspector> Inspect<'inner, Context, Inspector> {
    fn new(inner: &'inner mut dyn Buffer<Context = Context>, inspector: Inspector) -> Self {
        Self { inner, inspector }
    }
}

impl<Context, Inspector> Debug for Inspect<'_, Context, Inspector> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inspect").finish()
    }
}

impl<Context, Inspector> Buffer for Inspect<'_, Context, Inspector>
where
    Inspector: FnMut(&FormatNode),
{
    type Context = Context;

    fn write_node(&mut self, node: FormatNode) {
        (self.inspector)(&node);
        self.inner.write_node(node);
    }

    fn nodes(&self) -> &[FormatNode] {
        self.inner.nodes()
    }

    fn state(&self) -> &FormatState<Self::Context> {
        self.inner.state()
    }

    fn state_mut(&mut self) -> &mut FormatState<Self::Context> {
        self.inner.state_mut()
    }

    fn snapshot(&self) -> BufferSnapshot {
        self.inner.snapshot()
    }

    fn restore_snapshot(&mut self, snapshot: BufferSnapshot) {
        self.inner.restore_snapshot(snapshot);
    }
}

/// A Buffer that removes any soft line breaks or [`if_group_breaks`](crate::builders::if_group_breaks) nodes.
/// - Removes [`lines`](FormatNode::Line) with the mode [`Soft`](LineMode::Soft).
/// - Replaces [`lines`](FormatNode::Line) with the mode [`Soft`](LineMode::SoftOrSpace) with a [`Space`](FormatNode::Space)
/// - Removes [`if_group_breaks`](crate::builders::if_group_breaks) and all its content.
/// - Unwraps the content of [`if_group_fits_on_line`](crate::builders::if_group_fits_on_line) nodes (but retains it).
pub struct RemoveSoftLinesBuffer<'a, Context> {
    inner: &'a mut dyn Buffer<Context = Context>,

    /// Caches the interned nodes after the soft line breaks have been removed.
    ///
    /// The `key` is the [Interned] node as it has been passed to [`Self::write_node`] or the child of another
    /// [Interned] node. The `value` is the matching document of the key where all soft line breaks have been removed.
    ///
    /// It's fine to not snapshot the cache. The worst that can happen is that it holds on interned nodes
    /// that are now unused. But there's little harm in that and the cache is cleaned when dropping the buffer.
    interned_cache: HashMap<Interned, Interned>,

    state: RemoveSoftLineBreaksState,
}

impl<'a, Context> RemoveSoftLinesBuffer<'a, Context> {
    /// Creates a new buffer that removes the soft line breaks before writing them into `buffer`.
    pub fn new(inner: &'a mut dyn Buffer<Context = Context>) -> Self {
        Self {
            inner,
            state: RemoveSoftLineBreaksState::default(),
            interned_cache: HashMap::default(),
        }
    }

    /// Removes the soft line breaks from an interned node.
    fn clean_interned(&mut self, interned: &Interned) -> Interned {
        clean_interned(interned, &mut self.interned_cache)
    }
}

impl<Context> Debug for RemoveSoftLinesBuffer<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoveSoftLinesBuffer").finish()
    }
}

// Extracted to function to avoid monomorphization
#[allow(clippy::mutable_key_type)]
fn clean_interned(
    interned: &Interned,
    interned_cache: &mut HashMap<Interned, Interned>,
) -> Interned {
    if let Some(cleaned) = interned_cache.get(interned) {
        cleaned.clone()
    } else {
        let mut state = RemoveSoftLineBreaksState::default();

        // Find the first soft line break node or interned node that must be changed
        let result = interned
            .iter()
            .enumerate()
            .find_map(|(index, node)| match node {
                FormatNode::Line(LineMode::SoftOrSpace) => {
                    let mut cleaned = Vec::new();
                    let (before, after) = interned.split_at(index);
                    cleaned.extend_from_slice(before);
                    Some((cleaned, &after[1..]))
                }
                FormatNode::Interned(inner) => {
                    let cleaned_inner = clean_interned(inner, interned_cache);

                    if &cleaned_inner == inner {
                        None
                    } else {
                        let mut cleaned = Vec::with_capacity(interned.len());
                        cleaned.extend_from_slice(&interned[..index]);
                        cleaned.push(FormatNode::Interned(cleaned_inner));
                        Some((cleaned, &interned[index + 1..]))
                    }
                }

                node => {
                    if state.should_drop(node) {
                        let mut cleaned = Vec::new();
                        let (before, after) = interned.split_at(index);
                        cleaned.extend_from_slice(before);
                        Some((cleaned, &after[1..]))
                    } else {
                        None
                    }
                }
            });

        let result = match result {
            // Copy the whole interned buffer so that becomes possible to change the necessary nodes.
            Some((mut cleaned, rest)) => {
                for node in rest {
                    if state.should_drop(node) {
                        continue;
                    }

                    let node = match node {
                        FormatNode::Line(LineMode::SoftOrSpace) => FormatNode::Space,
                        FormatNode::Interned(interned) => {
                            FormatNode::Interned(clean_interned(interned, interned_cache))
                        }

                        node => node.clone(),
                    };
                    cleaned.push(node);
                }

                Interned::new(cleaned)
            }
            // No change necessary, return existing interned node
            None => interned.clone(),
        };

        interned_cache.insert(interned.clone(), result.clone());
        result
    }
}

impl<Context> Buffer for RemoveSoftLinesBuffer<'_, Context> {
    type Context = Context;

    fn write_node(&mut self, node: FormatNode) {
        if self.state.should_drop(&node) {
            return;
        }

        let node = match node {
            FormatNode::Line(LineMode::SoftOrSpace) => FormatNode::Space,
            FormatNode::Interned(interned) => FormatNode::Interned(self.clean_interned(&interned)),

            node => node,
        };

        self.inner.write_node(node);
    }

    fn nodes(&self) -> &[FormatNode] {
        self.inner.nodes()
    }

    fn state(&self) -> &FormatState<Self::Context> {
        self.inner.state()
    }

    fn state_mut(&mut self) -> &mut FormatState<Self::Context> {
        self.inner.state_mut()
    }

    fn snapshot(&self) -> BufferSnapshot {
        BufferSnapshot::Any(Box::new(RemoveSoftLinebreaksSnapshot {
            inner: self.inner.snapshot(),
            state: self.state,
        }))
    }

    fn restore_snapshot(&mut self, snapshot: BufferSnapshot) {
        let RemoveSoftLinebreaksSnapshot { inner, state } = snapshot.unwrap_any();
        self.inner.restore_snapshot(inner);
        self.state = state;
    }
}

#[derive(Copy, Clone, Debug, Default)]
enum RemoveSoftLineBreaksState {
    #[default]
    Default,
    InIfGroupBreaks {
        conditional_content_level: NonZeroUsize,
    },
}

impl RemoveSoftLineBreaksState {
    fn should_drop(&mut self, node: &FormatNode) -> bool {
        match self {
            Self::Default => match node {
                FormatNode::Line(LineMode::Soft) => true,

                // Entered the start of an `if_group_breaks` or `if_group_fits`
                // For `if_group_breaks`: Remove the start and end tag and all content in between.
                // For `if_group_fits_on_line`: Unwrap the content. This is important because the enclosing group
                // might still *expand* if the content exceeds the line width limit, in which case the
                // `if_group_fits_on_line` content would be removed.
                FormatNode::Tag(FormatTag::StartConditionalContent(condition)) => {
                    if condition.mode.is_expanded() {
                        *self = Self::InIfGroupBreaks {
                            conditional_content_level: NonZeroUsize::new(1).unwrap(),
                        };
                    }
                    true
                }
                FormatNode::Tag(FormatTag::EndConditionalContent) => true,
                _ => false,
            },
            Self::InIfGroupBreaks {
                conditional_content_level,
            } => {
                match node {
                    // A nested `if_group_breaks` or `if_group_fits_on_line`
                    FormatNode::Tag(FormatTag::StartConditionalContent(_)) => {
                        *conditional_content_level = conditional_content_level.saturating_add(1);
                    }
                    // The end of an `if_group_breaks` or `if_group_fits_on_line`.
                    FormatNode::Tag(FormatTag::EndConditionalContent) => {
                        if let Some(level) = NonZeroUsize::new(conditional_content_level.get() - 1)
                        {
                            *conditional_content_level = level;
                        } else {
                            // Found the end tag of the initial `if_group_breaks`. Skip this node but retain
                            // the nodes coming after
                            *self = RemoveSoftLineBreaksState::Default;
                        }
                    }
                    _ => {}
                }

                true
            }
        }
    }
}

struct RemoveSoftLinebreaksSnapshot {
    inner: BufferSnapshot,
    state: RemoveSoftLineBreaksState,
}

pub trait BufferExtensions: Buffer + Sized {
    /// Returns a new buffer that calls the passed inspector for every node that gets written to the output
    #[must_use]
    fn inspect<F>(&mut self, inspector: F) -> Inspect<'_, Self::Context, F>
    where
        F: FnMut(&FormatNode),
    {
        Inspect::new(self, inspector)
    }

    /// Starts a recording that gives you access to all nodes that have been written between the start
    /// and end of the recording
    #[must_use]
    fn start_recording(&mut self) -> Recording<'_, Self> {
        Recording::new(self)
    }

    /// Writes a sequence of nodes into this buffer.
    fn write_nodes<I>(&mut self, nodes: I)
    where
        I: IntoIterator<Item = FormatNode>,
    {
        for node in nodes {
            self.write_node(node);
        }
    }
}

impl<T> BufferExtensions for T where T: Buffer {}

#[derive(Debug)]
pub struct Recording<'buf, Buffer> {
    start: usize,
    buffer: &'buf mut Buffer,
}

impl<'buf, B> Recording<'buf, B>
where
    B: Buffer,
{
    fn new(buffer: &'buf mut B) -> Self {
        Self {
            start: buffer.nodes().len(),
            buffer,
        }
    }

    #[inline]
    pub fn write_format(&mut self, arguments: Arguments<'_, B::Context>) -> FormatResult<()> {
        self.buffer.write_format(arguments)
    }

    #[inline]
    pub fn write_node(&mut self, node: FormatNode) {
        self.buffer.write_node(node);
    }

    pub fn stop(self) -> Recorded<'buf> {
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

#[derive(Debug, Copy, Clone)]
pub struct Recorded<'a>(&'a [FormatNode]);

impl Deref for Recorded<'_> {
    type Target = [FormatNode];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::SimpleFormatContext;
    use crate::prelude::*;
    use crate::{format, format_args, write};

    /// Writes a [`crate::FormatNode`] into this buffer, returning whether the write succeeded.
    #[test]
    fn test_buffer_write_node() {
        let mut state = FormatState::new(SimpleFormatContext::empty_dyst());
        let mut buffer = VecBuffer::new(&mut state);

        buffer.write_node(FormatNode::Token { text: "test" });

        assert_eq!(buffer.into_vec(), vec![FormatNode::Token { text: "test" }]);
    }

    /// Glue for usage of the [`write!`] macro with implementers of this trait.
    #[test]
    fn test_buffer_write_format() {
        let mut state = FormatState::new(SimpleFormatContext::empty_dyst());
        let mut buffer = VecBuffer::new(&mut state);

        buffer
            .write_format(format_args!(token("Hello World")))
            .unwrap();

        assert_eq!(
            buffer.into_vec(),
            vec![FormatNode::Token {
                text: "Hello World"
            }]
        );
    }

    /// A Buffer that removes any soft line breaks or [`if_group_breaks`](crate::builders::if_group_breaks) nodes.
    #[test]
    fn test_remove_soft_lines_buffer() {
        let formatted = format!(
            SimpleFormatContext::empty_dyst(),
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

    /// Starts a recording that gives you access to all nodes that have been written between the start
    /// and end of the recording
    #[test]
    fn test_buffer_start_recording() {
        let formatted = format!(
            SimpleFormatContext::empty_dyst(),
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
}
