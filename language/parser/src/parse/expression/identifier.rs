use crate::parse::prelude::*;
use crate::{ParseResult, Parser, ParserMark};

use destack_ast::{Expression, LocalNodeId, NodeType, Path, TokenType};
use smallvec::smallvec;

impl Parser {
    pub(super) fn eat_identifier_expression_path(
        &mut self,
        start: &ParserMark,
    ) -> ParseResult<LocalNodeId<Expression>> {
        let _identifier_timing = self.timing_scope(tags::PARSE_EXPRESSION_PRIMARY_IDENTIFIER);
        // static and type contexts use qualified references
        if self.options.is_in_type() || self.options.is_in_static() {
            let next_token_type = self.peek_next_token_type();
            let (path, segment_spans, _last_span) =
                if next_token_type != TokenType::Dot && next_token_type != TokenType::Newline {
                    let (segment, segment_span) = self.eat_identifier_with_span()?;
                    let path = Path {
                        segments: smallvec![segment],
                    };
                    (path, smallvec![segment_span], segment_span)
                } else {
                    self.eat_path_with_endpoint_spans()
                        .for_node_type(NodeType::Expression)?
                };

            // speculatively unwrap postfix static parameterisation with `<` or `<<`
            //  (might also be just a comparison operator)
            //  `<<` (ShiftLeft) handles cases like `Extends<<T>() => ...>`
            let static_arguments =
                if self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft) {
                    self.eat_static_arguments_in_expression(true)
                } else {
                    None
                };

            let expression = Expression::QualifiedReference {
                path,
                static_arguments,
            };
            let expression_id = self.insert_node(expression, self.get_span_from(start));
            self.set_path_expression_spans(expression_id, &segment_spans);
            return Ok(expression_id);
        }

        // value contexts start as bare identifiers and let postfix parsing build
        // real member, call, and instantiation chains
        let (name, name_span) = self.eat_identifier_with_span()?;
        let expression = Expression::Identifier { name };
        let expression_id = self.insert_node(expression, self.get_span_from(start));
        self.tree.set_main_span(expression_id, name_span);

        Ok(expression_id)
    }
}
