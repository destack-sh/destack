use super::options::DestackFormatOptions;
use crate::format::context::source::token_keyword_map;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

pub use destack_ast::Annotation;
use destack_ast::{
    Argument, Blank, Block, Comment, Declaration, Declarator, Decorator, DependencyItem, Doc,
    EnumField, Expression, Keyword, LocalNodeId, LocalNodeIdAny, MatchCase, Member, Node,
    NodeParentIndex, NodeTree, NodeTreeImpl, NodeType, Parameter, Pattern, PatternField, Property,
    TokenSpan, TokenType, WhereClause,
};
use destack_core::ImmutableStringPool;
use destack_fir::format::{Format, FormatContext, FormatResult, Formatter, GroupId};
use destack_source::{File, MultiSpan, NodeSourceMap, Span};
use rustc_hash::FxHashMap;

pub(crate) const NODE_BOOL_STATE_UNKNOWN: u8 = 0;
pub(crate) const NODE_BOOL_STATE_FALSE: u8 = 1;
pub(crate) const NODE_BOOL_STATE_TRUE: u8 = 2;
pub(crate) const TYPE_CONTEXT_STATE_UNKNOWN: u8 = 0;
pub(crate) const TYPE_CONTEXT_STATE_FALSE: u8 = 1;
pub(crate) const TYPE_CONTEXT_STATE_TRUE: u8 = 2;

/// The explicit formatter role for one expression subtree.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ExpressionFormatRole {
    /// The expression is formatted as a value subtree.
    Value,
    /// The expression is formatted as a type subtree.
    Type,
}

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
    /// The current argument list group id, if any.
    pub current_argument_group_id: Option<GroupId>,
    /// Cached source slices for repeated span lookups.
    pub span_text_by_span: RefCell<FxHashMap<Span, &'a str>>,
    /// Cached parsed identifier keywords by token span.
    pub token_keyword_by_span: RefCell<FxHashMap<Span, Option<Keyword>>>,
    /// Whether the file text is fully ASCII.
    pub source_is_ascii: bool,
    /// Cached newline byte offsets in file text.
    pub newline_offsets: OnceCell<Vec<u32>>,
    /// Cached newline checks for repeated span newline predicates.
    pub span_has_newline_by_span: RefCell<FxHashMap<Span, bool>>,
    /// Cached newline predicates keyed by node id.
    pub(crate) node_has_newline_by_node_id: Vec<Cell<u8>>,
    /// Cached transparent inner expression ids keyed by expression node id.
    pub(crate) transparent_inner_expression_by_node_id: Vec<Cell<Option<LocalNodeId<Expression>>>>,
    /// Cached template interpolation ancestry states keyed by expression node id.
    pub(crate) expression_template_interpolation_by_node_id: Vec<Cell<u8>>,
    /// Cached type-conditional ancestry states keyed by expression node id.
    pub(crate) expression_type_conditional_ancestor_by_node_id: Vec<Cell<u8>>,
    /// Cached sorted comment tokens for ignore-range scans.
    pub comment_tokens_sorted: OnceCell<Vec<TokenSpan>>,
    /// Comment spans for this file, sorted by start position.
    pub comment_spans: Vec<Span>,
    /// Line-comment spans for this file, sorted by start position.
    pub line_comment_spans: Vec<Span>,
    /// Whether file text contains formatter ignore directive markers.
    pub has_ignore_directive_markers: bool,
    /// Whether file text contains template literal markers.
    pub has_template_literal_markers: bool,
    /// Whether file-level ignore was applied during formatting.
    pub file_ignore_applied: Rc<Cell<bool>>,
    /// Comment ids already owned by an outer formatter shell.
    pub owned_comment_nodes: Rc<RefCell<Vec<u32>>>,
    /// Expression roots with explicit formatter roles.
    pub expression_format_role_roots:
        Rc<RefCell<Vec<(LocalNodeId<Expression>, ExpressionFormatRole)>>>,
}

