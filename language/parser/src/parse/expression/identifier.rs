use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{Expression, LocalNodeId, NodeType, Path, PostfixPosition, TokenType};
use smallvec::smallvec;

use super::super::annotation::{AnnotationSeamKind, DotBoundaryKind};

impl Parser {
    pub(super) fn eat_identifier_expression_path(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _identifier_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_IDENTIFIER);
        let path_start_token_index = self.pos_index();

        // fast path: single segment identifiers dominate value expressions
        let next_token_type = self.peek_next_token_type();
        let (path, last_span, parsed_multi_segment_path) =
            if next_token_type != TokenType::Dot && next_token_type != TokenType::Newline {
                let (segment, segment_span) = self.eat_identifier_with_span()?;
                let path = Path {
                    segments: smallvec![segment],
                };
                (path, segment_span, false)
            } else {
                self.eat_path_with_last_span()
                    .for_node_type(NodeType::Expression)
                    .map(|(path, last_span)| (path, last_span, true))?
            };

        // speculatively unwrap postfix static parameterisation with `<` or `<<`
        //  (might also be just a comparison operator)
        //  `<<` (ShiftLeft) handles cases like `Extends<<T>() => ...>`
        let static_arguments = self.eat_static_arguments_in_expression(true);

        // immediately parse call if we have static arguments
        // (so we can stuff the arguments into the call expression)
        if static_arguments.is_some()
            && self.peek_is(TokenType::OpenParenthesis)
            && !self.options.in_new_receiver
        {
            let _call_timing = self.timing_scope(tags::PARSE_EXPRESSION_POSTFIX_CALL);
            let receiver = Expression::Path {
                path,
                static_arguments: None,
            };
            let receiver_id = self.tree.insert(receiver, self.get_span_from(start));
            self.tree.set_main_span(receiver_id, last_span);
            if parsed_multi_segment_path {
                self.attach_path_segment_boundary_annotations(
                    path_start_token_index,
                    self.pos_index(),
                    receiver_id.id,
                );
            }
            self.eat_call(receiver_id, static_arguments, PostfixPosition::Direct)
        } else {
            let expression = Expression::Path {
                path,
                static_arguments,
            };
            let expression_id = self.tree.insert(expression, self.get_span_from(start));
            self.tree.set_main_span(expression_id, last_span);
            if parsed_multi_segment_path {
                self.attach_path_segment_boundary_annotations(
                    path_start_token_index,
                    self.pos_index(),
                    expression_id.id,
                );
            }
            Ok(expression_id)
        }
    }

    /// Attach boundary annotations for dot-separated path segments.
    fn attach_path_segment_boundary_annotations(
        &mut self,
        start_token_index: usize,
        end_token_index: usize,
        target_node_id: u32,
    ) {
        let mut token_index = start_token_index.saturating_add(1);
        while token_index < end_token_index {
            if self.token_type_at(token_index) == TokenType::Dot {
                let mut skipped_newline_count = 0usize;
                let mut newline_index = token_index;
                while newline_index > 0
                    && self.token_type_at(newline_index.saturating_sub(1)) == TokenType::Newline
                {
                    skipped_newline_count += 1;
                    newline_index -= 1;
                }

                self.bind_annotation_seam(
                    token_index,
                    skipped_newline_count,
                    target_node_id,
                    AnnotationSeamKind::DotBoundary(DotBoundaryKind::Member),
                );
            }

            token_index += 1;
        }
    }
}
