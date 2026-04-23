use crate::Parser;

use destack_ast::{
    Declaration, FunctionDeclaration, FunctionKind, GenericArgument, Keyword, LocalNodeId,
    TokenType, TypeExpression,
};

impl Parser {
    /// Return true when one shift-left generic argument start is a generic lambda head.
    fn shift_left_generic_arguments_start_generic_lambda(
        &self,
        generic_arguments: &[LocalNodeId<GenericArgument>],
    ) -> bool {
        generic_arguments.first().is_some_and(|argument| {
            // require one type generic argument
            let GenericArgument::Type { value } = self.tree.get(*argument) else {
                return false;
            };

            // require one generic lambda type head
            match self.tree.get(*value) {
                TypeExpression::FunctionTypeDeclaration(function) => {
                    !function.generic_parameters.is_empty()
                }
                TypeExpression::Declaration { declaration } => {
                    let Declaration::Function(FunctionDeclaration { signature, .. }) =
                        self.tree.get(*declaration)
                    else {
                        return false;
                    };

                    signature.kind == FunctionKind::Lambda
                        && !signature.generic_parameters.is_empty()
                }
                _ => false,
            }
        })
    }

    /// Return true when the current follow token can continue one parsed generic argument list.
    fn generic_arguments_have_valid_follow(
        &mut self,
        allow_object_literal: bool,
        allow_newline_prefix: bool,
    ) -> bool {
        // inspect the immediate follow token
        let cursor = self.scanner_cursor_from(self.pos_index());

        // line breaks terminate the speculative ambiguity
        if cursor.has_line_break_before {
            return true;
        }

        let token_type = cursor.token_type;

        // static type arguments may be followed by angle closers
        if token_type == TokenType::End
            || self.options.is_in_static() && Self::starts_type_angle_close(token_type)
        {
            return true;
        }

        // conditional and arrow continuations stay valid
        if self.options.is_in_ternary_condition() && token_type == TokenType::Colon {
            return true;
        }

        if self.options.is_in_type()
            && matches!(token_type, TokenType::Arrow | TokenType::ArrowWide)
        {
            return true;
        }

        // statement keywords after newline-prefixed `new` receivers stay valid
        if allow_newline_prefix
            && token_type == TokenType::Identifier
            && self.keyword_for_index(cursor.index).is_some()
        {
            return true;
        }

        // plain delimiters and grouped continuations stay valid
        if matches!(
            token_type,
            TokenType::Comma
                | TokenType::Semicolon
                | TokenType::Newline
                | TokenType::Maybe
                | TokenType::TemplateString
                | TokenType::TemplateStringStart
        ) || Self::is_close_delimiter_token(token_type)
            || matches!(
                token_type,
                TokenType::OpenParenthesis | TokenType::OpenBracket | TokenType::Dot
            )
        {
            return true;
        }

        // heritage clauses may continue with structural heads or later constraints
        if self.options.is_in_super_type()
            && (token_type == TokenType::OpenBrace
                || token_type == TokenType::Identifier
                    && matches!(
                        self.keyword_for_index(cursor.index),
                        Some(Keyword::Implements | Keyword::With | Keyword::Where)
                    ))
        {
            return true;
        }

        // object literals can continue directly when allowed by the caller
        if allow_object_literal && token_type == TokenType::OpenBrace {
            return true;
        }

        self.has_infix_or_assign_operator_at_index(cursor.index)
    }

    /// Speculatively eat one generic argument list at one grammar site.
    ///
    /// Examples:
    /// ```
    /// Foo<T>
    /// foo<T>(value)
    /// new Foo
    ///   <T>(value)
    /// <<T>(value) => value>
    /// ```
    pub(crate) fn try_eat_generic_arguments(
        &mut self,
        allow_object_literal: bool,
        allow_newline_prefix: bool,
    ) -> Option<Vec<LocalNodeId<GenericArgument>>> {
        // untyped value mode does not support generic arguments
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
                if used_shift_left_start
                    && !self.shift_left_generic_arguments_start_generic_lambda(&generic_arguments)
                {
                    self.restore(speculative_start, speculative_start_idx);
                    return None;
                }

                // validate that a follow token makes sense for a type argument list
                if !self
                    .generic_arguments_have_valid_follow(allow_object_literal, allow_newline_prefix)
                {
                    self.restore(speculative_start, speculative_start_idx);
                    return None;
                }

                // keep the parsed generic arguments
                Some(generic_arguments)
            }
            Err(_err) => {
                // roll back failed speculative parses
                self.restore(speculative_start, speculative_start_idx);
                None
            }
        }
    }
}
