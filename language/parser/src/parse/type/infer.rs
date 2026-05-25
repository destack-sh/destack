use crate::{Parser, ParserResult};

use destack_dir::{InferForm, Keyword, LocalNodeId, NodeType, TokenType, TypeExpression};

impl Parser {
    /// Eat one `infer` type expression.
    ///
    /// Examples:
    /// ```ds
    /// infer T
    /// infer T extends U
    /// infer T extends (U extends V ? X : Y)
    /// ```
    pub fn eat_type_infer_expression(&mut self) -> ParserResult<LocalNodeId<TypeExpression>> {
        // `infer T`
        let start = self.span_start();
        self.eat_keyword(Keyword::Infer)?;
        let (name, name_span) = self.eat_infer_binding()?;
        let constraint = self.eat_infer_constraint()?;

        // infer node
        let type_expression_id = self.insert_node(
            TypeExpression::Infer {
                form: InferForm::Infer,
                name,
                constraint,
            },
            self.get_span_from(&start),
        );
        self.tree.set_main_span(type_expression_id, name_span);

        Ok(type_expression_id)
    }

    /// Eat an infer binding name.
    ///
    /// Examples:
    /// ```ds
    /// T
    /// _
    /// Result
    /// ```
    fn eat_infer_binding(
        &mut self,
    ) -> ParserResult<(Option<destack_core::StringId>, destack_source::Span)> {
        let (name, name_span) = self.eat_identifier_with_span()?;
        if self.language.is_destack() && self.get_span_str(name_span) == "_" {
            return Ok((None, name_span));
        }

        Ok((Some(name), name_span))
    }

    /// Eat an infer constraint when present.
    ///
    /// Examples:
    /// ```ds
    /// extends string
    /// extends keyof T
    /// extends { id: string }
    /// ```
    fn eat_infer_constraint(&mut self) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        if !self.is_keyword(Keyword::Extends) {
            return Ok(None);
        }

        let checkpoint = self.checkpoint();
        let mark = self.tree.next_id();
        self.bump();

        let constraint = self.eat_infer_constraint_type()?;
        if self.infer_extends_is_conditional_boundary() {
            self.restore(checkpoint, mark);
            return Ok(None);
        }

        Ok(Some(constraint))
    }

    /// Eat the type after `infer T extends`.
    ///
    /// Examples:
    /// ```ds
    /// string
    /// readonly string[]
    /// T extends U ? A : B
    /// ```
    fn eat_infer_constraint_type(&mut self) -> ParserResult<LocalNodeId<TypeExpression>> {
        let mut flags = self
            .flags
            .not_in_position()
            .in_type()
            .disallow_type_conditional();
        if self.flags.is_in_type_conditional_right() {
            flags = flags.in_type_conditional_right();
        }

        self.eat_type_expression_or_recover_missing(flags, NodeType::TypeExpression)
    }

    /// Return whether `extends` should stay with the surrounding conditional type.
    fn infer_extends_is_conditional_boundary(&mut self) -> bool {
        !self.flags.is_disallow_type_conditional()
            && (self.peek_is(TokenType::Maybe)
                || self.current_token_is_on_new_line() && self.peek_is(TokenType::Maybe))
    }
}
