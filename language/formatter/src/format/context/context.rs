use super::options::DestackFormatOptions;
use crate::format::context::source::{SourceText, token_keyword_map};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

pub use destack_ast::Decorator;
use destack_ast::{
    Argument, Block, Declaration, Declarator, DependencyItem, EnumField, Expression,
    GenericArgument, GenericParameter, Keyword, LocalNodeId, LocalNodeIdAny, MatchCase, Member,
    Node, NodeParentIndex, NodeTree, NodeTreeImpl, NodeType, Parameter, Pattern, PatternField,
    Property, TokenSpan, TokenType, TupleElement, TypeExpression, TypeMember, WhereClause,
};
use destack_core::ImmutableStringPool;
use destack_fir::format::{Format, FormatContext, FormatResult, Formatter};
use destack_source::{File, MultiSpan, NodeSourceMap, Span};
use rustc_hash::FxHashMap;

use super::comment::Comments;

pub(crate) const NODE_BOOL_STATE_UNKNOWN: u8 = 0;
pub(crate) const NODE_BOOL_STATE_FALSE: u8 = 1;
pub(crate) const NODE_BOOL_STATE_TRUE: u8 = 2;
pub(crate) const TYPE_CONTEXT_STATE_UNKNOWN: u8 = 0;
pub(crate) const TYPE_CONTEXT_STATE_FALSE: u8 = 1;
pub(crate) const TYPE_CONTEXT_STATE_TRUE: u8 = 2;

/// One explicit type root with its leading comment boundary.
#[derive(Debug, Copy, Clone)]
pub struct TypeExpressionRoot {
    /// The node formatted in type position.
    pub node_id: LocalNodeIdAny,
    /// The earliest offset that may own leading raw comments for this root.
    pub leading_comment_start: Option<u32>,
}

