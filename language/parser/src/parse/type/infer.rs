use crate::{ParseResult, Parser};

use destack_dir::{Keyword, LocalNodeId, NodeType, TokenType, TypeExpression};

impl Parser {
    /// Eat one `infer` type expression.
    ///
    /// Examples:
    /// ```
    /// infer T
    /// infer T extends U
    /// infer T extends (U extends V ? X : Y)
    /// ```
    pub fn eat_type_infer_expression(&mut self) -> ParseResult<LocalNodeId<TypeExpression>> {
        // `infer T`
        let start = self.span_start();
        self.eat_keyword(Keyword::Infer)?;
        let (name, name_span) = self.eat_identifier_with_span()?;

        // constraint: `infer T extends U`
        let constraint = if self.is_keyword(Keyword::Extends) {
            let mark = self.checkpoint();
            let tree_mark = self.tree.next_id();

            self.bump(); // eat extends

            let mut constraint_flags = self
                .flags
                .not_in_position()
                .in_type()
                .disallow_type_conditional();
            if self.flags.is_in_type_conditional_right() {
                constraint_flags = constraint_flags.in_type_conditional_right();
            }

            let constraint = self.eat_type_expression_or_recover_missing(
                constraint_flags,
                NodeType::TypeExpression,
            )?;

            // `infer T extends U ? X : Y` belongs to the surrounding conditional type
            let has_conditional_marker = self.peek_is(TokenType::Maybe)
                || (self.current_token_is_on_new_line() && self.peek_is(TokenType::Maybe));
            if has_conditional_marker && !self.flags.is_disallow_type_conditional() {
                self.restore(mark, tree_mark);
                None
            } else {
                Some(constraint)
            }
        } else {
            None
        };

        // infer node
        let type_expression_id = self.insert_node(
            TypeExpression::Infer { name, constraint },
            self.get_span_from(&start),
        );
        self.tree.set_main_span(type_expression_id, name_span);

        Ok(type_expression_id)
    }
}
