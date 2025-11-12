use dyst_ast::{
    Annotation, AnnotationPosition, Argument, Blank, Block, Comment, Decorator, Definition,
    DependencyItem, Doc, EnumField, Expression, Field, MatchCase, MutableNodeTree,
    MutableNodeTreeImpl, Node, NodeId, NodeIdAny, NodeParentIndex, NodeType, Parameter, Pattern,
    PatternField, Tag, TokenSpan, TokenType, UnionField, WhereClause, WithClause,
};
use dyst_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter};
use dyst_fir::print::PrintOptions;
use dyst_source::{
    File, FileSourceMap, ImmutableStringPool, IndentStyle, LanguageCompatibility, LanguageOptions,
    LineEnding, MultiSpan, Span,
};

pub type DystFormatter<'ast, 'buf> = Formatter<'buf, DystFormatContext<'ast>>;

/// Dyst format options.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct DystFormatOptions {
    /// The compatibility mode.
    pub compatibility: Option<LanguageCompatibility>,
    /// The type of line ending to apply to the printed input.  
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
}

impl From<LanguageOptions> for DystFormatOptions {
    #[inline]
    fn from(options: LanguageOptions) -> Self {
        Self {
            compatibility: options.compatibility,
            line_ending: options.formatting.line_ending,
            indent_style: options.formatting.indent_style,
            indent_width: options.formatting.indent_width,
            line_width: options.formatting.line_width,
        }
    }
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
    #[inline]
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    #[inline]
    fn indent_width(&self) -> u8 {
        self.indent_width
    }

    #[inline]
    fn line_width(&self) -> u8 {
        self.line_width
    }

    #[inline]
    fn as_print_options(&self) -> PrintOptions {
        self.as_print_options()
    }
}

/// Dyst format context.
#[derive(Debug, Clone)]
pub struct DystFormatContext<'a> {
    /// The format options.
    pub options: DystFormatOptions,
    /// The file.
    pub file: &'a File,
    /// The main tokens.
    pub tokens: &'a Vec<TokenSpan>,
    /// The side tokens.
    pub side_tokens: &'a Vec<TokenSpan>,
    /// The side span.
    pub side_span: &'a MultiSpan,
    /// The tree.
    pub tree: &'a MutableNodeTree,
    /// The source map.
    pub source_map: &'a FileSourceMap,
    /// The parent index.
    pub parents: NodeParentIndex,
    /// The string pool.
    pub strings: &'a ImmutableStringPool,
}

