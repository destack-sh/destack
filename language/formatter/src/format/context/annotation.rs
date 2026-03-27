use super::format_context::{ANNOTATION_STATE_NONE, Annotation, DestackFormatContext};
use destack_ast as ast;
use destack_ast::{
    AnnotationPosition, Argument, Comment, Expression, LocalNodeId, Node, NodeTree, NodeTreeImpl,
};
use destack_source::Span;

impl<'a> DestackFormatContext<'a> {
    /// Return whether one annotation position is one prefix position.
    #[inline]
    fn is_prefix_annotation_position(position: AnnotationPosition) -> bool {
        matches!(
            position,
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        )
    }

    /// Get one formatter-owned annotation by id.
    #[inline]
    pub fn annotation(&self, annotation_id: LocalNodeId<Annotation>) -> Annotation {
        self.annotation_entries[annotation_id.id as usize].annotation
    }

    /// Get one formatter-owned annotation span by id.
    #[inline]
    pub fn annotation_span(&self, annotation_id: LocalNodeId<Annotation>) -> Span {
        self.annotation_entries[annotation_id.id as usize].span
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
    fn annotation_ids_for_node<T>(
        &self,
        node_id: LocalNodeId<T>,
    ) -> Option<&[LocalNodeId<Annotation>]>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_index = node_id.id as usize;
        let state = self.annotation_state_by_node_id[node_index].get();
        if state == ANNOTATION_STATE_NONE {
            return None;
        }

        self.annotation_ids_by_node_id
            .get(node_index)
            .map(|annotation_ids| annotation_ids.as_slice())
    }

