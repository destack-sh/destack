use crate::Parser;
use crate::parse::parser::NonNewlineTokenCursor;

use super::{
    AnnotationSeamKind, BlankBoundaryKind, LeadingAnnotationKind, PendingDecorators,
    TrailingAnnotationKind,
};

impl Parser {
    /// Bind one owner-leading seam at a cursor boundary.
    #[inline]
    pub(crate) fn bind_owner_leading_seam_at_cursor(
        &mut self,
        owner_node_id: u32,
        cursor: NonNewlineTokenCursor,
        leading_kind: LeadingAnnotationKind,
    ) {
        self.bind_annotation_seam(
            cursor.index,
            cursor.skipped_newline_count,
            owner_node_id,
            AnnotationSeamKind::Leading(leading_kind),
        );
    }

    /// Bind statement and wrapper leading seams at one cursor boundary.
    #[inline]
    pub(crate) fn bind_owner_statement_and_wrapper_leading_seams(
        &mut self,
        owner_node_id: u32,
        cursor: NonNewlineTokenCursor,
    ) {
        self.bind_owner_leading_seam_at_cursor(
            owner_node_id,
            cursor,
            LeadingAnnotationKind::Statement,
        );
        self.bind_owner_leading_seam_at_cursor(
            owner_node_id,
            cursor,
            LeadingAnnotationKind::Wrapper,
        );
    }

    /// Bind one wrapper leading seam at an explicit token boundary.
    #[inline]
    pub(crate) fn bind_owner_wrapper_leading_seam_at_token(
        &mut self,
        owner_node_id: u32,
        token_index: usize,
        skipped_newline_count: usize,
    ) {
        self.bind_owner_leading_seam_at_token(
            owner_node_id,
            token_index,
            skipped_newline_count,
            LeadingAnnotationKind::Wrapper,
        );
    }

    /// Bind one owner-leading seam at an explicit token boundary.
    #[inline]
    pub(crate) fn bind_owner_leading_seam_at_token(
        &mut self,
        owner_node_id: u32,
        token_index: usize,
        skipped_newline_count: usize,
        leading_kind: LeadingAnnotationKind,
    ) {
        self.bind_annotation_seam(
            token_index,
            skipped_newline_count,
            owner_node_id,
            AnnotationSeamKind::Leading(leading_kind),
        );
    }

    /// Bind owner-leading seams and attach pending decorators in source order.
    #[inline]
    pub(crate) fn bind_prefixed_owner_with_decorators(
        &mut self,
        owner_node_id: u32,
        owner_cursor: NonNewlineTokenCursor,
        leading_kind: LeadingAnnotationKind,
        pending_decorators: &mut PendingDecorators,
    ) {
        if pending_decorators.is_empty() {
            self.bind_owner_leading_seam_at_cursor(owner_node_id, owner_cursor, leading_kind);
            return;
        }

        let decorators = std::mem::take(pending_decorators);
        for pending in decorators {
            self.bind_annotation_seam(
                pending.cursor.token_index,
                pending.cursor.skipped_newline_count,
                owner_node_id,
                AnnotationSeamKind::Leading(leading_kind),
            );
            self.attach_decorator(owner_node_id, pending.decorator_id);
        }

        self.bind_owner_leading_seam_at_cursor(owner_node_id, owner_cursor, leading_kind);
    }

    /// Bind one trailing seam on the parser current boundary.
    #[inline]
    pub(crate) fn bind_owner_trailing_seam_at_current(
        &mut self,
        owner_node_id: u32,
        trailing_kind: TrailingAnnotationKind,
    ) {
        self.bind_owner_trailing_seam_at_token(owner_node_id, self.pos_index(), 0, trailing_kind);
    }

    /// Bind one trailing seam at an explicit token boundary.
    #[inline]
    pub(crate) fn bind_owner_trailing_seam_at_token(
        &mut self,
        owner_node_id: u32,
        token_index: usize,
        skipped_newline_count: usize,
        trailing_kind: TrailingAnnotationKind,
    ) {
        self.bind_annotation_seam(
            token_index,
            skipped_newline_count,
            owner_node_id,
            AnnotationSeamKind::Trailing(trailing_kind),
        );
    }

    /// Bind one trailing default seam on the current boundary.
    #[inline]
    pub(crate) fn bind_owner_trailing_default_at_current(&mut self, owner_node_id: u32) {
        self.bind_owner_trailing_seam_at_current(owner_node_id, TrailingAnnotationKind::Default);
    }

    /// Bind one trailing default seam at an explicit token boundary.
    #[inline]
    pub(crate) fn bind_owner_trailing_default_at_token(
        &mut self,
        owner_node_id: u32,
        token_index: usize,
        skipped_newline_count: usize,
    ) {
        self.bind_owner_trailing_seam_at_token(
            owner_node_id,
            token_index,
            skipped_newline_count,
            TrailingAnnotationKind::Default,
        );
    }

    /// Bind one trailing line boundary seam on the current boundary.
    #[inline]
    pub(crate) fn bind_owner_trailing_line_at_current(&mut self, owner_node_id: u32) {
        self.bind_current_annotation_seam(owner_node_id, AnnotationSeamKind::TrailingLineBoundary);
    }

