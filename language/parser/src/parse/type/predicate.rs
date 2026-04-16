use crate::{ParseResult, Parser};

use destack_ast::{Keyword, LocalNodeId, NodeType, TypeExpression, TypePredicateSubject};

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
        let start = self.mark_span();
        self.eat_keyword(Keyword::Asserts)?;
        self.eat_newlines_maybe()?;

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
        let target = if self.is_keyword(Keyword::Is) {
            self.bump(); // eat is
            self.eat_newlines_maybe()?;

            let mut target_options = self.options.not_in_position().in_type();
            if self.options.is_in_type_conditional_right() {
                target_options = target_options.in_type_conditional_right();
            }

            let target = self
                .eat_type_expression_or_recover_missing(target_options, NodeType::TypeExpression)?;
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

        Ok(type_expression_id)
    }
}
