use destack_source::Span;
use std::cell::Cell;
use std::marker::PhantomData;

use crate::format::{
    Argument, Arguments, BestFittingMode, BestFittingVariants, Buffer, Condition, DedentMode,
    FormatContext, FormatOptions, FormatTag, GroupId, GroupMode, Interned, PrintMode, TextWidth,
    VecBuffer, tag,
};
use crate::prelude::*;
use crate::write;

#[allow(clippy::enum_glob_use)]
use FormatTag::*;

/// A line break that only gets printed if the enclosing `Group` doesn't fit on a single line.
/// It's omitted if the enclosing `Group` fits on a single line.
/// A soft line break is identical to a hard line break when not enclosed inside of a `Group`.
#[inline]
pub const fn soft_line_break() -> Line {
    Line::new(LineMode::Soft)
}

/// A forced line break that are always printed. A hard line break forces any enclosing `Group`
/// to be printed over multiple lines.
#[inline]
pub const fn hard_line_break() -> Line {
    Line::new(LineMode::Hard)
}

/// A forced empty line. An empty line inserts enough line breaks in the output for
/// the previous and next node to be separated by an empty line.
#[inline]
pub const fn empty_line() -> Line {
    Line::new(LineMode::Empty)
}

/// A line break if the enclosing `Group` doesn't fit on a single line, a space otherwise.
#[inline]
pub const fn soft_line_break_or_space() -> Line {
    Line::new(LineMode::SoftOrSpace)
}

/// A line break.
#[derive(Copy, Clone, PartialEq)]
pub struct Line {
    mode: LineMode,
}

impl Line {
    const fn new(mode: LineMode) -> Self {
        Self { mode }
    }
}

impl<Context> Format<Context> for Line {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Line(self.mode));
        Ok(())
    }
}

impl std::fmt::Debug for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Line").field(&self.mode).finish()
    }
}

/// Creates a token that gets written as is to the output. A token must be ASCII only and is not allowed
/// to contain any line breaks or tab characters.
#[inline]
pub fn token(text: &'static str) -> Token {
    debug_assert!(text.is_ascii(), "Token must be ASCII text only");
    debug_assert!(
        !text.contains(['\n', '\r', '\t']),
        "A token should not contain any newlines or tab characters"
    );

    Token { text }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Token {
    text: &'static str,
}

impl<Context> Format<Context> for Token {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Token { text: self.text });
        Ok(())
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "Token({})", self.text)
    }
}

/// Creates a text from a dynamic string.
///
/// This is done by allocating a new string internally.
pub fn text(text: &str) -> Text<'_> {
    debug_assert_no_newlines(text);
    Text { text }
}

#[derive(Eq, PartialEq)]
pub struct Text<'a> {
    text: &'a str,
}

impl<Context> Format<Context> for Text<'_>
where
    Context: FormatContext,
{
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Text {
            text: self.text.to_string().into_boxed_str(),
            width: TextWidth::from_text(self.text, f.options().indent_width()),
        });

        Ok(())
    }
}

impl std::fmt::Debug for Text<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(f, "Text({})", self.text)
    }
}

/// Emits a text as it is written in the source document. Optimized to avoid allocations.
pub const fn source_text_slice(range: Span) -> FileSliceBuilder {
    FileSliceBuilder { span: range }
}

/// Emit one source position marker for subsequent generated output.
pub const fn source_position(source: u32) -> SourcePosition {
    SourcePosition { source }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct SourcePosition {
    source: u32,
}

impl<Context> Format<Context> for SourcePosition {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::SourcePosition {
            source: self.source,
        });

        Ok(())
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct FileSliceBuilder {
    span: Span,
}

impl<Context> Format<Context> for FileSliceBuilder
where
    Context: FormatContext,
{
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        let source = f.context().file();

        let text = source.get_span_str(self.span).unwrap_or_default();
        let text_width = TextWidth::from_text(text, f.context().options().indent_width());

        f.write_node(FormatNode::FileSlice {
            slice: self.span,
            width: text_width,
        });

        Ok(())
    }
}

fn debug_assert_no_newlines(text: &str) {
    debug_assert!(
        !text.contains('\r'),
        "The content '{text}' contains an unsupported '\\r' line terminator character but text must only use line feeds '\\n' as line separator. Use '\\n' instead of '\\r' and '\\r\\n' to insert a line break in strings."
    );
}

/// Pushes some content to the end of the current line.
#[inline]
pub fn line_suffix<Content, Context>(inner: &Content) -> LineSuffix<'_, Context>
where
    Content: Format<Context>,
{
    LineSuffix {
        content: Argument::new(inner),
    }
}

#[derive(Copy, Clone)]
pub struct LineSuffix<'a, Context> {
    content: Argument<'a, Context>,
}

impl<Context> Format<Context> for LineSuffix<'_, Context> {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Tag(StartLineSuffix));
        Arguments::from(&self.content).format(f)?;
        f.write_node(FormatNode::Tag(EndLineSuffix));

        Ok(())
    }
}

impl<Context> std::fmt::Debug for LineSuffix<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LineSuffix").field(&"{{content}}").finish()
    }
}

/// Inserts a boundary for line suffixes that forces the printer to print all pending line suffixes.
/// Helpful if a line suffix shouldn't pass a certain point.
pub const fn line_suffix_boundary() -> LineSuffixBoundary {
    LineSuffixBoundary
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct LineSuffixBoundary;

impl<Context> Format<Context> for LineSuffixBoundary {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::LineSuffixBoundary);

        Ok(())
    }
}

/// Inserts a single space. Allows to separate different tokens.
#[inline]
pub const fn space() -> Space {
    Space
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Space;

impl<Context> Format<Context> for Space {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Space);
        Ok(())
    }
}

/// It adds a level of indentation to the given content
///
/// It doesn't add any line breaks at the edges of the content, meaning that
/// the line breaks have to be manually added.
///
/// This helper should be used only in rare cases, instead you should rely more on
/// [`block_indent`] and [`soft_block_indent`]
#[inline]
pub fn indent<Content, Context>(content: &Content) -> Indent<'_, Context>
where
    Content: Format<Context>,
{
    Indent {
        content: Argument::new(content),
    }
}