    /// Bind trailing line boundary and default trailing seams on the current boundary.
    #[inline]
    pub(crate) fn bind_owner_trailing_line_and_default_at_current(&mut self, owner_node_id: u32) {
        self.bind_owner_trailing_line_at_current(owner_node_id);
        self.bind_owner_trailing_default_at_current(owner_node_id);
    }

    /// Bind trailing line boundary and default trailing seams on an explicit cursor boundary.
    #[inline]
    pub(crate) fn bind_owner_trailing_line_and_default_at_cursor(
        &mut self,
        owner_node_id: u32,
        cursor: NonNewlineTokenCursor,
    ) {
        self.bind_owner_trailing_line_and_default_at_token(
            owner_node_id,
            cursor.index,
            cursor.skipped_newline_count,
        );
    }

    /// Bind trailing line boundary and default trailing seams at an explicit token boundary.
    #[inline]
    pub(crate) fn bind_owner_trailing_line_and_default_at_token(
        &mut self,
        owner_node_id: u32,
        token_index: usize,
        skipped_newline_count: usize,
    ) {
        self.bind_annotation_seam(
            token_index,
            skipped_newline_count,
            owner_node_id,
            AnnotationSeamKind::TrailingLineBoundary,
        );
        self.bind_annotation_seam(
            token_index,
            skipped_newline_count,
            owner_node_id,
            AnnotationSeamKind::Trailing(TrailingAnnotationKind::Default),
        );
    }

    /// Bind owner-leading seams with decorators and one trailing seam.
    #[inline]
    pub(crate) fn bind_prefixed_owner_with_decorators_and_trailing_at_current(
        &mut self,
        owner_node_id: u32,
        owner_cursor: NonNewlineTokenCursor,
        leading_kind: LeadingAnnotationKind,
        trailing_kind: TrailingAnnotationKind,
        pending_decorators: &mut PendingDecorators,
    ) {
        self.bind_prefixed_owner_with_decorators(
            owner_node_id,
            owner_cursor,
            leading_kind,
            pending_decorators,
        );
        self.bind_owner_trailing_seam_at_current(owner_node_id, trailing_kind);
    }

    /// Bind close-boundary blank and trailing seams for one owner.
    #[inline]
    pub(crate) fn bind_owner_close_seams(
        &mut self,
        owner_node_id: u32,
        close_cursor: NonNewlineTokenCursor,
        blank_kind: BlankBoundaryKind,
        trailing_kind: TrailingAnnotationKind,
    ) {
        self.bind_annotation_seam(
            close_cursor.index,
            close_cursor.skipped_newline_count,
            owner_node_id,
            AnnotationSeamKind::Blank(blank_kind),
        );
        self.bind_annotation_seam(
            close_cursor.index,
            0,
            owner_node_id,
            AnnotationSeamKind::Trailing(trailing_kind),
        );
    }

    /// Bind statement-leading seams and attach pending decorators in source order.
    #[inline]
    pub(crate) fn bind_statement_owner_with_decorators(
        &mut self,
        owner_node_id: u32,
        owner_cursor: NonNewlineTokenCursor,
        pending_decorators: &mut PendingDecorators,
    ) {
        self.bind_prefixed_owner_with_decorators(
            owner_node_id,
            owner_cursor,
            LeadingAnnotationKind::Statement,
            pending_decorators,
        );
    }

    /// Bind statement-leading seams with decorators and default trailing at current boundary.
    #[inline]
    pub(crate) fn bind_statement_owner_with_decorators_and_default_trailing_at_current(
        &mut self,
        owner_node_id: u32,
        owner_cursor: NonNewlineTokenCursor,
        pending_decorators: &mut PendingDecorators,
    ) {
        self.bind_prefixed_owner_with_decorators_and_trailing_at_current(
            owner_node_id,
            owner_cursor,
            LeadingAnnotationKind::Statement,
            TrailingAnnotationKind::Default,
            pending_decorators,
        );
    }

    /// Bind close boundary seams using default postfix ownership.
    #[inline]
    pub(crate) fn bind_owner_close_postfix_default_seams(
        &mut self,
        owner_node_id: u32,
        close_cursor: NonNewlineTokenCursor,
    ) {
        self.bind_owner_close_seams(
            owner_node_id,
            close_cursor,
            BlankBoundaryKind::Postfix,
            TrailingAnnotationKind::Default,
        );
    }

    /// Bind close boundary seams with postfix blank, trailing line, and default trailing ownership.
    #[inline]
    pub(crate) fn bind_owner_close_postfix_trailing_line_and_default_seams(
        &mut self,
        owner_node_id: u32,
        close_cursor: NonNewlineTokenCursor,
    ) {
        self.bind_annotation_seam(
            close_cursor.index,
            close_cursor.skipped_newline_count,
            owner_node_id,
            AnnotationSeamKind::Blank(BlankBoundaryKind::Postfix),
        );
        self.bind_owner_trailing_line_and_default_at_current(owner_node_id);
    }
}
