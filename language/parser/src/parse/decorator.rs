use crate::{ParseResult, Parser};
use destack_ast::{Decorator, DecoratorPosition, LocalNodeId, TokenType};
use smallvec::SmallVec;

const DECORATOR_EXPRESSION_PRECEDENCE: u16 = u16::MAX;

/// A parsed decorator pending attachment to the next owner at this site.
#[derive(Debug, Copy, Clone)]
pub(crate) struct PendingDecorator {
    /// The parsed decorator node id.
    pub decorator_id: LocalNodeId<Decorator>,
}

/// A small pending decorator buffer for hot parse loops.
pub(crate) type PendingDecorators = SmallVec<[PendingDecorator; 2]>;

impl Parser {
    /// Parse a decorator prefix sequence if present at the current token.
    pub(crate) fn eat_decorators_maybe(&mut self) -> ParseResult<PendingDecorators> {
        let mut decorators = PendingDecorators::new();

        while self.peek_is(TokenType::At) {
            let start = self.span_start();
            let decorator = self.with_statement_recovery(
                &start,
                |parser| parser.eat_decorator().map(Some),
                None,
            );
            if let Some(decorator_id) = decorator {
                decorators.push(PendingDecorator { decorator_id });
            }

            // consume trailing newlines between decorator entries
        }

        Ok(decorators)
    }

    /// Attach decorators to a parsed owner node in source order.
    pub(crate) fn attach_decorators(&mut self, target_node_id: u32, decorators: PendingDecorators) {
        if decorators.is_empty() {
            return;
        }

        for pending in decorators {
            self.attach_decorator(target_node_id, pending.decorator_id);
        }
    }

    /// Parse one decorator expression.
    fn eat_decorator(&mut self) -> ParseResult<LocalNodeId<Decorator>> {
        let start = self.span_start();

        // eat @ marker
        self.eat_token(TokenType::At)?;

        // decorators always parse as value expressions
        let mut decorator_flags = self
            .flags
            .not_in_position()
            .in_left_precedence(DECORATOR_EXPRESSION_PRECEDENCE)
            .not_in_sequence_expression()
            .in_decorator();
        decorator_flags.set_in_type(false);
        decorator_flags.set_in_static(false);
        decorator_flags.set_in_super_type(false);
        decorator_flags.set_in_before_type(false);
        decorator_flags.set_in_type_conditional_right(false);
        decorator_flags.set_in_type_mapped_constraint(false);

        // parse decorator target expression
        let expression = self.eat_expression(decorator_flags)?;

        // store decorator side node
        let decorator = self.tree.insert(
            Decorator {
                expression,
                position: DecoratorPosition::BlockPrefix,
            },
            self.get_span_from(&start),
        );
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
        self.tree.append_decorator(target_node_id, decorator_id);
    }
}
