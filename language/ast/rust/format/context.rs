use dyst_language_fir::format::{
    Format, FormatContext, FormatOptions, FormatResult, Formatter, IndentStyle, LineEnding,
};
use dyst_language_fir::print::PrintOptions;
use dyst_language_session::Session;
use dyst_language_source::{Path, PathId, Source, Span, StringId};
use dyst_language_token::TokenSpan;

use crate::{
    Annotation, AnnotationPosition, Node, NodeId, NodeParentIndex, NodeSpanIndex, NodeTree,
    NodeTreeStore, NodeType,
};

pub type DystFormatter<'ast, 'buf> = Formatter<'buf, DystFormatContext<'ast>>;

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
    /// Default options with a given line width.
    pub fn default_with_line_width(line_width: u8) -> Self {
        Self {
            line_width,
            ..Self::default()
        }
    }

    /// Default options with tab indent style.
    pub fn default_tab() -> Self {
        Self {
            indent_style: IndentStyle::Tab,
            ..Self::default()
        }
    }

    /// Default options with tab indent style and a given line width.
    pub fn default_tab_with_line_width(line_width: u8) -> Self {
        Self {
            indent_style: IndentStyle::Tab,
            line_width,
            ..Self::default()
        }
    }

    /// Set the line ending.
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Set the indent style.
    pub fn with_indent_style(mut self, indent_style: IndentStyle) -> Self {
        self.indent_style = indent_style;
        self
    }

    /// Set the indent width.
    pub fn with_indent_width(mut self, indent_width: u8) -> Self {
        self.indent_width = indent_width;
        self
    }

    /// Set the line width.
    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.line_width = line_width;
        self
    }

    /// Convert to print options.
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
    /// The span index.
    pub spans: &'ast NodeSpanIndex,
    /// The parent index.
    pub parents: NodeParentIndex,
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
    #[inline]
    pub fn get_node<T>(&self, node_id: NodeId<T>) -> &T
    where
        T: Node,
        NodeTree: NodeTreeStore<T>,
    {
        self.tree.get(node_id)
    }

    /// Get a parent node id and its type from the tree.
    #[inline]
    pub fn get_parent<T>(&self, node_id: NodeId<T>) -> Option<(u32, NodeType)>
    where
        T: Node,
        NodeTree: NodeTreeStore<T>,
    {
        let parent_id = self.parents.get(node_id);
        if let Some(parent_id) = parent_id {
            let parent_type = self.tree.get_type(parent_id);
            Some((parent_id, parent_type))
        } else {
            None
        }
    }

    /// Get all ancestors of a node.
    #[inline]
    pub fn get_ancestors<T>(&self, node_id: NodeId<T>) -> Vec<(u32, NodeType)>
    where
        T: Node,
        NodeTree: NodeTreeStore<T>,
    {
        self.parents
            .get_ancestors(node_id)
            .into_iter()
            .map(|parent_id| {
                let parent_type = self.tree.get_type(parent_id);
                (parent_id, parent_type)
            })
            .collect()
    }

    /// Get the container type of a node (block, statement, or expression).
    /// Excludes the node_id itself.
    #[inline]
    pub fn get_container_type<T>(&self, node_id: NodeId<T>) -> Option<NodeType>
    where
        T: Node,
        NodeTree: NodeTreeStore<T>,
    {
        let mut current_id = self.parents.get_by_id(node_id.id)?;
        while let Some(parent_id) = self.parents.get_by_id(current_id) {
            let parent_type = self.tree.get_type(parent_id);
            if parent_type == NodeType::Block
                || parent_type == NodeType::Statement
                || parent_type == NodeType::Expression
            {
                // expression direct parent may be statement wrapping it
                //  (in which case it's really a statement, not an expression)
                if parent_type == NodeType::Expression {
                    let parent_parent_id = self.parents.get_by_id(parent_id);
                    if let Some(parent_parent_id) = parent_parent_id {
                        let parent_parent_type = self.tree.get_type(parent_parent_id);
                        if parent_parent_type == NodeType::Statement {
                            return Some(NodeType::Statement);
                        }
                    }
                }

                return Some(parent_type);
            }
            current_id = parent_id;
        }
        None
    }

    /// Get a Span from the tree.
    #[inline]
    pub fn get_span<T>(&self, node_id: NodeId<T>) -> Span
    where
        T: Node,
        NodeTree: NodeTreeStore<T>,
    {
        self.tree.get_span(node_id)
    }

    /// Get annotations for a node.
    #[inline]
    pub fn get_annotations<T>(&self, node_id: NodeId<T>) -> Option<Vec<NodeId<Annotation>>>
    where
        T: Node,
        NodeTree: NodeTreeStore<T>,
    {
        if !self.tree.has_annotations_for(node_id.id) {
            return None;
        }
        Some(self.tree.get_annotations_for(node_id.id).to_vec())
    }

    /// Check if a node has a blank block prefix annotation.
    #[inline]
    pub fn has_blank_prefix_annotation<T>(&self, node_id: NodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeStore<T>,
    {
        self.get_annotations(node_id).is_some_and(|annotations| {
            annotations.iter().any(
                |annotation| match self.tree.get::<Annotation>(*annotation) {
                    Annotation::Blank { position, .. } => {
                        *position == AnnotationPosition::BlockPrefix
                    }
                    _ => false,
                },
            )
        })
    }
}

impl FormatContext for DystFormatContext<'_> {
    type Options = DystFormatOptions;

    #[inline]
    fn options(&self) -> &Self::Options {
        &self.options
    }

    #[inline]
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
    fn format_node(&self, node_id: NodeId<T>, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()>;
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
        let node = context.tree.get(*self);
        node.format_node(*self, f)
    }
}
