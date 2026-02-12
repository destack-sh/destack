use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{Expression, LocalNodeId, NodeType, Path, PostfixPosition, TokenType};
use smallvec::smallvec;

impl Parser {
    pub(super) fn eat_identifier_expression_path(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _identifier_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_IDENTIFIER);

        // fast path: single segment identifiers dominate value expressions
        let next_token_type = self.peek_next_token_type();
        let (path, last_span) =
            if next_token_type != TokenType::Dot && next_token_type != TokenType::Newline {
                let (segment, segment_span) = self.eat_identifier_with_span()?;
                let path = Path {
                    segments: smallvec![segment],
                };
                (path, segment_span)
            } else {
                self.eat_path_with_last_span()
                    .for_node_type(NodeType::Expression)?
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
            self.eat_call(receiver_id, static_arguments, PostfixPosition::Direct)
        } else {
            let expression = Expression::Path {
                path,
                static_arguments,
            };
            let expression_id = self.tree.insert(expression, self.get_span_from(start));
            self.tree.set_main_span(expression_id, last_span);
            Ok(expression_id)
        }
    }
}
