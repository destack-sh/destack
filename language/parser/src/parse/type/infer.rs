use crate::{ParseResult, Parser};

use destack_ast::{Expression, Keyword, LocalNodeId, NodeType, TokenType, TypeExpression};

impl Parser {
    /// Return true when `infer ... extends ...` should continue into a conditional type.
    /// FUGU #Cleanup: infer_extends_starts_conditional now looks extra sus.. do we still need this?
    fn infer_extends_starts_conditional(&mut self) -> bool {
        // `extends` must be next
        if !self.is_keyword(Keyword::Extends) {
            return false;
        }

        // conditional right sides only accept nested conditionals here
        let mut require_nested_close =
            self.options.is_in_type_conditional_right() && !self.options.is_in_parenthesis();
        if require_nested_close {
            let mut index = self.pos() as isize - 1;
            while index >= 0 {
                let token = self.tokens().get(index as usize);
                let Some(token) = token else {
                    break;
                };
                let token_ty = token.token.ty;
                if token_ty == TokenType::Newline {
                    index -= 1;
                    continue;
                }
                if token_ty == TokenType::Identifier
                    && self.keyword_for_index(index as usize) == Some(Keyword::Infer)
                {
                    index -= 1;
                    while index >= 0 {
                        let token = self.tokens().get(index as usize);
                        let Some(token) = token else {
                            break;
                        };
                        let token_ty = token.token.ty;
                        if token_ty == TokenType::Newline {
                            index -= 1;
                            continue;
                        }
                        if matches!(
                            token_ty,
                            TokenType::OpenBracket
                                | TokenType::OpenBrace
                                | TokenType::OpenParenthesis
                                | TokenType::Comma
                        ) {
                            require_nested_close = false;
                        }
                        break;
                    }
                    break;
                }
                index -= 1;
            }
        }

        // scan until `?` or one outer boundary token
        let mut index = self.pos() as usize + 1;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;

        loop {
            self.ensure_token(index);
            let Some(token) = self.tokens().get(index) else {
                break;
            };
            match token.token.ty {
                TokenType::OpenParenthesis => paren_depth += 1,
                TokenType::CloseParenthesis => {
                    if paren_depth == 0 {
                        break;
                    }
                    paren_depth -= 1;
                }
                TokenType::OpenBracket => bracket_depth += 1,
                TokenType::CloseBracket => {
                    if bracket_depth == 0 {
                        break;
                    }
                    bracket_depth -= 1;
                }
                TokenType::OpenBrace => brace_depth += 1,
                TokenType::CloseBrace => {
                    if brace_depth == 0 {
                        break;
                    }
                    brace_depth -= 1;
                }

                // a top-level `?` means conditional type unless the right side required nesting
                TokenType::Maybe => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        if !require_nested_close {
                            return true;
                        }

                        let mut look_index = index + 1;
                        let mut look_paren_depth = 0usize;
                        let mut look_bracket_depth = 0usize;
                        let mut look_brace_depth = 0usize;
                        loop {
                            self.ensure_token(look_index);
                            let Some(look_token) = self.tokens().get(look_index) else {
                                break;
                            };
                            match look_token.token.ty {
                                TokenType::OpenParenthesis => look_paren_depth += 1,
                                TokenType::CloseParenthesis => {
                                    if look_paren_depth == 0 {
                                        return true;
                                    }
                                    look_paren_depth -= 1;
                                }
                                TokenType::OpenBracket => look_bracket_depth += 1,
                                TokenType::CloseBracket => {
                                    if look_bracket_depth == 0 {
                                        return true;
                                    }
                                    look_bracket_depth -= 1;
                                }
                                TokenType::OpenBrace => look_brace_depth += 1,
                                TokenType::CloseBrace => {
                                    if look_brace_depth == 0 {
                                        return true;
                                    }
                                    look_brace_depth -= 1;
                                }

                                // stop once we hit a non-nested outer boundary
                                TokenType::Comma
                                | TokenType::Semicolon
                                | TokenType::Arrow
                                | TokenType::ArrowWide
                                | TokenType::TemplateStringMiddle
                                | TokenType::TemplateStringEnd => {
                                    if look_paren_depth == 0
                                        && look_bracket_depth == 0
                                        && look_brace_depth == 0
                                    {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            look_index += 1;
                        }
                        return false;
                    }
                }

                // outer boundaries stop the scan
                TokenType::Comma
                | TokenType::Semicolon
                | TokenType::Colon
                | TokenType::Arrow
                | TokenType::ArrowWide
                | TokenType::TemplateStringMiddle
                | TokenType::TemplateStringEnd => {
                    if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            index += 1;
        }

        false
    }

    /// Eat one `infer` type expression.
    pub fn eat_type_infer_expression(&mut self) -> ParseResult<LocalNodeId<Expression>> {
        // `infer T`
        let start = self.mark_span();
        self.eat_keyword(Keyword::Infer)?;
        let (name, name_span) = self.eat_identifier_with_span()?;

        // constraint: `infer T extends U`
        let mut constraint_options = self.options.not_in_position().in_type();
        if self.options.is_in_type_conditional_right() {
            constraint_options = constraint_options.in_type_conditional_right();
        }
        let constraint =
            if self.is_keyword(Keyword::Extends) && !self.infer_extends_starts_conditional() {
                self.bump(); // eat extends
                self.eat_newlines_maybe()?;

                let constraint = self.eat_type_expression_or_recover_missing(
                    constraint_options,
                    NodeType::TypeExpression,
                )?;
                Some(constraint)
            } else {
                None
            };

        // infer node
        let type_expression_id = self.insert_node(
            TypeExpression::Infer { name, constraint },
            self.get_span_from(&start),
        );
        self.tree.set_main_span(type_expression_id, name_span);

        Ok(self.insert_type_expression_value(type_expression_id))
    }
}
