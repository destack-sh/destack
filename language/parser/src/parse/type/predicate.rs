use crate::{ParseError, ParseResult, Parser};

use destack_dir::{
    Keyword, LocalNodeId, NodeType, TokenType, TypeExpression, TypePredicateSubject,
};
use destack_source::{NodeSpanBoundary, NodeSpanRegion, NodeSpanType, Span};

/// Parsed type predicate target.
struct TypePredicateTarget {
    /// The optional `is` operator span.
    operator_span: Option<Span>,
    /// The optional target type.
    target: Option<LocalNodeId<TypeExpression>>,
}

impl Parser {
    /// Eat one `asserts` type predicate.
    ///
    /// Examples:
    /// ```ds
    /// asserts value
    /// asserts value is string
    /// asserts this is ReadyState
    /// ```
    pub fn eat_type_predicate_asserts(&mut self) -> ParseResult<LocalNodeId<TypeExpression>> {
        let start = self.span_start();
        self.eat_keyword(Keyword::Asserts)?;

        let subject_is_on_new_line = self.current_token_is_on_new_line();
        let (subject, subject_span) = self.eat_type_predicate_subject()?;
        let target = self.eat_type_predicate_target(subject_is_on_new_line)?;

        // predicate node
        let type_expression_id = self.insert_node(
            TypeExpression::Predicate {
                asserts: true,
                subject,
                target: target.target,
            },
            self.get_span_from(&start),
        );
        self.tree.set_main_span(type_expression_id, subject_span);
        if let Some(operator_span) = target.operator_span {
            self.tree.set_side_span(
                type_expression_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                operator_span,
            );
        }

        Ok(type_expression_id)
    }

    /// Eat a type predicate subject.
    ///
    /// Examples:
    /// ```ds
    /// value
    /// this
    /// subject
    /// ```
    fn eat_type_predicate_subject(&mut self) -> ParseResult<(TypePredicateSubject, Span)> {
        if self.is_keyword(Keyword::This) {
            let token = *self.peek()?;
            self.bump();
            return Ok((TypePredicateSubject::This, token.span));
        }

        let (name, span) = self.eat_identifier_with_span()?;

        Ok((TypePredicateSubject::Identifier(name), span))
    }

    /// Eat a type predicate target.
    ///
    /// Examples:
    /// ```ds
    /// is string
    /// is Ready | Error
    /// is { id: string }
    /// ```
    fn eat_type_predicate_target(
        &mut self,
        subject_is_on_new_line: bool,
    ) -> ParseResult<TypePredicateTarget> {
        if subject_is_on_new_line && self.is_keyword(Keyword::Is) {
            self.report_misplaced_type_predicate_target()?;
            return Ok(TypePredicateTarget {
                operator_span: None,
                target: None,
            });
        }

        if self.current_token_is_on_new_line() || !self.is_keyword(Keyword::Is) {
            return Ok(TypePredicateTarget {
                operator_span: None,
                target: None,
            });
        }

        let operator_span = self.eat_keyword(Keyword::Is)?.span;
        let target = self.eat_type_predicate_target_type(operator_span)?;

        Ok(TypePredicateTarget {
            operator_span: Some(operator_span),
            target: Some(target),
        })
    }

    /// Report a misplaced predicate target after a newline.
    fn report_misplaced_type_predicate_target(&mut self) -> ParseResult<()> {
        let is_span = self.eat_keyword(Keyword::Is)?.span;
        self.error(&ParseError::unexpected(is_span));

        if !Self::is_type_expression_boundary_token(self.peek_token_type()) {
            let target = self.eat_type_expression()?;
            self.error(&ParseError::unexpected(self.tree.get_span(target)));
        }

        if self.peek_is(TokenType::OpenBrace) {
            let brace_span = self.peek()?.span;
            self.error(&ParseError::unexpected(brace_span));
        }

        Ok(())
    }

    /// Eat a type predicate target type.
    ///
    /// Examples:
    /// ```ds
    /// string
    /// Ready | Error
    /// { id: string }
    /// ```
    fn eat_type_predicate_target_type(
        &mut self,
        operator_span: Span,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        let target_flags = self.type_nested_flags();

        let target =
            self.eat_type_expression_or_recover_missing(target_flags, NodeType::TypeExpression)?;
        let target_span = self.tree.get_span(target);
        self.tree.set_side_span(
            target,
            NodeSpanType::Boundary(NodeSpanBoundary::Leading),
            Span::new(target_span.file, operator_span.end, target_span.start),
        );

        Ok(target)
    }
}