    /// Get annotations for a node. Annotations are sorted by position.
    #[inline]
    pub fn annotations<T>(&self, node_id: LocalNodeId<T>) -> Option<Vec<LocalNodeId<Annotation>>>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_ids_for_node(node_id).map(ToOwned::to_owned)
    }

    /// Read node annotations without cloning.
    #[inline]
    pub fn visit_annotations<T, R, F>(&self, node_id: LocalNodeId<T>, f: F) -> Option<R>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
        F: FnOnce(&[LocalNodeId<Annotation>]) -> R,
    {
        self.annotation_ids_for_node(node_id).map(f)
    }

    /// Return whether any annotation on a node matches one predicate.
    #[inline]
    fn any_annotation<T, F>(&self, node_id: LocalNodeId<T>, mut predicate: F) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
        F: FnMut(usize, LocalNodeId<Annotation>, Annotation) -> bool,
    {
        self.visit_annotations(node_id, |annotation_ids| {
            annotation_ids
                .iter()
                .copied()
                .enumerate()
                .any(|(index, annotation_id)| {
                    predicate(index, annotation_id, self.annotation(annotation_id))
                })
        })
        .unwrap_or(false)
    }

    /// Return whether any annotation on a node matches one predicate.
    #[inline]
    pub fn any_annotation_id<T, F>(&self, node_id: LocalNodeId<T>, mut predicate: F) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
        F: FnMut(LocalNodeId<Annotation>) -> bool,
    {
        self.visit_annotations(node_id, |annotation_ids| {
            annotation_ids.iter().copied().any(&mut predicate)
        })
        .unwrap_or(false)
    }

    /// Return the first mapped annotation result for one node.
    #[inline]
    pub fn find_annotation_id<T, R, F>(
        &self,
        node_id: LocalNodeId<T>,
        mut predicate: F,
    ) -> Option<R>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
        F: FnMut(LocalNodeId<Annotation>) -> Option<R>,
    {
        self.visit_annotations(node_id, |annotation_ids| {
            annotation_ids.iter().copied().find_map(&mut predicate)
        })
        .flatten()
    }

    /// Check if a node has an annotation.
    #[inline]
    pub fn has_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.annotation_state_by_node_id[node_id.id as usize].get() != ANNOTATION_STATE_NONE
    }

    /// Check if a node has a prefix annotation.
    #[inline]
    pub fn has_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.any_annotation(node_id, |_, _, annotation| {
            matches!(
                annotation.position(),
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
        self.any_annotation(node_id, |_, _, annotation| {
            annotation.position() == AnnotationPosition::BlockPrefix
        })
    }

    /// Check if a node has a line prefix annotation.
    #[inline]
    pub fn has_line_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.any_annotation(node_id, |_, _, annotation| {
            annotation.position() == AnnotationPosition::LinePrefix
        })
    }

    /// Check if a node has a block infix annotation.
    #[inline]
    pub fn has_infix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.any_annotation(node_id, |_, _, annotation| {
            annotation.position() == AnnotationPosition::BlockInfix
        })
    }

    /// Check if a node has a non-blank annotation.
    #[inline]
    pub fn has_non_blank_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.any_annotation(node_id, |_, _, annotation| {
            !matches!(annotation, Annotation::Blank { .. })
        })
    }

    /// Check if a node has a non-blank annotation other than one boundary postfix comment.
    #[inline]
    pub fn has_non_blank_non_boundary_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.any_annotation(node_id, |_, _, annotation| {
            !matches!(annotation, Annotation::Blank { .. })
                && !matches!(
                    annotation,
                    Annotation::Comment {
                        position: AnnotationPosition::LinePostfixBoundary,
                        ..
                    }
                )
        })
    }

    /// Check if a node has a non-blank block infix annotation.
    #[inline]
    pub fn has_non_blank_infix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.any_annotation(node_id, |_, _, annotation| {
            annotation.position() == AnnotationPosition::BlockInfix
                && !matches!(annotation, Annotation::Blank { .. })
        })
    }

    /// Check if a node has a postfix annotation.
    #[inline]
    pub fn has_postfix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.any_annotation(node_id, |_, _, annotation| {
            matches!(
                annotation.position(),
                AnnotationPosition::BlockPostfix
                    | AnnotationPosition::LinePostfix
                    | AnnotationPosition::LinePostfixBoundary
            )
        })
    }

    /// Check if a node has a non-blank postfix annotation.
    #[inline]
    pub fn has_non_blank_postfix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.any_annotation(node_id, |_, _, annotation| {
            matches!(
                annotation.position(),
                AnnotationPosition::BlockPostfix
                    | AnnotationPosition::LinePostfix
                    | AnnotationPosition::LinePostfixBoundary
            ) && !matches!(annotation, Annotation::Blank { .. })
        })
    }

    /// Check if a node has a blank postfix annotation.
    #[inline]
    pub fn has_blank_postfix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.any_annotation(node_id, |_, _, annotation| {
            matches!(annotation, Annotation::Blank { .. })
                && matches!(
                    annotation.position(),
                    AnnotationPosition::BlockPostfix
                        | AnnotationPosition::LinePostfix
                        | AnnotationPosition::LinePostfixBoundary
                )
        })
    }

    /// Check if a node has a blank block prefix annotation.
    #[inline]
    pub fn has_blank_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.any_annotation(node_id, |_, _, annotation| {
            matches!(annotation, Annotation::Blank { .. })
                && matches!(
                    annotation.position(),
                    AnnotationPosition::BlockPrefix | AnnotationPosition::LinePrefix
                )
        })
    }

    /// Check if a node has a boundary postfix comment annotation.
    #[inline]
    pub fn has_boundary_comment_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        self.any_annotation(node_id, |_, _, annotation| {
            matches!(
                annotation,
                Annotation::Comment {
                    position: AnnotationPosition::LinePostfixBoundary,
                    ..
                }
            )
        })
    }

    /// Return whether one argument has a slash-style line comment annotation.
    pub fn argument_has_line_comment_annotation(&self, argument_id: LocalNodeId<Argument>) -> bool {
        if !self.has_annotation(argument_id) {
            return false;
        }

        let argument_end = self.span(argument_id).end;
        self.visit_annotations(argument_id, |annotations| {
            annotations.iter().copied().any(|annotation_id| {
                let Annotation::Comment { node, .. } = self.annotation(annotation_id) else {
                    return false;
                };

                if self.tree.get::<Comment>(node).style != ast::CommentStyle::Slash {
                    return false;
                }

                self.annotation_span(annotation_id).start >= argument_end
            })
        })
        .unwrap_or(false)
    }

    /// Return whether one argument has a slash-style prefix comment annotation.
    pub fn argument_has_prefix_line_comment_annotation(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> bool {
        if !self.has_annotation(argument_id) {
            return false;
        }

        self.visit_annotations(argument_id, |annotations| {
            annotations.iter().copied().any(|annotation_id| {
                let annotation = self.annotation(annotation_id);
                if !Self::is_prefix_annotation_position(annotation.position()) {
                    return false;
                }

                let Annotation::Comment { node, .. } = annotation else {
                    return false;
                };
                self.tree.get::<Comment>(node).style == ast::CommentStyle::Slash
            })
        })
        .unwrap_or(false)
    }

    /// Return whether one argument has a non-blank prefix annotation signal.
    pub fn argument_has_prefix_annotation_signal(
        &self,
        argument_id: LocalNodeId<Argument>,
    ) -> bool {
        if self.has_annotation(argument_id)
            && self
                .visit_annotations(argument_id, |annotations| {
                    annotations.iter().copied().any(|annotation_id| {
                        let annotation = self.annotation(annotation_id);
                        !matches!(annotation, Annotation::Blank { .. })
                            && Self::is_prefix_annotation_position(annotation.position())
                    })
                })
                .unwrap_or(false)
        {
            return true;
        }

        let argument_value_expression = match self.tree.get(argument_id) {
            Argument::Named { value, .. }
            | Argument::Labeled { value, .. }
            | Argument::Positional { value, .. }
            | Argument::Spread { value, .. } => Some(*value),
            Argument::Error => None,
        };
        let Some(argument_value_expression) = argument_value_expression else {
            return false;
        };

        let value_id = self.transparent_inner_expression(argument_value_expression);
        if self.has_annotation(value_id) && self.has_prefix_annotation(value_id) {
            return true;
        }

        if let Expression::Declaration(declaration_id) = self.tree.get(value_id) {
            return self.has_annotation(*declaration_id)
                && self.has_prefix_annotation(*declaration_id);
        }

        false
    }
}
