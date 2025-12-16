use destack_ast::{
    Annotation, AnnotationPosition, Argument, Blank, Block, Comment, Declaration, Declarator,
    Decorator, DependencyItem, Doc, EnumField, Expression, LocalNodeId, LocalNodeIdAny, MatchCase,
    Member, Node, NodeParentIndex, NodeTree, NodeTreeImpl, NodeType, Parameter, Pattern,
    PatternField, Property, TokenSpan, TokenType, WhereClause,
};
use destack_base::ImmutableStringPool;
use destack_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter};
use destack_fir::print::PrintOptions;
use destack_source::{File, IndentStyle, LanguageType, LineEnding, MultiSpan, NodeSourceMap, Span};

pub type DestackFormatter<'ast, 'buf> = Formatter<'buf, DestackFormatContext<'ast>>;

/// Destack format options.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct DestackFormatOptions {
    /// The source language type.
    pub language_type: LanguageType = LanguageType::Destack,
    /// The type of line ending to apply to the printed input.
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
}

impl DestackFormatOptions {
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

impl FormatOptions for DestackFormatOptions {
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

/// Destack format context.
#[derive(Debug, Clone)]
pub struct DestackFormatContext<'a> {
    /// The format options.
    pub options: DestackFormatOptions,
    /// The file.
    pub file: &'a File,
    /// The main tokens.
    pub tokens: &'a Vec<TokenSpan>,
    /// The side tokens.
    pub side_tokens: &'a Vec<TokenSpan>,
    /// The side span.
    pub side_span: &'a MultiSpan,
    /// The tree.
    pub tree: &'a NodeTree,
    /// The source map.
    pub source_map: &'a NodeSourceMap,
    /// The parent index.
    pub parents: NodeParentIndex,
    /// The string pool.
    pub strings: &'a ImmutableStringPool,
}

impl<'a> DestackFormatContext<'a> {
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
    pub fn get_node<T>(&self, node_id: LocalNodeId<T>) -> &T
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.get(node_id)
    }

    /// Get a Node from the tree.
    #[inline]
    pub fn get_node_type<T>(&self, node_id: LocalNodeId<T>) -> NodeType
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.get_node_type(node_id.id)
    }

    /// Get a parent node id and its type from the tree.
    #[inline]
    pub fn get_parent<T>(&self, node_id: LocalNodeId<T>) -> Option<(u32, NodeType)>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let parent_id = self.parents.get(node_id);
        if let Some(parent_id) = parent_id {
            let parent_type = self.tree.get_node_type(parent_id);
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
            let parent_type = self.tree.get_node_type(parent_id);
            Some((parent_id, parent_type))
        } else {
            None
        }
    }

    /// Get all ancestors of a node.
    #[inline]
    pub fn get_ancestors<T>(&self, node_id: LocalNodeId<T>) -> Vec<(u32, NodeType)>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.parents
            .get_ancestors(node_id)
            .into_iter()
            .map(|parent_id| {
                let parent_type = self.tree.get_node_type(parent_id);
                (parent_id, parent_type)
            })
            .collect()
    }

    /// Get a Span from the tree.
    #[inline]
    pub fn get_span<T>(&self, node_id: LocalNodeId<T>) -> Span
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
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
        let span_str = self.get_span_str(span);
        span_str.contains('\n')
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
    pub fn get_annotations<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Option<Vec<LocalNodeId<Annotation>>>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        if !self.tree.has_annotations(node_id.id) {
            return None;
        }
        Some(self.tree.get_annotations(node_id.id).to_vec())
    }

    /// Check if a node has an annotation.
    #[inline]
    pub fn has_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.get_annotations(node_id).is_some()
    }

    /// Check if a node has a prefix annotation.
    #[inline]
    pub fn has_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
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
    pub fn has_infix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
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
    pub fn has_postfix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
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
    pub fn has_blank_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
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
    pub fn has_blank_prefix_annotation_in_first_position<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
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

impl FormatContext for DestackFormatContext<'_> {
    type Options = DestackFormatOptions;

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
    DestackFormatContext<'a>: FormatContext,
{
    /// Format a node.
    fn format_node(
        &self,
        node_id: LocalNodeId<T>,
        f: &mut DestackFormatter<'a, '_>,
    ) -> FormatResult<()>;
}

/// Implement Format for FormatNode for NodeIds.
impl<'a, T: Node> Format<DestackFormatContext<'a>> for LocalNodeId<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
    T: FormatNode<'a, T>,
{
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'a, '_>) -> FormatResult<()> {
        let context = f.context();
        let node = context.tree.get(*self);
        node.format_node(*self, f)
    }
}

/// Implement Format for FormatNode for NodeIdsAny.
impl<'a> Format<DestackFormatContext<'a>> for LocalNodeIdAny {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'a, '_>) -> FormatResult<()> {
        let context = f.context();
        match self.ty {
            NodeType::Expression => {
                let node_id = LocalNodeId::<Expression>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Block => {
                let node_id = LocalNodeId::<Block>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Declaration => {
                let node_id = LocalNodeId::<Declaration>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Property => {
                let node_id = LocalNodeId::<Property>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Member => {
                let node_id = LocalNodeId::<Member>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::EnumField => {
                let node_id = LocalNodeId::<EnumField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::WhereClause => {
                let node_id = LocalNodeId::<WhereClause>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::DependencyItem => {
                let node_id = LocalNodeId::<DependencyItem>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Parameter => {
                let node_id = LocalNodeId::<Parameter>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Argument => {
                let node_id = LocalNodeId::<Argument>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::MatchCase => {
                let node_id = LocalNodeId::<MatchCase>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Pattern => {
                let node_id = LocalNodeId::<Pattern>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::PatternField => {
                let node_id = LocalNodeId::<PatternField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Declarator => {
                let node_id = LocalNodeId::<Declarator>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Annotation => {
                let node_id = LocalNodeId::<Annotation>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Blank => {
                let node_id = LocalNodeId::<Blank>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Doc => {
                let node_id = LocalNodeId::<Doc>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Comment => {
                let node_id = LocalNodeId::<Comment>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Decorator => {
                let node_id = LocalNodeId::<Decorator>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
        }
    }
}
