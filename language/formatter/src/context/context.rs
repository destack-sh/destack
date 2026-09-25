use super::options::TsppFormatOptions;
use super::source::SourceText;
use super::{FormatElementCache, FormatSourceIndex};

use tspp_core::StringPool;
pub use tspp_dir::Decorator;
use tspp_dir::{
    Argument, AssignPattern, AssignPatternField, Block, Catch, Comment, Declaration, Declarator,
    DependencyItem, EnumField, Expression, GenericArgument, GenericParameter, LocalNodeId,
    LocalNodeIdAny, MatchArm, Member, Node, NodeParentIndex, NodeType, Parameter, Pattern,
    PatternField, Property, SwitchCase, TokenSpan, Tree, TreeAttribute, TreeChild, TreeStore,
    TupleElement, TypeExpression, TypeMappedParameter, TypeMember, WhereClause,
};
use tspp_fir::format::{
    Format, FormatContext, FormatElement as FirElement, FormatLayout, FormatResult, Formatter,
};
use tspp_source::{File, MultiSpan, Span};

use super::comment::Comments;

/// The formatter implementation specialized for the TS++ context.
pub type TsppFormatter<'ast, 'state> = Formatter<'state, 'ast, TsppFormatContext<'ast>>;

/// Run one formatter callback with a temporary following sibling boundary.
pub(crate) fn with_following_span_start<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    following_span_start: Option<u32>,
    format: impl FnOnce(&mut TsppFormatter<'ast, '_>) -> FormatResult<()>,
) -> FormatResult<()> {
    let previous_following_span_start = f
        .context_mut()
        .replace_following_span_start(following_span_start);
    let result = format(f);
    f.context_mut()
        .replace_following_span_start(previous_following_span_start);

    result
}

/// Run one formatter callback with expanded tree callback bodies.
pub(crate) fn with_expanded_tree_callback_bodies<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    format: impl FnOnce(&mut TsppFormatter<'ast, '_>) -> FormatResult<()>,
) -> FormatResult<()> {
    let previous_should_expand = f
        .context_mut()
        .replace_should_expand_tree_callback_bodies(true);
    let result = format(f);
    f.context_mut()
        .replace_should_expand_tree_callback_bodies(previous_should_expand);

    result
}

/// TS++ format context.
#[derive(Debug)]
pub struct TsppFormatContext<'a> {
    /// The format options.
    pub options: TsppFormatOptions,
    /// The file.
    pub file: &'a File,
    /// The semantic tokens in source order.
    pub tokens: &'a [TokenSpan],
    /// The side span.
    pub side_span: &'a MultiSpan,
    /// The tree.
    pub tree: &'a Tree,
    /// The parent index.
    pub parents: &'a NodeParentIndex,
    /// The string pool.
    pub strings: &'a StringPool,
    /// The immutable source index for source-order lookups.
    pub source_index: FormatSourceIndex,
    /// The FIR element cache for this formatter pass.
    element_cache: FormatElementCache<'a>,
    /// The start position of the following sibling for the node currently being formatted.
    pub current_following_span_start: Option<u32>,
    /// Whether tree callback bodies should expand like tree return elements.
    pub should_expand_tree_callback_bodies: bool,
    /// The comment cursor for this formatting pass.
    pub comments: Comments<'a>,
}

impl<'a> TsppFormatContext<'a> {
    /// Construct a formatting context from parse artifacts.
    pub fn new(
        options: TsppFormatOptions,
        file: &'a File,
        tree: &'a Tree,
        tokens: &'a [TokenSpan],
        comments: &'a [Comment],
        side_span: &'a MultiSpan,
        strings: &'a StringPool,
        parents: &'a NodeParentIndex,
    ) -> Self {
        debug_assert!(tokens.iter().all(|token| token.token.ty().is_semantic()));
        let source_index = FormatSourceIndex::new(file, tree, comments);

        Self {
            options,
            file,
            tokens,
            side_span,
            tree,
            parents,
            strings,
            source_index,
            element_cache: FormatElementCache::default(),
            current_following_span_start: None,
            should_expand_tree_callback_bodies: false,
            comments: Comments::new(SourceText::new(file.text()), comments),
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

    /// Return one cached FIR element for a source span.
    pub(crate) fn cached_element(&self, span: &Span) -> Option<FirElement<'a>> {
        self.element_cache.get(span.range())
    }

    /// Cache one FIR element for a source span.
    pub(crate) fn cache_element(&mut self, span: &Span, element: FirElement<'a>) {
        self.element_cache.insert(span.range(), element);
    }

    /// Return the current following sibling start used for trailing comment ownership.
    pub fn following_span_start(&self) -> Option<u32> {
        self.current_following_span_start
    }

    /// Replace the current following sibling start and return the previous value.
    pub fn replace_following_span_start(
        &mut self,
        following_span_start: Option<u32>,
    ) -> Option<u32> {
        std::mem::replace(&mut self.current_following_span_start, following_span_start)
    }

    /// Return whether tree callback bodies should expand like tree return elements.
    #[inline]
    pub fn should_expand_tree_callback_bodies(&self) -> bool {
        self.should_expand_tree_callback_bodies
    }

    /// Replace whether tree callback bodies should expand like tree return elements.
    #[inline]
    pub fn replace_should_expand_tree_callback_bodies(&mut self, should_expand: bool) -> bool {
        std::mem::replace(&mut self.should_expand_tree_callback_bodies, should_expand)
    }

    /// Return the source line distance between two byte offsets.
    pub(crate) fn line_distance(&self, start: u32, end: u32) -> Option<u32> {
        let (start_line, _) = self.file.get_position(start)?;
        let (end_line, _) = self.file.get_position(end)?;

        end_line.checked_sub(start_line)
    }
}

impl FormatContext for TsppFormatContext<'_> {
    type Options = TsppFormatOptions;

