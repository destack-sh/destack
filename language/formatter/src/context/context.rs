use super::options::DestackFormatOptions;
use super::source::SourceText;
use super::{FormatElementCache, FormatSourceIndex};
use std::rc::Rc;

use destack_core::StringPool;
pub use destack_dir::Decorator;
use destack_dir::{
    Argument, AssignPattern, AssignPatternField, Block, Catch, Declaration, Declarator,
    DependencyItem, EnumField, Expression, GenericArgument, GenericParameter, LocalNodeId,
    LocalNodeIdAny, MatchCase, Member, Node, NodeParentIndex, NodeType, Parameter, Pattern,
    PatternField, Property, TokenSpan, Tree, TreeStore, TupleElement, TypeExpression,
    TypeMappedParameter, TypeMember, WhereClause,
};
use destack_fir::format::{
    Format, FormatContext, FormatNode as FirNode, FormatNodes, FormatResult, Formatter,
};
use destack_source::{File, MultiSpan, Span};

use super::comment::Comments;

/// The formatter implementation specialized for the Destack context.
pub type DestackFormatter<'ast, 'buf> = Formatter<'buf, DestackFormatContext<'ast>>;

/// Run one formatter callback with a temporary following sibling boundary.
pub(crate) fn with_following_span_start<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    following_span_start: u32,
    format: impl FnOnce(&mut DestackFormatter<'ast, '_>) -> FormatResult<()>,
) -> FormatResult<()> {
    let previous_following_span_start = f
        .context_mut()
        .replace_following_span_start(following_span_start);
    let result = format(f);
    f.context_mut()
        .replace_following_span_start(previous_following_span_start);

    result
}

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
    pub tree: &'a Tree,
    /// The parent index.
    pub parents: NodeParentIndex,
    /// The string pool.
    pub strings: &'a StringPool,
    /// The immutable source index shared by cloned contexts.
    pub source_index: Rc<FormatSourceIndex>,
    /// The formatted element cache for this formatter pass.
    pub element_cache: FormatElementCache,
    /// The start position of the following sibling for the node currently being formatted.
    pub current_following_span_start: u32,
    /// The comment cursor for this formatting pass.
    pub comments: Comments<'a>,
}

impl<'a> DestackFormatContext<'a> {
    /// Construct a formatting context from parse artifacts.
    pub fn new(
        options: DestackFormatOptions,
        file: &'a File,
        tree: &'a Tree,
        tokens: &'a [TokenSpan],
        side_tokens: &'a [TokenSpan],
        side_span: &'a MultiSpan,
        strings: &'a StringPool,
        parents: NodeParentIndex,
    ) -> Self {
        let source_index = Rc::new(FormatSourceIndex::new(file, tokens, side_tokens));

        Self {
            options,
            file,
            tokens,
            side_tokens,
            side_span,
            tree,
            parents,
            strings,
            source_index,
            element_cache: FormatElementCache::default(),
            current_following_span_start: 0,
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
        self.element_cache.get(span)
    }

    /// Cache one formatted element for one source span.
    pub fn cache_element(&mut self, span: &Span, node: FirNode) {
        self.element_cache.insert(*span, node);
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

/// Format one typed source node with full context.
pub(crate) trait FormatNode<'a, T: Node>
where
    DestackFormatContext<'a>: FormatContext,
{
    /// Format one source node id.
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
    Tree: TreeStore<T>,
    T: FormatNode<'a, T>,
{
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'a, '_>) -> FormatResult<()> {
        destack_core::ensure_sufficient_stack(|| {
            let context = f.context();
            let node = context.tree.get(*self);

            node.format_node(*self, f)
        })
    }
}

/// Format a node without trailing comments.
impl<'a, T: Node> Format<DestackFormatContext<'a>> for FormatNodeWithoutTrailingComments<T>
where
    T: Node + Clone,
    Tree: TreeStore<T>,
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
            NodeType::Catch => {
                let node_id = LocalNodeId::<Catch>::new(self.id);
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
            NodeType::TypeMappedParameter => {
                let node_id = LocalNodeId::<TypeMappedParameter>::new(self.id);
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