#[derive(Copy, Clone)]
pub struct Indent<'a, Context> {
    content: Argument<'a, Context>,
}

impl<Context> Format<Context> for Indent<'_, Context> {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Tag(StartIndent));
        Arguments::from(&self.content).format(f)?;
        f.write_node(FormatNode::Tag(EndIndent));

        Ok(())
    }
}

impl<Context> std::fmt::Debug for Indent<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Indent").field(&"{{content}}").finish()
    }
}

/// It reduces the indentation for the given content depending on the closest [indent] or [align] parent node.
/// - [align] Undoes the spaces added by [align]
/// - [indent] Reduces the indentation level by one
///
/// This is a No-op if the indentation level is zero.
#[inline]
pub fn dedent<Content, Context>(content: &Content) -> Dedent<'_, Context>
where
    Content: Format<Context>,
{
    Dedent {
        content: Argument::new(content),
        mode: DedentMode::Level,
    }
}

#[derive(Copy, Clone)]
pub struct Dedent<'a, Context> {
    content: Argument<'a, Context>,
    mode: DedentMode,
}

impl<Context> Format<Context> for Dedent<'_, Context> {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Tag(StartDedent(self.mode)));
        Arguments::from(&self.content).format(f)?;
        f.write_node(FormatNode::Tag(EndDedent(self.mode)));

        Ok(())
    }
}

impl<Context> std::fmt::Debug for Dedent<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Dedent").field(&"{{content}}").finish()
    }
}

/// It resets the indent document so that the content will be printed at the start of the line.
///
/// ## Prettier
///
/// This resembles the behaviour of Prettier's `align(Number.NEGATIVE_INFINITY, content)` IR node.
#[inline]
pub fn dedent_to_root<Content, Context>(content: &Content) -> Dedent<'_, Context>
where
    Content: Format<Context>,
{
    Dedent {
        content: Argument::new(content),
        mode: DedentMode::Root,
    }
}

/// Aligns its content by indenting the content by `count` spaces.
///
/// [align] is a variant of `[indent]` that indents its content by a specified number of spaces rather than
/// using the configured indent character (tab or a specified number of spaces).
///
/// You should use [align] when you want to indent a content by a specific number of spaces.
/// Using [indent] is preferred in all other situations as it respects the users preferred indent character.
pub fn align<Content, Context>(count: u8, content: &Content) -> Align<'_, Context>
where
    Content: Format<Context>,
{
    Align {
        count,
        content: Argument::new(content),
    }
}

#[derive(Copy, Clone)]
pub struct Align<'a, Context> {
    count: u8,
    content: Argument<'a, Context>,
}

impl<Context> Format<Context> for Align<'_, Context> {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Tag(StartAlign(self.count)));
        Arguments::from(&self.content).format(f)?;
        f.write_node(FormatNode::Tag(EndAlign));

        Ok(())
    }
}

impl<Context> std::fmt::Debug for Align<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Align")
            .field("count", &self.count)
            .field("content", &"{{content}}")
            .finish()
    }
}

/// Inserts a hard line break before and after the content and increases the indentation level for the content by one.
///
/// Block indents indent a block of code, such as in a function body, and therefore insert a line
/// break before and after the content.
///
/// Doesn't create an indentation if the passed in content is [`FormatNode.is_empty`].
#[inline]
pub fn block_indent<Context>(content: &impl Format<Context>) -> BlockIndent<'_, Context> {
    BlockIndent {
        content: Argument::new(content),
        mode: IndentMode::Block,
    }
}

/// Indents the content by inserting a line break before and after the content and increasing
/// the indentation level for the content by one if the enclosing group doesn't fit on a single line.
/// Doesn't change the formatting if the enclosing group fits on a single line.
#[inline]
pub fn soft_block_indent<Context>(content: &impl Format<Context>) -> BlockIndent<'_, Context> {
    BlockIndent {
        content: Argument::new(content),
        mode: IndentMode::Soft,
    }
}

/// If the enclosing `Group` doesn't fit on a single line, inserts a line break and indent.
/// Otherwise, just inserts a space.
///
/// Line indents are used to break a single line of code, and therefore only insert a line
/// break before the content and not after the content.
#[inline]
pub fn soft_line_indent_or_space<Context>(
    content: &impl Format<Context>,
) -> BlockIndent<'_, Context> {
    BlockIndent {
        content: Argument::new(content),
        mode: IndentMode::SoftLineOrSpace,
    }
}

#[derive(Copy, Clone)]
pub struct BlockIndent<'a, Context> {
    content: Argument<'a, Context>,
    mode: IndentMode,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum IndentMode {
    Soft,
    Block,
    SoftSpace,
    SoftLineOrSpace,
}

impl<Context> Format<Context> for BlockIndent<'_, Context> {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        let content = {
            let mut content_buffer = VecBuffer::new(f.state_mut());
            content_buffer.write_format(Arguments::from(&self.content))?;
            content_buffer.into_vec()
        };

        let Some(content) = f.intern_vec(content) else {
            return Ok(());
        };

        f.write_node(FormatNode::Tag(StartIndent));

        match self.mode {
            IndentMode::Soft => write!(f, [soft_line_break()])?,
            IndentMode::Block => write!(f, [hard_line_break()])?,
            IndentMode::SoftLineOrSpace | IndentMode::SoftSpace => {
                write!(f, [soft_line_break_or_space()])?;
            }
        }

        f.write_node(content);

        f.write_node(FormatNode::Tag(EndIndent));

        match self.mode {
            IndentMode::Soft => write!(f, [soft_line_break()]),
            IndentMode::Block => write!(f, [hard_line_break()]),
            IndentMode::SoftSpace => write!(f, [soft_line_break_or_space()]),
            IndentMode::SoftLineOrSpace => Ok(()),
        }
    }
}

impl<Context> std::fmt::Debug for BlockIndent<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.mode {
            IndentMode::Soft => "SoftBlockIndent",
            IndentMode::Block => "HardBlockIndent",
            IndentMode::SoftLineOrSpace => "SoftLineIndentOrSpace",
            IndentMode::SoftSpace => "SoftSpaceBlockIndent",
        };

        f.debug_tuple(name).field(&"{{content}}").finish()
    }
}

