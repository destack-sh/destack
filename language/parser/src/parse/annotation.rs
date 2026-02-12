use crate::{ParseResult, Parser};
use destack_ast::{Annotation, AnnotationPosition, Decorator, LocalNodeId, TokenType};

const DECORATOR_EXPRESSION_PRECEDENCE: u16 = u16::MAX;

impl Parser {
    /// Eat any leading decorators and leave the parser at the next token.
    pub(crate) fn eat_decorators_prefix_maybe(&mut self) -> ParseResult<()> {
        let _decorators = self.eat_decorators_prefix_collect_maybe()?;
        Ok(())
    }

    /// Eat any leading decorators and return collected decorator ids.
    pub(crate) fn eat_decorators_prefix_collect_maybe(
        &mut self,
    ) -> ParseResult<Vec<LocalNodeId<Decorator>>> {
        let mut decorators = Vec::new();
        while self.peek_is(TokenType::At) {
            let start = self.mark_span();
            let decorator = self.with_recovery(
                &start,
                |parser| parser.eat_decorator().map(Some),
                None,
                TokenType::Newline,
            );
            if let Some(decorator) = decorator {
                decorators.push(decorator);
            }

            // consume trailing newlines between decorator entries
            self.eat_newlines_maybe()?;
        }

        Ok(decorators)
    }

    /// Eat one decorator expression.
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

    /// Attach decorator nodes to a parsed target.
    pub(crate) fn attach_decorators_to_target(
        &mut self,
        decorators: Vec<LocalNodeId<Decorator>>,
        target_node_id: u32,
    ) {
        for decorator_id in decorators {
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
}
