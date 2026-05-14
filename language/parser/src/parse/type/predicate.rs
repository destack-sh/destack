use crate::{ParseResult, Parser};

use destack_dir::{Keyword, LocalNodeId, NodeType, TypeExpression, TypePredicateSubject};
use destack_source::{NodeSpanBoundary, NodeSpanRegion, NodeSpanType, Span};

impl Parser {
    /// Eat one `asserts` type predicate.
    ///
    /// Examples:
    /// ```
    /// asserts value
    /// asserts value is string
    /// asserts this is ReadyState
    /// ```
    pub fn eat_type_predicate_asserts(&mut self) -> ParseResult<LocalNodeId<TypeExpression>> {
        let start = self.span_start();
        self.eat_keyword(Keyword::Asserts)?;

        // subject: `this` or one identifier
        let (subject, subject_span) = if self.is_keyword(Keyword::This) {
            let token = *self.peek()?;
            self.bump(); // eat this
            (TypePredicateSubject::This, token.span)
        } else {
            let (name, span) = self.eat_identifier_with_span()?;
            (TypePredicateSubject::Identifier(name), span)
        };

        // target: `asserts x is T`
        let mut operator_span = None;
        let target = if self.is_keyword(Keyword::Is) {
            let is_span = self.eat_keyword(Keyword::Is)?.span;
            operator_span = Some(is_span);

            let mut target_flags = self.flags.not_in_position().in_type();
            if self.flags.is_in_type_conditional_right() {
                target_flags = target_flags.in_type_conditional_right();
            }

            let target = self
                .eat_type_expression_or_recover_missing(target_flags, NodeType::TypeExpression)?;
            let target_span = self.tree.get_span(target);
            self.tree.set_side_span(
                target,
                NodeSpanType::Boundary(NodeSpanBoundary::Leading),
                Span::new(target_span.file, is_span.end, target_span.start),
            );
            Some(target)
        } else {
            None
        };

        // predicate node
        let type_expression_id = self.insert_node(
            TypeExpression::Predicate {
                asserts: true,
                subject,
                target,
            },
            self.get_span_from(&start),
        );
        self.tree.set_main_span(type_expression_id, subject_span);
        if let Some(operator_span) = operator_span {
            self.tree.set_side_span(
                type_expression_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                operator_span,
            );
        }

        Ok(type_expression_id)
    }
}
