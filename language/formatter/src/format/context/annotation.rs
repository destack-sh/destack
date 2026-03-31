use super::context::DestackFormatContext;
use destack_ast as ast;
use destack_ast::{Annotation, AnnotationPosition, LocalNodeId, Node, NodeTree, NodeTreeImpl};
use destack_source::Span;

impl<'a> DestackFormatContext<'a> {
    /// Get one annotation by id.
    #[inline]
    pub fn annotation(&self, annotation_id: LocalNodeId<Annotation>) -> Annotation {
        *self.tree.get(annotation_id)
    }

    /// Get one annotation span by id.
    #[inline]
    pub fn annotation_span(&self, annotation_id: LocalNodeId<Annotation>) -> Span {
        self.tree.get_span(annotation_id)
    }

    /// Return whether one annotation starts on its own source line.
    #[inline]
    pub fn annotation_starts_on_own_line(&self, annotation_id: LocalNodeId<Annotation>) -> bool {
        self.span_starts_on_own_line(self.annotation_span(annotation_id))
    }

    /// Return whether one annotation source starts after at least one newline.
    #[inline]
    pub fn annotation_has_leading_newline(&self, annotation_id: LocalNodeId<Annotation>) -> bool {
        self.has_newline(self.annotation_span(annotation_id))
    }

    /// Return the nearest non-whitespace token before one annotation span.
    #[inline]
    pub fn annotation_previous_non_whitespace_token(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<ast::TokenSpan> {
        let annotation_span = self.annotation_span(annotation_id);
        self.previous_non_whitespace_token_before_span(annotation_span)
    }

    /// Return the nearest non-whitespace token after one annotation span.
    #[inline]
    pub fn annotation_next_non_whitespace_token(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<ast::TokenSpan> {
        let annotation_span = self.annotation_span(annotation_id);
        self.next_non_whitespace_token_after_span(annotation_span)
    }

    /// Return the previous non-whitespace token type before one annotation span.
    #[inline]
    pub fn annotation_previous_non_whitespace_token_type(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<ast::TokenType> {
        self.annotation_previous_non_whitespace_token(annotation_id)
            .map(|token| token.token.ty)
    }

    /// Return whether the next non-whitespace token after one annotation starts on the same line.
    pub fn annotation_next_token_is_on_same_line(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> bool {
        let annotation_span = self.annotation_span(annotation_id);
        let Some(next_token) = self.annotation_next_non_whitespace_token(annotation_id) else {
            return false;
        };
        if annotation_span.file != next_token.span.file {
            return false;
        }

        self.file
            .is_same_line(annotation_span.end.saturating_sub(1), next_token.span.start)
    }

    /// Return the next non-whitespace token type after one annotation span.
    #[inline]
    pub fn annotation_next_non_whitespace_token_type(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<ast::TokenType> {
        self.annotation_next_non_whitespace_token(annotation_id)
            .map(|token| token.token.ty)
    }

    /// Return the next non-trivia token type after one annotation span.
    #[inline]
    pub fn annotation_next_non_trivia_token_type(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<ast::TokenType> {
        self.next_non_trivia_token_type_after_span(self.annotation_span(annotation_id))
    }

    /// Return the guard target token type after one annotation-following semicolon.
    #[inline]
    pub fn annotation_semicolon_guard_target_token_type(
        &self,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<ast::TokenType> {
        let token_after_annotation = self.annotation_next_non_whitespace_token(annotation_id)?;
        if token_after_annotation.token.ty != ast::TokenType::Semicolon {
            return None;
        }

        self.next_non_trivia_token_type_after_span(token_after_annotation.span)
    }

    /// Return whether one annotation starts after at least one leading indentation column.
    #[inline]
    pub fn annotation_starts_indented(&self, annotation_id: LocalNodeId<Annotation>) -> bool {
        let annotation_span = self.annotation_span(annotation_id);
        let annotation_column = self
            .file
            .get_position(annotation_span.start)
            .map_or(1, |(_, column)| column);

        annotation_column > 1
    }

    /// Return annotation ids for a node.
    #[inline]
    pub fn annotation_ids<T>(&self, node_id: LocalNodeId<T>) -> &[LocalNodeId<Annotation>]
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.tree.get_annotations_ref(node_id.id)
    }

    /// Check if a node has an annotation.
    #[inline]
    pub fn has_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        !self.tree.get_annotations_ref(node_id.id).is_empty()
    }

    /// Check if a node has a prefix annotation.
    #[inline]
    pub fn has_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_ids(node_id)
            .iter()
            .copied()
            .any(|annotation_id| {
                matches!(
                    self.annotation(annotation_id).position(),
                    AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
                )
            })
    }

    /// Check if a node has a block prefix annotation.
    #[inline]
    pub fn has_block_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_ids(node_id)
            .iter()
            .copied()
            .any(|annotation_id| {
                self.annotation(annotation_id).position() == AnnotationPosition::BlockPrefix
            })
    }

    /// Check if a node has a line prefix annotation.
    #[inline]
    pub fn has_line_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_ids(node_id)
            .iter()
            .copied()
            .any(|annotation_id| {
                self.annotation(annotation_id).position() == AnnotationPosition::LinePrefix
            })
    }

    /// Check if a node has a block infix annotation.
    #[inline]
    pub fn has_infix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_ids(node_id)
            .iter()
            .copied()
            .any(|annotation_id| {
                self.annotation(annotation_id).position() == AnnotationPosition::BlockInfix
            })
    }

    /// Check if a node has a semantic annotation other than one boundary postfix comment.
    #[inline]
    pub fn has_non_boundary_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_ids(node_id)
            .iter()
            .copied()
            .any(|annotation_id| {
                self.annotation(annotation_id).position() != AnnotationPosition::LinePostfixBoundary
            })
    }

    /// Check if a node has a postfix annotation.
    #[inline]
    pub fn has_postfix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_ids(node_id)
            .iter()
            .copied()
            .any(|annotation_id| {
                matches!(
                    self.annotation(annotation_id).position(),
                    AnnotationPosition::BlockPostfix
                        | AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                )
            })
    }
}