/// Adds spaces around the content if its enclosing group fits on a line, otherwise indents the content and separates it by line breaks.
pub fn soft_space_or_block_indent<Context>(
    content: &impl Format<Context>,
) -> BlockIndent<'_, Context> {
    BlockIndent {
        content: Argument::new(content),
        mode: IndentMode::SoftSpace,
    }
}

/// Creates a logical `Group` around the content that should either consistently be printed on a single line
/// or broken across multiple lines.
///
/// The printer will try to print the content of the `Group` on a single line, ignoring all soft line breaks and
/// emitting spaces for soft line breaks or spaces. The printer tracks back if it isn't successful either
/// because it encountered a hard line break, or because printing the `Group` on a single line exceeds
/// the configured line width, and thus it must print all its content on multiple lines,
/// emitting line breaks for all line break kinds.
#[inline]
pub fn group<Context>(content: &impl Format<Context>) -> Group<'_, Context> {
    Group {
        content: Argument::new(content),
        id: None,
        should_expand: false,
    }
}

#[derive(Copy, Clone)]
pub struct Group<'a, Context> {
    content: Argument<'a, Context>,
    id: Option<GroupId>,
    should_expand: bool,
}

impl<Context> Group<'_, Context> {
    #[must_use]
    pub fn with_id(mut self, group_id: Option<GroupId>) -> Self {
        self.id = group_id;
        self
    }

    /// Changes the [`PrintMode`] of the group from [`Flat`](PrintMode::Flat) to [`Expanded`](PrintMode::Expanded).
    /// The result is that any soft-line break gets printed as a regular line break.
    ///
    /// This is useful for content rendered inside of a [`FormatNode::BestFitting`] that prints each variant
    /// in [`PrintMode::Flat`] to change some content to be printed in [`Expanded`](PrintMode::Expanded) regardless.
    /// See the documentation of the [`best_fitting`] macro for an example.
    #[must_use]
    pub fn should_expand(mut self, should_expand: bool) -> Self {
        self.should_expand = should_expand;
        self
    }
}

impl<Context> Format<Context> for Group<'_, Context> {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        let mode = if self.should_expand {
            GroupMode::Expand
        } else {
            GroupMode::Flat
        };

        f.write_node(FormatNode::Tag(StartGroup(
            crate::format::Group::new().with_id(self.id).with_mode(mode),
        )));

        Arguments::from(&self.content).format(f)?;

        f.write_node(FormatNode::Tag(EndGroup));

        Ok(())
    }
}

impl<Context> std::fmt::Debug for Group<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Group")
            .field("id", &self.id)
            .field("should_expand", &self.should_expand)
            .field("content", &"{{content}}")
            .finish()
    }
}

/// Content that may get parenthesized if it exceeds the configured line width but only if the parenthesized
/// layout doesn't exceed the line width too, in which case it falls back to the flat layout.
///
/// The node breaks from left-to-right because it uses the unintended version as *expanded* layout, the same as the above showed best fitting example.
#[inline]
pub fn best_fit_parenthesize<Context>(
    content: &impl Format<Context>,
) -> BestFitParenthesize<'_, Context> {
    BestFitParenthesize {
        content: Argument::new(content),
        group_id: None,
    }
}

#[derive(Copy, Clone)]
pub struct BestFitParenthesize<'a, Context> {
    content: Argument<'a, Context>,
    group_id: Option<GroupId>,
}

impl<Context> BestFitParenthesize<'_, Context> {
    /// Optional ID that can be used in conditional content that supports [`Condition`] to gate content
    /// depending on whether the parentheses are rendered (flat: no parentheses, expanded: parentheses).
    #[must_use]
    pub fn with_group_id(mut self, group_id: Option<GroupId>) -> Self {
        self.group_id = group_id;
        self
    }
}

impl<Context> Format<Context> for BestFitParenthesize<'_, Context> {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Tag(StartBestFitParenthesize {
            id: self.group_id,
        }));

        Arguments::from(&self.content).format(f)?;

        f.write_node(FormatNode::Tag(EndBestFitParenthesize));

        Ok(())
    }
}

impl<Context> std::fmt::Debug for BestFitParenthesize<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BestFitParenthesize")
            .field("group_id", &self.group_id)
            .field("content", &"{{content}}")
            .finish()
    }
}

/// Sets the `condition` for the group. The node will behave as a regular group if `condition` is met,
/// and as *ungrouped* content if the condition is not met.
#[inline]
pub fn conditional_group<Content, Context>(
    content: &Content,
    condition: Condition,
) -> ConditionalGroup<'_, Context>
where
    Content: Format<Context>,
{
    ConditionalGroup {
        content: Argument::new(content),
        condition,
    }
}

#[derive(Clone)]
pub struct ConditionalGroup<'content, Context> {
    content: Argument<'content, Context>,
    condition: Condition,
}

impl<Context> Format<Context> for ConditionalGroup<'_, Context> {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Tag(StartConditionalGroup(
            crate::format::group::ConditionalGroup::new(self.condition),
        )));
        f.write_format(Arguments::from(&self.content))?;
        f.write_node(FormatNode::Tag(EndConditionalGroup));

        Ok(())
    }
}

impl<Context> std::fmt::Debug for ConditionalGroup<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConditionalGroup")
            .field("condition", &self.condition)
            .field("content", &"{{content}}")
            .finish()
    }
}

/// IR node that forces the parent group to print in expanded mode.
///
/// Has no effect if used outside of a group or node that introduce implicit groups (fill node).
///
/// # Prettier
/// Equivalent to Prettier's `break_parent` IR node
pub const fn expand_parent() -> ExpandParent {
    ExpandParent
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ExpandParent;

impl<Context> Format<Context> for ExpandParent {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::ExpandParent);

        Ok(())
    }
}

/// Adds a conditional content that is emitted only if it isn't inside an enclosing `Group` that
/// is printed on a single line. The node allows, for example, to insert a trailing comma after the last
/// array node only if the array doesn't fit on a single line.
///
/// The node has no special meaning if used outside of a `Group`. In that case, the content is always emitted.
///
/// If you're looking for a way to only print something if the `Group` fits on a single line see [`self::if_group_fits_on_line`].
#[inline]
pub fn if_group_breaks<Content, Context>(content: &Content) -> IfGroupBreaks<'_, Context>
where
    Content: Format<Context>,
{
    IfGroupBreaks {
        content: Argument::new(content),
        group_id: None,
        mode: PrintMode::Expanded,
    }
}