/// One expression root formatted through one assignment-like type shell.
#[derive(Debug, Copy, Clone)]
pub struct AssignmentLikeTypeRoot {
    /// The expression formatted through the assignment-like shell.
    pub node_id: LocalNodeId<Expression>,
    /// Whether the shell keeps the rhs attached after the operator.
    pub rhs_is_inline_attached: bool,
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
    /// The raw comment cursor for this formatting pass.
    pub comments: Rc<RefCell<Comments<'a>>>,
    /// Expression roots that are explicitly formatted in type position.
    pub type_expression_roots: Rc<RefCell<Vec<TypeExpressionRoot>>>,
    /// Expression roots that are formatted through assignment-like type shells.
    pub assignment_like_type_roots: Rc<RefCell<Vec<AssignmentLikeTypeRoot>>>,
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
        let has_ignore_directive_markers = false;
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
                }
                TokenType::BlockComment | TokenType::DocBlockComment => {
                    comment_spans.push(token.span);
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
            comments: Rc::new(RefCell::new(Comments::new(
                file.id,
                SourceText::new(file.text()),
                tree.comments(),
            ))),
            type_expression_roots: Rc::new(RefCell::new(Vec::new())),
            assignment_like_type_roots: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Borrow the raw comment cursor.
    pub fn comments(&self) -> std::cell::Ref<'_, Comments<'a>> {
        self.comments.borrow()
    }

    /// Borrow the raw comment cursor mutably.
    pub fn comments_mut(&self) -> std::cell::RefMut<'_, Comments<'a>> {
        self.comments.borrow_mut()
    }

    /// Clone this context with one isolated raw comment cursor.
    pub fn fork_for_inspection(&self) -> Self {
        let unprinted_comments = {
            let comments = self.comments();
            comments.unprinted_comments()
        };

        let mut cloned = self.clone();
        cloned.comments = Rc::new(RefCell::new(Comments::new(
            cloned.file.id,
            SourceText::new(cloned.file.text()),
            unprinted_comments,
        )));
        cloned
    }

    /// Run one operation while one node is an explicit type root.
    pub fn with_type_expression_root<N: Node + Clone, T>(
        &self,
        node_id: LocalNodeId<N>,
        operation: impl FnOnce() -> T,
    ) -> T {
        self.with_type_expression_root_from(node_id, None, operation)
    }

    /// Run one operation while one node is an explicit type root with one leading boundary.
    pub fn with_type_expression_root_from<N: Node + Clone, T>(
        &self,
        node_id: LocalNodeId<N>,
        leading_comment_start: Option<u32>,
        operation: impl FnOnce() -> T,
    ) -> T {
        self.type_expression_roots
            .borrow_mut()
            .push(TypeExpressionRoot {
                node_id: node_id.into_any(),
                leading_comment_start,
            });

        let result = operation();

        let _ = self.type_expression_roots.borrow_mut().pop();
        result
    }

    /// Run one operation while one expression is an assignment-like type root.
    pub fn with_assignment_like_type_root<T>(
        &self,
        node_id: LocalNodeId<Expression>,
        rhs_is_inline_attached: bool,
        operation: impl FnOnce() -> T,
    ) -> T {
        self.assignment_like_type_roots
            .borrow_mut()
            .push(AssignmentLikeTypeRoot {
                node_id,
                rhs_is_inline_attached,
            });

        let result = operation();

        let _ = self.assignment_like_type_roots.borrow_mut().pop();
        result
    }

    /// Return whether one node is inside one explicit type root.
    pub fn is_in_type_expression_root<N: Node + Clone>(&self, node_id: LocalNodeId<N>) -> bool {
        let type_roots = self.type_expression_roots.borrow();
        if type_roots.is_empty() {
            return false;
        }

        let mut current_id = node_id.id;
        let mut current_type = N::TYPE;
        loop {
            if type_roots
                .iter()
                .rev()
                .any(|root| root.node_id == LocalNodeIdAny::new(current_id, current_type))
            {
                return true;
            }

            let Some((parent_id, parent_type)) = self.parent_by_id(current_id) else {
                return false;
            };

            current_id = parent_id;
            current_type = parent_type;
        }
    }

    /// Return the nearest explicit type root that owns one node.
    pub fn type_expression_root_id<N: Node + Clone>(
        &self,
        node_id: LocalNodeId<N>,
    ) -> Option<LocalNodeIdAny> {
        let type_roots = self.type_expression_roots.borrow();
        if type_roots.is_empty() {
            return None;
        }

        let mut current_id = node_id.id;
        let mut current_type = N::TYPE;
        loop {
            if let Some(root) = type_roots
                .iter()
                .rev()
                .find(|root| root.node_id == LocalNodeIdAny::new(current_id, current_type))
            {
                return Some(root.node_id);
            }

            let Some((parent_id, parent_type)) = self.parent_by_id(current_id) else {
                return None;
            };

            current_id = parent_id;
            current_type = parent_type;
        }
    }

    /// Return the leading raw-comment boundary for one explicit type root, if any.
    pub fn type_expression_leading_comment_start<N: Node + Clone>(
        &self,
        node_id: LocalNodeId<N>,
    ) -> Option<u32> {
        let type_roots = self.type_expression_roots.borrow();
        if type_roots.is_empty() {
            return None;
        }

        let mut current_id = node_id.id;
        let mut current_type = N::TYPE;
        loop {
            if let Some(root) = type_roots
                .iter()
                .rev()
                .find(|root| root.node_id == LocalNodeIdAny::new(current_id, current_type))
            {
                return root.leading_comment_start;
            }

            let Some((parent_id, parent_type)) = self.parent_by_id(current_id) else {
                return None;
            };

            current_id = parent_id;
            current_type = parent_type;
        }
    }

    /// Return whether one expression is inside one assignment-like type root.
    pub fn is_in_assignment_like_type_root(&self, node_id: LocalNodeId<Expression>) -> bool {
        let type_roots = self.assignment_like_type_roots.borrow();
        if type_roots.is_empty() {
            return false;
        }

        let mut current_id = node_id.id;
        loop {
            let current_expression_id = LocalNodeId::<Expression>::new(current_id);
            if type_roots
                .iter()
                .rev()
                .any(|root| root.node_id == current_expression_id)
            {
                return true;
            }

            let Some((parent_id, parent_type)) = self.parent_by_id(current_id) else {
                return false;
            };
            if parent_type != NodeType::Expression {
                return false;
            }

            current_id = parent_id;
        }
    }

    /// Return whether one assignment-like type root keeps the rhs attached inline.
    pub fn assignment_like_type_root_rhs_is_inline_attached(
        &self,
        node_id: LocalNodeId<Expression>,
    ) -> Option<bool> {
        let type_roots = self.assignment_like_type_roots.borrow();
        if type_roots.is_empty() {
            return None;
        }

        let mut current_id = node_id.id;
        loop {
            let current_expression_id = LocalNodeId::<Expression>::new(current_id);
            if let Some(root) = type_roots
                .iter()
                .rev()
                .find(|root| root.node_id == current_expression_id)
            {
                return Some(root.rhs_is_inline_attached);
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