impl<'a> DystFormatContext<'a> {
    /// Gets the str source backing a Span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &'a str {
        self.file.get_span_str(span).unwrap_or_default()
    }

    /// Gets the str source backing a TokenSpan.
    #[inline]
    pub fn get_token_str(&self, token: TokenSpan) -> &'a str {
        self.file.get_span_str(token.span).unwrap_or_default()
    }

    /// Get a Node from the tree.
    #[inline]
    pub fn get_node<T>(&self, node_id: NodeId<T>) -> &T
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        self.tree.get(node_id)
    }

    /// Get a Node from the tree.
    #[inline]
    pub fn get_node_type<T>(&self, node_id: NodeId<T>) -> NodeType
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        self.tree.get_type(node_id.id)
    }

    /// Get a parent node id and its type from the tree.
    #[inline]
    pub fn get_parent<T>(&self, node_id: NodeId<T>) -> Option<(u32, NodeType)>
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        let parent_id = self.parents.get(node_id);
        if let Some(parent_id) = parent_id {
            let parent_type = self.tree.get_type(parent_id);
            Some((parent_id, parent_type))
        } else {
            None
        }
    }

    /// Get a parent node id and its type from the tree.
    #[inline]
    pub fn get_parent_by_id(&self, node_id: u32) -> Option<(u32, NodeType)> {
        let parent_id = self.parents.get_by_id(node_id);
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
        MutableNodeTree: MutableNodeTreeImpl<T>,
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

    /// Get a Span from the tree.
    #[inline]
    pub fn get_span<T>(&self, node_id: NodeId<T>) -> Span
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        self.tree.get_span(node_id)
    }

    /// Get a Span from the tree.
    #[inline]
    pub fn get_span_by_id(&self, node_id: u32) -> Span {
        self.source_map.get(node_id)
    }

    /// Whether the given span has a newline.
    #[inline]
    pub fn has_newline(&self, span: Span) -> bool {
        let Some(mut token_idx) = self
            .tokens
            .iter()
            .position(|token| token.span.start == span.start)
        else {
            return false;
        };
        while let Some(token) = self.tokens.get(token_idx)
            && token.span.end < span.end
        {
            if token.token.ty == TokenType::Newline {
                return true;
            }
            token_idx += 1;
        }
        false
    }

    /// Whether the given node is at a line start.
    /// (With no other semantic spans between it and the previous newline / start).
    pub fn is_at_line_start(&self, node_id: u32) -> bool {
        // find the token starting the node's span
        let span = self.get_span_by_id(node_id);
        #[cfg(debug_assertions)]
        let _span_str = self.get_span_str(span);
        let Some(mut token_idx) = self
            .tokens
            .iter()
            .position(|token| token.span.start == span.start)
        else {
            return false; // not found
        };

        // can we reach newline or start before hitting something not in side span
        while let Some(prev_token) = self.tokens.get(token_idx) {
            if token_idx == 0 || prev_token.token.ty == TokenType::Newline {
                return true; // reached start
            } else if self.side_span.contains(&prev_token.span) {
                token_idx -= 1; // keep looking
            } else {
                return false; // hit something else
            }
        }

        // reached start
        true
    }

    /// Get annotations for a node. Annotations are sorted by position.
    #[inline]
    pub fn get_annotations<T>(&self, node_id: NodeId<T>) -> Option<Vec<NodeId<Annotation>>>
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        if !self.tree.has_annotations(node_id.id) {
            return None;
        }
        Some(self.tree.get_annotations(node_id.id).to_vec())
    }

    /// Check if a node has an annotation.
    #[inline]
    pub fn has_annotation<T>(&self, node_id: NodeId<T>) -> bool
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        self.get_annotations(node_id).is_some()
    }

    /// Check if a node has a prefix annotation.
    #[inline]
    pub fn has_prefix_annotation<T>(&self, node_id: NodeId<T>) -> bool
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        self.get_annotations(node_id).is_some_and(|annotations| {
            annotations.iter().any(|annotation| {
                let position = self.tree.get::<Annotation>(*annotation).position();
                position == AnnotationPosition::BlockPrefix
                    || position == AnnotationPosition::LinePrefix
            })
        })
    }

    /// Check if a node has a block infix annotation.
    #[inline]
    pub fn has_infix_annotation<T>(&self, node_id: NodeId<T>) -> bool
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        self.get_annotations(node_id).is_some_and(|annotations| {
            annotations.iter().any(|annotation| {
                let position = self.tree.get::<Annotation>(*annotation).position();
                position == AnnotationPosition::BlockInfix
            })
        })
    }

    /// Check if a node has a postfix annotation.
    #[inline]
    pub fn has_postfix_annotation<T>(&self, node_id: NodeId<T>) -> bool
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        self.get_annotations(node_id).is_some_and(|annotations| {
            annotations.iter().any(|annotation| {
                let position = self.tree.get::<Annotation>(*annotation).position();
                position == AnnotationPosition::BlockPostfix
                    || position == AnnotationPosition::LinePostfix
                    || position == AnnotationPosition::LinePostfixBoundary
            })
        })
    }

    /// Check if a node has a blank block prefix annotation.
    #[inline]
    pub fn has_blank_prefix_annotation<T>(&self, node_id: NodeId<T>) -> bool
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
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

    /// Check if a node has a blank prefix annotation in first position.
    #[inline]
    pub fn has_blank_prefix_annotation_in_first_position<T>(&self, node_id: NodeId<T>) -> bool
    where
        T: Node,
        MutableNodeTree: MutableNodeTreeImpl<T>,
    {
        let Some(annotations) = self.get_annotations(node_id) else {
            return false;
        };
        annotations.first().is_some_and(|annotation| {
            matches!(
                self.tree.get::<Annotation>(*annotation),
                Annotation::Blank {
                    position: AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix,
                    ..
                }
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
    fn file(&self) -> &File {
        self.file
    }
}

/// Format Nodes with more information.
pub(crate) trait FormatNode<'a, T: Node>
where
    DystFormatContext<'a>: FormatContext,
{
    /// Format a node.
    fn format_node(&self, node_id: NodeId<T>, f: &mut DystFormatter<'a, '_>) -> FormatResult<()>;
}

/// Implement Format for FormatNode for NodeIds.
impl<'a, T: Node> Format<DystFormatContext<'a>> for NodeId<T>
where
    T: Node + Clone,
    MutableNodeTree: MutableNodeTreeImpl<T>,
    T: FormatNode<'a, T>,
{
    #[inline]
    fn format(&self, f: &mut DystFormatter<'a, '_>) -> FormatResult<()> {
        let context = f.context();
        let node = context.tree.get(*self);
        node.format_node(*self, f)
    }
}

/// Implement Format for FormatNode for NodeIdsAny.
impl<'a> Format<DystFormatContext<'a>> for NodeIdAny {
    #[inline]
    fn format(&self, f: &mut DystFormatter<'a, '_>) -> FormatResult<()> {
        let context = f.context();
        match self.ty {
            NodeType::Expression => {
                let node_id = NodeId::<Expression>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Block => {
                let node_id = NodeId::<Block>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Definition => {
                let node_id = NodeId::<Definition>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Field => {
                let node_id = NodeId::<Field>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::EnumField => {
                let node_id = NodeId::<EnumField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::UnionField => {
                let node_id = NodeId::<UnionField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::WithClause => {
                let node_id = NodeId::<WithClause>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::WhereClause => {
                let node_id = NodeId::<WhereClause>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::DependencyItem => {
                let node_id = NodeId::<DependencyItem>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Parameter => {
                let node_id = NodeId::<Parameter>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Argument => {
                let node_id = NodeId::<Argument>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::MatchCase => {
                let node_id = NodeId::<MatchCase>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Pattern => {
                let node_id = NodeId::<Pattern>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::PatternField => {
                let node_id = NodeId::<PatternField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Annotation => {
                let node_id = NodeId::<Annotation>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Blank => {
                let node_id = NodeId::<Blank>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Doc => {
                let node_id = NodeId::<Doc>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Comment => {
                let node_id = NodeId::<Comment>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Tag => {
                let node_id = NodeId::<Tag>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Decorator => {
                let node_id = NodeId::<Decorator>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
        }
    }
}
