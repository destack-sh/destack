use super::options::DestackFormatOptions;
use super::source::SourceText;
use crate::format::file::comment_text_has_ignore_directive_marker;
use rustc_hash::FxHashMap;
use std::cell::OnceCell;

pub use destack_ast::Decorator;
use destack_ast::{
    Argument, AssignPattern, AssignPatternField, Block, Declaration, Declarator, DependencyItem,
    EnumField, Expression, GenericArgument, GenericParameter, LocalNodeId, LocalNodeIdAny,
    MatchCase, Member, Node, NodeParentIndex, NodeTree, NodeTreeImpl, NodeType, Parameter, Pattern,
    PatternField, Property, TokenSpan, TokenType, TupleElement, TypeExpression, TypeMember,
    WhereClause,
};
use destack_core::ImmutableStringPool;
use destack_fir::format::{
    Buffer, Format, FormatContext, FormatNode as FirNode, FormatNodes, FormatResult, Formatter,
};
use destack_source::{File, MultiSpan, Span};

use super::comment::Comments;

/// The formatter implementation specialized for the Destack context.
pub type DestackFormatter<'ast, 'buf> = Formatter<'buf, DestackFormatContext<'ast>>;

/// Destack format context.
#[derive(Debug, Clone)]
pub struct DestackFormatContext<'a> {
    /// The format options.
    pub options: DestackFormatOptions,
    /// The file.
    pub file: &'a File,
    /// The main tokens.
    pub tokens: &'a [TokenSpan],
    /// The side tokens.
    pub side_tokens: &'a [TokenSpan],
    /// The side span.
    pub side_span: &'a MultiSpan,
    /// The tree.
    pub tree: &'a NodeTree,
    /// The parent index.
    pub parents: NodeParentIndex,
    /// The string pool.
    pub strings: &'a ImmutableStringPool,
    /// Cached newline byte offsets in file text.
    pub newline_offsets: OnceCell<Vec<u32>>,
    /// Cached sorted comment tokens for ignore-range scans.
    pub comment_tokens_sorted: OnceCell<Vec<TokenSpan>>,
    /// Cached sorted tokens across main and side streams.
    pub all_tokens_sorted: OnceCell<Vec<TokenSpan>>,
    /// Cached formatted elements keyed by source span.
    pub cached_elements: FxHashMap<Span, FirNode>,
    /// The start position of the following sibling for the node currently being formatted.
    pub current_following_span_start: u32,
    /// Whether file text contains formatter ignore directive markers.
    pub has_ignore_directive_markers: bool,
    /// The comment cursor for this formatting pass.
    pub comments: Comments<'a>,
}

impl<'a> DestackFormatContext<'a> {
    /// Construct a formatting context from parse artifacts.
    pub fn new(
        options: DestackFormatOptions,
        file: &'a File,
        tree: &'a NodeTree,
        tokens: &'a [TokenSpan],
        side_tokens: &'a [TokenSpan],
        side_span: &'a MultiSpan,
        strings: &'a ImmutableStringPool,
        parents: NodeParentIndex,
    ) -> Self {
        // ignore directives
        let has_ignore_directive_markers = tokens.iter().chain(side_tokens.iter()).any(|token| {
            matches!(
                token.token.ty,
                TokenType::LineComment
                    | TokenType::DocLineComment
                    | TokenType::BlockComment
                    | TokenType::DocBlockComment
            ) && comment_text_has_ignore_directive_marker(file.span_str(token.span))
        });

        Self {
            options,
            file,
            tokens,
            side_tokens,
            side_span,
            tree,
            parents,
            strings,
            newline_offsets: OnceCell::new(),
            comment_tokens_sorted: OnceCell::new(),
            all_tokens_sorted: OnceCell::new(),
            cached_elements: FxHashMap::default(),
            current_following_span_start: 0,
            has_ignore_directive_markers,
            comments: Comments::new(SourceText::new(file.text()), tree.comments()),
        }
    }

