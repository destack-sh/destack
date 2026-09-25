use crate::source::TokenType;
use tspp_source::Span;

use crate::{Block, Function, Global, TypeDeclaration};

use super::Parser;

impl Parser {
    /// Attach parsed comment ownership to renderable MIR anchors.
    pub(super) fn attach_comment_ownership(&mut self) {
        // top level
        let item_anchor_ids = self.top_level_anchor_ids();
        self.attach_leading_comments(&item_anchor_ids, 0);

        // function bodies
        let function_ids: Vec<_> = self
            .tree
            .iter_nodes::<Function>()
            .map(|(function_id, _)| function_id)
            .collect();

        for function_id in function_ids {
            let Some(function_span) = self.tree.get_span(function_id) else {
                continue;
            };

            let body_anchor_ids = self.function_body_anchor_ids(function_id);
            self.attach_leading_comments(&body_anchor_ids, function_span.start);

            // blocks
            let block_ids = self.tree.get(function_id).blocks().to_vec();
            for block_id in block_ids {
                let Some(block_span) = self.tree.get_span(block_id) else {
                    continue;
                };

                let block_anchor_ids = self.block_anchor_ids(block_id);
                self.attach_leading_comments(&block_anchor_ids, block_span.start);
            }
        }
    }

    /// Return the top level render anchors in source order.
    fn top_level_anchor_ids(&self) -> Vec<u32> {
        self.sorted_anchor_ids(
            self.tree
                .iter_nodes::<TypeDeclaration>()
                .map(|(id, _)| id.id)
                .chain(self.tree.iter_nodes::<Global>().map(|(id, _)| id.id))
                .chain(self.tree.iter_nodes::<Function>().map(|(id, _)| id.id)),
        )
    }

    /// Return the function body render anchors in source order.
    fn function_body_anchor_ids(&self, function_id: crate::LocalNodeId<Function>) -> Vec<u32> {
        let function = self.tree.get(function_id);
        self.sorted_anchor_ids(
            function
                .locals()
                .iter()
                .map(|id| id.id)
                .chain(function.blocks().iter().map(|id| id.id)),
        )
    }

    /// Return the block body render anchors in source order.
    fn block_anchor_ids(&self, block_id: crate::LocalNodeId<Block>) -> Vec<u32> {
        let block = self.tree.get(block_id);
        self.sorted_anchor_ids(
            block
                .instructions
                .iter()
                .map(|id| id.id)
                .chain(std::iter::once(block.terminator.id)),
        )
    }

    /// Return anchor ids sorted by source order.
    fn sorted_anchor_ids(&self, node_ids: impl IntoIterator<Item = u32>) -> Vec<u32> {
        let mut node_ids: Vec<_> = node_ids.into_iter().collect();

        // source order
        node_ids.sort_by_key(|&node_id| self.tree.get_span_by_id(node_id).map(|span| span.start));

        node_ids
    }

    /// Attach leading comment ownership within one render scope.
    fn attach_leading_comments(&mut self, node_ids: &[u32], scope_start: u32) {
        let Some((&first_id, rest)) = node_ids.split_first() else {
            return;
        };

        let Some(first_span) = self.tree.get_span_by_id(first_id) else {
            return;
        };

        // initial comments
        if let Some(span) = self.leading_comments_before(first_span.start, scope_start)
            && !span.is_empty()
        {
            self.tree.set_leading_comment_span_by_id(first_id, span);
        }

        let mut previous_span = first_span;

        // interior sibling gaps
        for &node_id in rest {
            let Some(current_span) = self.tree.get_span_by_id(node_id) else {
                continue;
            };

            if let Some(span) = self.leading_comments_in_gap(previous_span.end, current_span.start)
            {
                self.tree.set_leading_comment_span_by_id(node_id, span);
            }

            previous_span = current_span;
        }
    }

    /// Return the contiguous trivia span immediately before one anchor.
    fn leading_comments_before(&self, boundary: u32, scope_start: u32) -> Option<Span> {
        let mut span_start = None;
        let mut span_end = None;

        // walk backward and keep only the immediate trivia suffix
        for token in self.tree.tokens().iter().rev() {
            if token.span.start < scope_start {
                break;
            }

            if token.span.end > boundary {
                continue;
            }

            if !token.is_trivia() {
                break;
            }

            span_start = Some(token.span.start);
            span_end.get_or_insert(token.span.end);
        }

        let span_start = span_start?;
        let span_end = span_end?;

        Some(Span::new(self.file_id, span_start, span_end))
    }

    /// Return the leading comment span inside one sibling gap.
    fn leading_comments_in_gap(&self, start: u32, end: u32) -> Option<Span> {
        let mut gap_tokens = Vec::new();

        // keep trivia inside the sibling gap
        for token in self.tree.tokens() {
            if token.span.end <= start {
                continue;
            }

            if token.span.start >= end {
                break;
            }

            if token.is_trivia() {
                gap_tokens.push(token);
            }
        }

        if gap_tokens.is_empty() {
            return None;
        }

        let mut leading_index = 0usize;

        // skip a same line trailing comment so it does not attach to the next anchor
        for (index, token) in gap_tokens.iter().enumerate() {
            match token.ty {
                TokenType::Whitespace => {}
                TokenType::Comment => {
                    leading_index = index + 1;
                    break;
                }
                TokenType::Newline => {
                    break;
                }
                _ => unreachable!("non trivia token stored inside MIR comment gap"),
            }
        }

        let span_start = gap_tokens.get(leading_index)?.span.start;
        let span_end = gap_tokens.last()?.span.end;

        Some(Span::new(self.file_id, span_start, span_end))
    }
}