impl<'a> DestackFormatContext<'a> {
    /// Construct a formatting context from parse artifacts.
    pub fn new(
        options: DestackFormatOptions,
        file: &'a File,
        tree: &'a NodeTree,
        tokens: &'a Vec<TokenSpan>,
        side_tokens: &'a Vec<TokenSpan>,
        side_span: &'a MultiSpan,
        strings: &'a ImmutableStringPool,
        parents: NodeParentIndex,
    ) -> Self {
        let token_keyword_by_span = token_keyword_map(file, tokens, side_tokens);
        let node_count = tree.next_id() as usize;
        let source_is_ascii = file.text().is_ascii();
        let mut has_ignore_directive_markers = false;
        let mut has_template_literal_markers = false;
        let mut comment_spans = Vec::new();
        let mut line_comment_spans = Vec::new();
        for token in tokens.iter().chain(side_tokens.iter()) {
            if matches!(
                token.token.ty,
                TokenType::TemplateStringStart
                    | TokenType::TemplateStringMiddle
                    | TokenType::TemplateStringEnd
                    | TokenType::TemplateString
            ) {
                has_template_literal_markers = true;
            }

            match token.token.ty {
                TokenType::LineComment | TokenType::DocLineComment => {
                    comment_spans.push(token.span);
                    line_comment_spans.push(token.span);

                    if !has_ignore_directive_markers {
                        let raw = file.span_str(token.span);
                        has_ignore_directive_markers =
                            crate::format::directive::is_any_ignore_directive_comment(raw);
                    }
                }
                TokenType::BlockComment | TokenType::DocBlockComment => {
                    comment_spans.push(token.span);

                    if !has_ignore_directive_markers {
                        let raw = file.span_str(token.span);
                        has_ignore_directive_markers =
                            crate::format::directive::is_any_ignore_directive_comment(raw);
                    }
                }
                _ => {}
            }
        }
        comment_spans.sort_by_key(|span| span.start);
        line_comment_spans.sort_by_key(|span| span.start);

        Self {
            options,
            file,
            tokens,
            side_tokens,
            side_span,
            tree,
            source_map: &tree.source_map,
            parents,
            strings,
            current_argument_group_id: None,
            span_text_by_span: RefCell::new(FxHashMap::default()),
            token_keyword_by_span: RefCell::new(token_keyword_by_span),
            source_is_ascii,
            newline_offsets: OnceCell::new(),
            span_has_newline_by_span: RefCell::new(FxHashMap::default()),
            node_has_newline_by_node_id: vec![Cell::new(NODE_BOOL_STATE_UNKNOWN); node_count],
            transparent_inner_expression_by_node_id: vec![Cell::new(None); node_count],
            expression_template_interpolation_by_node_id: vec![
                Cell::new(TYPE_CONTEXT_STATE_UNKNOWN);
                node_count
            ],
            expression_type_conditional_ancestor_by_node_id: vec![
                Cell::new(
                    TYPE_CONTEXT_STATE_UNKNOWN
                );
                node_count
            ],
            comment_tokens_sorted: OnceCell::new(),
            comment_spans,
            line_comment_spans,
            has_ignore_directive_markers,
            has_template_literal_markers,
            file_ignore_applied: Rc::new(Cell::new(false)),
            owned_comment_nodes: Rc::new(RefCell::new(Vec::new())),
            expression_format_role_roots: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Push comment ids already owned by an outer formatter shell.
    pub fn push_owned_comment_nodes(&self, comment_ids: &[LocalNodeId<Comment>]) {
        let mut owned_comment_nodes = self.owned_comment_nodes.borrow_mut();
        owned_comment_nodes.extend(comment_ids.iter().map(|comment_id| comment_id.id));
    }

    /// Run one operation while a batch of comment ids is owned by an outer formatter shell.
    pub fn with_owned_comment_nodes<T>(
        &self,
        comment_ids: &[LocalNodeId<Comment>],
        operation: impl FnOnce() -> T,
    ) -> T {
        self.push_owned_comment_nodes(comment_ids);

        let result = operation();

        self.pop_owned_comment_nodes(comment_ids.len());
        result
    }

    /// Pop one trailing batch of outer-owned comment ids.
    pub fn pop_owned_comment_nodes(&self, count: usize) {
        let mut owned_comment_nodes = self.owned_comment_nodes.borrow_mut();
        let new_len = owned_comment_nodes.len().saturating_sub(count);
        owned_comment_nodes.truncate(new_len);
    }

    /// Return whether one comment id is already owned by an outer formatter shell.
    pub fn is_comment_owned(&self, comment_id: LocalNodeId<Comment>) -> bool {
        self.owned_comment_nodes.borrow().contains(&comment_id.id)
    }

    /// Run one operation while one expression has one explicit formatter role.
    pub fn with_expression_format_role_root<T>(
        &self,
        node_id: LocalNodeId<Expression>,
        role: ExpressionFormatRole,
        operation: impl FnOnce() -> T,
    ) -> T {
        self.expression_format_role_roots
            .borrow_mut()
            .push((node_id, role));

        let result = operation();

        let _ = self.expression_format_role_roots.borrow_mut().pop();
        result
    }

    /// Return the nearest explicit formatter role for one expression subtree.
    pub fn expression_format_role(
        &self,
        node_id: LocalNodeId<Expression>,
    ) -> Option<ExpressionFormatRole> {
        let format_roots = self.expression_format_role_roots.borrow();
        if format_roots.is_empty() {
            return None;
        }

        let mut current_id = node_id.id;
        loop {
            let current_expression_id = LocalNodeId::<Expression>::new(current_id);
            if let Some((_, role)) = format_roots
                .iter()
                .rev()
                .find(|(root_id, _)| *root_id == current_expression_id)
            {
                return Some(*role);
            }

            let Some((parent_id, parent_type)) = self.parent_by_id(current_id) else {
                return None;
            };
            if parent_type != NodeType::Expression {
                return None;
            }

            current_id = parent_id;
        }
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

/// Implement formatter dispatch for typed local node ids.
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

/// Implement formatter dispatch for dynamically typed local node ids.
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
                let node = context.annotation(node_id);
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