    /// Borrow the comment cursor.
    pub fn comments(&self) -> &Comments<'a> {
        &self.comments
    }

    /// Borrow the comment cursor mutably.
    pub fn comments_mut(&mut self) -> &mut Comments<'a> {
        &mut self.comments
    }

    /// Return one cached formatted element for one source span.
    pub fn get_cached_element(&self, span: &Span) -> Option<FirNode> {
        self.cached_elements.get(span).cloned()
    }

    /// Cache one formatted element for one source span.
    pub fn cache_element(&mut self, span: &Span, node: FirNode) {
        self.cached_elements.insert(*span, node);
    }

    /// Return the current following sibling start used for trailing comment ownership.
    pub fn following_span_start(&self) -> u32 {
        self.current_following_span_start
    }

    /// Replace the current following sibling start and return the previous value.
    pub fn replace_following_span_start(&mut self, following_span_start: u32) -> u32 {
        std::mem::replace(&mut self.current_following_span_start, following_span_start)
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

/// The memoized formatted content for one formatter payload.
pub(crate) struct MemoizedFormat<T> {
    /// The content to format.
    content: T,
    /// The cached formatted node.
    cached: OnceCell<Option<FirNode>>,
}

impl<T> MemoizedFormat<T> {
    /// Construct one memoized formatter payload.
    pub(crate) fn new(content: T) -> Self {
        Self {
            content,
            cached: OnceCell::new(),
        }
    }

    /// Inspect the formatted content without formatting it twice.
    pub(crate) fn inspect<'ast>(
        &self,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<Option<FirNode>>
    where
        T: Format<DestackFormatContext<'ast>>,
    {
        // cached
        if let Some(cached) = self.cached.get() {
            return Ok(cached.clone());
        }

        // fresh
        let interned = f.intern(&self.content)?;
        let _ = self.cached.set(interned.clone());

        Ok(interned)
    }
}

impl<'ast, T> Format<DestackFormatContext<'ast>> for MemoizedFormat<T>
where
    T: Format<DestackFormatContext<'ast>>,
{
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        // cached content
        let Some(cached) = self.inspect(f)? else {
            return Ok(());
        };

        f.write_node(cached);
        Ok(())
    }
}

/// Memoize one formatting payload for reuse and inspection.
pub(crate) trait MemoizeFormatExt<'ast>: Format<DestackFormatContext<'ast>> + Sized {
    /// Return one memoized wrapper around this payload.
    fn memoized(self) -> MemoizedFormat<Self> {
        MemoizedFormat::new(self)
    }
}

impl<'ast, T> MemoizeFormatExt<'ast> for T where T: Format<DestackFormatContext<'ast>> + Sized {}

/// Speculative formatting helpers for one Destack formatter.
pub(crate) trait DestackFormatterSpeculationExt<'ast> {
    /// Return whether formatting `content` after one source start would break.
    fn speculate_will_break_after(
        &mut self,
        start: u32,
        content: &dyn Format<DestackFormatContext<'ast>>,
    ) -> FormatResult<bool>;
}

impl<'ast> DestackFormatterSpeculationExt<'ast> for DestackFormatter<'ast, '_> {
    fn speculate_will_break_after(
        &mut self,
        start: u32,
        content: &dyn Format<DestackFormatContext<'ast>>,
    ) -> FormatResult<bool> {
        // speculation snapshot
        let snapshot = self.context().comments().snapshot();

        // speculative pass
        self.context_mut()
            .comments_mut()
            .skip_comments_before(start);

        let will_break = self
            .intern(content)?
            .is_some_and(|content| content.will_break());

        // restore
        self.context_mut().comments_mut().restore(snapshot);

        Ok(will_break)
    }
}

/// Format one typed AST node with full context.
pub(crate) trait FormatNode<'a, T: Node>
where
    DestackFormatContext<'a>: FormatContext,
{
    /// Format one AST node id.
    fn format_node(
        &self,
        node_id: LocalNodeId<T>,
        f: &mut DestackFormatter<'a, '_>,
    ) -> FormatResult<()>;
}

/// The formatted content for one node without trailing comments.
pub(crate) struct FormatNodeWithoutTrailingComments<T: Node>(pub LocalNodeId<T>);

/// Format a node.
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

/// Format a node without trailing comments.
impl<'a, T: Node> Format<DestackFormatContext<'a>> for FormatNodeWithoutTrailingComments<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
    T: FormatNode<'a, T>,
{
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'a, '_>) -> FormatResult<()> {
        let node_end = f.context().span(self.0).end;
        let previous_limit = f
            .context_mut()
            .comments_mut()
            .limit_comments_up_to(node_end);
        let result = self.0.format(f);
        f.context_mut()
            .comments_mut()
            .restore_view_limit(previous_limit);
        result
    }
}

/// Format a dynamically typed node.
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
            NodeType::TypeExpression => {
                let node_id = LocalNodeId::<TypeExpression>::new(self.id);
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
            NodeType::TypeMember => {
                let node_id = LocalNodeId::<TypeMember>::new(self.id);
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
            NodeType::GenericParameter => {
                let node_id = LocalNodeId::<GenericParameter>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Parameter => {
                let node_id = LocalNodeId::<Parameter>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::GenericArgument => {
                let node_id = LocalNodeId::<GenericArgument>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::TupleElement => {
                let node_id = LocalNodeId::<TupleElement>::new(self.id);
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
            NodeType::AssignPattern => {
                let node_id = LocalNodeId::<AssignPattern>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::AssignPatternField => {
                let node_id = LocalNodeId::<AssignPatternField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Declarator => {
                let node_id = LocalNodeId::<Declarator>::new(self.id);
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
