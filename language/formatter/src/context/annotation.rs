use super::context::TsppFormatContext;
use tspp_dir::{Decorator, DecoratorPosition, LocalNodeId, Node, TokenType, Tree, TreeStore};
use tspp_source::Span;

impl<'a> TsppFormatContext<'a> {
    /// Get one annotation by id.
    #[inline]
    pub fn annotation(&self, annotation_id: LocalNodeId<Decorator>) -> &Decorator {
        self.tree.get(annotation_id)
    }

    /// Get one annotation span by id.
    #[inline]
    pub fn annotation_span(&self, annotation_id: LocalNodeId<Decorator>) -> Span {
        self.tree.get_span(annotation_id)
    }

    /// Return whether one annotation starts on its own source line.
    #[inline]
    pub fn annotation_starts_on_own_line(&self, annotation_id: LocalNodeId<Decorator>) -> bool {
        self.span_starts_on_own_line(self.annotation_span(annotation_id))
    }

    /// Return whether the next token after one annotation starts on the same line.
    pub fn annotation_next_token_is_on_same_line(
        &self,
        annotation_id: LocalNodeId<Decorator>,
    ) -> bool {
        let annotation_span = self.annotation_span(annotation_id);
        let Some(next_token) = self.next_token_after_span(annotation_span) else {
            return false;
        };
        if annotation_span.file != next_token.span.file {
            return false;
        }

        self.file
            .is_same_line(annotation_span.end.saturating_sub(1), next_token.span.start)
    }

    /// Return the next token type after one annotation span.
    #[inline]
    pub fn annotation_next_token_type(
        &self,
        annotation_id: LocalNodeId<Decorator>,
    ) -> Option<TokenType> {
        self.next_token_after_span(self.annotation_span(annotation_id))
            .map(|token| token.token.ty())
    }

    /// Return annotation ids for a node.
    #[inline]
    pub fn annotation_ids<T>(&self, node_id: LocalNodeId<T>) -> &[LocalNodeId<Decorator>]
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        self.tree.get_decorators_ref(node_id.id)
    }

    /// Return the earliest prefix annotation start for one node.
    pub fn prefix_annotation_start<T>(&self, node_id: LocalNodeId<T>, default: u32) -> u32
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        self.annotation_ids(node_id)
            .iter()
            .copied()
            .filter(|annotation_id| {
                matches!(
                    self.annotation(*annotation_id).position,
                    DecoratorPosition::BlockPrefix | DecoratorPosition::LinePrefix
                )
            })
            .map(|annotation_id| self.annotation_span(annotation_id).start)
            .fold(default, u32::min)
    }

    /// Check if a node has an annotation.
    #[inline]
    pub fn has_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        !self.tree.get_decorators_ref(node_id.id).is_empty()
    }

    /// Check if a node has a prefix annotation.
    #[inline]
    pub fn has_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node + Clone,
        Tree: TreeStore<T>,
    {
        self.annotation_ids(node_id)
            .iter()
            .copied()
            .any(|annotation_id| {
                let position = self.annotation(annotation_id).position;

                matches!(
                    position,
                    DecoratorPosition::BlockPrefix | DecoratorPosition::LinePrefix
                )
            })
    }

    /// Check if a node has a block infix annotation.
    #[inline]
    pub fn has_infix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        self.annotation_ids(node_id)
            .iter()
            .copied()
            .any(|annotation_id| {
                self.annotation(annotation_id).position == DecoratorPosition::BlockInfix
            })
    }

    /// Check if a node has a postfix annotation.
    #[inline]
    pub fn has_postfix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        self.annotation_ids(node_id)
            .iter()
            .copied()
            .any(|annotation_id| {
                let position = self.annotation(annotation_id).position;

                matches!(
                    position,
                    DecoratorPosition::BlockPostfix | DecoratorPosition::LinePostfix
                )
            })
    }
}
