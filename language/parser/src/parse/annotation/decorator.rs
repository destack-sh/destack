use crate::{ParseResult, Parser};
use destack_ast::{Annotation, AnnotationPosition, Decorator, LocalNodeId, TokenType};
use smallvec::SmallVec;

const DECORATOR_EXPRESSION_PRECEDENCE: u16 = u16::MAX;

/// The normalized cursor where a decorator prefix starts.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct DecoratorCursor {
    /// The semantic token index for this prefix.
    pub token_index: usize,
    /// The number of skipped semantic newline tokens.
    pub skipped_newline_count: usize,
}

/// A parsed decorator pending attachment to the next owner at this site.
#[derive(Debug, Copy, Clone)]
pub(crate) struct PendingDecorator {
    /// The parsed decorator node id.
    pub decorator_id: LocalNodeId<Decorator>,
    /// The prefix cursor where this decorator began.
    pub cursor: DecoratorCursor,
}

/// A small pending decorator buffer for hot parse loops.
pub(crate) type PendingDecorators = SmallVec<[PendingDecorator; 2]>;

impl Parser {
    /// Parse a decorator prefix sequence if present at the current token.
    pub(crate) fn eat_decorators_maybe(&mut self) -> ParseResult<PendingDecorators> {
        let mut decorators = PendingDecorators::new();

        while self.peek_is(TokenType::At) {
            let decorator_cursor = self.peek_cursor();
            let start = self.mark_span();
            let decorator = self.with_recovery(
                &start,
                |parser| parser.eat_decorator().map(Some),
                None,
                TokenType::Newline,
            );
            if let Some(decorator_id) = decorator {
                decorators.push(PendingDecorator {
                    decorator_id,
                    cursor: DecoratorCursor {
                        token_index: decorator_cursor.index,
                        skipped_newline_count: decorator_cursor.skipped_newline_count,
                    },
                });
            }

            // consume trailing newlines between decorator entries
            self.eat_newlines_maybe()?;
        }

        Ok(decorators)
    }

    /// Attach decorators to a parsed owner node in source order.
    pub(crate) fn attach_decorators(&mut self, target_node_id: u32, decorators: PendingDecorators) {
        for pending in decorators {
            self.attach_decorator(target_node_id, pending.decorator_id);
        }
    }

    /// Parse one decorator expression.
    fn eat_decorator(&mut self) -> ParseResult<LocalNodeId<Decorator>> {
        let start = self.mark_span();

        // eat @ marker
        self.eat_token(TokenType::At)?;

        // decorators always parse as value expressions
        let mut decorator_options = self
            .options
            .not_in_position()
            .in_left_precedence(DECORATOR_EXPRESSION_PRECEDENCE)
            .not_in_sequence_expression()
            .in_decorator();
        decorator_options.in_type = false;
        decorator_options.in_static = false;
        decorator_options.in_super_type = false;
        decorator_options.in_before_type = false;
        decorator_options.in_type_conditional_right = false;
        decorator_options.in_type_mapped_constraint = false;

        // parse decorator target expression
        let expression = self.eat_expression(decorator_options)?;

        // store decorator side node
        let decorator = self
            .tree
            .insert(Decorator { expression }, self.get_span_from(&start));
        let main_span = self
            .tree
            .get_main_span(expression)
            .unwrap_or_else(|| self.tree.get_span(expression));
        self.tree.set_main_span(decorator, main_span);
        Ok(decorator)
    }

    /// Attach one parsed decorator onto a parsed target.
    pub(crate) fn attach_decorator(
        &mut self,
        target_node_id: u32,
        decorator_id: LocalNodeId<Decorator>,
    ) {
        let span = self.tree.get_span(decorator_id);
        let annotation_id = self.tree.insert(
            Annotation::Decorator {
                node: decorator_id,
                position: AnnotationPosition::BlockPrefix,
            },
            span,
        );
        self.tree.append_annotation(target_node_id, annotation_id);
    }
}
