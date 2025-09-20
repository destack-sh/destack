use dyst_language_fir::format::{
    Format, FormatContext, FormatOptions, FormatResult, Formatter, IndentStyle, LineEnding,
};
use dyst_language_fir::print::PrintOptions;
use dyst_language_session::Session;
use dyst_language_source::{Path, PathId, Source, Span, StringId};
use dyst_language_token::TokenSpan;

use crate::{Node, NodeId, NodeTree, NodeTreeStore};

pub(crate) type DystFormatter<'ast, 'buf> = Formatter<'buf, DystFormatContext<'ast>>;

/// Dyst format options (mostly for testing).
#[derive(Debug, Default, PartialEq, Clone)]
pub struct DystFormatOptions {
    /// The type of line ending to apply to the printed input.  
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
}

impl DystFormatOptions {
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    pub fn with_indent_style(mut self, indent_style: IndentStyle) -> Self {
        self.indent_style = indent_style;
        self
    }

    pub fn with_indent_width(mut self, indent_width: u8) -> Self {
        self.indent_width = indent_width;
        self
    }

    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.line_width = line_width;
        self
    }

    pub fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
        }
    }
}

impl FormatOptions for DystFormatOptions {
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    fn indent_width(&self) -> u8 {
        self.indent_width
    }

    fn line_width(&self) -> u8 {
        self.line_width
    }

    fn as_print_options(&self) -> PrintOptions {
        self.as_print_options()
    }
}

/// Dyst format context.
#[derive(Debug, Clone)]
pub struct DystFormatContext<'ast> {
    /// The format options.
    pub options: DystFormatOptions,
    /// The source.
    pub source: &'ast Source,
    /// The tree.
    pub tree: &'ast NodeTree,
    /// The session.
    pub session: &'ast Session,
}

impl<'ast> DystFormatContext<'ast> {
    /// Gets the str source backing a Span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &'ast str {
        &self.source.content[span.start as usize..span.end as usize]
    }

    /// Gets the str source backing a TokenSpan.
    #[inline]
    pub fn get_token_str(&self, token: TokenSpan) -> &'ast str {
        &self.source.content[token.span.start as usize..token.span.end as usize]
    }

    /// Get an interned string.
    pub fn get_string(&self, string_id: StringId) -> &str {
        self.session.get_string(string_id)
    }

    /// Get an interned path.
    pub fn get_path(&self, path_id: PathId) -> &Path {
        self.session.get_path(path_id)
    }

    /// Get a Node from the tree.
    pub fn get_node<T>(&self, node_id: NodeId<T>) -> &T
    where
        T: Node,
        NodeTree: NodeTreeStore<T>,
    {
        self.tree.get(node_id)
    }

    /// Get a Span from the tree.
    pub fn get_span<T>(&self, node_id: NodeId<T>) -> Span
    where
        T: Node,
        NodeTree: NodeTreeStore<T>,
    {
        self.tree.get_span(node_id)
    }
}

impl FormatContext for DystFormatContext<'_> {
    type Options = DystFormatOptions;

    fn options(&self) -> &Self::Options {
        &self.options
    }

    fn source(&self) -> &Source {
        self.source
    }
}

/// Format Nodes with more information.
pub(crate) trait FormatNode<'ast, T: Node>
where
    DystFormatContext<'ast>: FormatContext,
{
    /// Format a node.
    fn format_node(
        &self,
        node_id: NodeId<T>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()>;
}

/// Implement Format for FormatNode via context.
impl<'ast, T: Node> Format<DystFormatContext<'ast>> for NodeId<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeStore<T>,
    T: FormatNode<'ast, T>,
{
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let context = f.context();
        let node = context.get_node(*self).clone(); // nocheckin: don't clone
        node.format_node(*self, f)
    }
}
