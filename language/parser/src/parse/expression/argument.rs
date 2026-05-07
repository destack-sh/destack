use crate::Parser;

use destack_ast::{
    Declaration, FunctionDeclaration, FunctionForm, GenericArgument, Keyword, LocalNodeId,
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

                    signature.form == FunctionForm::Lambda
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
        // line breaks terminate the speculative ambiguity
        if self.current_token_is_on_new_line() {
            return true;
        }

        let token_type = self.peek_token_type();

        // static type arguments may be followed by angle closers
        if token_type == TokenType::End
            || self.flags.is_in_static() && Self::starts_type_angle_close(token_type)
        {
            return true;
        }

        // conditional and arrow continuations stay valid
        if self.flags.is_in_ternary_condition() && token_type == TokenType::Colon {
            return true;
        }

        if self.flags.is_in_type() && matches!(token_type, TokenType::Arrow | TokenType::ArrowWide)
        {
            return true;
        }

        // statement keywords after newline-prefixed `new` receivers stay valid
        if allow_newline_prefix
            && token_type == TokenType::Identifier
            && self.current_keyword().is_some()
        {
            return true;
        }

        // plain delimiters and grouped continuations stay valid
        if self.current_token_is_on_new_line()
            || matches!(
                token_type,
                TokenType::Comma
                    | TokenType::Semicolon
                    | TokenType::Maybe
                    | TokenType::TemplateString
                    | TokenType::TemplateStringStart
            )
            || Self::is_close_delimiter_token(token_type)
            || matches!(
                token_type,
                TokenType::OpenParenthesis | TokenType::OpenBracket | TokenType::Dot
            )
        {
            return true;
        }

        // heritage clauses may continue with structural heads or later constraints
        if self.flags.is_in_super_type()
            && (token_type == TokenType::OpenBrace
                || token_type == TokenType::Identifier
                    && matches!(
                        self.current_keyword(),
                        Some(Keyword::Implements | Keyword::With | Keyword::Where)
                    ))
        {
            return true;
        }

        // object literals can continue directly when allowed by the caller
        if allow_object_literal && token_type == TokenType::OpenBrace {
            return true;
        }

        self.current_token_can_start_infix_or_assign_operator()
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
            && !self.flags.is_in_type()
            && !self.flags.is_in_decorator()
        {
            return None;
        }

        // `<...>` at the current position, or after a newline in `new` receivers
        let has_generic_argument_start =
            if self.peek_is(TokenType::LessThan) || self.peek_is(TokenType::ShiftLeft) {
                true
            } else {
                allow_newline_prefix
                    && self.current_token_is_on_new_line()
                    && matches!(
                        self.peek_token_type(),
                        TokenType::LessThan | TokenType::ShiftLeft
                    )
            };
        if !has_generic_argument_start {
            return None;
        }

        // speculative parse: restore on invalid follow tokens or shift expressions
        let speculative_start = self.checkpoint();
        let speculative_start_idx = self.tree.next_id();

        // `<<...>` starts need an extra value-position admissibility check
        let used_shift_left_start = self.peek_is(TokenType::ShiftLeft);
        match self.eat_generic_arguments() {
            Ok(generic_arguments) => {
                // in type or decorator context, type arguments are always valid
                if self.flags.is_in_type() || self.flags.is_in_decorator() {
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
