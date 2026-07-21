use destack_source::{FileId, Span};

use crate::{
    ConstantId, FunctionId, FunctionTypeId, GlobalId, Linkage, ObjectBuilder, ParseError,
    ParseResult, Token, TokenType, TypeId,
};

use super::cursor::TokenCursor;
use super::symbol::SymbolTable;

/// Parser over one tokenized bytecode module.
#[derive(Debug)]
pub struct Parser<'a> {
    /// Tokenized source cursor.
    pub(super) cursor: TokenCursor<'a>,
    /// Bytecode object being built.
    pub(super) object: ObjectBuilder,
    /// Symbols indexed before declarations are parsed.
    pub(super) symbols: SymbolTable,
}

impl<'a> Parser<'a> {
    /// Create one parser.
    pub fn new(file_id: FileId, source: &'a str) -> Self {
        Self {
            cursor: TokenCursor::new(file_id, source),
            object: ObjectBuilder::new(),
            symbols: SymbolTable::default(),
        }
    }

    /// Return an empty span in this parser's source file.
    pub(super) fn empty_span(&self) -> Span {
        Span::empty(self.cursor.file_id)
    }

    /// Return the raw token index of every top-level declaration.
    pub(super) fn declaration_positions(&self) -> ParseResult<Vec<usize>> {
        let mut positions = Vec::new();
        let mut brace_depth = 0usize;
        let mut parenthesis_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut is_line_start = true;

        for (position, token) in self.cursor.tokens().iter().copied().enumerate() {
            // retain line starts across indentation and comments
            if token.ty == TokenType::Newline {
                is_line_start = true;
                continue;
            }
            if matches!(token.ty, TokenType::Whitespace | TokenType::Comment) {
                continue;
            }

            // collect declarations only outside nested forms and function bodies
            let is_top_level = brace_depth == 0 && parenthesis_depth == 0 && bracket_depth == 0;
            if is_line_start && is_top_level && token.ty == TokenType::Identifier {
                positions.push(position);
            }

            // reject unmatched closing delimiters during the declaration pass
            let is_unmatched = matches!(token.ty, TokenType::CloseBrace if brace_depth == 0)
                || matches!(
                    token.ty,
                    TokenType::CloseParenthesis if parenthesis_depth == 0
                )
                || matches!(token.ty, TokenType::CloseBracket if bracket_depth == 0);
            if is_unmatched {
                return Err(ParseError::new("unmatched closing delimiter", token.span));
            }

            // track exact delimiter nesting for multiline declarations
            match token.ty {
                TokenType::OpenBrace => brace_depth += 1,
                TokenType::CloseBrace => brace_depth -= 1,
                TokenType::OpenParenthesis => parenthesis_depth += 1,
                TokenType::CloseParenthesis => parenthesis_depth -= 1,
                TokenType::OpenBracket => bracket_depth += 1,
                TokenType::CloseBracket => bracket_depth -= 1,
                _ => {}
            }
            is_line_start = false;
        }

        // reject declarations left inside an unterminated form or body
        if brace_depth != 0 || parenthesis_depth != 0 || bracket_depth != 0 {
            return Err(ParseError::new("unterminated delimiter", self.empty_span()));
        }

        Ok(positions)
    }

    /// Assign dense ids to every top-level symbol and function type.
    pub(super) fn index_declarations(&mut self, positions: &[usize]) -> ParseResult<()> {
        self.index_named_function_types(positions)?;
        self.index_symbols(positions)?;
        self.cursor.reset();

        Ok(())
    }

    /// Assign dense ids to every named function type.
    fn index_named_function_types(&mut self, positions: &[usize]) -> ParseResult<()> {
        for position in positions {
            let (_, keyword) = self.begin_declaration(*position)?;
            if self.text(keyword) != "type" {
                continue;
            }
            let name = self.eat_token(TokenType::Identifier)?;
            if !self.peek_is(TokenType::Equal) {
                continue;
            }
            let text = self.text(name).to_string();
            SymbolTable::insert(
                &mut self.symbols.function_types,
                text,
                FunctionTypeId,
                name.span,
            )?;
        }

        Ok(())
    }

    /// Assign dense ids to every linkable declaration.
    fn index_symbols(&mut self, positions: &[usize]) -> ParseResult<()> {
        for position in positions {
            let (_, keyword) = self.begin_declaration(*position)?;
            let keyword_text = self.text(keyword).to_string();
            let is_declaration = matches!(
                keyword_text.as_str(),
                "type" | "constant" | "function" | "global"
            );
            if !is_declaration {
                continue;
            }

            let name = self.eat_token(TokenType::Identifier)?;
            let text = self.text(name).to_string();

            match keyword_text.as_str() {
                "type" if self.peek_is(TokenType::Equal) => {}
                "type" => {
                    SymbolTable::insert(&mut self.symbols.types, text.clone(), TypeId, name.span)?;
                    let name = self.object.intern_string(&text);
                    self.object.push_type(name);
                }
                "constant" => {
                    SymbolTable::insert(&mut self.symbols.constants, text, ConstantId, name.span)?
                }
                "function" => {
                    let function_type = FunctionTypeId(
                        (self.symbols.function_types.len()
                            + self.symbols.function_declarations.len())
                            as u32,
                    );
                    SymbolTable::insert(&mut self.symbols.functions, text, FunctionId, name.span)?;
                    self.symbols
                        .function_declarations
                        .push(super::symbol::FunctionDeclaration {
                            function_type,
                            resume: Vec::new(),
                        });
                }
                "global" => {
                    SymbolTable::insert(&mut self.symbols.globals, text, GlobalId, name.span)?
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Begin one indexed declaration and return its linkage and keyword.
    pub(super) fn begin_declaration(&mut self, position: usize) -> ParseResult<(Linkage, Token)> {
        self.cursor.seek(position);
        let modifiers = self.parse_declaration_modifiers();
        let keyword = self.eat_token(TokenType::Identifier)?;

        Ok((modifiers.linkage(), keyword))
    }
}
