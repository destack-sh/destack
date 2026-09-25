use crate::parse::lookahead::DelimiterDepth;
use crate::parse::{TypePosition, TypeStop};
use crate::{Parser, ParserResult, TokenProbe};

use tspp_core::StringId;
use tspp_dir::{InferForm, Keyword, LocalNodeId, NodeType, TokenType, TypeExpression};
use tspp_source::ByteRange;

impl Parser {
    /// Parse one `infer` type expression.
    ///
    /// Examples:
    /// ```tspp
    /// infer T
    /// infer T extends U
    /// infer T extends (U extends V ? X : Y)
    /// ```
    pub(crate) fn parse_type_infer(
        &mut self,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        // `infer T`
        let start = self.mark_parse_start();
        self.eat_keyword(Keyword::Infer)?;
        let (name, name_range) = self.parse_infer_binding()?;
        let constraint = self.parse_infer_constraint(stop)?;

        // infer node
        let type_expression_id = self.insert_node(
            TypeExpression::Infer {
                form: InferForm::Infer,
                name,
                constraint,
            },
            self.range_since(&start),
        );
        self.tree.set_main_range(type_expression_id, name_range);

        Ok(type_expression_id)
    }

    /// Parse an infer binding name.
    ///
    /// Examples:
    /// ```tspp
    /// T
    /// _
    /// Result
    /// ```
    fn parse_infer_binding(&mut self) -> ParserResult<(Option<StringId>, ByteRange)> {
        let (name, name_range) = self.eat_identifier_with_range()?;
        if self.range_str(name_range) == "_" {
            return Ok((None, name_range));
        }

        Ok((Some(name), name_range))
    }

    /// Parse an infer constraint when present.
    ///
    /// Examples:
    /// ```tspp
    /// extends string
    /// extends keyof T
    /// extends { id: string }
    /// ```
    fn parse_infer_constraint(
        &mut self,
        stop: TypeStop,
    ) -> ParserResult<Option<LocalNodeId<TypeExpression>>> {
        if !self.peek_is_keyword(Keyword::Extends) {
            return Ok(None);
        }
        if !stop.has(TypeStop::INFER_CONSTRAINT) && self.peek_infer_conditional() {
            return Ok(None);
        }

        self.bump();
        let constraint = self.parse_infer_constraint_type(stop)?;

        Ok(Some(constraint))
    }

    /// Parse the type after `infer T extends`.
    ///
    /// Examples:
    /// ```tspp
    /// string
    /// readonly string[]
    /// T extends U ? A : B
    /// ```
    fn parse_infer_constraint_type(
        &mut self,
        stop: TypeStop,
    ) -> ParserResult<LocalNodeId<TypeExpression>> {
        let stop = stop.nest().add(TypeStop::RELATION);

        self.parse_type_or_recover_missing(TypePosition::Type, stop, NodeType::TypeExpression)
    }

    /// Return whether this `extends` is followed by a top-level conditional question.
    fn peek_infer_conditional(&self) -> bool {
        let mut probe = self.cursor.probe(&self.file);
        probe.bump();

        probe.scan_conditional_type_question()
    }
}

impl TokenProbe<'_> {
    /// Return whether this type scan reaches a top-level conditional question.
    fn scan_conditional_type_question(&mut self) -> bool {
        let mut depth = DelimiterDepth::type_expression();

        loop {
            let token_type = self.peek_token_type();
            if depth.is_top_level() && token_type == TokenType::Maybe {
                return true;
            }
            if depth.is_top_level()
                && matches!(
                    token_type,
                    TokenType::Comma
                        | TokenType::Semicolon
                        | TokenType::CloseParenthesis
                        | TokenType::CloseBracket
                        | TokenType::CloseBrace
                        | TokenType::TemplateStringMiddle
                        | TokenType::TemplateStringEnd
                        | TokenType::End
                )
            {
                return false;
            }
            if !depth.advance(token_type) {
                return false;
            }

            self.bump();
        }
    }
}
