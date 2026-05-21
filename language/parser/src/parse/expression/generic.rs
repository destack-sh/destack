use crate::Parser;
use crate::parse::is_declaration_keyword;
use destack_dir::{GenericArgument, Keyword, LocalNodeId, TokenType, TypeExpression};

impl Parser {
    /// Return whether a generic arrow starts here.
    pub(crate) fn can_start_generic_arrow_expression(&mut self) -> bool {
        let needs_tree_disambiguator =
            self.language.is_typescript() && self.language.supports_jsx();
        if self.flags.is_disallow_ambiguous_tree_literal() && !needs_tree_disambiguator {
            return false;
        }

        self.peek_generic_arrow_after_type_parameters(needs_tree_disambiguator)
    }

    /// Return whether a tree literal starts here.
    pub(crate) fn can_start_tree_literal(&mut self) -> bool {
        self.allow_tree_literals() && self.is_tree_literal_start()
    }

    /// Parse generic arguments speculatively.
    ///
    /// Examples:
    /// ```ds
    /// <T, U>
    /// <T extends Base, U = Default>
    /// <<T>() => T>
    /// ```
    pub(crate) fn eat_generic_arguments_if_valid(
        &mut self,
        require_value_postfix_commit: bool,
    ) -> Option<Vec<LocalNodeId<GenericArgument>>> {
        let checkpoint = self.checkpoint();
        let mark = self.tree.next_id();
        let started_with_shift_left = self.peek_is(TokenType::ShiftLeft);

        // parse speculative arguments
        let Ok(arguments) = self.eat_generic_arguments() else {
            self.restore(checkpoint, mark);
            return None;
        };

        // validate shift-left generic arguments
        if started_with_shift_left && !self.shift_left_generic_arguments_are_valid(&arguments) {
            self.restore(checkpoint, mark);
            return None;
        }

        // validate value postfix commitment
        if require_value_postfix_commit && !self.generic_arguments_can_commit_in_value_postfix() {
            self.restore(checkpoint, mark);
            return None;
        }

        Some(arguments)
    }

    /// Return true when `<<...>` generic arguments are structurally intentional.
    fn shift_left_generic_arguments_are_valid(
        &self,
        arguments: &[LocalNodeId<GenericArgument>],
    ) -> bool {
        let Some(first_argument) = arguments.first() else {
            return false;
        };

        matches!(
            self.tree.get(*first_argument),
            GenericArgument::Type { value }
                if matches!(
                    self.tree.get(*value),
                    TypeExpression::FunctionTypeDeclaration(_)
                )
        )
    }

    /// Return true when speculative value type arguments can commit here.
    fn generic_arguments_can_commit_in_value_postfix(&mut self) -> bool {
        if self.current_token_is_on_new_line() {
            let newline_keeps_arguments = self.current_keyword().is_some_and(|keyword| {
                is_declaration_keyword(keyword) || keyword == Keyword::Import
            });
            if newline_keeps_arguments {
                return true;
            }

            if self.peek_unary_prefix_operator_maybe().is_some() {
                return false;
            }
        }

        if self.current_token_can_follow_value_generic_arguments() {
            return true;
        }

        self.peek_infix_operator_maybe().is_some()
    }

    /// Return whether the current token can follow value generic arguments.
    fn current_token_can_follow_value_generic_arguments(&mut self) -> bool {
        if self.is_any_stop() {
            return true;
        }

        if self.current_keyword() == Some(Keyword::If) {
            return true;
        }

        if matches!(
            self.current_keyword(),
            Some(Keyword::Implements | Keyword::With | Keyword::Where)
        ) {
            return true;
        }

        matches!(
            self.peek_token_type(),
            TokenType::End
                | TokenType::Comma
                | TokenType::Semicolon
                | TokenType::Colon
                | TokenType::CloseParenthesis
                | TokenType::CloseBracket
                | TokenType::CloseBrace
                | TokenType::OpenBrace
                | TokenType::OpenParenthesis
                | TokenType::OpenBracket
                | TokenType::Dot
                | TokenType::Maybe
                | TokenType::Not
                | TokenType::TemplateString
                | TokenType::TemplateStringStart
        )
    }
}