/// Adds a conditional content specific for `Group`s that fit on a single line. The content isn't
/// emitted for `Group`s spanning multiple lines.
///
/// See [`if_group_breaks`] if you're looking for a way to print content only for groups spanning multiple lines.
#[inline]
pub fn if_group_fits_on_line<Content, Context>(flat_content: &Content) -> IfGroupBreaks<'_, Context>
where
    Content: Format<Context>,
{
    IfGroupBreaks {
        mode: PrintMode::Flat,
        group_id: None,
        content: Argument::new(flat_content),
    }
}

#[derive(Copy, Clone)]
pub struct IfGroupBreaks<'a, Context> {
    content: Argument<'a, Context>,
    group_id: Option<GroupId>,
    mode: PrintMode,
}

impl<Context> IfGroupBreaks<'_, Context> {
    /// Inserts some content that the printer only prints if the group with the specified `group_id`
    /// is printed in multiline mode. The referred group must appear before this node in the document
    /// but doesn't have to one of its ancestors.
    #[must_use]
    pub fn with_group_id(mut self, group_id: Option<GroupId>) -> Self {
        self.group_id = group_id;
        self
    }
}

impl<Context> Format<Context> for IfGroupBreaks<'_, Context> {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Tag(StartConditionalContent(
            Condition::new(self.mode).with_group_id(self.group_id),
        )));
        Arguments::from(&self.content).format(f)?;
        f.write_node(FormatNode::Tag(EndConditionalContent));

        Ok(())
    }
}

impl<Context> std::fmt::Debug for IfGroupBreaks<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.mode {
            PrintMode::Flat => "IfGroupFitsOnLine",
            PrintMode::Expanded => "IfGroupBreaks",
        };

        f.debug_struct(name)
            .field("group_id", &self.group_id)
            .field("content", &"{{content}}")
            .finish()
    }
}

/// Increases the indent level by one if the group with the specified id breaks.
///
/// This IR has the same semantics as using [`if_group_breaks`] and [`if_group_fits_on_line`] together.
///
/// If you want to indent some content if the enclosing group breaks, use [`indent`].
///
/// Use [`if_group_breaks`] or [`if_group_fits_on_line`] if the fitting and breaking content differs more than just the
/// indentation level.
#[inline]
pub fn indent_if_group_breaks<Content, Context>(
    content: &Content,
    group_id: GroupId,
) -> IndentIfGroupBreaks<'_, Context>
where
    Content: Format<Context>,
{
    IndentIfGroupBreaks {
        group_id,
        content: Argument::new(content),
    }
}

#[derive(Copy, Clone)]
pub struct IndentIfGroupBreaks<'a, Context> {
    content: Argument<'a, Context>,
    group_id: GroupId,
}

impl<Context> Format<Context> for IndentIfGroupBreaks<'_, Context> {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Tag(StartIndentIfGroupBreaks(self.group_id)));
        Arguments::from(&self.content).format(f)?;
        f.write_node(FormatNode::Tag(EndIndentIfGroupBreaks(self.group_id)));

        Ok(())
    }
}

impl<Context> std::fmt::Debug for IndentIfGroupBreaks<'_, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndentIfGroupBreaks")
            .field("group_id", &self.group_id)
            .field("content", &"{{content}}")
            .finish()
    }
}

/// Changes the declaration of *fits* for `content`. It measures the width of all lines and allows
/// the content inside of the [`fits_expanded`] to exceed the configured line width. The content
/// coming before and after [`fits_expanded`] must fit into the configured line width.
///
/// The [`fits_expanded`] acts as a expands boundary similar to best fitting,
/// meaning that a [`hard_line_break`] will not cause the parent group to expand.
///
/// Useful in conjunction with a group with a condition.
pub fn fits_expanded<Content, Context>(content: &Content) -> FitsExpanded<'_, Context>
where
    Content: Format<Context>,
{
    FitsExpanded {
        content: Argument::new(content),
        condition: None,
    }
}

#[derive(Debug, Clone)]
pub struct FitsExpanded<'a, Context> {
    content: Argument<'a, Context>,
    condition: Option<Condition>,
}

impl<Context> FitsExpanded<'_, Context> {
    /// Sets a `condition` to when the content should fit in expanded mode. The content uses the regular fits
    /// declaration if the `condition` is not met.
    #[must_use]
    pub fn with_condition(mut self, condition: Option<Condition>) -> Self {
        self.condition = condition;
        self
    }
}

impl<Context> Format<Context> for FitsExpanded<'_, Context> {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        f.write_node(FormatNode::Tag(StartFitsExpanded(
            tag::FitsExpanded::new().with_condition(self.condition),
        )));
        f.write_format(Arguments::from(&self.content))?;
        f.write_node(FormatNode::Tag(EndFitsExpanded));

        Ok(())
    }
}

/// Utility for formatting some content with an inline lambda function.
#[derive(Copy, Clone)]
pub struct FormatWith<Context, T> {
    formatter: T,
    context: PhantomData<Context>,
}

impl<Context, T> Format<Context> for FormatWith<Context, T>
where
    T: Fn(&mut Formatter<'_, Context>) -> FormatResult<()>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        (self.formatter)(f)
    }
}

impl<Context, T> std::fmt::Debug for FormatWith<Context, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FormatWith").field(&"{{formatter}}").finish()
    }
}

