use crate::Parser;

use destack_ast::{
    Declaration, Expression, FunctionDeclaration, FunctionKind, GenericArgument, Keyword,
    LocalNodeId, TokenType, TypeExpression,
};

impl Parser {
    /// Check whether a generic argument list can be followed by a specific token.
    pub(crate) fn can_follow_generic_arguments_at_index(
        &mut self,
        index: usize,
        allow_statement_keyword: bool,
    ) -> bool {
        let token_type = self.token_type_at(index);

        // allow end and generic closers
        if token_type == TokenType::End {
            return true;
        }
        if self.options.is_in_static() && Self::starts_type_angle_close(token_type) {
            return true;
        }

        // allow ternary and arrow continuations
        if self.options.is_in_ternary_condition() && token_type == TokenType::Colon {
            return true;
        }
        if self.options.is_in_type()
            && matches!(token_type, TokenType::Arrow | TokenType::ArrowWide)
        {
            return true;
        }

        // allow statement-start keywords after generic arguments in `new` receivers
        if allow_statement_keyword
            && token_type == TokenType::Identifier
            && self.keyword_for_index(index).is_some()
        {
            return true;
        }

        // allow stops and delimiters
        if matches!(
            token_type,
            TokenType::Comma | TokenType::Semicolon | TokenType::Newline | TokenType::End
        ) {
            return true;
        }
        if Self::is_close_delimiter_token(token_type) {
            return true;
        }
        if matches!(
            token_type,
            TokenType::OpenParenthesis | TokenType::OpenBracket | TokenType::Dot
        ) {
            return true;
        }
        if token_type == TokenType::Maybe {
            return true;
        }
        if matches!(
            token_type,
            TokenType::TemplateString | TokenType::TemplateStringStart
        ) {
            return true;
        }

        // allow heritage terminators after generic arguments
        if self.options.is_in_super_type() {
            if token_type == TokenType::OpenBrace {
                return true;
            }
            if token_type == TokenType::Identifier
                && matches!(
                    self.keyword_for_index(index),
                    Some(Keyword::Implements | Keyword::With | Keyword::Where)
                )
            {
                return true;
            }
        }

        if self.has_infix_or_assign_operator_at_index(index) {
            return true;
        }

        false
    }

    /// Speculatively eat one generic argument list at one grammar site.
    pub(crate) fn try_eat_generic_arguments(
        &mut self,
        allow_object_literal: bool,
        allow_newline_prefix: bool,
    ) -> Option<Vec<LocalNodeId<GenericArgument>>> {
        // javascript modes do not support generic arguments
        if self.language.is_javascript()
            && !self.options.is_in_type()
            && !self.options.is_in_decorator()
        {
            return None;
        }

        // `<...>` at the current position, or after a newline in `new` receivers
        let start_cursor = allow_newline_prefix.then(|| self.scanner_cursor_from(self.pos_index()));
        let has_generic_argument_start =
            if self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft) {
                true
            } else if let Some(cursor) = start_cursor {
                cursor.has_line_break_before
                    && self.with_pos(cursor.index, |parser| {
                        parser.peek_is(TokenType::LessThan) || parser.peek_is(TokenType::ShiftLeft)
                    })
            } else {
                false
            };
        if !has_generic_argument_start {
            return None;
        }

        // speculative parse: restore on invalid follow tokens or shift expressions
        let speculative_start = self.mark();
        let speculative_start_idx = self.tree.next_id();

        // align to the normalized `<...>` start before parsing
        if let Some(cursor) = start_cursor
            && cursor.index != self.pos_index()
        {
            self.advance_to(cursor.index);
        }

        // `<<...>` starts need an extra value-position admissibility check
        let used_shift_left_start = self.peek_is(TokenType::ShiftLeft);
        match self.eat_generic_arguments() {
            Ok(generic_arguments) => {
                // in type or decorator context, type arguments are always valid
                if self.options.is_in_type() || self.options.is_in_decorator() {
                    return Some(generic_arguments);
                }

                // value expressions only accept `<<...>` when the parsed payload is a generic arrow
                if used_shift_left_start {
                    let starts_generic_lambda = generic_arguments.first().is_some_and(|argument| {
                        let GenericArgument::Positional { value } = self.tree.get(*argument) else {
                            return false;
                        };
                        let Expression::Type { value } = self.tree.get(*value) else {
                            return false;
                        };
                        let TypeExpression::Declaration { declaration } = self.tree.get(*value)
                        else {
                            return false;
                        };
                        let Declaration::Function(FunctionDeclaration { signature, .. }) =
                            self.tree.get(*declaration)
                        else {
                            return false;
                        };

                        signature.kind == FunctionKind::Lambda
                            && !signature.generic_parameters.is_empty()
                    });
                    if !starts_generic_lambda {
                        self.restore(speculative_start, speculative_start_idx);
                        return None;
                    }
                }

                // validate that a follow token makes sense for a type argument list
                let cursor = self.scanner_cursor_from(self.pos_index());
                let has_valid_follow = cursor.has_line_break_before
                    || self
                        .can_follow_generic_arguments_at_index(cursor.index, allow_newline_prefix)
                    || (allow_object_literal && cursor.token_type == TokenType::OpenBrace);
                if !has_valid_follow {
                    self.restore(speculative_start, speculative_start_idx);
                    return None;
                }

                Some(generic_arguments)
            }
            Err(_err) => {
                self.restore(speculative_start, speculative_start_idx);
                None
            }
        }
    }
}