    #[inline]
    fn options(&self) -> &Self::Options {
        &self.options
    }

    #[inline]
    fn file(&self) -> &File {
        self.file
    }
}

/// Speculative formatting helpers for one TS++ formatter.
pub(crate) trait TsppFormatterSpeculationExt<'ast> {
    /// Return whether formatting `content` after one source start would break.
    fn speculate_will_break_after(
        &mut self,
        start: u32,
        content: &dyn Format<'ast, TsppFormatContext<'ast>>,
    ) -> FormatResult<bool>;
}

impl<'ast> TsppFormatterSpeculationExt<'ast> for TsppFormatter<'ast, '_> {
    fn speculate_will_break_after(
        &mut self,
        start: u32,
        content: &dyn Format<'ast, TsppFormatContext<'ast>>,
    ) -> FormatResult<bool> {
        // speculation snapshot
        let snapshot = self.context().comments().snapshot();

        // speculative pass
        self.context_mut()
            .comments_mut()
            .skip_comments_before(start);

        let content = self.capture(content);

        // restore
        self.context_mut().comments_mut().restore(snapshot);
        let will_break = content?.is_some_and(|content| content.will_break());

        Ok(will_break)
    }
}

/// Format one typed source node with full context.
pub(crate) trait FormatNode<'a, T: Node>
where
    TsppFormatContext<'a>: FormatContext,
{
    /// Format one source node id.
    fn format_node(
        &self,
        node_id: LocalNodeId<T>,
        f: &mut TsppFormatter<'a, '_>,
    ) -> FormatResult<()>;
}

/// The formatted content for one node without trailing comments.
pub(crate) struct FormatNodeWithoutTrailingComments<T: Node>(pub LocalNodeId<T>);

/// Format a node.
impl<'a, T: Node> Format<'a, TsppFormatContext<'a>> for LocalNodeId<T>
where
    T: Node + Clone,
    Tree: TreeStore<T>,
    T: FormatNode<'a, T>,
{
    #[inline]
    fn format(&self, f: &mut TsppFormatter<'a, '_>) -> FormatResult<()> {
        let context = f.context();
        let node = context.tree.get(*self);

        node.format_node(*self, f)
    }
}

/// Format a node without trailing comments.
impl<'a, T: Node> Format<'a, TsppFormatContext<'a>> for FormatNodeWithoutTrailingComments<T>
where
    T: Node + Clone,
    Tree: TreeStore<T>,
    T: FormatNode<'a, T>,
{
    #[inline]
    fn format(&self, f: &mut TsppFormatter<'a, '_>) -> FormatResult<()> {
        if !f.context().comments().has_comments() {
            return self.0.format(f);
        }

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
impl<'a> Format<'a, TsppFormatContext<'a>> for LocalNodeIdAny {
    #[inline]
    fn format(&self, f: &mut TsppFormatter<'a, '_>) -> FormatResult<()> {
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
            NodeType::TreeAttribute => {
                let node_id = LocalNodeId::<TreeAttribute>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::TreeChild => {
                let node_id = LocalNodeId::<TreeChild>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::MatchArm => {
                let node_id = LocalNodeId::<MatchArm>::new(self.id);
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
            NodeType::SwitchCase => {
                let node_id = LocalNodeId::<SwitchCase>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
        }
    }
}