/// Creates an object implementing `Format` that calls the passed closure to perform the formatting.
pub const fn format_with<Context, T>(formatter: T) -> FormatWith<Context, T>
where
    T: Fn(&mut Formatter<'_, Context>) -> FormatResult<()>,
{
    FormatWith {
        formatter,
        context: PhantomData,
    }
}

/// Creates an inline `Format` object that can only be formatted once.
///
/// This can be useful in situation where the borrow checker doesn't allow you to use [`format_with`]
/// because the code formatting the content consumes the value and cloning the value is too expensive.
/// An example of this is if you want to nest a `FormatNode` or non-cloneable `Iterator` inside of a
/// `block_indent` as shown can see in the examples section.
///
/// # Panics
///
/// Panics if the object gets formatted more than once.
pub const fn format_once<T, Context>(formatter: T) -> FormatOnce<T, Context>
where
    T: FnOnce(&mut Formatter<'_, Context>) -> FormatResult<()>,
{
    FormatOnce {
        formatter: Cell::new(Some(formatter)),
        context: PhantomData,
    }
}

pub struct FormatOnce<T, Context> {
    formatter: Cell<Option<T>>,
    context: PhantomData<Context>,
}

impl<T, Context> Format<Context> for FormatOnce<T, Context>
where
    T: FnOnce(&mut Formatter<'_, Context>) -> FormatResult<()>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        let formatter = self.formatter.take().expect("tried to format a `format_once` at least twice. This is not allowed. You may want to use `format_with` or `format.memoized` instead.");

        (formatter)(f)
    }
}

impl<T, Context> std::fmt::Debug for FormatOnce<T, Context> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FormatOnce").field(&"{{formatter}}").finish()
    }
}

/// Builder to join together a sequence of content.
/// See [`Formatter::join`]
#[must_use = "must eventually call `finish()` on Format builders"]
#[derive(Debug)]
pub struct JoinBuilder<'fmt, 'buf, Separator, Context> {
    result: FormatResult<()>,
    fmt: &'fmt mut Formatter<'buf, Context>,
    with: Option<Separator>,
    has_nodes: bool,
}

impl<'fmt, 'buf, Separator, Context> JoinBuilder<'fmt, 'buf, Separator, Context>
where
    Separator: Format<Context>,
{
    /// Creates a new instance that joins the nodes without a separator
    pub(super) fn new(fmt: &'fmt mut Formatter<'buf, Context>) -> Self {
        Self {
            result: Ok(()),
            fmt,
            has_nodes: false,
            with: None,
        }
    }

    /// Creates a new instance that prints the passed separator between every two entries.
    pub(super) fn with_separator(fmt: &'fmt mut Formatter<'buf, Context>, with: Separator) -> Self {
        Self {
            result: Ok(()),
            fmt,
            has_nodes: false,
            with: Some(with),
        }
    }

    /// Adds a new entry to the join output.
    pub fn entry(&mut self, entry: &dyn Format<Context>) -> &mut Self {
        self.result = self.result.and_then(|()| {
            if let Some(with) = &self.with
                && self.has_nodes
            {
                with.format(self.fmt)?;
            }
            self.has_nodes = true;

            entry.format(self.fmt)
        });

        self
    }

    /// Adds the contents of an iterator of entries to the join output.
    pub fn entries<F, I>(&mut self, entries: I) -> &mut Self
    where
        F: Format<Context>,
        I: IntoIterator<Item = F>,
    {
        for entry in entries {
            self.entry(&entry);
        }

        self
    }

    /// Finishes the output and returns any error encountered.
    pub fn finish(&mut self) -> FormatResult<()> {
        self.result
    }
}

/// Builder to fill as many nodes as possible on a single line.
#[must_use = "must eventually call `finish()` on Format builders"]
#[derive(Debug)]
pub struct FillBuilder<'fmt, 'buf, Context> {
    result: FormatResult<()>,
    fmt: &'fmt mut Formatter<'buf, Context>,
    empty: bool,
}

impl<'a, 'buf, Context> FillBuilder<'a, 'buf, Context> {
    pub(crate) fn new(fmt: &'a mut Formatter<'buf, Context>) -> Self {
        fmt.write_node(FormatNode::Tag(StartFill));

        Self {
            result: Ok(()),
            fmt,
            empty: true,
        }
    }

    /// Adds an iterator of entries to the fill output. Uses the passed `separator` to separate any two items.
    pub fn entries<F, I>(&mut self, separator: &dyn Format<Context>, entries: I) -> &mut Self
    where
        F: Format<Context>,
        I: IntoIterator<Item = F>,
    {
        for entry in entries {
            self.entry(separator, &entry);
        }

        self
    }

    /// Adds a new entry to the fill output. The `separator` isn't written if this is the first node in the list.
    pub fn entry(
        &mut self,
        separator: &dyn Format<Context>,
        entry: &dyn Format<Context>,
    ) -> &mut Self {
        self.result = self.result.and_then(|()| {
            if self.empty {
                self.empty = false;
            } else {
                self.fmt.write_node(FormatNode::Tag(StartEntry));
                separator.format(self.fmt)?;
                self.fmt.write_node(FormatNode::Tag(EndEntry));
            }

            self.fmt.write_node(FormatNode::Tag(StartEntry));
            entry.format(self.fmt)?;
            self.fmt.write_node(FormatNode::Tag(EndEntry));
            Ok(())
        });

        self
    }

    /// Finishes the output and returns any error encountered
    pub fn finish(&mut self) -> FormatResult<()> {
        if self.result.is_ok() {
            self.fmt.write_node(FormatNode::Tag(EndFill));
        }
        self.result
    }
}

/// The first variant is the most flat, and the last is the most expanded variant.
#[derive(Copy, Clone, Debug)]
pub struct BestFitting<'a, Context> {
    variants: Arguments<'a, Context>,
    mode: BestFittingMode,
}

impl<'a, Context> BestFitting<'a, Context> {
    /// Creates a new best fitting IR with the given variants.
    ///
    /// Callers are required to ensure that the number of variants given
    /// is at least 2.
    ///
    /// You're looking for a way to create a `BestFitting` object, use the `best_fitting![least_expanded, most_expanded]` macro.
    ///
    /// # Panics
    ///
    /// When the slice contains less than two variants.
    pub const fn from_arguments_unchecked(variants: Arguments<'a, Context>) -> Self {
        assert!(
            variants.0.len() >= 2,
            "Requires at least the least expanded and most expanded variants"
        );

        Self {
            variants,
            mode: BestFittingMode::FirstLine,
        }
    }

    /// Changes the mode used by this best fitting node to determine whether a variant fits.
    #[must_use]
    pub fn with_mode(mut self, mode: BestFittingMode) -> Self {
        self.mode = mode;
        self
    }
}

impl<Context> Format<Context> for BestFitting<'_, Context> {
    fn format(&self, f: &mut Formatter<'_, Context>) -> FormatResult<()> {
        let variants = self.variants.items();

        let mut variant_nodes = Vec::with_capacity(variants.len());

        for variant in variants {
            let mut buffer = VecBuffer::with_capacity(8, f.state_mut());

            buffer.write_node(FormatNode::Tag(StartEntry));
            buffer.write_format(Arguments::from(variant))?;
            buffer.write_node(FormatNode::Tag(EndEntry));

            variant_nodes.push(Interned::new(buffer.into_vec()));
        }

        // OK because the constructor guarantees that there are always at
        // least two variants.
        let variants = BestFittingVariants::from_vec_unchecked(variant_nodes);
        let node = FormatNode::BestFitting {
            variants,
            mode: self.mode,
        };

        f.write_node(node);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use destack_source::FileType;

    use crate::format::{IndentStyle, SimpleFormatContext, SimpleFormatOptions};
    use crate::prelude::*;
    use crate::{best_fitting, format, format_args, write};

    /// Soft line breaks are omitted if the enclosing Group fits on a single line
    #[test]
    fn test_soft_line_break_fits_on_single_line() {
        let nodes = format!(
            SimpleFormatContext::empty_destack(),
            [group(&format_args![
                token("a,"),
                soft_line_break(),
                token("b")
            ])]
        )
        .unwrap();

        assert_eq!("a,b", nodes.print().unwrap().as_str());
    }

    /// Soft line breaks are emitted if the enclosing Group doesn't fit on a single line
    #[test]
    fn test_soft_line_break_breaks_when_group_doesnt_fit() {
        let context = SimpleFormatContext::new(
            SimpleFormatOptions {
                line_width: 10,
                ..SimpleFormatOptions::default()
            },
            File::empty_text(FileType::Destack),
        );

        let nodes = format!(
            context,
            [group(&format_args![
                token("a long word,"),
                soft_line_break(),
                token("so that the group doesn't fit on a single line"),
            ])]
        )
        .unwrap();

        assert_eq!(
            "a long word,\nso that the group doesn't fit on a single line",
            nodes.print().unwrap().as_str()
        );
    }

    /// Hard line breaks are always printed, even if the enclosing Group fits on a single line
    #[test]
    fn test_hard_line_break_always_breaks() {
        let nodes = format!(
            SimpleFormatContext::empty_destack(),
            [group(&format_args![
                token("a,"),
                hard_line_break(),
                token("b"),
                hard_line_break()
            ])]
        )
        .unwrap();

        assert_eq!("a,\nb\n", nodes.print().unwrap().as_str());
    }

    /// Empty line inserts enough line breaks for nodes to be separated by an empty line
    #[test]
    fn test_empty_line_separates_nodes() {
        let nodes = format!(
            SimpleFormatContext::empty_destack(),
            [group(&format_args![
                token("a,"),
                empty_line(),
                token("b"),
                empty_line()
            ])]
        )
        .unwrap();

        assert_eq!("a,\n\nb\n\n", nodes.print().unwrap().as_str());
    }

    /// Soft line break or space emits spaces when group fits on single line
    #[test]
    fn test_soft_line_break_or_space_fits_on_line() {
        let nodes = format!(
            SimpleFormatContext::empty_destack(),
            [group(&format_args![
                token("a,"),
                soft_line_break_or_space(),
                token("b"),
            ])]
        )
        .unwrap();

        assert_eq!("a, b", nodes.print().unwrap().as_str());
    }

    /// Soft line break or space breaks lines when group doesn't fit
    #[test]
    fn test_soft_line_break_or_space_breaks_when_group_doesnt_fit() {
        let context = SimpleFormatContext::new(
            SimpleFormatOptions {
                line_width: 10,
                ..SimpleFormatOptions::default()
            },
            File::empty_text(FileType::Destack),
        );

        let nodes = format!(
            context,
            [group(&format_args![
                token("a long word,"),
                soft_line_break_or_space(),
                token("so that the group doesn't fit on a single line"),
            ])]
        )
        .unwrap();

        assert_eq!(
            "a long word,\nso that the group doesn't fit on a single line",
            nodes.print().unwrap().as_str()
        );
    }

    /// Soft line break or space should break inside one parent group's expanded indent shell
    #[test]
    fn test_soft_line_break_or_space_breaks_inside_expanded_parent_group() {
        let context = SimpleFormatContext::new(
            SimpleFormatOptions {
                line_width: 30,
                ..SimpleFormatOptions::default()
            },
            File::empty_text(FileType::Destack),
        );

        let nodes = format!(
            context,
            [group(&format_args![
                token("const longVariableName"),
                token(" ="),
                group(&indent(&format_args![
                    soft_line_break_or_space(),
                    token("("),
                    group(&token("alpha")),
                    token(" ="),
                    soft_line_break_or_space(),
                    group(&token("beta")),
                    token(" ="),
                    indent(&format_args![
                        soft_line_break_or_space(),
                        token("computeValue()")
                    ]),
                    token(")"),
                ])),
            ])]
        )
        .unwrap();

        assert_eq!(
            "const longVariableName =\n    (alpha =\n    beta =\n        computeValue())",
            nodes.print().unwrap().as_str()
        );
    }

    /// Token writes content as-is to output
    #[test]
    fn test_token_writes_content() {
        let nodes = format!(SimpleFormatContext::empty_destack(), [token("Hello World")]).unwrap();

        assert_eq!("Hello World", nodes.print().unwrap().as_str());
    }

    /// Token properly handles escaped string literals
    #[test]
    fn test_token_handles_escaped_strings() {
        let nodes = format!(
            SimpleFormatContext::empty_destack(),
            [token("\"Hello\\tWorld\"")]
        )
        .unwrap();

        assert_eq!(r#""Hello\tWorld""#, nodes.print().unwrap().as_str());
    }

    /// Line suffix pushes content to end of current line
    #[test]
    fn test_line_suffix_pushes_to_end() {
        let nodes = format!(
            SimpleFormatContext::empty_destack(),
            [token("a"), line_suffix(&token("c")), token("b")]
        )
        .unwrap();

        assert_eq!("abc", nodes.print().unwrap().as_str());
    }

    /// Soft line indents should still keep line suffix comments inline when the group fits.
    #[test]
    fn test_soft_line_indent_or_space_keeps_line_suffix_inline() {
        let nodes = format!(
            SimpleFormatContext::empty_destack(),
            [group(&format_args![
                token("if (done)"),
                soft_line_indent_or_space(&format_args![
                    token("break;"),
                    line_suffix(&format_args![space(), token("// break-tail")]),
                ]),
            ])]
        )
        .unwrap();

        assert_eq!(
            "if (done) break; // break-tail",
            nodes.print().unwrap().as_str()
        );
    }

    /// Nested control-head groups should still keep inline line suffix comments when they fit.
    #[test]
    fn test_grouped_control_head_keeps_inline_line_suffix_comments() {
        let nodes = format!(
            SimpleFormatContext::empty_destack(),
            [group(&format_args![
                token("if"),
                space(),
                token("("),
                group(&soft_block_indent(&token("done"))),
                token(")"),
                soft_line_indent_or_space(&format_args![
                    token("break;"),
                    line_suffix(&format_args![space(), token("// break-tail")]),
                ]),
            ])]
        )
        .unwrap();

        assert_eq!(
            "if (done) break; // break-tail",
            nodes.print().unwrap().as_str()
        );
    }

    /// Line suffix boundary forces printing of pending line suffixes
    #[test]
    fn test_line_suffix_boundary_forces_printing() {
        let nodes = format!(
            SimpleFormatContext::empty_destack(),
            [
                token("a"),
                line_suffix(&token("c")),
                token("b"),
                line_suffix_boundary(),
                token("d")
            ]
        )
        .unwrap();

        assert_eq!("abc\nd", nodes.print().unwrap().as_str());
    }

    /// Closing content after one nested trailing line comment should still print.
    #[test]
    fn test_nested_line_suffix_before_container_close_keeps_trailing_tokens() {
        let nodes = format!(
            SimpleFormatContext::empty_destack(),
            [
                token("switch (state)"),
                space(),
                token("{"),
                group(&format_args![
                    hard_line_break(),
                    block_indent(&format_args![
                        token("default:"),
                        hard_line_break(),
                        block_indent(&format_args![
                            token("stop();"),
                            line_suffix(&format_args![space(), token("// default-tail")]),
                        ]),
                    ]),
                    line_suffix_boundary(),
                    hard_line_break(),
                ]),
                token("}"),
            ]
        )
        .unwrap();

        assert_eq!(
            "switch (state) {\n    default:\n        stop(); // default-tail\n}",
            nodes.print().unwrap().as_str()
        );
    }

    /// Space inserts single space between tokens
    #[test]
    fn test_space_separates_tokens() {
        let nodes = format!(
            SimpleFormatContext::empty_destack(),
            [token("a"), space(), token("b")]
        )
        .unwrap();

        assert_eq!("a b", nodes.print().unwrap().as_str());
    }

    /// Indent adds level of indentation to content
    #[test]
    fn test_indent_adds_indentation_level() {
        let block = format!(
            SimpleFormatContext::empty_destack(),
            [
                token("switch {"),
                block_indent(&format_args![
                    token("default:"),
                    indent(&format_args![hard_line_break(), token("break;"),])
                ]),
                token("}"),
            ]
        )
        .unwrap();

        assert_eq!(
            "switch {\n    default:\n        break;\n}",
            block.print().unwrap().as_str()
        );
    }

    /// Indent should apply to a leading hard line.
    #[test]
    fn test_indent_applies_to_leading_hard_line() {
        let content = format_with(|f| {
            write!(
                f,
                [
                    hard_line_break(),
                    text("// comment"),
                    hard_line_break(),
                    token(".run();"),
                ]
            )
        });
        let block = format!(
            SimpleFormatContext::empty_destack(),
            [group(&format_args![token("yield task"), indent(&content)])]
        )
        .unwrap();

        assert_eq!(
            "yield task\n    // comment\n    .run();",
            block.print().unwrap().as_str()
        );
    }

    /// Block indent adds indentation and line breaks around content
    #[test]
    fn test_block_indent_adds_indentation_and_breaks() {
        let formatted = format!(
            SimpleFormatContext::empty_destack(),
            [
                token("switch {"),
                block_indent(&format_args![
                    token("default:"),
                    block_indent(&token("break;"))
                ]),
                token("}"),
            ]
        )
        .unwrap();

        assert_eq!(
            "switch {\n    default:\n        break;\n}",
            formatted.print().unwrap().as_str()
        );
    }

    /// Soft block indent adds indentation and soft line breaks
    #[test]
    fn test_soft_block_indent_adds_soft_breaks() {
        let formatted = format!(
            SimpleFormatContext::empty_destack(),
            [
                token("switch {"),
                soft_block_indent(&format_args![
                    token("default:"),
                    soft_block_indent(&token("break;"))
                ]),
                token("}"),
            ]
        )
        .unwrap();

        assert_eq!(
            "switch {\n    default:\n        break;\n}",
            formatted.print().unwrap().as_str()
        );
    }

    /// If group breaks shows content only when group breaks
    #[test]
    fn test_if_group_breaks_shows_when_breaking() {
        let context = SimpleFormatContext::new(
            SimpleFormatOptions {
                indent_style: IndentStyle::Tab,
                line_width: 10,
                ..SimpleFormatOptions::default()
            },
            File::empty_text(FileType::Destack),
        );

        let formatted = format!(
            context,
            [group(&format_args![
                token("["),
                soft_block_indent(&format_args![
                    token("'A somewhat longer string to force a line break',"),
                    soft_line_break(),
                    token("2,"),
                    soft_line_break(),
                    token("3"),
                    if_group_breaks(&token(","))
                ]),
                token("]")
            ])]
        )
        .unwrap();

        assert_eq!(
            "[\n\t'A somewhat longer string to force a line break',\n\t2,\n\t3,\n]",
            formatted.print().unwrap().as_str()
        );
    }

    /// If group fits on line shows content only when group fits
    #[test]
    fn test_if_group_fits_on_line_shows_when_fitting() {
        let formatted = format!(
            SimpleFormatContext::empty_destack(),
            [group(&format_args![
                token("["),
                soft_block_indent(&format_args![
                    token("1,"),
                    soft_line_break_or_space(),
                    token("2,"),
                    soft_line_break_or_space(),
                    token("3"),
                    if_group_fits_on_line(&space())
                ]),
                token("]")
            ])]
        )
        .unwrap();

        assert_eq!("[1, 2, 3 ]", formatted.print().unwrap().as_str());
    }

    /// Indent if group breaks adds indentation when specific group breaks
    #[test]
    fn test_indent_if_group_breaks() {
        let content = format_with(|f| {
            let group_id = f.group_id("header");
            write!(
                f,
                [
                    group(&format_args![
                        token("const fn = ("),
                        soft_block_indent(&format_args![
                            token("param1,"),
                            soft_line_break(),
                            token("param2")
                        ]),
                        token(")")
                    ])
                    .with_id(Some(group_id)),
                    space(),
                    token("=>"),
                    indent_if_group_breaks(
                        &format_args![hard_line_break(), token("body")],
                        group_id
                    )
                ]
            )
        });

        let context = SimpleFormatContext::new(
            SimpleFormatOptions {
                indent_style: IndentStyle::Tab,
                line_width: 20,
                ..SimpleFormatOptions::default()
            },
            File::empty_text(FileType::Destack),
        );

        let formatted = format!(context, [content]).unwrap();

        assert_eq!(
            "const fn = (\n\tparam1,\n\tparam2\n) =>\n\tbody",
            formatted.print().unwrap().as_str()
        );
    }

    /// Fits expanded allows content to exceed line width
    #[test]
    fn test_fits_expanded_allows_exceeding_line_width() {
        let content = format_with(|f| {
            f.group_id("header");
            write!(
                f,
                [group(&format_args![
                    token("a"),
                    soft_line_break_or_space(),
                    token("+"),
                    space(),
                    fits_expanded(&group(&format_args![
                        token("["),
                        soft_block_indent(&format_args![
                            token("a,"),
                            space(),
                            token("# comment"),
                            expand_parent(),
                            soft_line_break_or_space(),
                            token(
                                "'A very long string that exceeds the configured line width of 80 characters but the enclosing binary expression still fits.'"
                            )
                        ]),
                        token("]")
                    ]))
                ]),]
            )
        });

        let formatted = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 21,
                    ..SimpleFormatOptions::default()
                },
                File::empty_text(FileType::Destack),
            ),
            [content]
        )
        .unwrap();

        assert_eq!(
            "a + [\n\ta, # comment\n\t'A very long string that exceeds the configured line width of 80 characters but the enclosing binary expression still fits.'\n]",
            formatted.print().unwrap().as_str()
        );
    }

    /// Text creates dynamic text content
    #[test]
    fn test_text_creates_dynamic_content() {
        let dynamic_text = "Hello World";
        let nodes = format!(SimpleFormatContext::empty_destack(), [text(dynamic_text)]).unwrap();

        assert_eq!("Hello World", nodes.print().unwrap().as_str());
    }

    /// Best fitting chooses first variant that fits
    #[test]
    fn test_best_fitting_chooses_first_variant_that_fits() {
        let document = format_with(|f| {
            write!(
                f,
                [best_fitting![
                    format_args![token("["), token("1, 2, 3"), token("]")],
                    format_args![
                        token("["),
                        soft_block_indent(&format_args![
                            token("1,"),
                            soft_line_break(),
                            token("2,"),
                            soft_line_break(),
                            token("3")
                        ]),
                        token("]")
                    ]
                ]]
            )
        });

        // First variant fits
        let formatted = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 20,
                    ..SimpleFormatOptions::default()
                },
                File::empty_text(FileType::Destack)
            ),
            [document.clone()]
        )
        .unwrap();

        assert_eq!("[1, 2, 3]", formatted.print().unwrap().as_str());

        // First variant doesn't fit, use second
        let formatted = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 8,
                    ..SimpleFormatOptions::default()
                },
                File::empty_text(FileType::Destack)
            ),
            [document]
        )
        .unwrap();

        assert_eq!("[\n\t1,\n\t2,\n\t3\n]", formatted.print().unwrap().as_str());
    }

    /// Best fit parenthesize keeps content flat when it fits
    #[test]
    fn test_best_fit_parenthesize_content_fits() {
        let formatted = format!(
            SimpleFormatContext::empty_destack(),
            [format_with(|f| {
                write!(
                    f,
                    [
                        token("aLongerVariableName = "),
                        best_fit_parenthesize(&token("'a string'"))
                    ]
                )
            })]
        )
        .unwrap();

        assert_eq!(
            "aLongerVariableName = 'a string'",
            formatted.print().unwrap().as_str()
        );
    }

    /// Best fit parenthesize adds parentheses when content exceeds line width but fits parenthesized
    #[test]
    fn test_best_fit_parenthesize_content_fits_parenthesized() {
        let formatted = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions::default().with_line_width(80),
                File::empty_text(FileType::Destack)
            ),
            [format_with(|f| {
                write!(
                    f,
                    [
                        token("aLongerVariableName = "),
                        best_fit_parenthesize(&token(
                            "'a string that exceeds configured line width but fits parenthesized'"
                        ))
                    ]
                )
            })]
        )
        .unwrap();

        assert_eq!(
            "aLongerVariableName = (\n    'a string that exceeds configured line width but fits parenthesized'\n)",
            formatted.print().unwrap().as_str()
        );
    }

    /// Best fit parenthesize keeps content flat when even parenthesizing doesn't help
    #[test]
    fn test_best_fit_parenthesize_content_exceeds_even_parenthesized() {
        let formatted = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions::default().with_line_width(80),
                File::empty_text(FileType::Destack)
            ),
            [format_with(|f| {
                write!(
                    f,
                    [
                        token("aLongerVariableName = "),
                        best_fit_parenthesize(&token(
                            "'a string that exceeds the configured line width and even parenthesizing doesn't make it fit'"
                        ))
                    ]
                )
            })]
        )
        .unwrap();

        assert_eq!(
            "aLongerVariableName = 'a string that exceeds the configured line width and even parenthesizing doesn't make it fit'",
            formatted.print().unwrap().as_str()
        );
    }
}
